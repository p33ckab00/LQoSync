//! Read-only compatibility checks. A clear report does not authorize service activation.
use super::{
    catalog::{BaseRateAuthority, CapacityPolicy, FactorChoice, GroupFormula, SourceOrigin},
    identity::SourceSelection,
    legacy_rate::{LegacyPriority, LegacyRateSource},
    model::{RegistryDocument, SourceKind, SourceScope},
    rates::{Fraction, RateBps, resolve_subscriber},
};
use crate::config::{Config, DhcpMode, RouterCfg};
use anyhow::{Context, ensure};
use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum PolicyIssueKind {
    IpExclusions,
    PppProfileExclusions,
    DhcpServerExclusions,
    DhcpAddressOverrides,
    DhcpMembership,
    SourcePriority,
    BaseAuthority,
    SubscriberBounds,
    DynamicSubscriberBounds,
    GroupFactors,
    GroupBackstop,
    GroupMinimum,
    GroupCapacity,
    GroupFormula,
    DhcpServicePolicy,
}
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct PolicyIssue {
    pub scope: SourceScope,
    /// A setting path or catalog ID. Never an address, credential, or source comment.
    pub subject: String,
    pub kind: PolicyIssueKind,
}

fn issue(
    out: &mut Vec<PolicyIssue>,
    selection: &SourceSelection,
    subject: &str,
    kind: PolicyIssueKind,
) {
    out.push(PolicyIssue {
        scope: selection.scope.clone(),
        subject: subject.into(),
        kind,
    });
}
fn selected<'a>(
    cfg: &'a Config,
    registry: &'a RegistryDocument,
) -> anyhow::Result<Vec<(&'a SourceSelection, &'a RouterCfg)>> {
    registry.validate()?;
    let catalog = registry
        .catalog
        .as_ref()
        .context("topology catalog is unconfigured")?;
    let selections = registry
        .source_settings
        .as_ref()
        .context("source settings are unconfigured")?
        .ready_selections(catalog)?;
    let by_id: HashMap<_, _> = catalog.routers.iter().map(|r| (r.id.as_str(), r)).collect();
    let mut by_name: HashMap<&str, Vec<_>> = HashMap::new();
    for router in &cfg.routers {
        by_name.entry(&router.name).or_default().push(router);
    }
    let mut names = HashMap::new();
    let mut out = Vec::with_capacity(selections.len());
    for selection in selections {
        let router = by_id
            .get(selection.scope.router_id.as_str())
            .context("selected catalog router is missing")?;
        let matches = by_name
            .get(router.name.as_str())
            .context("selected router has no configured connection")?;
        ensure!(
            matches.len() == 1
                && names
                    .insert(router.name.as_str(), router.id.as_str())
                    .is_none_or(|old| old == router.id),
            "selected router connection is ambiguous"
        );
        let configured = matches[0];
        let enabled = match selection.scope.kind {
            SourceKind::Ppp => configured.pppoe.enabled,
            SourceKind::Dhcp => configured.dhcp.enabled,
            SourceKind::Hotspot => configured.hotspot.enabled,
        };
        ensure!(enabled, "selected subscriber protocol is disabled");
        out.push((selection, configured));
    }
    Ok(out)
}
fn source_checks(
    cfg: &Config,
    registry: &RegistryDocument,
    bindings: &[(&SourceSelection, &RouterCfg)],
) -> Vec<PolicyIssue> {
    let mut out = super::dhcp_overrides::issues(registry, bindings);
    for &(selection, router) in bindings {
        for (prefix, exclude) in [
            ("exclude", &cfg.exclude),
            ("routers.exclude", &router.exclude),
        ] {
            if exclude
                .ips
                .iter()
                .any(|ip| crate::config::parse_ipv4_net(ip).is_none())
            {
                issue(
                    &mut out,
                    selection,
                    &format!("{prefix}.ips"),
                    PolicyIssueKind::IpExclusions,
                );
            }
            if selection.scope.kind == SourceKind::Ppp
                && exclude
                    .ppp_profiles
                    .iter()
                    .any(|name| name.trim().is_empty())
            {
                issue(
                    &mut out,
                    selection,
                    &format!("{prefix}.ppp_profiles"),
                    PolicyIssueKind::PppProfileExclusions,
                );
            }
            if selection.scope.kind == SourceKind::Dhcp
                && exclude
                    .dhcp_servers
                    .iter()
                    .any(|name| name.trim().is_empty())
            {
                issue(
                    &mut out,
                    selection,
                    &format!("{prefix}.dhcp_servers"),
                    PolicyIssueKind::DhcpServerExclusions,
                );
            }
        }
        if selection.scope.kind == SourceKind::Dhcp {
            let declared: BTreeSet<_> = router
                .dhcp
                .servers
                .iter()
                .map(|s| s.name.as_str())
                .collect();
            let selected: BTreeSet<_> =
                selection.service_names.iter().map(String::as_str).collect();
            if !router.dhcp.require_bound
                || router.dhcp.shape_unassigned
                || (!router.dhcp.discover && (selected.is_empty() || selected != declared))
            {
                issue(
                    &mut out,
                    selection,
                    "routers.dhcp",
                    PolicyIssueKind::DhcpMembership,
                );
            }
        }
    }
    out.sort();
    out.dedup();
    out
}

