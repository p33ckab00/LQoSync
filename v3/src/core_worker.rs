//! Fresh verification and bounded offline execution. No console or automatic enable caller exists.
use crate::{
    core_console::{self, Runtime},
    core_writer::{Batch, Evidence, Executor, Rows},
    core_writer_transport::{self, RouterTransport, Verifier},
    model::{Settings, Subscriber},
    normalizer,
    routeros::Inventory,
    sessions::{self, Session},
    wan_health::{self, HealthStatus},
};
use anyhow::{Context, Result, ensure};
use serde::Serialize;
use std::{
    collections::{BTreeMap, BTreeSet},
    future::Future,
    path::{Path, PathBuf},
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
};

#[derive(Clone, Default)]
pub struct Approval {
    enabled: Arc<AtomicBool>,
    expires_at: u64,
}
impl Approval {
    pub fn revoke(&self) {
        self.enabled.store(false, Ordering::SeqCst);
    }
    pub fn active(&self) -> bool {
        self.enabled.load(Ordering::SeqCst) && crate::orchestrator::now() <= self.expires_at
    }
}
/// Capture actual selected routers, not cached App inventory. Confirm settings after all parsing work.
pub trait Source {
    fn capture(&mut self) -> impl Future<Output = Result<Snapshot>>;
    fn confirm(&mut self, revision: &str) -> impl Future<Output = Result<()>>;
}
pub struct Snapshot {
    root: PathBuf,
    settings: Settings,
    inventories: BTreeMap<String, Arc<Inventory>>,
    runtime: Runtime,
}
impl Snapshot {
    /// Parse an initial complete observation. Synthetic protocol fixtures use this constructor too.
    pub fn from_inventories(
        settings: Settings,
        inventories: BTreeMap<String, Arc<Inventory>>,
        root: &Path,
        previous: &Runtime,
        now: u64,
    ) -> Result<Self> {
        let runtime = core_console::compute(
            &settings,
            &inventories,
            &BTreeMap::new(),
            root,
            previous,
            now,
        )?;
        Ok(Self {
            root: root.canonicalize()?,
            settings,
            inventories,
            runtime,
        })
    }
    pub fn runtime(&self) -> &Runtime {
        &self.runtime
    }
}
pub struct RouterSource {
    root: PathBuf,
    previous: Runtime,
}
impl RouterSource {
    pub fn new(root: &Path) -> Result<Self> {
        Ok(Self {
            root: root.canonicalize()?,
            previous: Runtime::default(),
        })
    }
    /// Collect a reconciliation round for review. The first probe sample cannot establish health.
    pub async fn preview(&mut self) -> Result<Snapshot> {
        self.collect(true).await
    }
    async fn collect(&mut self, reconcile: bool) -> Result<Snapshot> {
        let result = self.read(reconcile).await;
        if result.is_err() {
            self.previous = Runtime::default();
        }
        result
    }
    async fn settings(&self) -> Result<Settings> {
        let root = self.root.clone();
        tokio::task::spawn_blocking(move || crate::store::load_settings(&root)).await?
    }
    async fn read(&mut self, reconcile: bool) -> Result<Snapshot> {
        let settings = self.settings().await?;
        ensure!(
            core_console::enabled(&settings),
            "Core control is paused or disabled"
        );
        let mut selected: BTreeSet<String> = normalizer::selected_routers(&settings)
            .into_iter()
            .map(str::to_string)
            .collect();
        selected.extend(
            settings
                .core_lists
                .bindings
                .iter()
                .map(|b| b.router.clone()),
        );
        if let Some(c) = &settings.dynamic {
            selected.extend(c.wans.iter().map(|w| w.router.clone()));
        }
        let mut pending = settings
            .routers
            .iter()
            .filter(|r| selected.contains(&r.id))
            .cloned();
        let mut tasks = tokio::task::JoinSet::new();
        let mut inventories = BTreeMap::new();
        // Refill one slot after each result. A failed collection drops and aborts the remaining reads.
        loop {
            while tasks.len() < 4 {
                let Some(router) = pending.next() else {
                    break;
                };
                let root = self.root.clone();
                let health = core_console::monitor_router(&settings, &router.id);
                tasks.spawn(async move {
                    let inv = if health {
                        crate::routeros::inspect_health_from(&router, &root).await
                    } else {
                        crate::routeros::inspect_from(&router, &root).await
                    }?;
                    Ok::<_, anyhow::Error>((router.id, Arc::new(inv)))
                });
            }
            let Some(result) = tasks.join_next().await else {
                break;
            };
            let (id, inv) = result??;
            inventories.insert(id, inv);
        }
        ensure!(
            self.settings().await? == settings,
            "Settings changed during router collection"
        );
        let root = self.root.clone();
        let previous = std::mem::take(&mut self.previous);
        let snapshot = tokio::task::spawn_blocking(move || {
            if reconcile {
                Snapshot::from_inventories(
                    settings,
                    inventories,
                    &root,
                    &previous,
                    crate::orchestrator::now(),
                )
            } else {
                let runtime = core_console::health_snapshot(
                    &settings,
                    &inventories,
                    &BTreeMap::new(),
                    &previous,
                    crate::orchestrator::now(),
                )?;
                Ok(Snapshot {
                    root,
                    settings,
                    inventories,
                    runtime,
                })
            }
        })
        .await??;
        self.previous = Runtime {
            settings_revision: snapshot.runtime.settings_revision.clone(),
            health: snapshot.runtime.health.clone(),
            observation: snapshot.runtime.observation.clone(),
            cleanup: snapshot.runtime.cleanup.clone(),
            ..Default::default()
        };
        Ok(snapshot)
    }
}
impl Source for RouterSource {
    async fn capture(&mut self) -> Result<Snapshot> {
        self.collect(false).await
    }
    async fn confirm(&mut self, revision: &str) -> Result<()> {
        let current = self.settings().await?;
        ensure!(
            core_console::enabled(&current) && wan_health::settings_revision(&current)? == revision,
            "Settings or enable state changed after collection"
        );
        Ok(())
    }
}
#[derive(Serialize)]
struct Guard {
    root: PathBuf,
    revision: String,
    subscribers: Vec<Subscriber>,
    sessions: BTreeMap<String, Session>,
    health: BTreeMap<String, (HealthStatus, String)>,
    uptimes: BTreeMap<String, u64>,
    policy: BTreeMap<String, String>,
    captured_at: u64,
}
fn digest(value: &impl Serialize) -> Result<String> {
    Ok(uuid::Uuid::new_v5(&uuid::Uuid::NAMESPACE_OID, &serde_json::to_vec(value)?).to_string())
}
impl Guard {
    fn capture(snapshot: &Snapshot, now: u64) -> Result<(Self, Rows, u64)> {
        let s = &snapshot.settings;
        crate::model::validate(s)?;
        ensure!(
            core_console::enabled(s),
            "Core control is paused or disabled"
        );
        let revision = wan_health::settings_revision(s)?;
        let ttl = s
            .core_control
            .as_ref()
            .context("Missing worker policy")?
            .options
            .evidence_seconds
            .min(30);
        ensure!(
            snapshot.runtime.settings_revision == revision
                && snapshot.runtime.captured_at <= now
                && now - snapshot.runtime.captured_at <= ttl,
            "Stale or superseded preview round"
        );
        let mut subscribers = normalizer::collect(s, &snapshot.inventories, &BTreeMap::new(), now)?;
        subscribers.sort_by(|a, b| a.id.cmp(&b.id));
        let sessions = sessions::all_sessions(s, &mut subscribers, &snapshot.inventories, now)?;
        sessions::validate(&sessions)?;
        let mut until = now.saturating_add(ttl);
        let mut uptimes = BTreeMap::new();
        for (id, inv) in &snapshot.inventories {
            ensure!(
                inv.router_id == *id
                    && inv.errors.is_empty()
                    && inv.collected_at <= now
                    && now - inv.collected_at <= ttl,
                "Incomplete, stale or mismatched actual inventory"
            );
            until = until.min(inv.collected_at.saturating_add(ttl));
            let resource = inv
                .tables
                .get("/system/resource/print")
                .context("Missing router resource table")?;
            ensure!(resource.len() == 1, "Ambiguous router resource table");
            uptimes.insert(
                id.clone(),
                sessions::uptime(resource[0].get("uptime").context("Missing router uptime")?)?,
            );
        }
        let config = s
            .wan_health
            .as_ref()
            .context("Missing health configuration")?;
        let dynamic = s.dynamic.as_ref().context("Missing dynamic topology")?;
        let mut health = BTreeMap::new();
        for monitor in &config.monitors {
            let wan = dynamic
                .wans
                .iter()
                .find(|w| w.id == monitor.wan)
                .context("Unknown monitored WAN")?;
            let inv = snapshot
                .inventories
                .get(&wan.router)
                .context("Missing health core")?;
            let e = wan_health::from_inventory(config, monitor, inv, &wan.interface, &revision)?;
            let state = snapshot
                .runtime
                .health
                .get(&monitor.wan)
                .context("Missing completed health state")?;
            ensure!(
                state.settings_revision == revision && state.wan == monitor.wan,
                "Health identity changed"
            );
            until = until.min(state.verification_deadline(ttl));
            health.insert(monitor.wan.clone(), (state.status_at(now, ttl), e.context));
        }
        ensure!(until >= now, "Completed health evidence expired");
        let mut rows = Rows::new();
        let mut policy = BTreeMap::new();
        for binding in &s.core_lists.bindings {
            if rows.contains_key(&binding.router) {
                continue;
            }
            let inv = snapshot
                .inventories
                .get(&binding.router)
                .context("Missing bound core")?;
            rows.insert(
                binding.router.clone(),
                core_writer_transport::members(
                    inv.tables
                        .get("/ip/firewall/address-list/print")
                        .context("Missing actual core rows")?,
                )?,
            );
            // Freeze forwarding policy as well as WAN probe paths. Preserve rule order.
            let mut tables = BTreeMap::new();
            for path in crate::core_plan::EXTRA_TABLES.iter().copied().chain([
                "/ip/route/print",
                "/interface/print",
                "/interface/list/member/print",
            ]) {
                if path != "/ip/firewall/address-list/print" {
                    tables.insert(
                        path,
                        inv.tables
                            .get(path)
                            .context("Incomplete core routing policy")?,
                    );
                }
            }
            policy.insert(binding.router.clone(), digest(&tables)?);
        }
        Ok((
            Self {
                root: snapshot.root.clone(),
                revision,
                subscribers,
                sessions: sessions.active,
                health,
                uptimes,
                policy,
                captured_at: now,
            },
            rows,
            until,
        ))
    }
    fn accepts(&self, next: &Self, ttl: u64) -> Result<()> {
        ensure!(
            self.root == next.root
                && self.revision == next.revision
                && self.subscribers == next.subscribers
                && self.health == next.health
                && self.policy == next.policy,
            "Settings, source identities, rates, WAN health or routing policy changed"
        );
        ensure!(
            next.captured_at >= self.captured_at && next.captured_at - self.captured_at <= ttl,
            "Source continuity window expired"
        );
        ensure!(
            self.sessions.len() == next.sessions.len()
                && self.sessions.iter().all(|(id, old)| next
                    .sessions
                    .get(id)
                    .is_some_and(|new| sessions::continuous(old, new))),
            "Client reconnected or source sessions changed"
        );
        ensure!(
            self.uptimes.len() == next.uptimes.len()
                && self
                    .uptimes
                    .iter()
                    .all(|(id, old)| next.uptimes.get(id).is_some_and(|new| new >= old)),
            "Router rebooted or inventory scope changed"
        );
        Ok(())
    }
}
pub struct Prepared {
    batch: Batch,
    settings: Settings,
    guard: Guard,
    expires_at: u64,
}
impl Prepared {
    pub fn new(snapshot: Snapshot, approval_seconds: u64) -> Result<Self> {
        ensure!(
            (1..=300).contains(&approval_seconds),
            "Approval lifetime must be 1–300 seconds"
        );
        let now = crate::orchestrator::now();
        let (guard, rows, _) = Guard::capture(&snapshot, now)?;
        let plan = snapshot
            .runtime
            .preview
            .as_ref()
            .context("No complete reconciliation preview")?;
        let mutations = plan
            .decisions
            .iter()
            .filter(|d| !matches!(d, crate::core_reconcile::Decision::Keep { .. }))
            .count();
        ensure!(
            mutations <= snapshot.settings.core_control.as_ref().unwrap().batch_size,
            "Reconciliation exceeds configured batch limit; review a smaller scope"
        );
        let expires_at = now.saturating_add(approval_seconds);
        let batch =
            Batch::from_reconcile(&snapshot.settings, plan, digest(&guard)?, expires_at, rows)?;
        Ok(Self {
            batch,
            settings: snapshot.settings,
            guard,
            expires_at,
        })
    }
    pub fn revision(&self) -> Result<String> {
        self.batch.revision()
    }
    /// Exact approval does not bypass the exclusive offline fence or create a scheduler enable path.
    pub fn open<S: Source>(self, root: &Path, approved: &str, source: S) -> Result<Worker<S>> {
        ensure!(
            root.canonicalize()? == self.guard.root,
            "Prepared batch belongs to another data directory"
        );
        ensure!(
            crate::orchestrator::now() <= self.expires_at,
            "Worker approval expired"
        );
        let executor = Executor::open(root, self.batch, approved)?;
        let approval = Approval {
            enabled: Arc::new(AtomicBool::new(true)),
            expires_at: self.expires_at,
        };
        let verifier = GuardedVerifier {
            context: digest(&self.guard)?,
            last: self.guard,
            approval: approval.clone(),
            source,
        };
        let io = RouterTransport::new(root, self.settings, verifier)?;
        Ok(Worker {
            executor,
            io,
            approval,
        })
    }
}
pub struct GuardedVerifier<S> {
    context: String,
    last: Guard,
    approval: Approval,
    source: S,
}
impl<S: Source> Verifier for GuardedVerifier<S> {
    async fn verify(&mut self) -> Result<Evidence> {
        ensure!(self.approval.active(), "Writer approval revoked or expired");
        let result = async {
            let snapshot = self.source.capture().await?;
            ensure!(
                self.approval.active(),
                "Writer approval revoked during collection"
            );
            let now = crate::orchestrator::now();
            let ttl = snapshot
                .settings
                .core_control
                .as_ref()
                .context("Missing worker policy")?
                .options
                .evidence_seconds
                .min(30);
            let (next, rows, until) =
                tokio::task::spawn_blocking(move || Guard::capture(&snapshot, now)).await??;
            self.last.accepts(&next, ttl)?;
            self.source.confirm(&next.revision).await?;
            ensure!(
                self.approval.active() && crate::orchestrator::now() <= until,
                "Approval or verification expired after collection"
            );
            self.last = next;
            Ok(Evidence {
                context: self.context.clone(),
                captured_at: now,
                valid_until: until.min(self.approval.expires_at),
                rows,
            })
        }
        .await;
        if result.is_err() {
            self.approval.revoke();
        }
        result
    }
}
pub struct Worker<S> {
    executor: Executor,
    io: RouterTransport<GuardedVerifier<S>>,
    approval: Approval,
}
impl<S: Source> Worker<S> {
    pub fn approval(&self) -> Approval {
        self.approval.clone()
    }
    pub async fn forward(&mut self) -> Result<()> {
        self.executor.forward(&mut self.io).await
    }
    pub async fn rollback(&mut self) -> Result<()> {
        self.executor.rollback(&mut self.io).await
    }
    pub fn finish(self) -> Result<()> {
        ensure!(self.approval.active(), "Worker approval revoked or expired");
        self.executor.finish()
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn expired_approval_never_authorizes_verification() {
        let approval = Approval {
            enabled: Arc::new(AtomicBool::new(true)),
            expires_at: crate::orchestrator::now() - 1,
        };
        assert!(!approval.active());
        let approval = Approval {
            expires_at: crate::orchestrator::now() + 30,
            ..approval
        };
        assert!(approval.active());
        approval.clone().revoke();
        assert!(!approval.active());
    }
}
