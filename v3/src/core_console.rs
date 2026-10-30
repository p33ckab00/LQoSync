//! Scheduler/UI preview integration. Automatic writer execution remains gated pending acceptance proof.
use crate::{
    core_reconcile::{self, Cleanup, Options, ReconcilePlan},
    model::Settings,
    routeros::Inventory,
    sessions::{self, Observation},
    wan_health::{self, State},
    web::Shared,
};
use anyhow::{Result, ensure};
use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, sync::Arc};
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Mode {
    #[default]
    Preview,
    Paused,
    Automatic,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Config {
    pub mode: Mode,
    pub options: Options,
    pub batch_size: usize,
}
impl Default for Config {
    fn default() -> Self {
        Self {
            mode: Mode::Preview,
            options: Options::default(),
            batch_size: 20,
        }
    }
}
impl Config {
    pub fn validate(&self) -> Result<()> {
        ensure!(
            self.mode != Mode::Automatic,
            "Automatic writes blocked: isolated RouterOS acceptance, live verifier and console writer coordination are not complete"
        );
        ensure!(
            (1..=20).contains(&self.batch_size),
            "Batch size must be 1–20"
        );
        ensure!(
            (1..=3600).contains(&self.options.offline_seconds)
                && (1..=300).contains(&self.options.evidence_seconds),
            "Invalid reconciliation timing"
        );
        Ok(())
    }
}
#[derive(Clone, Default)]
pub struct Runtime {
    pub settings_revision: String,
    pub health: BTreeMap<String, State>,
    pub preview: Option<ReconcilePlan>,
    pub cleanup: Cleanup,
    pub observation: Option<Observation>,
    pub status: String,
    pub observed: BTreeMap<String, usize>,
    pub ambiguous: usize,
    pub captured_at: u64,
    pub(crate) epoch: u64,
    pub(crate) busy: bool,
}
pub fn revision(settings: &Settings) -> Result<String> {
    wan_health::settings_revision(settings)
}
pub fn enabled(settings: &Settings) -> bool {
    settings
        .core_control
        .as_ref()
        .is_some_and(|c| c.mode == Mode::Preview)
        && settings
            .wan_health
            .as_ref()
            .is_some_and(|h| !h.monitors.is_empty())
}
pub fn interval(settings: &Settings) -> u64 {
    if enabled(settings) {
        settings
            .interval_seconds
            .min(settings.wan_health.as_ref().unwrap().interval_seconds)
    } else {
        settings.interval_seconds
    }
}
pub fn monitor_router(settings: &Settings, router: &str) -> bool {
    enabled(settings)
        && settings.dynamic.as_ref().is_some_and(|c| {
            c.wans.iter().any(|w| {
                w.router == router
                    && settings
                        .wan_health
                        .as_ref()
                        .unwrap()
                        .monitors
                        .iter()
                        .any(|m| m.wan == w.id)
            })
        })
}
pub(crate) fn persisted_policy(root: &std::path::Path, expected: &str) -> Result<()> {
    // Do not recreate a removed settings file during a preview check.
    let current: Settings = toml::from_str(&std::fs::read_to_string(root.join("settings.toml"))?)?;
    ensure!(
        enabled(&current) && revision(&current)? == expected,
        "Persisted settings changed or preview paused in another console"
    );
    Ok(())
}
pub async fn refresh(shared: Shared) {
    let (settings, inventories, errors, epochs, root, previous, ticket, coordinator) = {
        let mut a = shared.lock().await;
        a.core_coordinator.configure(
            &revision(&a.settings).unwrap_or_default(),
            enabled(&a.settings),
        );
        if !enabled(&a.settings) {
            let epoch = a.core_runtime.epoch.wrapping_add(1);
            a.core_runtime = Runtime {
                epoch,
                status: if a
                    .settings
                    .core_control
                    .as_ref()
                    .is_some_and(|c| c.mode == Mode::Paused)
                {
                    "Core preview paused".into()
                } else {
                    "Configure health monitors and select Preview to collect evidence".into()
                },
                ..Default::default()
            };
            return;
        }
        if a.core_runtime.busy {
            return;
        }
        a.core_runtime.busy = true;
        a.core_runtime.epoch = a.core_runtime.epoch.wrapping_add(1);
        (
            a.settings.clone(),
            a.inventories.clone(),
            a.inventory_errors.clone(),
            a.read_epochs.clone(),
            a.root.clone(),
            Runtime {
                settings_revision: a.core_runtime.settings_revision.clone(),
                health: a.core_runtime.health.clone(),
                cleanup: a.core_runtime.cleanup.clone(),
                observation: a.core_runtime.observation.clone(),
                ..Default::default()
            },
            a.core_runtime.epoch,
            a.core_coordinator.clone(),
        )
    };
    // Finish bookkeeping even if the HTTP caller disconnects or its task is cancelled.
    let worker = tokio::spawn(async move {
        let work_settings = settings.clone();
        let work_inventories = inventories.clone();
        let work_errors = errors.clone();
        let result = tokio::task::spawn_blocking(move || {
            let revision = revision(&work_settings)?;
            let ticket = coordinator.begin(&root, &revision)?;
            persisted_policy(&root, &revision)?;
            let runtime = compute(
                &work_settings,
                &work_inventories,
                &work_errors,
                &root,
                &previous,
                crate::orchestrator::now(),
            )?;
            ticket.check(&revision)?;
            persisted_policy(&root, &revision)?;
            Ok::<_, anyhow::Error>(runtime)
        })
        .await;
        let mut a = shared.lock().await;
        if a.core_runtime.epoch != ticket {
            return;
        }
        if a.settings != settings
            || a.inventory_errors != errors
            || a.read_epochs != epochs
            || !crate::orchestrator::same_inventories(&a.inventories, &inventories)
        {
            a.core_runtime = Runtime {
                epoch: ticket,
                status: "Preview superseded by newer settings or inventory; no writes sent".into(),
                ..Default::default()
            };
            return;
        }
        a.core_runtime = match result {
            Ok(Ok(mut runtime)) => {
                runtime.epoch = ticket;
                runtime
            }
            Ok(Err(e)) => Runtime {
                epoch: ticket,
                status: format!("Preview blocked: {e}; cleanup evidence discarded"),
                ..Default::default()
            },
            Err(_) => Runtime {
                epoch: ticket,
                status: "Preview worker failed; cleanup evidence discarded".into(),
                ..Default::default()
            },
        };
    });
    let _ = worker.await;
}
/// Evaluate health without reconciliation or local candidate persistence.
pub(crate) fn health_snapshot(
    settings: &Settings,
    inventories: &BTreeMap<String, Arc<Inventory>>,
    errors: &BTreeMap<String, String>,
    previous: &Runtime,
    now: u64,
) -> Result<Runtime> {
    crate::model::validate(settings)?;
    ensure!(
        enabled(settings),
        "Health preview is not configured or is paused"
    );
    let health_config = settings.wan_health.as_ref().unwrap();
    let dynamic = settings
        .dynamic
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("Dynamic topology required"))?;
    let rev = revision(settings)?;
    let same = previous.settings_revision == rev;
    let mut runtime = Runtime {
        settings_revision: rev.clone(),
        captured_at: now,
        ..Default::default()
    };
    for monitor in &health_config.monitors {
        let wan = dynamic
            .wans
            .iter()
            .find(|w| w.id == monitor.wan)
            .ok_or_else(|| anyhow::anyhow!("Unknown monitored WAN"))?;
        let evidence = if errors.contains_key(&wan.router) {
            Err("Latest core read failed".into())
        } else {
            inventories
                .get(&wan.router)
                .ok_or_else(|| "No core inventory".to_string())
                .and_then(|inv| {
                    wan_health::from_inventory(health_config, monitor, inv, &wan.interface, &rev)
                        .map_err(|e| e.to_string())
                })
        };
        let old = if same {
            previous.health.get(&monitor.wan)
        } else {
            None
        };
        runtime.health.insert(
            monitor.wan.clone(),
            wan_health::evaluate(health_config, old, evidence, now)?,
        );
    }
    Ok(runtime)
}
pub fn compute(
    settings: &Settings,
    inventories: &BTreeMap<String, Arc<Inventory>>,
    errors: &BTreeMap<String, String>,
    root: &std::path::Path,
    previous: &Runtime,
    now: u64,
) -> Result<Runtime> {
    let mut runtime = health_snapshot(settings, inventories, errors, previous, now)?;
    let config = settings.core_control.as_ref().unwrap();
    let same = previous.settings_revision == runtime.settings_revision;
    let plan_result = (|| -> Result<()> {
        let mut raw = crate::normalizer::collect(settings, inventories, errors, now)?;
        let saved = match previous.observation.as_ref().filter(|_| same) {
            Some(observation) => Some(observation.clone()),
            None => sessions::load(root)?,
        };
        let committed = crate::store::load_assignments(root)?;
        let observation = sessions::plan(
            settings,
            &mut raw,
            &committed,
            saved.as_ref(),
            inventories,
            errors,
            now,
        )?;
        let cleanup = if same {
            previous.cleanup.clone()
        } else {
            Cleanup::default()
        };
        let preview = core_reconcile::plan(core_reconcile::Input {
            settings,
            inventories,
            observation: &observation,
            previous: saved.as_ref(),
            health: &runtime.health,
            cleanup: &cleanup,
            options: config.options.clone(),
            now,
        })?;
        // Count actual coverage independently of proposed assignments, including protected ranges.
        let mut spans: BTreeMap<&str, Vec<(u32, u32)>> = BTreeMap::new();
        for binding in &settings.core_lists.bindings {
            for row in inventories[&binding.router].tables["/ip/firewall/address-list/print"]
                .iter()
                .filter(|r| r.get("list") == Some(&binding.list))
            {
                if !crate::core_routes::flag(row, "disabled", false)? {
                    spans
                        .entry(&binding.wan)
                        .or_default()
                        .push(crate::core_plan::span(
                            row.get("address")
                                .ok_or_else(|| anyhow::anyhow!("Missing core address"))?,
                        )?);
                }
            }
        }
        for ranges in spans.values_mut() {
            ranges.sort_unstable();
            let mut end = 0;
            for (_, z) in ranges {
                end = end.max(*z);
                *z = end;
            }
        }
        for row in &raw {
            let ip = crate::core_plan::host(&row.ipv4)?;
            let found: Vec<_> = spans
                .iter()
                .filter(|(_, ranges)| {
                    let n = ranges.partition_point(|(a, _)| *a <= ip);
                    n > 0 && ranges[n - 1].1 >= ip
                })
                .map(|(w, _)| *w)
                .collect();
            if found.len() == 1 {
                *runtime.observed.entry(found[0].into()).or_default() += 1;
            } else {
                runtime.ambiguous += 1;
            }
        }
        runtime.cleanup = preview.cleanup.clone();
        runtime.observation = Some(observation);
        runtime.preview = Some(preview);
        runtime.status = "Fresh local preview only. No router writes or production apply.".into();
        Ok(())
    })();
    if let Err(e) = plan_result {
        runtime.preview = None;
        runtime.observation = None;
        runtime.cleanup = Cleanup::default();
        runtime.observed.clear();
        runtime.ambiguous = 0;
        runtime.status = format!("Preview blocked: {e}; cleanup evidence discarded");
    }
    Ok(runtime)
}
