//! Builder edits on a registry copy. The runner reads sources, rebalances, and commits.

use super::catalog::{
    CapacityPolicy, CatalogNode, CatalogRouter, CatalogSource, FactorChoice, Factors, RouterRole,
    SourceOrigin, TopologyCatalog,
};
use super::ids::{IdKind, reserve_ids};
use super::layout::NodeKind;
use super::model::RegistryDocument;
use super::rates::{Fraction, RateBps};
use super::store::InvalidPolicy;
use crate::config::Config;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::collections::BTreeSet;

/// One Builder edit, as the service API receives it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", deny_unknown_fields)]
pub enum Edit {
    #[serde(rename = "wan.create")]
    WanCreate {
        router_id: String,
        endpoint_ref: String,
        name: String,
        down_mbps: u64,
        up_mbps: u64,
        #[serde(default)]
        factor_percent: Option<[u64; 2]>,
    },
    #[serde(rename = "node.create")]
    NodeCreate {
        name: String,
        parent: Option<String>,
        capacity: CapacityInput,
    },
    #[serde(rename = "instance.add")]
    InstanceAdd {
        source_id: String,
        parent: Option<String>,
        #[serde(default)]
        name: Option<String>,
    },
    #[serde(rename = "node.move")]
    NodeMove {
        node_id: String,
        parent: Option<String>,
    },
    #[serde(rename = "node.update")]
    NodeUpdate {
        node_id: String,
        #[serde(default)]
        name: Option<String>,
        #[serde(default)]
        capacity: Option<CapacityInput>,
    },
    #[serde(rename = "node.remove")]
    NodeRemove { node_id: String },
}

/// Node capacity on the wire.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "mode", rename_all = "snake_case", deny_unknown_fields)]
pub enum CapacityInput {
    Inherit,
    Fixed {
        down_mbps: u64,
        up_mbps: u64,
        #[serde(default)]
        factor_percent: Option<[u64; 2]>,
    },
    WanPercent {
        down_percent: u64,
        up_percent: u64,
    },
    Automatic,
}

/// The edited registry copy, the subscriber sources whose pools need a rebalance, and the
/// records that lost their saved assignment.
#[derive(Debug)]
pub struct Applied {
    pub candidate: RegistryDocument,
    pub affected: Vec<String>,
    pub cleared: Vec<String>,
}

fn invalid(message: impl Into<String>) -> anyhow::Error {
    anyhow::Error::new(InvalidPolicy(message.into()))
}

fn check_name(name: &str) -> anyhow::Result<()> {
    let len = name.chars().count();
    if len == 0 || len > 64 || name.trim() != name {
        return Err(invalid(
            "a name must have 1 to 64 characters without outer spaces",
        ));
    }
    Ok(())
}

fn mbps(down: u64, up: u64) -> anyhow::Result<RateBps> {
    if !(1..=1_000_000).contains(&down) || !(1..=1_000_000).contains(&up) {
        return Err(invalid(
            "a rate must be a whole number of Mbps from 1 to 1000000",
        ));
    }
    Ok(RateBps {
        down: down * 1_000_000,
        up: up * 1_000_000,
    })
}

fn percent(p: u64) -> anyhow::Result<Fraction> {
    if !(1..=100).contains(&p) {
        return Err(invalid("a percentage must be a whole number from 1 to 100"));
    }
    Ok(Fraction {
        numerator: p,
        denominator: 100,
    })
}

fn factors(percents: Option<[u64; 2]>) -> anyhow::Result<FactorChoice> {
    Ok(match percents {
        None => FactorChoice::Inherit,
        Some([down, up]) => FactorChoice::Override(Factors {
            down: percent(down)?,
            up: percent(up)?,
        }),
    })
}

impl CapacityInput {
    fn policy(self, kind: NodeKind) -> anyhow::Result<CapacityPolicy> {
        match (kind, self) {
            (NodeKind::Wan, Self::Fixed { .. }) => {}
            (NodeKind::Wan, _) => return Err(invalid("a WAN node uses fixed")),
            (NodeKind::Intermediate, Self::Automatic) => {
                return Err(invalid(
                    "an intermediate node uses inherit, fixed, or wan_percent",
                ));
            }
            (NodeKind::Intermediate, _) => {}
            (NodeKind::SubscriberGroup, Self::Automatic | Self::Fixed { .. }) => {}
            (NodeKind::SubscriberGroup, _) => {
                return Err(invalid("a subscriber group uses automatic or fixed"));
            }
        }
        Ok(match self {
            Self::Inherit => CapacityPolicy::Inherit,
            Self::Fixed {
                down_mbps,
                up_mbps,
                factor_percent,
            } => CapacityPolicy::Fixed {
                base: mbps(down_mbps, up_mbps)?,
                factors: factors(factor_percent)?,
            },
            Self::WanPercent {
                down_percent,
                up_percent,
            } => CapacityPolicy::WanPercentage {
                down: percent(down_percent)?,
                up: percent(up_percent)?,
            },
            Self::Automatic => CapacityPolicy::Automatic {
                factors: FactorChoice::Inherit,
            },
        })
    }
}