/// Report source rules that the configured collector cannot apply. No current lease can waive them.
pub fn source_issues(
    cfg: &Config,
    registry: &RegistryDocument,
) -> anyhow::Result<Vec<PolicyIssue>> {
    Ok(source_checks(cfg, registry, &selected(cfg, registry)?))
}

/// Bind effective filters and DHCP override rates. The binding contains no session data.
pub(super) fn source_filter_digest(
    cfg: &Config,
    registry: &RegistryDocument,
) -> anyhow::Result<Option<String>> {
    let empty = |ex: &crate::config::Exclude| {
        ex.ips.is_empty() && ex.ppp_profiles.is_empty() && ex.dhcp_servers.is_empty()
    };
    if empty(&cfg.exclude)
        && cfg
            .routers
            .iter()
            .all(|r| empty(&r.exclude) && r.dhcp.clients.is_empty())
    {
        return Ok(None);
    }
    let mut bindings = selected(cfg, registry)?;
    bindings.sort_unstable_by(|a, b| a.0.scope.cmp(&b.0.scope));
    let mut records = Vec::new();
    let mut rates = BTreeMap::new();
    let none: &[String] = &[];
    for (selection, router) in bindings {
        let ppp = selection.scope.kind == SourceKind::Ppp;
        let dhcp = selection.scope.kind == SourceKind::Dhcp;
        let global = [
            &cfg.exclude.ips[..],
            if ppp {
                &cfg.exclude.ppp_profiles[..]
            } else {
                none
            },
            if dhcp {
                &cfg.exclude.dhcp_servers[..]
            } else {
                none
            },
        ];
        let local = [
            &router.exclude.ips[..],
            if ppp {
                &router.exclude.ppp_profiles[..]
            } else {
                none
            },
            if dhcp {
                &router.exclude.dhcp_servers[..]
            } else {
                none
            },
        ];
        // The legacy MAC is not a selector. Read each router's rate list once across its scopes.
        if dhcp && !router.dhcp.clients.is_empty() {
            rates
                .entry(selection.scope.router_id.as_str())
                .or_insert_with(|| {
                    (
                        router.name.as_str(),
                        router
                            .dhcp
                            .clients
                            .iter()
                            .map(|c| (c.download_mbps.to_bits(), c.upload_mbps.to_bits()))
                            .collect::<Vec<_>>(),
                    )
                });
        }
        if global.iter().chain(&local).any(|rules| !rules.is_empty()) {
            records.push((&selection.scope, router.name.as_str(), global, local));
        }
    }
    Ok(if records.is_empty() && rates.is_empty() {
        None
    } else {
        Some(crate::write_receipt::sha256(&serde_json::to_vec(&(
            records, rates,
        ))?))
    })
}

