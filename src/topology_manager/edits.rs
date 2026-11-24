//! Builder edits on a registry copy. The runner reads sources, rebalances, and commits.

use super::catalog::{
    CapacityPolicy, CatalogNode, CatalogRouter, CatalogSource, FactorChoice, Factors, RouterRole,
    SourceOrigin, TopologyCatalog,
};
use super::ids::{IdKind, reserve_ids};
use super::layout::NodeKind;
use super::model::{IdentityRef, RegistryDocument, SourceKind, SourceScope};
use super::rates::{Fraction, RateBps, RateRule, RuleValue, SpeedOverrides, SpeedPolicy};
use super::store::InvalidPolicy;
use crate::config::Config;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::collections::BTreeSet;
use web_contract::topology::{GeoType, Location, NodeGeo};

/// One Builder edit, as the service API receives it.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", deny_unknown_fields)]
pub enum Edit {
    #[serde(rename = "topology.initialize")]
    Initialize,
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
        /// `"intermediate"` (default) or `"group"`: a subscriber group without a source.
        #[serde(default)]
        node_kind: Option<String>,
    },
    #[serde(rename = "fallback.set")]
    FallbackSet {
        node_ids: Vec<String>,
        default_down_mbps: u64,
        default_up_mbps: u64,
    },
    #[serde(rename = "fallback.clear")]
    FallbackClear,
    /// Topology Studio policy save (plan task B4). Refused until that task is complete.
    #[serde(rename = "router.sources_set")]
    RouterSourcesSet {
        router_id: String,
        expected_config_digest: String,
        policies: Vec<web_contract::source_studio::SourcePolicyDraft>,
    },
    /// Topology Studio WAN candidate decision (plan task B4). Refused until that task is complete.
    #[serde(rename = "wan.candidate_set")]
    WanCandidateSet {
        router_id: String,
        expected_config_digest: String,
        interface: String,
        decision: web_contract::source_studio::WanCandidateDecision,
    },
    /// Add one source of a fresh enrollment draft. The runner reads the routers for it.
    #[serde(rename = "source.add")]
    SourceAdd {
        key: String,
        #[serde(default)]
        parent: Option<String>,
    },
    #[serde(rename = "instance.add")]
    InstanceAdd {
        source_id: String,
        parent: Option<String>,
        #[serde(default)]
        name: Option<String>,
    },
    /// Copy a router node and its groups below one more WAN. The groups divide their subscribers.
    #[serde(rename = "node.instance_add")]
    NodeInstanceAdd {
        node_id: String,
        parent: Option<String>,
    },
    #[serde(rename = "node.move")]
    NodeMove {
        node_id: String,
        parent: Option<String>,
    },
    /// Set or clear the `via-` address-list name of a WAN node.
    #[serde(rename = "node.address_list")]
    NodeAddressList {
        node_id: String,
        list: Option<String>,
    },
    /// Keep an intermediate node on the canvas, but not in `network.json`.
    #[serde(rename = "node.tree_exclude")]
    NodeTreeExclude { node_id: String, excluded: bool },
    /// Do not shape a source, and do not add it again automatically.
    #[serde(rename = "source.exclude")]
    SourceExclude { source_id: String, excluded: bool },
    /// Set or clear the access router that a node represents.
    #[serde(rename = "node.router")]
    NodeRouter {
        node_id: String,
        router_id: Option<String>,
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
    /// Set or clear the map type and location of one node. Both coordinates change together.
    #[serde(rename = "node.location")]
    NodeLocation {
        node_id: String,
        geo_type: Option<GeoType>,
        location: Option<Location>,
    },
    #[serde(rename = "speed.defaults")]
    SpeedDefaults {
        down_min: RuleInput,
        up_min: RuleInput,
        down_max: RuleInput,
        up_max: RuleInput,
    },
    #[serde(rename = "group.factor")]
    GroupFactor {
        source_id: String,
        #[serde(default)]
        down: Option<String>,
        #[serde(default)]
        up: Option<String>,
    },
    /// How a source spreads its subscribers over its instances. It moves nobody by itself.
    #[serde(rename = "group.allocation")]
    GroupAllocation {
        source_id: String,
        /// `null` returns the source to the topology default.
        mode: Option<super::catalog::AllocationMode>,
    },
    /// The topology default allocation mode. It moves nobody by itself.
    #[serde(rename = "allocation.default")]
    AllocationDefault {
        mode: super::catalog::AllocationMode,
    },
    /// Return subscribers to a recovered WAN without a Rebalance (on) or only by Rebalance (off).
    #[serde(rename = "failback.auto")]
    AutoFailback { enabled: bool },
    /// Return subscribers to the WANs that wait for a Rebalance. No registry change.
    #[serde(rename = "failback.release")]
    FailbackRelease,
    #[serde(rename = "routing.rebalance")]
    RoutingRebalance { apply: bool },
    #[serde(rename = "link.remove")]
    LinkRemove { parent: String, child: String },
    #[serde(rename = "card.capacity")]
    CardCapacity {
        card_id: String,
        capacity: CapacityInput,
    },
    /// One time: assign each client to a node below the WAN of its operator range list.
    #[serde(rename = "wan.adopt")]
    WanAdopt,
    #[serde(rename = "rebalance")]
    Rebalance { source_id: Option<String> },
    /// Local plans as reviewed edits: the review lists rate and capacity changes first.
    #[serde(rename = "plan.create")]
    PlanCreate {
        name: String,
        down_mbps: u64,
        up_mbps: u64,
    },
    #[serde(rename = "plan.update")]
    PlanUpdate {
        plan_id: String,
        #[serde(default)]
        name: Option<String>,
        #[serde(default)]
        down_mbps: Option<u64>,
        #[serde(default)]
        up_mbps: Option<u64>,
    },
    #[serde(rename = "plan.delete")]
    PlanDelete { plan_id: String },
    /// Select or clear the plan of one saved subscriber, for all its identities.
    #[serde(rename = "subscriber.plan")]
    SubscriberPlan {
        subscriber_id: String,
        plan_id: Option<String>,
    },
    #[serde(rename = "subscriber.pin")]
    SubscriberPin {
        #[serde(default)]
        identity: Option<IdentityInput>,
        #[serde(default)]
        subscriber_id: Option<String>,
        wan_node_id: Option<String>,
    },
}

/// One subscriber speed rule on the wire: a percent of the plan or a fixed rate.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum RuleInput {
    Percent(f64),
    Mbps(u64),
}

const RULE_TEXT: &str = "a speed rule must be a minimum percent from 0.1 to 100, a maximum percent from 1 to 1000 with one decimal, or a whole number of Mbps from 1 to 1000000";

impl RuleInput {
    fn rule(self, minimum: bool) -> anyhow::Result<RateRule> {
        match self {
            Self::Percent(p) => {
                let tenths = (p * 10.0).round();
                let range = if minimum {
                    1.0..=1000.0
                } else {
                    10.0..=10000.0
                };
                if (p * 10.0 - tenths).abs() > 1e-9 || !range.contains(&tenths) {
                    return Err(invalid(RULE_TEXT));
                }
                Ok(RateRule::BaseFraction(Fraction {
                    numerator: tenths as u64,
                    denominator: 1000,
                }))
            }
            Self::Mbps(n) if (1..=1_000_000).contains(&n) => Ok(RateRule::FixedBps(n * 1_000_000)),
            Self::Mbps(_) => Err(invalid(RULE_TEXT)),
        }
    }
}

