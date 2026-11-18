//! Aggregate limits from fresh circuit placements. Native export checks remain separate.

use super::catalog::{CapacityPolicy, GroupFormula, SourceOrigin};
use super::layout::{NodeKind, validate_layout};
use super::model::RegistryDocument;
use super::observation::ActiveSubscriber;
use super::rates::{AggregateRates, RateBps};
use super::rates::{AggregateRule, resolve_aggregate};
use super::resolver::{Placement, PlacementResolution};
use anyhow::{Context, ensure};
use std::collections::{HashMap, HashSet};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AggregateNode {
    pub node_id: String,
    pub parent_id: Option<String>,
    pub direct_circuits: u64,
    pub active_circuits: u64,
    pub plan_demand: RateBps,
    pub maximum_demand: RateBps,
    pub minimum_demand: RateBps,
    pub minimum_commitment: RateBps,
    pub basis: Option<RateBps>,
    pub calculated: RateBps,
    pub rates: AggregateRates,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Directions {
    pub down: bool,
    pub up: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AggregateWarning {
    pub node_id: String,
    pub plan_over: Directions,
    pub maximum_over: Directions,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AggregateConflictReason {
    MinimumExceedsCeiling {
        minimum: RateBps,
        ceiling: RateBps,
    },
    FloorExceedsCeiling {
        floor: RateBps,
        ceiling: RateBps,
    },
    MinimumBelowDescendants {
        declared: super::rates::NodeMinimum,
        required: RateBps,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AggregateConflict {
    pub node_id: String,
    pub reason: AggregateConflictReason,
}

#[derive(Debug)]
pub struct AggregateResolution {
    pub generation: u64,
    pub active: Vec<ActiveSubscriber>,
    pub placements: Vec<Placement>,
    pub nodes: Vec<AggregateNode>,
    pub omitted_nodes: Vec<String>,
    pub warnings: Vec<AggregateWarning>,
    pub conflicts: Vec<AggregateConflict>,
    /// Export validation and native verification remain required after this stage.
    pub candidate: Option<RegistryDocument>,
}

const ZERO: RateBps = RateBps { down: 0, up: 0 };

#[derive(Clone, Copy)]
struct Demand {
    circuits: u64,
    plan: RateBps,
    min: RateBps,
    max: RateBps,
}

impl Default for Demand {
    fn default() -> Self {
        Self {
            circuits: 0,
            plan: ZERO,
            min: ZERO,
            max: ZERO,
        }
    }
}

impl Demand {
    fn add(self, other: Self) -> anyhow::Result<Self> {
        fn sum(a: RateBps, b: RateBps) -> anyhow::Result<RateBps> {
            Ok(RateBps {
                down: a
                    .down
                    .checked_add(b.down)
                    .context("download demand overflow")?,
                up: a.up.checked_add(b.up).context("upload demand overflow")?,
            })
        }
        Ok(Self {
            circuits: self
                .circuits
                .checked_add(other.circuits)
                .context("circuit count overflow")?,
            plan: sum(self.plan, other.plan)?,
            min: sum(self.min, other.min)?,
            max: sum(self.max, other.max)?,
        })
    }
}

impl Directions {
    fn over(demand: RateBps, ceiling: RateBps) -> Self {
        Self {
            down: demand.down > ceiling.down,
            up: demand.up > ceiling.up,
        }
    }
    fn any(self) -> bool {
        self.down || self.up
    }
}

/// Check mathematical commitments. This stage performs no publication or native admission check.
pub fn evaluate(input: PlacementResolution) -> anyhow::Result<AggregateResolution> {
    let PlacementResolution {
        generation,
        active,
        placements,
        conflicts,
        candidate,
    } = input;
    if !conflicts.is_empty() {
        anyhow::bail!(super::conflicts::PreparationConflicts(
            conflicts
                .iter()
                .map(|c| super::conflicts::ConflictView {
                    kind: "placement",
                    subject: c.subscriber_id.clone(),
                    detail: format!("{:?}", c.reason),
                })
                .collect()
        ));
    }
    let candidate = candidate.context("aggregate evaluation requires a placement candidate")?;
    let registry_index = candidate.index()?;
    let catalog = candidate
        .catalog
        .as_ref()
        .context("aggregate evaluation requires a catalog")?;
    let mut layout = catalog.layout()?;
    let node_index: HashMap<_, _> = layout
        .iter()
        .enumerate()
        .map(|(i, n)| (n.id.as_str(), i))
        .collect();
    let sources: HashMap<_, _> = catalog.sources.iter().map(|s| (s.id.as_str(), s)).collect();
    let mut direct = vec![Demand::default(); layout.len()];
    let active_ids: HashSet<_> = active.iter().map(|a| a.subscriber_id.as_str()).collect();
    ensure!(
        active_ids.len() == active.len(),
        "duplicate active circuit in aggregate input"
    );
    ensure!(
        active
            .iter()
            .all(|a| !a.accounts.is_empty() && !a.endpoints.is_empty()),
        "aggregate input requires current account and endpoint evidence"
    );
    let mut placed_ids = HashSet::with_capacity(placements.len());
    for placement in &placements {
        ensure!(
            placed_ids.insert(placement.subscriber_id.as_str()),
            "duplicate aggregate circuit placement"
        );
        ensure!(
            active_ids.contains(placement.subscriber_id.as_str()),
            "placement has no current active circuit"
        );
        let record = registry_index
            .by_id
            .get(placement.subscriber_id.as_str())
            .context("aggregate circuit is absent from the registry")?;
        ensure!(
            candidate.subscribers[*record].policy.assignment.as_deref()
                == Some(placement.target_id.as_str()),
            "aggregate placement differs from candidate assignment"
        );
        placement.base.validate()?;
        placement.rates.min.validate()?;
        placement.rates.max.validate()?;
        ensure!(
            placement.rates.min.down <= placement.rates.max.down
                && placement.rates.min.up <= placement.rates.max.up,
            "aggregate subscriber minimum exceeds maximum"
        );
        let &target = node_index
            .get(placement.target_id.as_str())
            .context("aggregate target is absent from the catalog")?;
        direct[target] = direct[target]
            .add(Demand {
                circuits: 1,
                plan: placement.base,
                min: placement.rates.min,
                max: placement.rates.max,
            })
            .with_context(|| format!("node {} direct demand", placement.target_id))?;
    }
    ensure!(
        placed_ids == active_ids,
        "current circuit has no aggregate placement"
    );
    let parents: Vec<_> = layout
        .iter()
        .map(|n| n.parent.as_deref().map(|id| node_index[id]))
        .collect();
    // Retain flat child links so deep hierarchies do not require recursive calls or small heap allocations.
    let mut first_child = vec![None; layout.len()];
    let mut next_sibling = vec![None; layout.len()];
    let mut order = Vec::with_capacity(layout.len());
    for (i, parent) in parents.iter().enumerate() {
        if let Some(parent) = parent {
            next_sibling[i] = first_child[*parent];
            first_child[*parent] = Some(i);
        } else {
            order.push(i);
        }
    }
    drop(node_index);
    for (node, demand) in layout.iter_mut().zip(&direct) {
        node.direct_subscribers = usize::try_from(demand.circuits)
            .context("direct circuit count exceeds platform limits")?;
    }
    validate_layout(&layout)?;
    let mut cursor = 0;
    while cursor < order.len() {
        let mut child = first_child[order[cursor]];
        while let Some(index) = child {
            order.push(index);
            child = next_sibling[index];
        }
        cursor += 1;
    }
    ensure!(
        order.len() == layout.len(),
        "aggregate graph traversal is incomplete"
    );
    let mut totals = direct.clone();
    for &i in order.iter().rev() {
        if let Some(parent) = parents[i] {
            totals[parent] = totals[parent]
                .add(totals[i])
                .with_context(|| format!("node {} subtree demand", layout[parent].id))?;
        }
    }
    let mut ceilings = vec![None; layout.len()];
    let mut wans = vec![None; layout.len()];
    let mut nodes = Vec::with_capacity(layout.len());
    let mut omitted_nodes = Vec::new();
    let mut conflicts = Vec::new();
    let mut warnings = Vec::new();
    let mut output_indices = vec![None; layout.len()];
    for &i in &order {
        let node = &layout[i];
        let automatic = matches!(node.capacity, AggregateRule::Automatic { .. });
        if automatic && totals[i].circuits == 0 {
            omitted_nodes.push(node.id.clone());
            continue;
        }
        let group =
            catalog.nodes[i]
                .source_id
                .as_deref()
                .and_then(|id| match &sources[id].origin {
                    SourceOrigin::Subscribers { group, .. } => Some(group),
                    _ => None,
                });
        let basis = if automatic {
            Some(
                match group.map(|g| g.formula).unwrap_or(GroupFormula::PlanSum) {
                    GroupFormula::PlanSum => direct[i].plan,
                    GroupFormula::CountTimesUnit(unit) => RateBps {
                        down: unit
                            .down
                            .checked_mul(direct[i].circuits)
                            .context("download unit/count product overflow")?,
                        up: unit
                            .up
                            .checked_mul(direct[i].circuits)
                            .context("upload unit/count product overflow")?,
                    },
                },
            )
        } else {
            None
        };
        let wan = if node.kind == NodeKind::Wan {
            match catalog.nodes[i].capacity {
                CapacityPolicy::Fixed { base, .. } => Some(base),
                _ => unreachable!("validated WAN capacity"),
            }
        } else {
            parents[i].and_then(|p| wans[p])
        };
        wans[i] = wan;
        let parent = parents[i].and_then(|p| ceilings[p]);
        let mut rates = resolve_aggregate(&node.capacity, basis.unwrap_or(ZERO), wan, parent)
            .with_context(|| format!("node {} aggregate capacity", node.id))?;
        let calculated = rates.local;
        let backstop = if automatic {
            group
                .and_then(|g| g.backstop)
                .map(|cap| cap.base.apply_factors(cap.factor, cap.factor))
                .transpose()?
        } else {
            None
        };
        let floor = if automatic {
            group.and_then(|g| g.floor)
        } else {
            None
        };
        if let Some(cap) = backstop {
            rates.local = rates.local.constrain(cap);
        }
        if let Some(floor) = floor {
            rates.local = RateBps {
                down: rates.local.down.max(floor.down),
                up: rates.local.up.max(floor.up),
            };
        }
        rates.ceiling = rates.local;
        for bound in [wan, parent, backstop].into_iter().flatten() {
            rates.ceiling = rates.ceiling.constrain(bound);
        }
        if let Some(floor) = floor
            && Directions::over(floor, rates.ceiling).any()
        {
            conflicts.push(AggregateConflict {
                node_id: node.id.clone(),
                reason: AggregateConflictReason::FloorExceedsCeiling {
                    floor,
                    ceiling: rates.ceiling,
                },
            });
        }
        ceilings[i] = Some(rates.ceiling);
        let plan_over = Directions::over(totals[i].plan, rates.ceiling);
        let maximum_over = Directions::over(totals[i].max, rates.ceiling);
        if plan_over.any() || maximum_over.any() {
            warnings.push(AggregateWarning {
                node_id: node.id.clone(),
                plan_over,
                maximum_over,
            });
        }
        output_indices[i] = Some(nodes.len());
        nodes.push(AggregateNode {
            node_id: node.id.clone(),
            parent_id: node.parent.clone(),
            direct_circuits: direct[i].circuits,
            active_circuits: totals[i].circuits,
            plan_demand: totals[i].plan,
            maximum_demand: totals[i].max,
            minimum_demand: totals[i].min,
            minimum_commitment: totals[i].min,
            basis,
            calculated,
            rates,
        });
    }
    // Propagate each child obligation once; a parent's own guarantee replaces its smaller subtree sum.
    let mut required: Vec<_> = direct.iter().map(|d| d.min).collect();
    for &i in order.iter().rev() {
        let Some(output) = output_indices[i] else {
            continue;
        };
        let declared = catalog.nodes[i].minimum;
        if declared.down.is_some_and(|n| n < required[i].down)
            || declared.up.is_some_and(|n| n < required[i].up)
        {
            conflicts.push(AggregateConflict {
                node_id: layout[i].id.clone(),
                reason: AggregateConflictReason::MinimumBelowDescendants {
                    declared,
                    required: required[i],
                },
            });
        }
        required[i] = RateBps {
            down: declared.down.unwrap_or(0).max(required[i].down),
            up: declared.up.unwrap_or(0).max(required[i].up),
        };
        let ceiling = nodes[output].rates.ceiling;
        if Directions::over(required[i], ceiling).any() {
            conflicts.push(AggregateConflict {
                node_id: layout[i].id.clone(),
                reason: AggregateConflictReason::MinimumExceedsCeiling {
                    minimum: required[i],
                    ceiling,
                },
            });
        }
        nodes[output].minimum_commitment = required[i];
        if let Some(parent) = parents[i] {
            required[parent] = RateBps {
                down: required[parent]
                    .down
                    .checked_add(required[i].down)
                    .context("download node commitments overflow")?,
                up: required[parent]
                    .up
                    .checked_add(required[i].up)
                    .context("upload node commitments overflow")?,
            };
        }
    }
    nodes.sort_unstable_by(|a, b| a.node_id.cmp(&b.node_id));
    omitted_nodes.sort_unstable();
    warnings.sort_unstable_by(|a, b| a.node_id.cmp(&b.node_id));
    conflicts.sort_by(|a, b| a.node_id.cmp(&b.node_id));
    let candidate = conflicts.is_empty().then_some(candidate);
    Ok(AggregateResolution {
        generation,
        active,
        placements,
        nodes,
        omitted_nodes,
        warnings,
        conflicts,
        candidate,
    })
}

#[cfg(test)]
#[path = "aggregate_tests.rs"]
mod tests;