fn find<'a>(c: &'a TopologyCatalog, id: &str) -> anyhow::Result<&'a CatalogNode> {
    c.nodes
        .iter()
        .find(|n| n.id == id)
        .ok_or_else(|| invalid("the node does not exist"))
}

fn position(c: &TopologyCatalog, id: &str) -> anyhow::Result<usize> {
    c.nodes
        .iter()
        .position(|n| n.id == id)
        .ok_or_else(|| invalid("the node does not exist"))
}

fn check_parent(c: &TopologyCatalog, parent: Option<&str>) -> anyhow::Result<()> {
    match parent {
        Some(p) => find(c, p).map(|_| ()),
        None => Ok(()),
    }
}

/// The root ancestor of a node, or the node itself at the root.
fn top(c: &TopologyCatalog, id: &str) -> String {
    let mut cursor = id.to_string();
    while let Some(parent) = c
        .nodes
        .iter()
        .find(|n| n.id == cursor)
        .and_then(|n| n.parent.clone())
    {
        cursor = parent;
    }
    cursor
}

fn is_below(c: &TopologyCatalog, id: &str, ancestor: &str) -> bool {
    let mut cursor = Some(id.to_string());
    while let Some(n) = cursor {
        if n == ancestor {
            return true;
        }
        cursor = c
            .nodes
            .iter()
            .find(|x| x.id == n)
            .and_then(|x| x.parent.clone());
    }
    false
}

/// Subscriber sources with an instance at or below `root`.
fn sources_below(c: &TopologyCatalog, root: &str) -> BTreeSet<String> {
    c.nodes
        .iter()
        .filter(|n| is_below(c, &n.id, root))
        .filter_map(|n| n.source_id.clone())
        .filter(|s| {
            c.sources
                .iter()
                .any(|x| x.id == *s && matches!(x.origin, SourceOrigin::Subscribers { .. }))
        })
        .collect()
}

fn aliased(doc: &RegistryDocument, id: &str) -> bool {
    doc.export_ownership
        .as_ref()
        .is_some_and(|o| o.nodes.iter().any(|a| a.node_id == id))
}

fn discovered<'a>(discovery: Option<&'a Value>, router_id: &str) -> Option<&'a Value> {
    discovery?["routers"]
        .as_array()?
        .iter()
        .find(|r| r["router_id"] == router_id && r["state"] == "ok")
}

/// Split the group minimum of a source over its instances in the capacity ratio above them.
/// Shares round down to whole Mbps; the remainder goes to the instance with the largest weight.
fn split_minimum(c: &mut TopologyCatalog, source_id: &str) -> anyhow::Result<()> {
    let weights = super::resolver::rebalance::instance_weights(c, source_id)?;
    if weights.is_empty() {
        return Ok(());
    }
    let ids: Vec<&str> = weights.iter().map(|(id, _)| id.as_str()).collect();
    let mut shares = vec![(None, None); weights.len()];
    for up in [false, true] {
        let get = |n: &CatalogNode| if up { n.minimum.up } else { n.minimum.down };
        let total = c
            .nodes
            .iter()
            .filter(|n| ids.contains(&n.id.as_str()))
            .filter_map(get)
            .map(|v| v / 1_000_000)
            .reduce(|a, b| a + b);
        let Some(total) = total else { continue };
        let w: Vec<u128> = weights
            .iter()
            .map(|(_, r)| u128::from(if up { r.up } else { r.down }))
            .collect();
        let sum = w.iter().sum::<u128>().max(1);
        let mut parts: Vec<u64> = w
            .iter()
            .map(|x| u64::try_from(u128::from(total) * x / sum).unwrap_or(0))
            .collect();
        let largest = (0..w.len())
            .max_by_key(|&i| (w[i], std::cmp::Reverse(i)))
            .unwrap_or(0);
        parts[largest] += total - parts.iter().sum::<u64>();
        for (i, part) in parts.into_iter().enumerate() {
            let value = (part > 0).then_some(part * 1_000_000);
            if up {
                shares[i].1 = value;
            } else {
                shares[i].0 = value;
            }
        }
    }
    for ((id, _), (down, up)) in weights.iter().zip(shares) {
        let node = c
            .nodes
            .iter_mut()
            .find(|n| n.id == *id)
            .expect("weighted instance exists");
        node.minimum.down = down;
        node.minimum.up = up;
    }
    Ok(())
}

