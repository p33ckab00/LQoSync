//! Independent source policies, exported nodes, and routing WANs.
use crate::{
    assignment::AssignmentState,
    model::{self, Node, Settings},
    selection::Selection,
};
use anyhow::{Context, Result, ensure};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap, HashSet};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Config {
    pub version: u8,
    pub wans: Vec<Wan>,
    pub policies: Vec<Policy>,
    #[serde(default)]
    pub pins: BTreeMap<String, DynamicPin>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub replacements: Vec<Replacement>,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Wan {
    pub id: String,
    pub name: String,
    pub router: String,
    pub interface: String,
    pub download: f64,
    pub upload: f64,
}
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Target {
    pub node: String,
    pub wan: String,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Policy {
    pub id: String,
    #[serde(default)]
    pub name: String,
    pub selections: Vec<String>,
    pub targets: Vec<Target>,
    pub default: Target,
    pub balance: bool,
    pub metric: String,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DynamicPin {
    pub policy: String,
    pub target: Target,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Replacement {
    pub policy: String,
    pub from: Target,
    pub to: Target,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Route {
    pub selection: String,
    pub policy: String,
    pub node: String,
    pub wan: String,
}

pub fn count_mode(s: &Settings) -> bool {
    if let Some(c) = &s.dynamic {
        c.policies
            .iter()
            .any(|p| p.balance && p.metric == "online_clients")
    } else {
        s.assignment
            .pools
            .iter()
            .any(|p| p.balance && p.metric == "online_clients")
    }
}
pub fn wan_nodes(s: &Settings) -> Vec<Node> {
    if let Some(c) = &s.dynamic {
        c.wans
            .iter()
            .map(|w| Node {
                id: w.id.clone(),
                name: w.name.clone(),
                parent: String::new(),
                kind: "wan".into(),
                router: w.router.clone(),
                source: w.interface.clone(),
                mode: "manual".into(),
                download: w.download,
                upload: w.upload,
                download_factor: 1.,
                upload_factor: 1.,
            })
            .collect()
    } else {
        s.nodes
            .iter()
            .filter(|n| n.kind == "wan")
            .cloned()
            .collect()
    }
}
pub fn validate(s: &Settings) -> Result<()> {
    let Some(c) = &s.dynamic else { return Ok(()) };
    ensure!(c.version == 1, "Unsupported dynamic topology version");
    ensure!(
        s.assignment.pools.is_empty() && s.assignment.pins.is_empty(),
        "Dynamic topology cannot mix legacy pools or pins"
    );
    ensure!(
        s.nodes
            .iter()
            .all(|n| n.router.is_empty() && n.source.is_empty()),
        "Dynamic source and WAN bindings must be independent of exported nodes"
    );
    let nodes: HashSet<_> = s.nodes.iter().map(|n| n.id.as_str()).collect();
    let routers: HashSet<_> = s.routers.iter().map(|r| r.id.as_str()).collect();
    let mut wans = HashSet::new();
    let mut interfaces = HashSet::new();
    for w in &c.wans {
        ensure!(
            model::valid_id(&w.id) && wans.insert(w.id.as_str()),
            "Invalid or duplicate routing WAN ID"
        );
        ensure!(
            !w.name.trim().is_empty()
                && routers.contains(w.router.as_str())
                && !w.interface.is_empty(),
            "Routing WAN requires a known router and interface"
        );
        ensure!(
            interfaces.insert((&w.router, &w.interface)),
            "A router interface cannot map to two routing WANs"
        );
        ensure!(
            [w.download, w.upload]
                .iter()
                .all(|v| v.is_finite() && *v > 0.),
            "Routing WAN capacities must be positive and finite"
        );
    }
    let selections: HashMap<_, _> = s.selections.iter().map(|x| (x.id.as_str(), x)).collect();
    let mut policy_ids = HashSet::new();
    let mut owned = HashSet::new();
    let mut policies = HashMap::new();
    for p in &c.policies {
        ensure!(
            model::valid_id(&p.id) && policy_ids.insert(&p.id),
            "Invalid or duplicate assignment policy ID"
        );
        ensure!(
            p.name.len() <= 256 && !p.name.chars().any(char::is_control),
            "Policy name is invalid"
        );
        ensure!(
            matches!(p.metric.as_str(), "online_clients" | "planned_load"),
            "Unknown assignment metric"
        );
        ensure!(
            !p.selections.is_empty() && !p.targets.is_empty() && p.targets.contains(&p.default),
            "Policy requires sources, destinations, and an eligible default"
        );
        for id in &p.selections {
            ensure!(
                owned.insert(id),
                "Source selection belongs to more than one policy"
            );
            ensure!(
                selections.get(id.as_str()).is_some_and(|x| !x.exclude),
                "Policy refers to an excluded or unknown selection"
            );
        }
        let mut targets = HashSet::new();
        for t in &p.targets {
            ensure!(
                nodes.contains(t.node.as_str())
                    && wans.contains(t.wan.as_str())
                    && targets.insert(t),
                "Unknown or duplicate node/WAN destination"
            );
        }
        policies.insert(p.id.as_str(), p);
    }
    ensure!(
        s.selections
            .iter()
            .filter(|x| !x.exclude)
            .all(|x| owned.contains(&x.id)),
        "Every selected source requires one assignment policy"
    );
    let index = replacement_index(c);
    let mut transitions = HashSet::new();
    for r in &c.replacements {
        ensure!(
            transitions.insert((&r.policy, &r.from)) && r.from != r.to,
            "Duplicate or circular destination replacement"
        );
        ensure!(
            [&r.from.node, &r.from.wan, &r.to.node, &r.to.wan]
                .iter()
                .all(|id| model::valid_id(id)),
            "Invalid replacement identity"
        );
        let p = policies
            .get(r.policy.as_str())
            .context("Replacement policy no longer exists")?;
        resolve_with(&index, p, &r.from)?;
    }
    for (id, pin) in &c.pins {
        uuid::Uuid::parse_str(id).context("A subscriber pin requires a normalized UUID")?;
        ensure!(
            policies
                .get(pin.policy.as_str())
                .is_some_and(|p| p.targets.contains(&pin.target)),
            "Pin target is outside its policy"
        );
    }
    Ok(())
}
fn root<'a>(s: &'a Settings, id: &str) -> Result<&'a Node> {
    let mut n = s
        .nodes
        .iter()
        .find(|n| n.id == id)
        .context("Legacy node no longer exists")?;
    for _ in 0..=256 {
        if n.parent.is_empty() {
            return Ok(n);
        }
        n = s
            .nodes
            .iter()
            .find(|p| p.id == n.parent)
            .context("Missing legacy parent")?;
    }
    anyhow::bail!("Legacy hierarchy exceeds depth limit")
}
pub fn migrate(s: &Settings) -> Result<Settings> {
    model::validate(s)?;
    if s.dynamic.is_some() {
        return Ok(s.clone());
    }
    let mut next = s.clone();
    let mut policies = vec![];
    for n in &s.nodes {
        if crate::normalizer::service(&n.kind).is_some()
            && !n.router.is_empty()
            && !n.source.is_empty()
        {
            let key = serde_json::to_vec(&(&n.router, &n.kind, "group", &n.source))?;
            next.selections.push(Selection {
                id: uuid::Uuid::new_v5(&uuid::Uuid::NAMESPACE_OID, &key).to_string(),
                router: n.router.clone(),
                kind: n.kind.clone(),
                selector: "group".into(),
                value: n.source.clone(),
                node: n.id.clone(),
                exclude: false,
            });
        }
    }
    let mut grouped: BTreeMap<&str, Vec<String>> = BTreeMap::new();
    for x in &next.selections {
        if !x.exclude {
            grouped.entry(&x.node).or_default().push(x.id.clone());
        }
    }
    for (node, selections) in grouped {
        let pool = s.assignment.pools.iter().find(|p| p.source_node == node);
        let targets = pool
            .map(|p| p.targets.clone())
            .unwrap_or_else(|| vec![node.into()]);
        let target = |id: &str| -> Result<Target> {
            Ok(Target {
                node: id.into(),
                wan: root(s, id)?.id.clone(),
            })
        };
        policies.push(Policy {
            id: node.into(),
            name: s
                .nodes
                .iter()
                .find(|n| n.id == node)
                .map(|n| n.name.clone())
                .unwrap_or_else(|| node.into()),
            selections,
            targets: targets.iter().map(|n| target(n)).collect::<Result<_>>()?,
            default: target(node)?,
            balance: pool.is_some_and(|p| p.balance),
            metric: pool
                .map(|p| p.metric.clone())
                .unwrap_or_else(|| "online_clients".into()),
        });
    }
    let wans = s
        .nodes
        .iter()
        .filter(|n| n.kind == "wan")
        .filter(|n| !n.router.is_empty() && !n.source.is_empty())
        .map(|n| Wan {
            id: n.id.clone(),
            name: n.name.clone(),
            router: n.router.clone(),
            interface: n.source.clone(),
            download: n.download,
            upload: n.upload,
        })
        .collect();
    let pins = s
        .assignment
        .pins
        .iter()
        .map(|(id, p)| {
            Ok((
                id.clone(),
                DynamicPin {
                    policy: p.source_node.clone(),
                    target: Target {
                        node: p.node.clone(),
                        wan: root(s, &p.node)?.id.clone(),
                    },
                },
            ))
        })
        .collect::<Result<_>>()?;
    for n in &mut next.nodes {
        n.router.clear();
        n.source.clear();
        if n.kind == "wan" {
            n.kind = "group".into();
        }
    }
    for x in &mut next.selections {
        x.node.clear();
    }
    next.assignment = Default::default();
    next.dynamic = Some(Config {
        version: 1,
        wans,
        policies,
        pins,
        replacements: vec![],
    });
    model::validate(&next)?;
    Ok(next)
}
pub fn validate_routes(state: &AssignmentState) -> Result<()> {
    for (id, r) in &state.routes {
        ensure!(
            [&r.selection, &r.policy, &r.node, &r.wan]
                .iter()
                .all(|id| model::valid_id(id)),
            "Invalid routing assignment identity"
        );
        ensure!(
            state
                .assignments
                .get(id)
                .is_some_and(|p| p.node == r.node && p.source_node == r.selection),
            "Routing assignment disagrees with its exported parent or policy"
        );
    }
    if !state.routes.is_empty() {
        ensure!(
            state.routes.len() == state.assignments.len(),
            "Routing assignments are incomplete"
        );
    }
    Ok(())
}

/// Translate legacy evidence only when its selected destination has one interpretation.
pub fn adopt(
    s: &Settings,
    rows: &[crate::model::Subscriber],
    old: &AssignmentState,
) -> Result<AssignmentState> {
    let c = s
        .dynamic
        .as_ref()
        .context("Dynamic topology is unavailable")?;
    validate_routes(old)?;
    let current: HashMap<_, _> = rows
        .iter()
        .map(|r| (r.id.as_str(), r.node.as_str()))
        .collect();
    let source_policies: HashMap<_, _> = c
        .policies
        .iter()
        .flat_map(|p| p.selections.iter().map(move |id| (id.as_str(), p)))
        .collect();
    let legacy_policies: HashMap<_, _> = c.policies.iter().map(|p| (p.id.as_str(), p)).collect();
    let replacements = replacement_index(c);
    let mut state = old.clone();
    for (id, pin) in &old.assignments {
        if old.routes.contains_key(id) {
            continue;
        }
        let source = if let Some(source) = current.get(id.as_str()) {
            (*source).to_string()
        } else {
            let policy = legacy_policies
                .get(pin.source_node.as_str())
                .or_else(|| source_policies.get(pin.source_node.as_str()))
                .context("Inactive legacy evidence has no unambiguous policy")?;
            if policy.selections.contains(&pin.source_node) {
                pin.source_node.clone()
            } else {
                ensure!(
                    policy.selections.len() == 1,
                    "Inactive legacy evidence has ambiguous source ownership; resolve it before migration"
                );
                policy.selections[0].clone()
            }
        };
        let p = source_policies
            .get(source.as_str())
            .context("Selected source has no policy")?;
        ensure!(
            pin.source_node == p.id || pin.source_node == source,
            "Legacy evidence source has an ambiguous policy"
        );
        let matches: std::collections::BTreeSet<_> = p
            .targets
            .iter()
            .chain(
                c.replacements
                    .iter()
                    .filter(|r| r.policy == p.id)
                    .map(|r| &r.from),
            )
            .filter(|t| t.node == pin.node)
            .cloned()
            .collect();
        ensure!(
            matches.len() == 1,
            "Legacy destination has an ambiguous WAN; restore its mapping before migration"
        );
        let target = resolve_with(&replacements, p, matches.first().expect("one target"))?;
        state.assignments.insert(
            id.clone(),
            crate::assignment::Pin {
                source_node: source.clone(),
                node: target.node.clone(),
            },
        );
        state.routes.insert(
            id.clone(),
            Route {
                selection: source.clone(),
                policy: p.id.clone(),
                node: target.node,
                wan: target.wan,
            },
        );
    }
    Ok(state)
}
pub fn assign_targets(
    s: &Settings,
    rows: &mut [crate::model::Subscriber],
    previous: &AssignmentState,
) -> Result<AssignmentState> {
    let c = s
        .dynamic
        .as_ref()
        .context("Dynamic topology is unavailable")?;
    let previous = adopt(s, rows, previous)?;
    let policies: HashMap<_, _> = c
        .policies
        .iter()
        .flat_map(|p| p.selections.iter().map(move |id| (id.as_str(), p)))
        .collect();
    let wans: HashMap<_, _> = c.wans.iter().map(|w| (w.id.as_str(), w)).collect();
    let mut planned = rows.to_vec();
    let replacements = replacement_index(c);
    let nodes: HashMap<_, _> = s.nodes.iter().map(|n| (n.id.as_str(), n)).collect();
    let mut state = AssignmentState::default();
    for (id, r) in &previous.routes {
        if let Some(p) = policies.get(r.selection.as_str()) {
            let target = resolve_with(
                &replacements,
                p,
                &Target {
                    node: r.node.clone(),
                    wan: r.wan.clone(),
                },
            )?;
            ensure!(
                r.policy == p.id
                    || p.targets.contains(&Target {
                        node: r.node.clone(),
                        wan: r.wan.clone()
                    }),
                "A changed source policy must preserve its assigned destination or provide an explicit replacement"
            );
            state.routes.insert(
                id.clone(),
                Route {
                    node: target.node.clone(),
                    wan: target.wan,
                    policy: p.id.clone(),
                    ..r.clone()
                },
            );
            state.assignments.insert(
                id.clone(),
                crate::assignment::Pin {
                    source_node: r.selection.clone(),
                    node: target.node,
                },
            );
        }
    }
    let mut wan_load = HashMap::<String, (usize, f64, f64)>::new();
    let mut node_load = HashMap::<String, (usize, f64, f64)>::new();
    let mut pending = vec![];
    let mut seen = HashSet::new();
    let mut put = |row: &mut crate::model::Subscriber,
                   source: &str,
                   p: &Policy,
                   target: &Target,
                   state: &mut AssignmentState|
     -> Result<()> {
        let w = wan_load.entry(target.wan.clone()).or_default();
        w.0 += 1;
        w.1 += row.download;
        w.2 += row.upload;
        let n = node_load.entry(target.node.clone()).or_default();
        n.0 += 1;
        n.1 += row.download;
        n.2 += row.upload;
        ensure!(
            [w.1, w.2, n.1, n.2].iter().all(|v| v.is_finite()),
            "Assignment load overflow"
        );
        row.node = target.node.clone();
        state.assignments.insert(
            row.id.clone(),
            crate::assignment::Pin {
                source_node: source.into(),
                node: target.node.clone(),
            },
        );
        state.routes.insert(
            row.id.clone(),
            Route {
                selection: source.into(),
                policy: p.id.clone(),
                node: target.node.clone(),
                wan: target.wan.clone(),
            },
        );
        Ok(())
    };
    for (i, row) in planned.iter_mut().enumerate() {
        ensure!(
            seen.insert(row.id.clone()),
            "Duplicate assignment subscriber identity"
        );
        ensure!(
            [row.download, row.upload]
                .iter()
                .all(|v| v.is_finite() && *v > 0.),
            "Invalid subscriber assignment rates"
        );
        let source = row.node.clone();
        let p = policies
            .get(source.as_str())
            .context("Selected source has no assignment policy")?;
        let target = if let Some(pin) = c.pins.get(&row.id) {
            ensure!(
                pin.policy == p.id && p.targets.contains(&pin.target),
                "Pinned subscriber changed policy; explicitly replace its pin"
            );
            Some(pin.target.clone())
        } else if let Some(old) = previous
            .routes
            .get(&row.id)
            .filter(|r| r.selection == source)
        {
            let target = Target {
                node: old.node.clone(),
                wan: old.wan.clone(),
            };
            ensure!(
                old.policy == p.id || p.targets.contains(&target),
                "A changed source policy must preserve its active destination"
            );
            Some(resolve_with(&replacements, p, &target)?)
        } else if !p.balance {
            Some(p.default.clone())
        } else {
            None
        };
        if let Some(target) = target {
            put(row, &source, p, &target, &mut state)?;
        } else {
            pending.push((i, source));
        }
    }
    pending.sort_by(|(a, _), (b, _)| planned[*a].id.cmp(&planned[*b].id));
    for (i, source) in pending {
        let row = &mut planned[i];
        let p = policies[source.as_str()];
        let mut best: Option<(f64, usize, &Target)> = None;
        for t in &p.targets {
            let w = wan_load.get(&t.wan).copied().unwrap_or_default();
            let n = node_load.get(&t.node).copied().unwrap_or_default();
            let score = if p.metric == "online_clients" {
                w.0 as f64
            } else {
                let cap = wans[t.wan.as_str()];
                let score =
                    ((w.1 + row.download) / cap.download).max((w.2 + row.upload) / cap.upload);
                let node = nodes[t.node.as_str()];
                if node.mode == "manual" {
                    score.max(
                        ((n.1 + row.download) / node.download)
                            .max((n.2 + row.upload) / node.upload),
                    )
                } else {
                    score
                }
            };
            ensure!(score.is_finite(), "Assignment load overflow");
            let value = (score, if p.metric == "online_clients" { n.0 } else { 0 }, t);
            if best.is_none_or(|b| value < b) {
                best = Some(value);
            }
        }
        let target = best.context("Empty assignment policy")?.2;
        let w = wan_load.entry(target.wan.clone()).or_default();
        w.0 += 1;
        w.1 += row.download;
        w.2 += row.upload;
        let n = node_load.entry(target.node.clone()).or_default();
        n.0 += 1;
        n.1 += row.download;
        n.2 += row.upload;
        ensure!(
            [w.1, w.2, n.1, n.2].iter().all(|v| v.is_finite()),
            "Assignment load overflow"
        );
        row.node = target.node.clone();
        state.assignments.insert(
            row.id.clone(),
            crate::assignment::Pin {
                source_node: source.clone(),
                node: target.node.clone(),
            },
        );
        state.routes.insert(
            row.id.clone(),
            Route {
                selection: source,
                policy: p.id.clone(),
                node: target.node.clone(),
                wan: target.wan.clone(),
            },
        );
    }
    validate_routes(&state)?;
    rows.clone_from_slice(&planned);
    Ok(state)
}

type ReplacementIndex<'a> = HashMap<(&'a str, &'a Target), &'a Target>;
fn replacement_index(config: &Config) -> ReplacementIndex<'_> {
    config
        .replacements
        .iter()
        .map(|r| ((r.policy.as_str(), &r.from), &r.to))
        .collect()
}
fn resolve_with(index: &ReplacementIndex<'_>, policy: &Policy, target: &Target) -> Result<Target> {
    let mut current = target.clone();
    let mut seen = HashSet::new();
    while !policy.targets.contains(&current) {
        ensure!(
            seen.insert(current.clone()) && seen.len() <= 256,
            "Destination replacement contains a cycle or exceeds its limit"
        );
        current = (*index
            .get(&(policy.id.as_str(), &current))
            .context("An active assignment requires an explicit destination replacement")?)
        .clone();
    }
    Ok(current)
}
