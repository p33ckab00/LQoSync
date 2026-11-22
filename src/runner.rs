//! One full cycle: lock, live files, state, parallel router reads, the core, safe writes, reload, status.

use crate::cli::target_name;
use crate::config::{Config, Runtime, RuntimeMode, Target};
use crate::cycle::{CycleInput, RouterResult, assemble};
use crate::files;
use crate::model::{Columns, RouterMemo, State};
use crate::routeros::{Connector, TABLES, read_router};
use crate::routerstate::classify;
use crate::status::{ReportSummary, RouterStatus, Status, Trigger};
use crate::writer::pair::{render_csv, render_network_json};
use crate::writer::v3network::render_libreqos_network;
use anyhow::Context;
use std::collections::{BTreeMap, HashMap};
use std::io::Read;
use std::path::Path;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

mod live_test;
mod topology_cycle;
pub use live_test::{LiveIo, LiveTestReport, NativeIo, restore_backup, restore_pair};

/// The time that LibreQoS v3 can take to import a new `shaped_devices` pair. Its scheduler
/// imports the pair at start, on a reload request, and every 30 minutes.
pub const IMPORT_LIMIT_SECS: u64 = 40 * 60;

/// A random ID of `len` characters from `A-Z0-9`.
pub fn random_id(len: usize) -> String {
    const CHARS: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789";
    let mut bytes = vec![0u8; len];
    std::fs::File::open("/dev/urandom")
        .and_then(|mut f| f.read_exact(&mut bytes))
        .expect("/dev/urandom is readable");
    bytes
        .iter()
        .map(|b| CHARS[*b as usize % CHARS.len()] as char)
        .collect()
}

/// The result of one cycle.
#[derive(Debug, Clone)]
pub struct CycleSummary {
    /// A LibreQoS or candidate file changed.
    pub changed: bool,
    pub error: Option<String>,
    pub status: Status,
}

impl CycleSummary {
    /// Seconds until the next cycle.
    pub fn next_sleep(&self, rt: &Runtime) -> u64 {
        if self.error.is_some() {
            rt.error_retry_interval
        } else if self.changed {
            rt.scan_interval_active
        } else {
            rt.scan_interval_idle
        }
    }
}

/// Read-only enrollment readiness. `prepare_error` holds a source or identity conflict.
#[derive(Debug, Default)]
pub struct TopologyCheck {
    pub policy_issues: Vec<crate::topology_manager::preflight::PolicyIssue>,
    pub prepare_error: Option<String>,
    /// Current accounts with no legacy row, by source scope. They enter after activation.
    pub candidate_only_by_scope: std::collections::BTreeMap<String, usize>,
    pub suggested: usize,
    pub unmatched_legacy: usize,
    /// Locally administered DHCP client MACs. A random MAC can change the identity.
    pub random_macs: Vec<String>,
    pub rounded: Vec<crate::topology_manager::publication::NodeExportAdjustment>,
}

/// What an enrollment installs, or would install when `revision` is `None`.
#[derive(Debug)]
pub struct EnrollOutcome {
    pub revision: Option<u64>,
    pub mapped: usize,
    pub protected: usize,
    pub dropped: Vec<crate::topology_manager::migration::RowIdentity>,
    pub rounded: Vec<crate::topology_manager::publication::NodeExportAdjustment>,
}

/// Runs cycles. The router sessions stay open between cycles.
pub struct Runner<C: Connector + 'static> {
    pub cfg: Config,
    connector: Arc<C>,
    sessions: HashMap<String, Option<C::Session>>,
    /// The instance lock. Taken at the first cycle and held while the runner lives.
    lock: Option<Arc<files::InstanceLock>>,
    /// The error of the last config reload, until a good reload.
    config_error: Option<String>,
    /// The reason for the next cycle. The caller sets it.
    pub trigger: Trigger,
    /// A web edit was saved; the service loop starts the next cycle soon.
    pub reconcile_requested: bool,
    config_path: String,
    config_digest: String,
    cycle_seq: u64,
    cycle_observed_at: u64,
    collection_config_digest: String,
    telegram: String,
    /// The last status written.
    last: Option<Status>,
    collection: Option<crate::collection::Report>,
    pub health: crate::system_health::Hub,
    write_tracker: Option<Arc<Mutex<crate::write_receipt::Tracker>>>,
    topology_store: Option<crate::topology_manager::store::RegistryStore>,
    /// Web mutation receipts. `None` until the registry opens, or when the directory is unsafe.
    web_receipts: Option<crate::topology_manager::web_receipts::Receipts>,
    /// The receipt of the web mutation in progress. `api_edit` marks it prepared before writing.
    web_pending: Option<crate::topology_manager::web_receipts::Record>,
    last_topology_generation: u64,
    /// Start time of the most recent cycle, in Unix seconds.
    last_cycle_started: Option<u64>,
    /// The latest on-demand preview. It is for display only.
    last_preview: Option<serde_json::Value>,
    /// The last `wan.discover` answer. `wan.create` checks candidates against it.
    wan_discovery: Option<serde_json::Value>,
    /// The cycle of the last WAN discovery read by the cycle. `None`: no read since the start.
    wan_read_at: Option<u64>,
    /// The one kept Builder edit proposal.
    edit_proposal: Option<EditProposal>,
    edit_sequence: u64,
    topology_sources: crate::topology_manager::coordinator::Coordinator<C>,
    /// The WAN monitor of the service. `None` in one-shot commands and in most tests.
    pub wan_monitor: Option<crate::wan_monitor::Handle>,
    /// The routing list writer of the service. `None` without writer credentials.
    pub wan_writer: Option<crate::wan_writer::Handle>,
    /// The result of the last routing list write, for the topology view.
    pub wan_lists: Option<serde_json::Value>,
    /// The Topology Manager failure: (first failure time, error), or `None` when it works.
    /// The outer `None` means not read yet: it comes from the last `status.json`, so a restart
    /// is no state change.
    manager_state: Option<Option<(u64, String)>>,
    /// Live source activity (`topology-manager/source-activity.json`), loaded at the first read.
    activity: Option<crate::topology_manager::activity::SourceActivity>,
    topology_compiler: Option<crate::topology_manager::compiler::ControlledCompiler>,
}

/// A Builder edit result that `topology.edit.confirm` commits as it is.
struct EditProposal {
    id: String,
    base_revision: u64,
    candidate: crate::topology_manager::model::RegistryDocument,
    expires: tokio::time::Instant,
    /// A confirmed Rebalance also releases the WANs that wait for it.
    rebalance: bool,
}

/// A proposal stays for this time after `topology.edit.propose`.
const PROPOSAL_LIFETIME: Duration = Duration::from_secs(120);

fn stale_proposal() -> anyhow::Error {
    anyhow::Error::new(crate::topology_manager::store::RevisionConflict)
        .context("the proposal is stale, propose again")
}

fn rule(message: String) -> anyhow::Error {
    anyhow::Error::new(crate::topology_manager::store::InvalidPolicy(message))
}

/// A failure reason for a browser: the rule text or a class, never an internal path.
fn safe_error(error: &anyhow::Error) -> String {
    use crate::topology_manager::store::{InvalidPolicy, RevisionConflict};
    if error.chain().any(|c| c.is::<RevisionConflict>()) {
        "conflict".into()
    } else if let Some(rule) = error
        .chain()
        .find_map(|c| c.downcast_ref::<InvalidPolicy>())
    {
        format!("invalid: {}", rule.0)
    } else {
        "internal".into()
    }
}

fn state_name(r: &RouterResult) -> &'static str {
    match r {
        RouterResult::Online(_) => "online",
        RouterResult::Failed => "failed",
        RouterResult::Unreachable => "unreachable",
        RouterResult::Offline => "offline",
    }
}

/// The files that a cycle reads before the router reads.
pub struct Inputs {
    pub live: files::Live,
    pub state: State,
    pub lqos_dir: std::path::PathBuf,
}

/// Read the live LibreQoS files and the state (or the first state from the live files). No write.
/// The cycle and the settings preview use it.
pub fn cycle_inputs(cfg: &Config, now: u64) -> anyhow::Result<Inputs> {
    let state_dir = Path::new(&cfg.runtime.state_dir);
    let lqos_dir = Path::new(&cfg.libreqos.directory).to_path_buf();
    let first_run = !state_dir.join("state.json").exists();
    let live = files::read_live(
        &lqos_dir,
        &cfg.runtime.static_comment,
        first_run || cfg.network.preserve_network_config,
    )?;
    let state = files::load_state(state_dir, &live, &cfg.circuits, now)?;
    Ok(Inputs {
        live,
        state,
        lqos_dir,
    })
}

/// `<state_dir>/snapshot.json`: the router reads of this cycle. It holds subscriber data, so mode 0600.
fn write_snapshot(
    state_dir: &Path,
    routers: &[(String, RouterResult)],
    statuses: &[RouterStatus],
    now: u64,
) -> anyhow::Result<()> {
    let list: Vec<serde_json::Value> = routers
        .iter()
        .zip(statuses)
        .map(|((name, r), st)| {
            let snapshot = match r {
                RouterResult::Online(s) => serde_json::to_value(s).expect("snapshot serializes"),
                _ => serde_json::Value::Null,
            };
            serde_json::json!({
                "name": name,
                "state": state_name(r),
                "read_at": now,
                "read_ms": st.read_ms,
                "snapshot": snapshot,
            })
        })
        .collect();
    let v = serde_json::json!({"version": 1, "taken": now, "routers": list});
    files::write_private(&state_dir.join("snapshot.json"), v.to_string().as_bytes())
}