fn wan_create(
    d: &mut RegistryDocument,
    revision: u64,
    edit: (&str, &str, &str, CapacityPolicy),
    discovery: Option<&Value>,
    cfg: &Config,
) -> anyhow::Result<()> {
    let (router_id, endpoint_ref, name, capacity) = edit;
    let router = discovered(discovery, router_id)
        .ok_or_else(|| invalid("the WAN candidate is not in the last discovery"))?;
    let router_name = router["name"].as_str().unwrap_or_default().to_string();
    let candidate = router["candidates"]
        .as_array()
        .into_iter()
        .flatten()
        .find(|c| c["name"] == endpoint_ref)
        .ok_or_else(|| invalid("the WAN candidate is not in the last discovery"))?;
    let catalog = d.catalog.as_ref().expect("enrolled");
    let catalog_router = catalog
        .routers
        .iter()
        .find(|r| r.name == router_name)
        .map(|r| r.id.clone());
    let used = catalog.sources.iter().any(|s| {
        matches!(&s.origin, SourceOrigin::Wan { router_id: r, endpoint_ref: e }
            if e == endpoint_ref && catalog_router.as_ref() == Some(r))
    });
    if used || !candidate["source_id"].is_null() {
        return Err(invalid("the WAN candidate is already used"));
    }
    if !cfg.routers.iter().any(|r| r.name == router_name) {
        return Err(invalid("the router has no configured connection"));
    }
    let ids = reserve_ids(d, revision, &[IdKind::Source, IdKind::Node])?;
    let c = d.catalog.as_mut().expect("enrolled");
    let router_ref = match catalog_router {
        Some(id) => {
            let r = c
                .routers
                .iter_mut()
                .find(|r| r.id == id)
                .expect("found above");
            if !r.roles.contains(&RouterRole::Wan) {
                r.roles.push(RouterRole::Wan);
            }
            id
        }
        None => {
            c.routers.push(CatalogRouter {
                id: router_id.into(),
                name: router_name,
                roles: vec![RouterRole::Wan],
            });
            router_id.into()
        }
    };
    c.sources.push(CatalogSource {
        id: ids[0].clone(),
        name: name.into(),
        origin: SourceOrigin::Wan {
            router_id: router_ref,
            endpoint_ref: endpoint_ref.into(),
        },
        profile_id: None,
        targets: Vec::new(),
    });
    c.nodes.push(CatalogNode {
        minimum: Default::default(),
        id: ids[1].clone(),
        name: name.into(),
        parent: None,
        source_id: Some(ids[0].clone()),
        kind: NodeKind::Wan,
        capacity,
    });
    Ok(())
}

fn instance_add(
    d: &mut RegistryDocument,
    revision: u64,
    source_id: &str,
    parent: Option<&str>,
    name: Option<&str>,
) -> anyhow::Result<()> {
    let c = d.catalog.as_ref().expect("enrolled");
    let source = c
        .sources
        .iter()
        .find(|s| s.id == source_id && matches!(s.origin, SourceOrigin::Subscribers { .. }))
        .ok_or_else(|| invalid("the source does not exist"))?;
    if !c
        .nodes
        .iter()
        .any(|n| n.source_id.as_deref() == Some(source_id))
    {
        return Err(invalid("the source is not selected"));
    }
    check_parent(c, parent)?;
    let name = match (name, parent) {
        (Some(n), _) => n.to_string(),
        (None, Some(p)) => format!("{} {}", source.name, find(c, p)?.name),
        (None, None) => source.name.clone(),
    };
    check_name(&name)?;
    let ids = reserve_ids(d, revision, &[IdKind::Node])?;
    let c = d.catalog.as_mut().expect("enrolled");
    let source = c
        .sources
        .iter_mut()
        .find(|s| s.id == source_id)
        .expect("found above");
    if !source.targets.is_empty() {
        source.targets.push(ids[0].clone());
    }
    c.nodes.push(CatalogNode {
        minimum: Default::default(),
        id: ids[0].clone(),
        name,
        parent: parent.map(Into::into),
        source_id: Some(source_id.into()),
        kind: NodeKind::SubscriberGroup,
        capacity: CapacityPolicy::Automatic {
            factors: FactorChoice::Inherit,
        },
    });
    Ok(())
}

