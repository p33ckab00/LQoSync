//! Offline maintenance adapter. It never runs from the web console or its scheduler.
use crate::{
    core_migration::{self, Evidence, Host, Member, RowKey, Transport},
    core_plan, dynamic,
    migration_fence::Lease,
    model::{self, Settings, Subscriber},
    normalizer, orchestrator,
    routeros::{self, Inventory, Row},
    sessions::{self, Observation},
    store,
};
use anyhow::{Context, Result, ensure};
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, BTreeSet, HashMap},
    path::{Path, PathBuf},
    sync::Arc,
};
const MEMBERS: &str = "/ip/firewall/address-list/print";
type Inputs = BTreeMap<String, Arc<Inventory>>;
pub struct Bootstrap {
    pub rows: Vec<Subscriber>,
    pub observation: Observation,
}
fn digest(value: &impl Serialize) -> Result<String> {
    Ok(uuid::Uuid::new_v5(&uuid::Uuid::NAMESPACE_OID, &serde_json::to_vec(value)?).to_string())
}
fn file_revision(path: &Path) -> Result<String> {
    match std::fs::read(path) {
        Ok(v) => digest(&v),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => digest(&Option::<()>::None),
        Err(e) => Err(e.into()),
    }
}
fn observed(settings: &Settings, inputs: &Inputs, ip: &str) -> Result<String> {
    let ip = core_plan::host(ip)?;
    let mut found = BTreeSet::new();
    for b in &settings.core_lists.bindings {
        for r in inputs[&b.router]
            .tables
            .get(MEMBERS)
            .context("Missing address-list inventory")?
        {
            if r.get("list") == Some(&b.list) && !flag(r, "disabled")? {
                let (a, z) = core_plan::span(r.get("address").context("Missing list address")?)?;
                if ip >= a && ip <= z {
                    found.insert(b.wan.clone());
                }
            }
        }
    }
    ensure!(
        found.len() == 1,
        "Observed WAN membership is absent or ambiguous"
    );
    Ok(found.into_iter().next().unwrap())
}
pub fn bootstrap(settings: &Settings, inputs: &Inputs, root: &Path) -> Result<Bootstrap> {
    model::validate(settings)?;
    core_plan::validate(settings)?;
    let config = settings
        .dynamic
        .as_ref()
        .context("Migration requires dynamic topology")?;
    ensure!(
        config
            .policies
            .iter()
            .all(|p| p.balance && p.metric == "online_clients"),
        "Migration requires online-client policies for complete session continuity evidence"
    );
    let errors = BTreeMap::new();
    let now = orchestrator::now();
    let mut rows = normalizer::collect(settings, inputs, &errors, now)?;
    ensure!(!rows.is_empty(), "No selected online clients");
    let raw = rows.clone();
    let prior = sessions::load(root)?;
    let committed = store::load_assignments(root)?;
    let first = sessions::plan(
        settings,
        &mut rows,
        &committed,
        prior.as_ref(),
        inputs,
        &errors,
        now,
    )?;
    let mut pinned = settings.clone();
    for row in &rows {
        let route = first
            .assignments
            .routes
            .get(&row.id)
            .context("Missing dynamic route")?;
        let wan = observed(settings, inputs, &row.ipv4)?;
        let policy = config
            .policies
            .iter()
            .find(|p| p.id == route.policy)
            .context("Missing source policy")?;
        let targets = policy
            .targets
            .iter()
            .filter(|t| t.wan == wan)
            .collect::<Vec<_>>();
        ensure!(
            targets.len() == 1,
            "Observed WAN has no unique eligible destination"
        );
        let pin = dynamic::DynamicPin {
            policy: policy.id.clone(),
            target: targets[0].clone(),
        };
        ensure!(
            config.pins.get(&row.id).is_none_or(|old| old == &pin),
            "Existing manual pin conflicts with observed WAN; operator review required"
        );
        pinned
            .dynamic
            .as_mut()
            .unwrap()
            .pins
            .insert(row.id.clone(), pin);
    }
    rows = raw.clone();
    let seeded = sessions::plan(
        &pinned,
        &mut rows,
        &committed,
        prior.as_ref(),
        inputs,
        &errors,
        now,
    )?;
    rows = raw;
    let verified = sessions::plan(
        settings,
        &mut rows,
        &committed,
        Some(&seeded),
        inputs,
        &errors,
        now,
    )?;
    ensure!(
        verified.assignments.routes == seeded.assignments.routes,
        "Observed routes do not survive temporary pin removal"
    );
    crate::minimums::allocate_assigned(settings, &mut rows, &verified.assignments)?;
    model::compile_assigned(settings, &rows, &verified.assignments)?;
    Ok(Bootstrap {
        rows,
        observation: verified,
    })
}
fn flag(row: &Row, key: &str) -> Result<bool> {
    match row.get(key).map(String::as_str) {
        None | Some("false") => Ok(false),
        Some("true") => Ok(true),
        _ => anyhow::bail!("Unknown RouterOS flag"),
    }
}
fn members(rows: &[Row]) -> Result<Vec<Member>> {
    let mut result = rows
        .iter()
        .map(|r| {
            Ok(Member {
                id: r.get(".id").context("Missing row ID")?.clone(),
                list: r.get("list").context("Missing list")?.clone(),
                address: r.get("address").context("Missing address")?.clone(),
                comment: r.get("comment").cloned().unwrap_or_default(),
                disabled: flag(r, "disabled")?,
                dynamic: flag(r, "dynamic")?,
                timeout: r
                    .get("timeout")
                    .filter(|t| !t.is_empty() && *t != "0s")
                    .cloned(),
            })
        })
        .collect::<Result<Vec<_>>>()?;
    result.sort_by(|a, b| a.id.cmp(&b.id));
    Ok(result)
}
fn check_continuity(old: &Observation, new: &Observation) -> Result<()> {
    ensure!(
        old.active.len() == new.active.len(),
        "Online client set changed"
    );
    for (id, a) in &old.active {
        let b = new.active.get(id).context("Selected session disappeared")?;
        ensure!(
            a.router == b.router
                && a.service == b.service
                && a.source_node == b.source_node
                && a.id == b.id
                && match (a.uptime, b.uptime) {
                    (Some(a), Some(b)) => b >= a,
                    (None, None) => true,
                    _ => false,
                },
            "Selected session reconnected or changed"
        );
    }
    Ok(())
}
fn context_revision(settings: &Settings, inputs: &Inputs, seed: &Bootstrap) -> Result<String> {
    let mut rows = seed.rows.clone();
    rows.sort_by(|a, b| a.id.cmp(&b.id));
    let session_keys = seed
        .observation
        .active
        .iter()
        .map(|(id, s)| (id, &s.router, &s.service, &s.source_node, &s.id))
        .collect::<Vec<_>>();
    let mut policies = BTreeMap::new();
    for b in &settings.core_lists.bindings {
        let inv = &inputs[&b.router];
        let mut tables = BTreeMap::new();
        for path in core_plan::CORE_TABLES
            .iter()
            .chain(["/interface/list/print", "/interface/list/member/print"].iter())
        {
            if *path == MEMBERS || *path == "/system/resource/print" {
                continue;
            }
            let mut values = inv
                .tables
                .get(*path)
                .context("Incomplete routing inventory")?
                .clone();
            values.sort();
            tables.insert(*path, values);
        }
        let resources = inv
            .tables
            .get("/system/resource/print")
            .context("Missing router resource table")?
            .iter()
            .map(|r| {
                r.iter()
                    .filter(|(k, _)| {
                        ["version", "board-name", "total-memory"].contains(&k.as_str())
                    })
                    .collect::<BTreeMap<_, _>>()
            })
            .collect::<Vec<_>>();
        policies.insert(&b.router, (tables, resources));
    }
    digest(&(settings, rows, session_keys, policies))
}
async fn inputs(root: &Path, settings: &Settings) -> Result<Inputs> {
    let mut result = BTreeMap::new();
    for id in crate::validation::required_routers(settings) {
        let router = settings
            .routers
            .iter()
            .find(|r| r.id == id)
            .context("Missing required router")?;
        let inv = routeros::inspect_from(router, root).await?;
        ensure!(inv.errors.is_empty(), "Fresh router collection has errors");
        result.insert(id.to_string(), Arc::new(inv));
    }
    let nodes = dynamic::wan_nodes(settings);
    let interfaces: HashMap<_, _> = nodes
        .iter()
        .map(|n| (n.id.as_str(), n.source.as_str()))
        .collect();
    for id in settings
        .core_lists
        .bindings
        .iter()
        .map(|b| &b.router)
        .collect::<BTreeSet<_>>()
    {
        let bs = settings
            .core_lists
            .bindings
            .iter()
            .filter(|b| &b.router == id)
            .collect::<Vec<_>>();
        crate::core_routes::audit(&result[id], &bs, &interfaces)?;
    }
    Ok(result)
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Prepared {
    pub plan: core_migration::Plan,
    pub operations: serde_json::Value,
    settings: Settings,
    baseline: Observation,
    candidate_revision: String,
    generation_revision: String,
}
impl Prepared {
    pub fn revision(&self) -> Result<String> {
        digest(self)
    }
}
fn journal(root: &Path, prepared: &Prepared) -> Result<PathBuf> {
    let folder = root.join("state/core-migrations");
    std::fs::create_dir_all(&folder)?;
    Ok(folder.join(format!("{}.json", prepared.revision()?)))
}
fn validate_root(root: &Path) -> Result<()> {
    let root = root.canonicalize()?;
    ensure!(
        !root.starts_with("/opt/libreqos") && !root.starts_with("/etc/libreqos"),
        "Maintenance data must be separate from LibreQoS"
    );
    ensure!(
        root.join("settings.toml").is_file(),
        "ROOT must be an existing application data directory with settings.toml"
    );
    Ok(())
}
pub async fn prepare(root: &Path, protected: Vec<RowKey>) -> Result<Prepared> {
    validate_root(root)?;
    let _fence = Lease::exclusive(root)?;
    let settings = store::load_settings(root)?;
    let inputs = inputs(root, &settings).await?;
    let seed = bootstrap(&settings, &inputs, root)?;
    let mut hosts = vec![];
    for row in &seed.rows {
        let route = &seed.observation.assignments.routes[&row.id];
        let b = settings
            .core_lists
            .bindings
            .iter()
            .find(|b| b.wan == route.wan)
            .context("Missing core binding")?;
        hosts.push(Host {
            router: b.router.clone(),
            list: b.list.clone(),
            wan: b.wan.clone(),
            subscriber: row.id.clone(),
            address: format!("{}/32", row.ipv4),
        });
    }
    let evidence = Evidence {
        revision: context_revision(&settings, &inputs, &seed)?,
        captured_at: inputs
            .values()
            .map(|i| i.collected_at)
            .min()
            .context("No required inputs")?,
        rows: settings
            .core_lists
            .bindings
            .iter()
            .map(|b| {
                Ok((
                    b.router.clone(),
                    members(&inputs[&b.router].tables[MEMBERS])?,
                ))
            })
            .collect::<Result<_>>()?,
    };
    let plan = core_migration::Plan::prepare(
        evidence,
        settings.core_lists.bindings.clone(),
        settings.core_lists.owner.clone(),
        hosts,
        protected,
        300,
    )?;
    let operations = plan.preview()?;
    Ok(Prepared {
        operations,
        plan,
        settings,
        baseline: seed.observation,
        candidate_revision: file_revision(&root.join("state/online-assignments.toml"))?,
        generation_revision: file_revision(&root.join("state/current.toml"))?,
    })
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct BootstrapIntent {
    before: String,
    after: String,
    observation: Observation,
}
struct Adapter<'a> {
    root: &'a Path,
    prepared: &'a Prepared,
    expected_candidate: String,
    clients: BTreeMap<String, routeros::Client>,
    last_rows: Option<BTreeMap<String, Vec<Member>>>,
    rolling_back: bool,
}
impl Adapter<'_> {
    async fn capture(&self) -> Result<(Inputs, Bootstrap)> {
        ensure!(
            store::load_settings(self.root)? == self.prepared.settings,
            "Settings changed"
        );
        ensure!(
            file_revision(&self.root.join("state/online-assignments.toml"))?
                == self.expected_candidate,
            "Candidate session state changed outside maintenance"
        );
        let inv = inputs(self.root, &self.prepared.settings).await?;
        let seed = bootstrap(&self.prepared.settings, &inv, self.root)?;
        check_continuity(&self.prepared.baseline, &seed.observation)?;
        Ok((inv, seed))
    }
    async fn client(&mut self, id: &str) -> Result<&routeros::Client> {
        if !self.clients.contains_key(id) {
            let r = self
                .prepared
                .settings
                .routers
                .iter()
                .find(|r| r.id == id)
                .context("Unknown core router")?;
            ensure!(
                self.prepared
                    .settings
                    .core_lists
                    .bindings
                    .iter()
                    .any(|b| b.router == id),
                "Router outside migration scope"
            );
            let password = crate::credentials::password(self.root, r).await?;
            let client = routeros::Client::connect_with_password(r, &password).await?;
            drop(password);
            self.clients.insert(id.into(), client);
        }
        Ok(&self.clients[id])
    }
    async fn before_write(&mut self, router: &str, expected: Option<&Member>) -> Result<()> {
        self.client(router).await?;
        let old = self
            .last_rows
            .clone()
            .context("No engine observation before write")?;
        let evidence = self.observe().await?;
        ensure!(
            evidence.rows == old && evidence.revision == self.prepared.plan.baseline().revision,
            "Rows or source evidence changed before request"
        );
        if let Some(member) = expected {
            self.exact(router, member).await?;
        }
        let now = orchestrator::now();
        ensure!(
            now >= evidence.captured_at && now - evidence.captured_at <= 30,
            "Observation expired during final target read"
        );
        ensure!(
            self.rolling_back || now <= self.prepared.plan.expires_at(),
            "Approval expired during adapter preflight"
        );
        Ok(())
    }
    async fn exact(&mut self, router: &str, expected: &Member) -> Result<()> {
        let rows = self.client(router).await?.read(MEMBERS).await?;
        let rows = members(&rows)?;
        ensure!(
            self.last_rows
                .as_ref()
                .context("No expected router rows")?
                .get(router)
                == Some(&rows),
            "Address-list rows changed during final target read"
        );
        ensure!(
            rows.iter()
                .filter(|r| r.id == expected.id)
                .collect::<Vec<_>>()
                == vec![expected],
            "Target row changed immediately before request"
        );
        Ok(())
    }
}
impl Transport for Adapter<'_> {
    async fn observe(&mut self) -> Result<Evidence> {
        let (inv, seed) = self.capture().await?;
        let evidence = Evidence {
            revision: context_revision(&self.prepared.settings, &inv, &seed)?,
            captured_at: inv
                .values()
                .map(|i| i.collected_at)
                .min()
                .context("No input")?,
            rows: self
                .prepared
                .settings
                .core_lists
                .bindings
                .iter()
                .map(|b| Ok((b.router.clone(), members(&inv[&b.router].tables[MEMBERS])?)))
                .collect::<Result<_>>()?,
        };
        self.last_rows = Some(evidence.rows.clone());
        Ok(evidence)
    }
    async fn add(&mut self, router: &str, m: &Member) -> Result<String> {
        ensure!(
            !m.disabled && !m.dynamic && m.timeout.is_none() && m.id.is_empty(),
            "Invalid add fields"
        );
        self.before_write(router, None)
            .await
            .map_err(|e| e.context(core_migration::RequestNotSent))?;
        self.client(router)
            .await?
            .migration_command(
                "add",
                &[
                    ("list", &m.list),
                    ("address", &m.address),
                    ("comment", &m.comment),
                    ("disabled", "no"),
                ],
            )
            .await?
            .context("Router add did not return a row ID")
    }
    async fn set_address(&mut self, router: &str, expected: &Member, address: &str) -> Result<()> {
        self.before_write(router, Some(expected))
            .await
            .map_err(|e| e.context(core_migration::RequestNotSent))?;
        self.client(router)
            .await?
            .migration_command("set", &[(".id", &expected.id), ("address", address)])
            .await?;
        Ok(())
    }
    async fn remove(&mut self, router: &str, expected: &Member) -> Result<()> {
        self.before_write(router, Some(expected))
            .await
            .map_err(|e| e.context(core_migration::RequestNotSent))?;
        self.client(router)
            .await?
            .migration_command("remove", &[(".id", &expected.id)])
            .await?;
        Ok(())
    }
}
/// The caller must obtain explicit protected-entry authorization and quiesce external RouterOS writers.
/// Local OS access to private plan/data files is the CLI authentication boundary.
pub async fn execute(
    root: &Path,
    prepared: &Prepared,
    approved: &str,
    external_ack: &str,
    rollback: bool,
) -> Result<()> {
    ensure!(
        prepared.operations == prepared.plan.preview()?,
        "Review operations do not match the executable plan"
    );
    validate_root(root)?;
    if !rollback {
        let now = orchestrator::now();
        ensure!(
            now >= prepared.plan.baseline().captured_at && now <= prepared.plan.expires_at(),
            "Plan approval expired or future-dated; no bootstrap or mutation was started"
        );
    }
    let revision = prepared.revision()?;
    ensure!(
        approved == revision && external_ack == format!("EXCLUSIVE_ROUTER_ACCESS:{revision}"),
        "Exact plan approval and external-writer maintenance acknowledgement required"
    );
    let path = journal(root, prepared)?;
    let receipt = path.with_extension("bootstrap.json");
    let _lease = Lease::maintenance(
        root,
        path.to_str().context("Invalid journal path")?,
        &revision,
    )?;
    let current_candidate = file_revision(&root.join("state/online-assignments.toml"))?;
    let mut needs_bootstrap = true;
    if receipt.exists() {
        let intent: BootstrapIntent = serde_json::from_str(&std::fs::read_to_string(&receipt)?)?;
        ensure!(
            intent.before == prepared.candidate_revision
                && intent.after == digest(&toml::to_string(&intent.observation)?.into_bytes())?,
            "Bootstrap intent is inconsistent"
        );
        ensure!(
            current_candidate == intent.before || current_candidate == intent.after,
            "Candidate differs from both bootstrap states; manual recovery required"
        );
        needs_bootstrap = current_candidate == intent.before;
    } else {
        ensure!(
            current_candidate == prepared.candidate_revision,
            "Candidate changed since plan preparation"
        );
    }
    let expected_candidate = current_candidate;
    let mut adapter = Adapter {
        root,
        prepared,
        expected_candidate,
        clients: BTreeMap::new(),
        last_rows: None,
        rolling_back: rollback,
    };
    let mut executor = if path.exists() {
        core_migration::Executor::open(&path, &prepared.plan.revision()?)?
    } else {
        ensure!(!rollback, "No migration journal exists");
        ensure!(
            file_revision(&root.join("state/current.toml"))? == prepared.generation_revision,
            "Committed generation changed"
        );
        core_migration::Executor::create(&path, prepared.plan.clone(), &prepared.plan.revision()?)?
    };
    if !rollback && needs_bootstrap {
        let (_, seed) = adapter.capture().await?;
        ensure!(
            context_revision(
                &prepared.settings,
                &inputs(root, &prepared.settings).await?,
                &seed
            )? == prepared.plan.baseline().revision,
            "Fresh source evidence differs from approved plan"
        );
        let now = orchestrator::now();
        ensure!(
            now <= prepared.plan.expires_at()
                && seed
                    .observation
                    .collected
                    .values()
                    .all(|at| now >= *at && now - *at <= 30),
            "Bootstrap evidence or approval expired"
        );
        let (_, cas) = sessions::load_for_update(root)?;
        let text = toml::to_string(&seed.observation)?;
        let expected = digest(&text.as_bytes().to_vec())?;
        let intent = BootstrapIntent {
            before: prepared.candidate_revision.clone(),
            after: expected.clone(),
            observation: seed.observation.clone(),
        };
        store::atomic_write(&receipt, &serde_json::to_string(&intent)?)?;
        ensure!(
            orchestrator::now() <= prepared.plan.expires_at(),
            "Approval expired before bootstrap CAS"
        );
        cas.save(root, &seed.observation)?;
        adapter.expected_candidate = expected;
    }
    if rollback {
        executor.rollback(&mut adapter).await?;
    } else {
        executor.forward(&mut adapter).await?;
        let (inv, seed) = adapter.capture().await?;
        let plan = core_plan::build(
            &prepared.settings,
            &seed.rows,
            &seed.observation.assignments,
            &inv,
            &BTreeMap::new(),
            orchestrator::now(),
        )?;
        ensure!(
            plan.entries
                .iter()
                .all(|e| e.action == core_plan::Action::Keep),
            "Actual core plan still requires changes"
        );
        let output = model::compile_assigned(
            &prepared.settings,
            &seed.rows,
            &seed.observation.assignments,
        )?;
        let validation_root = path.with_extension("actual-validation");
        std::fs::create_dir_all(&validation_root)?;
        store::publish_orchestrated(
            &validation_root,
            &output,
            &seed.rows,
            &seed.observation.assignments,
            Some(&plan),
        )?;
        sessions::save(&validation_root, &seed.observation)?;
        let report = crate::validation::review(
            &prepared.settings,
            &inv,
            &BTreeMap::new(),
            &validation_root,
            orchestrator::now(),
            crate::core_diagnostics::Origin::Live,
        );
        store::atomic_write(
            &path.with_extension("actual-validation.json"),
            &serde_json::to_string_pretty(&report)?,
        )?;
        ensure!(
            report.status() == crate::core_diagnostics::Outcome::Pass,
            "Fresh actual validation failed; maintenance remains fenced"
        );
    }
    if rollback {
        executor.rollback(&mut adapter).await?;
    } else {
        executor.forward(&mut adapter).await?;
    }
    _lease.finish()?;
    Ok(())
}
