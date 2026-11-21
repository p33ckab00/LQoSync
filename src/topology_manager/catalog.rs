//! Durable catalog definitions and hierarchy input. Availability and endpoints stay separate.

use super::layout::{NodeKind, PolicyNode, validate_layout};
use super::model::{SourceKind, SourceScope, SubscriberRecord};
use super::rates::{
    AggregateRule, Fraction, RateBps, RateRule, RuleValue, SpeedOverrides, SpeedPolicy,
    new_policy_defaults, resolve_subscriber,
};
use anyhow::{Context, ensure};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeSet, HashMap, HashSet};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum RouterRole {
    Wan,
    Ppp,
    Dhcp,
    Hotspot,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CatalogRouter {
    pub id: String,
    pub name: String,
    pub roles: Vec<RouterRole>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Factors {
    pub down: Fraction,
    pub up: Fraction,
}

impl Default for Factors {
    fn default() -> Self {
        let one = Fraction {
            numerator: 1,
            denominator: 1,
        };
        Self { down: one, up: one }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum FactorChoice {
    Inherit,
    Override(Factors),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum GroupFormula {
    PlanSum,
    CountTimesUnit(RateBps),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BackstopPolicy {
    pub base: RateBps,
    pub factor: Fraction,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GroupDefaults {
    pub formula: GroupFormula,
    pub factors: Factors,
    pub backstop: Option<BackstopPolicy>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub floor: Option<RateBps>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub enum SourceOrigin {
    Wan {
        router_id: String,
        endpoint_ref: String,
    },
    Subscribers {
        scope: SourceScope,
        profile_ref: Option<String>,
        group: GroupDefaults,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CatalogSource {
    pub id: String,
    pub name: String,
    pub origin: SourceOrigin,
    pub profile_id: Option<String>,
    pub targets: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BaseRateAuthority {
    Local(RateBps),
    SourceRead,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SubscriberProfile {
    pub id: String,
    pub name: String,
    pub base: BaseRateAuthority,
    pub speed: SpeedOverrides,
    pub eligible_targets: Option<Vec<String>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub enum CapacityPolicy {
    Inherit,
    Fixed {
        base: RateBps,
        factors: FactorChoice,
    },
    WanPercentage {
        down: Fraction,
        up: Fraction,
    },
    Automatic {
        factors: FactorChoice,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CatalogNode {
    #[serde(default, skip_serializing_if = "super::rates::NodeMinimum::is_empty")]
    pub minimum: super::rates::NodeMinimum,
    pub id: String,
    pub name: String,
    pub parent: Option<String>,
    pub source_id: Option<String>,
    pub kind: NodeKind,
    pub capacity: CapacityPolicy,
    /// The `network.json` type and location. Added in registry schema 5.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub geo: Option<web_contract::topology::NodeGeo>,
}

/// Group nodes that take subscribers whose source the catalog does not know.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FallbackPolicy {
    pub node_ids: Vec<String>,
    /// The plan of a fallback subscriber without a readable router rate-limit.
    pub default_plan: RateBps,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TopologyCatalog {
    pub root_label: String,
    pub routers: Vec<CatalogRouter>,
    pub sources: Vec<CatalogSource>,
    pub profiles: Vec<SubscriberProfile>,
    pub defaults: SpeedPolicy,
    pub nodes: Vec<CatalogNode>,
    /// Absent in registries written before Phase 2c-2; such files serialize unchanged.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fallback: Option<FallbackPolicy>,
}

impl Default for TopologyCatalog {
    fn default() -> Self {
        Self {
            root_label: "Topology".into(),
            routers: Vec::new(),
            sources: Vec::new(),
            profiles: Vec::new(),
            defaults: new_policy_defaults(),
            nodes: Vec::new(),
            fallback: None,
        }
    }
}

impl TopologyCatalog {
    pub fn validate(&self) -> anyhow::Result<()> {
        self.index().map(|_| ())
    }

    /// Check configured potential destinations. Current source-profile membership remains separate.
    pub fn validate_references(&self, subscribers: &[SubscriberRecord]) -> anyhow::Result<()> {
        self.checked_scopes(subscribers).map(|_| ())
    }

    pub(super) fn checked_scopes(
        &self,
        subscribers: &[SubscriberRecord],
    ) -> anyhow::Result<Vec<SourceScope>> {
        let index = self.index()?;
        for subscriber in subscribers {
            ensure!(
                !subscriber.identities.is_empty(),
                "subscriber requires identity references"
            );
            for identity in &subscriber.identities {
                ensure!(
                    index.scope_targets.contains_key(&identity.scope),
                    "subscriber source scope is absent from the catalog"
                );
            }
            if let Some(profile) = &subscriber.policy.profile_id {
                ensure!(
                    index.profiles.contains_key(profile.as_str()),
                    "subscriber profile is absent from the catalog"
                );
            }
            for target in [&subscriber.policy.assignment, &subscriber.policy.pin]
                .into_iter()
                .flatten()
            {
                ensure!(
                    index.leaves.contains(target.as_str()),
                    "subscriber destination is absent or cannot contain direct subscribers"
                );
                if let Some(profile) = &subscriber.policy.profile_id
                    && let Some(allowed) = index.profile_targets.get(profile.as_str())
                {
                    ensure!(
                        allowed.contains(target.as_str()),
                        "subscriber destination violates its profile restriction"
                    );
                }
                // A fallback node takes subscribers of any scope.
                let fallback = self
                    .fallback
                    .as_ref()
                    .is_some_and(|f| f.node_ids.iter().any(|n| n == target));
                ensure!(
                    fallback
                        || subscriber.identities.iter().any(|identity| index
                            .scope_targets
                            .get(&identity.scope)
                            .is_some_and(|targets| targets.contains(target.as_str()))),
                    "subscriber destination is outside configured source targets"
                );
            }
        }
        Ok(index.selected_scopes.into_iter().collect())
    }

    pub(super) fn canonicalize(&mut self) {
        self.routers.sort_unstable_by(|a, b| a.id.cmp(&b.id));
        for router in &mut self.routers {
            router.roles.sort_unstable();
        }
        self.sources.sort_unstable_by(|a, b| a.id.cmp(&b.id));
        for source in &mut self.sources {
            source.targets.sort_unstable();
        }
        self.profiles.sort_unstable_by(|a, b| a.id.cmp(&b.id));
        for profile in &mut self.profiles {
            if let Some(targets) = &mut profile.eligible_targets {
                targets.sort_unstable();
            }
        }
        self.nodes.sort_unstable_by(|a, b| a.id.cmp(&b.id));
    }

    pub(super) fn validate_origins(&self, previous: &Self) -> anyhow::Result<()> {
        let previous: HashMap<_, _> = previous
            .sources
            .iter()
            .map(|source| (source.id.as_str(), source.origin.key()))
            .collect();
        for source in &self.sources {
            if let Some(origin) = previous.get(source.id.as_str()) {
                ensure!(
                    *origin == source.origin.key(),
                    "existing source ID cannot change its authoritative origin"
                );
            }
        }
        Ok(())
    }

    /// Return local shaping rules. WAN percentages still need the stored configured WAN base.
    pub fn layout(&self) -> anyhow::Result<Vec<PolicyNode>> {
        Ok(self.index()?.layout)
    }

    /// Subscriber collection scopes only. WAN inventory uses the router's separate read role.
    pub fn required_scopes(&self) -> anyhow::Result<Vec<SourceScope>> {
        Ok(self.index()?.selected_scopes.into_iter().collect())
    }

    fn index(&self) -> anyhow::Result<CatalogIndex<'_>> {
        nonempty(&self.root_label, "synthetic root label")?;
        validate_speed(&self.defaults, &SpeedOverrides::default(), None)?;
        let routers = named_index(&self.routers, |r| (&r.id, &r.name)).context("router catalog")?;
        let sources = named_index(&self.sources, |s| (&s.id, &s.name)).context("source catalog")?;
        let profiles =
            named_index(&self.profiles, |p| (&p.id, &p.name)).context("profile catalog")?;
        let nodes = named_index(&self.nodes, |n| (&n.id, &n.name)).context("node catalog")?;
        for node in &self.nodes {
            if let Some(location) = node.geo.as_ref().and_then(|g| g.location.as_ref()) {
                location
                    .validate()
                    .map_err(anyhow::Error::msg)
                    .with_context(|| format!("node {} location", node.id))?;
            }
        }
        for router in &self.routers {
            ensure!(
                router.roles.iter().collect::<HashSet<_>>().len() == router.roles.len(),
                "duplicate router read role"
            );
        }
        let mut selected: HashSet<&str> = self
            .sources
            .iter()
            .filter(|s| !s.targets.is_empty())
            .map(|s| s.id.as_str())
            .collect();
        selected.extend(self.nodes.iter().filter_map(|n| n.source_id.as_deref()));
        let mut origins = HashSet::with_capacity(self.sources.len());
        let mut selected_scopes = BTreeSet::new();
        let mut scope_targets: HashMap<&SourceScope, HashSet<&str>> = HashMap::new();
        for source in &self.sources {
            if let Some(profile) = &source.profile_id {
                ensure!(
                    profiles.contains_key(profile.as_str()),
                    "source profile is absent from the catalog"
                );
            }
            let (router_id, role) = match &source.origin {
                SourceOrigin::Wan {
                    router_id,
                    endpoint_ref,
                } => {
                    nonempty(endpoint_ref, "WAN endpoint reference")?;
                    ensure!(
                        source.profile_id.is_none() && source.targets.is_empty(),
                        "WAN inventory cannot define subscriber policy"
                    );
                    (router_id, RouterRole::Wan)
                }
                SourceOrigin::Subscribers {
                    scope,
                    profile_ref,
                    group,
                } => {
                    scope.validate()?;
                    if let Some(profile) = profile_ref {
                        nonempty(profile, "source profile reference")?;
                    }
                    group.factors.validate()?;
                    let valid_formula = match scope.kind {
                        SourceKind::Ppp => matches!(group.formula, GroupFormula::PlanSum),
                        SourceKind::Hotspot => {
                            matches!(group.formula, GroupFormula::CountTimesUnit(_))
                        }
                        // DHCP site and per-plan modes have different declared bases.
                        SourceKind::Dhcp => true,
                    };
                    ensure!(
                        valid_formula,
                        "group formula does not match its source kind"
                    );
                    if let GroupFormula::CountTimesUnit(unit) = group.formula {
                        unit.validate()?;
                    }
                    if let Some(cap) = group.backstop {
                        cap.base.apply_factors(cap.factor, cap.factor)?;
                    }
                    if let Some(floor) = group.floor {
                        floor.validate()?;
                    }
                    scope_targets
                        .entry(scope)
                        .or_default()
                        .extend(source.targets.iter().map(String::as_str));
                    if selected.contains(source.id.as_str()) {
                        selected_scopes.insert(scope.clone());
                    }
                    let role = match scope.kind {
                        SourceKind::Ppp => RouterRole::Ppp,
                        SourceKind::Dhcp => RouterRole::Dhcp,
                        SourceKind::Hotspot => RouterRole::Hotspot,
                    };
                    (&scope.router_id, role)
                }
            };
            let router = routers
                .get(router_id.as_str())
                .context("source router is absent from the catalog")?;
            if selected.contains(source.id.as_str()) {
                ensure!(
                    router.roles.contains(&role),
                    "selected source requires its router read role"
                );
            }
            ensure!(
                origins.insert(source.origin.key()),
                "duplicate source origin"
            );
        }
        for profile in &self.profiles {
            let base = match profile.base {
                BaseRateAuthority::Local(rate) => Some(rate),
                BaseRateAuthority::SourceRead => None,
            };
            validate_speed(&self.defaults, &profile.speed, base)?;
        }

        let mut layout = Vec::with_capacity(self.nodes.len());
        let mut instances = HashSet::new();
        for node in &self.nodes {
            node.minimum.validate()?;
            let source = node
                .source_id
                .as_deref()
                .map(|id| {
                    sources
                        .get(id)
                        .copied()
                        .context("node source is absent from the catalog")
                })
                .transpose()?;
            match (node.kind, source.map(|s| &s.origin)) {
                (NodeKind::Wan, Some(SourceOrigin::Wan { .. })) => {}
                (NodeKind::Wan, _) => anyhow::bail!("WAN node requires a WAN source"),
                (_, Some(SourceOrigin::Wan { .. })) => {
                    anyhow::bail!("WAN source requires a WAN node")
                }
                _ => {}
            }
            if let Some(source_id) = node.source_id.as_deref() {
                ensure!(
                    instances.insert((source_id, node.parent.as_deref())),
                    "duplicate source instance under the same parent"
                );
            }
            let inherited = match source.map(|s| &s.origin) {
                Some(SourceOrigin::Subscribers { group, .. }) => group.factors,
                _ => Factors::default(),
            };
            let capacity = match node.capacity {
                CapacityPolicy::Inherit => AggregateRule::Inherit,
                CapacityPolicy::Fixed { base, factors } => {
                    let factors = factors.resolve(inherited);
                    if node.kind == NodeKind::Wan {
                        ensure!(
                            factors.down.numerator <= factors.down.denominator
                                && factors.up.numerator <= factors.up.denominator,
                            "WAN shaping factors cannot exceed configured true capacity"
                        );
                    }
                    AggregateRule::Fixed(base.apply_factors(factors.down, factors.up)?)
                }
                CapacityPolicy::WanPercentage { down, up } => {
                    AggregateRule::WanFraction { down, up }
                }
                CapacityPolicy::Automatic { factors } => {
                    let factors = factors.resolve(inherited);
                    factors.validate()?;
                    AggregateRule::Automatic {
                        down: factors.down,
                        up: factors.up,
                    }
                }
            };
            layout.push(PolicyNode {
                id: node.id.clone(),
                name: node.name.clone(),
                parent: node.parent.clone(),
                kind: node.kind,
                capacity,
                direct_subscribers: 0,
            });
        }
        validate_layout(&layout)?;
        let parents: HashSet<_> = self
            .nodes
            .iter()
            .filter_map(|n| n.parent.as_deref())
            .collect();
        let leaves: HashSet<_> = self
            .nodes
            .iter()
            .filter(|n| n.kind != NodeKind::Intermediate && !parents.contains(n.id.as_str()))
            .map(|n| n.id.as_str())
            .collect();
        // An explicit source instance is a selected placement without a shared target rule.
        for node in self.nodes.iter().filter(|n| leaves.contains(n.id.as_str())) {
            if let Some(source) = node.source_id.as_deref()
                && let SourceOrigin::Subscribers { scope, .. } = &sources[source].origin
                && sources[source].targets.is_empty()
            {
                scope_targets
                    .entry(scope)
                    .or_default()
                    .insert(node.id.as_str());
            }
        }
        for source in &self.sources {
            validate_targets(&source.targets, &nodes, &leaves)?;
        }
        let mut profile_targets = HashMap::new();
        for profile in &self.profiles {
            if let Some(targets) = &profile.eligible_targets {
                ensure!(
                    !targets.is_empty(),
                    "explicit profile targets must not be empty"
                );
                validate_targets(targets, &nodes, &leaves)?;
                profile_targets.insert(
                    profile.id.as_str(),
                    targets.iter().map(String::as_str).collect::<HashSet<_>>(),
                );
            }
        }
        if let Some(fallback) = &self.fallback {
            ensure!(!fallback.node_ids.is_empty(), "the fallback needs a node");
            ensure!(
                fallback.default_plan.down > 0 && fallback.default_plan.up > 0,
                "the fallback default plan must be positive"
            );
            let mut seen = HashSet::new();
            for id in &fallback.node_ids {
                ensure!(seen.insert(id.as_str()), "duplicate fallback node");
                ensure!(
                    self.nodes.iter().any(|n| n.id == *id
                        && n.kind == NodeKind::SubscriberGroup
                        && n.source_id.is_none()),
                    "a fallback node must be a group without a source"
                );
            }
        }
        Ok(CatalogIndex {
            profiles,
            leaves,
            scope_targets,
            profile_targets,
            selected_scopes,
            layout,
        })
    }
}

struct CatalogIndex<'a> {
    profiles: HashMap<&'a str, &'a SubscriberProfile>,
    leaves: HashSet<&'a str>,
    scope_targets: HashMap<&'a SourceScope, HashSet<&'a str>>,
    profile_targets: HashMap<&'a str, HashSet<&'a str>>,
    selected_scopes: BTreeSet<SourceScope>,
    layout: Vec<PolicyNode>,
}

#[derive(PartialEq, Eq, Hash)]
enum OriginKey<'a> {
    Wan(&'a str, &'a str),
    Subscribers(&'a SourceScope, Option<&'a str>),
}

impl SourceOrigin {
    fn key(&self) -> OriginKey<'_> {
        match self {
            Self::Wan {
                router_id,
                endpoint_ref,
            } => OriginKey::Wan(router_id, endpoint_ref),
            Self::Subscribers {
                scope, profile_ref, ..
            } => OriginKey::Subscribers(scope, profile_ref.as_deref()),
        }
    }
}

fn nonempty(value: &str, field: &str) -> anyhow::Result<()> {
    ensure!(!value.trim().is_empty(), "{field} must not be empty");
    Ok(())
}

fn named_index<'a, T>(
    items: &'a [T],
    fields: impl Fn(&'a T) -> (&'a str, &'a str),
) -> anyhow::Result<HashMap<&'a str, &'a T>> {
    let mut index = HashMap::with_capacity(items.len());
    for item in items {
        let (id, name) = fields(item);
        nonempty(id, "catalog ID")?;
        nonempty(name, "catalog label")?;
        ensure!(index.insert(id, item).is_none(), "duplicate catalog ID");
    }
    Ok(index)
}

impl Factors {
    fn validate(self) -> anyhow::Result<()> {
        self.down.validate()?;
        self.up.validate()
    }
}

impl FactorChoice {
    fn resolve(self, inherited: Factors) -> Factors {
        match self {
            Self::Inherit => inherited,
            Self::Override(factors) => factors,
        }
    }
}

fn validate_targets(
    targets: &[String],
    nodes: &HashMap<&str, &CatalogNode>,
    leaves: &HashSet<&str>,
) -> anyhow::Result<()> {
    let mut unique = HashSet::with_capacity(targets.len());
    for target in targets {
        ensure!(
            nodes.contains_key(target.as_str()) && leaves.contains(target.as_str()),
            "configured target must be a node that can contain direct subscribers"
        );
        ensure!(
            unique.insert(target.as_str()),
            "duplicate configured target"
        );
    }
    Ok(())
}

fn validate_speed(
    defaults: &SpeedPolicy,
    overrides: &SpeedOverrides,
    base: Option<RateBps>,
) -> anyhow::Result<()> {
    for (rule, value) in [
        (defaults.down_min, overrides.down_min),
        (defaults.up_min, overrides.up_min),
        (defaults.down_max, overrides.down_max),
        (defaults.up_max, overrides.up_max),
    ] {
        rule.validate()?;
        if let RuleValue::Override(rule) = value {
            rule.validate()?;
        }
    }
    if let Some(base) = base {
        resolve_subscriber(base, defaults, overrides)?;
    }
    for (min, max) in [
        (
            overrides.down_min.select(defaults.down_min),
            overrides.down_max.select(defaults.down_max),
        ),
        (
            overrides.up_min.select(defaults.up_min),
            overrides.up_max.select(defaults.up_max),
        ),
    ] {
        let valid = match (min, max) {
            (RateRule::FixedBps(min), RateRule::FixedBps(max)) => min <= max,
            (RateRule::BaseFraction(min), RateRule::BaseFraction(max)) => {
                u128::from(min.numerator) * u128::from(max.denominator)
                    <= u128::from(max.numerator) * u128::from(min.denominator)
            }
            (
                RateRule::LegacyRoundedBase {
                    factor: min,
                    floor_bps: min_floor,
                },
                RateRule::LegacyRoundedBase {
                    factor: max,
                    floor_bps: max_floor,
                },
            ) => {
                min_floor <= max_floor
                    && u128::from(min.numerator) * u128::from(max.denominator)
                        <= u128::from(max.numerator) * u128::from(min.denominator)
            }
            _ => true,
        };
        ensure!(valid, "subscriber minimum rule exceeds its maximum rule");
    }
    Ok(())
}

#[cfg(test)]
#[path = "catalog_tests.rs"]
pub(super) mod tests;