impl<C> Runner<C>
where
    C: Connector + 'static,
    C::Session: 'static,
{
    pub fn new(cfg: Config, connector: Arc<C>) -> Self {
        Runner {
            cfg,
            topology_sources: crate::topology_manager::coordinator::Coordinator::new(
                connector.clone(),
            ),
            wan_monitor: None,
            wan_writer: None,
            wan_lists: None,
            manager_state: None,
            activity: None,
            topology_compiler: None,
            connector,
            sessions: HashMap::new(),
            lock: None,
            config_error: None,
            trigger: Trigger::Start,
            reconcile_requested: false,
            config_path: String::new(),
            config_digest: String::new(),
            cycle_seq: 0,
            cycle_observed_at: 0,
            collection_config_digest: String::new(),
            telegram: "none".into(),
            last: None,
            collection: None,
            health: crate::system_health::Hub::default(),
            write_tracker: None,
            topology_store: None,
            web_receipts: None,
            web_pending: None,
            last_topology_generation: 0,
            last_cycle_started: None,
            last_preview: None,
            wan_discovery: None,
            wan_read_at: None,
            edit_proposal: None,
            edit_sequence: 0,
        }
    }

    pub fn pending_tracker(&self) -> Option<Arc<Mutex<crate::write_receipt::Tracker>>> {
        self.write_tracker.clone()
    }

    /// The config file and the digest of its loaded text, for `status.json`.
    pub fn set_source(&mut self, path: &Path, digest: String) {
        self.config_path = path.display().to_string();
        self.config_digest = digest;
    }

    /// The result of the last Telegram send. A new value is written to `status.json` at once.
    pub fn set_telegram(&mut self, result: String) {
        if self.telegram == result {
            return;
        }
        self.telegram = result;
        if let Some(mut s) = self.last.take() {
            s.telegram = self.telegram.clone();
            self.write_status(s);
        }
    }

    /// Write `status.json` at once after a config reload (good or bad), before the next cycle.
    /// `cycle_seq` and the cycle data stay the same.
    pub fn write_reload_status(&mut self, now: u64) {
        if self.lock().is_err() {
            return;
        }
        let mut s = self.last.take().unwrap_or_else(|| self.base_status(now));
        s.trigger = Trigger::Reload;
        s.updated = now;
        s.config_path = self.config_path.clone();
        s.config_digest = self.config_digest.clone();
        s.config_error = self.config_error.clone();
        self.write_status(s);
    }

    fn base_status(&self, now: u64) -> Status {
        Status {
            version: 3,
            updated: now,
            mode: match self.cfg.runtime.mode {
                RuntimeMode::Shadow => "shadow".into(),
                RuntimeMode::Apply => "apply".into(),
            },
            target: target_name(self.cfg.libreqos.target).into(),
            reload: "skipped".into(),
            config_error: self.config_error.clone(),
            config_path: self.config_path.clone(),
            config_digest: self.config_digest.clone(),
            cycle_seq: self.cycle_seq,
            pid: std::process::id(),
            trigger: self.trigger,
            program_version: crate::VERSION.into(),
            telegram: self.telegram.clone(),
            writer: self.writer().into(),
            ..Status::default()
        }
    }

    /// The current Topology Manager failure: (first failure time, error). Read once from the
    /// last `status.json`.
    pub(super) fn manager_failure(&mut self) -> Option<(u64, String)> {
        let state_dir = Path::new(&self.cfg.runtime.state_dir);
        self.manager_state
            .get_or_insert_with(|| {
                let last: serde_json::Value = std::fs::read(state_dir.join("status.json"))
                    .ok()
                    .and_then(|b| serde_json::from_slice(&b).ok())
                    .unwrap_or_default();
                (last["manager_failed"] == true).then(|| {
                    (
                        last["manager_failed_since"].as_u64().unwrap_or_default(),
                        last["manager_error"].as_str().unwrap_or_default().into(),
                    )
                })
            })
            .clone()
    }

    /// Record one topology publication result: the status fields, the `writer` health component,
    /// and one notice when the failure starts or ends. `error` is `None` for a good cycle.
    pub(crate) fn record_manager(&mut self, now: u64, error: Option<String>, status: &mut Status) {
        let state_dir = std::path::PathBuf::from(&self.cfg.runtime.state_dir);
        let previous = self.manager_failure().map(|(since, _)| since);
        let failed = previous.is_some();
        let notice = match (&error, failed) {
            (Some(e), false) => Some(format!(
                "lqosync3: the Topology Manager failed; the last pair stays. {e}"
            )),
            (None, true) => Some("lqosync3: the Topology Manager works again.".to_string()),
            _ => None,
        };
        if let Some(text) = notice
            && let Err(e) =
                crate::notify::outbox::enqueue(&state_dir, &self.cfg.telegram, &text, now)
        {
            eprintln!("warning: cannot queue the writer notice: {e:#}");
        }
        let since = error.as_ref().map(|_| previous.unwrap_or(now));
        self.manager_state = Some(since.zip(error.clone()));
        status.writer = "topology".into();
        status.manager_failed = error.is_some();
        status.manager_failed_since = since;
        let degraded = error.is_some();
        status.manager_error = error;
        let _ = self.health.observe(crate::system_health::Observation {
            domain: crate::system_health::Domain::Apply,
            component: "writer".into(),
            state: if degraded {
                crate::system_health::State::Degraded
            } else {
                crate::system_health::State::Healthy
            },
            reason: if degraded {
                "manager_failed"
            } else {
                "topology"
            }
            .into(),
            observed_at: now,
            max_age: IMPORT_LIMIT_SECS,
            source_epoch: "service".into(),
            last_success: (!degraded).then_some(now),
        });
    }

    fn write_status(&mut self, mut s: Status) {
        s.wan_capacity = self
            .topology_store
            .as_ref()
            .and_then(|store| store.document().ok())
            .and_then(|doc| {
                doc.catalog.as_ref().and_then(|catalog| {
                    crate::status::WanCapacity::from_catalog(catalog, doc.revision)
                })
            });
        self.record_health(&s);
        s.health = Some(
            self.health.snapshot(
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs(),
                None,
            ),
        );
        let path = Path::new(&self.cfg.runtime.state_dir).join("status.json");
        if let Err(e) = files::write_if_changed(&path, s.to_json().as_bytes()) {
            eprintln!("error: cannot write status.json: {e:#}");
        }
        self.last = Some(s);
    }

    fn record_health(&self, s: &Status) {
        use crate::system_health::{Domain, Observation, State};
        let budget = s
            .next_scan_seconds
            .max(self.cfg.runtime.scan_interval_idle)
            .saturating_mul(3)
            .max(30);
        let epoch = format!("{}:{}", s.pid, s.config_digest);
        let receipt = |domain, component: &str, state, reason: &str| {
            if let Err(error) = self.health.observe(Observation {
                domain,
                component: component.into(),
                state,
                reason: reason.into(),
                observed_at: if domain == Domain::Collection {
                    self.collection.as_ref().map_or(0, |r| r.updated)
                } else if domain == Domain::Service && component == "configuration" {
                    s.updated
                } else {
                    self.cycle_observed_at
                },
                max_age: budget,
                source_epoch: epoch.clone(),
                last_success: None,
            }) {
                eprintln!("health receipt: {error}");
            }
        };
        receipt(
            Domain::Service,
            "sync_cycle",
            if s.error.is_some() {
                State::Degraded
            } else {
                State::Healthy
            },
            if s.error.is_some() {
                "cycle_error"
            } else {
                "cycle_complete"
            },
        );
        receipt(
            Domain::Service,
            "configuration",
            if s.config_error.is_some() {
                State::Degraded
            } else {
                State::Healthy
            },
            if s.config_error.is_some() {
                "old_valid_configuration_retained"
            } else {
                "configuration_active"
            },
        );
        if self.collection.as_ref().is_some_and(|r| {
            r.cycle_seq == s.cycle_seq && self.collection_config_digest == self.config_digest
        }) {
            if s.routers.is_empty() {
                receipt(
                    Domain::Collection,
                    "routers",
                    State::NotApplicable,
                    "no_configured_routers",
                );
            }
            for r in &s.routers {
                receipt(
                    Domain::Collection,
                    &r.name,
                    if r.state == "online" {
                        State::Healthy
                    } else {
                        State::Degraded
                    },
                    if r.state == "online" {
                        "collection_complete"
                    } else {
                        "router_collection_unavailable"
                    },
                );
            }
        }
        if s.error.is_none() {
            receipt(
                Domain::Write,
                "cycle_validation",
                if s.report.held.is_empty() {
                    State::Healthy
                } else {
                    State::Degraded
                },
                if s.report.held.is_empty() {
                    "output_validation_complete"
                } else {
                    "removal_guard_held"
                },
            );
        }
        if self.write_tracker.as_ref().is_none_or(|t| {
            t.lock()
                .unwrap_or_else(|p| p.into_inner())
                .current()
                .is_none()
        }) {
            receipt(
                Domain::Apply,
                "latest_update",
                State::NotApplicable,
                "no_tracked_update",
            );
        }
        if !self.cfg.telegram.enabled {
            receipt(
                Domain::Notification,
                "telegram",
                State::NotApplicable,
                "telegram_disabled",
            );
        }
        if s.target == "v2_pair" {
            receipt(
                Domain::Dataplane,
                "native_bridge",
                State::NotApplicable,
                "v2_target",
            );
            receipt(
                Domain::Telemetry,
                "native_telemetry",
                State::NotApplicable,
                "v2_target",
            );
        }
    }

    /// Take the instance lock on `state_dir` and hold it while the runner lives.
    pub fn lock(&mut self) -> anyhow::Result<()> {
        let directory = Path::new(&self.cfg.runtime.state_dir);
        let lock = match &self.lock {
            Some(lock) => {
                lock.check(directory)?;
                lock.clone()
            }
            None => files::InstanceLock::acquire(directory)?,
        };
        self.recover_topology_pending(lock.clone())?;
        if self.lock.is_none() {
            self.lock = Some(lock);
            match crate::write_receipt::Tracker::load(Path::new(&self.cfg.runtime.state_dir)) {
                Ok(tracker) => {
                    tracker.restore_health(&self.health);
                    if let Err(error) = tracker.recover_events(&self.health) {
                        eprintln!("write event recovery: {error}");
                    }
                    if tracker.recovered_pair() {
                        self.request_recovered_import();
                    }
                    self.write_tracker = Some(Arc::new(Mutex::new(tracker)));
                }
                Err(error) => {
                    eprintln!("write receipt recovery: {error:#}");
                    let _ = self.health.observe(crate::system_health::Observation {
                        domain: crate::system_health::Domain::Write,
                        component: "write_receipt_recovery".into(),
                        state: crate::system_health::State::Degraded,
                        reason: "write_receipt_recovery_failed".into(),
                        observed_at: std::time::SystemTime::now()
                            .duration_since(std::time::UNIX_EPOCH)
                            .unwrap_or_default()
                            .as_secs(),
                        max_age: IMPORT_LIMIT_SECS,
                        source_epoch: "service".into(),
                        last_success: None,
                    });
                }
            }
        }
        Ok(())
    }

    fn recover_topology_pending(&mut self, lock: Arc<files::InstanceLock>) -> anyhow::Result<()> {
        if !crate::topology_manager::staging::pending(lock.directory())? {
            return Ok(());
        }
        // Cleanup can fail after journal removal. Never retain an older cached registry on error.
        self.topology_store = None;
        self.sessions.clear();
        self.topology_sources.clear_sessions();
        let mut store = crate::topology_manager::store::RegistryStore::open_owned(lock)?;
        crate::topology_manager::staging::recover(&mut store)?;
        self.topology_store = Some(store);
        Ok(())
    }

    /// Install configured migration policy. Keep runtime activation as a separate action.
    /// The current live pair must still match the fresh manifest. No shaping output is written.
    pub(crate) fn enroll_topology(
        &mut self,
        pair: &crate::topology_manager::publication::PreparedPair,
        manifest: &crate::topology_manager::migration::MigrationManifest,
        protected_roots: &[String],
        attachments: &[crate::topology_manager::ownership::StaticAttachment],
    ) -> anyhow::Result<u64> {
        self.topology_preflight()?;
        let proposal = crate::topology_manager::enrollment::prepare(
            &self.cfg,
            pair,
            manifest,
            protected_roots,
            attachments,
        )?;
        self.open_topology_store()?;
        crate::topology_manager::staging::stage(
            self.topology_store
                .as_mut()
                .context("registry owner is missing")?,
            &proposal,
        )?;
        // Use the same recovery boundary as startup, including after an interrupted install.
        self.lock()?;
        Ok(self
            .topology_store
            .as_ref()
            .context("registry owner is missing")?
            .document()?
            .revision)
    }

    /// The registry document when one exists. A read never creates the store.
    fn api_document(
        &mut self,
    ) -> anyhow::Result<Option<crate::topology_manager::model::RegistryDocument>> {
        self.lock()?;
        let path = Path::new(&self.cfg.runtime.state_dir).join("topology-manager/registry.json");
        if self.topology_store.is_none() && !path.exists() {
            return Ok(None);
        }
        self.open_topology_store()?;
        Ok(Some(
            self.topology_store
                .as_ref()
                .context("registry owner is missing")?
                .document()?
                .clone(),
        ))
    }

    /// API `status`: version, mode, registry revision, enrollment, and the last cycle start.
    pub fn api_status(&mut self) -> anyhow::Result<serde_json::Value> {
        let doc = self.api_document()?;
        let failure = self.manager_failure();
        Ok(serde_json::json!({
            "version": crate::VERSION,
            "mode": format!("{:?}", self.cfg.runtime.mode).to_lowercase(),
            "registry_revision": doc.as_ref().map(|d| d.revision),
            "enrolled": doc.as_ref().is_some_and(|d| d.export_ownership.is_some()),
            "last_cycle_started": self.last_cycle_started,
            "writer": self.writer(),
            "manager_failed": failure.is_some(),
            "manager_error": failure.map(|(_, e)| e),
        }))
    }

    /// Write the web Topology view: registry, cached preview, and the last collection. No router
    /// I/O. The cycle, each registry commit, and each preview refresh call it.
    fn publish_web_view(&mut self, now: u64) {
        let doc = self.api_document().ok().flatten();
        let mode = format!("{:?}", self.cfg.runtime.mode).to_lowercase();
        let publication = self
            .write_tracker
            .as_ref()
            .and_then(|t| t.lock().ok().and_then(|t| t.current().cloned()));
        let history = self
            .web_receipts
            .as_ref()
            .and_then(|r| r.recent().ok())
            .unwrap_or_default();
        let wan = self.wan_monitor.as_ref().map(|m| m.view(now));
        let layout = crate::topology_manager::canvas::load(
            &Path::new(&self.cfg.runtime.state_dir).join("topology-manager"),
        )
        .ok();
        let wan_routers = self.wan_routers(doc.as_ref());
        let view =
            crate::topology_manager::web_view::project(&crate::topology_manager::web_view::Input {
                doc: doc.as_ref(),
                preview: self.last_preview.as_ref(),
                collection: self.collection.as_ref(),
                mode: &mode,
                writer: self.writer(),
                cycle_seq: self.cycle_seq,
                now,
                fresh_for: self.collection.as_ref().map_or(300, |c| c.fresh_for),
                publication: publication.as_ref(),
                history: &history,
                layout: layout.as_ref(),
                wan: wan.as_ref(),
                wan_lists: self.wan_lists.as_ref(),
                activity: self.activity.as_ref(),
                grace: self.cfg.runtime.subscriber_grace,
                wan_discovery: self.wan_discovery.as_ref(),
                wan_routers: &wan_routers,
            });
        let state_dir = Path::new(&self.cfg.runtime.state_dir);
        if let Err(error) = crate::topology_manager::web_view::publish(state_dir, &view) {
            eprintln!("warning: cannot write topology.json: {error:#}");
        }
    }

    /// A restart installed an interrupted pair: ask LibreQoS to import it, as after a write.
    fn request_recovered_import(&self) {
        let v3_pair = crate::detect::resolve(&self.cfg.libreqos)
            .is_ok_and(|d| d.target == crate::config::Target::V3Pair);
        if !v3_pair || self.cfg.runtime.mode != RuntimeMode::Apply {
            return;
        }
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0, |d| d.as_secs());
        let state_dir = Path::new(&self.cfg.runtime.state_dir);
        let lqos_state = Path::new(&self.cfg.libreqos.state_directory);
        if let Err(e) = files::write_if_changed(
            &state_dir.join("import-pending"),
            now.to_string().as_bytes(),
        )
        .and_then(|_| files::request_v3_import(lqos_state, now, &random_id(16)))
        {
            eprintln!("warning: cannot request the import of the recovered pair: {e:#}");
        }
    }

    /// The shaping writer: `topology`, `legacy`, or `unknown` when the claim cannot be read.
    fn writer(&self) -> &'static str {
        match crate::topology_manager::runtime::topology_owns(Path::new(
            &self.cfg.runtime.state_dir,
        )) {
            Ok(true) => "topology",
            Ok(false) => "legacy",
            Err(_) => "unknown",
        }
    }

    /// API `registry.get`.
    pub fn api_registry(
        &mut self,
    ) -> anyhow::Result<Option<crate::topology_manager::model::RegistryDocument>> {
        self.api_document()
    }

    /// API `policy.commit`: replace the catalog and source settings under a revision check.
    /// A failed commit drops the cached store, and the next request opens it again.
    pub fn api_policy_commit(
        &mut self,
        expected_revision: u64,
        catalog: crate::topology_manager::catalog::TopologyCatalog,
        source_settings: crate::topology_manager::settings::SourceSettings,
    ) -> anyhow::Result<u64> {
        self.lock()?;
        self.open_topology_store()?;
        let store = self
            .topology_store
            .as_mut()
            .context("registry owner is missing")?;
        let mut candidate = store.document()?.clone();
        candidate.catalog = Some(catalog);
        candidate.source_settings = Some(source_settings);
        match store.commit(expected_revision, candidate) {
            Ok(outcome) => Ok(outcome.revision),
            Err(e) => {
                self.topology_store = None;
                Err(e)
            }
        }
    }

    /// Read the interfaces of every configured router as WAN candidates. No registry change.
    /// `only`: router IDs to read. Empty: every enabled router.
    pub async fn api_wan_discover(&mut self, only: &[String]) -> anyhow::Result<serde_json::Value> {
        let catalog = self.api_document()?.and_then(|d| d.catalog);
        let collected_at = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0, |d| d.as_secs());
        let reads = crate::topology_manager::wan::read_all(
            self.connector.clone(),
            &self.cfg,
            Duration::from_secs(30),
            only,
        )
        .await;
        let answer = crate::topology_manager::wan::answer(&reads, catalog.as_ref(), collected_at);
        self.wan_discovery = Some(answer.clone());
        Ok(answer)
    }

    /// API `router.test`: one connection test of one enabled router: TCP, TLS, sign-in, and an
    /// identity read. It uses its own session, never the cycle session. A disabled router gets no
    /// connection. The answer has a class, never error text, and the audit log gets one line.
    pub async fn api_router_test(
        &mut self,
        id: &str,
        actor: &str,
    ) -> anyhow::Result<serde_json::Value> {
        use crate::routeros::{ReadError, RouterSession};
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0, |d| d.as_secs());
        let started = Instant::now();
        let router = self
            .cfg
            .routers
            .iter()
            .find(|r| crate::topology_manager::policy_draft::router_id(&r.name) == id)
            .cloned();
        let (class, identity) = match &router {
            None => ("not_found", None),
            Some(r) if !r.enabled => ("disabled", None),
            Some(r) => {
                let access = self.cfg.router_access.clone();
                let limit = Duration::from_secs(access.connect_timeout + access.api_timeout);
                let test = async {
                    let mut session = self.connector.connect(r, &access).await?;
                    let rows = session.print("/system/identity", "name").await?;
                    Ok::<_, ReadError>(rows.first().and_then(|row| row.get("name").cloned()))
                };
                match tokio::time::timeout(limit, test).await {
                    Ok(Ok(name)) => ("ok", name),
                    Ok(Err(ReadError::Unreachable(_))) => ("unreachable", None),
                    Ok(Err(ReadError::Login(_))) => ("login", None),
                    Ok(Err(ReadError::Tls(_))) => ("tls", None),
                    Ok(Err(ReadError::Timeout(_))) | Err(_) => ("timeout", None),
                    Ok(Err(ReadError::Trap(_))) => ("trap", None),
                    Ok(Err(ReadError::Protocol(_))) => ("protocol", None),
                }
            }
        };
        let result = web_contract::routers::RouterTestResult {
            router_id: id.into(),
            class: class.into(),
            identity: identity.map(|i| i.chars().take(64).collect()),
            elapsed_ms: started.elapsed().as_millis() as u64,
            at: now,
        };
        if let Some(r) = &router {
            let event = web_contract::routers::RouterEvent {
                at: now,
                actor: actor.chars().take(64).collect(),
                router_id: id.into(),
                router: r.name.clone(),
                action: "test".into(),
                changes: vec![class.into()],
                digest: self.config_digest.clone(),
            };
            if let Err(e) =
                crate::router_edit::audit(Path::new(&self.cfg.runtime.state_dir), &event)
            {
                eprintln!("warning: cannot write the router audit log: {e:#}");
            }
        }
        Ok(serde_json::to_value(result)?)
    }

    /// API `topology.tree`: every catalog node, without a router read.
    pub fn api_tree(&mut self) -> anyhow::Result<serde_json::Value> {
        let doc = self.api_document()?.unwrap_or_default();
        let mut tree = crate::topology_manager::edits::tree(&doc);
        // A pin is online when one identity of its record is in the cached preview.
        let seen: Vec<&serde_json::Value> = self
            .last_preview
            .as_ref()
            .and_then(|p| p["subscribers"].as_array())
            .into_iter()
            .flatten()
            .flat_map(|s| s["identities"].as_array().into_iter().flatten())
            .collect();
        if let Some(pins) = tree["pins"].as_array_mut() {
            for pin in pins {
                let record = doc
                    .subscribers
                    .iter()
                    .find(|s| pin["subscriber_id"] == s.id.as_str());
                let online = record.is_some_and(|r| {
                    r.identities.iter().any(|i| {
                        let wire = serde_json::json!({
                            "router_id": i.scope.router_id, "source_id": i.scope.source_id,
                            "kind": i.scope.kind, "account_ref": i.account_ref,
                        });
                        seen.contains(&&wire)
                    })
                });
                pin["online"] = serde_json::json!(online);
            }
        }
        Ok(tree)
    }

    /// API `topology.edit.propose`: apply one Builder edit to a copy, rebalance each affected
    /// source pool on a fresh read, and keep the result for `topology.edit.confirm`.
    pub async fn api_edit_propose(
        &mut self,
        expected_revision: u64,
        edit: crate::topology_manager::edits::Edit,
    ) -> anyhow::Result<serde_json::Value> {
        use crate::topology_manager::observation::Reconciliation;
        use crate::topology_manager::resolver::rebalance::{
            RebalanceRequest, RebalanceScope, rebalance,
        };
        self.edit_proposal = None;
        let doc = match self.api_document()? {
            Some(doc) => doc,
            None if matches!(edit, crate::topology_manager::edits::Edit::Initialize) => {
                Default::default()
            }
            None => {
                return Err(rule(
                    "enroll first: the registry has no export ownership".into(),
                ));
            }
        };
        if doc.export_ownership.is_none() && self.writer() != "legacy" {
            return Err(rule(
                "draft editing requires an inactive topology writer".into(),
            ));
        }
        if doc.revision != expected_revision {
            return Err(anyhow::Error::new(
                crate::topology_manager::store::RevisionConflict,
            ));
        }
        if matches!(edit, crate::topology_manager::edits::Edit::WanAdopt) {
            return self.propose_wan_adopt(doc, edit).await;
        }
        let mut applied = match &edit {
            crate::topology_manager::edits::Edit::SourceAdd { key, parent } => {
                let draft = self
                    .propose_policy(Duration::from_secs(30))
                    .await
                    .map_err(|e| rule(format!("{e:#}")))?;
                crate::topology_manager::edits::add_source(&doc, &draft, key, parent.as_deref())?
            }
            _ => crate::topology_manager::edits::apply(
                &doc,
                &edit,
                self.wan_discovery.as_ref(),
                &self.cfg,
            )?,
        };
        if applied.clear_fallback {
            self.check_fallback_clear(&doc, &mut applied).await?;
        }
        if applied.check_only {
            return self.propose_check_only(&doc, edit, applied).await;
        }
        let mut candidate = applied.candidate;
        let mut online = std::collections::HashSet::new();
        let mut cleared_extra = Vec::new();
        if !applied.affected.is_empty() && !crate::topology_manager::edits::frontend_draft(&doc) {
            let mut current = self
                .topology_sources
                .reconcile_for(
                    &self.cfg,
                    &candidate,
                    self.last_topology_generation,
                    Duration::from_secs(30),
                )
                .await?;
            online.extend(current.active.iter().map(|a| a.subscriber_id.clone()));
            if applied.require_online.iter().any(|id| !online.contains(id)) {
                return Err(rule("the subscriber must be online to pin".into()));
            }
            if applied.clear_offline {
                let catalog = current.registry.catalog.clone().unwrap_or_default();
                let in_group = |node: &str| {
                    catalog
                        .nodes
                        .iter()
                        .find(|n| n.id == node)
                        .and_then(|n| n.source_id.as_ref())
                        .is_some_and(|s| applied.affected.contains(s))
                };
                for s in &mut current.registry.subscribers {
                    if !online.contains(&s.id)
                        && s.policy.pin.is_none()
                        && s.policy.assignment.as_deref().is_some_and(in_group)
                    {
                        s.policy.assignment = None;
                        cleared_extra.push(s.id.clone());
                    }
                }
            }
            for source in &applied.affected {
                let generation = current.generation;
                let wan = current.wan.clone();
                let request = RebalanceRequest {
                    expected_revision: current.registry.revision,
                    scope: RebalanceScope::SourcePool(source.clone()),
                };
                let result = rebalance(current, request).map_err(|e| {
                    if format!("{e:#}").contains("rebalance requires resolved placements") {
                        rule("resolve the placement conflicts first".into())
                    } else {
                        rule(format!("{e:#}"))
                    }
                })?;
                let evaluation = result.evaluation;
                if !evaluation.conflicts.is_empty() {
                    return Err(rule(format!("{:?}", evaluation.conflicts)));
                }
                current = Reconciliation {
                    generation,
                    registry: evaluation
                        .candidate
                        .context("the rebalance gave no candidate")?,
                    active: evaluation.active,
                    wan,
                };
            }
            candidate = current.registry;
        }
        let before: std::collections::HashMap<_, _> = doc
            .subscribers
            .iter()
            .map(|s| (s.id.as_str(), s.policy.assignment.as_deref()))
            .collect();
        let labels: std::collections::HashMap<String, String> = self
            .last_preview
            .as_ref()
            .and_then(|p| p["subscribers"].as_array())
            .into_iter()
            .flatten()
            .filter_map(|s| Some((s["id"].as_str()?.into(), s["label"].as_str()?.into())))
            .collect();
        let mut moves = Vec::new();
        let mut assigned = 0;
        for s in candidate
            .subscribers
            .iter()
            .filter(|s| online.contains(&s.id))
        {
            let after = s.policy.assignment.as_deref();
            match before.get(s.id.as_str()).copied().flatten() {
                Some(from) if Some(from) != after => moves.push(serde_json::json!({
                    "subscriber_id": s.id, "label": labels.get(&s.id),
                    "from": from, "to": after,
                })),
                None if after.is_some() => assigned += 1,
                _ => {}
            }
        }
        let mut cleared: Vec<&String> = applied
            .cleared
            .iter()
            .chain(&cleared_extra)
            .filter(|id| !online.contains(*id))
            .collect();
        cleared.sort();
        cleared.dedup();
        let new_records = candidate
            .subscribers
            .len()
            .saturating_sub(doc.subscribers.len());
        self.edit_sequence += 1;
        let id = format!("edit-{}-{}", doc.revision, self.edit_sequence);
        let answer = serde_json::json!({
            "proposal_id": id,
            "expires_at": std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map_or(0, |d| d.as_secs())
                + PROPOSAL_LIFETIME.as_secs(),
            "edit": edit,
            "nodes": crate::topology_manager::edits::tree(&candidate)["nodes"],
            "moves": moves,
            "cleared": cleared,
            "assigned": assigned,
            "new_records": new_records,
            "sources": applied.affected,
        });
        self.edit_proposal = Some(EditProposal {
            id,
            base_revision: doc.revision,
            candidate,
            expires: tokio::time::Instant::now() + PROPOSAL_LIFETIME,
            rebalance: matches!(edit, crate::topology_manager::edits::Edit::Rebalance { .. }),
        });
        Ok(answer)
    }

    /// `fallback.clear`: refuse while an online subscriber is placed in a fallback node, and
    /// clear the saved assignments of offline records to those nodes.
    async fn check_fallback_clear(
        &mut self,
        doc: &crate::topology_manager::model::RegistryDocument,
        applied: &mut crate::topology_manager::edits::Applied,
    ) -> anyhow::Result<()> {
        let nodes: Vec<String> = doc
            .catalog
            .as_ref()
            .and_then(|c| c.fallback.as_ref())
            .map(|f| f.node_ids.clone())
            .unwrap_or_default();
        let read = self
            .topology_sources
            .reconcile_for(
                &self.cfg,
                doc,
                self.last_topology_generation,
                Duration::from_secs(30),
            )
            .await?;
        let online: std::collections::HashSet<String> = read
            .active
            .iter()
            .map(|a| a.subscriber_id.clone())
            .collect();
        let placed = crate::topology_manager::resolver::resolve(read)?;
        if placed
            .placements
            .iter()
            .any(|p| p.target_id.as_ref().is_some_and(|t| nodes.contains(t)))
        {
            return Err(rule("move the fallback subscribers first".into()));
        }
        for s in &mut applied.candidate.subscribers {
            if !online.contains(&s.id)
                && s.policy
                    .assignment
                    .as_ref()
                    .is_some_and(|a| nodes.contains(a))
            {
                s.policy.assignment = None;
                applied.cleared.push(s.id.clone());
            }
        }
        Ok(())
    }

    /// API `topology.sources.candidates`: draft sources that the registry does not know.
    pub async fn api_source_candidates(&mut self) -> anyhow::Result<serde_json::Value> {
        let doc = self
            .api_document()?
            .filter(|d| {
                d.export_ownership.is_some()
                    || (crate::topology_manager::edits::frontend_draft(d)
                        && self.writer() == "legacy")
            })
            .ok_or_else(|| rule("enroll first: the registry has no export ownership".into()))?;
        let draft = self
            .propose_policy(Duration::from_secs(30))
            .await
            .map_err(|e| rule(format!("{e:#}")))?;
        Ok(serde_json::json!({
            "candidates": crate::topology_manager::edits::candidates(&doc, &draft),
        }))
    }

    /// A speed or factor edit: one source read, placement and aggregate checks for the current
    /// and the edited catalog over the same subscribers, and no rebalance. Only a conflict that
    /// the current registry does not have refuses the edit.
    async fn propose_check_only(
        &mut self,
        doc: &crate::topology_manager::model::RegistryDocument,
        edit: crate::topology_manager::edits::Edit,
        applied: crate::topology_manager::edits::Applied,
    ) -> anyhow::Result<serde_json::Value> {
        use crate::topology_manager::aggregate::evaluate;
        use crate::topology_manager::observation::Reconciliation;
        use crate::topology_manager::resolver::{PlacementConflictReason, resolve};
        let read = self
            .topology_sources
            .reconcile_for(
                &self.cfg,
                &applied.candidate,
                self.last_topology_generation,
                Duration::from_secs(30),
            )
            .await?;
        let mut current = read.registry.clone();
        current.catalog = doc.catalog.clone();
        // The saved policy of each known record, so a plan choice shows its rate change.
        for s in &mut current.subscribers {
            if let Some(saved) = doc.subscribers.iter().find(|o| o.id == s.id) {
                s.policy = saved.policy.clone();
            }
        }
        let side = |registry| {
            resolve(Reconciliation {
                generation: read.generation,
                registry,
                active: read.active.clone(),
                wan: read.wan.clone(),
            })
        };
        let (before, after) = (side(current)?, side(read.registry.clone())?);
        // A conflict is the same when its subject and its kind are the same.
        let kind = |d: String| {
            d.split(['(', ' ', '{'])
                .next()
                .unwrap_or_default()
                .to_string()
        };
        let placement = |r: &crate::topology_manager::resolver::PlacementResolution| {
            r.conflicts
                .iter()
                .map(|c| (c.subscriber_id.clone(), kind(format!("{:?}", c.reason))))
                .collect::<std::collections::HashSet<_>>()
        };
        let old = placement(&before);
        let new: Vec<_> = after
            .conflicts
            .iter()
            .filter(|c| !old.contains(&(c.subscriber_id.clone(), kind(format!("{:?}", c.reason)))))
            .collect();
        let rates_bad: Vec<&str> = new
            .iter()
            .filter(|c| matches!(c.reason, PlacementConflictReason::InvalidRates(_)))
            .map(|c| c.subscriber_id.as_str())
            .collect();
        if !rates_bad.is_empty() {
            return Err(rule(format!(
                "the minimum is above the maximum for: {}",
                rates_bad.join(", ")
            )));
        }
        if !new.is_empty() {
            return Err(rule("resolve the placement conflicts first".into()));
        }
        let mbps = |r: crate::topology_manager::rates::RateBps| {
            crate::speed::Rate::try_from(r).map(|r| serde_json::json!([r.down, r.up]))
        };
        let labels: std::collections::HashMap<String, String> = self
            .last_preview
            .as_ref()
            .and_then(|p| p["subscribers"].as_array())
            .into_iter()
            .flatten()
            .filter_map(|s| Some((s["id"].as_str()?.into(), s["label"].as_str()?.into())))
            .collect();
        let earlier: std::collections::HashMap<_, _> = before
            .placements
            .iter()
            .map(|p| (p.subscriber_id.as_str(), p.rates))
            .collect();
        let mut rates = Vec::new();
        for p in &after.placements {
            if let Some(old) = earlier.get(p.subscriber_id.as_str())
                && (old.min != p.rates.min || old.max != p.rates.max)
            {
                rates.push(serde_json::json!({
                    "subscriber_id": p.subscriber_id, "label": labels.get(&p.subscriber_id),
                    "before": {"min_mbps": mbps(old.min)?, "max_mbps": mbps(old.max)?},
                    "after": {"min_mbps": mbps(p.rates.min)?, "max_mbps": mbps(p.rates.max)?},
                }));
            }
        }
        let rates_changed = rates.len();
        rates.truncate(8);
        // Aggregates need placements without conflicts. Leave out each subscriber with an old
        // placement conflict on either side, so one stray subscriber cannot hide a new
        // aggregate conflict, then evaluate both sides over the same subscribers.
        let stray: std::collections::HashSet<String> = before
            .conflicts
            .iter()
            .chain(&after.conflicts)
            .map(|c| c.subscriber_id.clone())
            .collect();
        let clean = |registry| {
            resolve(Reconciliation {
                generation: read.generation,
                registry,
                wan: read.wan.clone(),
                active: read
                    .active
                    .iter()
                    .filter(|a| !stray.contains(&a.subscriber_id))
                    .cloned()
                    .collect(),
            })
            .and_then(evaluate)
        };
        let mut current = read.registry.clone();
        current.catalog = doc.catalog.clone();
        let (before, after) = (Some(clean(current)?), Some(clean(read.registry.clone())?));
        let mut capacities = Vec::new();
        if let Some(after) = &after {
            let old: std::collections::HashSet<_> = before
                .iter()
                .flat_map(|b| &b.conflicts)
                .map(|c| (c.node_id.clone(), kind(format!("{:?}", c.reason))))
                .collect();
            let fresh: Vec<_> = after
                .conflicts
                .iter()
                .filter(|c| !old.contains(&(c.node_id.clone(), kind(format!("{:?}", c.reason)))))
                .collect();
            if !fresh.is_empty() {
                return Err(rule(format!("{fresh:?}")));
            }
            let ceilings: std::collections::HashMap<_, _> = before
                .iter()
                .flat_map(|b| &b.nodes)
                .map(|n| (n.node_id.as_str(), n.rates.ceiling))
                .collect();
            for n in &after.nodes {
                if let Some(old) = ceilings.get(n.node_id.as_str())
                    && *old != n.rates.ceiling
                {
                    capacities.push(serde_json::json!({
                        "node_id": n.node_id, "before_mbps": mbps(*old)?,
                        "after_mbps": mbps(n.rates.ceiling)?,
                    }));
                }
            }
        }
        let warnings: Vec<String> = (applied.legacy_replaced > 0)
            .then(|| {
                format!(
                    "the legacy rounded rules of {} profiles are replaced; the output no longer matches the legacy loop exactly",
                    applied.legacy_replaced
                )
            })
            .into_iter()
            .collect();
        self.edit_sequence += 1;
        let id = format!("edit-{}-{}", doc.revision, self.edit_sequence);
        let candidate = applied.candidate;
        let answer = serde_json::json!({
            "proposal_id": id,
            "expires_at": std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map_or(0, |d| d.as_secs())
                + PROPOSAL_LIFETIME.as_secs(),
            "edit": edit,
            "nodes": crate::topology_manager::edits::tree(&candidate)["nodes"],
            "moves": [], "cleared": [], "assigned": 0, "new_records": 0, "sources": [],
            "rates": rates, "rates_changed": rates_changed,
            "capacities": capacities, "warnings": warnings,
        });
        self.edit_proposal = Some(EditProposal {
            id,
            base_revision: doc.revision,
            candidate,
            expires: tokio::time::Instant::now() + PROPOSAL_LIFETIME,
            rebalance: matches!(edit, crate::topology_manager::edits::Edit::Rebalance { .. }),
        });
        Ok(answer)
    }

    /// API `topology.edit.confirm`: commit the kept proposal when it is still current.
    pub fn api_edit_confirm(&mut self, proposal_id: &str) -> anyhow::Result<serde_json::Value> {
        // A wrong ID keeps the current proposal; an expired one is dropped.
        if self
            .edit_proposal
            .as_ref()
            .is_none_or(|p| p.id != proposal_id)
        {
            return Err(stale_proposal());
        }
        let proposal = self.edit_proposal.take().ok_or_else(stale_proposal)?;
        if tokio::time::Instant::now() >= proposal.expires {
            return Err(stale_proposal());
        }
        let is_draft = crate::topology_manager::edits::frontend_draft(&proposal.candidate);
        if is_draft && self.writer() != "legacy" {
            return Err(rule(
                "draft editing requires an inactive topology writer".into(),
            ));
        }
        let (revision, ()) = self.api_edit(proposal.base_revision, |current, _| {
            if is_draft
                && !(crate::topology_manager::edits::frontend_draft(current)
                    || crate::topology_manager::edits::can_initialize(current))
            {
                return Err(rule(
                    "the registry is no longer an editable frontend draft".into(),
                ));
            }
            Ok((proposal.candidate, ()))
        })?;
        self.last_preview = None;
        if proposal.rebalance
            && let Some(monitor) = &self.wan_monitor
        {
            monitor.release_waiting();
        }
        Ok(serde_json::json!({"revision": revision}))
    }

    /// A reviewed proposal for `wan.adopt`: each unpinned client whose address is in an operator
    /// range list gets a node below the WAN of that list. A client already there keeps its node.
    async fn propose_wan_adopt(
        &mut self,
        doc: crate::topology_manager::model::RegistryDocument,
        edit: crate::topology_manager::edits::Edit,
    ) -> anyhow::Result<serde_json::Value> {
        let monitor = self
            .wan_monitor
            .clone()
            .ok_or_else(|| rule("the WAN monitor is not running".into()))?;
        let catalog = doc
            .catalog
            .clone()
            .ok_or_else(|| rule("enroll first: the registry has no catalog".into()))?;
        let pair = self.prepare_topology(Duration::from_secs(30)).await?;
        let index: HashMap<&str, usize> = catalog
            .nodes
            .iter()
            .enumerate()
            .map(|(i, n)| (n.id.as_str(), i))
            .collect();
        let wan_of = |id: &str| {
            let mut i = *index.get(id)?;
            while let Some(p) = catalog.nodes[i].parent.as_deref() {
                i = index[p];
            }
            Some(catalog.nodes[i].id.as_str())
        };
        let automatic = crate::topology_manager::resolver::automatic_sets(&catalog)?;
        // The pair holds the records of clients that no publication saved yet.
        let mut candidate = pair.candidate().clone();
        anyhow::ensure!(
            candidate.revision == doc.revision,
            "the policy changed during the adoption read"
        );
        let (mut moves, mut skipped) = (Vec::new(), 0u64);
        for s in &pair.view().subscribers {
            let wan = s
                .ips
                .iter()
                .filter_map(|ip| ip.parse().ok())
                .find_map(|ip| monitor.legacy_wan_of(ip));
            let Some(wan) = wan else { continue };
            if s.pin.is_some() {
                skipped += 1;
                continue;
            }
            if wan_of(&s.node) == Some(wan.as_str()) {
                continue;
            }
            // A node below that WAN in the automatic set of its router, or another instance of
            // its source group below that WAN.
            let router = s.identities.first().map(|i| i.router_id.as_str());
            let source = index
                .get(s.node.as_str())
                .and_then(|i| catalog.nodes[*i].source_id.as_deref());
            let target = router
                .and_then(|r| automatic.get(r))
                .into_iter()
                .flatten()
                .find(|n| wan_of(n) == Some(wan.as_str()))
                .cloned()
                .or_else(|| {
                    catalog
                        .nodes
                        .iter()
                        .find(|n| {
                            source.is_some()
                                && n.source_id.as_deref() == source
                                && wan_of(&n.id) == Some(wan.as_str())
                        })
                        .map(|n| n.id.clone())
                });
            let Some(target) = target else {
                skipped += 1;
                continue;
            };
            if let Some(record) = candidate.subscribers.iter_mut().find(|r| r.id == s.id) {
                record.policy.assignment = Some(target.clone());
                moves.push(
                    serde_json::json!({"subscriber_id": s.id, "label": s.account_ref,
                    "from": s.node, "to": target}),
                );
            }
        }
        candidate.validate().map_err(|e| rule(format!("{e:#}")))?;
        self.edit_sequence += 1;
        let id = format!("edit-{}-{}", doc.revision, self.edit_sequence);
        let warnings: Vec<String> = (skipped > 0)
            .then(|| {
                format!(
                    "{skipped} listed client(s) keep their node: pinned, or no node below their WAN"
                )
            })
            .into_iter()
            .collect();
        let answer = serde_json::json!({
            "proposal_id": id,
            "expires_at": std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map_or(0, |d| d.as_secs())
                + PROPOSAL_LIFETIME.as_secs(),
            "edit": edit,
            "nodes": crate::topology_manager::edits::tree(&candidate)["nodes"],
            "moves": moves, "cleared": [], "assigned": 0, "new_records": 0, "sources": [],
            "rates": [], "rates_changed": 0, "capacities": [], "warnings": warnings,
        });
        self.edit_proposal = Some(EditProposal {
            id,
            base_revision: doc.revision,
            candidate,
            expires: tokio::time::Instant::now() + PROPOSAL_LIFETIME,
            rebalance: false,
        });
        Ok(answer)
    }

    /// API `topology.edit.cancel`: drop the kept proposal.
    pub fn api_edit_cancel(&mut self, proposal_id: &str) -> anyhow::Result<serde_json::Value> {
        if self
            .edit_proposal
            .as_ref()
            .is_some_and(|p| p.id == proposal_id)
        {
            self.edit_proposal = None;
        }
        Ok(serde_json::json!({}))
    }

    /// Test only: commit a registry document as the next revision.
    #[cfg(test)]
    pub(crate) fn test_commit(
        &mut self,
        doc: crate::topology_manager::model::RegistryDocument,
    ) -> anyhow::Result<u64> {
        let revision = doc.revision;
        Ok(self.api_edit(revision, |_, _| Ok((doc, ())))?.0)
    }

    /// API `topology.preview.refresh`: read routers, prepare a candidate without the writer-only
    /// checks, and cache the view. A conflict preview is cached too. Other errors keep the cache.
    pub async fn api_preview_refresh(&mut self) -> anyhow::Result<serde_json::Value> {
        use crate::topology_manager::conflicts::PreparationConflicts;
        let doc = self
            .api_document()?
            .filter(|d| d.catalog.is_some() && d.source_settings.is_some())
            .ok_or_else(|| {
                anyhow::Error::new(crate::topology_manager::store::InvalidPolicy(
                    "no installed topology policy".into(),
                ))
            })?;
        let collected_at = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0, |d| d.as_secs());
        let prepared = self
            .topology_sources
            .prepare(
                &self.cfg,
                &doc,
                self.last_topology_generation,
                Duration::from_secs(30),
            )
            .await;
        let value = match prepared {
            Ok(pair) => {
                let mut rounded = Vec::new();
                for a in pair.node_export_adjustments() {
                    let calculated = crate::speed::Rate::try_from(a.calculated_ceiling)?;
                    let exported = crate::speed::Rate::try_from(a.exported_ceiling)?;
                    rounded.push(serde_json::json!({
                        "node_id": a.node_id,
                        "calculated_mbps": [calculated.down, calculated.up],
                        "exported_mbps": [exported.down, exported.up],
                    }));
                }
                let view = pair.view();
                serde_json::json!({
                    "collected_at": collected_at,
                    "registry_revision": doc.revision,
                    "generation": pair.generation(),
                    "state": "ok",
                    "nodes": view.nodes,
                    "subscribers": view.subscribers,
                    "inventory": view.inventory,
                    "conflicts": [],
                    "rounded": rounded,
                })
            }
            Err(e) => match e
                .chain()
                .find_map(|c| c.downcast_ref::<PreparationConflicts>())
            {
                Some(conflicts) => serde_json::json!({
                    "collected_at": collected_at,
                    "registry_revision": doc.revision,
                    "generation": null,
                    "state": "conflicts",
                    "nodes": [],
                    "subscribers": [],
                    "inventory": [],
                    "conflicts": conflicts.0,
                    "rounded": [],
                }),
                None => return Err(e),
            },
        };
        self.last_preview = Some(value.clone());
        self.publish_web_view(collected_at);
        Ok(value)
    }

    /// API `topology.preview.get`: the cached preview, or `null`. No router read.
    pub fn api_preview_get(&mut self) -> anyhow::Result<serde_json::Value> {
        self.lock()?;
        Ok(self.last_preview.clone().unwrap_or(serde_json::Value::Null))
    }

    /// API `topology.policies`: speed defaults and profiles. A source-read profile has no fixed
    /// base rate, so its resolved rates are `null`.
    pub fn api_policies(&mut self) -> anyhow::Result<serde_json::Value> {
        use crate::topology_manager::catalog::BaseRateAuthority;
        let Some(doc) = self.api_document()? else {
            return Ok(serde_json::Value::Null);
        };
        let Some(catalog) = doc.catalog.as_ref() else {
            return Ok(serde_json::Value::Null);
        };
        let mut profiles = Vec::new();
        for p in &catalog.profiles {
            let (base, min, max) = match p.base {
                BaseRateAuthority::SourceRead => (serde_json::json!("source_read"), None, None),
                BaseRateAuthority::Local(rate) => {
                    let r = crate::topology_manager::rates::resolve_subscriber(
                        rate,
                        &catalog.defaults,
                        &p.speed,
                    )?;
                    let (lo, hi) = (
                        crate::speed::Rate::try_from(r.min)?,
                        crate::speed::Rate::try_from(r.max)?,
                    );
                    let b = crate::speed::Rate::try_from(rate)?;
                    (
                        serde_json::json!({"local_mbps": [b.down, b.up]}),
                        Some([lo.down, lo.up]),
                        Some([hi.down, hi.up]),
                    )
                }
            };
            profiles.push(serde_json::json!({
                "id": p.id,
                "name": p.name,
                "base": base,
                "speed": p.speed,
                "min_mbps": min,
                "max_mbps": max,
            }));
        }
        let plans: Vec<_> = catalog
            .profiles
            .iter()
            .filter(|p| crate::topology_manager::plans::is_plan(p))
            .filter_map(|p| match p.base {
                BaseRateAuthority::Local(rate) => Some(serde_json::json!({
                    "id": p.id,
                    "name": p.name,
                    "down_mbps": rate.down / 1_000_000,
                    "up_mbps": rate.up / 1_000_000,
                    "subscribers": doc
                        .subscribers
                        .iter()
                        .filter(|s| s.policy.profile_id.as_deref() == Some(p.id.as_str()))
                        .count(),
                })),
                BaseRateAuthority::SourceRead => None,
            })
            .collect();
        let mbps = |r: crate::topology_manager::rates::RateBps| {
            crate::speed::Rate::try_from(r).map(|r| serde_json::json!([r.down, r.up]))
        };
        let mut groups = Vec::new();
        for source in &catalog.sources {
            if let crate::topology_manager::catalog::SourceOrigin::Subscribers { group, .. } =
                &source.origin
            {
                let text = crate::topology_manager::edits::factor_text;
                groups.push(serde_json::json!({
                    "source_id": source.id, "name": source.name,
                    "factor": [text(group.factors.down), text(group.factors.up)],
                    "formula": group.formula,
                    "backstop_mbps": group.backstop.map(|b| mbps(b.base)).transpose()?,
                    "floor_mbps": group.floor.map(mbps).transpose()?,
                }));
            }
        }
        Ok(
            serde_json::json!({"defaults": catalog.defaults, "profiles": profiles, "plans": plans, "groups": groups}),
        )
    }

    /// Apply one registry edit under the lock and a revision check, then commit it.
    /// A failed commit drops the cached store, and the next request opens it again.
    fn api_edit<T>(
        &mut self,
        expected_revision: u64,
        edit: impl FnOnce(
            &crate::topology_manager::model::RegistryDocument,
            Option<&serde_json::Value>,
        )
            -> anyhow::Result<(crate::topology_manager::model::RegistryDocument, T)>,
    ) -> anyhow::Result<(u64, T)> {
        self.lock()?;
        self.open_topology_store()?;
        let store = self
            .topology_store
            .as_mut()
            .context("registry owner is missing")?;
        let doc = store.document()?.clone();
        if doc.revision != expected_revision {
            return Err(anyhow::Error::new(
                crate::topology_manager::store::RevisionConflict,
            ));
        }
        let (candidate, out) = edit(&doc, self.last_preview.as_ref())?;
        let committed = match (self.web_pending.take(), self.web_receipts.as_ref()) {
            (Some(record), Some(receipts)) => {
                let mut prepare = |doc: &crate::topology_manager::model::RegistryDocument| {
                    receipts.prepare(&record, doc)
                };
                store.commit_observed(expected_revision, candidate, &mut prepare)
            }
            _ => store.commit(expected_revision, candidate),
        };
        match committed {
            Ok(outcome) => {
                if outcome.changed {
                    let now = std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .map_or(0, |d| d.as_secs());
                    self.publish_web_view(now);
                }
                Ok((outcome.revision, out))
            }
            Err(e) => {
                self.topology_store = None;
                Err(e)
            }
        }
    }

    /// API `plan.create`.
    pub fn api_plan_create(
        &mut self,
        expected_revision: u64,
        name: &str,
        down_mbps: u64,
        up_mbps: u64,
    ) -> anyhow::Result<serde_json::Value> {
        let (revision, plan_id) = self.api_edit(expected_revision, |doc, _| {
            crate::topology_manager::plans::create(doc, name, down_mbps, up_mbps)
        })?;
        Ok(serde_json::json!({"revision": revision, "plan_id": plan_id}))
    }

    /// API `plan.update`. A field that is `None` keeps its value.
    pub fn api_plan_update(
        &mut self,
        expected_revision: u64,
        plan_id: &str,
        name: Option<&str>,
        down_mbps: Option<u64>,
        up_mbps: Option<u64>,
    ) -> anyhow::Result<serde_json::Value> {
        let (revision, ()) = self.api_edit(expected_revision, |doc, _| {
            crate::topology_manager::plans::update(doc, plan_id, name, down_mbps, up_mbps)
                .map(|d| (d, ()))
        })?;
        Ok(serde_json::json!({"revision": revision}))
    }

    /// API `plan.delete`. A plan that a subscriber uses stays.
    pub fn api_plan_delete(
        &mut self,
        expected_revision: u64,
        plan_id: &str,
    ) -> anyhow::Result<serde_json::Value> {
        let (revision, ()) = self.api_edit(expected_revision, |doc, _| {
            crate::topology_manager::plans::delete(doc, plan_id).map(|d| (d, ()))
        })?;
        Ok(serde_json::json!({"revision": revision}))
    }

    /// API `subscriber.plan`: attach a plan, or detach with `None`. An account without a record
    /// must be in the cached preview of the current revision. A stale preview gives a conflict.
    pub fn api_subscriber_plan(
        &mut self,
        expected_revision: u64,
        identity: crate::topology_manager::model::IdentityRef,
        plan_id: Option<String>,
    ) -> anyhow::Result<serde_json::Value> {
        let (revision, subscriber_id) = self.api_edit(expected_revision, |doc, preview| {
            let has_record = doc
                .subscribers
                .iter()
                .any(|s| s.identities.contains(&identity));
            let preview = preview.filter(|p| p["state"] == "ok");
            if !has_record
                && preview
                    .is_some_and(|p| p["registry_revision"] != serde_json::json!(doc.revision))
            {
                return Err(anyhow::Error::new(
                    crate::topology_manager::store::RevisionConflict,
                ));
            }
            let wanted = serde_json::json!({
                "router_id": identity.scope.router_id,
                "source_id": identity.scope.source_id,
                "kind": identity.scope.kind,
                "account_ref": identity.account_ref,
            });
            let in_preview = preview.is_some_and(|p| {
                p["subscribers"].as_array().is_some_and(|subs| {
                    subs.iter().any(|s| {
                        s["identities"]
                            .as_array()
                            .is_some_and(|ids| ids.contains(&wanted))
                    })
                })
            });
            crate::topology_manager::plans::attach(doc, &identity, plan_id.as_deref(), in_preview)
        })?;
        Ok(serde_json::json!({"revision": revision, "subscriber_id": subscriber_id}))
    }

    /// Bind the service API socket. The held instance lock proves socket ownership.
    pub fn api_bind(
        &self,
    ) -> anyhow::Result<(tokio::net::UnixListener, crate::api::server::SocketGuard)> {
        let lock = self.lock.as_ref().context("instance lock is missing")?;
        crate::api::server::bind(Path::new(&self.cfg.runtime.state_dir), lock)
    }

    /// Commit the automatic groups of `draft` on top of `base`. `false`: nothing to add, or a
    /// newer registry revision (a web edit came first; the next cycle tries again).
    pub(crate) fn commit_live_groups(
        &mut self,
        base: &crate::topology_manager::model::RegistryDocument,
        draft: &crate::topology_manager::policy_draft::PolicyDraft,
    ) -> anyhow::Result<bool> {
        use crate::topology_manager::edits::{auto_groups, auto_router_nodes};
        let groups = auto_groups(base, draft)?;
        // A router card is out of the tree by default with root_promotion (owner decision 3).
        let exclude = self.cfg.network.mode == crate::config::NetworkMode::RootPromotion;
        let routers = auto_router_nodes(groups.as_ref().unwrap_or(base), draft, exclude)?;
        let Some(next) = routers.or(groups) else {
            return Ok(false);
        };
        match self.api_edit(base.revision, |_, _| Ok((next, ()))) {
            Ok(_) => Ok(true),
            Err(e)
                if e.downcast_ref::<crate::topology_manager::store::RevisionConflict>()
                    .is_some() =>
            {
                Ok(false)
            }
            Err(e) => Err(e),
        }
    }

    /// The names of the WAN routers: the WAN role in the catalog, or no subscriber protocol.
    fn wan_routers(
        &self,
        doc: Option<&crate::topology_manager::model::RegistryDocument>,
    ) -> std::collections::BTreeSet<String> {
        use crate::topology_manager::catalog::RouterRole;
        let catalog = doc.and_then(|d| d.catalog.as_ref());
        self.cfg
            .routers
            .iter()
            .filter(|r| {
                !r.reads_subscribers()
                    || catalog.is_some_and(|c| {
                        c.routers
                            .iter()
                            .any(|cr| cr.name == r.name && cr.roles.contains(&RouterRole::Wan))
                    })
            })
            .map(|r| r.name.clone())
            .collect()
    }

    fn wan_discovery_path(&self) -> std::path::PathBuf {
        Path::new(&self.cfg.runtime.state_dir).join("topology-manager/wan-discovery.json")
    }

    /// Load the last WAN discovery, so that the WAN cards stay after a restart.
    pub(crate) fn load_wan_discovery(&mut self) {
        if self.wan_discovery.is_none() {
            self.wan_discovery = std::fs::read(self.wan_discovery_path())
                .ok()
                .and_then(|b| serde_json::from_slice(&b).ok());
        }
    }

    /// Live source canvas: read the WAN routers at the first cycle and every 10 cycles. Only an
    /// enrolled topology has a canvas, so a legacy installation gets no extra router reads.
    pub(crate) async fn refresh_wan_discovery(&mut self, now: u64) {
        self.load_wan_discovery();
        if self.wan_read_at.is_some_and(|at| self.cycle_seq < at + 10) {
            return;
        }
        let Some(doc) = self
            .api_document()
            .ok()
            .flatten()
            .filter(|d| d.catalog.is_some() && d.export_ownership.is_some())
        else {
            return;
        };
        let only: Vec<String> = self
            .wan_routers(Some(&doc))
            .iter()
            .map(|n| crate::topology_manager::policy_draft::router_id(n))
            .collect();
        // An empty list reads every router: no WAN router means no read.
        if only.is_empty() {
            return;
        }
        self.wan_read_at = Some(self.cycle_seq);
        let reads = crate::topology_manager::wan::read_all(
            self.connector.clone(),
            &self.cfg,
            Duration::from_secs(30),
            &only,
        )
        .await;
        let answer = crate::topology_manager::wan::answer(&reads, doc.catalog.as_ref(), now);
        if let Err(e) = crate::files::write_secret(
            &self.wan_discovery_path(),
            &serde_json::to_vec(&answer).unwrap_or_default(),
        ) {
            eprintln!("warning: cannot write the WAN discovery: {e:#}");
        }
        self.wan_discovery = Some(answer);
    }

    /// Read the live sources, add new groups, and record which groups are active. A failed read
    /// changes no activity.
    pub(crate) async fn record_live_groups(&mut self, now: u64) {
        let Some(active) = self.sync_live_groups().await else {
            return;
        };
        let dir = Path::new(&self.cfg.runtime.state_dir).join("topology-manager");
        let activity = self
            .activity
            .get_or_insert_with(|| crate::topology_manager::activity::SourceActivity::load(&dir));
        activity.observe(now, &active);
        if let Err(e) = activity.save(&dir) {
            eprintln!("warning: cannot write the source activity: {e:#}");
        }
    }

    /// Live source canvas: read the current sources and add the groups of new active sources.
    /// Returns the source IDs of the active groups; `None` when the read failed or did not run.
    pub(crate) async fn sync_live_groups(&mut self) -> Option<std::collections::BTreeSet<String>> {
        use crate::topology_manager::layout::NodeKind;
        // The proposal supports only root_promotion.
        if self.cfg.network.mode != crate::config::NetworkMode::RootPromotion {
            return None;
        }
        let base = self
            .api_document()
            .ok()
            .flatten()
            .filter(|d| d.catalog.is_some() && d.export_ownership.is_some())?;
        let draft = match self.propose_policy(Duration::from_secs(30)).await {
            Ok(d) => d,
            Err(e) => {
                eprintln!("warning: live sources: cannot read the current sources: {e:#}");
                return None;
            }
        };
        match self.commit_live_groups(&base, &draft) {
            Ok(true) => eprintln!("live sources: added the groups of new active sources"),
            Ok(false) => {}
            Err(e) => eprintln!("warning: live sources: no group added: {e:#}"),
        }
        // The source IDs, not the node IDs: a web node or an instance has another node ID.
        Some(
            draft
                .catalog
                .nodes
                .iter()
                .filter(|n| n.kind == NodeKind::SubscriberGroup)
                .filter_map(|n| n.source_id.clone())
                .collect(),
        )
    }

    /// Propose a first topology policy from the configuration and current router data.
    /// No lock is taken and no file is written.
    pub async fn propose_policy(
        &mut self,
        max_age: Duration,
    ) -> anyhow::Result<crate::topology_manager::policy_draft::PolicyDraft> {
        let current = self
            .topology_sources
            .read_for_proposal(&self.cfg, max_age)
            .await?;
        crate::topology_manager::policy_draft::propose(&self.cfg, &current)
    }

    /// Read-only enrollment readiness for a draft policy. No lock is taken and no file is written.
    pub async fn check_topology(
        &mut self,
        draft: &crate::topology_manager::policy_draft::PolicyDraft,
        max_age: Duration,
    ) -> anyhow::Result<TopologyCheck> {
        anyhow::ensure!(
            self.cfg.runtime.mode == RuntimeMode::Apply
                && crate::detect::resolve(&self.cfg.libreqos)?.target == Target::V3Pair,
            "manager enrollment requires the v3 pair apply target"
        );
        let doc = crate::topology_manager::policy_draft::document(draft)?;
        let mut out = TopologyCheck {
            policy_issues: crate::topology_manager::preflight::legacy_issues(&self.cfg, &doc)?,
            ..TopologyCheck::default()
        };
        let (rows, network) = crate::topology_manager::enrollment::read_live_pair(&self.cfg)?;
        match self
            .topology_sources
            .prepare(&self.cfg, &doc, self.last_topology_generation, max_age)
            .await
        {
            Err(e) => out.prepare_error = Some(format!("{e:#}")),
            Ok(pair) => {
                use crate::topology_manager::enroll_mapping::RowDecision;
                let file = crate::topology_manager::enroll_mapping::propose(
                    &pair,
                    &rows,
                    &network,
                    None,
                    &self.cfg.runtime.static_comment,
                )?;
                for row in &file.rows {
                    match row.decision {
                        RowDecision::Map { .. } => out.suggested += 1,
                        RowDecision::Undecided => out.unmatched_legacy += 1,
                        _ => {}
                    }
                }
                let identities = pair.device_identities();
                for account in &file.candidate_only {
                    if let Some(identity) = identities.values().find(|i| &i.account_ref == account)
                    {
                        *out.candidate_only_by_scope
                            .entry(identity.scope.source_id.clone())
                            .or_default() += 1;
                    }
                }
                let mut macs: Vec<_> = identities
                    .values()
                    .filter_map(|i| i.account_ref.strip_prefix("dhcp-mac/v1/"))
                    // Bit 1 of the first octet marks a locally administered (often random) MAC.
                    .filter(|mac| u8::from_str_radix(&mac[..2], 16).is_ok_and(|b| b & 2 != 0))
                    .map(str::to_string)
                    .collect();
                macs.sort_unstable();
                macs.dedup();
                out.random_macs = macs;
                out.rounded = pair.node_export_adjustments().to_vec();
            }
        }
        Ok(out)
    }

    /// Install the first topology policy. Shaping stays with the legacy loop until activation.
    pub fn install_topology_policy(
        &mut self,
        draft: &crate::topology_manager::policy_draft::PolicyDraft,
    ) -> anyhow::Result<u64> {
        self.topology_preflight()?;
        self.open_topology_store()?;
        let store = self
            .topology_store
            .as_mut()
            .context("registry owner is missing")?;
        let current = store.document()?.clone();
        anyhow::ensure!(
            current.export_ownership.is_none(),
            "the topology registry is already enrolled; a first policy cannot replace it"
        );
        let mut candidate = current.clone();
        candidate.catalog = Some(draft.catalog.clone());
        candidate.source_settings = Some(draft.source_settings.clone());
        store.commit(current.revision, candidate)?;
        Ok(store.document()?.revision)
    }

    /// Propose an enrollment mapping file from a fresh candidate and the live legacy pair.
    pub async fn propose_enrollment(
        &mut self,
        previous: Option<&crate::topology_manager::enroll_mapping::MappingFile>,
        max_age: Duration,
        replace: bool,
    ) -> anyhow::Result<crate::topology_manager::enroll_mapping::MappingFile> {
        let pair = self.prepare_topology_for(max_age, replace).await?;
        let (rows, network) = crate::topology_manager::enrollment::read_live_pair(&self.cfg)?;
        crate::topology_manager::enroll_mapping::propose(
            &pair,
            &rows,
            &network,
            previous,
            &self.cfg.runtime.static_comment,
        )
    }

    /// Build the manifest from a confirmed mapping file. Install it only when `install` is true.
    pub async fn enroll_with_mapping(
        &mut self,
        file: &crate::topology_manager::enroll_mapping::MappingFile,
        max_age: Duration,
        install: bool,
        replace: bool,
    ) -> anyhow::Result<EnrollOutcome> {
        let pair = self.prepare_topology_for(max_age, replace).await?;
        let (rows, network) = crate::topology_manager::enrollment::read_live_pair(&self.cfg)?;
        let (manifest, roots, attachments) =
            crate::topology_manager::enroll_mapping::build_manifest(&pair, &rows, &network, file)
                .map_err(|drift| anyhow::anyhow!("{drift}"))?;
        let mut out = EnrollOutcome {
            revision: None,
            mapped: manifest.rows.len(),
            protected: manifest.protected_rows.len(),
            dropped: manifest.dropped_rows.clone(),
            rounded: pair.node_export_adjustments().to_vec(),
        };
        if install {
            out.revision = Some(self.enroll_topology(&pair, &manifest, &roots, &attachments)?);
        }
        Ok(out)
    }

    fn topology_preflight(&mut self) -> anyhow::Result<()> {
        self.lock()?;
        anyhow::ensure!(
            self.cfg.runtime.mode == RuntimeMode::Apply
                && crate::detect::resolve(&self.cfg.libreqos)?.target == Target::V3Pair,
            "manager publication requires the v3 pair apply target"
        );
        anyhow::ensure!(
            self.write_tracker.is_some(),
            "write receipt recovery must succeed before manager publication"
        );
        crate::topology_manager::runtime::topology_owns(Path::new(&self.cfg.runtime.state_dir))?;
        Ok(())
    }

    fn open_topology_store(&mut self) -> anyhow::Result<()> {
        if self.topology_store.is_none() {
            let store = crate::topology_manager::store::RegistryStore::open_owned(
                self.lock
                    .as_ref()
                    .context("instance lock is missing")?
                    .clone(),
            )?;
            // Resolve web receipts that a crash left open, before any other commit. A receipt
            // error disables web mutations only; it never blocks the registry owner.
            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map_or(0, |d| d.as_secs());
            self.web_receipts = match crate::topology_manager::web_receipts::Receipts::open(
                store.owner_directory(),
            )
            .and_then(|r| {
                r.recover(store.document()?)?;
                r.prune(now)?;
                Ok(r)
            }) {
                Ok(r) => Some(r),
                Err(error) => {
                    eprintln!("warning: web receipts unavailable: {error:#}");
                    None
                }
            };
            self.topology_store = Some(store);
        }
        Ok(())
    }

    /// API `topology.web.mutate`: run one closed web command once per request key. A repeated
    /// key with the same actor and command returns the stored receipt and commits nothing.
    pub async fn api_web_mutate(
        &mut self,
        key: &str,
        actor: &str,
        command: web_contract::topology::WebCommand,
    ) -> anyhow::Result<serde_json::Value> {
        use crate::topology_manager::web_receipts::Begin;
        use web_contract::topology::WebCommand as C;
        self.lock()?;
        self.open_topology_store()?;
        let base = self
            .topology_store
            .as_ref()
            .context("registry owner is missing")?
            .document()?
            .revision;
        let receipts = self
            .web_receipts
            .as_ref()
            .context("web receipts are unavailable; web changes are disabled")?;
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0, |d| d.as_secs());
        let record = match receipts.begin(key, actor, &command, base, now)? {
            Begin::Existing(_) => {
                return self.api_web_operation(key, actor);
            }
            Begin::New(record) => record,
        };
        self.web_pending = Some(record.clone());
        let result = match &command {
            C::PlanCreate {
                expected_revision,
                name,
                down_mbps,
                up_mbps,
            } => self.api_plan_create(*expected_revision, name, *down_mbps, *up_mbps),
            C::PlanUpdate {
                expected_revision,
                plan_id,
                name,
                down_mbps,
                up_mbps,
            } => self.api_plan_update(
                *expected_revision,
                plan_id,
                name.as_deref(),
                *down_mbps,
                *up_mbps,
            ),
            C::PlanDelete {
                expected_revision,
                plan_id,
            } => self.api_plan_delete(*expected_revision, plan_id),
            C::SubscriberPlan {
                expected_revision,
                identity,
                plan_id,
            } => serde_json::from_value(serde_json::json!(identity.kind))
                .map_err(|_| rule("unknown identity kind".into()))
                .and_then(|kind| {
                    self.api_subscriber_plan(
                        *expected_revision,
                        crate::topology_manager::model::IdentityRef {
                            scope: crate::topology_manager::model::SourceScope {
                                router_id: identity.router_id.clone(),
                                source_id: identity.source_id.clone(),
                                kind,
                            },
                            account_ref: identity.account_ref.clone(),
                        },
                        plan_id.clone(),
                    )
                }),
            C::EditConfirm { proposal_id } => self.api_edit_confirm(proposal_id),
            C::Edit {
                expected_revision,
                edit,
            } => self.api_edit_apply(*expected_revision, edit).await,
        };
        self.web_pending = None;
        let receipts = self
            .web_receipts
            .as_ref()
            .context("web receipts are unavailable")?;
        let done = match result {
            Ok(value) => receipts.finish_saved(
                &record,
                value["revision"].as_u64().unwrap_or(base),
                value["plan_id"]
                    .as_str()
                    .or(value["subscriber_id"].as_str())
                    .map(str::to_owned),
            )?,
            // The registry write may have started: decide from the registry, never by guessing.
            Err(_) if receipts.get(key)?.is_some_and(|r| r.state != record.state) => {
                self.open_topology_store()?;
                let doc = self
                    .topology_store
                    .as_ref()
                    .context("registry owner is missing")?
                    .document()?
                    .clone();
                let receipts = self
                    .web_receipts
                    .as_ref()
                    .context("web receipts are unavailable")?;
                receipts
                    .lookup(key, actor, &doc)?
                    .context("the web receipt is missing")?
            }
            Err(error) => receipts.finish_failed(&record, &safe_error(&error))?,
        };
        // The commit published the view before this receipt was final; show the final state.
        self.publish_web_view(now);
        Ok(serde_json::to_value(done.view())?)
    }

    /// API `topology.layout.save`: replace the canvas card positions. The registry revision
    /// stays; no cycle is scheduled.
    pub fn api_layout_save(
        &mut self,
        expected_revision: u64,
        positions: std::collections::BTreeMap<String, [f64; 2]>,
    ) -> anyhow::Result<serde_json::Value> {
        self.lock()?;
        self.open_topology_store()?;
        let known = self
            .topology_store
            .as_ref()
            .context("registry owner is missing")?
            .document()?
            .catalog
            .iter()
            .flat_map(|c| &c.nodes)
            .map(|n| n.id.clone())
            .collect();
        let directory = Path::new(&self.cfg.runtime.state_dir).join("topology-manager");
        let revision =
            crate::topology_manager::canvas::save(&directory, expected_revision, positions, &known)
                .map_err(|e| {
                    if e.is::<crate::topology_manager::store::RevisionConflict>() {
                        e
                    } else {
                        rule(format!("{e:#}"))
                    }
                })?;
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0, |d| d.as_secs());
        self.publish_web_view(now);
        Ok(serde_json::json!({ "revision": revision }))
    }

    /// One automatic Builder edit: the proposal checks run, then the commit, with no review step.
    async fn api_edit_apply(
        &mut self,
        expected_revision: u64,
        edit: &web_contract::topology::BuilderEdit,
    ) -> anyhow::Result<serde_json::Value> {
        let edit = serde_json::to_value(edit)
            .and_then(serde_json::from_value)
            .map_err(|_| rule("the edit is not a service edit".into()))?;
        if matches!(edit, crate::topology_manager::edits::Edit::FailbackRelease) {
            let monitor = self
                .wan_monitor
                .as_ref()
                .ok_or_else(|| rule("the WAN monitor is not running".into()))?;
            monitor.release_waiting();
            self.reconcile_requested = true;
            return Ok(serde_json::json!({"revision": expected_revision}));
        }
        let proposal = self.api_edit_propose(expected_revision, edit).await?;
        let id = proposal["proposal_id"]
            .as_str()
            .context("the edit made no proposal")?
            .to_owned();
        let saved = self.api_edit_confirm(&id)?;
        self.reconcile_requested = true;
        Ok(saved)
    }

    /// API `topology.web.operation`: the receipt of one request key, or `null` for another actor.
    pub fn api_web_operation(
        &mut self,
        key: &str,
        actor: &str,
    ) -> anyhow::Result<serde_json::Value> {
        self.lock()?;
        self.open_topology_store()?;
        let doc = self
            .topology_store
            .as_ref()
            .context("registry owner is missing")?
            .document()?
            .clone();
        let receipts = self
            .web_receipts
            .as_ref()
            .context("web receipts are unavailable")?;
        Ok(match receipts.lookup(key, actor, &doc)? {
            Some(record) => serde_json::to_value(record.view())?,
            None => serde_json::Value::Null,
        })
    }

    /// Prepare fresh configured output for preview or enrollment. Do not write or claim a writer.
    pub async fn prepare_topology(
        &mut self,
        max_age: Duration,
    ) -> anyhow::Result<crate::topology_manager::publication::PreparedPair> {
        self.prepare_topology_for(max_age, false).await
    }

    /// As `prepare_topology`. With `replace`, prepare from the registry without its legacy
    /// ownership, so that enrollment can run again before activation.
    pub async fn prepare_topology_for(
        &mut self,
        max_age: Duration,
        replace: bool,
    ) -> anyhow::Result<crate::topology_manager::publication::PreparedPair> {
        self.topology_preflight()?;
        self.open_topology_store()?;
        let mut document = self
            .topology_store
            .as_ref()
            .context("registry owner is missing")?
            .document()?
            .clone();
        if replace {
            anyhow::ensure!(
                !crate::topology_manager::runtime::topology_owns(Path::new(
                    &self.cfg.runtime.state_dir
                ))
                .unwrap_or(true),
                "the Topology Manager is active; re-enrollment is not allowed"
            );
            anyhow::ensure!(
                document.export_ownership.is_some(),
                "the topology registry is not enrolled; use enroll without --replace"
            );
            document.export_ownership = None;
        }
        // Owner decision: an enabled protocol without a saved selection is read too, so that
        // automatic placement shapes its subscribers. The publication commits the selections.
        // shortcut: a failed draft (for example an unsupported config) keeps the saved policy
        // without a status message; add one when operators need to see it.
        if !replace
            && crate::topology_manager::edits::unread_protocols(&document, &self.cfg)
            && let Ok(draft) = self.propose_policy(max_age).await
            && let Ok(Some(next)) = crate::topology_manager::edits::auto_read(&document, &draft)
        {
            document = next;
        }
        // The WAN monitor probes the WAN nodes of this policy; placement uses its snapshot.
        if let Some(monitor) = &self.wan_monitor {
            let (wans, auto_failback) = crate::wan_monitor::probes_of(&document);
            monitor.set_wans(wans, auto_failback);
            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs();
            self.topology_sources.set_wan(monitor.snapshot(now));
        }
        let receipt_generation = self
            .write_tracker
            .as_ref()
            .context("receipt owner is missing")?
            .lock()
            .map_err(|_| anyhow::anyhow!("write receipt owner is poisoned"))?
            .current()
            .and_then(|r| r.meta.topology.as_ref())
            .map_or(0, |t| t.observation_generation);
        self.topology_sources
            .prepare(
                &self.cfg,
                &document,
                self.last_topology_generation.max(receipt_generation),
                max_age,
            )
            .await
    }

    /// Collect, authorize, compile, and publish one complete current source batch.
    /// The service loop must pass all activation gates before it calls this API.
    pub async fn sync_topology(
        &mut self,
        compiler: &mut crate::topology_manager::compiler::ControlledCompiler,
        max_age: Duration,
        now: u64,
    ) -> anyhow::Result<crate::write_receipt::WriteResult> {
        self.topology_preflight()?;
        compiler.check_config(&self.cfg.libreqos)?;
        let pair = self.prepare_topology(max_age).await?;
        self.publish_topology(compiler, pair, now).await
    }

    /// Compile and publish a manager pair through this runner's sole writer owner.
    /// The service loop must pass all activation gates before it calls this API.
    pub async fn publish_topology(
        &mut self,
        compiler: &mut crate::topology_manager::compiler::ControlledCompiler,
        pair: crate::topology_manager::publication::PreparedPair,
        now: u64,
    ) -> anyhow::Result<crate::write_receipt::WriteResult> {
        self.topology_preflight()?;
        anyhow::ensure!(
            pair.generation() > self.last_topology_generation,
            "out-of-order topology observation generation"
        );
        self.check_topology_pair(&pair)?;
        compiler.check_config(&self.cfg.libreqos)?;
        let wanted = self.wanted_lists(&pair)?;
        let compiled = compiler.compile(pair).await?;
        self.publish_with_lists(compiled, wanted, now).await
    }

    /// The routing list entries of a pair, when the writer is on. `None` when it is off.
    pub(crate) fn wanted_lists(
        &self,
        pair: &crate::topology_manager::publication::PreparedPair,
    ) -> anyhow::Result<
        Option<
            std::collections::BTreeMap<
                std::net::Ipv4Addr,
                crate::topology_manager::wan_lists::ListEntry,
            >,
        >,
    > {
        if !self.cfg.wan_writer.enabled || self.wan_writer.is_none() {
            return Ok(None);
        }
        let catalog = pair.catalog().context("the pair has no catalog")?;
        crate::topology_manager::wan_lists::wanted(pair, catalog).map(Some)
    }

    /// Write the routing lists, then publish. A failed or held write keeps the previous pair,
    /// so shaping and routing do not part.
    pub(crate) async fn publish_with_lists(
        &mut self,
        compiled: crate::topology_manager::compiler::CompiledPair,
        wanted: Option<
            std::collections::BTreeMap<
                std::net::Ipv4Addr,
                crate::topology_manager::wan_lists::ListEntry,
            >,
        >,
        now: u64,
    ) -> anyhow::Result<crate::write_receipt::WriteResult> {
        if let (Some(wanted), Some(writer)) = (wanted, self.wan_writer.clone()) {
            use crate::wan_writer::WriteError;
            let mut lists = std::collections::BTreeMap::<String, u64>::new();
            for e in wanted.values() {
                *lists.entry(e.list.clone()).or_default() += 1;
            }
            match writer.write(wanted, &self.cfg.wan_writer).await {
                Ok(r) => {
                    self.wan_lists = Some(serde_json::json!({"state": "written", "at": now,
                        "added": r.added, "changed": r.changed, "removed": r.removed,
                        "owned": r.owned_after, "lists": lists}));
                }
                Err(e) => {
                    let (state, reason) = match e {
                        WriteError::Held { reason, .. } => ("held", reason),
                        WriteError::Router(reason) => ("failed", reason),
                    };
                    self.wan_lists =
                        Some(serde_json::json!({"state": state, "at": now, "error": reason}));
                    anyhow::bail!(
                        "routing lists {state}: {reason}; the previous shaping pair stays"
                    );
                }
            }
        }
        self.publish_compiled(compiled, now)
    }
    fn publish_compiled(
        &mut self,
        compiled: crate::topology_manager::compiler::CompiledPair,
        now: u64,
    ) -> anyhow::Result<crate::write_receipt::WriteResult> {
        self.lock()?;
        anyhow::ensure!(
            self.cfg.runtime.mode == RuntimeMode::Apply
                && crate::detect::resolve(&self.cfg.libreqos)?.target == Target::V3Pair,
            "manager publication requires the v3 pair apply target"
        );
        anyhow::ensure!(
            compiled.generation() > self.last_topology_generation,
            "out-of-order topology observation generation"
        );
        compiled.check()?;
        compiled.binding().check_config(&self.cfg.libreqos)?;
        let tracker = self
            .write_tracker
            .as_ref()
            .context("write receipt recovery must succeed before manager publication")?
            .clone();
        let mut tracker = tracker
            .lock()
            .map_err(|_| anyhow::anyhow!("write receipt owner is poisoned"))?;
        self.open_topology_store()?;
        let store = self
            .topology_store
            .as_mut()
            .context("registry owner is missing")?;
        anyhow::ensure!(
            store.document()?.revision == compiled.expected_revision(),
            "manager publication revision conflict"
        );
        self.last_topology_generation = compiled.generation();
        let mut result = compiled.publish(
            store,
            &mut tracker,
            crate::topology_manager::publication::PublishRequest {
                directory: Path::new(&self.cfg.libreqos.directory),
                meta: crate::write_receipt::Meta {
                    topology: None,
                    cycle_seq: self.cycle_seq,
                    config_digest: self.config_digest.clone(),
                    notice: None,
                },
                now,
                hub: &self.health,
            },
        )?;
        if result.error.is_none()
            && let Some(receipt) = tracker.current()
            && receipt.phase.pending()
        {
            let reload = files::request_v3_import(
                Path::new(&self.cfg.libreqos.state_directory),
                receipt.started,
                &receipt.write_id,
            );
            let _ = self.health.observe(crate::system_health::Observation {
                domain: crate::system_health::Domain::Write,
                component: "topology_reload_request".into(),
                state: if reload.is_ok() {
                    crate::system_health::State::Updating
                } else {
                    crate::system_health::State::Degraded
                },
                reason: if reload.is_ok() {
                    "reload_requested"
                } else {
                    "reload_request_failed"
                }
                .into(),
                observed_at: now,
                max_age: IMPORT_LIMIT_SECS,
                source_epoch: "service".into(),
                last_success: reload.as_ref().ok().map(|_| now),
            });
            if reload.is_err() {
                result.error = Some("pair written; scheduler reload request failed".into());
            }
        }
        Ok(result)
    }

    /// Use a new config. The sessions close, because an address or a password can change.
    pub fn set_config(&mut self, cfg: Config) {
        self.cfg = cfg;
        self.health.clear(crate::system_health::Domain::Collection);
        self.health
            .clear(crate::system_health::Domain::Notification);
        self.sessions.clear();
        self.topology_sources.clear_sessions();
        self.topology_compiler = None;
        self.config_error = None;
    }

    /// A config reload failed. The old config stays, and `status.json` shows the error.
    pub fn config_failed(&mut self, error: String) {
        self.config_error = Some(error);
    }

    /// Run one cycle. `force` releases a guard hold (a manual sync).
    pub async fn cycle(&mut self, now: u64, force: bool) -> CycleSummary {
        self.last_cycle_started = Some(now);
        let start = Instant::now();
        let state_dir = Path::new(&self.cfg.runtime.state_dir).to_path_buf();
        let mut status = self.base_status(now);
        // Without the lock, another process owns the files: write nothing, not even the status.
        let topology = match self
            .lock()
            .and_then(|()| crate::topology_manager::runtime::topology_owns(&state_dir))
        {
            Ok(topology) => topology,
            Err(e) => {
                let error = Some(format!("{e:#}"));
                status.error = error.clone();
                return CycleSummary {
                    changed: false,
                    error,
                    status,
                };
            }
        };
        self.cycle_seq += 1;
        self.cycle_observed_at = now;
        status.cycle_seq = self.cycle_seq;
        let result = if topology {
            self.topology_cycle_inner(now, &state_dir, &mut status)
                .await
        } else {
            self.cycle_inner(now, force, &state_dir, &mut status).await
        };
        let (changed, error) = match result {
            Ok((changed, None)) => (changed, None),
            Ok((changed, Some(reload))) => {
                (changed, Some(format!("LibreQoS reload failed: {reload}")))
            }
            Err(e) => (status.files_changed, Some(format!("{e:#}"))),
        };
        status.files_changed = changed;
        status.error = error.clone();
        status.cycle_ms = start.elapsed().as_millis() as u64;
        let mut summary = CycleSummary {
            changed,
            error,
            status,
        };
        summary.status.next_scan_seconds = summary.next_sleep(&self.cfg.runtime);
        self.record_health(&summary.status);
        summary.status.health = Some(
            self.health.snapshot(
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs(),
                None,
            ),
        );
        self.write_status(summary.status.clone());
        summary
    }

    async fn native_diagnostics(
        &self,
        now: u64,
        state_dir: &Path,
        status: &mut Status,
    ) -> Option<crate::probe::Probe> {
        let (native, native_new) = crate::probe::status(&self.cfg.libreqos, state_dir, now).await;
        status.native = native.as_ref().map_or(String::new(), |p| p.summary());
        status.native_ready_new = native_new;
        if native.is_some() {
            status.bridge = crate::bridge::read().await;
        }
        native
    }

    async fn cycle_inner(
        &mut self,
        now: u64,
        force: bool,
        state_dir: &Path,
        status: &mut Status,
    ) -> anyhow::Result<(bool, Option<String>)> {
        // The target can change between cycles (an upgrade), so select it each cycle.
        let target = crate::detect::resolve(&self.cfg.libreqos)?.target;
        status.target = target_name(target).into();
        let v3 = target != Target::V2Pair;
        // LibreQoS 3.0.7 compiles libreqos_network.json with 1 Mbps nodes. The probe runs again
        // after a LibreQoS upgrade; v3_network writes only when it passes.
        let native = self.native_diagnostics(now, state_dir, status).await;
        if target == Target::V3Network
            && self.cfg.runtime.mode == RuntimeMode::Apply
            && !native.as_ref().is_some_and(|p| p.ready())
        {
            anyhow::bail!(
                "v3_network: the native probe of LibreQoS did not pass ({}); nothing was written. Set [topology] source = \"shaped_devices\" in {}",
                native.map_or(
                    "it cannot run: LibreQoS v3 and its Python binding are necessary".into(),
                    |p| p.summary()
                ),
                self.cfg.libreqos.v3_config
            );
        }
        let Inputs {
            live,
            state,
            lqos_dir,
        } = cycle_inputs(&self.cfg, now)?;

        // Read all routers at the same time. Each task owns its session and gives it back.
        // A router without a subscriber protocol (a WAN-only core) has no legacy read.
        let mut tasks = Vec::new();
        for router in self.cfg.routers.clone() {
            if !router.reads_subscribers() {
                continue;
            }
            let mut session = self.sessions.remove(&router.name).flatten();
            let (conn, access) = (self.connector.clone(), self.cfg.router_access.clone());
            let name = router.name.clone();
            tasks.push((
                name,
                tokio::spawn(async move {
                    let t = Instant::now();
                    let out = read_router(&*conn, &mut session, &router, &access).await;
                    (out, session, t.elapsed().as_millis() as u64)
                }),
            ));
        }
        let mut outcomes = HashMap::new();
        for (name, task) in tasks {
            // A task that panics fails only its own router.
            let (out, session, ms) = task.await.unwrap_or_else(|e| {
                (
                    Err(crate::routeros::ReadError::Protocol(format!(
                        "the read task stopped: {e}"
                    ))),
                    None,
                    0,
                )
            });
            self.sessions.insert(name.clone(), session);
            outcomes.insert(name, (out, ms));
        }
        let mut routers = Vec::new();
        let mut memos = BTreeMap::new();
        for router in self.cfg.routers.iter().filter(|r| r.reads_subscribers()) {
            let (out, ms) = outcomes
                .remove(&router.name)
                .expect("one outcome for each router");
            let memo = state
                .routers
                .get(&router.name)
                .cloned()
                .unwrap_or(RouterMemo {
                    unreachable_since: None,
                });
            let (result, memo, error) = classify(out, &memo, now, &self.cfg.router_access);
            let mut rows = BTreeMap::new();
            if let RouterResult::Online(snap) = &result {
                let tables = [
                    &snap.ppp_secret,
                    &snap.ppp_active,
                    &snap.ppp_profile,
                    &snap.hotspot_active,
                    &snap.dhcp_server,
                    &snap.dhcp_lease,
                ];
                for ((path, _), t) in TABLES.iter().zip(tables) {
                    rows.insert(path.to_string(), t.len());
                }
                if let Some(users) = &snap.hotspot_user {
                    rows.insert("/ip/hotspot/user".into(), users.len());
                }
            }
            status.routers.push(RouterStatus {
                name: router.name.clone(),
                state: state_name(&result),
                error,
                rows,
                read_ms: ms,
            });
            memos.insert(router.name.clone(), memo);
            routers.push((router.name.clone(), result));
        }
        let mut collection =
            crate::collection::Report::collect(&self.cfg, &routers, self.collection.as_ref(), now);
        collection.cycle_seq = self.cycle_seq;
        if let Err(error) = crate::collection::publish(state_dir, &collection) {
            eprintln!("warning: cannot write collection.json: {error:#}");
        }
        // The web router inventory: config, catalog WAN roles, and this collection. No router I/O.
        let catalog = self.api_document().ok().flatten().and_then(|d| d.catalog);
        let inventory = crate::router_view::project(
            &self.cfg,
            catalog.as_ref(),
            Some(&collection),
            Some(&state),
            &self.config_digest,
            now,
        );
        if let Err(error) = crate::router_view::publish(state_dir, &inventory) {
            eprintln!("warning: cannot write routers.json: {error:#}");
        }
        self.collection_config_digest = self.config_digest.clone();
        self.collection = Some(collection);
        self.record_live_groups(now).await;
        self.refresh_wan_discovery(now).await;
        self.publish_web_view(now);
        // A cache for the Settings preview (Plan 4b). A write error never stops the cycle.
        if let Err(e) = write_snapshot(state_dir, &routers, &status.routers, now) {
            eprintln!("warning: cannot write snapshot.json: {e:#}");
        }

        let id_len = self.cfg.runtime.id_length;
        let input = CycleInput {
            config: &self.cfg,
            state: &state,
            static_rows: &live.static_rows,
            routers,
            current_network: live.network.clone(),
            v3,
            force,
            now,
        };
        let mut out = assemble(input, &mut || random_id(id_len))?;
        out.state.routers = memos;
        let target_dir = match self.cfg.runtime.mode {
            RuntimeMode::Shadow => state_dir.join("candidate"),
            RuntimeMode::Apply => lqos_dir.clone(),
        };
        // The files of the target, and their text.
        let files: Vec<(std::path::PathBuf, String)> = match target {
            Target::V3Network => {
                let (text, alerts) = render_libreqos_network(&out.rows, &out.network)?;
                out.report.alerts.extend(alerts);
                vec![(target_dir.join(&self.cfg.libreqos.v3_network_file), text)]
            }
            _ => {
                let cols = if v3 { Columns::V3 } else { Columns::V2 };
                // The nodes first: each Parent Node of the new CSV is then already in network.json.
                vec![
                    (
                        target_dir.join("network.json"),
                        render_network_json(&out.network),
                    ),
                    (
                        target_dir.join("ShapedDevices.csv"),
                        render_csv(&out.rows, cols),
                    ),
                ]
            }
        };
        status.report = ReportSummary::from_report(&out.report, &out.rows, &state);
        // LibreQoS.py --updateonly needs the last loaded file of a full run. A new LibreQoS has
        // none, so the reload is a full run until LibreQoS writes that file.
        let full = !Path::new(&self.cfg.libreqos.state_directory)
            .join("shaping/ShapedDevices.lastLoaded.csv")
            .exists();

        // The state first: if the process stops here, the next cycle gives the same files.
        files::write_if_changed(
            &state_dir.join("state.json"),
            out.state.to_json().as_bytes(),
        )?;
        let differs = |p: &Path, b: &str| std::fs::read(p).map_or(true, |old| old != b.as_bytes());
        let changed = files.iter().any(|(p, t)| differs(p, t));
        if changed && self.cfg.runtime.mode == RuntimeMode::Apply {
            // shortcut: the backup folder name is Unix seconds (12 digits, so it sorts); a UTC date name needs a date crate.
            let paths: Vec<&Path> = files.iter().map(|(p, _)| p.as_path()).collect();
            files::backup(
                &state_dir.join("applied"),
                &paths,
                &format!("{now:012}"),
                self.cfg.runtime.keep_backups,
            )?;
        }
        self.record_health(status);
        if target == Target::V3Pair && self.cfg.runtime.mode == RuntimeMode::Apply {
            if let Some(tracker) = &self.write_tracker {
                let mut tracker = tracker.lock().unwrap_or_else(|p| p.into_inner());
                let result = tracker.write(
                    &files,
                    crate::write_receipt::Meta {
                        topology: None,
                        cycle_seq: self.cycle_seq,
                        config_digest: self.config_digest.clone(),
                        notice: Some(crate::notify::outbox::Frozen::from_status(
                            &self.cfg.telegram,
                            status,
                        )),
                    },
                    now,
                    &self.health,
                );
                status.files_changed = result.changed;
                if let Some(error) = result.tracking_error {
                    eprintln!("write receipt storage: {error}");
                }
                if let Some(error) = result.error {
                    anyhow::bail!("{error}");
                }
            } else {
                for (p, t) in &files {
                    status.files_changed |= files::write_if_changed(p, t.as_bytes())?;
                }
            }
        } else {
            for (p, t) in &files {
                status.files_changed |= files::write_if_changed(p, t.as_bytes())?;
            }
        }
        // A failed reload stays pending until it works, also when the files do not change again.
        let pending = state_dir.join("reload-pending");
        let mut reload_error = None;
        if v3 {
            // LibreQoS v3: LibreQoS.py --updateonly does nothing. lqosd watches libreqos_network.json;
            // the scheduler imports the pair (below).
            if self.cfg.runtime.mode == RuntimeMode::Apply {
                status.reload = if target == Target::V3Pair {
                    "no change"
                } else {
                    "lqosd watches the files"
                }
                .into();
            }
            if pending.exists() {
                std::fs::remove_file(&pending)?;
            }
            // The v3 scheduler imports the pair later. Check each cycle that the import has our files.
            let import = state_dir.join("import-pending");
            if target == Target::V3Pair && self.cfg.runtime.mode == RuntimeMode::Apply {
                let lqos_state = Path::new(&self.cfg.libreqos.state_directory);
                if changed {
                    files::write_if_changed(&import, now.to_string().as_bytes())?;
                    // The reload request of the LibreQoS UI button: the scheduler imports the pair
                    // in about 1 s. Without it, the import waits for the 30-minute job.
                    if let Err(e) = files::request_v3_import(lqos_state, now, &random_id(16)) {
                        eprintln!("warning: cannot write scheduler reload request: {e:#}");
                    }
                }
                if import.exists() {
                    // files[0] is network.json for the pair targets.
                    if files::imported(lqos_state, &files[0].1, &out.rows) {
                        status.reload = "imported by LibreQoS".into();
                        if let Some(tracker) = &self.write_tracker {
                            let mut tracker = tracker.lock().unwrap_or_else(|p| p.into_inner());
                            if tracker.current().is_some_and(|r| r.phase.pending())
                                && let Err(error) = tracker.transition(
                                    crate::system_health::Phase::ImportConfirmed,
                                    now,
                                    &self.health,
                                )
                            {
                                eprintln!("write import receipt: {error:#}");
                            }
                        }
                        std::fs::remove_file(&import)?;
                    } else {
                        // An unreadable time starts the wait again.
                        let since = std::fs::read_to_string(&import)
                            .ok()
                            .and_then(|t| t.trim().parse::<u64>().ok())
                            .unwrap_or(now);
                        let waited = now.saturating_sub(since);
                        status.reload = format!("waiting for the LibreQoS import ({waited} s)");
                        if waited >= IMPORT_LIMIT_SECS {
                            let e = format!(
                                "LibreQoS did not import the new files in {} min; examine the libreqos-scheduler log",
                                waited / 60
                            );
                            status.reload = e.clone();
                            reload_error = Some(e);
                            if let Some(tracker) = &self.write_tracker {
                                let mut tracker = tracker.lock().unwrap_or_else(|p| p.into_inner());
                                if tracker.current().is_some_and(|r| r.phase.pending())
                                    && let Err(error) = tracker.transition(
                                        crate::system_health::Phase::VerifyTimeout,
                                        now,
                                        &self.health,
                                    )
                                {
                                    eprintln!("write timeout receipt: {error:#}");
                                }
                            }
                        }
                    }
                }
            } else if import.exists() {
                std::fs::remove_file(&import)?;
            }
        } else if self.cfg.runtime.mode == RuntimeMode::Apply && (changed || pending.exists()) {
            let l = &self.cfg.libreqos;
            let args = if full { &l.v2_first_args } else { &l.v2_args };
            match files::run_reload(&l.v2_command, args, Duration::from_secs(l.v2_timeout)).await {
                Ok(()) => {
                    status.reload = if full { "ok (first full run)" } else { "ok" }.into();
                    if pending.exists() {
                        std::fs::remove_file(&pending)?;
                    }
                }
                Err(e) => {
                    status.reload = e.clone();
                    files::write_if_changed(&pending, e.as_bytes())?;
                    reload_error = Some(e);
                }
            }
        }
        Ok((changed, reload_error))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{Config, RouterAccess, RouterCfg};
    use crate::model::RosRow;
    use crate::routeros::{Connector, ReadError, RouterSession};
    use crate::status::Trigger;
    use std::collections::{BTreeMap, HashSet};
    use std::path::{Path, PathBuf};
    use std::sync::{Arc, Mutex};

    /// Rows for each router and path. A router in `failing` gives a trap on each read.
    #[derive(Default)]
    struct Fake {
        rows: Mutex<BTreeMap<String, BTreeMap<String, Vec<RosRow>>>>,
        failing: Mutex<HashSet<String>>,
        panicking: Mutex<HashSet<String>>,
        /// A router in `stalling` never finishes its connection.
        stalling: Mutex<HashSet<String>>,
        reads: std::sync::atomic::AtomicUsize,
    }
    struct FakeSession {
        router: String,
        fake: Arc<Fake>,
    }
    #[derive(Clone)]
    struct FakeConnector(Arc<Fake>);

    impl Connector for FakeConnector {
        type Session = FakeSession;
        fn connect(
            &self,
            r: &RouterCfg,
            _a: &RouterAccess,
        ) -> impl Future<Output = Result<FakeSession, ReadError>> + Send {
            let s = FakeSession {
                router: r.name.clone(),
                fake: self.0.clone(),
            };
            let boom = self.0.panicking.lock().unwrap().contains(&r.name);
            let stall = self.0.stalling.lock().unwrap().contains(&r.name);
            async move {
                assert!(!boom, "test panic in a router task");
                if stall {
                    tokio::time::sleep(Duration::from_secs(3600)).await;
                }
                Ok(s)
            }
        }
    }
    impl RouterSession for FakeSession {
        fn print(
            &mut self,
            path: &str,
            _p: &str,
        ) -> impl Future<Output = Result<Vec<RosRow>, ReadError>> + Send {
            self.fake
                .reads
                .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            let fail = self.fake.failing.lock().unwrap().contains(&self.router);
            let rows = self
                .fake
                .rows
                .lock()
                .unwrap()
                .get(&self.router)
                .and_then(|m| m.get(path))
                .cloned()
                .unwrap_or_default();
            async move {
                if fail {
                    Err(ReadError::Trap("down".into()))
                } else {
                    Ok(rows)
                }
            }
        }
    }

    fn row(kv: &[(&str, &str)]) -> RosRow {
        kv.iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect()
    }

    struct Env {
        _dir: tempfile::TempDir,
        state: PathBuf,
        lqos: PathBuf,
        fake: Arc<Fake>,
    }

    fn env() -> Env {
        let dir = tempfile::tempdir().unwrap();
        let (state, lqos) = (dir.path().join("state"), dir.path().join("lqos"));
        std::fs::create_dir_all(&lqos).unwrap();
        let fake = Arc::new(Fake::default());
        Env {
            _dir: dir,
            state,
            lqos,
            fake,
        }
    }

    fn users(e: &Env, names: &[(&str, &str)]) {
        let mut m = BTreeMap::new();
        m.insert(
            "/ppp/secret".to_string(),
            names
                .iter()
                .map(|(n, _)| row(&[("name", n), ("profile", "HS")]))
                .collect(),
        );
        m.insert(
            "/ppp/active".to_string(),
            names
                .iter()
                .map(|(n, ip)| row(&[("name", n), ("address", ip)]))
                .collect(),
        );
        m.insert(
            "/ppp/profile".to_string(),
            vec![row(&[("name", "HS"), ("rate-limit", "5M/20M")])],
        );
        e.fake.rows.lock().unwrap().insert("R1".into(), m);
    }

    fn cfg(e: &Env, mode: &str, extra: &str) -> Config {
        Config::from_toml_str(&format!(
            "[runtime]\nmode=\"{mode}\"\nstate_dir=\"{1}\"\n[libreqos]\ntarget=\"v2_pair\"\ndpkg_status=\"{0}/no-dpkg-status\"\ndirectory=\"{0}\"\nstate_directory=\"{0}/lqstate\"\nv2_command=\"/bin/sh\"\nv2_args=[\"-c\",\"touch {0}/reloaded\"]\nv2_first_args=[\"-c\",\"touch {0}/full && mkdir -p {0}/lqstate/shaping && touch {0}/lqstate/shaping/ShapedDevices.lastLoaded.csv\"]\n\
             [network]\nmode=\"root_promotion\"\n[safety]\nenabled=false\n{extra}\n\
             [[routers]]\nname=\"R1\"\naddress=\"192.0.2.1\"\nusername=\"u\"\npassword_file=\"/x\"\n[routers.pppoe]\nenabled=true\nper_plan_node=true\n",
            e.lqos.display(),
            e.state.display(),
        ))
        .unwrap()
    }

    /// A LibreQoS that did a full run before: it has its last loaded file, so a reload uses --updateonly.
    fn live_csv(e: &Env) {
        std::fs::write(
            e.lqos.join("ShapedDevices.csv"),
            "Circuit ID,Circuit Name\n",
        )
        .unwrap();
        let shaping = e.lqos.join("lqstate/shaping");
        std::fs::create_dir_all(&shaping).unwrap();
        std::fs::write(shaping.join("ShapedDevices.lastLoaded.csv"), "").unwrap();
    }

    fn runner(e: &Env, c: Config) -> Runner<FakeConnector> {
        Runner::new(c, Arc::new(FakeConnector(e.fake.clone())))
    }

    fn read(p: &Path) -> String {
        std::fs::read_to_string(p).unwrap_or_default()
    }

    #[tokio::test]
    async fn shadow_writes_candidates_only() {
        let e = env();
        std::fs::write(
            e.lqos.join("ShapedDevices.csv"),
            "Circuit ID,Circuit Name\n",
        )
        .unwrap();
        users(&e, &[("alice", "100.64.0.2")]);
        let mut r = runner(&e, cfg(&e, "shadow", ""));
        let s = r.cycle(1_000, false).await;
        assert_eq!(s.error, None);
        assert!(s.changed);
        assert!(read(&e.state.join("candidate/ShapedDevices.csv")).contains("alice"));
        assert_eq!(
            read(&e.lqos.join("ShapedDevices.csv")),
            "Circuit ID,Circuit Name\n"
        );
        let st: serde_json::Value =
            serde_json::from_str(&read(&e.state.join("status.json"))).unwrap();
        assert_eq!(st["routers"][0]["state"], "online");
        assert_eq!(st["reload"], "skipped");
        assert!(!e.lqos.join("reloaded").exists());
    }

    #[tokio::test]
    async fn apply_backs_up_writes_and_reloads() {
        let e = env();
        live_csv(&e);
        users(&e, &[("alice", "100.64.0.2")]);
        let s = runner(&e, cfg(&e, "apply", "")).cycle(1_000, false).await;
        assert_eq!((s.error, s.status.reload.as_str()), (None, "ok"));
        assert!(read(&e.lqos.join("ShapedDevices.csv")).contains("alice"));
        assert!(read(&e.lqos.join("network.json")).contains("HS-R1"));
        assert!(e.lqos.join("reloaded").exists());
        let backups: Vec<_> = std::fs::read_dir(e.state.join("applied"))
            .unwrap()
            .collect();
        assert_eq!(backups.len(), 1);
    }

    #[tokio::test]
    async fn a_fresh_libreqos_directory_gets_both_files() {
        // A new LibreQoS has no ShapedDevices.csv and no network.json.
        let e = env();
        assert!(!e.lqos.join("ShapedDevices.csv").exists());
        assert!(!e.lqos.join("network.json").exists());
        users(&e, &[("alice", "100.64.0.2")]);
        let s = runner(&e, cfg(&e, "apply", "")).cycle(1_000, false).await;
        assert_eq!(s.error, None);
        let csv = read(&e.lqos.join("ShapedDevices.csv"));
        assert!(csv.starts_with("Circuit ID,"), "{csv}");
        assert!(csv.contains("alice"), "{csv}");
        let net: serde_json::Value =
            serde_json::from_str(&read(&e.lqos.join("network.json"))).unwrap();
        assert!(net.get("HS-R1").is_some(), "{net}");
        // LibreQoS has no last loaded files yet, so the first reload is a full run.
        assert!(e.lqos.join("full").exists());
        assert!(!e.lqos.join("reloaded").exists());
    }

    #[tokio::test]
    async fn a_shipped_csv_without_a_full_run_still_gets_a_full_run() {
        let e = env();
        std::fs::write(
            e.lqos.join("ShapedDevices.csv"),
            "Circuit ID,Circuit Name\n",
        )
        .unwrap();
        users(&e, &[("alice", "100.64.0.2")]);
        let s = runner(&e, cfg(&e, "apply", "")).cycle(1_000, false).await;
        assert_eq!(s.error, None);
        assert!(e.lqos.join("full").exists());
    }

    #[tokio::test]
    async fn a_live_shaper_without_its_csv_gets_no_full_run() {
        // The CSV was removed by hand, but LibreQoS did a full run before.
        let e = env();
        live_csv(&e);
        std::fs::remove_file(e.lqos.join("ShapedDevices.csv")).unwrap();
        users(&e, &[("alice", "100.64.0.2")]);
        let s = runner(&e, cfg(&e, "apply", "")).cycle(1_000, false).await;
        assert_eq!((s.error, s.status.reload.as_str()), (None, "ok"));
        assert!(e.lqos.join("reloaded").exists());
        assert!(!e.lqos.join("full").exists());
    }

    #[tokio::test]
    async fn the_full_run_is_retried_until_it_works_then_updateonly_is_used() {
        let e = env();
        users(&e, &[("alice", "100.64.0.2")]);
        let mut c = cfg(&e, "apply", "");
        let first_args = c.libreqos.v2_first_args.clone();
        c.libreqos.v2_first_args = vec!["-c".into(), "exit 1".into()];
        let mut r = runner(&e, c.clone());
        let s = r.cycle(1_000, false).await;
        assert!(s.error.is_some());
        // The CSV exists now, but the full run did not work: the next cycle tries it again.
        c.libreqos.v2_first_args = first_args;
        r.set_config(c);
        let s = r.cycle(1_010, false).await;
        assert_eq!(s.error, None);
        assert_eq!(s.status.reload, "ok (first full run)");
        assert!(e.lqos.join("full").exists());
        assert!(!e.lqos.join("reloaded").exists());
        users(&e, &[("alice", "100.64.0.2"), ("bob", "100.64.0.3")]);
        let s = r.cycle(1_020, false).await;
        assert_eq!(s.error, None);
        assert!(e.lqos.join("reloaded").exists());
    }

    #[tokio::test]
    async fn second_cycle_without_change_writes_nothing() {
        let e = env();
        live_csv(&e);
        users(&e, &[("alice", "100.64.0.2")]);
        let mut r = runner(&e, cfg(&e, "apply", ""));
        r.cycle(1_000, false).await;
        std::fs::remove_file(e.lqos.join("reloaded")).unwrap();
        let s = r.cycle(1_025, false).await;
        assert_eq!((s.changed, s.status.reload.as_str()), (false, "skipped"));
        assert!(!e.lqos.join("reloaded").exists());
        assert_eq!(
            s.next_sleep(&r.cfg.runtime),
            r.cfg.runtime.scan_interval_idle
        );
    }

    #[tokio::test]
    async fn failed_router_keeps_rows_and_status_says_failed() {
        let e = env();
        users(&e, &[("alice", "100.64.0.2")]);
        let mut r = runner(
            &e,
            cfg(
                &e,
                "shadow",
                "[router_access]\nread_retries=0\nretry_delay=0\n",
            ),
        );
        r.cycle(1_000, false).await;
        e.fake.failing.lock().unwrap().insert("R1".into());
        let s = r.cycle(1_025, false).await;
        assert_eq!(s.error, None);
        assert!(read(&e.state.join("candidate/ShapedDevices.csv")).contains("alice"));
        assert_eq!(s.status.routers[0].state, "failed");
        assert!(
            s.status.routers[0]
                .error
                .as_deref()
                .unwrap_or_default()
                .contains("down")
        );
    }

    #[tokio::test]
    async fn a_router_without_protocols_is_not_read_and_does_not_hold_removals() {
        let e = env();
        users(&e, &[("alice", "100.64.0.2")]);
        let mut c = cfg(
            &e,
            "shadow",
            "[router_access]\nread_retries=0\nretry_delay=0\n",
        );
        let core = Config::from_toml_str(
            "[[routers]]\nname=\"Core\"\naddress=\"192.0.2.9\"\nusername=\"u\"\npassword_file=\"/x\"\n",
        )
        .unwrap()
        .routers
        .remove(0);
        assert!(!core.reads_subscribers());
        c.routers.push(core);
        // A read of Core would panic its task and show Core as failed.
        e.fake.panicking.lock().unwrap().insert("Core".into());
        e.fake.failing.lock().unwrap().insert("Core".into());
        let s = runner(&e, c).cycle(1_000, false).await;
        assert_eq!(s.error, None);
        let names: Vec<_> = s.status.routers.iter().map(|r| r.name.as_str()).collect();
        assert_eq!(names, ["R1"]);
        assert_eq!(s.status.routers[0].state, "online");
        let st: serde_json::Value =
            serde_json::from_str(&read(&e.state.join("status.json"))).unwrap();
        assert!(!st.to_string().contains("Core"), "{st}");
    }

    #[tokio::test]
    async fn a_connection_test_reads_the_identity_of_one_enabled_router_only() {
        let e = env();
        let mut c = cfg(&e, "shadow", "");
        let off = Config::from_toml_str(
            "[[routers]]\nname=\"Off\"\nenabled=false\naddress=\"192.0.2.9\"\nusername=\"u\"\npassword_file=\"/x\"\n",
        )
        .unwrap()
        .routers
        .remove(0);
        c.routers.push(off);
        e.fake.panicking.lock().unwrap().insert("Off".into());
        e.fake
            .rows
            .lock()
            .unwrap()
            .entry("R1".into())
            .or_default()
            .insert("/system/identity".into(), vec![row(&[("name", "edge-r1")])]);
        // The service writes its state folder in its first cycle; a test before it makes the folder.
        std::fs::create_dir_all(&e.state).unwrap();
        let mut r = runner(&e, c);
        let id = |n: &str| crate::topology_manager::policy_draft::router_id(n);
        let ok = r.api_router_test(&id("R1"), "operator-a").await.unwrap();
        assert_eq!(
            (ok["class"].as_str(), ok["identity"].as_str()),
            (Some("ok"), Some("edge-r1"))
        );
        let off = r.api_router_test(&id("Off"), "operator-a").await.unwrap();
        assert_eq!(
            off["class"], "disabled",
            "a disabled router gets no connection"
        );
        let none = r
            .api_router_test("lq3-ffffffffffffffff", "a")
            .await
            .unwrap();
        assert_eq!(none["class"], "not_found");
        e.fake.failing.lock().unwrap().insert("R1".into());
        let trap = r.api_router_test(&id("R1"), "operator-a").await.unwrap();
        assert_eq!(trap["class"], "trap");
        let log = read(&e.state.join(crate::router_edit::AUDIT));
        assert_eq!(
            log.lines().count(),
            3,
            "one audit line for each known router"
        );
        assert!(log.contains("\"action\":\"test\"") && log.contains("\"disabled\""));
    }

    #[tokio::test]
    async fn a_disabled_router_is_not_read_by_the_cycle_or_wan_discovery() {
        let e = env();
        users(&e, &[("alice", "100.64.0.2")]);
        let mut c = cfg(
            &e,
            "shadow",
            "[router_access]\nread_retries=0\nretry_delay=0\n",
        );
        let off = Config::from_toml_str(
            "[[routers]]\nname=\"Off\"\nenabled=false\naddress=\"192.0.2.9\"\nusername=\"u\"\npassword_file=\"/x\"\n[routers.pppoe]\nenabled=true\n",
        )
        .unwrap()
        .routers
        .remove(0);
        c.routers.push(off);
        // Any connection to Off panics its task and would show Off as failed.
        e.fake.panicking.lock().unwrap().insert("Off".into());
        let mut r = runner(&e, c);
        let s = r.cycle(1_000, false).await;
        assert_eq!(s.error, None);
        let names: Vec<_> = s.status.routers.iter().map(|r| r.name.as_str()).collect();
        assert_eq!(
            names,
            ["R1"],
            "a disabled router has no read and no held rows"
        );
        let inv: web_contract::routers::RouterInventory =
            serde_json::from_str(&read(&e.state.join("routers.json"))).unwrap();
        let off = inv.routers.iter().find(|r| r.name == "Off").unwrap();
        assert!(!off.enabled && off.collection.is_none());
        let found = r.api_wan_discover(&[]).await.unwrap();
        let off = found["routers"]
            .as_array()
            .unwrap()
            .iter()
            .find(|r| r["name"] == "Off")
            .unwrap();
        assert_eq!(off["state"], "failed");
        assert!(off["error"].as_str().unwrap().contains("disabled"), "{off}");
    }

    #[tokio::test]
    async fn wan_discovery_reads_only_the_selected_routers() {
        let e = env();
        let mut c = cfg(&e, "shadow", "");
        let core = Config::from_toml_str(
            "[[routers]]\nname=\"Core\"\naddress=\"192.0.2.9\"\nusername=\"u\"\npassword_file=\"/x\"\n",
        ).unwrap().routers.remove(0);
        c.routers.push(core);
        // Any connection to R1 panics its task and would show R1 as failed.
        e.fake.panicking.lock().unwrap().insert("R1".into());
        let mut r = runner(&e, c);
        let only = [crate::topology_manager::policy_draft::router_id("Core")];
        let found = r.api_wan_discover(&only).await.unwrap();
        let names: Vec<_> = found["routers"]
            .as_array()
            .unwrap()
            .iter()
            .map(|r| r["name"].clone())
            .collect();
        assert_eq!(names, [serde_json::json!("Core")]);
    }

    #[tokio::test]
    async fn wan_discovery_rereads_a_wan_only_router_without_subscriber_protocols() {
        let e = env();
        let mut c = cfg(&e, "shadow", "");
        let core = Config::from_toml_str(
            "[[routers]]\nname=\"Core\"\naddress=\"192.0.2.9\"\nusername=\"u\"\npassword_file=\"/x\"\n",
        ).unwrap().routers.remove(0);
        assert!(!core.reads_subscribers());
        c.routers.push(core);
        e.fake.rows.lock().unwrap().insert(
            "Core".into(),
            BTreeMap::from([(
                "/interface".into(),
                vec![row(&[
                    ("name", "pppoe-out1"),
                    ("type", "pppoe-out"),
                    ("running", "true"),
                ])],
            )]),
        );
        let mut r = runner(&e, c);
        let first = r.api_wan_discover(&[]).await.unwrap();
        assert_eq!(first["routers"].as_array().unwrap().len(), 2);
        assert_eq!(first["routers"][0]["candidates"], serde_json::json!([]));
        assert_eq!(first["routers"][1]["candidates"][0]["name"], "pppoe-out1");
        e.fake
            .rows
            .lock()
            .unwrap()
            .get_mut("Core")
            .unwrap()
            .get_mut("/interface")
            .unwrap()[0]
            .insert("name".into(), "pppoe-out2".into());
        let second = r.api_wan_discover(&[]).await.unwrap();
        assert_eq!(second["routers"][1]["candidates"][0]["name"], "pppoe-out2");
        // Each discovery reads `/interface` and `/ip/route` on two routers.
        assert_eq!(e.fake.reads.load(std::sync::atomic::Ordering::SeqCst), 8);
    }

    #[tokio::test]
    async fn each_cycle_publishes_the_router_inventory_without_reading_a_wan_only_router() {
        let e = env();
        users(&e, &[("alice", "100.64.0.2")]);
        let mut c = cfg(
            &e,
            "shadow",
            "[router_access]\nread_retries=0\nretry_delay=0\n",
        );
        let core = Config::from_toml_str(
            "[[routers]]\nname=\"Core\"\naddress=\"192.0.2.9\"\nusername=\"u\"\npassword_file=\"/x\"\n",
        )
        .unwrap()
        .routers
        .remove(0);
        c.routers.push(core);
        e.fake.panicking.lock().unwrap().insert("Core".into());
        let mut r = runner(&e, c);
        r.cycle(1_000, false).await;
        // The circuit counts come from the state at the start of a cycle: the second cycle has them.
        r.cycle(1_100, false).await;
        let inv: web_contract::routers::RouterInventory =
            serde_json::from_str(&read(&e.state.join("routers.json"))).unwrap();
        let names: Vec<_> = inv.routers.iter().map(|r| r.name.as_str()).collect();
        assert_eq!(names, ["R1", "Core"], "every configured router is listed");
        assert_eq!(inv.routers[0].circuits, Some(1));
        assert_eq!(inv.routers[1].circuits, Some(0));
        assert_eq!(inv.shaping.total_circuits, 1);
        assert!(inv.shaping.guard_max_count > 0 && inv.shaping.guard_held.is_none());
        assert_eq!(inv.routers[0].collection.as_ref().unwrap().state, "online");
        assert_eq!(inv.routers[1].collection, None);
        assert!(!read(&e.state.join("routers.json")).contains("password_file"));
    }

    #[tokio::test]
    async fn each_cycle_publishes_the_topology_view() {
        let e = env();
        users(&e, &[("alice", "100.64.0.2")]);
        let mut r = runner(&e, cfg(&e, "shadow", ""));
        r.cycle(1_000, false).await;
        let text = read(&e.state.join(web_contract::topology::CACHE));
        let view: web_contract::topology::TopologyView = serde_json::from_str(&text).unwrap();
        assert_eq!(view.updated, 1_000);
        assert_eq!(view.writer, "legacy");
        let alice = view
            .observations
            .iter()
            .find(|o| o.name == "alice")
            .unwrap();
        assert_eq!(
            (alice.kind.as_str(), alice.state.as_str()),
            ("ppp", "online")
        );
        assert_eq!(
            (alice.link.as_str(), alice.subscriber_id.as_ref()),
            ("none", None)
        );
        assert_eq!(view.collection[0].state, "online");
        assert!(!text.contains("password_file"));
    }

    #[test]
    fn reads_subscribers_is_false_only_with_all_protocols_disabled() {
        let base = "[[routers]]\nname=\"R\"\naddress=\"192.0.2.9\"\nusername=\"u\"\npassword_file=\"/x\"\n";
        let one = |extra: &str| {
            Config::from_toml_str(&format!("{base}{extra}"))
                .unwrap()
                .routers
                .remove(0)
                .reads_subscribers()
        };
        assert!(!one(""));
        assert!(one("[routers.pppoe]\nenabled=true\n"));
        assert!(one("[routers.dhcp]\nenabled=true\n"));
        assert!(one("[routers.hotspot]\nenabled=true\n"));
    }

    #[tokio::test]
    async fn assemble_error_writes_nothing() {
        let e = env();
        users(&e, &[("alice", "100.64.0.2")]);
        e.fake.rows.lock().unwrap().get_mut("R1").unwrap().insert(
            "/ip/hotspot/active".into(),
            vec![row(&[("user", "g"), ("address", "100.65.0.2")])],
        );
        let c = cfg(&e, "shadow", "");
        let mut c2 = c.clone();
        c2.routers[0].hotspot.enabled = true;
        let s = runner(&e, c2).cycle(1_000, false).await;
        assert!(
            s.error.as_deref().unwrap_or_default().contains("HS-R1"),
            "{:?}",
            s.error
        );
        assert!(!e.state.join("candidate/ShapedDevices.csv").exists());
        let st: serde_json::Value =
            serde_json::from_str(&read(&e.state.join("status.json"))).unwrap();
        assert!(st["error"].as_str().unwrap().contains("HS-R1"));
    }

    #[tokio::test]
    async fn restart_after_state_write_gives_same_files() {
        let a = env();
        users(&a, &[("alice", "100.64.0.2"), ("bob", "100.64.0.3")]);
        runner(&a, cfg(&a, "shadow", "")).cycle(1_000, false).await;
        // A crash after the state write: only state.json is there, not the LibreQoS files.
        let b = env();
        users(&b, &[("alice", "100.64.0.2"), ("bob", "100.64.0.3")]);
        std::fs::create_dir_all(&b.state).unwrap();
        std::fs::copy(a.state.join("state.json"), b.state.join("state.json")).unwrap();
        runner(&b, cfg(&b, "shadow", "")).cycle(1_025, false).await;
        assert_eq!(
            read(&a.state.join("candidate/ShapedDevices.csv")),
            read(&b.state.join("candidate/ShapedDevices.csv"))
        );
        assert_eq!(
            read(&a.state.join("candidate/network.json")),
            read(&b.state.join("candidate/network.json"))
        );
    }

    #[test]
    fn random_ids_have_the_length_and_charset() {
        let id = random_id(8);
        assert!(
            id.len() == 8
                && id
                    .chars()
                    .all(|c| c.is_ascii_uppercase() || c.is_ascii_digit()),
            "{id}"
        );
        assert_ne!(random_id(8), random_id(8));
    }

    #[tokio::test]
    async fn a_panicking_router_task_fails_only_that_router() {
        let e = env();
        users(&e, &[("alice", "100.64.0.2")]);
        let mut c = cfg(&e, "shadow", "");
        let mut r2 = c.routers[0].clone();
        r2.name = "R2".into();
        c.routers.push(r2);
        e.fake.panicking.lock().unwrap().insert("R2".into());
        let s = runner(&e, c).cycle(1_000, false).await;
        assert_eq!(s.error, None);
        let states: Vec<_> = s
            .status
            .routers
            .iter()
            .map(|r| (r.name.as_str(), r.state))
            .collect();
        assert_eq!(states, [("R1", "online"), ("R2", "failed")]);
    }

    #[tokio::test]
    async fn failed_reload_is_retried_until_it_works() {
        let e = env();
        live_csv(&e);
        users(&e, &[("alice", "100.64.0.2")]);
        let mut c = cfg(&e, "apply", "");
        c.libreqos.v2_command = "/bin/false".into();
        c.libreqos.v2_args = vec![];
        let mut r = runner(&e, c.clone());
        let s1 = r.cycle(1_000, false).await;
        assert!(
            s1.error.as_deref().unwrap_or_default().contains("reload"),
            "{:?}",
            s1.error
        );
        let s2 = r.cycle(1_025, false).await;
        assert!(!s2.changed);
        assert!(
            s2.error.as_deref().unwrap_or_default().contains("reload"),
            "the reload must run again: {:?}",
            s2.error
        );
        c.libreqos.v2_command = "/bin/true".into();
        r.set_config(c);
        let s3 = r.cycle(1_050, false).await;
        assert_eq!((s3.error, s3.status.reload.as_str()), (None, "ok"));
        let s4 = r.cycle(1_075, false).await;
        assert_eq!(s4.status.reload, "skipped");
    }

    #[tokio::test]
    async fn a_held_lock_stops_the_cycle_without_touching_status() {
        let e = env();
        users(&e, &[("alice", "100.64.0.2")]);
        let _other = crate::files::lock(&e.state).unwrap();
        let s = runner(&e, cfg(&e, "shadow", "")).cycle(1_000, false).await;
        assert!(
            s.error
                .as_deref()
                .unwrap_or_default()
                .contains("another lqosync3")
        );
        assert!(!e.state.join("status.json").exists());
    }

    #[tokio::test]
    async fn a_config_reload_error_shows_in_status_until_a_good_reload() {
        let e = env();
        users(&e, &[("alice", "100.64.0.2")]);
        let c = cfg(&e, "shadow", "");
        let mut r = runner(&e, c.clone());
        r.config_failed("bad key".into());
        let s = r.cycle(1_000, false).await;
        assert_eq!(s.error, None);
        let st: serde_json::Value =
            serde_json::from_str(&read(&e.state.join("status.json"))).unwrap();
        assert_eq!(st["config_error"], "bad key");
        r.set_config(c);
        let s = r.cycle(1_025, false).await;
        assert_eq!(s.status.config_error, None);
    }

    #[tokio::test]
    async fn network_json_is_written_before_the_csv() {
        let e = env();
        users(&e, &[("alice", "100.64.0.2")]);
        // A write of network.json that fails must leave the CSV as it was.
        std::fs::create_dir_all(e.state.join("candidate/network.json")).unwrap();
        let s = runner(&e, cfg(&e, "shadow", "")).cycle(1_000, false).await;
        assert!(s.error.is_some());
        assert!(!e.state.join("candidate/ShapedDevices.csv").exists());
    }

    #[tokio::test]
    async fn v3_pair_writes_15_columns_and_runs_no_reload() {
        let e = env();
        users(&e, &[("alice", "100.64.0.2")]);
        let mut c = cfg(&e, "apply", "");
        c.libreqos.target = crate::config::Target::V3Pair;
        let s = runner(&e, c).cycle(1_000, false).await;
        assert_eq!(s.error, None);
        let csv = read(&e.lqos.join("ShapedDevices.csv"));
        assert!(
            csv.lines()
                .next()
                .unwrap()
                .contains("Parent Node ID,Anchor Node ID"),
            "{csv}"
        );
        assert!(!e.lqos.join("reloaded").exists());
        assert_eq!(s.status.target, "v3_pair");
        assert!(
            s.status
                .reload
                .starts_with("waiting for the LibreQoS import"),
            "{}",
            s.status.reload
        );
    }

    fn v3_apply(e: &Env) -> Config {
        let mut c = cfg(e, "apply", "");
        c.libreqos.target = crate::config::Target::V3Pair;
        c
    }

    /// Write the `topology_import.json` that LibreQoS makes from the live pair.
    fn libreqos_imports(e: &Env) {
        let net: serde_json::Value =
            serde_json::from_str(&read(&e.lqos.join("network.json"))).unwrap();
        let rows = crate::state::read_csv(&read(&e.lqos.join("ShapedDevices.csv"))).unwrap();
        let f = |s: &str| s.parse::<f64>().unwrap();
        let devices: Vec<_> = rows
            .iter()
            .map(|r| {
                serde_json::json!({
                    "circuit_id": r.circuit_id, "device_id": r.device_id,
                    "parent_node": r.parent_node, "parent_node_id": r.parent_node_id,
                    "circuit_name":r.circuit_name, "device_name":r.device_name, "mac":r.mac, "comment":r.comment,
                    "ipv4": r.ipv4.split(',').filter(|s| !s.is_empty()).map(|ip| (ip,32)).collect::<Vec<_>>(),
                    "ipv6":[], "download_min_mbps":f(&r.download_min), "upload_min_mbps":f(&r.upload_min),
                    "download_max_mbps": f(&r.download_max), "upload_max_mbps": f(&r.upload_max),
                })
            })
            .collect();
        let dir = e.lqos.join("lqstate/topology");
        std::fs::create_dir_all(&dir).unwrap();
        let v = serde_json::json!({"imported": {"compatibility_network_json": net, "shaped_devices": devices}});
        std::fs::write(dir.join("topology_import.json"), v.to_string()).unwrap();
    }

    #[tokio::test]
    async fn config_reload_does_not_refresh_cycle_or_collection_observations() {
        let e = env();
        users(&e, &[("alice", "100.64.0.2")]);
        let config = v3_apply(&e);
        let mut r = runner(&e, config.clone());
        r.set_source(Path::new("config.toml"), "old".into());
        assert!(r.cycle(1000, false).await.error.is_none());
        r.set_config(config);
        r.set_source(Path::new("config.toml"), "new".into());
        r.write_reload_status(1100);
        let report = r.health.snapshot(1100, None);
        assert_eq!(
            report
                .components
                .iter()
                .find(|c| c.component == "sync_cycle")
                .unwrap()
                .observed_at,
            1000
        );
        assert!(
            report
                .components
                .iter()
                .filter(|c| c.domain == crate::system_health::Domain::Collection)
                .all(|c| c.observed_at <= 1000)
        );
    }

    #[tokio::test]
    async fn v3_pair_waits_until_libreqos_imports_the_new_files() {
        let e = env();
        users(&e, &[("alice", "100.64.0.2")]);
        let mut r = runner(&e, v3_apply(&e));
        let s1 = r.cycle(1_000, false).await;
        assert_eq!(s1.error, None);
        assert!(
            s1.status.reload.starts_with("waiting"),
            "{}",
            s1.status.reload
        );
        // An old import (from before this change) is not the new files.
        let s2 = r.cycle(1_120, false).await;
        assert_eq!(s2.error, None);
        assert!(
            s2.status.reload.starts_with("waiting"),
            "{}",
            s2.status.reload
        );
        libreqos_imports(&e);
        let s3 = r.cycle(1_240, false).await;
        assert_eq!(
            (s3.error, s3.status.reload.as_str()),
            (None, "imported by LibreQoS")
        );
        assert!(!e.state.join("import-pending").exists());
        let s4 = r.cycle(1_360, false).await;
        assert_eq!(s4.status.reload, "no change");
    }

    #[tokio::test]
    async fn v3_pair_asks_the_scheduler_for_a_reload_only_when_the_files_change() {
        let e = env();
        users(&e, &[("alice", "100.64.0.2")]);
        let req = e.lqos.join("lqstate/scheduler/reload_request.json");
        let mut r = runner(&e, v3_apply(&e));
        r.cycle(1_000, false).await;
        let t: serde_json::Value = serde_json::from_str(&read(&req)).unwrap();
        assert_eq!(
            (&t["requested_at"], &t["requested_by"]),
            (&serde_json::json!(1_000), &serde_json::json!("lqosync3"))
        );
        let nonce = t["nonce"].as_str().unwrap().to_string();
        assert!(!nonce.is_empty());
        // The scheduler deletes the request. No change: no new request.
        std::fs::remove_file(&req).unwrap();
        r.cycle(1_120, false).await;
        assert!(!req.exists());
        // A change: a new request with a new nonce.
        users(&e, &[("alice", "100.64.0.2"), ("bob", "100.64.0.3")]);
        r.cycle(1_240, false).await;
        let t: serde_json::Value = serde_json::from_str(&read(&req)).unwrap();
        assert_eq!(t["requested_at"], 1_240);
        assert_ne!(t["nonce"].as_str().unwrap(), nonce);
    }

    #[tokio::test]
    async fn a_restart_completes_an_interrupted_pair_and_asks_for_its_import() {
        let e = env();
        users(&e, &[("alice", "100.64.0.2")]);
        let req = e.lqos.join("lqstate/scheduler/reload_request.json");
        let mut r = runner(&e, v3_apply(&e));
        r.cycle(1_000, false).await;
        drop(r);
        std::fs::remove_file(&req).unwrap();
        let _ = std::fs::remove_file(e.state.join("import-pending"));
        // Rewind to a crash after network.json: the CSV is old and its stage holds the new text.
        let csv = e.lqos.join("ShapedDevices.csv");
        let receipt_path = e.state.join("write-receipt.json");
        let mut receipt: serde_json::Value = serde_json::from_str(&read(&receipt_path)).unwrap();
        let id = receipt["write_id"].as_str().unwrap().to_string();
        let index = (0..2)
            .find(|&i| {
                receipt["files"][i]["path"]
                    .as_str()
                    .unwrap()
                    .ends_with("ShapedDevices.csv")
            })
            .unwrap();
        let new = read(&csv);
        std::fs::write(crate::files::stage_path(&csv, &id), &new).unwrap();
        std::fs::write(&csv, "old\n").unwrap();
        receipt["phase"] = "INTENT".into();
        receipt["files"][index]["outcome"] = "PENDING".into();
        crate::files::write_secret(&receipt_path, &serde_json::to_vec(&receipt).unwrap()).unwrap();
        let mut r = runner(&e, v3_apply(&e));
        r.lock().unwrap();
        assert_eq!(read(&csv), new, "the restart installs the staged CSV");
        let t: serde_json::Value = serde_json::from_str(&read(&req)).unwrap();
        assert_eq!(t["requested_by"], "lqosync3");
        assert!(e.state.join("import-pending").exists());
    }

    #[tokio::test]
    async fn v3_pair_in_shadow_mode_asks_for_no_reload() {
        let e = env();
        users(&e, &[("alice", "100.64.0.2")]);
        let mut c = v3_apply(&e);
        c.runtime.mode = RuntimeMode::Shadow;
        runner(&e, c).cycle(1_000, false).await;
        assert!(
            !e.lqos
                .join("lqstate/scheduler/reload_request.json")
                .exists()
        );
    }

    #[tokio::test]
    async fn v3_pair_reports_an_import_that_does_not_come() {
        let e = env();
        users(&e, &[("alice", "100.64.0.2")]);
        let mut r = runner(&e, v3_apply(&e));
        r.cycle(1_000, false).await;
        let s = r.cycle(1_000 + IMPORT_LIMIT_SECS, false).await;
        assert!(
            s.error
                .as_deref()
                .unwrap_or_default()
                .contains("LibreQoS did not import"),
            "{:?}",
            s.error
        );
        // The check stays on until LibreQoS imports the files.
        libreqos_imports(&e);
        let s = r.cycle(1_100 + IMPORT_LIMIT_SECS, false).await;
        assert_eq!(
            (s.error, s.status.reload.as_str()),
            (None, "imported by LibreQoS")
        );
    }

    #[tokio::test]
    async fn v3_pair_with_a_new_network_json_and_an_old_csv_is_repaired_by_the_next_cycle() {
        // A crash between the two renames leaves the new network.json and state.json with the old CSV.
        let e = env();
        users(&e, &[("alice", "100.64.0.2")]);
        let mut r = runner(&e, v3_apply(&e));
        r.cycle(1_000, false).await;
        let old_csv = read(&e.lqos.join("ShapedDevices.csv"));
        users(&e, &[("alice", "100.64.0.2"), ("bob", "100.64.0.3")]);
        r.cycle(1_120, false).await;
        let (new_csv, new_net) = (
            read(&e.lqos.join("ShapedDevices.csv")),
            read(&e.lqos.join("network.json")),
        );
        assert!(new_csv.contains("bob") && !old_csv.contains("bob"));
        std::fs::write(e.lqos.join("ShapedDevices.csv"), &old_csv).unwrap();
        drop(r); // the process stops

        let s = runner(&e, v3_apply(&e)).cycle(1_240, false).await;
        assert_eq!(s.error, None);
        assert!(s.changed);
        assert_eq!(read(&e.lqos.join("ShapedDevices.csv")), new_csv);
        assert_eq!(read(&e.lqos.join("network.json")), new_net);
        let r = &s.status.report;
        assert_eq!((r.added.len(), r.removed.len()), (0, 0), "{r:?}");
    }

    #[tokio::test]
    async fn v3_network_writes_only_the_json_and_reports_skipped_circuits() {
        let e = env();
        users(&e, &[("alice", "100.64.0.2")]);
        std::fs::write(
            e.lqos.join("ShapedDevices.csv"),
            "Circuit ID,Circuit Name,Device ID,Device Name,Parent Node,MAC,IPv4,IPv6,Download Min Mbps,Upload Min Mbps,Download Max Mbps,Upload Max Mbps,Comment\n\
             S1,office,SD1,office,,,10.9.9.9,,5,5,10,10,static\n",
        )
        .unwrap();
        let mut c = cfg(&e, "apply", "");
        c.libreqos.target = crate::config::Target::V3Network;
        native_probe(&e, &mut c, "ready");
        let before_csv = read(&e.lqos.join("ShapedDevices.csv"));
        let s = runner(&e, c).cycle(1_000, false).await;
        assert_eq!(s.error, None);
        assert_eq!(s.status.native, "ready");
        let v: serde_json::Value =
            serde_json::from_str(&read(&e.lqos.join("libreqos_network.json"))).unwrap();
        assert_eq!(v["schema_version"], 2);
        assert_eq!(v["circuits"].as_array().unwrap().len(), 1, "{v}");
        assert_eq!(v["circuits"][0]["display_name"], "alice");
        assert_eq!(
            read(&e.lqos.join("ShapedDevices.csv")),
            before_csv,
            "the pair is not written"
        );
        assert!(!e.lqos.join("network.json").exists());
        assert!(!e.lqos.join("reloaded").exists());
        assert!(
            s.status.report.alerts.iter().any(|a| a.contains("office")),
            "{:?}",
            s.status.report.alerts
        );
        assert_eq!(s.status.target, "v3_network");
    }

    /// A LibreQoS v3 with the binding, and a cached native probe with `state`.
    fn native_probe(e: &Env, c: &mut Config, state: &str) {
        let dpkg = e.lqos.join("dpkg-v3");
        std::fs::write(
            &dpkg,
            "Package: libreqos\nStatus: install ok installed\nVersion: 3.0.7.1\n",
        )
        .unwrap();
        let conf = e.lqos.join("lqos.conf");
        std::fs::write(&conf, "[topology]\ncompile_mode = \"full\"\n").unwrap();
        std::fs::write(e.lqos.join(crate::probe::BINDING), "x").unwrap();
        c.libreqos.dpkg_status = dpkg.display().to_string();
        c.libreqos.v3_config = conf.display().to_string();
        let p = crate::probe::Probe {
            key: crate::probe::key(&c.libreqos).unwrap(),
            state: state.into(),
            detail: if state == "ready" {
                ""
            } else {
                "node PROBE-SITE compiled to 1/1 Mbps"
            }
            .into(),
            at: 1,
        };
        std::fs::create_dir_all(&e.state).unwrap();
        std::fs::write(
            e.state.join(crate::probe::CACHE),
            serde_json::to_string(&p).unwrap(),
        )
        .unwrap();
    }

    #[tokio::test]
    async fn v3_network_writes_nothing_when_the_native_probe_is_blocked() {
        let e = env();
        users(&e, &[("alice", "100.64.0.2")]);
        let mut c = cfg(&e, "apply", "");
        c.libreqos.target = crate::config::Target::V3Network;
        native_probe(&e, &mut c, "blocked");
        let s = runner(&e, c).cycle(1_000, false).await;
        let err = s.error.unwrap_or_default();
        assert!(err.contains("native") && err.contains("1/1 Mbps"), "{err}");
        assert!(!e.lqos.join("libreqos_network.json").exists());
        assert!(
            s.status.native.starts_with("blocked"),
            "{}",
            s.status.native
        );
    }

    #[tokio::test]
    async fn v3_network_without_a_probe_writes_nothing() {
        // A fixed target without dpkg data: the probe cannot run, so the write is not safe.
        let e = env();
        users(&e, &[("alice", "100.64.0.2")]);
        let mut c = cfg(&e, "apply", "");
        c.libreqos.target = crate::config::Target::V3Network;
        let s = runner(&e, c).cycle(1_000, false).await;
        assert!(s.error.unwrap_or_default().contains("native"));
        assert!(!e.lqos.join("libreqos_network.json").exists());
    }

    #[tokio::test]
    async fn v3_network_in_shadow_mode_writes_the_candidate_when_blocked() {
        let e = env();
        users(&e, &[("alice", "100.64.0.2")]);
        let mut c = cfg(&e, "shadow", "");
        c.libreqos.target = crate::config::Target::V3Network;
        native_probe(&e, &mut c, "blocked");
        let s = runner(&e, c).cycle(1_000, false).await;
        assert_eq!(s.error, None);
        assert!(e.state.join("candidate/libreqos_network.json").exists());
    }

    #[tokio::test]
    async fn v3_pair_reports_the_native_probe_and_is_not_blocked() {
        let e = env();
        users(&e, &[("alice", "100.64.0.2")]);
        let mut c = v3_apply(&e);
        native_probe(&e, &mut c, "blocked");
        let s = runner(&e, c).cycle(1_000, false).await;
        assert_eq!(s.error, None);
        assert!(
            s.status.native.starts_with("blocked"),
            "{}",
            s.status.native
        );
        assert!(!s.status.native_ready_new);
    }

    #[tokio::test]
    async fn auto_on_v2_gives_the_v2_pair() {
        let e = env();
        live_csv(&e);
        users(&e, &[("alice", "100.64.0.2")]);
        let dpkg = e.lqos.join("dpkg-status");
        std::fs::write(
            &dpkg,
            "Package: libreqos\nStatus: install ok installed\nVersion: 2.2.1\n",
        )
        .unwrap();
        let mut c = cfg(&e, "apply", "");
        c.libreqos.target = crate::config::Target::Auto;
        c.libreqos.dpkg_status = dpkg.display().to_string();
        let s = runner(&e, c).cycle(1_000, false).await;
        assert_eq!((s.error, s.status.target.as_str()), (None, "v2_pair"));
        assert!(!read(&e.lqos.join("ShapedDevices.csv")).contains("Anchor Node ID"));
        assert!(e.lqos.join("reloaded").exists());
    }

    fn status(e: &Env) -> serde_json::Value {
        serde_json::from_str(&read(&e.state.join("status.json"))).unwrap()
    }

    #[tokio::test]
    async fn status_has_digest_seq_pid_trigger() {
        let e = env();
        users(&e, &[("alice", "100.64.0.2")]);
        let mut r = runner(&e, cfg(&e, "shadow", ""));
        r.set_source(Path::new("/etc/x.toml"), "abc".into());
        r.cycle(1_000, false).await;
        assert_eq!(status(&e)["trigger"], "start");
        r.trigger = Trigger::SyncNow;
        let s = r.cycle(1_001, true).await;
        assert_eq!(s.status.cycle_seq, 2);
        let st = status(&e);
        assert_eq!(st["version"], 3);
        assert_eq!(st["cycle_seq"], 2);
        assert_eq!(st["trigger"], "sync_now");
        assert_eq!(st["pid"], std::process::id());
        assert_eq!(st["config_digest"], "abc");
        assert_eq!(st["config_path"], "/etc/x.toml");
        assert_eq!(st["program_version"], crate::VERSION);
        assert_eq!(st["telegram"], "none");
    }

    #[tokio::test]
    async fn reload_status_keeps_the_cycle_seq() {
        let e = env();
        users(&e, &[("alice", "100.64.0.2")]);
        let mut r = runner(&e, cfg(&e, "shadow", ""));
        r.cycle(1_000, false).await;
        r.config_failed("x".into());
        r.write_reload_status(2_000);
        let st = status(&e);
        assert_eq!(st["trigger"], "reload");
        assert_eq!(st["config_error"], "x");
        assert_eq!(st["updated"], 2_000);
        assert_eq!(st["cycle_seq"], 1);
        assert_eq!(st["routers"][0]["state"], "online");
    }

    #[tokio::test]
    async fn reload_status_without_a_cycle() {
        let e = env();
        let mut r = runner(&e, cfg(&e, "shadow", ""));
        r.set_source(Path::new("/etc/x.toml"), "abc".into());
        r.write_reload_status(5);
        let st = status(&e);
        assert_eq!(st["trigger"], "reload");
        assert_eq!(st["cycle_seq"], 0);
        assert_eq!(st["config_digest"], "abc");
        assert_eq!(st["pid"], std::process::id());
    }

    #[tokio::test]
    async fn telegram_result_is_written_at_once() {
        let e = env();
        users(&e, &[("alice", "100.64.0.2")]);
        let mut r = runner(&e, cfg(&e, "shadow", ""));
        r.cycle(1_000, false).await;
        r.set_telegram("ok".into());
        assert_eq!(status(&e)["telegram"], "ok");
    }

    #[tokio::test]
    async fn snapshot_json_has_each_router_and_mode_0600() {
        use std::os::unix::fs::PermissionsExt;
        let e = env();
        users(&e, &[("alice", "100.64.0.2")]);
        std::fs::create_dir_all(&e.state).unwrap();
        let tmp = e.state.join("snapshot.json.tmp");
        std::fs::write(&tmp, "old").unwrap();
        std::fs::set_permissions(&tmp, std::fs::Permissions::from_mode(0o644)).unwrap();
        let mut c = cfg(&e, "shadow", "");
        let mut r2 = c.routers[0].clone();
        r2.name = "R2".into();
        c.routers.push(r2);
        e.fake.failing.lock().unwrap().insert("R2".into());
        runner(&e, c).cycle(1_000, false).await;
        let path = e.state.join("snapshot.json");
        let mode = std::fs::metadata(&path).unwrap().permissions().mode();
        assert_eq!(mode & 0o777, 0o600);
        let v: serde_json::Value = serde_json::from_str(&read(&path)).unwrap();
        assert_eq!(v["version"], 1);
        assert_eq!(v["taken"], 1_000);
        let rs = v["routers"].as_array().unwrap();
        assert_eq!(
            (rs[0]["name"].as_str(), rs[0]["state"].as_str()),
            (Some("R1"), Some("online"))
        );
        assert_eq!(rs[0]["read_at"], 1_000);
        assert!(rs[0]["read_ms"].is_u64());
        assert_eq!(rs[0]["snapshot"]["ppp_active"][0]["name"], "alice");
        let collection = crate::collection::read(&e.state.join(crate::collection::CACHE)).unwrap();
        assert_eq!(collection.updated, 1000);
        assert_eq!(collection.routers[0].sources["ppp"].online, Some(1));
        assert_eq!(collection.routers[1].state, "failed");
        assert!(collection.routers[1].sources["ppp"].online.is_none());
        assert_eq!(
            std::fs::metadata(e.state.join(crate::collection::CACHE))
                .unwrap()
                .permissions()
                .mode()
                & 0o777,
            0o644
        );

        assert_eq!(
            (rs[1]["state"].as_str(), rs[1]["snapshot"].is_null()),
            (Some("failed"), true)
        );
    }

    #[tokio::test]
    async fn snapshot_write_error_does_not_stop_the_cycle() {
        let e = env();
        users(&e, &[("alice", "100.64.0.2")]);
        std::fs::create_dir_all(e.state.join("snapshot.json.tmp")).unwrap();
        let s = runner(&e, cfg(&e, "shadow", "")).cycle(1_000, false).await;
        assert_eq!(s.error, None);
        assert!(read(&e.state.join("candidate/ShapedDevices.csv")).contains("alice"));
    }
    #[tokio::test]
    async fn a_changed_state_directory_cannot_use_the_previous_instance_lock() {
        let e = env();
        let mut r = runner(&e, cfg(&e, "shadow", ""));
        r.lock().unwrap();
        let changed = e._dir.path().join("other-state");
        std::fs::create_dir_all(&changed).unwrap();
        r.cfg.runtime.state_dir = changed.to_string_lossy().into_owned();
        let result = r.cycle(1_000, false).await;
        assert!(result.error.is_some());
        assert!(!changed.join("status.json").exists());
        assert!(!changed.join("candidate/ShapedDevices.csv").exists());
    }

    #[tokio::test]
    async fn activation_requires_enrollment_without_claiming_the_writer_or_reading_sources() {
        for yes in [false, true] {
            let e = env();
            source_registry(&e);
            let mut r = runner(&e, v3_apply(&e));
            let error = r
                .activate_topology(
                    Path::new("/nonexistent/report.json"),
                    Duration::from_secs(10),
                    yes,
                    false,
                )
                .await
                .err()
                .unwrap();
            assert!(error.to_string().contains("not enrolled"));
            assert!(!e.state.join("topology-manager/runtime-owner.json").exists());
            assert!(!e.lqos.join("ShapedDevices.csv").exists());
            assert_eq!(e.fake.reads.load(std::sync::atomic::Ordering::SeqCst), 0);
        }
    }

    #[tokio::test]
    async fn activation_cannot_bypass_the_running_services_instance_lease() {
        let e = env();
        source_registry(&e);
        let config = v3_apply(&e);
        let mut owner = runner(&e, config.clone());
        owner.lock().unwrap();
        let mut another = runner(&e, config);
        let error = another
            .activate_topology(
                Path::new("/nonexistent/report.json"),
                Duration::from_secs(10),
                true,
                false,
            )
            .await
            .err()
            .unwrap();
        assert!(error.to_string().contains("another lqosync3 is running"));
        assert!(!e.state.join("topology-manager/runtime-owner.json").exists());
    }

    #[tokio::test]
    async fn config_reload_discards_the_previous_topology_compiler_binding() {
        let e = env();
        let config = v3_apply(&e);
        let mut r = runner(&e, config.clone());
        let (_native, _, compiler) = crate::topology_manager::compiler::tests::fixture();
        r.topology_compiler = Some(compiler);
        r.set_config(config);
        assert!(r.topology_compiler.is_none());
    }

    #[tokio::test]
    async fn an_owned_topology_cycle_reports_its_failure_without_legacy_output() {
        let e = env();
        source_registry(&e);
        crate::topology_manager::runtime::claim(&e.state).unwrap();
        let mut r = runner(&e, v3_apply(&e));
        let mut doc = r.api_document().unwrap().unwrap();
        doc.export_ownership = Some(Default::default());
        r.test_commit(doc).unwrap();
        let summary = r.cycle(1_000, false).await;
        let error = summary.error.expect("the fixture has no native compiler");
        assert!(!error.contains("legacy fallback"), "{error}");
        assert_eq!(summary.status.cycle_seq, 1);
        let saved: serde_json::Value =
            serde_json::from_str(&read(&e.state.join("status.json"))).unwrap();
        assert_eq!(saved["updated"], 1_000);
        assert_eq!(saved["error"], error);
        assert!(!e.lqos.join("ShapedDevices.csv").exists());
        assert!(!e.state.join("candidate/ShapedDevices.csv").exists());
    }

    #[tokio::test]
    async fn an_owned_topology_cycle_keeps_native_diagnostics_on_publication_failure() {
        let e = env();
        source_registry(&e);
        crate::topology_manager::runtime::claim(&e.state).unwrap();
        let (_native, mut native_cfg, _) = crate::topology_manager::compiler::tests::fixture();
        native_cfg.target = Target::V3Pair;
        let dpkg = e._dir.path().join("dpkg-status");
        std::fs::write(
            &dpkg,
            "Package: libreqos\nStatus: install ok installed\nVersion: 3.0.7\n\n",
        )
        .unwrap();
        native_cfg.dpkg_status = dpkg.to_string_lossy().into();
        let key = crate::probe::key(&native_cfg).unwrap();
        crate::files::write_secret(
            &e.state.join(crate::probe::CACHE),
            &serde_json::to_vec(&crate::probe::Probe {
                key,
                state: "blocked".into(),
                detail: "fixture probe result".into(),
                at: 1_000,
            })
            .unwrap(),
        )
        .unwrap();
        let mut config = cfg(&e, "apply", "");
        config.libreqos = native_cfg;
        let mut r = runner(&e, config);
        let summary = r.cycle(1_000, false).await;
        assert!(summary.error.unwrap().contains("not enrolled"));
        assert_eq!(summary.status.native, "blocked: fixture probe result");
        let saved: serde_json::Value =
            serde_json::from_str(&read(&e.state.join("status.json"))).unwrap();
        assert_eq!(saved["native"], "blocked: fixture probe result");
    }

    #[tokio::test]
    async fn automatic_topology_publication_requires_enrollment_before_any_source_read() {
        let e = env();
        source_registry(&e);
        crate::topology_manager::runtime::claim(&e.state).unwrap();
        let mut r = runner(&e, v3_apply(&e));
        let summary = r.cycle(1_000, false).await;
        assert!(summary.error.unwrap().contains("not enrolled"));
        assert_eq!(e.fake.reads.load(std::sync::atomic::Ordering::SeqCst), 0);
        assert!(r.topology_compiler.is_none());
        assert!(!e.lqos.join("ShapedDevices.csv").exists());
    }

    #[tokio::test]
    async fn a_durable_topology_owner_prevents_legacy_fallback_after_restart() {
        let e = env();
        drop(crate::topology_manager::store::RegistryStore::open(&e.state).unwrap());
        crate::files::write_secret(
            &e.state.join("topology-manager/runtime-owner.json"),
            br#"{"schema_version":1,"writer":"topology"}"#,
        )
        .unwrap();
        let mut r = runner(&e, cfg(&e, "shadow", ""));
        let result = r.cycle(1_000, false).await;
        assert!(result.error.is_some());
        assert!(!e.state.join("candidate/ShapedDevices.csv").exists());
        assert!(e.state.join("status.json").exists());
        assert!(
            r.topology_compiler.is_none(),
            "shadow mode opens no compiler"
        );
        assert!(r.sessions.is_empty(), "shadow mode opens no legacy session");
    }

    #[tokio::test]
    async fn manager_publication_reuses_one_lock_tracker_and_durable_owner() {
        let e = env();
        let (_native, mut native_cfg, mut compiler) =
            crate::topology_manager::compiler::tests::fixture();
        native_cfg.target = Target::V3Pair;
        let pair = crate::topology_manager::publication::prepare(
            crate::topology_manager::publication::tests::input(1),
        )
        .unwrap();
        let compiled =
            crate::topology_manager::compiler::tests::compiled(&mut compiler, pair).await;
        let mut config = cfg(&e, "apply", "");
        config.libreqos = native_cfg.clone();
        let mut r = runner(&e, config.clone());
        r.lock().unwrap();
        let tracker = r.pending_tracker().unwrap();
        assert!(crate::topology_manager::store::RegistryStore::open(&e.state).is_err());
        let result = r.publish_compiled(compiled, 1000).unwrap();
        assert!(result.changed && result.error.is_none());
        assert!(Arc::ptr_eq(&tracker, &r.pending_tracker().unwrap()));
        let receipt = tracker.lock().unwrap().current().unwrap().clone();
        assert!(
            Path::new(&native_cfg.state_directory)
                .join("scheduler/reload_request.json")
                .exists()
        );
        assert!(receipt.meta.topology.as_ref().unwrap().compiler.is_some());
        crate::apply_evidence::prepare(&receipt).unwrap();
        // Activation claims the writer; then an uncompiled pair cannot publish.
        crate::topology_manager::runtime::claim(&e.state).unwrap();
        let mut current = crate::topology_manager::publication::tests::input(0);
        current.reconciliation.registry = r
            .topology_store
            .as_ref()
            .unwrap()
            .document()
            .unwrap()
            .clone();
        let unbound = crate::topology_manager::publication::prepare(current).unwrap();
        let (canonical, shaping) = crate::topology_manager::admission::tests::artifacts(&unbound);
        let unbound =
            crate::topology_manager::admission::check_compiled(unbound, &canonical, &shaping)
                .unwrap();
        assert!(
            unbound
                .publish(
                    r.topology_store.as_mut().unwrap(),
                    &mut tracker.lock().unwrap(),
                    crate::topology_manager::publication::PublishRequest {
                        directory: Path::new(&native_cfg.directory),
                        meta: crate::write_receipt::Meta {
                            topology: None,
                            cycle_seq: 1,
                            config_digest: "fixture".into(),
                            notice: None
                        },
                        now: 1001,
                        hub: &r.health
                    }
                )
                .is_err()
        );
        let output =
            std::fs::read(Path::new(&native_cfg.directory).join("ShapedDevices.csv")).unwrap();
        assert!(r.cycle(1001, false).await.error.is_some());
        drop(r);
        let mut restarted = runner(&e, config);
        assert!(restarted.cycle(1002, false).await.error.is_some());
        assert_eq!(
            std::fs::read(Path::new(&native_cfg.directory).join("ShapedDevices.csv")).unwrap(),
            output
        );
        std::fs::write(
            Path::new(&native_cfg.directory).join("liblqos_python.so"),
            b"changed",
        )
        .unwrap();
        assert!(crate::apply_evidence::prepare(&receipt).is_err());
    }
    #[tokio::test]
    async fn manager_rejects_shadow_and_registry_revision_conflicts_before_claim() {
        for shadow in [true, false] {
            let e = env();
            let (_native, mut native_cfg, mut compiler) =
                crate::topology_manager::compiler::tests::fixture();
            native_cfg.target = Target::V3Pair;
            if !shadow {
                let mut store =
                    crate::topology_manager::store::RegistryStore::open(&e.state).unwrap();
                let mut next = store.document().unwrap().clone();
                next.next_id += 1;
                store.commit(0, next).unwrap();
            }
            let pair = crate::topology_manager::publication::prepare(
                crate::topology_manager::publication::tests::input(1),
            )
            .unwrap();
            let compiled =
                crate::topology_manager::compiler::tests::compiled(&mut compiler, pair).await;
            let mut config = cfg(&e, if shadow { "shadow" } else { "apply" }, "");
            config.libreqos = native_cfg.clone();
            let mut r = runner(&e, config);
            assert!(r.publish_compiled(compiled, 1000).is_err());
            assert!(!e.state.join("topology-manager/runtime-owner.json").exists());
            assert!(
                !Path::new(&native_cfg.directory)
                    .join("ShapedDevices.csv")
                    .exists()
            );
        }
    }
    #[tokio::test]
    async fn manager_reload_failure_keeps_ownership_and_retries_without_rewriting_the_pair() {
        let e = env();
        let (_native, mut native_cfg, mut compiler) =
            crate::topology_manager::compiler::tests::fixture();
        native_cfg.target = Target::V3Pair;
        let blocked = Path::new(&native_cfg.state_directory).join("scheduler");
        std::fs::write(&blocked, b"blocked").unwrap();
        let pair = crate::topology_manager::publication::prepare(
            crate::topology_manager::publication::tests::input(1),
        )
        .unwrap();
        let bound = crate::topology_manager::compiler::tests::compiled(&mut compiler, pair).await;
        let mut config = cfg(&e, "apply", "");
        config.libreqos = native_cfg.clone();
        let mut r = runner(&e, config);
        let result = r.publish_compiled(bound, 1000).unwrap();
        assert!(result.changed && result.error.is_some());
        // Only activation claims the writer; a publication does not.
        assert!(!crate::topology_manager::runtime::topology_owns(&e.state).unwrap());
        let tracker = r.pending_tracker().unwrap();
        let first = tracker.lock().unwrap().current().unwrap().clone();
        assert!(first.phase.pending());
        std::fs::remove_file(blocked).unwrap();
        let mut current = crate::topology_manager::publication::tests::input(1);
        current.reconciliation.generation = 2;
        current.reconciliation.registry = r
            .topology_store
            .as_ref()
            .unwrap()
            .document()
            .unwrap()
            .clone();
        let pair = crate::topology_manager::publication::prepare(current).unwrap();
        let bound = crate::topology_manager::compiler::tests::compiled(&mut compiler, pair).await;
        let result = r.publish_compiled(bound, 1001).unwrap();
        assert!(!result.changed && result.error.is_none());
        assert_eq!(
            tracker.lock().unwrap().current().unwrap().write_id,
            first.write_id
        );
        let request: serde_json::Value = serde_json::from_slice(
            &std::fs::read(
                Path::new(&native_cfg.state_directory).join("scheduler/reload_request.json"),
            )
            .unwrap(),
        )
        .unwrap();
        assert_eq!(request["nonce"], first.write_id);
    }
    #[tokio::test]
    async fn corrupt_or_linked_writer_ownership_stops_legacy_output() {
        for linked in [false, true] {
            let e = env();
            drop(crate::topology_manager::store::RegistryStore::open(&e.state).unwrap());
            let marker = e.state.join("topology-manager/runtime-owner.json");
            if linked {
                std::os::unix::fs::symlink("/dev/null", &marker).unwrap();
            } else {
                std::fs::write(&marker, b"corrupt").unwrap();
            }
            assert!(
                runner(&e, cfg(&e, "shadow", ""))
                    .cycle(1000, false)
                    .await
                    .error
                    .is_some()
            );
            assert!(!e.state.join("candidate/ShapedDevices.csv").exists());
        }
    }
    #[tokio::test]
    async fn older_generations_cannot_replace_a_newer_manager_pair() {
        let e = env();
        let (_native, mut native_cfg, mut compiler) =
            crate::topology_manager::compiler::tests::fixture();
        native_cfg.target = Target::V3Pair;
        let mut source = crate::topology_manager::publication::tests::input(1);
        source.reconciliation.generation = 2;
        let pair = crate::topology_manager::publication::prepare(source).unwrap();
        let compiled =
            crate::topology_manager::compiler::tests::compiled(&mut compiler, pair).await;
        let mut config = cfg(&e, "apply", "");
        config.libreqos = native_cfg.clone();
        let mut r = runner(&e, config);
        r.publish_compiled(compiled, 1000).unwrap();
        let output =
            std::fs::read(Path::new(&native_cfg.directory).join("ShapedDevices.csv")).unwrap();
        let mut source = crate::topology_manager::publication::tests::input(1);
        source.reconciliation.registry = r
            .topology_store
            .as_ref()
            .unwrap()
            .document()
            .unwrap()
            .clone();
        source.reconciliation.active[0].endpoints[0].endpoint.ips =
            vec!["192.0.2.90".parse().unwrap()];
        let pair = crate::topology_manager::publication::prepare(source).unwrap();
        let compiled =
            crate::topology_manager::compiler::tests::compiled(&mut compiler, pair).await;
        assert!(r.publish_compiled(compiled, 1001).is_err());
        assert_eq!(
            std::fs::read(Path::new(&native_cfg.directory).join("ShapedDevices.csv")).unwrap(),
            output
        );
    }
    #[tokio::test]
    #[ignore = "requires explicit installed-binding runtime publication in temporary directories"]
    async fn installed_binding_runs_the_public_manager_api_without_live_writes() {
        let source = std::env::var("LQOSYNC3_RUNTIME_BINDING")
            .expect("set LQOSYNC3_RUNTIME_BINDING explicitly");
        let e = env();
        let (_native, mut native_cfg, old_compiler) =
            crate::topology_manager::compiler::tests::fixture();
        drop(old_compiler);
        std::fs::copy(
            Path::new(&source).join("liblqos_python.so"),
            Path::new(&native_cfg.directory).join("liblqos_python.so"),
        )
        .unwrap();
        let mut table: toml::Table = toml::from_str(
            &std::fs::read_to_string(Path::new(&source).join("lqos.example")).unwrap(),
        )
        .unwrap();
        table.insert("lqos_directory".into(), native_cfg.directory.clone().into());
        table.insert(
            "state_directory".into(),
            native_cfg.state_directory.clone().into(),
        );
        std::fs::write(&native_cfg.v3_config, toml::to_string(&table).unwrap()).unwrap();
        native_cfg.target = Target::V3Pair;
        let mut compiler = crate::topology_manager::compiler::ControlledCompiler::open(
            &native_cfg,
            &e.state.join("compiler-scratch"),
        )
        .unwrap();
        let mut config = cfg(&e, "apply", "");
        config.libreqos = native_cfg.clone();
        let mut r = runner(&e, config);
        let pair = crate::topology_manager::publication::prepare(
            crate::topology_manager::publication::tests::input(1),
        )
        .unwrap();
        let result = r.publish_topology(&mut compiler, pair, 1000).await.unwrap();
        assert!(result.changed && result.error.is_none());
        let mut current = crate::topology_manager::publication::tests::input(0);
        current.reconciliation.generation = 2;
        current.reconciliation.registry = r
            .topology_store
            .as_ref()
            .unwrap()
            .document()
            .unwrap()
            .clone();
        let pair = crate::topology_manager::publication::prepare(current).unwrap();
        let result = r.publish_topology(&mut compiler, pair, 1001).await.unwrap();
        assert!(result.changed && result.error.is_none());
        let receipt = r
            .pending_tracker()
            .unwrap()
            .lock()
            .unwrap()
            .current()
            .unwrap()
            .clone();
        crate::apply_evidence::prepare(&receipt).unwrap();
        let text =
            std::fs::read_to_string(Path::new(&native_cfg.directory).join("ShapedDevices.csv"))
                .unwrap();
        assert!(crate::state::read_csv(&text).unwrap().is_empty());
        assert!(r.cycle(1002, false).await.error.is_some());
        println!(
            "Public manager API preserves active and disconnected pairs in temporary directories; no live apply occurred."
        );
    }

    fn source_registry(e: &Env) {
        use crate::topology_manager::{
            identity::{ReferenceAuthority, SourceSelection},
            settings::SourceSettings,
            source_policy::{ProfileBinding, RateRead, SourcePolicy},
        };
        let mut registry = crate::topology_manager::publication::tests::input(0)
            .reconciliation
            .registry;
        registry.catalog.as_mut().unwrap().routers[1].name = "R1".into();
        let scope = registry
            .catalog
            .as_ref()
            .unwrap()
            .required_scopes()
            .unwrap()[0]
            .clone();
        registry.source_settings = Some(SourceSettings {
            selections: vec![SourceSelection {
                scope,
                service_names: vec![],
                authority: ReferenceAuthority::OperatorManagedComment,
                policy: SourcePolicy {
                    profiles: vec![ProfileBinding {
                        source_ref: "provider-profile-id".into(),
                        current_name: "gold".into(),
                        rate: RateRead::Disabled,
                    }],
                    ..Default::default()
                },
            }],
        });
        let mut store = crate::topology_manager::store::RegistryStore::open(&e.state).unwrap();
        store.commit(0, registry).unwrap();
        e.fake.rows.lock().unwrap().insert(
            "R1".into(),
            BTreeMap::from([
                (
                    "/ppp/secret".into(),
                    vec![row(&[
                        (".id", "*s"),
                        ("name", "alice"),
                        ("profile", "gold"),
                        ("disabled", "false"),
                        ("comment", "[lqosync-id:customer-1]"),
                    ])],
                ),
                (
                    "/ppp/profile".into(),
                    vec![row(&[
                        (".id", "*p"),
                        ("name", "gold"),
                        ("rate-limit", "10M/50M"),
                    ])],
                ),
                (
                    "/ppp/active".into(),
                    vec![row(&[
                        (".id", "*a"),
                        ("name", "alice"),
                        ("address", "192.0.2.1"),
                        ("service", "pppoe"),
                    ])],
                ),
            ]),
        );
    }

    #[tokio::test]
    async fn configured_source_preparation_reuses_the_runner_lock_without_publishing() {
        let e = env();
        source_registry(&e);
        let mut r = runner(&e, v3_apply(&e));
        let pair = r.prepare_topology(Duration::from_secs(10)).await.unwrap();
        assert_eq!(pair.rows().len(), 1);
        assert_eq!(pair.rows()[0].ipv4, "192.0.2.1");
        assert_eq!(pair.expected_revision(), 1);
        assert_eq!(
            r.topology_store
                .as_ref()
                .unwrap()
                .document()
                .unwrap()
                .subscribers
                .len(),
            0
        );
        assert!(crate::topology_manager::store::RegistryStore::open(&e.state).is_err());
        assert!(!e.lqos.join("ShapedDevices.csv").exists());
        assert!(!e.lqos.join("network.json").exists());
        assert!(!e.state.join("topology-manager/runtime-owner.json").exists());
    }

    #[tokio::test]
    async fn source_preparation_rejects_unsuitable_mode_or_target_before_source_reads() {
        for case in 0..2 {
            let e = env();
            source_registry(&e);
            let mut config = v3_apply(&e);
            if case == 0 {
                config.runtime.mode = RuntimeMode::Shadow;
            } else {
                config.libreqos.target = Target::V2Pair;
            }
            let mut r = runner(&e, config);
            assert!(r.prepare_topology(Duration::from_secs(10)).await.is_err());
            assert!(r.topology_store.is_none());
            assert!(!e.state.join("topology-manager/runtime-owner.json").exists());
        }
    }

    #[tokio::test]
    #[ignore = "requires explicit installed-binding configured source sync in temporary directories"]
    async fn installed_binding_runs_configured_source_sync_without_live_writes() {
        let source = std::env::var("LQOSYNC3_RUNTIME_BINDING")
            .expect("set LQOSYNC3_RUNTIME_BINDING explicitly");
        let e = env();
        source_registry(&e);
        let (_native, mut native_cfg, old_compiler) =
            crate::topology_manager::compiler::tests::fixture();
        drop(old_compiler);
        std::fs::copy(
            Path::new(&source).join("liblqos_python.so"),
            Path::new(&native_cfg.directory).join("liblqos_python.so"),
        )
        .unwrap();
        let mut table: toml::Table = toml::from_str(
            &std::fs::read_to_string(Path::new(&source).join("lqos.example")).unwrap(),
        )
        .unwrap();
        table.insert("lqos_directory".into(), native_cfg.directory.clone().into());
        table.insert(
            "state_directory".into(),
            native_cfg.state_directory.clone().into(),
        );
        std::fs::write(&native_cfg.v3_config, toml::to_string(&table).unwrap()).unwrap();
        native_cfg.target = Target::V3Pair;
        let mut compiler = crate::topology_manager::compiler::ControlledCompiler::open(
            &native_cfg,
            &e.state.join("compiler-scratch"),
        )
        .unwrap();
        let mut config = cfg(&e, "apply", "");
        config.libreqos = native_cfg.clone();
        let mut r = runner(&e, config.clone());
        let mut doc = r.api_document().unwrap().unwrap();
        doc.export_ownership = Some(Default::default());
        r.test_commit(doc).unwrap();
        let revision = r.api_document().unwrap().unwrap().revision;
        assert_eq!(r.api_document().unwrap().unwrap().revision, revision);
        assert!(!e.state.join("topology-manager/runtime-owner.json").exists());
        // Activation only claims the writer; the first sync writes the pair.
        crate::topology_manager::runtime::claim(&e.state).unwrap();
        r.cfg.exclude.ips.push("192.0.2.0/24".into());
        let filtered = r
            .sync_topology(&mut compiler, Duration::from_secs(10), 999)
            .await
            .unwrap();
        assert!(filtered.error.is_none());
        assert!(
            crate::state::read_csv(&read(
                &Path::new(&native_cfg.directory).join("ShapedDevices.csv")
            ))
            .unwrap()
            .is_empty()
        );
        r.cfg.exclude.ips.clear();
        let result = r
            .sync_topology(&mut compiler, Duration::from_secs(10), 1000)
            .await
            .unwrap();
        assert!(result.changed && result.error.is_none());
        let csv = Path::new(&native_cfg.directory).join("ShapedDevices.csv");
        let first = read(&csv);
        let id = crate::state::read_csv(&first).unwrap()[0]
            .circuit_id
            .clone();
        e.fake.failing.lock().unwrap().insert("R1".into());
        assert!(
            r.sync_topology(&mut compiler, Duration::from_secs(10), 1001)
                .await
                .is_err()
        );
        assert_eq!(read(&csv), first);
        e.fake.failing.lock().unwrap().clear();
        e.fake
            .rows
            .lock()
            .unwrap()
            .get_mut("R1")
            .unwrap()
            .insert("/ppp/active".into(), vec![]);
        drop(r);
        let mut r = runner(&e, config);
        let result = r
            .sync_topology(&mut compiler, Duration::from_secs(10), 1002)
            .await
            .unwrap();
        assert!(result.changed && result.error.is_none());
        assert!(crate::state::read_csv(&read(&csv)).unwrap().is_empty());
        assert!(
            r.pending_tracker()
                .unwrap()
                .lock()
                .unwrap()
                .current()
                .unwrap()
                .meta
                .topology
                .as_ref()
                .unwrap()
                .observation_generation
                > 1
        );
        e.fake.rows.lock().unwrap().get_mut("R1").unwrap().insert(
            "/ppp/active".into(),
            vec![row(&[
                (".id", "*b"),
                ("name", "alice"),
                ("address", "192.0.2.99"),
                ("caller-id", "aa:bb:cc:dd:ee:99"),
                ("service", "pppoe"),
            ])],
        );
        let result = r
            .sync_topology(&mut compiler, Duration::from_secs(10), 1003)
            .await
            .unwrap();
        assert!(result.changed && result.error.is_none());
        let rows = crate::state::read_csv(&read(&csv)).unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].circuit_id, id);
        assert_eq!(rows[0].ipv4, "192.0.2.99");
        assert_eq!(rows[0].mac, "AA:BB:CC:DD:EE:99");
        assert!(
            Path::new(&native_cfg.state_directory)
                .join("scheduler/reload_request.json")
                .exists()
        );
        let cycle = r.cycle(1004, false).await;
        assert!(cycle.error.is_none(), "{:?}", cycle.error);
        let collection: crate::collection::Report =
            serde_json::from_str(&read(&e.state.join("collection.json"))).unwrap();
        assert_eq!(collection.cycle_seq, cycle.status.cycle_seq);
        assert_eq!(collection.routers[0].subscribers.len(), 1);
        assert_eq!(collection.routers[0].subscribers[0].ipv4, "192.0.2.99");
        assert!(e.state.join("topology.json").exists());
        assert!(e.state.join("routers.json").exists());
        let routers: web_contract::routers::RouterInventory =
            serde_json::from_str(&read(&e.state.join("routers.json"))).unwrap();
        assert_eq!(routers.shaping.total_circuits, 1);
        assert_eq!(routers.routers[0].circuits, Some(1));
        let network = Path::new(&native_cfg.directory).join("network.json");
        let before = read(&network);
        let mut document = r.api_document().unwrap().unwrap();
        // Swap the two source instances, preserving one instance per source and parent.
        for node in &mut document.catalog.as_mut().unwrap().nodes {
            if node.kind == crate::topology_manager::layout::NodeKind::SubscriberGroup {
                node.parent = Some(
                    if node.parent.as_deref() == Some("wan1") {
                        "wan2"
                    } else {
                        "wan1"
                    }
                    .into(),
                );
            }
        }
        r.test_commit(document).unwrap();
        let moved = r.cycle(1005, false).await;
        assert!(moved.error.is_none(), "{:?}", moved.error);
        assert!(moved.changed);
        assert_ne!(
            read(&network),
            before,
            "the automatic cycle exports the confirmed hierarchy"
        );
        let accepted_csv = read(&csv);
        let exported = crate::state::read_csv(&accepted_csv).unwrap();
        assert_eq!(exported[0].circuit_id, id);
        e.fake.failing.lock().unwrap().insert("R1".into());
        let failed = r.cycle(1006, false).await;
        assert!(failed.error.is_some());
        assert_eq!(
            read(&csv),
            accepted_csv,
            "a failed batch cannot replace the live pair"
        );
        let kept: crate::collection::Report =
            serde_json::from_str(&read(&e.state.join("collection.json"))).unwrap();
        assert_eq!(
            kept.updated, 1005,
            "failed reads retain the last source timestamp"
        );
        assert_eq!(kept.routers[0].state, "failed");
        let failed_routers: web_contract::routers::RouterInventory =
            serde_json::from_str(&read(&e.state.join("routers.json"))).unwrap();
        assert_eq!(failed_routers.shaping.total_circuits, 1);
        assert_eq!(failed_routers.routers[0].circuits, Some(1));
        assert_eq!(
            failed_routers.routers[0].collection.as_ref().unwrap().state,
            "failed"
        );
        let web: web_contract::topology::TopologyView =
            serde_json::from_str(&read(&e.state.join("topology.json"))).unwrap();
        assert_eq!(web.observations[0].state, "stale");
        println!(
            "Configured source sync preserves identity, current endpoints, failure fencing, and restart generations in temporary directories."
        );
    }

    #[tokio::test]
    async fn a_filtered_pair_cannot_publish_after_its_filter_settings_change() {
        let e = env();
        source_registry(&e);
        let mut config = v3_apply(&e);
        config.exclude.ips.push("198.51.100.0/24".into());
        let mut r = runner(&e, config);
        let pair = r.prepare_topology(Duration::from_secs(10)).await.unwrap();
        let (_native, _cfg, mut compiler) = crate::topology_manager::compiler::tests::fixture();
        r.cfg.exclude.ips.clear();
        let error = r
            .publish_topology(&mut compiler, pair, 1000)
            .await
            .err()
            .unwrap();
        assert!(
            error
                .to_string()
                .contains("prepared source policies differ")
        );
        assert!(!e.lqos.join("ShapedDevices.csv").exists());
        assert!(!e.state.join("topology-manager/runtime-owner.json").exists());
    }
    mod api;
    mod canvas_shadow;
    mod enrollment;
    #[path = "../live_test_tests.rs"]
    mod live_test;
}
