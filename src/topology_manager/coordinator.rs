//! Current configured source batches. No saved subscriber endpoints enter this path.
use super::normalize::CurrentRouter;
use super::{
    catalog::RouterRole,
    identity::authorize_configured,
    model::{RegistryDocument, SourceKind},
    normalize::normalize_configured,
    observation::ObservationGate,
    publication::{PreparedPair, prepare_with_static},
    reader::{ReadGuard, ReadPlan},
};
use crate::{
    config::{Config, RouterCfg},
    routeros::Connector,
};
use anyhow::{Context, ensure};
use std::{
    collections::{BTreeMap, BTreeSet, HashMap},
    sync::Arc,
    time::Duration,
};
use tokio::{
    task::JoinSet,
    time::{Instant, timeout_at},
};

struct SourceSession<S> {
    router: RouterCfg,
    guard: ReadGuard,
    session: Option<S>,
}

fn same_connection(a: &RouterCfg, b: &RouterCfg) -> bool {
    a.name == b.name
        && a.address == b.address
        && a.port == b.port
        && a.tls == b.tls
        && a.tls_verify == b.tls_verify
        && a.tls_ca_file == b.tls_ca_file
        && a.username == b.username
        && a.password_file == b.password_file
}

pub(crate) struct Coordinator<C: Connector> {
    connector: Arc<C>,
    generation: u64,
    gate: ObservationGate,
    sessions: HashMap<String, SourceSession<C::Session>>,
    observed: Vec<(String, crate::model::RouterSnapshot, Vec<&'static str>)>,
}
impl<C: Connector + 'static> Coordinator<C>
where
    C::Session: 'static,
{
    pub(crate) fn new(connector: Arc<C>) -> Self {
        Self {
            connector,
            generation: 0,
            gate: ObservationGate::default(),
            sessions: HashMap::new(),
            observed: Vec::new(),
        }
    }
    pub(crate) fn clear_sessions(&mut self) {
        self.sessions.clear();
        self.observed.clear();
    }
    pub(crate) fn take_observed(
        &mut self,
    ) -> Vec<(String, crate::model::RouterSnapshot, Vec<&'static str>)> {
        std::mem::take(&mut self.observed)
    }

    /// Read and normalize the required protocols of each selected router in one bounded batch.
    async fn read_required(
        &mut self,
        cfg: &Config,
        catalog: &super::catalog::TopologyCatalog,
        required: &BTreeMap<&str, BTreeSet<SourceKind>>,
        minimum_generation: u64,
        deadline: Instant,
    ) -> anyhow::Result<(u64, Vec<CurrentRouter>)> {
        self.observed.clear();
        // A router disabled since the last batch loses its held session first, even if this batch fails.
        self.sessions.retain(|_, held| {
            cfg.routers
                .iter()
                .any(|r| r.name == held.router.name && r.enabled)
        });
        let mut by_name: HashMap<&str, Vec<_>> = HashMap::new();
        for router in &cfg.routers {
            by_name.entry(&router.name).or_default().push(router);
        }
        let by_id: HashMap<_, _> = catalog.routers.iter().map(|r| (r.id.as_str(), r)).collect();
        let mut plans = Vec::with_capacity(required.len());
        let mut names = BTreeSet::new();
        for (&id, kinds) in required {
            let catalog_router = by_id
                .get(id)
                .context("selected catalog router is missing")?;
            let matches = by_name
                .get(catalog_router.name.as_str())
                .context("selected router has no configured connection")?;
            ensure!(
                matches.len() == 1 && names.insert(&catalog_router.name),
                "selected router connection is ambiguous"
            );
            let router = matches[0];
            ensure!(
                router.enabled,
                "selected router is disabled; LQoSync makes no connection to it"
            );
            let mut roles = Vec::with_capacity(kinds.len());
            for kind in kinds {
                let (role, enabled) = match kind {
                    SourceKind::Ppp => (RouterRole::Ppp, router.pppoe.enabled),
                    SourceKind::Dhcp => (RouterRole::Dhcp, router.dhcp.enabled),
                    SourceKind::Hotspot => (RouterRole::Hotspot, router.hotspot.enabled),
                };
                ensure!(
                    catalog_router.roles.contains(&role) && enabled,
                    "selected subscriber protocol is disabled or lacks its catalog role"
                );
                roles.push(role);
            }
            plans.push((
                id.to_owned(),
                router.clone(),
                ReadPlan::for_roles(id, &roles),
            ));
        }
        ensure!(
            cfg.router_access.api_timeout > 0 && cfg.router_access.connect_timeout > 0,
            "source time limits must be positive"
        );
        let connect_limit = Duration::from_secs(cfg.router_access.connect_timeout)
            .checked_add(Duration::from_secs(cfg.router_access.api_timeout))
            .context("connection limit exceeds clock range")?;
        self.generation = self
            .generation
            .max(minimum_generation)
            .checked_add(1)
            .context("source generation is exhausted")?;
        let generation = self.generation;
        ensure!(
            Instant::now() < deadline,
            "source batch expired before connection"
        );
        self.sessions
            .retain(|id, _| required.contains_key(id.as_str()));
        // JoinSet aborts unfinished reads if this batch fails or its caller cancels it.
        let mut tasks = JoinSet::new();
        let global_exclusions = std::sync::Arc::new(cfg.exclude.clone());
        for (id, router, plan) in plans {
            let held = self
                .sessions
                .remove(&id)
                .filter(|held| same_connection(&held.router, &router));
            let (mut guard, mut session) = match held {
                Some(held) => (held.guard, held.session),
                None => (ReadGuard::new(id.clone())?, None),
            };
            let connector = self.connector.clone();
            let access = cfg.router_access.clone();
            let global_exclusions = global_exclusions.clone();
            tasks.spawn(async move {
                let read = async {
                    if session.is_none() {
                        let connect_deadline = Instant::now()
                            .checked_add(connect_limit)
                            .unwrap_or(deadline)
                            .min(deadline);
                        session = Some(
                            timeout_at(connect_deadline, connector.connect(&router, &access))
                                .await
                                .map_err(|_| anyhow::anyhow!("source connection expired"))?
                                .map_err(|_| anyhow::anyhow!("source connection failed"))?,
                        );
                    }
                    let evidence = guard
                        .read(
                            &mut session,
                            &plan,
                            generation,
                            Duration::from_secs(access.api_timeout),
                            deadline.saturating_duration_since(Instant::now()),
                        )
                        .await
                        .map_err(|_| anyhow::anyhow!("source read failed"))?;
                    let current =
                        normalize_configured(&evidence, &global_exclusions, &router.exclude)
                            .map_err(|_| anyhow::anyhow!("source normalization failed"))?;
                    use super::reader::Table;
                    let rows = |table| evidence.rows(table).map(<[_]>::to_vec).unwrap_or_default();
                    let snapshot = crate::model::RouterSnapshot {
                        ppp_secret: rows(Table::PppSecrets),
                        ppp_active: rows(Table::PppActive),
                        ppp_profile: rows(Table::PppProfiles),
                        hotspot_active: rows(Table::HotspotActive),
                        hotspot_user: evidence.rows(Table::HotspotUsers).ok().map(<[_]>::to_vec),
                        dhcp_server: rows(Table::DhcpServers),
                        dhcp_lease: rows(Table::DhcpLeases),
                    };
                    let protocols = evidence
                        .tables()
                        .filter_map(|table| match table {
                            Table::PppSecrets => Some("ppp"),
                            Table::DhcpLeases => Some("dhcp"),
                            Table::HotspotUsers => Some("hotspot"),
                            _ => None,
                        })
                        .collect();
                    Ok::<_, anyhow::Error>((current, snapshot, protocols))
                };
                let (current, snapshot, protocols) = timeout_at(deadline, read)
                    .await
                    .context("source batch expired")??;
                Ok::<_, anyhow::Error>((
                    id,
                    SourceSession {
                        router,
                        guard,
                        session,
                    },
                    current,
                    snapshot,
                    protocols,
                ))
            });
        }
        let mut current = Vec::with_capacity(tasks.len());
        let mut observed = Vec::with_capacity(tasks.len());
        while let Some(outcome) = tasks.join_next().await {
            let (id, session, router, snapshot, protocols) =
                outcome.map_err(|_| anyhow::anyhow!("source task stopped"))??;
            observed.push((session.router.name.clone(), snapshot, protocols));
            self.sessions.insert(id, session);
            current.push(router);
        }
        self.observed = observed;
        Ok((generation, current))
    }

    /// Read every configured router protocol for a first policy proposal. No registry is needed.
    pub(crate) async fn read_for_proposal(
        &mut self,
        cfg: &Config,
        max_age: Duration,
    ) -> anyhow::Result<Vec<CurrentRouter>> {
        self.observed.clear();
        ensure!(!max_age.is_zero(), "source age limit must be positive");
        let deadline = Instant::now()
            .checked_add(max_age)
            .context("source age limit exceeds clock range")?;
        let catalog = super::policy_draft::read_catalog(cfg);
        let mut required: BTreeMap<&str, BTreeSet<SourceKind>> = BTreeMap::new();
        for router in &catalog.routers {
            let kinds = required.entry(&router.id).or_default();
            for role in &router.roles {
                kinds.insert(match role {
                    RouterRole::Ppp => SourceKind::Ppp,
                    RouterRole::Dhcp => SourceKind::Dhcp,
                    RouterRole::Hotspot => SourceKind::Hotspot,
                    RouterRole::Wan => continue,
                });
            }
        }
        required.retain(|_, kinds| !kinds.is_empty());
        Ok(self
            .read_required(cfg, &catalog, &required, 0, deadline)
            .await?
            .1)
    }

    pub(crate) async fn prepare(
        &mut self,
        cfg: &Config,
        registry: &RegistryDocument,
        minimum_generation: u64,
        max_age: Duration,
    ) -> anyhow::Result<PreparedPair> {
        self.observed.clear();
        let started = Instant::now();
        ensure!(!max_age.is_zero(), "source age limit must be positive");
        let deadline = started
            .checked_add(max_age)
            .context("source age limit exceeds clock range")?;
        let issues = super::preflight::source_issues(cfg, registry)?;
        ensure!(
            issues.is_empty(),
            "unsupported configured source policies: {issues:?}"
        );
        let static_source = super::static_input::required(registry)
            .then(|| {
                super::static_input::CurrentStatic::load(std::path::Path::new(
                    &cfg.runtime.state_dir,
                ))
            })
            .transpose()?;
        let source_filters = super::preflight::source_filter_digest(cfg, registry)?;
        let mut authorized = self
            .authorized(cfg, registry, minimum_generation, deadline)
            .await?;
        authorized.router_locations = super::publication::router_locations(cfg);
        let mut pair = prepare_with_static(authorized, static_source.as_ref().map(|s| s.input()))?;
        pair.static_source = static_source.map(|s| s.binding);
        pair.check_fresh()?;
        pair.source_filters = source_filters;
        Ok(pair)
    }

    /// Read the selected sources of `registry` and reconcile them, without publication.
    pub(crate) async fn reconcile_for(
        &mut self,
        cfg: &Config,
        registry: &RegistryDocument,
        minimum_generation: u64,
        max_age: Duration,
    ) -> anyhow::Result<super::observation::Reconciliation> {
        self.observed.clear();
        ensure!(!max_age.is_zero(), "source age limit must be positive");
        let deadline = Instant::now()
            .checked_add(max_age)
            .context("source age limit exceeds clock range")?;
        let issues = super::preflight::source_issues(cfg, registry)?;
        ensure!(
            issues.is_empty(),
            "unsupported configured source policies: {issues:?}"
        );
        Ok(self
            .authorized(cfg, registry, minimum_generation, deadline)
            .await?
            .reconciliation)
    }

    async fn authorized(
        &mut self,
        cfg: &Config,
        registry: &RegistryDocument,
        minimum_generation: u64,
        deadline: Instant,
    ) -> anyhow::Result<super::identity::AuthorizedReconciliation> {
        let catalog = registry
            .catalog
            .as_ref()
            .context("topology catalog is unconfigured")?;
        let selections = registry
            .source_settings
            .as_ref()
            .context("source settings are unconfigured")?
            .ready_selections(catalog)?;
        let mut required: BTreeMap<&str, BTreeSet<SourceKind>> = BTreeMap::new();
        for selection in selections {
            required
                .entry(&selection.scope.router_id)
                .or_default()
                .insert(selection.scope.kind);
        }
        let (generation, current) = self
            .read_required(cfg, catalog, &required, minimum_generation, deadline)
            .await?;
        ensure!(
            Instant::now() < deadline,
            "source batch expired before authorization"
        );
        authorize_configured(registry, registry.revision, generation, current)?
            .reconcile(&mut self.gate)
    }
}
#[cfg(test)]
#[path = "coordinator_tests.rs"]
mod tests;
