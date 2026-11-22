//! Explicit scoped movement. Share the resolver index and keep all durable writes outside this stage.

use super::*;
use crate::topology_manager::aggregate::{AggregateResolution, evaluate};
use crate::topology_manager::catalog::AllocationMode;
use std::collections::BTreeMap;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RebalanceScope {
    SourcePool(String),
    Branch(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RebalanceRequest {
    pub expected_revision: u64,
    pub scope: RebalanceScope,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProposedMove {
    pub subscriber_id: String,
    pub from_target_id: String,
    pub to_target_id: String,
}

#[derive(Debug)]
pub struct RebalanceResult {
    pub scope: RebalanceScope,
    pub proposed_moves: Vec<ProposedMove>,
    pub evaluation: AggregateResolution,
}

type Changes = BTreeMap<Ratio, i64>;

#[derive(Default)]
struct Delta {
    independent: Changes,
    targets: Changes,
}

fn change(changes: &mut Changes, before: Ratio, after: Ratio) {
    for (ratio, amount) in [(before, -1), (after, 1)] {
        let count = changes.entry(ratio).or_default();
        *count += amount;
        if *count == 0 {
            changes.remove(&ratio);
        }
    }
}

// Compare sparse changes to a descending score multiset. Unchanged scores cancel exactly.
fn compare_changes(a: &Changes, b: &Changes) -> Ordering {
    let mut a = a.iter().rev().peekable();
    let mut b = b.iter().rev().peekable();
    loop {
        let order = match (a.peek(), b.peek()) {
            (Some((ra, _)), Some((rb, _))) => ra.cmp(rb),
            (Some(_), None) => Ordering::Greater,
            (None, Some(_)) => Ordering::Less,
            (None, None) => return Ordering::Equal,
        };
        let difference = match order {
            Ordering::Greater => *a.next().expect("present score").1,
            Ordering::Less => -*b.next().expect("present score").1,
            Ordering::Equal => {
                *a.next().expect("present score").1 - *b.next().expect("present score").1
            }
        };
        if difference != 0 {
            return difference.cmp(&0);
        }
    }
}

impl Delta {
    fn compare(&self, other: &Self) -> Ordering {
        compare_changes(&self.independent, &other.independent)
            .then_with(|| compare_changes(&self.targets, &other.targets))
    }
}

fn subtract(a: RateBps, b: RateBps) -> anyhow::Result<RateBps> {
    Ok(RateBps {
        down: a
            .down
            .checked_sub(b.down)
            .context("rebalance download demand underflow")?,
        up: a
            .up
            .checked_sub(b.up)
            .context("rebalance upload demand underflow")?,
    })
}

struct Loads {
    nodes: Vec<RateBps>,
    targets: Vec<RateBps>,
    paths: Vec<HashSet<usize>>,
}

impl Loads {
    fn trial(
        &self,
        index: &Index<'_>,
        from: usize,
        to: usize,
        base: RateBps,
    ) -> anyhow::Result<(Delta, bool)> {
        let mut delta = Delta::default();
        let mut before_worst = Ratio(0, 1);
        let mut after_worst = Ratio(0, 1);
        for &n in self.paths[from].difference(&self.paths[to]) {
            let before = ratio(self.nodes[n], index.capacities[n]);
            let after = ratio(subtract(self.nodes[n], base)?, index.capacities[n]);
            before_worst = before_worst.max(before);
            after_worst = after_worst.max(after);
            change(&mut delta.independent, before, after);
        }
        for &n in self.paths[to].difference(&self.paths[from]) {
            let before = ratio(self.nodes[n], index.capacities[n]);
            let after = ratio(add(self.nodes[n], base)?, index.capacities[n]);
            before_worst = before_worst.max(before);
            after_worst = after_worst.max(after);
            change(&mut delta.independent, before, after);
        }
        let from_before = ratio(self.targets[from], index.targets[from].capacity);
        let from_after = ratio(
            subtract(self.targets[from], base)?,
            index.targets[from].capacity,
        );
        let to_before = ratio(self.targets[to], index.targets[to].capacity);
        let to_after = ratio(add(self.targets[to], base)?, index.targets[to].capacity);
        change(&mut delta.targets, from_before, from_after);
        change(&mut delta.targets, to_before, to_after);
        let improves = after_worst < before_worst
            || (after_worst == before_worst
                && from_after.max(to_after) < from_before.max(to_before)
                && compare_changes(&delta.independent, &Changes::new()) != Ordering::Greater);
        Ok((delta, improves))
    }

    fn apply(&mut self, from: usize, to: usize, base: RateBps) -> anyhow::Result<()> {
        self.targets[from] = subtract(self.targets[from], base)?;
        self.targets[to] = add(self.targets[to], base)?;
        for &n in self.paths[from].difference(&self.paths[to]) {
            self.nodes[n] = subtract(self.nodes[n], base)?;
        }
        for &n in self.paths[to].difference(&self.paths[from]) {
            self.nodes[n] = add(self.nodes[n], base)?;
        }
        Ok(())
    }
}

struct Scope {
    targets: BTreeSet<usize>,
    source: Option<usize>,
}

fn scope(index: &Index<'_>, scope: &RebalanceScope) -> anyhow::Result<Scope> {
    let catalog = index.catalog;
    match scope {
        RebalanceScope::SourcePool(id) => {
            let source = catalog
                .sources
                .iter()
                .position(|s| s.id == *id)
                .context("rebalance source pool is absent")?;
            let definition = &catalog.sources[source];
            ensure!(
                matches!(definition.origin, SourceOrigin::Subscribers { .. }),
                "rebalance requires a subscriber source pool"
            );
            ensure!(
                !definition.targets.is_empty()
                    || catalog
                        .nodes
                        .iter()
                        .any(|n| n.source_id.as_deref() == Some(id.as_str())),
                "rebalance source pool is not selected"
            );
            let targets = if !definition.targets.is_empty() {
                index.source_targets[source].iter().copied().collect()
            } else {
                index
                    .targets
                    .iter()
                    .enumerate()
                    .filter(|(_, t)| t.source_path.contains(&source))
                    .map(|(i, _)| i)
                    .collect()
            };
            Ok(Scope {
                targets,
                source: Some(source),
            })
        }
        RebalanceScope::Branch(id) => {
            let nodes: HashMap<_, _> = catalog
                .nodes
                .iter()
                .enumerate()
                .map(|(i, n)| (n.id.as_str(), i))
                .collect();
            let &root = nodes
                .get(id.as_str())
                .context("rebalance branch is absent")?;
            let mut first = vec![None; catalog.nodes.len()];
            let mut next = vec![None; catalog.nodes.len()];
            for (i, node) in catalog.nodes.iter().enumerate() {
                if let Some(parent) = &node.parent {
                    let parent = nodes[parent.as_str()];
                    next[i] = first[parent];
                    first[parent] = Some(i);
                }
            }
            let mut selected = vec![false; catalog.nodes.len()];
            let mut queue = vec![root];
            let mut cursor = 0;
            while cursor < queue.len() {
                let n = queue[cursor];
                selected[n] = true;
                let mut child = first[n];
                while let Some(n) = child {
                    queue.push(n);
                    child = next[n];
                }
                cursor += 1;
            }
            let targets = index
                .targets
                .iter()
                .enumerate()
                .filter(|(_, t)| selected[t.node])
                .map(|(i, _)| i)
                .collect();
            Ok(Scope {
                targets,
                source: None,
            })
        }
    }
}

fn member(source: &SourceOrigin, active: &ActiveSubscriber) -> bool {
    match source {
        SourceOrigin::Subscribers { scope, profile_ref, .. } => active.accounts.iter().any(|a| {
            a.identity.scope == *scope && match profile_ref {
                None => true,
                Some(profile) => matches!(&a.policy.profile, ProfileObservation::Known(Some(current)) if current == profile),
            }
        }),
        SourceOrigin::Wan { .. } => false,
    }
}

/// Group demand relative to the capacity above an instance, in the worse direction.
// shortcut: f64 scores, exact enough for whole-bps loads; use exact ratios if ties misbehave.
fn utilization(load: RateBps, weight: RateBps) -> f64 {
    (load.down as f64 / weight.down.max(1) as f64).max(load.up as f64 / weight.up.max(1) as f64)
}

/// The share objective: move a subscriber when it lowers the worse utilization of the two
/// instances. Ties keep the current destination.
fn share_move(
    index: &Index<'_>,
    group: &[RateBps],
    from: usize,
    eligible: &[usize],
    base: RateBps,
    by_count: bool,
) -> anyhow::Result<usize> {
    // Count mode: every instance has the same weight, and `group` holds subscriber counts.
    let w = |t: usize| {
        if by_count {
            ONE
        } else {
            weight(index, &index.targets[t])
        }
    };
    let left = subtract(group[from], base)?;
    let mut best = (utilization(group[from], w(from)), from);
    for &to in eligible.iter().filter(|&&t| t != from) {
        let before = utilization(group[from], w(from)).max(utilization(group[to], w(to)));
        let after = utilization(left, w(from)).max(utilization(add(group[to], base)?, w(to)));
        if after < before && after < best.0 {
            best = (after, to);
        }
    }
    Ok(best.1)
}

/// Each instance of a source with the capacity above it: the nearest node with an independent
/// capacity on its path. An instance without such a node gets a weight of one in each direction.
pub(crate) fn instance_weights(
    catalog: &TopologyCatalog,
    source_id: &str,
) -> anyhow::Result<Vec<(String, RateBps)>> {
    let index = Index::new(catalog)?;
    Ok(index
        .targets
        .iter()
        .filter(|t| catalog.nodes[t.node].source_id.as_deref() == Some(source_id))
        .map(|t| (catalog.nodes[t.node].id.clone(), weight(&index, t)))
        .collect())
}

/// One subscriber in count mode.
const ONE: RateBps = RateBps { down: 1, up: 1 };

fn weight(index: &Index<'_>, target: &Target) -> RateBps {
    target
        .independent_path
        .first()
        .and_then(|&n| index.capacities[n])
        .unwrap_or(RateBps { down: 1, up: 1 })
}

/// Perform one explicit sweep. Equal scores retain the current destination; pins never enter the sweep.
pub fn rebalance(
    collection: Reconciliation,
    request: RebalanceRequest,
) -> anyhow::Result<RebalanceResult> {
    ensure!(
        collection.registry.revision == request.expected_revision,
        "rebalance policy revision is stale"
    );
    let mut baseline = resolve(collection)?;
    ensure!(
        baseline.conflicts.is_empty(),
        "rebalance requires resolved placements: {:?}",
        baseline.conflicts
    );
    let mut proposed_moves = Vec::new();
    let updates = {
        let candidate = baseline
            .candidate
            .as_ref()
            .context("rebalance requires a placement candidate")?;
        let registry_index = candidate.index()?;
        let catalog = candidate
            .catalog
            .as_ref()
            .context("rebalance requires a catalog")?;
        let index = Index::new(catalog)?;
        let scope = scope(&index, &request.scope)?;
        let bounded = scope
            .targets
            .iter()
            .filter(|t| index.targets[**t].capacity.is_some())
            .count();
        ensure!(
            bounded == 0 || bounded == scope.targets.len(),
            "rebalance scope mixes bounded and unbounded destinations"
        );
        let active: HashMap<_, _> = baseline
            .active
            .iter()
            .map(|a| (a.subscriber_id.as_str(), a))
            .collect();
        let mut loads = Loads {
            nodes: vec![ZERO; catalog.nodes.len()],
            targets: vec![ZERO; index.targets.len()],
            paths: index
                .targets
                .iter()
                .map(|t| t.independent_path.iter().copied().collect())
                .collect(),
        };
        let by_count = scope.source.is_some_and(|s| {
            matches!(&catalog.sources[s].origin,
                SourceOrigin::Subscribers { group, .. } if group.allocation == AllocationMode::Count)
        });
        // The share of one subscriber in the group totals: its plan, or one in count mode.
        let share = |base: RateBps| if by_count { ONE } else { base };
        let mut pending = Vec::new();
        let mut group = vec![ZERO; index.targets.len()];
        for (position, placement) in baseline.placements.iter().enumerate() {
            let target = index.target_ids[placement.target_id.as_str()];
            loads.targets[target] = add(loads.targets[target], placement.base)?;
            for &n in &index.targets[target].independent_path {
                loads.nodes[n] = add(loads.nodes[n], placement.base)?;
            }
            let record = registry_index.by_id[placement.subscriber_id.as_str()];
            let subscriber = &candidate.subscribers[record];
            if !scope.targets.contains(&target) {
                continue;
            }
            let current = active[placement.subscriber_id.as_str()];
            if scope
                .source
                .is_some_and(|s| !member(&catalog.sources[s].origin, current))
            {
                continue;
            }
            // Pins count in the group share but never move.
            group[target] = add(group[target], share(placement.base))?;
            if subscriber.policy.pin.is_some() {
                continue;
            }
            let prepared = index
                .prepare(subscriber, current)
                .map_err(|reason| anyhow::anyhow!("rebalance eligibility conflict: {reason:?}"))?;
            let eligible: Vec<_> = prepared
                .eligible
                .intersection(&scope.targets)
                .copied()
                .collect();
            pending.push((position, record, target, eligible));
        }
        pending.sort_unstable_by(|a, b| {
            let a = &baseline.placements[a.0];
            let b = &baseline.placements[b.0];
            b.base
                .down
                .max(b.base.up)
                .cmp(&a.base.down.max(a.base.up))
                .then_with(|| a.subscriber_id.cmp(&b.subscriber_id))
        });
        let mut updates = Vec::new();
        for (position, record, from, eligible) in pending {
            let placement = &baseline.placements[position];
            let dest = if scope.source.is_some() {
                share_move(
                    &index,
                    &group,
                    from,
                    &eligible,
                    share(placement.base),
                    by_count,
                )?
            } else {
                let mut best = (Delta::default(), from);
                for to in eligible {
                    if to == from {
                        continue;
                    }
                    let (delta, improves) = loads.trial(&index, from, to, placement.base)?;
                    if improves && delta.compare(&best.0) == Ordering::Less {
                        best = (delta, to);
                    }
                }
                best.1
            };
            if dest != from {
                loads.apply(from, dest, placement.base)?;
                group[from] = subtract(group[from], share(placement.base))?;
                group[dest] = add(group[dest], share(placement.base))?;
                let to = catalog.nodes[index.targets[dest].node].id.clone();
                proposed_moves.push(ProposedMove {
                    subscriber_id: placement.subscriber_id.clone(),
                    from_target_id: placement.target_id.clone(),
                    to_target_id: to.clone(),
                });
                updates.push((position, record, to));
            }
        }
        updates
    };
    let candidate = baseline
        .candidate
        .as_mut()
        .context("rebalance candidate is missing")?;
    for (position, record, to) in updates {
        baseline.placements[position].target_id = to.clone();
        candidate.subscribers[record].policy.assignment = Some(to);
    }
    proposed_moves.sort_unstable_by(|a, b| a.subscriber_id.cmp(&b.subscriber_id));
    let evaluation = evaluate(baseline)?;
    Ok(RebalanceResult {
        scope: request.scope,
        proposed_moves,
        evaluation,
    })
}

#[cfg(test)]
#[path = "rebalance_tests.rs"]
mod tests;
