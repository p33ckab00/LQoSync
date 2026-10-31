//! Isolated address-list migration engine. No production transport or console write route is installed.
use crate::{
    core_plan::{self, Binding, span},
    store,
};
use anyhow::{Context, Result, ensure};
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs::{self, File, OpenOptions},
    future::Future,
    net::Ipv4Addr,
    os::unix::fs::OpenOptionsExt,
    path::{Path, PathBuf},
};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Member {
    pub id: String,
    pub list: String,
    pub address: String,
    pub comment: String,
    pub dynamic: bool,
    pub disabled: bool,
    pub timeout: Option<String>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Evidence {
    /// Fingerprint of configuration, eligible destinations, selected IPs, and session identities.
    /// The adapter must obtain these values from fresh complete inputs under its writer fence.
    pub revision: String,
    pub captured_at: u64,
    pub rows: BTreeMap<String, Vec<Member>>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Host {
    pub router: String,
    pub list: String,
    pub wan: String,
    pub subscriber: String,
    pub address: String,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RowKey {
    pub router: String,
    pub id: String,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Plan {
    baseline: Evidence,
    bindings: Vec<Binding>,
    owner: String,
    hosts: Vec<Host>,
    protected: Vec<RowKey>,
    expires_at: u64,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
enum Operation {
    Add {
        router: String,
        member: Member,
    },
    Narrow {
        router: String,
        before: Member,
        address: String,
    },
}
fn fresh(evidence: &Evidence) -> Result<()> {
    let now = crate::orchestrator::now();
    ensure!(
        evidence.captured_at <= now && now - evidence.captured_at <= 30,
        "Evidence is stale or future-dated"
    );
    ensure!(
        !evidence.revision.is_empty(),
        "Missing source/configuration revision"
    );
    Ok(())
}
/// RouterOS form: a single host has no "/32". Verification compares these strings with actual rows.
fn address(a: u32, b: u32) -> String {
    if a == b {
        Ipv4Addr::from(a).to_string()
    } else {
        format!("{}-{}", Ipv4Addr::from(a), Ipv4Addr::from(b))
    }
}
fn coverage(rows: &[Member], list: &str) -> Result<Vec<(u32, u32)>> {
    let mut spans = rows
        .iter()
        .filter(|r| r.list == list && !r.disabled)
        .map(|r| span(&r.address))
        .collect::<Result<Vec<_>>>()?;
    spans.sort_unstable();
    let mut merged: Vec<(u32, u32)> = vec![];
    for (a, b) in spans {
        if let Some(last) = merged
            .last_mut()
            .filter(|last| a as u64 <= last.1 as u64 + 1)
        {
            last.1 = last.1.max(b);
        } else {
            merged.push((a, b));
        }
    }
    Ok(merged)
}
impl Plan {
    pub fn prepare(
        baseline: Evidence,
        bindings: Vec<Binding>,
        owner: String,
        hosts: Vec<Host>,
        protected: Vec<RowKey>,
        valid_for_seconds: u64,
    ) -> Result<Self> {
        fresh(&baseline)?;
        ensure!(
            (1..=300).contains(&valid_for_seconds),
            "Approval lifetime must be 1–300 seconds"
        );
        let expires_at = baseline
            .captured_at
            .checked_add(valid_for_seconds)
            .context("Expiry overflow")?;
        let plan = Self {
            baseline,
            bindings,
            owner,
            hosts,
            protected,
            expires_at,
        };
        plan.operations()?;
        Ok(plan)
    }
    pub fn revision(&self) -> Result<String> {
        Ok(uuid::Uuid::new_v5(&uuid::Uuid::NAMESPACE_OID, &serde_json::to_vec(self)?).to_string())
    }
    pub fn preview(&self) -> Result<serde_json::Value> {
        Ok(serde_json::to_value(self.operations()?)?)
    }
    pub fn expires_at(&self) -> u64 {
        self.expires_at
    }
    pub fn baseline(&self) -> &Evidence {
        &self.baseline
    }
    pub fn owner(&self) -> &str {
        &self.owner
    }
    fn operations(&self) -> Result<Vec<Operation>> {
        ensure!(
            !uuid::Uuid::parse_str(&self.owner)?.is_nil(),
            "Invalid ownership UUID"
        );
        ensure!(
            !self.hosts.is_empty() && !self.protected.is_empty(),
            "Empty migration scope"
        );
        ensure!(
            self.expires_at > self.baseline.captured_at
                && self.expires_at - self.baseline.captured_at <= 300,
            "Invalid approval lifetime"
        );
        let mut scopes = BTreeSet::new();
        let mut wans = BTreeSet::new();
        let mut tables = BTreeSet::new();
        for b in &self.bindings {
            ensure!(
                core_plan::valid_name(&b.list)
                    && core_plan::valid_name(&b.table)
                    && b.table != "main"
                    && crate::model::valid_id(&b.router)
                    && crate::model::valid_id(&b.wan),
                "Invalid binding"
            );
            ensure!(
                self.baseline.rows.contains_key(&b.router),
                "Missing core input"
            );
            ensure!(
                scopes.insert((b.router.as_str(), b.list.as_str()))
                    && wans.insert(b.wan.as_str())
                    && tables.insert((b.router.as_str(), b.table.as_str())),
                "Duplicate binding"
            );
        }
        for rows in self.baseline.rows.values() {
            let mut ids = BTreeSet::new();
            for r in rows {
                ensure!(
                    !r.id.is_empty() && ids.insert(&r.id),
                    "Missing or duplicate router row ID"
                );
            }
        }
        let mut selected: BTreeMap<(&str, &str), BTreeSet<u32>> = BTreeMap::new();
        let mut identities = BTreeSet::new();
        let mut ips = BTreeSet::new();
        let mut adds = vec![];
        for h in &self.hosts {
            let (a, b) = span(&h.address)?;
            ensure!(
                a == b && h.address == format!("{}/32", Ipv4Addr::from(a)),
                "Desired address must be a canonical IPv4 /32"
            );
            let subscriber = uuid::Uuid::parse_str(&h.subscriber)?;
            ensure!(
                !subscriber.is_nil()
                    && subscriber.to_string() == h.subscriber
                    && identities.insert(subscriber)
                    && ips.insert(a),
                "Invalid/duplicate subscriber or IP"
            );
            ensure!(
                self.bindings
                    .iter()
                    .any(|b| b.router == h.router && b.list == h.list && b.wan == h.wan),
                "Host has no eligible core binding"
            );
            // Membership must already exist in exactly the observed WAN. Never force a balancing cutover here.
            let mut observed = BTreeSet::new();
            for binding in &self.bindings {
                for r in &self.baseline.rows[&binding.router] {
                    if r.list == binding.list && !r.disabled {
                        let (first, last) = span(&r.address)?;
                        if a >= first && a <= last {
                            observed.insert(binding.wan.as_str());
                        }
                    }
                }
            }
            ensure!(
                observed.len() == 1 && observed.contains(h.wan.as_str()),
                "Desired WAN differs from unique observed membership"
            );
            selected.entry((&h.router, &h.list)).or_default().insert(a);
            adds.push(Operation::Add {
                router: h.router.clone(),
                member: Member {
                    id: String::new(),
                    list: h.list.clone(),
                    address: address(a, b),
                    comment: core_plan::marker(&self.owner, &h.subscriber, &h.wan),
                    dynamic: false,
                    disabled: false,
                    timeout: None,
                },
            });
        }
        let mut narrow = vec![];
        let mut keys = BTreeSet::new();
        let mut touched = BTreeSet::new();
        let mut modeled = self.baseline.rows.clone();
        for key in &self.protected {
            ensure!(
                keys.insert((&key.router, &key.id)),
                "Duplicate protected row"
            );
            let rows = modeled
                .get_mut(&key.router)
                .context("Unknown core router")?;
            let row = rows
                .iter_mut()
                .find(|r| r.id == key.id)
                .context("Unknown protected row")?;
            ensure!(
                scopes.contains(&(key.router.as_str(), row.list.as_str())),
                "Protected row is outside approved bindings"
            );
            ensure!(
                !row.dynamic
                    && !row.disabled
                    && row
                        .timeout
                        .as_ref()
                        .is_none_or(|t| t.is_empty() || t == "0s")
                    && !row.comment.starts_with("lqosync3:"),
                "Only enabled permanent unowned rows can be narrowed"
            );
            let (first, last) = span(&row.address)?;
            let hosts = selected
                .get(&(key.router.as_str(), row.list.as_str()))
                .context("Protected row has no desired hosts")?;
            let inside: Vec<_> = hosts.range(first..=last).copied().collect();
            ensure!(
                !inside.is_empty(),
                "Protected row does not overlap selected hosts"
            );
            let mut remaining = vec![];
            let mut start = first as u64;
            for ip in inside {
                touched.insert(ip);
                if start < ip as u64 {
                    remaining.push(address(start as u32, ip - 1));
                }
                start = ip as u64 + 1;
            }
            if start <= last as u64 {
                remaining.push(address(start as u32, last));
            }
            ensure!(
                !remaining.is_empty(),
                "Narrowing would remove the entire protected row; explicit removal is unsupported"
            );
            let before = row.clone();
            row.address = remaining[0].clone();
            for residual in &remaining[1..] {
                let mut member = before.clone();
                member.id.clear();
                member.address = residual.clone();
                member.timeout = None;
                adds.push(Operation::Add {
                    router: key.router.clone(),
                    member,
                });
            }
            narrow.push(Operation::Narrow {
                router: key.router.clone(),
                before,
                address: remaining[0].clone(),
            });
        }
        ensure!(
            touched.len() == self.hosts.len(),
            "Every host must replace protected membership"
        );
        // Append residuals and owned hosts before narrowing any original row.
        for op in &adds {
            if let Operation::Add { router, member } = op {
                modeled.get_mut(router).unwrap().push(member.clone());
            }
        }
        for h in &self.hosts {
            let ip = core_plan::host(&h.address)?;
            let matching = modeled[&h.router]
                .iter()
                .filter(|r| r.list == h.list && !r.disabled)
                .filter_map(|r| {
                    span(&r.address)
                        .ok()
                        .filter(|(a, b)| ip >= *a && ip <= *b)
                        .map(|_| r)
                })
                .collect::<Vec<_>>();
            ensure!(
                matching.len() == 1
                    && matching[0].comment == core_plan::marker(&self.owner, &h.subscriber, &h.wan),
                "Selected host still overlaps an existing manual or owned row"
            );
        }
        for (router, list) in scopes {
            ensure!(
                coverage(&self.baseline.rows[router], list)? == coverage(&modeled[router], list)?,
                "Migration changes IPv4 list membership"
            );
        }
        adds.extend(narrow);
        Ok(adds)
    }
}

/// An adapter must hold an exclusive writer fence for the entire call sequence.
/// Each observation must collect complete fresh rows plus configuration/session evidence.
/// Writes must target only the supplied fields/ID, recheck expected rows, and report returned add IDs.
/// This contract is not a RouterOS atomic transaction. No production adapter exists in this module.
#[derive(Debug)]
pub struct RequestNotSent;
impl std::fmt::Display for RequestNotSent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Migration preflight rejected; no mutation request was sent"
        )
    }
}
impl std::error::Error for RequestNotSent {}

pub trait Transport {
    fn observe(&mut self) -> impl Future<Output = Result<Evidence>>;
    fn add(&mut self, router: &str, member: &Member) -> impl Future<Output = Result<String>>;
    fn set_address(
        &mut self,
        router: &str,
        expected: &Member,
        address: &str,
    ) -> impl Future<Output = Result<()>>;
    fn remove(&mut self, router: &str, expected: &Member) -> impl Future<Output = Result<()>>;
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Status {
    Prepared,
    Applying,
    ForwardComplete,
    RollingBack,
    RolledBack,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Applied {
    operation: Operation,
    returned_id: Option<String>,
    undone: bool,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Pending {
    rollback: bool,
    index: usize,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Journal {
    plan: Plan,
    status: Status,
    completed: Vec<Applied>,
    pending: Option<Pending>,
}
/// One path per approved migration. Keep journals and their lock files on private local storage.
pub struct Executor {
    path: PathBuf,
    lock: PathBuf,
    journal: Journal,
    poisoned: bool,
}
impl Executor {
    fn claim(path: &Path) -> Result<PathBuf> {
        ensure!(
            path.parent().is_some_and(Path::is_dir),
            "Journal directory must already exist"
        );
        let lock = path.with_extension("lock");
        let file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .mode(0o600)
            .open(&lock)
            .context("Migration journal is locked; inspect stale locks manually")?;
        if let Err(error) = file
            .sync_all()
            .and_then(|_| File::open(path.parent().unwrap())?.sync_all())
        {
            let _ = fs::remove_file(&lock);
            return Err(error.into());
        }
        Ok(lock)
    }
    pub fn create(path: &Path, plan: Plan, approved_revision: &str) -> Result<Self> {
        ensure!(
            plan.revision()? == approved_revision,
            "Approval does not match this exact plan"
        );
        plan.operations()?;
        let lock = Self::claim(path)?;
        let mut executor = Self {
            path: path.into(),
            lock,
            journal: Journal {
                plan,
                status: Status::Prepared,
                completed: vec![],
                pending: None,
            },
            poisoned: false,
        };
        ensure!(
            !path.exists(),
            "Journal already exists; reopen it instead of repeating migration"
        );
        executor.save()?;
        Ok(executor)
    }
    pub fn open(path: &Path, approved_revision: &str) -> Result<Self> {
        let lock = Self::claim(path)?;
        let result = (|| {
            let journal: Journal = serde_json::from_str(&fs::read_to_string(path)?)?;
            ensure!(
                journal.plan.revision()? == approved_revision,
                "Approval does not match journal"
            );
            Self::validate_journal(&journal)?;
            Ok(journal)
        })();
        match result {
            Ok(journal) => Ok(Self {
                path: path.into(),
                lock,
                journal,
                poisoned: false,
            }),
            Err(e) => {
                let _ = fs::remove_file(lock);
                Err(e)
            }
        }
    }
    fn validate_journal(j: &Journal) -> Result<()> {
        let ops = j.plan.operations()?;
        ensure!(j.completed.len() <= ops.len(), "Invalid journal length");
        let mut ids: BTreeMap<_, BTreeSet<_>> = j
            .plan
            .baseline
            .rows
            .iter()
            .map(|(r, rows)| (r.as_str(), rows.iter().map(|m| m.id.as_str()).collect()))
            .collect();
        let mut saw_undone = false;
        for (done, op) in j.completed.iter().zip(&ops) {
            ensure!(&done.operation == op, "Journal operation differs from plan");
            match op {
                Operation::Add { router, .. } => {
                    let id = done
                        .returned_id
                        .as_deref()
                        .filter(|id| !id.is_empty())
                        .context("Missing returned row ID")?;
                    ensure!(
                        ids.get_mut(router.as_str()).unwrap().insert(id),
                        "Returned row ID collides"
                    );
                }
                Operation::Narrow { .. } => {
                    ensure!(done.returned_id.is_none(), "Unexpected returned row ID")
                }
            }
            if done.undone {
                saw_undone = true;
            } else {
                ensure!(!saw_undone, "Rollback journal is not a reverse prefix");
            }
        }
        match j.status {
            Status::Prepared => ensure!(
                j.completed.is_empty() && j.pending.is_none(),
                "Prepared journal has writes"
            ),
            Status::Applying => ensure!(!saw_undone, "Forward journal contains rollback results"),
            Status::ForwardComplete => ensure!(
                j.completed.len() == ops.len() && !saw_undone && j.pending.is_none(),
                "Incomplete forward journal"
            ),
            Status::RollingBack => {}
            Status::RolledBack => ensure!(
                j.completed.iter().all(|d| d.undone) && j.pending.is_none(),
                "Incomplete rollback journal"
            ),
        }
        if let Some(p) = &j.pending {
            ensure!(
                if p.rollback {
                    j.status == Status::RollingBack
                        && j.completed.iter().rposition(|d| !d.undone) == Some(p.index)
                } else {
                    j.status == Status::Applying
                        && p.index == j.completed.len()
                        && p.index < ops.len()
                },
                "Invalid pending intent"
            );
        }
        Ok(())
    }
    pub fn status(&self) -> Status {
        self.journal.status
    }
    fn save(&mut self) -> Result<()> {
        if let Err(error) =
            store::atomic_write(&self.path, &serde_json::to_string_pretty(&self.journal)?)
        {
            self.poisoned = true;
            return Err(error.context("Journal persistence failed; stop all writes"));
        }
        Ok(())
    }
    fn transport_result<T>(&mut self, result: Result<T>) -> Result<T> {
        if result
            .as_ref()
            .err()
            .is_some_and(|e| e.downcast_ref::<RequestNotSent>().is_some())
        {
            self.journal.pending = None;
            self.save()?;
        }
        result
    }
    fn ready(&self) -> Result<()> {
        ensure!(
            !self.poisoned,
            "Journal persistence failed; reopen and reconcile before more writes"
        );
        ensure!(
            self.journal.pending.is_none(),
            "Unknown write outcome: manual reconciliation required; automatic retry/rollback is blocked"
        );
        Ok(())
    }
    fn expected(&self) -> Result<BTreeMap<String, Vec<Member>>> {
        let mut expected = self.journal.plan.baseline.rows.clone();
        for done in self.journal.completed.iter().filter(|d| !d.undone) {
            match &done.operation {
                Operation::Add { router, member } => {
                    let mut m = member.clone();
                    m.id = done
                        .returned_id
                        .clone()
                        .context("Missing returned add ID")?;
                    expected.get_mut(router).unwrap().push(m);
                }
                Operation::Narrow {
                    router,
                    before,
                    address,
                } => {
                    expected
                        .get_mut(router)
                        .unwrap()
                        .iter_mut()
                        .find(|r| r.id == before.id)
                        .context("Missing original row")?
                        .address = address.clone();
                }
            }
        }
        Ok(expected)
    }
    async fn verify(&self, io: &mut impl Transport) -> Result<u64> {
        let evidence = io.observe().await?;
        fresh(&evidence)?;
        ensure!(
            evidence.revision == self.journal.plan.baseline.revision,
            "Configuration or source sessions changed"
        );
        let mut actual = evidence.rows;
        let mut expected = self.expected()?;
        for rows in actual.values_mut().chain(expected.values_mut()) {
            rows.sort_by(|a, b| a.id.cmp(&b.id));
        }
        ensure!(
            actual == expected,
            "Address-list rows changed; no further writes permitted"
        );
        Ok(evidence.captured_at)
    }
    fn before_request(&mut self, captured_at: u64, forward: bool) -> Result<()> {
        let now = crate::orchestrator::now();
        if captured_at > now
            || now - captured_at > 30
            || (forward && now > self.journal.plan.expires_at)
        {
            // No transport request has started. Clear only this known-unsent intent.
            self.journal.pending = None;
            self.save()?;
            anyhow::bail!("Approval or observation expired before request; no write was sent");
        }
        Ok(())
    }
    pub async fn forward(&mut self, io: &mut impl Transport) -> Result<()> {
        self.ready()?;
        ensure!(
            matches!(
                self.journal.status,
                Status::Prepared | Status::Applying | Status::ForwardComplete
            ),
            "Cannot execute a rolled-back migration"
        );
        if self.journal.status == Status::ForwardComplete {
            return self.verify(io).await.map(|_| ());
        }
        let ops = self.journal.plan.operations()?;
        for op in ops.into_iter().skip(self.journal.completed.len()) {
            ensure!(
                crate::orchestrator::now() <= self.journal.plan.expires_at,
                "Plan approval expired; regenerate evidence and proposal"
            );
            let captured_at = self.verify(io).await?;
            self.journal.status = Status::Applying;
            self.journal.pending = Some(Pending {
                rollback: false,
                index: self.journal.completed.len(),
            });
            self.save()?;
            self.before_request(captured_at, true)?;
            let result = match &op {
                Operation::Add { router, member } => io.add(router, member).await.map(Some),
                Operation::Narrow {
                    router,
                    before,
                    address,
                } => io.set_address(router, before, address).await.map(|_| None),
            };
            let returned_id = self.transport_result(result)?;
            if let Some(id) = &returned_id {
                ensure!(
                    !id.is_empty()
                        && match &op {
                            Operation::Add { router, .. } =>
                                !self.expected()?[router].iter().any(|r| &r.id == id),
                            Operation::Narrow { .. } => false,
                        },
                    "Returned add ID is absent or collides; reconcile pending write"
                );
            }
            self.journal.completed.push(Applied {
                operation: op,
                returned_id,
                undone: false,
            });
            self.journal.pending = None;
            self.save()?;
        }
        self.verify(io).await?;
        self.journal.status = Status::ForwardComplete;
        self.save()
    }
    pub async fn rollback(&mut self, io: &mut impl Transport) -> Result<()> {
        self.ready()?;
        if self.journal.status == Status::RolledBack {
            return self.verify(io).await.map(|_| ());
        }
        self.verify(io).await?;
        self.journal.status = Status::RollingBack;
        self.save()?;
        // Forward operations append all narrowings last. Reverse order restores broad rows first.
        for index in (0..self.journal.completed.len()).rev() {
            let done = self.journal.completed[index].clone();
            if done.undone {
                continue;
            }
            let captured_at = self.verify(io).await?;
            self.journal.pending = Some(Pending {
                rollback: true,
                index,
            });
            self.save()?;
            self.before_request(captured_at, false)?;
            let result = match &done.operation {
                Operation::Narrow {
                    router,
                    before,
                    address,
                } => {
                    let mut expected = before.clone();
                    expected.address = address.clone();
                    io.set_address(router, &expected, &before.address).await
                }
                Operation::Add { router, member } => {
                    let mut expected = member.clone();
                    expected.id = done.returned_id.context("Missing returned add ID")?;
                    io.remove(router, &expected).await
                }
            };
            self.transport_result(result)?;
            self.journal.completed[index].undone = true;
            self.journal.pending = None;
            self.save()?;
        }
        self.verify(io).await?;
        self.journal.status = Status::RolledBack;
        self.save()
    }
}
impl Drop for Executor {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.lock);
    }
}