fn node_remove(
    doc: &RegistryDocument,
    d: &mut RegistryDocument,
    node_id: &str,
    affected: &mut BTreeSet<String>,
    cleared: &mut Vec<String>,
) -> anyhow::Result<()> {
    let c = d.catalog.as_ref().expect("enrolled");
    let node = find(c, node_id)?.clone();
    if c.nodes.iter().any(|n| n.parent.as_deref() == Some(node_id)) {
        return Err(invalid("move or remove the children first"));
    }
    if aliased(doc, node_id) {
        return Err(invalid("an enrolled node keeps its legacy export name"));
    }
    let pinned: Vec<&str> = d
        .subscribers
        .iter()
        .filter(|s| s.policy.pin.as_deref() == Some(node_id))
        .map(|s| s.id.as_str())
        .collect();
    if !pinned.is_empty() {
        return Err(invalid(format!(
            "unpin the subscribers first: {}",
            pinned.join(", ")
        )));
    }
    let source = node
        .source_id
        .as_deref()
        .and_then(|s| c.sources.iter().find(|x| x.id == s))
        .cloned();
    let instances = |s: &str| {
        c.nodes
            .iter()
            .filter(|n| n.source_id.as_deref() == Some(s))
            .count()
    };
    let mut remove_source = false;
    if let Some(s) = &source {
        match s.origin {
            SourceOrigin::Subscribers { .. } => {
                if instances(&s.id) == 1 {
                    return Err(invalid("this is the last instance of the source"));
                }
                affected.insert(s.id.clone());
            }
            SourceOrigin::Wan { .. } => remove_source = instances(&s.id) == 1,
        }
    }
    for s in &mut d.subscribers {
        if s.policy.assignment.as_deref() == Some(node_id) {
            s.policy.assignment = None;
            cleared.push(s.id.clone());
        }
    }
    let c = d.catalog.as_mut().expect("enrolled");
    // The removed share of the group minimum goes to another instance; the split runs later.
    if let Some(s) = source
        .as_ref()
        .filter(|s| matches!(s.origin, SourceOrigin::Subscribers { .. }))
        && let Some(other) = c
            .nodes
            .iter_mut()
            .find(|n| n.id != node_id && n.source_id.as_deref() == Some(s.id.as_str()))
    {
        let add = |a: Option<u64>, b: Option<u64>| match (a, b) {
            (None, None) => None,
            (a, b) => Some(a.unwrap_or(0) + b.unwrap_or(0)),
        };
        other.minimum.down = add(other.minimum.down, node.minimum.down);
        other.minimum.up = add(other.minimum.up, node.minimum.up);
    }
    c.nodes.retain(|n| n.id != node_id);
    for s in &mut c.sources {
        s.targets.retain(|t| t != node_id);
    }
    if remove_source && let Some(s) = source {
        c.sources.retain(|x| x.id != s.id);
    }
    Ok(())
}