const FACTOR_TEXT: &str =
    "a group factor must be a decimal from 0.001 to 1 with up to six decimals";

/// Parse a factor such as `0.215` into millionths.
fn parse_factor(text: &str) -> anyhow::Result<Fraction> {
    let (int, frac) = text.split_once('.').unwrap_or((text, ""));
    let digits = |s: &str| s.bytes().all(|b| b.is_ascii_digit());
    if int.is_empty() || !digits(int) || !digits(frac) || frac.len() > 6 {
        return Err(invalid(FACTOR_TEXT));
    }
    let whole: u64 = int.parse().map_err(|_| invalid(FACTOR_TEXT))?;
    let part: u64 = format!("{frac:0<6}")
        .parse()
        .map_err(|_| invalid(FACTOR_TEXT))?;
    let numerator = whole
        .checked_mul(1_000_000)
        .and_then(|w| w.checked_add(part))
        .ok_or_else(|| invalid(FACTOR_TEXT))?;
    if !(1_000..=1_000_000).contains(&numerator) {
        return Err(invalid(FACTOR_TEXT));
    }
    Ok(Fraction {
        numerator,
        denominator: 1_000_000,
    })
}

/// A factor as a short decimal: `0.215`, `0.5`, `1`.
pub(crate) fn factor_text(f: Fraction) -> String {
    let d = u128::from(f.denominator.max(1));
    let micro = u128::from(f.numerator) * 1_000_000;
    let value = if micro % d == 0 {
        let m = micro / d;
        format!("{}.{:06}", m / 1_000_000, m % 1_000_000)
    } else {
        format!("{:.6}", f.numerator as f64 / f.denominator.max(1) as f64)
    };
    value
        .trim_end_matches('0')
        .trim_end_matches('.')
        .to_string()
}

fn is_legacy(o: &SpeedOverrides) -> bool {
    [o.down_min, o.up_min, o.down_max, o.up_max]
        .iter()
        .any(|v| matches!(v, RuleValue::Override(RateRule::LegacyRoundedBase { .. })))
}

/// A subscriber identity on the wire, as `subscriber.plan` takes it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct IdentityInput {
    pub router_id: String,
    pub source_id: String,
    pub kind: SourceKind,
    pub account_ref: String,
}