struct Expected<'a> {
    priority: LegacyPriority,
    factors: (f64, f64),
    cap: &'a crate::config::BackstopCap,
    node: &'a crate::config::NodeTemplate,
    count: bool,
}
fn expected<'a>(
    cfg: &'a Config,
    router: &RouterCfg,
    selection: &SourceSelection,
) -> anyhow::Result<Vec<Expected<'a>>> {
    let bd = &cfg.bandwidth_defaults;
    Ok(match selection.scope.kind {
        SourceKind::Ppp => vec![Expected {
            priority: LegacyPriority::ppp(&cfg.speed, &bd.pppoe_default_rate)?,
            factors: (bd.pppoe_aggregate_factor, bd.pppoe_aggregate_factor),
            cap: &cfg.backstop_caps.pppoe,
            node: if router.pppoe.per_plan_node {
                &cfg.nodes.ppp_plan
            } else {
                &cfg.nodes.ppp
            },
            count: false,
        }],
        SourceKind::Hotspot => vec![Expected {
            priority: LegacyPriority::hotspot(&cfg.speed, bd, &router.hotspot)?,
            factors: router.hotspot.factors(bd),
            cap: &cfg.backstop_caps.hotspot,
            node: &cfg.nodes.hotspot,
            count: true,
        }],
        SourceKind::Dhcp => {
            let names: Vec<_> = if selection.service_names.is_empty() {
                std::iter::once("")
                    .chain(router.dhcp.servers.iter().map(|s| s.name.as_str()))
                    .collect()
            } else {
                selection.service_names.iter().map(String::as_str).collect()
            };
            names
                .into_iter()
                .map(|name| {
                    let server = router.dhcp.server(name);
                    let site = server.mode == DhcpMode::PerSite;
                    // Match the legacy plan-group rule: the first per-plan server supplies its factors.
                    let factors = if site {
                        server.factors(bd)
                    } else {
                        router
                            .dhcp
                            .servers
                            .iter()
                            .find(|s| s.mode == DhcpMode::PerPlan)
                            .map_or((bd.dhcp_download_factor, bd.dhcp_upload_factor), |s| {
                                s.factors(bd)
                            })
                    };
                    Ok(Expected {
                        priority: LegacyPriority::dhcp(&cfg.speed, bd, &server)?,
                        factors,
                        cap: &cfg.backstop_caps.dhcp,
                        node: if site {
                            &cfg.nodes.dhcp_site
                        } else {
                            &cfg.nodes.dhcp_plan
                        },
                        count: site,
                    })
                })
                .collect::<anyhow::Result<Vec<_>>>()?
        }
    })
}
// Compare fractions without f64 equality or tolerance. More than six decimal places remains unproved.
fn same_factor(actual: Fraction, expected: f64) -> bool {
    let Ok(scaled) = RateBps::try_from(crate::speed::Rate {
        down: expected,
        up: expected,
    }) else {
        return false;
    };
    u128::from(actual.numerator) * 1_000_000
        == u128::from(scaled.down) * u128::from(actual.denominator)
}
fn constant(priority: &LegacyPriority) -> bool {
    !priority.sources.is_empty()
        && priority
            .sources
            .iter()
            .all(|s| *s == LegacyRateSource::Settings)
}