/// Apply one edit to a copy of an enrolled registry.
pub fn apply(
    doc: &RegistryDocument,
    edit: &Edit,
    discovery: Option<&Value>,
    cfg: &Config,
) -> anyhow::Result<Applied> {
    if doc.export_ownership.is_none() || doc.catalog.is_none() {
        return Err(invalid(
            "enroll first: the registry has no export ownership",
        ));
    }
    let mut d = doc.clone();
    let mut affected = BTreeSet::new();
    let mut cleared = Vec::new();
    match edit {
        Edit::WanCreate {
            router_id,
            endpoint_ref,
            name,
            down_mbps,
            up_mbps,
            factor_percent,
        } => {
            check_name(name)?;
            let capacity = CapacityInput::Fixed {
                down_mbps: *down_mbps,
                up_mbps: *up_mbps,
                factor_percent: *factor_percent,
            }
            .policy(NodeKind::Wan)?;
            wan_create(
                &mut d,
                doc.revision,
                (router_id, endpoint_ref, name, capacity),
                discovery,
                cfg,
            )?;
        }
        Edit::NodeCreate {
            name,
            parent,
            capacity,
        } => {
            check_name(name)?;
            let capacity = capacity.policy(NodeKind::Intermediate)?;
            check_parent(d.catalog.as_ref().expect("enrolled"), parent.as_deref())?;
            let ids = reserve_ids(&mut d, doc.revision, &[IdKind::Node])?;
            d.catalog
                .as_mut()
                .expect("enrolled")
                .nodes
                .push(CatalogNode {
                    minimum: Default::default(),
                    id: ids[0].clone(),
                    name: name.clone(),
                    parent: parent.clone(),
                    source_id: None,
                    kind: NodeKind::Intermediate,
                    capacity,
                });
        }
        Edit::InstanceAdd {
            source_id,
            parent,
            name,
        } => {
            instance_add(
                &mut d,
                doc.revision,
                source_id,
                parent.as_deref(),
                name.as_deref(),
            )?;
            affected.insert(source_id.clone());
        }
        Edit::NodeMove { node_id, parent } => {
            let c = d.catalog.as_ref().expect("enrolled");
            let node = find(c, node_id)?;
            if node.kind == NodeKind::Wan && parent.is_some() {
                return Err(invalid("a WAN node stays a root"));
            }
            check_parent(c, parent.as_deref())?;
            if parent.as_deref().is_some_and(|p| is_below(c, p, node_id)) {
                return Err(invalid("a node cannot move below itself"));
            }
            affected.extend(sources_below(c, &top(c, node_id)));
            let i = position(c, node_id)?;
            let c = d.catalog.as_mut().expect("enrolled");
            c.nodes[i].parent = parent.clone();
            affected.extend(sources_below(c, &top(c, node_id)));
        }
        Edit::NodeUpdate {
            node_id,
            name,
            capacity,
        } => {
            let c = d.catalog.as_ref().expect("enrolled");
            let node = find(c, node_id)?;
            if let Some(name) = name {
                check_name(name)?;
                if aliased(doc, node_id) {
                    return Err(invalid("the export name comes from the legacy node"));
                }
            }
            let policy = capacity.map(|c| c.policy(node.kind)).transpose()?;
            if policy.is_some_and(|p| p != node.capacity) {
                affected.extend(sources_below(c, &top(c, node_id)));
            }
            let i = position(c, node_id)?;
            let n = &mut d.catalog.as_mut().expect("enrolled").nodes[i];
            if let Some(name) = name {
                n.name = name.clone();
            }
            if let Some(p) = policy {
                n.capacity = p;
            }
        }
        Edit::NodeRemove { node_id } => {
            node_remove(doc, &mut d, node_id, &mut affected, &mut cleared)?;
        }
    }
    let c = d.catalog.as_mut().expect("enrolled");
    for source in &affected {
        split_minimum(c, source).map_err(|e| invalid(format!("{e:#}")))?;
    }
    c.canonicalize();
    d.validate().map_err(|e| invalid(format!("{e:#}")))?;
    cleared.sort();
    Ok(Applied {
        candidate: d,
        affected: affected.into_iter().collect(),
        cleared,
    })
}

fn capacity_out(policy: &CapacityPolicy) -> Value {
    let pct = |f: Fraction| f.numerator * 100 / f.denominator.max(1);
    match *policy {
        CapacityPolicy::Inherit => json!({"mode": "inherit"}),
        CapacityPolicy::Fixed { base, factors } => {
            let factor = match factors {
                FactorChoice::Inherit => Value::Null,
                FactorChoice::Override(f) => json!([pct(f.down), pct(f.up)]),
            };
            json!({"mode": "fixed", "down_mbps": base.down / 1_000_000,
                   "up_mbps": base.up / 1_000_000, "factor_percent": factor})
        }
        CapacityPolicy::WanPercentage { down, up } => {
            json!({"mode": "wan_percent", "down_percent": pct(down), "up_percent": pct(up)})
        }
        CapacityPolicy::Automatic { .. } => json!({"mode": "automatic"}),
    }
}

fn kind_out(kind: NodeKind) -> &'static str {
    match kind {
        NodeKind::Wan => "wan",
        NodeKind::Intermediate => "intermediate",
        NodeKind::SubscriberGroup => "group",
    }
}

/// The `topology.tree` answer: every catalog node, without a router read.
pub fn tree(doc: &RegistryDocument) -> Value {
    let Some(c) = doc.catalog.as_ref() else {
        return json!({"revision": doc.revision, "nodes": [], "sources": []});
    };
    let nodes: Vec<Value> = c
        .nodes
        .iter()
        .map(|n| {
            json!({
                "id": n.id, "name": n.name, "parent": n.parent, "kind": kind_out(n.kind),
                "source_id": n.source_id, "capacity": capacity_out(&n.capacity),
                "alias": aliased(doc, &n.id),
            })
        })
        .collect();
    let sources: Vec<Value> = c
        .sources
        .iter()
        .map(|s| {
            let kind = match s.origin {
                SourceOrigin::Wan { .. } => "wan",
                SourceOrigin::Subscribers { .. } => "subscribers",
            };
            json!({"id": s.id, "name": s.name, "kind": kind})
        })
        .collect();
    json!({"revision": doc.revision, "nodes": nodes, "sources": sources})
}

#[cfg(test)]
#[path = "edits_tests.rs"]
mod tests;
