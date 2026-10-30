//! Pure preview decisions. No transport, journal executor, or live writer calls this module.
//! Callers must discard cleanup evidence on any Err; failed reads cannot bridge absence confirmations.
use crate::{
    assignment::AssignmentState,
    core_migration::Member,
    core_plan,
    dynamic::{self, Target},
    model::{self, Settings},
    routeros::Inventory,
    sessions,
    wan_health::{HealthStatus, State},
};
use anyhow::{Context, Result, ensure};
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, BTreeSet, HashMap, HashSet},
    sync::Arc,
};
const MEMBERS: &str = "/ip/firewall/address-list/print";
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Options {
    pub evacuate_active: bool,
    pub pin_failover: bool,
    pub offline_seconds: u64,
    pub evidence_seconds: u64,
}
impl Default for Options {
    fn default() -> Self {
        Self {
            evacuate_active: true,
            pin_failover: false,
            offline_seconds: 30,
            evidence_seconds: 30,
        }
    }
}
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Cleanup {
    pub absent: BTreeMap<String, Absence>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Absence {
    context: String,
    router: String,
    source: String,
    first_at: u64,
    last_at: u64,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum Decision {
    Keep {
        subscriber: String,
        router: String,
        member: Member,
    },
    Add {
        subscriber: String,
        router: String,
        member: Member,
    },
    Move {
        subscriber: String,
        router: String,
        before: Member,
        after: Member,
    },
    Remove {
        subscriber: String,
        router: String,
        member: Member,
    },
    Blocked {
        subscriber: String,
        reason: String,
    },
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ReconcilePlan {
    pub revision: String,
    pub settings_revision: String,
    pub captured_at: u64,
    pub assignments: AssignmentState,
    pub decisions: Vec<Decision>,
    pub cleanup: Cleanup,
}
pub struct Input<'a> {
    pub settings: &'a Settings,
    pub inventories: &'a BTreeMap<String, Arc<Inventory>>,
    pub observation: &'a sessions::Observation,
    pub previous: Option<&'a sessions::Observation>,
    pub health: &'a BTreeMap<String, State>,
    pub cleanup: &'a Cleanup,
    pub options: Options,
    pub now: u64,
}
fn digest(value: &impl Serialize) -> Result<String> {
    Ok(uuid::Uuid::new_v5(&uuid::Uuid::NAMESPACE_OID, &serde_json::to_vec(value)?).to_string())
}
fn fresh(at: u64, now: u64, ttl: u64) -> bool {
    at <= now && now - at <= ttl
}
fn blocked(id: &str, reason: &str) -> Decision {
    Decision::Blocked {
        subscriber: id.into(),
        reason: reason.into(),
    }
}
struct Owned {
    router: String,
    wan: String,
    member: Member,
}
type InventoryIndex = (
    BTreeMap<String, Owned>,
    BTreeMap<u32, String>,
    Vec<(u32, u32)>,
);
fn inventory_rows(input: &Input<'_>) -> Result<InventoryIndex> {
    let mut owned = BTreeMap::new();
    let mut ips = BTreeMap::new();
    let mut protected = vec![];
    let mut grouped: BTreeMap<&str, Vec<&core_plan::Binding>> = BTreeMap::new();
    for b in &input.settings.core_lists.bindings {
        grouped.entry(&b.router).or_default().push(b);
    }
    let dynamic = input
        .settings
        .dynamic
        .as_ref()
        .context("Dynamic topology required")?;
    let interfaces: HashMap<_, _> = dynamic
        .wans
        .iter()
        .map(|w| (w.id.as_str(), w.interface.as_str()))
        .collect();
    let health_revision = crate::wan_health::settings_revision(input.settings)?;
    let unavailable: HashSet<_> = input
        .health
        .iter()
        .filter(|(id, h)| {
            h.settings_revision == health_revision
                && h.wan == **id
                && h.status_at(input.now, input.options.evidence_seconds) == HealthStatus::Unhealthy
        })
        .map(|(id, _)| id.as_str())
        .collect();
    for (router, bindings) in grouped {
        let inv = input
            .inventories
            .get(router)
            .context("Missing core inventory")?;
        ensure!(
            inv.router_id == router
                && fresh(inv.collected_at, input.now, input.options.evidence_seconds)
                && inv.errors.is_empty(),
            "Stale or incomplete core inventory"
        );
        for path in core_plan::CORE_TABLES {
            ensure!(
                inv.tables.contains_key(*path),
                "Missing required core table"
            );
        }
        for binding in &bindings {
            let healthy = input.health.get(&binding.wan).is_some_and(|h| {
                h.wan == binding.wan
                    && h.settings_revision == health_revision
                    && h.status_at(input.now, input.options.evidence_seconds)
                        == HealthStatus::Healthy
            });
            if healthy {
                let name = interfaces
                    .get(binding.wan.as_str())
                    .context("Missing destination interface mapping")?;
                let matches = inv.tables["/interface/print"]
                    .iter()
                    .filter(|r| r.get("name").is_some_and(|v| v == name))
                    .collect::<Vec<_>>();
                ensure!(
                    matches.len() == 1
                        && crate::core_routes::flag(matches[0], "running", false)?
                        && !crate::core_routes::flag(matches[0], "disabled", false)?,
                    "Healthy destination interface is down, disabled or ambiguous"
                );
            }
        }
        let analysis =
            crate::core_analysis::analyze_available(inv, &bindings, &interfaces, &unavailable);
        ensure!(
            analysis.findings.is_empty(),
            "Core structural/destination route validation failed: {}",
            analysis
                .findings
                .first()
                .map(|f| f.message.as_str())
                .unwrap_or("")
        );
        let lists: HashMap<_, _> = bindings.iter().map(|b| (b.list.as_str(), *b)).collect();
        let mut ids = HashSet::new();
        for r in &inv.tables[MEMBERS] {
            let id = r.get(".id").context("Missing address-list ID")?;
            ensure!(
                id.strip_prefix('*').is_some_and(|v| !v.is_empty()
                    && v.len() <= 32
                    && v.bytes().all(|c| c.is_ascii_hexdigit()))
                    && ids.insert(id),
                "Invalid or duplicate core row ID"
            );
            let Some(b) = r.get("list").and_then(|l| lists.get(l.as_str())) else {
                continue;
            };
            let address = r.get("address").context("Missing list address")?;
            let comment = r.get("comment").cloned().unwrap_or_default();
            if let Some((subscriber, wan)) =
                core_plan::owned(&comment, &input.settings.core_lists.owner)?
            {
                ensure!(
                    wan == b.wan
                        && !crate::core_routes::flag(r, "disabled", false)?
                        && !crate::core_routes::flag(r, "dynamic", false)?
                        && r.get("timeout").is_none_or(|v| v.is_empty() || v == "0s"),
                    "Owned row scope or flags are inconsistent"
                );
                let ip =
                    core_plan::host(address).context("Owned address must be an individual host")?;
                ensure!(
                    ips.insert(ip, subscriber.into()).is_none(),
                    "Duplicate owned IP"
                );
                let member = Member {
                    id: id.clone(),
                    list: b.list.clone(),
                    address: address.clone(),
                    comment: comment.clone(),
                    dynamic: false,
                    disabled: false,
                    timeout: r.get("timeout").cloned(),
                };
                ensure!(
                    owned
                        .insert(
                            subscriber.into(),
                            Owned {
                                router: router.into(),
                                wan: wan.into(),
                                member
                            }
                        )
                        .is_none(),
                    "Duplicate owned subscriber"
                );
            } else if !crate::core_routes::flag(r, "disabled", false)? {
                protected.push(core_plan::span(address).context("Unresolved protected address")?);
            }
        }
    }
    // Prefix maxima support O(log N) overlap checks across all configured lists/cores.
    protected.sort_unstable();
    let mut last = 0;
    for (_, end) in &mut protected {
        last = last.max(*end);
        *end = last;
    }
    Ok((owned, ips, protected))
}
pub fn plan(input: Input<'_>) -> Result<ReconcilePlan> {
    let s = input.settings;
    model::validate(s)?;
    sessions::validate(input.observation)?;
    ensure!(
        (1..=3600).contains(&input.options.offline_seconds)
            && (1..=300).contains(&input.options.evidence_seconds),
        "Invalid reconciliation timing"
    );
    let config = s
        .dynamic
        .as_ref()
        .context("Reconciliation requires dynamic topology")?;
    ensure!(!s.core_lists.bindings.is_empty(), "Core bindings required");
    if let Some(old) = input.previous {
        sessions::validate(old)?;
    }
    let mut rows = crate::normalizer::collect(s, input.inventories, &BTreeMap::new(), input.now)?;
    let source_routers = crate::normalizer::selected_routers(s);
    let source_index: HashMap<_, _> = s
        .selections
        .iter()
        .filter(|v| !v.exclude)
        .map(|v| (v.id.as_str(), v.router.as_str()))
        .collect();
    for router in &source_routers {
        let router = *router;
        let inv = input
            .inventories
            .get(router)
            .context("Missing selected source")?;
        ensure!(
            inv.errors.is_empty()
                && fresh(inv.collected_at, input.now, input.options.evidence_seconds)
                && input.observation.collected.get(router) == Some(&inv.collected_at),
            "Source observation is incomplete, stale or differs from inventory"
        );
    }
    let settings_revision = digest(s)?;
    let (owned, owned_ips, protected) = inventory_rows(&input)?;
    let healthy: BTreeSet<_> = input
        .health
        .iter()
        .filter(|(id, h)| {
            h.settings_revision == settings_revision
                && h.wan == **id
                && h.status_at(input.now, input.options.evidence_seconds) == HealthStatus::Healthy
        })
        .map(|(id, _)| id.clone())
        .collect();
    let failed: BTreeSet<_> = input
        .health
        .iter()
        .filter(|(id, h)| {
            h.settings_revision == settings_revision
                && h.wan == **id
                && h.status_at(input.now, input.options.evidence_seconds) == HealthStatus::Unhealthy
        })
        .map(|(id, _)| id.clone())
        .collect();
    let policies: HashMap<_, _> = config
        .policies
        .iter()
        .flat_map(|p| p.selections.iter().map(move |id| (id.as_str(), p)))
        .collect();
    let mut working = s.clone();
    let c = working.dynamic.as_mut().unwrap();
    c.pins.clear();
    for p in &mut c.policies {
        p.balance = true;
    }
    let mut reasons = BTreeMap::new();
    let mut excluded = BTreeSet::new();
    let mut active = BTreeSet::new();
    for row in &rows {
        ensure!(
            active.insert(row.id.clone()),
            "Duplicate selected subscriber"
        );
        let p = policies
            .get(row.node.as_str())
            .context("No source policy")?;
        let prior = input
            .previous
            .and_then(|old| old.assignments.routes.get(&row.id));
        let continuous = prior.is_some_and(|r| r.selection == row.node)
            && (p.metric == "planned_load"
                || !p.balance
                || input
                    .previous
                    .and_then(|old| old.active.get(&row.id))
                    .is_none_or(|old| {
                        input
                            .observation
                            .active
                            .get(&row.id)
                            .is_some_and(|new| sessions::continuous(old, new))
                    }));
        if p.balance && p.metric == "online_clients" {
            let session = input
                .observation
                .active
                .get(&row.id)
                .context("Online source lacks session evidence")?;
            ensure!(
                session.source_node == row.node
                    && fresh(
                        session.collected_at,
                        input.now,
                        input.options.evidence_seconds
                    ),
                "Session source differs from selection"
            );
        }
        let pin = config.pins.get(&row.id);
        let fixed = if let Some(pin) = pin {
            ensure!(
                pin.policy == p.id && p.targets.contains(&pin.target),
                "Invalid manual pin"
            );
            Some(pin.target.clone())
        } else if continuous {
            prior.map(|r| Target {
                node: r.node.clone(),
                wan: r.wan.clone(),
            })
        } else if !p.balance {
            Some(p.default.clone())
        } else {
            None
        };
        let mut retained = fixed.clone();
        if let Some(target) = &fixed {
            ensure!(
                p.targets.contains(target),
                "Previous target is no longer eligible"
            );
            if failed.contains(&target.wan) && input.options.evacuate_active {
                if pin.is_some() && !input.options.pin_failover {
                    reasons.insert(
                        row.id.clone(),
                        "Pinned WAN failed; override is disabled".to_string(),
                    );
                } else {
                    retained = None;
                }
            } else if !continuous && !healthy.contains(&target.wan) {
                reasons.insert(
                    row.id.clone(),
                    "Requested destination has no fresh healthy evidence".into(),
                );
            }
        }
        if retained.is_none() && !p.targets.iter().any(|t| healthy.contains(&t.wan)) {
            reasons.insert(row.id.clone(), "No eligible healthy destination".into());
            retained = fixed.or_else(|| {
                prior.map(|r| Target {
                    node: r.node.clone(),
                    wan: r.wan.clone(),
                })
            });
            if retained.is_none() {
                excluded.insert(row.id.clone());
            }
        }
        if let Some(target) = retained {
            c.pins.insert(
                row.id.clone(),
                dynamic::DynamicPin {
                    policy: p.id.clone(),
                    target,
                },
            );
        }
    }
    rows.retain(|r| !excluded.contains(&r.id));
    let assignments = dynamic::assign_targets_eligible(&working, &mut rows, &healthy)?;
    let bindings: HashMap<_, _> = s
        .core_lists
        .bindings
        .iter()
        .map(|b| (b.wan.as_str(), b))
        .collect();
    let mut decisions = vec![];
    for id in &excluded {
        decisions.push(blocked(id, &reasons[id]));
    }
    for row in &rows {
        if let Some(reason) = reasons.get(&row.id) {
            decisions.push(blocked(&row.id, reason));
            continue;
        }
        let route = &assignments.routes[&row.id];
        let binding = bindings
            .get(route.wan.as_str())
            .context("Target has no core binding")?;
        let ip = core_plan::host(&row.ipv4)?;
        let n = protected.partition_point(|(start, _)| *start <= ip);
        if n > 0 && protected[n - 1].1 >= ip {
            decisions.push(blocked(
                &row.id,
                "Protected address coverage overlaps this client",
            ));
            continue;
        }
        if owned_ips.get(&ip).is_some_and(|id| id != &row.id) {
            decisions.push(blocked(&row.id, "IP is owned by another subscriber"));
            continue;
        }
        let mut after = Member {
            id: String::new(),
            list: binding.list.clone(),
            address: format!("{}/32", row.ipv4),
            comment: core_plan::marker(&s.core_lists.owner, &row.id, &route.wan),
            dynamic: false,
            disabled: false,
            timeout: None,
        };
        match owned.get(&row.id) {
            None => {
                if healthy.contains(&route.wan) {
                    decisions.push(Decision::Add {
                        subscriber: row.id.clone(),
                        router: binding.router.clone(),
                        member: after,
                    });
                } else {
                    decisions.push(blocked(
                        &row.id,
                        "Missing owned host cannot be placed on an unverified WAN",
                    ));
                }
            }
            Some(old) => {
                if core_plan::host(&old.member.address)? != ip {
                    decisions.push(blocked(
                        &row.id,
                        "IP change requires a guarded multi-operation transition",
                    ));
                    continue;
                }
                if old.router != binding.router {
                    decisions.push(blocked(&row.id, "Cross-core moves are unsupported"));
                    continue;
                }
                if old.wan == route.wan {
                    decisions.push(Decision::Keep {
                        subscriber: row.id.clone(),
                        router: old.router.clone(),
                        member: old.member.clone(),
                    });
                } else if healthy.contains(&route.wan) {
                    after.id = old.member.id.clone();
                    after.address = old.member.address.clone();
                    after.timeout = old.member.timeout.clone();
                    decisions.push(Decision::Move {
                        subscriber: row.id.clone(),
                        router: old.router.clone(),
                        before: old.member.clone(),
                        after,
                    });
                } else {
                    decisions.push(blocked(&row.id, "Move destination is not healthy"));
                }
            }
        }
    }
    let mut cleanup = Cleanup::default();
    for (id, old) in &owned {
        if active.contains(id) {
            continue;
        }
        if active.is_empty() {
            decisions.push(blocked(
                id,
                "Wholly empty source scan cannot authorize cleanup",
            ));
            continue;
        }
        let old_receipt = input.cleanup.absent.get(id);
        let source = input
            .previous
            .and_then(|p| p.active.get(id).map(|v| v.source_node.clone()))
            .or_else(|| {
                input
                    .previous
                    .and_then(|p| p.assignments.routes.get(id).map(|r| r.selection.clone()))
            })
            .or_else(|| old_receipt.map(|r| r.source.clone()));
        let source_router = source
            .as_deref()
            .and_then(|source| source_index.get(source).map(|router| router.to_string()));
        let Some(router) = source_router else {
            decisions.push(blocked(
                id,
                "No prior session source for offline confirmation",
            ));
            continue;
        };
        if !source_routers.contains(router.as_str()) {
            decisions.push(blocked(id, "Offline source is no longer selected"));
            continue;
        }
        let at = *input
            .observation
            .collected
            .get(&router)
            .context("No offline source timestamp")?;
        let source = source.context("Missing offline selection")?;
        let context = digest(&(&settings_revision, id, &old.member, &router, &source))?;
        let receipt = old_receipt.filter(|r| {
            r.context == context
                && at >= r.last_at
                && at - r.last_at <= input.options.evidence_seconds
                && r.first_at <= r.last_at
        });
        let first = receipt.map_or(at, |r| r.first_at);
        if receipt.is_some_and(|r| at > r.last_at) && at - first >= input.options.offline_seconds {
            decisions.push(Decision::Remove {
                subscriber: id.clone(),
                router: old.router.clone(),
                member: old.member.clone(),
            });
        } else {
            cleanup.absent.insert(
                id.clone(),
                Absence {
                    context,
                    router,
                    source,
                    first_at: first,
                    last_at: at,
                },
            );
            decisions.push(blocked(id, "Waiting for confirmed offline grace period"));
        }
    }
    decisions.sort_by_key(|d| match d {
        Decision::Keep { subscriber, .. }
        | Decision::Add { subscriber, .. }
        | Decision::Move { subscriber, .. }
        | Decision::Remove { subscriber, .. }
        | Decision::Blocked { subscriber, .. } => subscriber.clone(),
    });
    let revision = digest(&(
        &settings_revision,
        input.now,
        &assignments,
        &decisions,
        &cleanup,
    ))?;
    Ok(ReconcilePlan {
        revision,
        settings_revision,
        captured_at: input.now,
        assignments,
        decisions,
        cleanup,
    })
}