impl IdentityInput {
    fn reference(&self) -> IdentityRef {
        IdentityRef {
            scope: SourceScope {
                router_id: self.router_id.clone(),
                source_id: self.source_id.clone(),
                kind: self.kind,
            },
            account_ref: self.account_ref.clone(),
        }
    }
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
    /// Subscriber IDs that the source read must contain (a new pin).
    pub require_online: Vec<String>,
    /// Clear the saved assignments of offline, unpinned records in the affected groups.
    pub clear_offline: bool,
    /// Check placements and aggregates without a rebalance (speed and factor edits).
    pub check_only: bool,
    /// Profiles whose legacy rounded speed rules the edit replaced.
    pub legacy_replaced: usize,
    /// The edit removes the fallback policy; the runner checks online fallback subscribers.
    pub clear_fallback: bool,
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
                ..Default::default()
            });
            router_id.into()
        }
    };
    c.sources.push(CatalogSource {
        excluded: false,
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
        address_list: None,
        tree_excluded: false,
        router_id: None,
        geo: None,
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
    if c.nodes.iter().any(|n| {
        n.source_id.as_deref() == Some(source_id) && below_router_card(c, n.parent.as_deref())
    }) {
        wan_to_router_card(c, parent)?;
    }
    if c.nodes
        .iter()
        .any(|n| n.source_id.as_deref() == Some(source_id) && n.parent.as_deref() == parent)
    {
        return Err(invalid(
            "the source already has an instance under this parent",
        ));
    }
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
        address_list: None,
        tree_excluded: false,
        router_id: None,
        geo: None,
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

fn below_router_card(c: &TopologyCatalog, parent: Option<&str>) -> bool {
    parent.is_some_and(|p| c.nodes.iter().any(|n| n.id == p && n.router_id.is_some()))
}

/// A group below a router card takes its WANs from the card, so a WAN parent goes on the card.
fn wan_to_router_card(c: &TopologyCatalog, parent: Option<&str>) -> anyhow::Result<()> {
    if parent.is_some_and(|p| c.nodes.iter().any(|n| n.id == p && n.kind == NodeKind::Wan)) {
        return Err(invalid("connect the WAN to the router card"));
    }
    Ok(())
}

/// Copy router node `node_id` below `parent`, with one instance of each source group below it.
/// shortcut: manual child nodes and groups added later are not copied; Task 9 can add them.
fn node_instance_add(
    d: &mut RegistryDocument,
    revision: u64,
    node_id: &str,
    parent: Option<&str>,
) -> anyhow::Result<()> {
    let c = d.catalog.as_ref().expect("enrolled");
    let node = find(c, node_id)?.clone();
    let Some(router) = node.router_id.clone() else {
        return Err(invalid(
            "only a router node can have more than one WAN parent",
        ));
    };
    check_parent(c, parent)?;
    if c.nodes
        .iter()
        .any(|n| n.router_id.as_ref() == Some(&router) && n.parent.as_deref() == parent)
    {
        return Err(invalid(
            "the router node already has a copy under this parent",
        ));
    }
    let sources: Vec<String> = c
        .nodes
        .iter()
        .filter(|n| n.parent.as_deref() == Some(node_id))
        .filter_map(|n| n.source_id.clone())
        .collect();
    let name = match parent {
        Some(p) => format!("{} {}", node.name, find(c, p)?.name),
        None => format!("{} copy", node.name),
    };
    check_name(&name)?;
    let ids = reserve_ids(d, revision, &[IdKind::Node])?;
    d.catalog
        .as_mut()
        .expect("enrolled")
        .nodes
        .push(CatalogNode {
            id: ids[0].clone(),
            name,
            parent: parent.map(Into::into),
            minimum: Default::default(),
            geo: None,
            ..node
        });
    for source in sources {
        instance_add(d, revision, &source, Some(&ids[0]), None)?;
    }
    Ok(())
}

/// The canvas card of a node: the instances of one source, or the copies of one router, share
/// one card. The web view uses the same rule.
pub fn card_key(n: &CatalogNode) -> String {
    match (n.kind, &n.source_id, &n.router_id) {
        (NodeKind::SubscriberGroup, Some(s), _) => format!("source:{s}"),
        (NodeKind::SubscriberGroup | NodeKind::Wan, _, _) => n.id.clone(),
        (_, _, Some(r)) => format!("router:{r}"),
        _ => n.id.clone(),
    }
}

/// Remove the canvas link from `parent` to `child`. The only parent: the child card moves to the
/// top level with its children. One of several: that instance goes (a router copy with its group
/// instances); its subscribers go to the other instances.
fn link_remove(
    doc: &RegistryDocument,
    d: &mut RegistryDocument,
    parent: &str,
    child: &str,
    affected: &mut BTreeSet<String>,
    cleared: &mut Vec<String>,
) -> anyhow::Result<()> {
    let c = d.catalog.as_ref().expect("enrolled");
    let node = find(c, child)?.clone();
    if node.parent.as_deref() != Some(parent) {
        return Err(invalid("the link does not exist"));
    }
    let above = find(c, parent)?;
    if node.kind == NodeKind::SubscriberGroup
        && above.kind != NodeKind::Wan
        && above.router_id.is_some()
    {
        return Err(invalid("the router branch keeps its groups"));
    }
    let key = card_key(&node);
    let others = c
        .nodes
        .iter()
        .filter(|n| n.id != node.id && card_key(n) == key)
        .count();
    if others == 0 || key == node.id {
        affected.extend(sources_below(c, &top(c, child)));
        let i = position(c, child)?;
        d.catalog.as_mut().expect("enrolled").nodes[i].parent = None;
        return Ok(());
    }
    if node.kind != NodeKind::SubscriberGroup {
        let groups: Vec<String> = c
            .nodes
            .iter()
            .filter(|n| n.parent.as_deref() == Some(child) && n.kind == NodeKind::SubscriberGroup)
            .map(|n| n.id.clone())
            .collect();
        for g in groups {
            node_remove(doc, d, &g, affected, cleared)?;
        }
    }
    node_remove(doc, d, child, affected, cleared)
}

/// Set the capacity of every instance of a canvas card. A fixed value is the card total, split
/// in whole Mbps over the instances in the ratio of the WAN capacity above each; the remainder
/// goes to the instance with the largest WAN.
fn card_capacity(
    d: &mut RegistryDocument,
    card_id: &str,
    capacity: &CapacityInput,
    affected: &mut BTreeSet<String>,
) -> anyhow::Result<()> {
    let c = d.catalog.as_ref().expect("enrolled");
    let members: Vec<usize> = (0..c.nodes.len())
        .filter(|&i| card_key(&c.nodes[i]) == card_id)
        .collect();
    let Some(&first) = members.first() else {
        return Err(invalid("the card does not exist"));
    };
    let policy = capacity.policy(c.nodes[first].kind)?;
    // The fixed capacity of the WAN at the root above a node.
    let wan_of = |mut i: usize| -> Option<RateBps> {
        while let Some(p) = c.nodes[i].parent.as_deref() {
            match c.nodes.iter().position(|n| n.id == p) {
                Some(j) => i = j,
                None => break,
            }
        }
        match (c.nodes[i].kind, c.nodes[i].capacity) {
            (NodeKind::Wan, CapacityPolicy::Fixed { base, .. }) => Some(base),
            _ => None,
        }
    };
    // A placement without a WAN above it: every placement gets an equal share.
    let weights: Vec<RateBps> = members
        .iter()
        .map(|&i| wan_of(i))
        .collect::<Option<Vec<_>>>()
        .unwrap_or_else(|| vec![RateBps { down: 1, up: 1 }; members.len()]);
    // Whole Mbps in the WAN ratio, at least 1 for each placement; the largest WAN takes the rest.
    let split = |total: u64, w: &[u64]| -> anyhow::Result<Vec<u64>> {
        if total < w.len() as u64 {
            return Err(invalid(
                "a card capacity needs at least 1 Mbps for each WAN link of the card",
            ));
        }
        let sum = w.iter().map(|&x| u128::from(x)).sum::<u128>().max(1);
        let mut parts: Vec<u64> = w
            .iter()
            .map(|&x| {
                u64::try_from(u128::from(total) * u128::from(x) / sum)
                    .unwrap_or(0)
                    .max(1)
            })
            .collect();
        let largest = (0..w.len())
            .max_by_key(|&i| (w[i], std::cmp::Reverse(i)))
            .unwrap_or(0);
        let others: u64 = parts.iter().sum::<u64>() - parts[largest];
        parts[largest] = total.saturating_sub(others);
        if parts.contains(&0) {
            return Err(invalid(
                "a card capacity needs at least 1 Mbps for each WAN link of the card",
            ));
        }
        Ok(parts)
    };
    let policies: Vec<CapacityPolicy> = match policy {
        CapacityPolicy::Fixed { base, factors } if members.len() > 1 => {
            let down = split(
                base.down / 1_000_000,
                &weights.iter().map(|w| w.down).collect::<Vec<_>>(),
            )?;
            let up = split(
                base.up / 1_000_000,
                &weights.iter().map(|w| w.up).collect::<Vec<_>>(),
            )?;
            down.into_iter()
                .zip(up)
                .map(|(d, u)| CapacityPolicy::Fixed {
                    base: RateBps {
                        down: d * 1_000_000,
                        up: u * 1_000_000,
                    },
                    factors,
                })
                .collect()
        }
        other => vec![other; members.len()],
    };
    for &i in &members {
        affected.extend(sources_below(c, &top(c, &c.nodes[i].id)));
    }
    let c = d.catalog.as_mut().expect("enrolled");
    for (&i, p) in members.iter().zip(policies) {
        c.nodes[i].capacity = p;
    }
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
    if c.fallback
        .as_ref()
        .is_some_and(|f| f.node_ids.iter().any(|n| n == node_id))
    {
        return Err(invalid("remove it from the fallback first"));
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

fn instances(c: &TopologyCatalog, source_id: &str) -> usize {
    c.nodes
        .iter()
        .filter(|n| n.source_id.as_deref() == Some(source_id))
        .count()
}

fn rebalance(c: &TopologyCatalog, source_id: Option<&str>) -> anyhow::Result<BTreeSet<String>> {
    match source_id {
        Some(id) => {
            if !c
                .sources
                .iter()
                .any(|s| s.id == id && matches!(s.origin, SourceOrigin::Subscribers { .. }))
            {
                return Err(invalid("the source does not exist"));
            }
            if instances(c, id) < 2 {
                return Err(invalid("the group has only one instance"));
            }
            Ok(BTreeSet::from([id.to_string()]))
        }
        None => {
            let all: BTreeSet<String> = c
                .sources
                .iter()
                .filter(|s| matches!(s.origin, SourceOrigin::Subscribers { .. }))
                .filter(|s| instances(c, &s.id) >= 2)
                .map(|s| s.id.clone())
                .collect();
            if all.is_empty() {
                return Err(invalid("no group has two or more instances"));
            }
            Ok(all)
        }
    }
}

/// Set or clear a pin. Returns the subscriber ID and the group to rebalance after a new pin.
fn subscriber_pin(
    d: &mut RegistryDocument,
    identity: Option<&IdentityInput>,
    subscriber_id: Option<&str>,
    wan_node_id: Option<&str>,
) -> anyhow::Result<(String, Option<String>)> {
    let record = match (identity, subscriber_id, wan_node_id) {
        (Some(i), None, _) => {
            let reference = i.reference();
            d.subscribers
                .iter()
                .position(|s| s.identities.contains(&reference))
        }
        (None, Some(id), None) => d.subscribers.iter().position(|s| s.id == id),
        _ => return Err(invalid("give identity or subscriber_id")),
    }
    .ok_or_else(|| invalid("confirm an edit or rebalance first, so the account gets a record"))?;
    let id = d.subscribers[record].id.clone();
    let Some(wan) = wan_node_id else {
        let policy = &mut d.subscribers[record].policy;
        if policy.pin.take().is_none() {
            return Err(invalid("the subscriber has no pin"));
        }
        return Ok((id, None));
    };
    let c = d.catalog.as_ref().expect("enrolled");
    let assignment = d.subscribers[record]
        .policy
        .assignment
        .as_deref()
        .and_then(|a| c.nodes.iter().find(|n| n.id == a))
        .ok_or_else(|| invalid("the subscriber has no placement yet"))?;
    let group = assignment
        .source_id
        .clone()
        .ok_or_else(|| invalid("the subscriber has no placement yet"))?;
    if c.nodes
        .iter()
        .find(|n| n.id == wan)
        .is_none_or(|n| n.kind != NodeKind::Wan)
    {
        return Err(invalid("the node is not a WAN"));
    }
    let below: Vec<String> = c
        .nodes
        .iter()
        .filter(|n| n.source_id.as_deref() == Some(group.as_str()) && is_below(c, &n.id, wan))
        .map(|n| n.id.clone())
        .collect();
    let instance = match below.as_slice() {
        [one] => one.clone(),
        [] => return Err(invalid("the group has no instance below this WAN")),
        _ => {
            return Err(invalid(
                "the group has more than one instance below this WAN",
            ));
        }
    };
    let policy = &mut d.subscribers[record].policy;
    policy.pin = Some(instance.clone());
    policy.assignment = Some(instance);
    Ok((id, Some(group)))
}

fn fallback_nodes(d: &RegistryDocument) -> Vec<String> {
    d.catalog
        .as_ref()
        .and_then(|c| c.fallback.as_ref())
        .map(|f| f.node_ids.clone())
        .unwrap_or_default()
}

/// Records assigned to nodes that leave the fallback lose that assignment; pins refuse.
fn drop_fallback_assignments(
    d: &mut RegistryDocument,
    leaving: &[String],
    cleared: &mut Vec<String>,
) -> anyhow::Result<()> {
    let pinned: Vec<&str> = d
        .subscribers
        .iter()
        .filter(|s| s.policy.pin.as_ref().is_some_and(|p| leaving.contains(p)))
        .map(|s| s.id.as_str())
        .collect();
    if !pinned.is_empty() {
        return Err(invalid(format!(
            "unpin the subscribers first: {}",
            pinned.join(", ")
        )));
    }
    for s in &mut d.subscribers {
        if s.policy
            .assignment
            .as_ref()
            .is_some_and(|a| leaving.contains(a))
        {
            s.policy.assignment = None;
            cleared.push(s.id.clone());
        }
    }
    Ok(())
}

/// The origin of a subscriber source: its scope and profile reference.
fn origin_key(s: &CatalogSource) -> Option<(&super::model::SourceScope, Option<&str>)> {
    match &s.origin {
        SourceOrigin::Subscribers {
            scope, profile_ref, ..
        } => Some((scope, profile_ref.as_deref())),
        SourceOrigin::Wan { .. } => None,
    }
}

/// Draft sources whose origin the registry catalog does not have.
pub fn candidates(doc: &RegistryDocument, draft: &super::policy_draft::PolicyDraft) -> Vec<Value> {
    let known: BTreeSet<_> = doc
        .catalog
        .iter()
        .flat_map(|c| &c.sources)
        .filter_map(origin_key)
        .collect();
    let mut out: Vec<Value> = draft
        .catalog
        .sources
        .iter()
        .filter_map(|s| {
            let key = origin_key(s)?;
            (!known.contains(&key)).then(|| {
                json!({"key": s.id, "kind": key.0.kind, "router": key.0.router_id,
                       "name": s.name, "subscribers": Value::Null, "rebind": false})
            })
        })
        .collect();
    out.extend(rebinds(doc, draft).into_iter().map(|r| {
        json!({"key": r.source_id, "kind": r.scope.kind, "router": r.scope.router_id,
               "name": format!("{} ({} -> {})", r.source_name, r.old, r.new),
               "subscribers": Value::Null, "rebind": true})
    }));
    out
}

/// A router profile renamed after enrollment: same row reference, a new name.
struct Rebind {
    source_id: String,
    source_name: String,
    scope: super::model::SourceScope,
    source_ref: String,
    old: String,
    new: String,
}

fn rebinds(doc: &RegistryDocument, draft: &super::policy_draft::PolicyDraft) -> Vec<Rebind> {
    let (Some(c), Some(settings)) = (doc.catalog.as_ref(), doc.source_settings.as_ref()) else {
        return Vec::new();
    };
    let mut out = Vec::new();
    for fresh in &draft.source_settings.selections {
        let Some(current) = settings.selections.iter().find(|s| s.scope == fresh.scope) else {
            continue;
        };
        for binding in &fresh.policy.profiles {
            let Some(old) = current.policy.profiles.iter().find(|b| {
                b.source_ref == binding.source_ref && b.current_name != binding.current_name
            }) else {
                continue;
            };
            if let Some(source) = c
                .sources
                .iter()
                .find(|s| origin_key(s) == Some((&fresh.scope, Some(binding.source_ref.as_str()))))
            {
                out.push(Rebind {
                    source_id: source.id.clone(),
                    source_name: source.name.clone(),
                    scope: fresh.scope.clone(),
                    source_ref: binding.source_ref.clone(),
                    old: old.current_name.clone(),
                    new: binding.current_name.clone(),
                });
            }
        }
    }
    out
}

/// The read role of a subscriber source kind.
fn read_role(kind: SourceKind) -> RouterRole {
    match kind {
        SourceKind::Ppp => RouterRole::Ppp,
        SourceKind::Dhcp => RouterRole::Dhcp,
        SourceKind::Hotspot => RouterRole::Hotspot,
    }
}

/// Saved selections as `(router name, kind)`. A router is matched by name, because a catalog
/// router ID can differ from the ID that the policy proposal derives from the config.
fn saved_protocols(doc: &RegistryDocument) -> std::collections::HashSet<(String, SourceKind)> {
    let names: std::collections::HashMap<_, _> = doc
        .catalog
        .iter()
        .flat_map(|c| &c.routers)
        .map(|r| (r.id.as_str(), r.name.as_str()))
        .collect();
    doc.source_settings
        .iter()
        .flat_map(|s| &s.selections)
        .map(|s| {
            let id = s.scope.router_id.as_str();
            (
                names.get(id).copied().unwrap_or(id).to_string(),
                s.scope.kind,
            )
        })
        .collect()
}

/// True if an enabled PPP or DHCP protocol of `cfg` has no saved selection. No router read.
/// Hotspot is not counted: the policy proposal does not support it.
pub fn unread_protocols(doc: &RegistryDocument, cfg: &Config) -> bool {
    let saved = saved_protocols(doc);
    doc.catalog.is_some()
        && cfg.routers.iter().any(|r| {
            (r.pppoe.enabled && !saved.contains(&(r.name.clone(), SourceKind::Ppp)))
                || (r.dhcp.enabled && !saved.contains(&(r.name.clone(), SourceKind::Dhcp)))
        })
}

/// Owner decision (2026-11-22): every enabled protocol is read, so that automatic placement
/// shapes each subscriber. For each router protocol without a saved selection, copy the draft
/// selections and their sources, with no node. `None` means that nothing is missing.
/// A draft router whose name has a different catalog ID is skipped: a merge would be ambiguous.
pub fn auto_read(
    doc: &RegistryDocument,
    draft: &super::policy_draft::PolicyDraft,
) -> anyhow::Result<Option<RegistryDocument>> {
    let Some(catalog) = &doc.catalog else {
        return Ok(None);
    };
    let saved = saved_protocols(doc);
    let selections: Vec<_> = draft
        .source_settings
        .selections
        .iter()
        .filter(|s| {
            let Some(router) = draft
                .catalog
                .routers
                .iter()
                .find(|r| r.id == s.scope.router_id)
            else {
                return false;
            };
            !saved.contains(&(router.name.clone(), s.scope.kind))
                && catalog
                    .routers
                    .iter()
                    .all(|r| r.name != router.name || r.id == router.id)
        })
        .cloned()
        .collect();
    if selections.is_empty() {
        return Ok(None);
    }
    let mut d = doc.clone();
    let c = d.catalog.as_mut().expect("checked above");
    for selection in &selections {
        let role = read_role(selection.scope.kind);
        match c
            .routers
            .iter_mut()
            .find(|r| r.id == selection.scope.router_id)
        {
            Some(r) if !r.roles.contains(&role) => r.roles.push(role),
            Some(_) => {}
            None => {
                let r = draft
                    .catalog
                    .routers
                    .iter()
                    .find(|r| r.id == selection.scope.router_id)
                    .ok_or_else(|| anyhow::anyhow!("a draft selection has no draft router"))?;
                c.routers.push(CatalogRouter {
                    id: r.id.clone(),
                    name: r.name.clone(),
                    roles: vec![role],
                    ..Default::default()
                });
            }
        }
        for source in &draft.catalog.sources {
            let SourceOrigin::Subscribers { scope, .. } = &source.origin else {
                continue;
            };
            if scope != &selection.scope || c.sources.iter().any(|s| s.id == source.id) {
                continue;
            }
            let mut added = source.clone();
            added.targets.clear();
            c.sources.push(added);
            if let Some(pid) = source.profile_id.as_deref()
                && !c.profiles.iter().any(|p| p.id == pid)
                && let Some(p) = draft.catalog.profiles.iter().find(|p| p.id == pid)
            {
                c.profiles.push(p.clone());
            }
        }
    }
    c.canonicalize();
    let settings = d
        .source_settings
        .get_or_insert_with(|| super::settings::SourceSettings {
            selections: Vec::new(),
        });
    settings.selections.extend(selections);
    settings.canonicalize();
    d.validate()?;
    Ok(Some(d))
}

/// Live source canvas (owner decision, 2026-11-22): add the group node of each active draft
/// source that the registry does not have yet. A new source comes in as `add_source` adds it.
/// A saved source without any node gets the draft node. It never removes or changes a node.
/// `None` means that nothing is missing. An unenrolled registry is not changed.
pub fn auto_groups(
    doc: &RegistryDocument,
    draft: &super::policy_draft::PolicyDraft,
) -> anyhow::Result<Option<RegistryDocument>> {
    if doc.catalog.is_none() || doc.export_ownership.is_none() {
        return Ok(None);
    }
    let mut d = doc.clone();
    let mut changed = false;
    for node in draft
        .catalog
        .nodes
        .iter()
        .filter(|n| n.kind == NodeKind::SubscriberGroup)
    {
        let Some(key) = node.source_id.as_deref() else {
            continue;
        };
        let c = d.catalog.as_ref().expect("checked above");
        // A source with any node keeps the operator's placement. An excluded source stays out.
        if c.sources.iter().any(|s| s.id == key && s.excluded)
            || c.nodes
                .iter()
                .any(|n| n.id == node.id || n.source_id.as_deref() == Some(key))
        {
            continue;
        }
        if c.sources.iter().any(|s| s.id == key) {
            let c = d.catalog.as_mut().expect("checked above");
            c.nodes.push(node.clone());
            c.canonicalize();
        } else {
            d = add_source(&d, draft, key, None)?.candidate;
        }
        changed = true;
    }
    if !changed {
        return Ok(None);
    }
    d.validate()?;
    Ok(Some(d))
}

/// The start ceiling of an automatic router card: 100 Gbit/s, so that it limits nothing.
pub const ROUTER_CARD_BPS: u64 = 100_000_000_000;

/// Live source canvas (owner decision 3): one router card for each router that has an active
/// group and no router node yet. It adopts the group nodes of its router that have no parent.
/// `exclude`: the card stays out of `network.json` (the default with `root_promotion`).
/// `None` means that nothing is missing.
pub fn auto_router_nodes(
    doc: &RegistryDocument,
    draft: &super::policy_draft::PolicyDraft,
    exclude: bool,
) -> anyhow::Result<Option<RegistryDocument>> {
    if doc.catalog.is_none() || doc.export_ownership.is_none() {
        return Ok(None);
    }
    let router_of = |c: &TopologyCatalog, source_id: &str| {
        c.sources
            .iter()
            .find(|s| s.id == source_id)
            .and_then(|s| match &s.origin {
                SourceOrigin::Subscribers { scope, .. } => Some(scope.router_id.clone()),
                SourceOrigin::Wan { .. } => None,
            })
    };
    let active: BTreeSet<String> = draft
        .catalog
        .nodes
        .iter()
        .filter(|n| n.kind == NodeKind::SubscriberGroup)
        .filter_map(|n| router_of(&draft.catalog, n.source_id.as_deref()?))
        .collect();
    let mut d = doc.clone();
    let c = d.catalog.as_mut().expect("checked above");
    let mut changed = false;
    for router in active {
        let Some(name) = c
            .routers
            .iter()
            .find(|r| r.id == router)
            .map(|r| r.name.clone())
        else {
            continue;
        };
        if !c.router_nodes(&router).is_empty() {
            continue;
        }
        let id = format!("router-{router}");
        let adopted: Vec<usize> = (0..c.nodes.len())
            .filter(|&i| {
                let n = &c.nodes[i];
                n.kind == NodeKind::SubscriberGroup
                    && n.parent.is_none()
                    && n.source_id
                        .as_deref()
                        .and_then(|s| router_of(c, s))
                        .is_some_and(|r| r == router)
            })
            .collect();
        for i in adopted {
            c.nodes[i].parent = Some(id.clone());
        }
        c.nodes.push(CatalogNode {
            address_list: None,
            tree_excluded: exclude,
            router_id: Some(router.clone()),
            geo: None,
            minimum: Default::default(),
            id,
            name,
            parent: None,
            source_id: None,
            kind: NodeKind::Intermediate,
            // A root node needs its own ceiling. 100 Gbit/s does not limit; the operator sets the
            // real capacity in the card before including the card in the tree.
            capacity: CapacityPolicy::Fixed {
                base: super::rates::RateBps {
                    down: ROUTER_CARD_BPS,
                    up: ROUTER_CARD_BPS,
                },
                factors: FactorChoice::Inherit,
            },
        });
        changed = true;
    }
    if !changed {
        return Ok(None);
    }
    c.canonicalize();
    d.validate()?;
    Ok(Some(d))
}

/// Copy one draft source into the registry, as enrollment builds it, with its first instance.
pub fn add_source(
    doc: &RegistryDocument,
    draft: &super::policy_draft::PolicyDraft,
    key: &str,
    parent: Option<&str>,
) -> anyhow::Result<Applied> {
    if (doc.export_ownership.is_none() && !frontend_draft(doc)) || doc.catalog.is_none() {
        return Err(invalid(
            "enroll first: the registry has no export ownership",
        ));
    }
    let gone = || invalid("the source is no longer on the router");
    let fresh: Vec<String> = candidates(doc, draft)
        .iter()
        .filter_map(|c| c["key"].as_str().map(str::to_string))
        .collect();
    if !fresh.iter().any(|k| k == key) {
        return Err(gone());
    }
    // A router with saved Studio policies has one source authority: its policies.
    let owner = draft
        .catalog
        .sources
        .iter()
        .find(|s| s.id == key)
        .and_then(|s| match &s.origin {
            SourceOrigin::Subscribers { scope, .. } => Some(scope.router_id.as_str()),
            SourceOrigin::Wan { .. } => None,
        });
    let owned = doc
        .catalog
        .iter()
        .flat_map(|c| &c.routers)
        .any(|r| Some(r.id.as_str()) == owner && r.source_policies.is_some());
    if owned {
        return Err(invalid(
            "source policies own the sources of this router; change them in Topology Studio",
        ));
    }
    if let Some(r) = rebinds(doc, draft).into_iter().find(|r| r.source_id == key) {
        let mut d = doc.clone();
        let settings = d.source_settings.as_mut().ok_or_else(gone)?;
        let binding = settings
            .selections
            .iter_mut()
            .find(|s| s.scope == r.scope)
            .and_then(|s| {
                s.policy
                    .profiles
                    .iter_mut()
                    .find(|b| b.source_ref == r.source_ref)
            })
            .ok_or_else(gone)?;
        binding.current_name = r.new;
        d.validate().map_err(|e| invalid(format!("{e:#}")))?;
        return Ok(Applied {
            candidate: d,
            affected: vec![key.to_string()],
            cleared: Vec::new(),
            require_online: Vec::new(),
            clear_offline: false,
            check_only: false,
            legacy_replaced: 0,
            clear_fallback: false,
        });
    }
    let source = draft
        .catalog
        .sources
        .iter()
        .find(|s| s.id == key)
        .ok_or_else(gone)?;
    let (scope, profile_ref) = origin_key(source).ok_or_else(gone)?;
    let mut d = doc.clone();
    let c = d.catalog.as_mut().expect("enrolled");
    check_parent(c, parent)?;
    // A protocol or a router enabled after enrollment: give the catalog router its read role.
    let role = match scope.kind {
        super::model::SourceKind::Ppp => RouterRole::Ppp,
        super::model::SourceKind::Dhcp => RouterRole::Dhcp,
        super::model::SourceKind::Hotspot => RouterRole::Hotspot,
    };
    match c.routers.iter_mut().find(|r| r.id == scope.router_id) {
        Some(r) if !r.roles.contains(&role) => r.roles.push(role),
        Some(_) => {}
        None => {
            let r = draft
                .catalog
                .routers
                .iter()
                .find(|r| r.id == scope.router_id)
                .ok_or_else(gone)?;
            c.routers.push(CatalogRouter {
                id: r.id.clone(),
                name: r.name.clone(),
                roles: vec![role],
                ..Default::default()
            });
        }
    }
    let mut added = source.clone();
    added.targets.clear();
    c.sources.push(added);
    if let Some(pid) = source.profile_id.as_deref()
        && !c.profiles.iter().any(|p| p.id == pid)
        && let Some(p) = draft.catalog.profiles.iter().find(|p| p.id == pid)
    {
        c.profiles.push(p.clone());
    }
    let mut node = draft
        .catalog
        .nodes
        .iter()
        .find(|n| n.source_id.as_deref() == Some(key))
        .cloned()
        .ok_or_else(gone)?;
    if c.nodes.iter().any(|n| n.id == node.id) {
        return Err(invalid("the source node exists already"));
    }
    node.parent = parent.map(Into::into);
    c.nodes.push(node);
    let draft_selection = draft
        .source_settings
        .selections
        .iter()
        .find(|s| s.scope == *scope)
        .ok_or_else(gone)?;
    let settings = d
        .source_settings
        .as_mut()
        .ok_or_else(|| invalid("enroll first: the registry has no source settings"))?;
    match settings.selections.iter_mut().find(|s| s.scope == *scope) {
        Some(selection) => {
            for name in &draft_selection.service_names {
                if !selection.service_names.contains(name) {
                    selection.service_names.push(name.clone());
                }
            }
            for binding in &draft_selection.policy.profiles {
                let wanted = profile_ref.is_none_or(|r| r == binding.source_ref);
                if wanted
                    && !selection
                        .policy
                        .profiles
                        .iter()
                        .any(|b| b.source_ref == binding.source_ref)
                {
                    selection.policy.profiles.push(binding.clone());
                }
            }
        }
        None => settings.selections.push(draft_selection.clone()),
    }
    let c = d.catalog.as_mut().expect("enrolled");
    split_minimum(c, key).map_err(|e| invalid(format!("{e:#}")))?;
    c.canonicalize();
    d.validate().map_err(|e| invalid(format!("{e:#}")))?;
    Ok(Applied {
        candidate: d,
        affected: vec![key.to_string()],
        cleared: Vec::new(),
        require_online: Vec::new(),
        clear_offline: false,
        check_only: false,
        legacy_replaced: 0,
        clear_fallback: false,
    })
}

/// A frontend-created draft has no published ownership or subscriber records.
pub fn frontend_draft(doc: &RegistryDocument) -> bool {
    doc.frontend_initialized
        && doc.export_ownership.is_none()
        && doc.subscribers.is_empty()
        && doc.catalog.is_some()
        && doc.source_settings.is_some()
}

/// Initialization never replaces installed intent, even if it is not enrolled.
pub fn can_initialize(doc: &RegistryDocument) -> bool {
    doc.revision == 0
        && doc.next_id == 1
        && doc.subscribers.is_empty()
        && doc.catalog.is_none()
        && doc.source_settings.is_none()
        && doc.export_ownership.is_none()
        && !doc.frontend_initialized
}

/// Apply one reviewed edit to an enrolled registry or an explicit frontend draft.
pub fn apply(
    doc: &RegistryDocument,
    edit: &Edit,
    discovery: Option<&Value>,
    cfg: &Config,
) -> anyhow::Result<Applied> {
    if matches!(
        edit,
        Edit::RouterSourcesSet { .. } | Edit::WanCandidateSet { .. }
    ) {
        return Err(invalid(
            "source policy edits are not available as a reviewed proposal; use the automatic edit",
        ));
    }
    if matches!(edit, Edit::Initialize) {
        if !can_initialize(doc) {
            return Err(invalid(
                "initialization requires an empty registry without an installed policy",
            ));
        }
        let mut candidate = doc.clone();
        candidate.catalog = Some(super::policy_draft::read_catalog(cfg));
        candidate.source_settings = Some(super::settings::SourceSettings {
            selections: Vec::new(),
        });
        candidate.frontend_initialized = true;
        candidate.validate()?;
        return Ok(Applied {
            candidate,
            affected: Vec::new(),
            cleared: Vec::new(),
            require_online: Vec::new(),
            clear_offline: false,
            check_only: false,
            legacy_replaced: 0,
            clear_fallback: false,
        });
    }
    let structural = matches!(
        edit,
        Edit::WanCreate { .. }
            | Edit::NodeCreate { .. }
            | Edit::InstanceAdd { .. }
            | Edit::NodeInstanceAdd { .. }
            | Edit::NodeMove { .. }
            | Edit::NodeUpdate { .. }
            | Edit::NodeRemove { .. }
            | Edit::NodeLocation { .. }
            | Edit::NodeRouter { .. }
            | Edit::NodeAddressList { .. }
            | Edit::NodeTreeExclude { .. }
            | Edit::SourceExclude { .. }
            | Edit::SourceAdd { .. }
            | Edit::LinkRemove { .. }
            | Edit::CardCapacity { .. }
    );
    if doc.catalog.is_none()
        || (doc.export_ownership.is_none() && !(frontend_draft(doc) && structural))
    {
        return Err(invalid(
            "enroll first: the registry has no export ownership",
        ));
    }
    let mut d = doc.clone();
    let mut affected = BTreeSet::new();
    let mut cleared = Vec::new();
    let mut require_online = Vec::new();
    let mut clear_offline = false;
    let mut check_only = false;
    let mut legacy_replaced = 0;
    let mut clear_fallback = false;
    match edit {
        Edit::Initialize | Edit::RouterSourcesSet { .. } | Edit::WanCandidateSet { .. } => {
            unreachable!("handled above")
        }
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
            node_kind,
        } => {
            check_name(name)?;
            let kind = match node_kind.as_deref() {
                None | Some("intermediate") => NodeKind::Intermediate,
                Some("group") => NodeKind::SubscriberGroup,
                Some(_) => return Err(invalid("a node kind is intermediate or group")),
            };
            let capacity = capacity.policy(kind)?;
            check_parent(d.catalog.as_ref().expect("enrolled"), parent.as_deref())?;
            let ids = reserve_ids(&mut d, doc.revision, &[IdKind::Node])?;
            d.catalog
                .as_mut()
                .expect("enrolled")
                .nodes
                .push(CatalogNode {
                    address_list: None,
                    tree_excluded: false,
                    router_id: None,
                    geo: None,
                    minimum: Default::default(),
                    id: ids[0].clone(),
                    name: name.clone(),
                    parent: parent.clone(),
                    source_id: None,
                    kind,
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
        Edit::NodeInstanceAdd { node_id, parent } => {
            let c = d.catalog.as_ref().expect("enrolled");
            affected.extend(sources_below(c, &top(c, node_id)));
            node_instance_add(&mut d, doc.revision, node_id, parent.as_deref())?;
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
            if node.source_id.is_some() && below_router_card(c, node.parent.as_deref()) {
                wan_to_router_card(c, parent.as_deref())?;
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
        Edit::LinkRemove { parent, child } => {
            link_remove(doc, &mut d, parent, child, &mut affected, &mut cleared)?;
        }
        Edit::CardCapacity { card_id, capacity } => {
            card_capacity(&mut d, card_id, capacity, &mut affected)?;
        }
        Edit::NodeLocation {
            node_id,
            geo_type,
            location,
        } => {
            let i = position(d.catalog.as_ref().expect("enrolled"), node_id)?;
            if let Some(location) = location {
                location.validate().map_err(invalid)?;
            }
            let geo = match (geo_type, location) {
                (None, Some(_)) => return Err(invalid("a location needs a Site or AP type")),
                (None, None) => None,
                (Some(geo_type), location) => Some(NodeGeo {
                    geo_type: *geo_type,
                    location: location.clone(),
                }),
            };
            d.catalog.as_mut().expect("enrolled").nodes[i].geo = geo;
        }
        Edit::NodeAddressList { node_id, list } => {
            let i = position(d.catalog.as_ref().expect("enrolled"), node_id)?;
            d.catalog.as_mut().expect("enrolled").nodes[i].address_list = list.clone();
        }
        Edit::NodeTreeExclude { node_id, excluded } => {
            let c = d.catalog.as_ref().expect("enrolled");
            let i = position(c, node_id)?;
            // A router node (since schema 9) or a display-only intermediate node.
            if *excluded
                && c.nodes[i].kind != NodeKind::Intermediate
                && c.nodes[i].router_id.is_none()
            {
                return Err(invalid(
                    "only an intermediate node or a router node can be excluded from the network tree",
                ));
            }
            affected.extend(sources_below(c, &top(c, node_id)));
            d.catalog.as_mut().expect("enrolled").nodes[i].tree_excluded = *excluded;
        }
        Edit::SourceExclude {
            source_id,
            excluded,
        } => {
            let c = d.catalog.as_mut().expect("enrolled");
            let source = c
                .sources
                .iter_mut()
                .find(|s| s.id == *source_id)
                .ok_or_else(|| invalid("the source is not in the catalog"))?;
            source.excluded = *excluded;
            affected.insert(source_id.clone());
        }
        Edit::NodeRouter { node_id, router_id } => {
            let c = d.catalog.as_ref().expect("enrolled");
            affected.extend(sources_below(c, &top(c, node_id)));
            let i = position(c, node_id)?;
            d.catalog.as_mut().expect("enrolled").nodes[i].router_id = router_id.clone();
        }
        Edit::FallbackSet {
            node_ids,
            default_down_mbps,
            default_up_mbps,
        } => {
            let default_plan = mbps(*default_down_mbps, *default_up_mbps)?;
            let leaving: Vec<String> = fallback_nodes(&d)
                .into_iter()
                .filter(|n| !node_ids.contains(n))
                .collect();
            drop_fallback_assignments(&mut d, &leaving, &mut cleared)?;
            let c = d.catalog.as_mut().expect("enrolled");
            for id in node_ids {
                if !c.nodes.iter().any(|n| {
                    n.id == *id && n.kind == NodeKind::SubscriberGroup && n.source_id.is_none()
                }) {
                    return Err(invalid("a fallback node must be a group without a source"));
                }
            }
            c.fallback = Some(super::catalog::FallbackPolicy {
                node_ids: node_ids.clone(),
                default_plan,
            });
        }
        Edit::FallbackClear => {
            let leaving = fallback_nodes(&d);
            drop_fallback_assignments(&mut d, &leaving, &mut cleared)?;
            d.catalog.as_mut().expect("enrolled").fallback = None;
            clear_fallback = true;
        }
        Edit::SourceAdd { .. } => {
            return Err(invalid("source.add needs a fresh source read"));
        }
        Edit::PlanCreate {
            name,
            down_mbps,
            up_mbps,
        } => {
            d = super::plans::create(&d, name, *down_mbps, *up_mbps)?.0;
            check_only = true;
        }
        Edit::PlanUpdate {
            plan_id,
            name,
            down_mbps,
            up_mbps,
        } => {
            d = super::plans::update(&d, plan_id, name.as_deref(), *down_mbps, *up_mbps)?;
            check_only = true;
        }
        Edit::PlanDelete { plan_id } => {
            d = super::plans::delete(&d, plan_id)?;
            check_only = true;
        }
        Edit::SubscriberPlan {
            subscriber_id,
            plan_id,
        } => {
            let record = d
                .subscribers
                .iter()
                .find(|s| s.id == *subscriber_id)
                .ok_or_else(|| invalid("unknown subscriber"))?;
            let identities = record.identities.clone();
            if identities.is_empty() {
                return Err(invalid("the subscriber has no verified identity"));
            }
            // The plan belongs to the subscriber record, so every identity gets the same plan.
            for identity in &identities {
                d = super::plans::attach(&d, identity, plan_id.as_deref(), true)?.0;
            }
            check_only = true;
        }
        Edit::SpeedDefaults {
            down_min,
            up_min,
            down_max,
            up_max,
        } => {
            let defaults = SpeedPolicy {
                down_min: down_min.rule(true)?,
                up_min: up_min.rule(true)?,
                down_max: down_max.rule(false)?,
                up_max: up_max.rule(false)?,
            };
            let c = d.catalog.as_mut().expect("enrolled");
            c.defaults = defaults;
            // One global policy: every profile inherits it.
            for p in &mut c.profiles {
                legacy_replaced += usize::from(is_legacy(&p.speed));
                p.speed = SpeedOverrides::default();
            }
            check_only = true;
        }
        Edit::GroupFactor {
            source_id,
            down,
            up,
        } => {
            let down = down.as_deref().map(parse_factor).transpose()?;
            let up = up.as_deref().map(parse_factor).transpose()?;
            let c = d.catalog.as_mut().expect("enrolled");
            let group = c
                .sources
                .iter_mut()
                .find(|s| s.id == *source_id)
                .and_then(|s| match &mut s.origin {
                    SourceOrigin::Subscribers { group, .. } => Some(group),
                    SourceOrigin::Wan { .. } => None,
                })
                .ok_or_else(|| invalid("the source does not exist"))?;
            if let Some(f) = down {
                group.factors.down = f;
            }
            if let Some(f) = up {
                group.factors.up = f;
            }
            check_only = true;
        }
        Edit::GroupAllocation { source_id, mode } => {
            let c = d.catalog.as_mut().expect("enrolled");
            let group = c
                .sources
                .iter_mut()
                .find(|s| s.id == *source_id)
                .and_then(|s| match &mut s.origin {
                    SourceOrigin::Subscribers { group, .. } => Some(group),
                    SourceOrigin::Wan { .. } => None,
                })
                .ok_or_else(|| invalid("the subscriber source does not exist"))?;
            group.allocation = *mode;
        }
        Edit::AllocationDefault { mode } => {
            d.catalog.as_mut().expect("enrolled").allocation = *mode;
        }
        Edit::FailbackRelease | Edit::WanAdopt | Edit::RoutingRebalance { .. } => {
            return Err(invalid(
                "the service runs this edit with its own router data",
            ));
        }
        Edit::AutoFailback { enabled } => {
            d.catalog.as_mut().expect("enrolled").auto_failback = *enabled;
        }
        Edit::Rebalance { source_id } => {
            affected = rebalance(d.catalog.as_ref().expect("enrolled"), source_id.as_deref())?;
            clear_offline = true;
        }
        Edit::SubscriberPin {
            identity,
            subscriber_id,
            wan_node_id,
        } => {
            let (id, group) = subscriber_pin(
                &mut d,
                identity.as_ref(),
                subscriber_id.as_deref(),
                wan_node_id.as_deref(),
            )?;
            if let Some(group) = group {
                require_online.push(id);
                affected.insert(group);
            }
        }
    }
    let c = d.catalog.as_mut().expect("enrolled");
    for source in &affected {
        split_minimum(c, source).map_err(|e| invalid(format!("{e:#}")))?;
    }
    c.canonicalize();
    d.validate().map_err(|e| invalid(format!("{e:#}")))?;
    d.catalog
        .as_ref()
        .expect("enrolled")
        .check_router_nodes()
        .map_err(|e| invalid(format!("{e:#}")))?;
    // Refuse a new violation only, so an older saved policy still opens and edits. A card
    // capacity sets the total of the card and splits it, so it is not a violation (owner, 11-23).
    if !matches!(edit, Edit::CardCapacity { .. })
        && let Some(name) = shared_fixed(d.catalog.as_ref().expect("enrolled"))
        && doc.catalog.as_ref().and_then(shared_fixed).is_none()
    {
        return Err(invalid(format!(
            "{name} has placements under several parents. A fixed capacity on one placement cannot \
             hold one shared limit: LibreQoS has no shared limit across parents. Use automatic \
             capacity, or keep one placement."
        )));
    }
    cleared.sort();
    Ok(Applied {
        candidate: d,
        affected: affected.into_iter().collect(),
        cleared,
        require_online,
        clear_offline,
        check_only,
        legacy_replaced,
        clear_fallback,
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

/// The name of a source with several placements where one has a fixed capacity. Two parents
/// would each get an independent limit, not one shared physical limit.
fn shared_fixed(c: &TopologyCatalog) -> Option<String> {
    c.sources
        .iter()
        .find(|s| {
            let placements: Vec<_> = c
                .nodes
                .iter()
                .filter(|n| n.source_id.as_deref() == Some(s.id.as_str()))
                .collect();
            placements.len() > 1
                && placements
                    .iter()
                    .any(|n| matches!(n.capacity, CapacityPolicy::Fixed { .. }))
        })
        .map(|s| s.name.clone())
}

/// The `topology.tree` answer: every catalog node, without a router read.
pub fn tree(doc: &RegistryDocument) -> Value {
    let Some(c) = doc.catalog.as_ref() else {
        return json!({"revision": doc.revision, "nodes": [], "sources": [], "pins": []});
    };
    let nodes: Vec<Value> = c
        .nodes
        .iter()
        .enumerate()
        .map(|(i, n)| {
            json!({
                "id": n.id, "name": n.name, "parent": n.parent, "kind": kind_out(n.kind),
                "source_id": n.source_id, "capacity": capacity_out(&n.capacity),
                "alias": aliased(doc, &n.id), "geo": n.geo, "router_id": n.router_id,
                "address_list": c.wan_list(i), "tree_excluded": n.tree_excluded,
            })
        })
        .collect();
    let automatic = super::resolver::automatic_sets(c).unwrap_or_default();
    let sources: Vec<Value> = c
        .sources
        .iter()
        .map(|s| {
            let (kind, auto, router, access, endpoint) = match &s.origin {
                SourceOrigin::Wan {
                    router_id,
                    endpoint_ref,
                } => ("wan", None, router_id, None, Some(endpoint_ref)),
                SourceOrigin::Subscribers { scope, .. } => {
                    let placed = !s.targets.is_empty()
                        || c.nodes.iter().any(|n| n.source_id.as_deref() == Some(&s.id));
                    let auto = (!placed).then(|| automatic.get(&scope.router_id)).flatten();
                    let access = serde_json::to_value(scope.kind).ok();
                    ("subscribers", auto, &scope.router_id, access, None)
                }
            };
            json!({"id": s.id, "name": s.name, "kind": kind, "auto_parents": auto.cloned().unwrap_or_default(),
                "excluded": s.excluded, "router_id": router, "access": access, "endpoint": endpoint})
        })
        .collect();
    let pins: Vec<Value> = doc
        .subscribers
        .iter()
        .filter_map(|s| {
            s.policy
                .pin
                .as_ref()
                .map(|p| json!({"subscriber_id": s.id, "node_id": p}))
        })
        .collect();
    let fallback = c.fallback.as_ref().map(|f| {
        json!({"node_ids": f.node_ids,
               "default_mbps": [f.default_plan.down / 1_000_000, f.default_plan.up / 1_000_000]})
    });
    json!({"revision": doc.revision, "nodes": nodes, "sources": sources, "pins": pins, "fallback": fallback})
}

#[cfg(test)]
#[path = "edits_tests.rs"]
mod tests;
