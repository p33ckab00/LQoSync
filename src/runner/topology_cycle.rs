//! Scheduled publication for an explicitly selected topology writer. Never a legacy fallback.
use super::*;

impl<C> Runner<C>
where
    C: Connector + 'static,
    C::Session: 'static,
{
    /// Explicit activation: dry-run compiles only. Confirmation uses the existing sole writer.
    pub async fn activate_topology(
        &mut self,
        confirmed: bool,
        now: u64,
    ) -> anyhow::Result<Option<crate::write_receipt::WriteResult>> {
        self.require_topology_enrollment()?;
        let state_dir = Path::new(&self.cfg.runtime.state_dir);
        let mut compiler = crate::topology_manager::compiler::ControlledCompiler::open(
            &self.cfg.libreqos,
            &state_dir.join("topology-manager/compiler"),
        )?;
        let pair = self.prepare_topology(Duration::from_secs(30)).await?;
        self.check_topology_pair(&pair)?;
        let wanted = self.wanted_lists(&pair)?;
        let compiled = compiler.compile(pair).await?;
        if confirmed {
            self.publish_with_lists(compiled, wanted, now)
                .await
                .map(Some)
        } else {
            Ok(None)
        }
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
                Err(error)
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