/// Check dormant policy declarations as well as current rows. Snapshot parity is still required.
pub fn legacy_issues(
    cfg: &Config,
    registry: &RegistryDocument,
) -> anyhow::Result<Vec<PolicyIssue>> {
    let bindings = selected(cfg, registry)?;
    let imported_bounds = super::legacy_bounds::import(&cfg.bandwidth_defaults).ok();
    let mut out = source_checks(cfg, registry, &bindings);
    let catalog = registry
        .catalog
        .as_ref()
        .context("topology catalog is unconfigured")?;
    let profiles: HashMap<_, _> = catalog
        .profiles
        .iter()
        .map(|p| (p.id.as_str(), p))
        .collect();
    let mut instances: HashMap<&str, Vec<_>> = HashMap::new();
    for node in &catalog.nodes {
        if let Some(source) = &node.source_id {
            instances.entry(source).or_default().push(node);
        }
    }
    let selected_ids: HashSet<_> = catalog
        .sources
        .iter()
        .filter(|s| !s.targets.is_empty())
        .map(|s| s.id.as_str())
        .chain(instances.keys().copied())
        .collect();
    let mut by_scope: HashMap<&SourceScope, Vec<_>> = HashMap::new();
    for source in &catalog.sources {
        if selected_ids.contains(source.id.as_str())
            && let SourceOrigin::Subscribers { scope, .. } = &source.origin
        {
            by_scope.entry(scope).or_default().push(source);
        }
    }
    let mut assigned_profiles: HashMap<&SourceScope, BTreeSet<&str>> = HashMap::new();
    for subscriber in &registry.subscribers {
        if let Some(profile) = subscriber.policy.profile_id.as_deref() {
            for identity in &subscriber.identities {
                assigned_profiles
                    .entry(&identity.scope)
                    .or_default()
                    .insert(profile);
            }
        }
    }
    for (selection, router) in bindings {
        let rules = expected(cfg, router, selection)?;
        let expected = rules.first().context("legacy source policy is absent")?;
        if rules.iter().any(|r| {
            r.priority != expected.priority
                || r.factors != expected.factors
                || r.count != expected.count
        }) {
            issue(
                &mut out,
                selection,
                "routers.dhcp.servers",
                PolicyIssueKind::DhcpServicePolicy,
            );
        }
        if selection.policy.legacy_priority.as_ref() != Some(&expected.priority) {
            issue(
                &mut out,
                selection,
                "source_settings.policy.legacy_priority",
                PolicyIssueKind::SourcePriority,
            );
        }
        let mut profile_ids = assigned_profiles
            .get(&selection.scope)
            .cloned()
            .unwrap_or_default();
        for source in by_scope.get(&selection.scope).into_iter().flatten() {
            let SourceOrigin::Subscribers { group, .. } = &source.origin else {
                unreachable!()
            };
            if !same_factor(group.factors.down, expected.factors.0)
                || !same_factor(group.factors.up, expected.factors.1)
            {
                issue(
                    &mut out,
                    selection,
                    &source.id,
                    PolicyIssueKind::GroupFactors,
                );
            }
            let legacy_cap = expected.cap.limit().map(RateBps::try_from).transpose()?;
            let manager_cap = group
                .backstop
                .map(|c| c.base.apply_factors(c.factor, c.factor))
                .transpose()?;
            if legacy_cap != manager_cap {
                issue(
                    &mut out,
                    selection,
                    &source.id,
                    PolicyIssueKind::GroupBackstop,
                );
            }
            // A capacity floor does not preserve a native minimum commitment.
            if group.floor.is_some()
                || expected.node.download_min_mbps.is_some()
                || expected.node.upload_min_mbps.is_some()
            {
                issue(
                    &mut out,
                    selection,
                    &source.id,
                    PolicyIssueKind::GroupMinimum,
                );
            }
            let formula_ok = match group.formula {
                GroupFormula::PlanSum => !expected.count,
                GroupFormula::CountTimesUnit(unit) => {
                    expected.count
                        && constant(&expected.priority)
                        && unit == expected.priority.settings
                }
            };
            if !formula_ok {
                issue(
                    &mut out,
                    selection,
                    &source.id,
                    PolicyIssueKind::GroupFormula,
                );
            }
            for node in instances.get(source.id.as_str()).into_iter().flatten() {
                match node.capacity {
                    CapacityPolicy::Automatic { factors } => {
                        if let FactorChoice::Override(f) = factors
                            && (!same_factor(f.down, expected.factors.0)
                                || !same_factor(f.up, expected.factors.1))
                        {
                            issue(&mut out, selection, &node.id, PolicyIssueKind::GroupFactors);
                        }
                    }
                    _ => issue(
                        &mut out,
                        selection,
                        &node.id,
                        PolicyIssueKind::GroupCapacity,
                    ),
                }
            }
            if let Some(profile) = source.profile_id.as_deref() {
                profile_ids.insert(profile);
            } else {
                issue(
                    &mut out,
                    selection,
                    &source.id,
                    PolicyIssueKind::BaseAuthority,
                );
            }
        }
        for id in profile_ids {
            let profile = profiles
                .get(id)
                .context("selected subscriber profile is missing")?;
            match profile.base {
                BaseRateAuthority::SourceRead => {
                    if imported_bounds.as_ref().is_none_or(|expected| {
                        !super::legacy_bounds::matches(&catalog.defaults, &profile.speed, expected)
                    }) {
                        issue(
                            &mut out,
                            selection,
                            id,
                            PolicyIssueKind::DynamicSubscriberBounds,
                        );
                    }
                }
                BaseRateAuthority::Local(base) => {
                    if !constant(&expected.priority) || base != expected.priority.settings {
                        issue(&mut out, selection, id, PolicyIssueKind::BaseAuthority);
                    }
                    let legacy = crate::speed::circuit_rates(
                        base.try_into()?,
                        cfg.bandwidth_defaults.min_rate_percentage,
                        cfg.bandwidth_defaults.safe_min_mbps,
                        cfg.bandwidth_defaults.max_floor_mbps,
                    );
                    let min = RateBps::try_from(crate::speed::Rate {
                        down: legacy.down_min,
                        up: legacy.up_min,
                    })?;
                    let max = RateBps::try_from(crate::speed::Rate {
                        down: legacy.down_max,
                        up: legacy.up_max,
                    })?;
                    let actual = resolve_subscriber(base, &catalog.defaults, &profile.speed)?;
                    if actual.min != min || actual.max != max {
                        issue(&mut out, selection, id, PolicyIssueKind::SubscriberBounds);
                    }
                }
            }
        }
    }
    out.sort();
    out.dedup();
    Ok(out)
}
#[cfg(test)]
#[path = "preflight_tests.rs"]
mod tests;
