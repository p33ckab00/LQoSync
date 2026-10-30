use crate::{
    assignment,
    core_diagnostics::{self, Origin, Outcome},
    core_plan,
    model::{self, Settings},
    normalizer,
    routeros::Inventory,
    store,
};
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, BTreeSet},
    path::Path,
    sync::Arc,
};
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Check {
    pub code: String,
    pub status: Outcome,
    pub detail: String,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Coverage {
    pub wan: String,
    pub subscribers: usize,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Report {
    pub origin: Origin,
    pub reviewed_at: u64,
    pub generation: Option<String>,
    pub subscribers: usize,
    pub coverage: Vec<Coverage>,
    pub checks: Vec<Check>,
}
impl Report {
    pub fn status(&self) -> Outcome {
        self.checks
            .iter()
            .map(|c| c.status)
            .max()
            .unwrap_or(Outcome::Unavailable)
    }
    fn add(&mut self, code: &str, status: Outcome, detail: impl Into<String>) {
        self.checks.push(Check {
            code: code.into(),
            status,
            detail: detail.into().chars().take(512).collect(),
        });
    }
}
pub fn required_routers(settings: &Settings) -> BTreeSet<&str> {
    settings
        .core_lists
        .bindings
        .iter()
        .map(|b| b.router.as_str())
        .chain(
            settings
                .nodes
                .iter()
                .filter(|n| {
                    !n.router.is_empty()
                        && !n.source.is_empty()
                        && (normalizer::service(&n.kind).is_some() || n.kind == "wan")
                })
                .map(|n| n.router.as_str()),
        )
        .chain(normalizer::selected_routers(settings))
        .chain(
            settings
                .dynamic
                .iter()
                .flat_map(|c| c.wans.iter().map(|w| w.router.as_str())),
        )
        .collect()
}
pub fn review(
    settings: &Settings,
    inventories: &BTreeMap<String, Arc<Inventory>>,
    errors: &BTreeMap<String, String>,
    root: &Path,
    now: u64,
    origin: Origin,
) -> Report {
    let mut report = Report {
        origin,
        reviewed_at: now,
        generation: None,
        subscribers: 0,
        coverage: vec![],
        checks: vec![],
    };
    if origin == Origin::Saved {
        report.add(
            "historical-input",
            Outcome::Unavailable,
            "Historical review cannot establish current live validation",
        );
    }
    let generation = match store::review_generation(root) {
        Ok(Some(g)) => {
            report.generation = Some(g.generation.clone());
            Some(g)
        }
        Ok(None) => {
            report.add(
                "committed-generation",
                Outcome::Unavailable,
                "No committed managed output generation is available",
            );
            None
        }
        Err(_) => {
            report.add(
                "committed-generation",
                Outcome::Conflict,
                "Committed generation is invalid, incomplete or exceeds review limits",
            );
            None
        }
    };
    if let Err(e) = model::validate(settings) {
        report.add("settings", Outcome::Conflict, e.to_string());
        return report;
    }
    let configured =
        normalizer::has_bindings(settings) && !crate::dynamic::wan_nodes(settings).is_empty();
    if !configured {
        report.add(
            "source-setup",
            Outcome::Unavailable,
            "Configure WAN roots and subscriber sources before end-to-end validation",
        );
    }
    if settings.core_lists.bindings.is_empty() {
        report.add(
            "core-setup",
            Outcome::Unavailable,
            "Configure exact WAN, core address-list and routing-table mappings",
        );
    }
    if !configured {
        return report;
    }
    for router in required_routers(settings) {
        if errors.contains_key(router)
            || !inventories.get(router).is_some_and(|i| {
                i.router_id == router
                    && now >= i.collected_at
                    && now - i.collected_at <= normalizer::MAX_INVENTORY_AGE_SECONDS
                    && !i.errors.contains_key("collection")
            })
        {
            report.add(
                "inventory-health",
                Outcome::Unavailable,
                "Required router inventory is missing, failed, stale or inconsistent",
            );
            return report;
        }
    }
    report.add(
        "inventory-health",
        Outcome::Pass,
        "Required router snapshots are current at the review timestamp",
    );
    let mut subscribers = match normalizer::normalize(settings, inventories, errors, now) {
        Ok(s) => s,
        Err(e) => {
            report.add("subscriber-speeds", Outcome::Unavailable, e.to_string());
            return report;
        }
    };
    report.subscribers = subscribers.len();
    report.add(
        "subscriber-speeds",
        Outcome::Pass,
        "Selected subscribers have unique identities, addresses and finite speeds",
    );
    let empty = assignment::AssignmentState::default();
    let previous = generation
        .as_ref()
        .map(|g| &g.assignments)
        .unwrap_or(&empty);
    let proposed = (|| -> anyhow::Result<_> {
        if origin == Origin::Live && crate::dynamic::count_mode(settings) {
            Ok(crate::sessions::plan(
                settings,
                &mut subscribers,
                previous,
                crate::sessions::load(root)?.as_ref(),
                inventories,
                errors,
                now,
            )?
            .assignments)
        } else {
            assignment::assign(
                settings,
                &mut subscribers,
                previous,
                inventories,
                errors,
                now,
            )
        }
    })();
    let assignments = match proposed {
        Ok(a) => a,
        Err(e) => {
            report.add("subscriber-assignment", Outcome::Conflict, e.to_string());
            return report;
        }
    };
    let topology = if settings.dynamic.is_none() {
        Some(assignment::Topology::new(settings).expect("validated topology"))
    } else {
        None
    };
    let mut coverage = BTreeMap::<String, usize>::new();
    for s in &subscribers {
        let wan = if let Some(t) = &topology {
            t.root(&s.node).expect("validated root").id.as_str()
        } else {
            assignments.routes[&s.id].wan.as_str()
        };
        *coverage.entry(wan.into()).or_default() += 1;
    }
    report.coverage = coverage
        .into_iter()
        .map(|(wan, subscribers)| Coverage { wan, subscribers })
        .collect();
    report.add(
        "subscriber-assignment",
        Outcome::Pass,
        "Every selected subscriber has a compatible branch and explicit WAN root",
    );
    match crate::minimums::allocate_assigned(settings,&mut subscribers,&assignments) {
        Ok(allocation)=>report.add("subscriber-minimums",Outcome::Pass,format!("Effective minimums fit every ancestor; {} subscribers adjusted across {} constrained directions",allocation.adjusted_subscribers,allocation.constrained_directions)),
        Err(e)=>{report.add("subscriber-minimums",Outcome::Conflict,e.to_string());return report;}
    }
    let shaping = match model::compile_assigned(settings, &subscribers, &assignments) {
        Ok(p) => p,
        Err(e) => {
            report.add("shaping-capacity", Outcome::Conflict, e.to_string());
            return report;
        }
    };
    report.add(
        "shaping-capacity",
        Outcome::Pass,
        "Subscriber rates, parent hierarchy, computed capacities and WAN caps compile",
    );
    let core = if settings.core_lists.bindings.is_empty() {
        None
    } else {
        match core_plan::build(
            settings,
            &subscribers,
            &assignments,
            inventories,
            errors,
            now,
        ) {
            Ok(p) => {
                report.add("core-membership",Outcome::Pass,"Subscriber /32 ownership and supported list-to-table-to-WAN policy are consistent");
                Some(p)
            }
            Err(e) => {
                let diagnostics =
                    core_diagnostics::review(settings, inventories, errors, now, origin);
                let status = diagnostics
                    .cores
                    .iter()
                    .flat_map(|c| &c.findings)
                    .map(|f| f.status)
                    .max()
                    .unwrap_or(Outcome::Conflict);
                report.add("core-membership", status, e.to_string());
                None
            }
        }
    };
    if let Some(g) = generation {
        let mut actual: Vec<_> = g.subscribers.iter().collect();
        actual.sort_by(|a, b| a.id.cmp(&b.id));
        let mut desired: Vec<_> = subscribers.iter().collect();
        desired.sort_by(|a, b| a.id.cmp(&b.id));
        if actual != desired || g.assignments != assignments {
            report.add("snapshot-consistency",Outcome::Conflict,"Committed subscriber speeds, identities or branch assignments differ from current inputs");
        } else {
            report.add(
                "snapshot-consistency",
                Outcome::Pass,
                "Committed subscriber speeds and branch assignments match current inputs",
            );
        }
        for (name, matches) in [
            ("ShapedDevices.csv", g.csv == shaping.csv),
            ("network.json", g.network == shaping.network),
        ] {
            report.add(
                name,
                if matches {
                    Outcome::Pass
                } else {
                    Outcome::Conflict
                },
                if matches {
                    format!("Current output matches: {name}")
                } else {
                    format!("{name} differs from current subscriber/topology output")
                },
            );
        }
        if let Some(core) = core {
            if g.core.as_ref() == Some(&core) {
                report.add(
                    "core-plan.json",
                    Outcome::Pass,
                    "Committed core plan matches current routing, ownership and provenance",
                );
            } else {
                report.add(
                    "core-plan.json",
                    Outcome::Conflict,
                    "Committed core plan differs from current routing, ownership or provenance",
                );
            }
        }
        if store::current_generation(root).as_deref() != Some(g.generation.as_str()) {
            report.add(
                "generation-changed",
                Outcome::Unavailable,
                "Committed generation changed during review; reload validation",
            );
        }
    }
    report
}
