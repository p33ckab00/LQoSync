//! Scheduled publication for an explicitly selected topology writer. Never a legacy fallback.
use super::*;

/// A source batch failure in topology mode: the pair stays, and it is no manager failure.
#[derive(Debug)]
struct SourceBatchFailed;
impl std::fmt::Display for SourceBatchFailed {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("the topology source batch failed; the last pair stays")
    }
}

/// The result of `topology activate`.
#[derive(Debug)]
pub struct ActivationOutcome {
    /// True when the owner file is written (`--yes`).
    pub activated: bool,
    /// Node capacities that the export rounds.
    pub rounded: Vec<crate::topology_manager::publication::NodeExportAdjustment>,
    /// Registry subscribers with more than one identity.
    pub multi_identity: usize,
}

/// The result of `topology deactivate`.
#[derive(Debug)]
pub struct DeactivationOutcome {
    /// True when the owner file is removed (`--yes`).
    pub deactivated: bool,
    /// Legacy circuits that got the manager IDs.
    pub seeded: usize,
    /// Subscribers with more than one device row. The legacy writer gives them new IDs.
    pub skipped_multi_identity: usize,
}

impl<C> Runner<C>
where
    C: Connector + 'static,
    C::Session: 'static,
{
    /// `topology activate`: run the gates, and with `install` claim the writer. It writes no
    /// pair: the next cycle publishes the first manager pair.
    pub async fn activate_topology(
        &mut self,
        report: &Path,
        max_age: Duration,
        install: bool,
    ) -> anyhow::Result<ActivationOutcome> {
        self.activate_topology_with(&mut super::live_test::NativeIo, report, max_age, install)
            .await
    }

    /// As `activate_topology`, with the LibreQoS compiler check through `io`. Gates in order:
    /// enrolled, not active, every legacy row has an alias, parity, compiler, live test report.
    pub async fn activate_topology_with(
        &mut self,
        io: &mut impl super::live_test::LiveIo,
        report: &Path,
        max_age: Duration,
        install: bool,
    ) -> anyhow::Result<ActivationOutcome> {
        use crate::topology_manager::parity::ParityIssue;
        self.lock()?;
        self.topology_preflight()?;
        self.open_topology_store()?;
        let document = self
            .topology_store
            .as_ref()
            .context("registry owner is missing")?
            .document()?
            .clone();
        anyhow::ensure!(
            document.export_ownership.is_some(),
            "the topology registry is not enrolled"
        );
        let state_dir = std::path::PathBuf::from(&self.cfg.runtime.state_dir);
        anyhow::ensure!(
            !crate::topology_manager::runtime::topology_owns(&state_dir)?,
            "the Topology Manager is already active"
        );
        let pair = self.prepare_topology(max_age).await?;
        let (rows, network) = crate::topology_manager::enrollment::read_live_pair(&self.cfg)?;
        let issues = crate::topology_manager::parity::compare_legacy(&pair, &rows, &network)?;
        if let Some(ParityIssue::MissingRow {
            circuit_id,
            device_id,
        }) = issues
            .iter()
            .find(|i| matches!(i, ParityIssue::MissingRow { .. }))
        {
            anyhow::bail!(
                "legacy row {circuit_id} / {device_id} has no alias; run topology propose --replace and topology enroll --replace"
            );
        }
        anyhow::ensure!(
            issues.is_empty(),
            "the manager pair differs from the legacy pair: {issues:?}"
        );
        let rounded = pair.node_export_adjustments().to_vec();
        io.admit(&self.cfg, pair)
            .await
            .context("the LibreQoS compiler check failed")?;
        let proved = std::fs::read(report)
            .ok()
            .and_then(|b| serde_json::from_slice::<serde_json::Value>(&b).ok())
            .is_some_and(|v| {
                v["passed"] == true && v["registry_revision"].as_u64() == Some(document.revision)
            });
        anyhow::ensure!(
            proved,
            "the live test report does not prove the current registry revision {}",
            document.revision
        );
        if install {
            crate::topology_manager::runtime::claim(&state_dir)?;
        }
        Ok(ActivationOutcome {
            activated: install,
            rounded,
            multi_identity: document
                .subscribers
                .iter()
                .filter(|s| s.identities.len() > 1)
                .count(),
        })
    }

    /// `topology deactivate`: seed the legacy `state.json` with the IDs of the last manager pair,
    /// and with `install` release the writer. The legacy names come from a fresh router read
    /// through the legacy collection code. A row matches a collected circuit by IPv4, else by MAC.
    pub async fn deactivate_topology(
        &mut self,
        now: u64,
        install: bool,
    ) -> anyhow::Result<DeactivationOutcome> {
        self.lock()?;
        let state_dir = std::path::PathBuf::from(&self.cfg.runtime.state_dir);
        anyhow::ensure!(
            crate::topology_manager::runtime::topology_owns(&state_dir)?,
            "the Topology Manager is not active"
        );
        let super::Inputs {
            live, mut state, ..
        } = super::cycle_inputs(&self.cfg, now)?;
        // A subscriber with more than one device row has no single legacy key: skip it.
        let mut devices: HashMap<&str, usize> = HashMap::new();
        for row in &live.other_rows {
            *devices.entry(row.circuit_id.as_str()).or_default() += 1;
        }
        let skipped_multi_identity = devices.values().filter(|n| **n > 1).count();
        let single: Vec<&crate::model::CsvRow> = live
            .other_rows
            .iter()
            .filter(|r| devices[r.circuit_id.as_str()] == 1)
            .collect();
        let speed = crate::speed::SpeedParser::new(&self.cfg.speed)?;
        let mut seeded = 0;
        // Each manager row seeds one legacy circuit only, so no Circuit ID is used twice.
        let mut used: std::collections::HashSet<String> = std::collections::HashSet::new();
        for router in self.cfg.routers.clone() {
            if !router.reads_subscribers() {
                continue;
            }
            let mut session = self.sessions.remove(&router.name).flatten();
            let read = read_router(
                &*self.connector,
                &mut session,
                &router,
                &self.cfg.router_access,
            )
            .await;
            self.sessions.insert(router.name.clone(), session);
            let snap = read.map_err(|e| {
                anyhow::anyhow!(
                    "router {} cannot be read: {e}; nothing changed",
                    router.name
                )
            })?;
            for c in
                crate::sources::collect_router(&self.cfg, &router, &snap, &speed, &mut Vec::new())
            {
                let by_ip = |r: &&&crate::model::CsvRow| {
                    !c.ipv4.is_empty() && r.ipv4.split(',').any(|ip| ip == c.ipv4)
                };
                let by_mac = |r: &&&crate::model::CsvRow| {
                    !c.mac.is_empty() && r.mac.eq_ignore_ascii_case(&c.mac)
                };
                let Some(row) = single
                    .iter()
                    .find(by_ip)
                    .or_else(|| single.iter().find(by_mac))
                    .filter(|r| !used.contains(&r.circuit_id))
                else {
                    continue;
                };
                used.insert(row.circuit_id.clone());
                let mut row = (*row).clone();
                row.circuit_name = c.name.clone();
                // An old entry under another name with the same ID would give a duplicate ID.
                state
                    .circuits
                    .retain(|name, old| *name == c.name || old.row.circuit_id != row.circuit_id);
                state.circuits.insert(
                    c.name,
                    crate::model::StoredCircuit {
                        router: Some(router.name.clone()),
                        source: c.source,
                        last_seen: now,
                        row,
                    },
                );
                seeded += 1;
            }
        }
        // Keep the manager node IDs for the legacy nodes of the same name.
        if let Some(network) = &live.network {
            for (name, node) in crate::state::seed_nodes(network) {
                if let Some(old) = state.nodes.get_mut(&name) {
                    old.id = node.id;
                }
            }
        }
        if install {
            files::write_if_changed(&state_dir.join("state.json"), state.to_json().as_bytes())?;
            crate::topology_manager::runtime::release(&state_dir)?;
        }
        Ok(DeactivationOutcome {
            deactivated: install,
            seeded,
            skipped_multi_identity,
        })
    }

    pub(super) fn check_topology_pair(
        &self,
        pair: &crate::topology_manager::publication::PreparedPair,
    ) -> anyhow::Result<()> {
        pair.check_source_filters(&self.cfg)?;
        let tracker = self
            .write_tracker
            .as_ref()
            .context("receipt owner is missing")?
            .lock()
            .map_err(|_| anyhow::anyhow!("write receipt owner is poisoned"))?;
        pair.check_static_previous(
            tracker
                .current()
                .and_then(|r| r.meta.topology.as_ref())
                .and_then(|t| t.static_source.as_ref()),
        )
    }

    fn require_topology_enrollment(&mut self) -> anyhow::Result<()> {
        self.topology_preflight()?;
        self.open_topology_store()?;
        anyhow::ensure!(
            self.topology_store
                .as_ref()
                .context("registry owner is missing")?
                .document()?
                .export_ownership
                .is_some(),
            "the Topology Manager is not enrolled; automatic publication is blocked"
        );
        Ok(())
    }

    pub(super) async fn topology_cycle_inner(
        &mut self,
        now: u64,
        state_dir: &Path,
        status: &mut Status,
    ) -> anyhow::Result<(bool, Option<String>)> {
        let result = self.topology_cycle_publish(now, state_dir, status).await;
        let manager_error = match &result {
            Err(e) if e.downcast_ref::<SourceBatchFailed>().is_none() => Some(format!("{e:#}")),
            _ => None,
        };
        if result.is_ok() || manager_error.is_some() {
            self.record_manager(now, manager_error, status);
        } else {
            // A source batch failure neither starts nor ends a manager failure.
            status.writer = "topology".into();
            if let Some((since, error)) = self.manager_failure() {
                status.manager_failed = true;
                status.manager_failed_since = Some(since);
                status.manager_error = Some(error);
            }
        }
        // Failure retains last good evidence with its old timestamp; it never pretends to be empty.
        self.publish_web_view(now);
        result
    }

    async fn topology_cycle_publish(
        &mut self,
        now: u64,
        state_dir: &Path,
        status: &mut Status,
    ) -> anyhow::Result<(bool, Option<String>)> {
        self.topology_preflight()?;
        self.native_diagnostics(now, state_dir, status).await;
        self.require_topology_enrollment()?;
        status.target = target_name(crate::detect::resolve(&self.cfg.libreqos)?.target).into();
        let mut compiler = match self.topology_compiler.take() {
            Some(c) if c.check_config(&self.cfg.libreqos).is_ok() => c,
            _ => crate::topology_manager::compiler::ControlledCompiler::open(
                &self.cfg.libreqos,
                &state_dir.join("topology-manager/compiler"),
            )?,
        };
        self.sync_live_groups().await;
        let prepared = self.prepare_topology(Duration::from_secs(30)).await;
        let result = match prepared {
            Ok(pair) => {
                self.cache_topology_cycle_observations(&pair, state_dir, now, status);
                self.publish_topology(&mut compiler, pair, now).await
            }
            Err(error) => {
                if let Some(collection) = &mut self.collection {
                    for router in &mut collection.routers {
                        router.state = "failed".into();
                        status.routers.push(RouterStatus {
                            name: router.name.clone(),
                            state: "failed",
                            error: Some(
                                "the topology source batch failed; last observations retained"
                                    .into(),
                            ),
                            ..RouterStatus::default()
                        });
                    }
                    if let Err(error) = crate::collection::publish(state_dir, collection) {
                        eprintln!("warning: cannot write collection.json: {error:#}");
                    }
                    // Keep last circuit counts, but never leave their router read marked healthy.
                    if let Ok(bytes) = std::fs::read(state_dir.join(web_contract::routers::CACHE))
                        && let Ok(mut inventory) =
                            serde_json::from_slice::<web_contract::routers::RouterInventory>(&bytes)
                        && inventory.config_digest == self.config_digest
                    {
                        inventory.updated = now;
                        for router in &mut inventory.routers {
                            if let Some(seen) =
                                collection.routers.iter().find(|r| r.name == router.name)
                            {
                                router.collection = Some(web_contract::routers::RouterCollection {
                                    state: seen.state.clone(),
                                    observed_at: seen.observed_at,
                                    subscribers: seen.subscribers.len() as u64,
                                });
                            }
                        }
                        if let Err(error) = crate::router_view::publish(state_dir, &inventory) {
                            eprintln!("warning: cannot write routers.json: {error:#}");
                        }
                    }
                }
                Err(error.context(SourceBatchFailed))
            }
        };
        self.topology_compiler = Some(compiler);
        let result = result?;
        status.files_changed = result.changed;
        status.reload = if result.error.is_some() {
            "publication failed"
        } else if result.changed {
            "scheduler import requested"
        } else {
            "no change"
        }
        .into();
        if let Some(error) = result.tracking_error {
            eprintln!("write receipt storage: {error}");
        }
        if let Some(error) = result.error {
            anyhow::bail!("{error}");
        }
        if let (Some(preview), Some(store)) = (&mut self.last_preview, &self.topology_store)
            && let Ok(document) = store.document()
        {
            preview["registry_revision"] = serde_json::json!(document.revision);
        }
        Ok((result.changed, None))
    }

    fn cache_topology_cycle_observations(
        &mut self,
        pair: &crate::topology_manager::publication::PreparedPair,
        state_dir: &Path,
        now: u64,
        status: &mut Status,
    ) {
        let snapshots = self.topology_sources.take_observed();
        let mut protocols_by_router = HashMap::new();
        let outcomes: Vec<_> = snapshots
            .into_iter()
            .map(|(name, snapshot, protocols)| {
                protocols_by_router.insert(name.clone(), protocols);
                status.routers.push(RouterStatus {
                    name: name.clone(),
                    state: "online",
                    ..RouterStatus::default()
                });
                (name, RouterResult::Online(snapshot))
            })
            .collect();
        let mut collection =
            crate::collection::Report::collect(&self.cfg, &outcomes, self.collection.as_ref(), now);
        collection.cycle_seq = self.cycle_seq;
        for router in &mut collection.routers {
            if let Some(protocols) = protocols_by_router.get(&router.name) {
                // A configured but unselected protocol was not read. It is not an empty source.
                router
                    .sources
                    .retain(|kind, _| protocols.contains(&kind.as_str()));
            }
        }
        if let Err(error) = crate::collection::publish(state_dir, &collection) {
            eprintln!("warning: cannot write collection.json: {error:#}");
        }
        let catalog = self.api_document().ok().flatten().and_then(|d| d.catalog);
        let mut inventory = crate::router_view::project(
            &self.cfg,
            catalog.as_ref(),
            Some(&collection),
            None,
            &self.config_digest,
            now,
        );
        // The topology writer has no legacy State. Count stable circuits and identities instead.
        inventory.shaping.total_circuits = pair
            .rows()
            .iter()
            .map(|row| &row.circuit_id)
            .collect::<std::collections::HashSet<_>>()
            .len() as u64;
        let mut circuits_by_router: HashMap<&str, std::collections::HashSet<&str>> = HashMap::new();
        for subscriber in &pair.view().subscribers {
            for identity in &subscriber.identities {
                circuits_by_router
                    .entry(&identity.router_id)
                    .or_default()
                    .insert(&subscriber.id);
            }
        }
        for router in &mut inventory.routers {
            let topology_id = catalog
                .as_ref()
                .and_then(|c| {
                    c.routers
                        .iter()
                        .find(|r| r.name == router.name)
                        .map(|r| r.id.as_str())
                })
                .unwrap_or(router.id.as_str());
            router.circuits = Some(
                circuits_by_router
                    .get(topology_id)
                    .map_or(0, |ids| ids.len()) as u64,
            );
        }
        if let Err(error) = crate::router_view::publish(state_dir, &inventory) {
            eprintln!("warning: cannot write routers.json: {error:#}");
        }
        self.collection_config_digest = self.config_digest.clone();
        self.collection = Some(collection);
        let view = pair.view();
        let rounded: Vec<_> = pair.node_export_adjustments().iter().map(|a| serde_json::json!({
            "node_id": a.node_id,
            "calculated_mbps": [a.calculated_ceiling.down as f64 / 1_000_000.0, a.calculated_ceiling.up as f64 / 1_000_000.0],
            "exported_mbps": [a.exported_ceiling.down as f64 / 1_000_000.0, a.exported_ceiling.up as f64 / 1_000_000.0],
        })).collect();
        self.last_preview = Some(serde_json::json!({
            "collected_at": now,
            "registry_revision": pair.expected_revision(),
            "generation": pair.generation(),
            "state": "ok",
            "nodes": view.nodes,
            "subscribers": view.subscribers,
            "inventory": view.inventory,
            "conflicts": [],
            "rounded": rounded,
        }));
    }
}
