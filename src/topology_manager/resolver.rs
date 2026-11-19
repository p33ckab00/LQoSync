//! Pure placement from a complete collection. A placement candidate is not a verified native world.

#[path = "rebalance.rs"]
pub mod rebalance;

use super::catalog::{BaseRateAuthority, CapacityPolicy, SourceOrigin, TopologyCatalog};
use super::layout::NodeKind;
use super::model::{RegistryDocument, SourceScope, SubscriberRecord};
use super::observation::{ActiveSubscriber, ProfileObservation, Reconciliation};
use super::rates::{
    AggregateRule, RateBps, SubscriberRates, resolve_aggregate, resolve_subscriber,
};
use anyhow::{Context, ensure};
use std::cmp::Ordering;
use std::collections::{BTreeSet, HashMap, HashSet, VecDeque};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PlacementConflictReason {
    MissingProfileEvidence,
    MissingRateAuthority,
    ConflictingProfiles,
    MissingPlan,
    ConflictingPlans,
    NoEligibleTarget,
    InvalidPin(String),
    InvalidAssignment(String),
    InvalidRates(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlacementConflict {
    pub subscriber_id: String,
    pub reason: PlacementConflictReason,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Placement {
    pub subscriber_id: String,
    pub target_id: String,
    pub profile_id: Option<String>,
    pub base: RateBps,
    pub rates: SubscriberRates,
}

#[derive(Debug)]
pub struct PlacementResolution {
    pub generation: u64,
    pub active: Vec<ActiveSubscriber>,
    pub placements: Vec<Placement>,
    pub conflicts: Vec<PlacementConflict>,
    /// Present only if all required active placements pass. Aggregate/native checks remain required.
    pub candidate: Option<RegistryDocument>,
}

#[derive(Default)]
struct Selectors<'a> {
    all: Vec<usize>,
    profiles: HashMap<&'a str, Vec<usize>>,
}

struct Target {
    node: usize,
    capacity: Option<RateBps>,
    independent_path: Vec<usize>,
    source_path: Vec<usize>,
}

struct Index<'a> {
    catalog: &'a TopologyCatalog,
    selectors: HashMap<&'a SourceScope, Selectors<'a>>,
    source_targets: Vec<Vec<usize>>,
    targets: Vec<Target>,
    target_ids: HashMap<&'a str, usize>,
    profiles: HashMap<&'a str, usize>,
    capacities: Vec<Option<RateBps>>,
    /// Targets for subscribers whose source the catalog does not know.
    fallback_targets: BTreeSet<usize>,
}

impl<'a> Index<'a> {
    fn new(catalog: &'a TopologyCatalog) -> anyhow::Result<Self> {
        let layout = catalog.layout()?;
        let nodes: HashMap<_, _> = catalog
            .nodes
            .iter()
            .enumerate()
            .map(|(i, n)| (n.id.as_str(), i))
            .collect();
        let sources: HashMap<_, _> = catalog
            .sources
            .iter()
            .enumerate()
            .map(|(i, s)| (s.id.as_str(), i))
            .collect();
        let parents: Vec<_> = catalog
            .nodes
            .iter()
            .map(|n| n.parent.as_deref().map(|p| nodes[p]))
            .collect();
        let mut children = vec![Vec::new(); layout.len()];
        let mut queue = VecDeque::new();
        for (i, parent) in parents.iter().enumerate() {
            if let Some(p) = parent {
                children[*p].push(i);
            } else {
                queue.push_back(i);
            }
        }
        let mut capacities = vec![None; layout.len()];
        let mut wans = vec![None; layout.len()];
        while let Some(i) = queue.pop_front() {
            let parent = parents[i].and_then(|p| capacities[p]);
            let wan = if catalog.nodes[i].kind == NodeKind::Wan {
                match catalog.nodes[i].capacity {
                    CapacityPolicy::Fixed { base, .. } => Some(base),
                    _ => unreachable!("validated WAN capacity"),
                }
            } else {
                parents[i].and_then(|p| wans[p])
            };
            wans[i] = wan;
            capacities[i] = match layout[i].capacity {
                AggregateRule::Automatic { .. } => parent,
                _ => Some(
                    resolve_aggregate(
                        &layout[i].capacity,
                        RateBps { down: 1, up: 1 },
                        wan,
                        parent,
                    )?
                    .ceiling,
                ),
            };
            queue.extend(children[i].iter().copied());
        }
        let mut leaves: Vec<_> = (0..layout.len())
            .filter(|i| children[*i].is_empty() && layout[*i].kind != NodeKind::Intermediate)
            .collect();
        leaves.sort_unstable_by_key(|i| &catalog.nodes[*i].id);
        let mut targets = Vec::with_capacity(leaves.len());
        let mut target_ids = HashMap::new();
        for i in leaves {
            let mut cursor = Some(i);
            let mut independent_path = Vec::new();
            let mut source_path = Vec::new();
            while let Some(n) = cursor {
                if matches!(
                    layout[n].capacity,
                    AggregateRule::Fixed(_) | AggregateRule::WanFraction { .. }
                ) {
                    independent_path.push(n);
                }
                if let Some(source) = catalog.nodes[n].source_id.as_deref() {
                    let source = sources[source];
                    if matches!(
                        catalog.sources[source].origin,
                        SourceOrigin::Subscribers { .. }
                    ) {
                        source_path.push(source);
                    }
                }
                cursor = parents[n];
            }
            target_ids.insert(catalog.nodes[i].id.as_str(), targets.len());
            targets.push(Target {
                node: i,
                capacity: capacities[i],
                independent_path,
                source_path,
            });
        }
        let mut source_targets = vec![Vec::new(); catalog.sources.len()];
        let selected: HashSet<_> = catalog
            .nodes
            .iter()
            .filter_map(|n| n.source_id.as_deref())
            .collect();
        let mut selectors: HashMap<&SourceScope, Selectors<'_>> = HashMap::new();
        for (i, source) in catalog.sources.iter().enumerate() {
            if let SourceOrigin::Subscribers {
                scope, profile_ref, ..
            } = &source.origin
            {
                if source.targets.is_empty() && !selected.contains(source.id.as_str()) {
                    continue;
                }
                let selectors = selectors.entry(scope).or_default();
                match profile_ref {
                    Some(profile) => selectors.profiles.entry(profile).or_default().push(i),
                    None => selectors.all.push(i),
                }
                source_targets[i].extend(source.targets.iter().map(|id| target_ids[id.as_str()]));
            }
        }
        for (t, target) in targets.iter().enumerate() {
            if let Some(source) = catalog.nodes[target.node].source_id.as_deref()
                && matches!(
                    catalog.sources[sources[source]].origin,
                    SourceOrigin::Subscribers { .. }
                )
                && catalog.sources[sources[source]].targets.is_empty()
            {
                source_targets[sources[source]].push(t);
            }
        }
        for targets in &mut source_targets {
            targets.sort_unstable();
            targets.dedup();
        }
        let fallback_targets = catalog
            .fallback
            .iter()
            .flat_map(|f| &f.node_ids)
            .filter_map(|id| target_ids.get(id.as_str()).copied())
            .collect();
        Ok(Self {
            catalog,
            selectors,
            source_targets,
            targets,
            target_ids,
            capacities,
            fallback_targets,
            profiles: catalog
                .profiles
                .iter()
                .enumerate()
                .map(|(i, p)| (p.id.as_str(), i))
                .collect(),
        })
    }

    fn prepare(
        &self,
        record: &SubscriberRecord,
        active: &ActiveSubscriber,
    ) -> Result<Prepared, PlacementConflictReason> {
        let mut eligible: Option<BTreeSet<usize>> = None;
        let mut defaults = BTreeSet::new();
        let mut any_matched = false;
        for account in &active.accounts {
            let mut matched = HashSet::new();
            if let Some(selectors) = self.selectors.get(&account.identity.scope) {
                matched.extend(selectors.all.iter().copied());
                match &account.policy.profile {
                    ProfileObservation::Unread if !selectors.profiles.is_empty() => {
                        return Err(PlacementConflictReason::MissingProfileEvidence);
                    }
                    ProfileObservation::Known(Some(profile)) => {
                        if let Some(sources) = selectors.profiles.get(profile.as_str()) {
                            matched.extend(sources.iter().copied());
                        }
                    }
                    _ => {}
                }
            }
            any_matched |= !matched.is_empty();
            let mut allowed = BTreeSet::new();
            for source in &matched {
                if let Some(profile) = self.catalog.sources[*source].profile_id.as_deref() {
                    defaults.insert(profile);
                }
                allowed.extend(self.source_targets[*source].iter().copied());
            }
            allowed.retain(|t| {
                self.targets[*t].source_path.iter().all(|s| {
                    matched.contains(s)
                        && (self.catalog.sources[*s].targets.is_empty()
                            || self.source_targets[*s].binary_search(t).is_ok())
                })
            });
            if let Some(eligible) = &mut eligible {
                eligible.retain(|t| allowed.contains(t));
            } else {
                eligible = Some(allowed);
            }
        }
        let mut eligible = eligible.unwrap_or_default();
        // No source knows this subscriber: the fallback nodes take it.
        let fallback = !any_matched && !self.fallback_targets.is_empty();
        if fallback {
            eligible = self.fallback_targets.clone();
        }
        let profile_id = match record.policy.profile_id.as_deref() {
            Some(profile) => Some(profile),
            None if defaults.len() > 1 => return Err(PlacementConflictReason::ConflictingProfiles),
            None => defaults.first().copied(),
        };
        let profile = profile_id.map(|id| &self.catalog.profiles[self.profiles[id]]);
        if let Some(allowed) = profile.and_then(|p| p.eligible_targets.as_ref()) {
            let allowed: HashSet<_> = allowed
                .iter()
                .map(|id| self.target_ids[id.as_str()])
                .collect();
            eligible.retain(|t| allowed.contains(t));
        }
        let fixed = if let Some(pin) = &record.policy.pin {
            Some(
                *self
                    .target_ids
                    .get(pin.as_str())
                    .filter(|t| eligible.contains(t))
                    .ok_or_else(|| PlacementConflictReason::InvalidPin(pin.clone()))?,
            )
        } else if let Some(assignment) = &record.policy.assignment {
            // Owner decision: an assignment that is no longer eligible is dropped, and the
            // subscriber is placed again. A pin above stays strict.
            self.target_ids
                .get(assignment.as_str())
                .filter(|t| eligible.contains(t))
                .copied()
        } else {
            None
        };
        if eligible.is_empty() {
            return Err(PlacementConflictReason::NoEligibleTarget);
        }
        if fallback {
            let plan = active.accounts.iter().find_map(|a| a.policy.plan);
            let base = match profile.map(|p| p.base) {
                Some(BaseRateAuthority::Local(rate)) => rate,
                _ => plan
                    .or(self.catalog.fallback.as_ref().map(|f| f.default_plan))
                    .ok_or(PlacementConflictReason::MissingPlan)?,
            };
            let speed = profile.map(|p| p.speed).unwrap_or_default();
            let rates = resolve_subscriber(base, &self.catalog.defaults, &speed)
                .map_err(|error| PlacementConflictReason::InvalidRates(error.to_string()))?;
            return Ok(Prepared {
                eligible,
                fixed,
                base,
                rates,
                profile_id: profile_id.map(str::to_owned),
            });
        }
        let profile = profile.ok_or(PlacementConflictReason::MissingRateAuthority)?;
        let base = match profile.base {
            BaseRateAuthority::Local(rate) => rate,
            BaseRateAuthority::SourceRead => {
                let mut base = None;
                for account in &active.accounts {
                    let current = account
                        .policy
                        .plan
                        .ok_or(PlacementConflictReason::MissingPlan)?;
                    if base.is_some_and(|old| old != current) {
                        return Err(PlacementConflictReason::ConflictingPlans);
                    }
                    base = Some(current);
                }
                base.ok_or(PlacementConflictReason::MissingPlan)?
            }
        };
        let rates = resolve_subscriber(base, &self.catalog.defaults, &profile.speed)
            .map_err(|error| PlacementConflictReason::InvalidRates(error.to_string()))?;
        Ok(Prepared {
            eligible,
            fixed,
            base,
            rates,
            profile_id: profile_id.map(str::to_owned),
        })
    }
}

struct Prepared {
    eligible: BTreeSet<usize>,
    fixed: Option<usize>,
    base: RateBps,
    rates: SubscriberRates,
    profile_id: Option<String>,
}

const ZERO: RateBps = RateBps { down: 0, up: 0 };

fn add(a: RateBps, b: RateBps) -> anyhow::Result<RateBps> {
    Ok(RateBps {
        down: a
            .down
            .checked_add(b.down)
            .context("assigned download plan demand overflow")?,
        up: a
            .up
            .checked_add(b.up)
            .context("assigned upload plan demand overflow")?,
    })
}

#[derive(Clone, Copy, Debug)]
struct Ratio(u64, u64);

impl PartialEq for Ratio {
    fn eq(&self, other: &Self) -> bool {
        self.cmp(other) == Ordering::Equal
    }
}
impl Eq for Ratio {}

impl Ord for Ratio {
    fn cmp(&self, other: &Self) -> Ordering {
        (u128::from(self.0) * u128::from(other.1)).cmp(&(u128::from(other.0) * u128::from(self.1)))
    }
}
impl PartialOrd for Ratio {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

fn ratio(demand: RateBps, capacity: Option<RateBps>) -> Ratio {
    let capacity = capacity.unwrap_or(RateBps { down: 1, up: 1 });
    Ratio(demand.down, capacity.down).max(Ratio(demand.up, capacity.up))
}

/// Keep valid saved intent. This function does not perform an explicit rebalance or any I/O.
pub fn resolve(collection: Reconciliation) -> anyhow::Result<PlacementResolution> {
    let registry_index = collection.registry.index()?;
    let catalog = collection
        .registry
        .catalog
        .as_ref()
        .context("placement requires a configured catalog")?;
    let index = Index::new(catalog)?;
    let mut active = collection.active;
    active.sort_unstable_by(|a, b| a.subscriber_id.cmp(&b.subscriber_id));
    let mut seen = HashSet::new();
    let mut pending = Vec::new();
    let mut conflicts = Vec::new();
    for (a, subscriber) in active.iter().enumerate() {
        ensure!(
            seen.insert(subscriber.subscriber_id.as_str()),
            "duplicate active subscriber circuit"
        );
        let record = *registry_index
            .by_id
            .get(subscriber.subscriber_id.as_str())
            .context("active subscriber is absent from the registry")?;
        ensure!(
            !subscriber.accounts.is_empty() && !subscriber.endpoints.is_empty(),
            "active subscriber requires current accounts and endpoints"
        );
        let accounts: HashSet<_> = subscriber.accounts.iter().map(|a| &a.identity).collect();
        ensure!(
            accounts.len() == subscriber.accounts.len(),
            "duplicate active account evidence"
        );
        ensure!(
            accounts
                .iter()
                .all(|id| registry_index.by_identity.get(id).copied() == Some(record)),
            "active identity belongs to a different subscriber"
        );
        let endpoints: HashSet<_> = subscriber.endpoints.iter().map(|e| &e.identity).collect();
        ensure!(
            accounts == endpoints,
            "active account and endpoint identities differ"
        );
        match index.prepare(&collection.registry.subscribers[record], subscriber) {
            Ok(prepared) => pending.push((a, record, prepared)),
            Err(reason) => conflicts.push(PlacementConflict {
                subscriber_id: subscriber.subscriber_id.clone(),
                reason,
            }),
        }
    }
    // Reserve pins and saved assignments before any new circuit can consume demand.
    pending.sort_by_key(|(a, _, p)| (p.fixed.is_none(), *a));
    let mut node_demand = vec![ZERO; catalog.nodes.len()];
    let mut target_demand = vec![ZERO; index.targets.len()];
    let mut placements = Vec::with_capacity(pending.len());
    let mut assignments = Vec::with_capacity(pending.len());
    for (a, record, prepared) in pending {
        let target = match prepared.fixed {
            Some(target) => target,
            None => {
                let mut best = None;
                for &t in &prepared.eligible {
                    let target = &index.targets[t];
                    let local = ratio(add(target_demand[t], prepared.base)?, target.capacity);
                    let mut path = local;
                    for &n in &target.independent_path {
                        path = path.max(ratio(
                            add(node_demand[n], prepared.base)?,
                            index.capacities[n],
                        ));
                    }
                    let score = (path, local, t);
                    if best.is_none_or(|old| score < old) {
                        best = Some(score);
                    }
                }
                best.context("eligible placement set is empty")?.2
            }
        };
        target_demand[target] = add(target_demand[target], prepared.base)?;
        for &n in &index.targets[target].independent_path {
            node_demand[n] = add(node_demand[n], prepared.base)?;
        }
        let target_id = &catalog.nodes[index.targets[target].node].id;
        assignments.push((record, target_id.clone()));
        placements.push(Placement {
            subscriber_id: active[a].subscriber_id.clone(),
            target_id: target_id.clone(),
            profile_id: prepared.profile_id,
            base: prepared.base,
            rates: prepared.rates,
        });
    }
    placements.sort_unstable_by(|a, b| a.subscriber_id.cmp(&b.subscriber_id));
    let candidate = if conflicts.is_empty() {
        let mut candidate = collection.registry;
        for (record, target) in assignments {
            candidate.subscribers[record].policy.assignment = Some(target);
        }
        candidate.validate()?;
        Some(candidate)
    } else {
        None
    };
    Ok(PlacementResolution {
        generation: collection.generation,
        active,
        placements,
        conflicts,
        candidate,
    })
}

#[cfg(test)]
#[path = "resolver_tests.rs"]
mod tests;
