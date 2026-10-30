use crate::{
    model::{Node, Settings, Subscriber},
    normalizer,
    routeros::Inventory,
};
use anyhow::{Context, Result, ensure};
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, HashMap, HashSet},
    sync::Arc,
};

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Policy {
    pub pools: Vec<Pool>,
    pub pins: BTreeMap<String, Pin>,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Pool {
    pub source_node: String,
    pub targets: Vec<String>,
    #[serde(default)]
    pub balance: bool,
    #[serde(default = "default_metric")]
    pub metric: String,
}
fn default_metric() -> String {
    "online_clients".into()
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Pin {
    pub source_node: String,
    pub node: String,
}
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct AssignmentState {
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub routes: BTreeMap<String, crate::dynamic::Route>,
    pub assignments: BTreeMap<String, Pin>,
}

pub struct Topology<'a> {
    pub nodes: HashMap<&'a str, &'a Node>,
    pub roots: HashMap<&'a str, &'a str>,
    sources: HashSet<&'a str>,
}
impl<'a> Topology<'a> {
    pub fn new(settings: &'a Settings) -> Result<Self> {
        let nodes: HashMap<_, _> = settings.nodes.iter().map(|n| (n.id.as_str(), n)).collect();
        ensure!(
            nodes.len() == settings.nodes.len(),
            "Duplicate topology node"
        );
        let mut roots = HashMap::new();
        for n in &settings.nodes {
            let mut cursor = n;
            let mut seen = HashSet::new();
            while !cursor.parent.is_empty() {
                ensure!(
                    seen.insert(cursor.id.as_str()) && seen.len() <= 256,
                    "Invalid topology path"
                );
                cursor = nodes
                    .get(cursor.parent.as_str())
                    .context("Missing topology parent")?;
            }
            ensure!(
                cursor.kind == "wan" && cursor.mode == "manual",
                "Assignment root must be an explicit WAN"
            );
            roots.insert(n.id.as_str(), cursor.id.as_str());
        }
        let sources = settings
            .nodes
            .iter()
            .filter(|n| {
                normalizer::service(&n.kind).is_some()
                    && !n.router.is_empty()
                    && !n.source.is_empty()
            })
            .map(|n| n.id.as_str())
            .chain(
                settings
                    .selections
                    .iter()
                    .filter(|s| !s.exclude)
                    .map(|s| s.node.as_str()),
            )
            .collect();
        Ok(Self {
            nodes,
            roots,
            sources,
        })
    }
    pub fn root(&self, node: &str) -> Result<&'a Node> {
        let id = self.roots.get(node).context("Unknown assignment node")?;
        Ok(self.nodes[id])
    }
    pub fn compatible(&self, source: &str, target: &str) -> Result<()> {
        let source = self
            .nodes
            .get(source)
            .context("Assignment source no longer exists")?;
        let target = self
            .nodes
            .get(target)
            .context("Assignment target no longer exists")?;
        ensure!(
            self.sources.contains(source.id.as_str()),
            "Assignment source must have a subscriber source binding"
        );
        ensure!(
            source.kind == target.kind
                || matches!(source.kind.as_str(), "group" | "site" | "wan")
                || matches!(target.kind.as_str(), "group" | "site" | "wan"),
            "Assignment target must match the service or be a generic group"
        );
        ensure!(
            target.id == source.id || !self.sources.contains(target.id.as_str()),
            "Assignment target belongs to another subscriber source"
        );
        for node in [source, target] {
            let root = self.root(&node.id)?;
            ensure!(
                !root.router.is_empty() && !root.source.is_empty(),
                "Map the WAN interface before assigning subscribers"
            );
        }
        Ok(())
    }
}
pub fn eligible_targets(settings: &Settings, source: &str) -> Result<Vec<String>> {
    let topology = Topology::new(settings)?;
    let mut targets: Vec<_> = settings
        .nodes
        .iter()
        .filter(|n| topology.compatible(source, &n.id).is_ok())
        .map(|n| n.id.clone())
        .collect();
    targets.sort();
    Ok(targets)
}
pub fn validate(settings: &Settings) -> Result<()> {
    if settings.dynamic.is_some() {
        return crate::dynamic::validate(settings);
    }
    let mut interfaces = HashSet::new();
    for root in settings
        .nodes
        .iter()
        .filter(|n| n.kind == "wan" && !n.router.is_empty() && !n.source.is_empty())
    {
        ensure!(
            interfaces.insert((&root.router, &root.source)),
            "A router interface cannot map to two WAN roots"
        );
    }
    if settings.assignment.pools.is_empty() && settings.assignment.pins.is_empty() {
        return Ok(());
    }
    let topology = Topology::new(settings)?;
    let mut sources = HashSet::new();
    let mut pools = HashMap::new();
    for pool in &settings.assignment.pools {
        ensure!(
            matches!(pool.metric.as_str(), "online_clients" | "planned_load"),
            "Unknown assignment metric"
        );
        ensure!(
            sources.insert(pool.source_node.as_str()),
            "Duplicate assignment pool"
        );
        ensure!(
            pool.targets.contains(&pool.source_node),
            "Pool must retain its original source node"
        );
        let mut seen = HashSet::new();
        for target in &pool.targets {
            ensure!(seen.insert(target), "Duplicate pool target");
            topology.compatible(&pool.source_node, target)?;
        }
        pools.insert(pool.source_node.as_str(), pool);
    }
    for (id, pin) in &settings.assignment.pins {
        uuid::Uuid::parse_str(id)
            .context("A subscriber pin requires a normalized subscriber UUID")?;
        topology.compatible(&pin.source_node, &pin.node)?;
        ensure!(
            pin.node == pin.source_node
                || pools
                    .get(pin.source_node.as_str())
                    .is_some_and(|p| p.targets.contains(&pin.node)),
            "Pin target is outside its source pool"
        );
    }
    Ok(())
}
pub fn validate_wans(
    settings: &Settings,
    inventories: &BTreeMap<String, Arc<Inventory>>,
    errors: &BTreeMap<String, String>,
    now: u64,
) -> Result<()> {
    let mut indexed: HashMap<&str, HashMap<&str, Option<&crate::routeros::Row>>> = HashMap::new();
    let wan_nodes = crate::dynamic::wan_nodes(settings);
    for root in wan_nodes
        .iter()
        .filter(|n| !n.router.is_empty() && !n.source.is_empty())
    {
        ensure!(
            !errors.contains_key(&root.router),
            "Latest mapped WAN router read failed: {}",
            root.router
        );
        if !indexed.contains_key(root.router.as_str()) {
            let inv = inventories
                .get(&root.router)
                .context("Read mapped WAN router inventory first")?;
            ensure!(
                inv.router_id == root.router
                    && now >= inv.collected_at
                    && now - inv.collected_at <= normalizer::MAX_INVENTORY_AGE_SECONDS,
                "Mapped WAN inventory is stale or inconsistent"
            );
            ensure!(
                !inv.errors.contains_key("collection")
                    && !inv.errors.contains_key("/interface/print"),
                "Mapped WAN interface table is incomplete"
            );
            let rows = inv
                .tables
                .get("/interface/print")
                .context("Mapped WAN interface table is missing")?;
            let mut interfaces = HashMap::new();
            for row in rows {
                if let Some(name) = row.get("name") {
                    interfaces
                        .entry(name.as_str())
                        .and_modify(|v| *v = None)
                        .or_insert(Some(row));
                }
            }
            indexed.insert(&root.router, interfaces);
        }
        let interface = indexed[&root.router.as_str()]
            .get(root.source.as_str())
            .and_then(|v| *v)
            .context("Mapped WAN interface is missing or duplicated")?;
        ensure!(
            interface.get("running").is_some_and(|v| v == "true")
                && interface.get("disabled").is_some_and(|v| v == "false"),
            "Mapped WAN interface is down, disabled or has unknown status: {}",
            root.source
        );
    }
    Ok(())
}
fn add_load(
    load: &mut HashMap<String, (f64, f64, usize)>,
    key: &str,
    subscriber: &Subscriber,
) -> Result<()> {
    let value = load.entry(key.into()).or_default();
    value.0 += subscriber.download;
    value.1 += subscriber.upload;
    value.2 += 1;
    ensure!(
        value.0.is_finite() && value.1.is_finite(),
        "Assignment load overflow"
    );
    Ok(())
}
fn score(
    topology: &Topology<'_>,
    target: &str,
    subscriber: &Subscriber,
    wan_load: &HashMap<String, (f64, f64, usize)>,
    node_load: &HashMap<String, (f64, f64, usize)>,
) -> Result<f64> {
    let root = topology.root(target)?;
    let load = wan_load.get(&root.id).copied().unwrap_or_default();
    let mut score = ((load.0 + subscriber.download) / root.download)
        .max((load.1 + subscriber.upload) / root.upload);
    let node = topology.nodes[target];
    if node.mode == "manual" {
        let load = node_load.get(target).copied().unwrap_or_default();
        score = score.max(
            ((load.0 + subscriber.download) / node.download)
                .max((load.1 + subscriber.upload) / node.upload),
        );
    }
    ensure!(
        score.is_finite(),
        "Invalid assignment capacity or projected load"
    );
    Ok(score)
}
pub fn assign(
    settings: &Settings,
    subscribers: &mut [Subscriber],
    previous: &AssignmentState,
    inventories: &BTreeMap<String, Arc<Inventory>>,
    errors: &BTreeMap<String, String>,
    now: u64,
) -> Result<AssignmentState> {
    validate(settings)?;
    validate_wans(settings, inventories, errors, now)?;
    if settings.dynamic.is_some() {
        return crate::dynamic::assign_targets(settings, subscribers, previous);
    }
    let topology = Topology::new(settings)?;
    let pools: HashMap<_, _> = settings
        .assignment
        .pools
        .iter()
        .map(|p| (p.source_node.as_str(), p))
        .collect();
    // The session planner releases disconnected count-mode clients before this step.
    // Legacy planned-load assignments can remain sticky without contributing load.
    let mut state = AssignmentState {
        routes: BTreeMap::new(),
        assignments: previous
            .assignments
            .iter()
            .filter(|(_, pin)| {
                topology.sources.contains(pin.source_node.as_str())
                    && (pin.node == pin.source_node
                        || pools
                            .get(pin.source_node.as_str())
                            .is_some_and(|p| p.targets.contains(&pin.node)))
            })
            .map(|(id, pin)| (id.clone(), pin.clone()))
            .collect(),
    };
    let mut pending = vec![];
    let mut wan_load = HashMap::new();
    let mut node_load = HashMap::new();
    let mut ids = HashSet::new();
    for (i, subscriber) in subscribers.iter_mut().enumerate() {
        ensure!(
            ids.insert(subscriber.id.clone()),
            "Duplicate assignment subscriber identity"
        );
        ensure!(
            subscriber.download.is_finite()
                && subscriber.download > 0.
                && subscriber.upload.is_finite()
                && subscriber.upload > 0.,
            "Invalid subscriber assignment rates"
        );
        let source = subscriber.node.clone();
        let pool = pools.get(source.as_str());
        let allowed = |target: &str| {
            target == source || pool.is_some_and(|p| p.targets.iter().any(|t| t == target))
        };
        let target = if let Some(pin) = settings.assignment.pins.get(&subscriber.id) {
            ensure!(
                pin.source_node == source && allowed(&pin.node),
                "Pinned subscriber changed source; clear or replace the pin"
            );
            Some(pin.node.clone())
        } else if let Some(old) = previous
            .assignments
            .get(&subscriber.id)
            .filter(|p| p.source_node == source && allowed(&p.node))
        {
            Some(old.node.clone())
        } else if pool.is_some_and(|p| p.balance) {
            None
        } else {
            Some(source.clone())
        };
        if let Some(target) = target {
            subscriber.node = target;
            add_load(
                &mut wan_load,
                &topology.root(&subscriber.node)?.id,
                subscriber,
            )?;
            add_load(&mut node_load, &subscriber.node, subscriber)?;
            state.assignments.insert(
                subscriber.id.clone(),
                Pin {
                    source_node: source,
                    node: subscriber.node.clone(),
                },
            );
        } else {
            pending.push((i, source));
        }
    }
    pending.sort_by(|(a, _), (b, _)| subscribers[*a].id.cmp(&subscribers[*b].id));
    for (i, source) in pending {
        let subscriber = &mut subscribers[i];
        let pool = pools[source.as_str()];
        let mut best: Option<(f64, usize, &str)> = None;
        for target in &pool.targets {
            let (value, branch_count) = if pool.metric == "online_clients" {
                let root = topology.root(target)?;
                (
                    wan_load.get(&root.id).map(|v| v.2).unwrap_or(0) as f64,
                    node_load.get(target).map(|v| v.2).unwrap_or(0),
                )
            } else {
                (
                    score(&topology, target, subscriber, &wan_load, &node_load)?,
                    0,
                )
            };
            if best.is_none_or(|old| (value, branch_count, target.as_str()) < old) {
                best = Some((value, branch_count, target));
            }
        }
        let target = best.context("Assignment pool is empty")?.2;
        subscriber.node = target.into();
        add_load(&mut wan_load, &topology.root(target)?.id, subscriber)?;
        add_load(&mut node_load, target, subscriber)?;
        state.assignments.insert(
            subscriber.id.clone(),
            Pin {
                source_node: source,
                node: target.into(),
            },
        );
    }
    Ok(state)
}
