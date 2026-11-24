use super::*;
use crate::topology_manager::catalog::{self, FactorChoice, Factors};
use crate::topology_manager::model::{IdentityRef, SourceKind, SourceScope};
use crate::topology_manager::observation::{
    AccountObservation, Endpoint, ObservationGate, ObservedPolicy, ProfileObservation, SourceRead,
    reconcile,
};
use crate::topology_manager::rates::{Fraction, RateRule, RuleValue};

fn rate(down: u64, up: u64) -> RateBps {
    RateBps { down, up }
}
fn frac(numerator: u64, denominator: u64) -> Fraction {
    Fraction {
        numerator,
        denominator,
    }
}
fn scope() -> SourceScope {
    SourceScope {
        router_id: "distro".into(),
        source_id: "ppp-service".into(),
        kind: SourceKind::Ppp,
    }
}
fn registry() -> RegistryDocument {
    RegistryDocument {
        catalog: Some(catalog::tests::catalog()),
        ..Default::default()
    }
}
fn observed(number: u32) -> AccountObservation {
    AccountObservation {
        account_ref: format!("account-{number:05}"),
        policy: ObservedPolicy {
            profile: ProfileObservation::Known(Some("provider-profile-id".into())),
            plan: Some(rate(50_000_000, 10_000_000)),
        },
        endpoints: vec![Endpoint {
            session_id: format!("session-{number}"),
            ips: vec![std::net::IpAddr::V4((0x0a000001 + number).into())],
            mac: None,
        }],
    }
}
fn collect(registry: &RegistryDocument, accounts: Vec<AccountObservation>) -> Reconciliation {
    let snapshot = ObservationGate::default()
        .accept(
            1,
            &[scope()],
            vec![SourceRead::Complete {
                scope: scope(),
                accounts,
            }],
        )
        .unwrap();
    reconcile(registry, snapshot).unwrap()
}
fn run(registry: &RegistryDocument, count: u32) -> PlacementResolution {
    resolve(collect(registry, (0..count).map(observed).collect())).unwrap()
}
fn count(result: &PlacementResolution, target: &str) -> usize {
    result
        .placements
        .iter()
        .filter(|p| p.target_id.as_deref() == Some(target))
        .count()
}

#[test]
fn a_single_profile_balances_plan_demand_across_wans_without_changing_maxima() {
    let result = run(&registry(), 60);
    assert!(result.conflicts.is_empty(), "{:?}", result.conflicts);
    assert_eq!(
        (count(&result, "group1"), count(&result, "group2")),
        (20, 40)
    );
    assert!(
        result
            .placements
            .iter()
            .all(|p| p.rates.max == rate(50_000_000, 10_000_000))
    );
    assert!(result.candidate.is_some());
}

#[test]
fn wan_only_destinations_support_the_same_shared_pool() {
    let mut r = registry();
    let c = r.catalog.as_mut().unwrap();
    c.nodes.truncate(2);
    c.sources[2].targets = vec!["wan1".into(), "wan2".into()];
    let result = run(&r, 60);
    assert_eq!((count(&result, "wan1"), count(&result, "wan2")), (20, 40));
}

#[test]
fn profile_only_groups_without_independent_capacity_receive_equal_plan_demand() {
    let mut r = registry();
    let c = r.catalog.as_mut().unwrap();
    c.nodes.drain(..2);
    for n in &mut c.nodes {
        n.parent = None;
    }
    // Different source definitions permit two explicitly selected root instances.
    let mut source = c.sources[2].clone();
    source.id = "second-source".into();
    if let SourceOrigin::Subscribers { profile_ref, .. } = &mut source.origin {
        *profile_ref = None;
    }
    source.targets = vec!["group2".into()];
    c.sources[2].targets = vec!["group1".into()];
    c.sources.push(source);
    c.nodes[1].source_id = Some("second-source".into());
    let result = run(&r, 20);
    assert_eq!(
        (count(&result, "group1"), count(&result, "group2")),
        (10, 10)
    );
}

#[test]
fn upload_and_wan_factors_affect_allocation() {
    let mut r = registry();
    r.catalog.as_mut().unwrap().profiles[0].base =
        BaseRateAuthority::Local(rate(50_000_000, 20_000_000));
    r.catalog.as_mut().unwrap().nodes[1].capacity = CapacityPolicy::Fixed {
        base: rate(620_000_000, 100_000_000),
        factors: FactorChoice::Inherit,
    };
    let equal_upload = run(&r, 60);
    assert_eq!(
        (
            count(&equal_upload, "group1"),
            count(&equal_upload, "group2")
        ),
        (30, 30)
    );
    r.catalog.as_mut().unwrap().nodes[1].capacity = CapacityPolicy::Fixed {
        base: rate(620_000_000, 200_000_000),
        factors: FactorChoice::Override(Factors {
            down: frac(1, 2),
            up: frac(1, 2),
        }),
    };
    let factored = run(&r, 60);
    assert_eq!(
        (count(&factored, "group1"), count(&factored, "group2")),
        (30, 30)
    );
}

#[test]
fn valid_saved_assignments_survive_capacity_edits_and_reply_order_changes() {
    let first = run(&registry(), 30);
    let mut r = first.candidate.unwrap();
    r.catalog.as_mut().unwrap().nodes[1].capacity = CapacityPolicy::Fixed {
        base: rate(50_000_000, 10_000_000),
        factors: FactorChoice::Inherit,
    };
    let second = resolve(collect(&r, (0..30).rev().map(observed).collect())).unwrap();
    assert_eq!(first.placements, second.placements);
    assert_eq!(second.candidate.unwrap(), r);
}

#[test]
fn pins_are_counted_before_new_automatic_assignments() {
    let mut r = collect(&registry(), vec![observed(0)]).registry;
    r.subscribers[0].policy.pin = Some("group1".into());
    let result = run(&r, 3);
    assert_eq!(result.placements[0].target_id.as_deref(), Some("group1"));
    assert_eq!((count(&result, "group1"), count(&result, "group2")), (1, 2));
}

#[test]
fn current_membership_cannot_be_bypassed_by_a_saved_pin() {
    let mut r = run(&registry(), 1).candidate.unwrap();
    let p = &mut r.subscribers[0].policy;
    p.pin = p.assignment.clone();
    let before = r.clone();
    let mut a = observed(0);
    a.policy.profile = ProfileObservation::Known(Some("different-profile".into()));
    let result = resolve(collect(&r, vec![a])).unwrap();
    assert!(result.candidate.is_none());
    assert!(matches!(
        &result.conflicts[0].reason,
        PlacementConflictReason::InvalidPin(_)
    ));
    assert_eq!(r, before);
}

#[test]
fn unread_profile_evidence_differs_from_verified_no_profile() {
    let mut a = observed(0);
    a.policy.profile = ProfileObservation::Unread;
    let result = resolve(collect(&registry(), vec![a])).unwrap();
    assert_eq!(
        result.conflicts[0].reason,
        PlacementConflictReason::MissingProfileEvidence
    );
    assert!(result.candidate.is_none());
    // A verified "no profile" matches no placed source: automatic placement takes it (R5).
    let mut a = observed(0);
    a.policy.profile = ProfileObservation::Known(None);
    let result = resolve(collect(&registry(), vec![a])).unwrap();
    assert!(result.conflicts.is_empty(), "{:?}", result.conflicts);
    assert!(result.candidate.is_some());
}

#[test]
fn source_read_rates_require_current_evidence_and_local_authority_ignores_source_changes() {
    let mut a = observed(0);
    a.policy.plan = None;
    assert_eq!(
        resolve(collect(&registry(), vec![a])).unwrap().placements[0].base,
        rate(50_000_000, 10_000_000)
    );
    let mut r = registry();
    r.catalog.as_mut().unwrap().profiles[0].base = BaseRateAuthority::SourceRead;
    let mut a = observed(0);
    a.policy.plan = None;
    let result = resolve(collect(&r, vec![a])).unwrap();
    assert_eq!(
        result.conflicts[0].reason,
        PlacementConflictReason::MissingPlan
    );
    let mut a = observed(0);
    a.policy.plan = Some(rate(20_000_000, 5_000_000));
    let result = resolve(collect(&r, vec![a])).unwrap();
    assert_eq!(result.placements[0].base, rate(20_000_000, 5_000_000));
}

#[test]
fn per_control_minimum_and_maximum_policies_replace_defaults_once() {
    let mut r = registry();
    let p = &mut r.catalog.as_mut().unwrap().profiles[0];
    p.speed.down_min = RuleValue::Override(RateRule::BaseFraction(frac(1, 100)));
    p.speed.down_max = RuleValue::Override(RateRule::FixedBps(80_000_000));
    let result = run(&r, 1);
    assert_eq!(result.placements[0].rates.min.down, 500_000);
    assert_eq!(result.placements[0].rates.max.down, 80_000_000);
    assert_eq!(result.placements[0].rates.max.up, 10_000_000);
}

#[test]
fn profile_restrictions_narrow_the_shared_pool() {
    let mut r = registry();
    r.catalog.as_mut().unwrap().profiles[0].eligible_targets = Some(vec!["group1".into()]);
    let result = run(&r, 10);
    assert_eq!(count(&result, "group1"), 10);
}

#[test]
fn one_circuit_with_multiple_current_endpoints_counts_once() {
    let mut a = observed(0);
    a.endpoints.extend(observed(10).endpoints);
    let result = resolve(collect(&registry(), vec![a, observed(1), observed(2)])).unwrap();
    assert_eq!(result.placements.len(), 3);
    assert_eq!(result.active[0].endpoints.len(), 2);
    assert_eq!((count(&result, "group1"), count(&result, "group2")), (1, 2));
}

#[test]
fn absence_keeps_intent_but_removes_active_placement_and_return_restores_it() {
    let first = run(&registry(), 1);
    let r = first.candidate.unwrap();
    let empty = run(&r, 0);
    assert!(empty.placements.is_empty());
    assert!(empty.active.is_empty());
    assert_eq!(empty.candidate.unwrap(), r);
    let mut a = observed(0);
    a.endpoints = observed(99).endpoints;
    let returned = resolve(collect(&r, vec![a])).unwrap();
    assert_eq!(returned.placements, first.placements);
    assert_eq!(
        returned.active[0].endpoints[0].endpoint.session_id,
        "session-99"
    );
}

#[test]
fn synthetic_ten_thousand_pool_is_deterministic() {
    let r = registry();
    let forward = run(&r, 10_000);
    let reverse = resolve(collect(&r, (0..10_000).rev().map(observed).collect())).unwrap();
    assert!(forward.conflicts.is_empty());
    assert_eq!(forward.placements, reverse.placements);
    assert_eq!(forward.candidate, reverse.candidate);
    assert_eq!(count(&forward, "group1"), 3333);
}

#[test]
fn equivalent_directional_ratios_have_consistent_equality() {
    assert_eq!(Ratio(1, 2).cmp(&Ratio(2, 4)), Ordering::Equal);
    assert_eq!(Ratio(1, 2), Ratio(2, 4));
}

#[test]
fn explicit_source_instances_remain_eligible_without_a_shared_target_rule() {
    let mut r = registry();
    r.catalog.as_mut().unwrap().sources[2].targets.clear();
    let result = run(&r, 6);
    assert!(result.conflicts.is_empty());
    assert_eq!((count(&result, "group1"), count(&result, "group2")), (2, 4));
}

#[test]
fn wan_percentages_use_the_configured_base_and_allocation_respects_child_caps() {
    let mut r = registry();
    let c = r.catalog.as_mut().unwrap();
    c.nodes[0].capacity = CapacityPolicy::Fixed {
        base: rate(310_000_000, 100_000_000),
        factors: FactorChoice::Override(Factors {
            down: frac(1, 2),
            up: frac(1, 2),
        }),
    };
    c.nodes[2].capacity = CapacityPolicy::WanPercentage {
        down: frac(1, 2),
        up: frac(1, 2),
    };
    c.nodes[3].capacity = CapacityPolicy::Fixed {
        base: rate(310_000_000, 100_000_000),
        factors: FactorChoice::Override(Factors::default()),
    };
    let result = run(&r, 60);
    assert_eq!(
        (count(&result, "group1"), count(&result, "group2")),
        (20, 40)
    );
}

#[test]
fn source_backed_ancestor_restricts_a_broad_explicit_pool() {
    let mut r = registry();
    let c = r.catalog.as_mut().unwrap();
    let mut restrictive = c.sources[2].clone();
    restrictive.id = "restrictive".into();
    restrictive.targets.clear();
    if let SourceOrigin::Subscribers { profile_ref, .. } = &mut restrictive.origin {
        *profile_ref = Some("other-profile".into());
    }
    c.sources.push(restrictive);
    let mut parent = c.nodes[2].clone();
    parent.id = "restricted-parent".into();
    parent.kind = NodeKind::Intermediate;
    parent.source_id = Some("restrictive".into());
    parent.capacity = CapacityPolicy::Inherit;
    c.nodes[2].parent = Some(parent.id.clone());
    c.nodes.push(parent);
    let result = run(&r, 8);
    assert_eq!(count(&result, "group2"), 8);
    assert_eq!(count(&result, "group1"), 0);
}

#[test]
fn conflicting_active_alias_plans_block_a_single_circuit() {
    let mut r = collect(&registry(), vec![observed(0), observed(1)]).registry;
    let second = r.subscribers.pop().unwrap();
    r.subscribers[0].identities.extend(second.identities);
    r.catalog.as_mut().unwrap().profiles[0].base = BaseRateAuthority::SourceRead;
    let mut a = observed(1);
    a.policy.plan = Some(rate(30_000_000, 5_000_000));
    let result = resolve(collect(&r, vec![observed(0), a])).unwrap();
    assert_eq!(result.active.len(), 1);
    assert_eq!(
        result.conflicts[0].reason,
        PlacementConflictReason::ConflictingPlans
    );
    assert!(result.candidate.is_none());
    let consistent = resolve(collect(&r, vec![observed(0), observed(1)])).unwrap();
    assert_eq!(consistent.placements.len(), 1);
    assert_eq!(consistent.active[0].accounts.len(), 2);
}

#[test]
fn conflicting_source_defaults_need_an_explicit_profile_policy() {
    let mut r = registry();
    let c = r.catalog.as_mut().unwrap();
    let mut profile = c.profiles[0].clone();
    profile.id = "other-local-plan".into();
    c.profiles.push(profile);
    let mut source = c.sources[2].clone();
    source.id = "wildcard".into();
    source.profile_id = Some("other-local-plan".into());
    if let SourceOrigin::Subscribers { profile_ref, .. } = &mut source.origin {
        *profile_ref = None;
    }
    c.sources.push(source);
    let result = run(&r, 1);
    assert_eq!(
        result.conflicts[0].reason,
        PlacementConflictReason::ConflictingProfiles
    );
    let mut r = collect(&r, vec![observed(0)]).registry;
    r.subscribers[0].policy.profile_id = Some("local-plan".into());
    assert!(run(&r, 1).conflicts.is_empty());
}

#[test]
fn an_ineligible_saved_assignment_is_dropped_and_placed_again() {
    let r = run(&registry(), 1).candidate.unwrap();
    let mut a = observed(0);
    a.policy.profile = ProfileObservation::Known(None);
    let result = resolve(collect(&r, vec![a])).unwrap();
    // Owner decision (2c-2): the ineligible group assignment is dropped. The subscriber is
    // placed again on a root (R5). Intent in the old registry stays intact.
    assert_eq!(targets(&result).len(), 1);
    assert!(set(&["wan1", "wan2"]).is_superset(&targets(&result)));
    r.subscribers[0].policy.assignment.as_ref().unwrap();
}

#[test]
fn demand_overflow_fails_without_a_partial_candidate() {
    let mut r = registry();
    let c = r.catalog.as_mut().unwrap();
    c.profiles[0].base = BaseRateAuthority::Local(rate(u64::MAX, u64::MAX));
    c.profiles[0].eligible_targets = Some(vec!["group1".into()]);
    assert!(resolve(collect(&r, vec![observed(0), observed(1)])).is_err());
}

#[test]
fn different_plan_sizes_use_plan_demand_instead_of_subscriber_count() {
    let mut r = registry();
    r.catalog.as_mut().unwrap().profiles[0].base = BaseRateAuthority::SourceRead;
    let accounts = (0..30)
        .map(|i| {
            let mut a = observed(i);
            a.policy.plan = Some(if i < 10 {
                rate(100_000_000, 20_000_000)
            } else {
                rate(10_000_000, 2_000_000)
            });
            a
        })
        .collect();
    let result = resolve(collect(&r, accounts)).unwrap();
    let demand: u64 = result
        .placements
        .iter()
        .filter(|p| p.target_id.as_deref() == Some("group1"))
        .map(|p| p.base.down)
        .sum();
    assert_eq!(demand, 400_000_000);
    assert_ne!(count(&result, "group1"), 10);
}

#[test]
fn new_unselected_wan_does_not_expand_an_existing_pool() {
    let mut r = registry();
    let c = r.catalog.as_mut().unwrap();
    let mut source = c.sources[0].clone();
    source.id = "new-wan-source".into();
    if let SourceOrigin::Wan { endpoint_ref, .. } = &mut source.origin {
        *endpoint_ref = "new-endpoint".into();
    }
    c.sources.push(source);
    let mut wan = c.nodes[0].clone();
    wan.id = "new-wan".into();
    wan.source_id = Some("new-wan-source".into());
    c.nodes.push(wan);
    let result = run(&r, 60);
    assert_eq!(count(&result, "new-wan"), 0);
    assert_eq!(
        (count(&result, "group1"), count(&result, "group2")),
        (20, 40)
    );
}

#[test]
fn source_read_policy_error_is_a_typed_conflict() {
    let mut r = registry();
    r.catalog.as_mut().unwrap().profiles[0].base = BaseRateAuthority::SourceRead;
    let mut a = observed(0);
    a.policy.plan = Some(rate(500_000, 500_000));
    let result = resolve(collect(&r, vec![a])).unwrap();
    assert!(matches!(
        result.conflicts[0].reason,
        PlacementConflictReason::InvalidRates(_)
    ));
    assert!(result.candidate.is_none());
}

#[test]
fn checked_placements_persist_across_restart_without_current_observations() {
    use crate::topology_manager::store::RegistryStore;
    let dir = tempfile::tempdir().unwrap();
    let mut store = RegistryStore::open(dir.path()).unwrap();
    store.commit(0, registry()).unwrap();
    let result = run(store.document().unwrap(), 6);
    store.commit(1, result.candidate.unwrap()).unwrap();
    let bytes = std::fs::read_to_string(dir.path().join("topology-manager/registry.json")).unwrap();
    assert!(!bytes.contains("session-"));
    assert!(!bytes.contains("10.0.0."));
    drop(store);
    let store = RegistryStore::open(dir.path()).unwrap();
    let mut fresh = (0..6).map(observed).collect::<Vec<_>>();
    fresh[0].endpoints = observed(99).endpoints;
    let returned = resolve(collect(store.document().unwrap(), fresh)).unwrap();
    assert_eq!(returned.placements, result.placements);
    assert_eq!(
        returned.active[0].endpoints[0].endpoint.session_id,
        "session-99"
    );
}

#[test]
fn an_explicit_target_set_restricts_other_instances_of_the_same_source() {
    let mut r = registry();
    r.catalog.as_mut().unwrap().sources[2].targets = vec!["group2".into()];
    let result = run(&r, 30);
    assert_eq!(count(&result, "group2"), 30);
    assert_eq!(count(&result, "group1"), 0);
}

#[test]
fn an_ancestor_target_restriction_cannot_be_bypassed_by_a_broader_source() {
    let mut r = registry();
    let c = r.catalog.as_mut().unwrap();
    let mut broad = c.sources[2].clone();
    broad.id = "broad".into();
    if let SourceOrigin::Subscribers { profile_ref, .. } = &mut broad.origin {
        *profile_ref = None;
    }
    c.sources.push(broad);
    let mut parent = c.nodes[2].clone();
    parent.id = "ancestor".into();
    parent.kind = NodeKind::Intermediate;
    parent.capacity = CapacityPolicy::Inherit;
    c.nodes[2].parent = Some(parent.id.clone());
    c.nodes[2].source_id = Some("broad".into());
    c.nodes.push(parent);
    c.sources[2].targets = vec!["group2".into()];
    let result = run(&r, 6);
    assert_eq!(count(&result, "group1"), 0);
    assert_eq!(count(&result, "group2"), 6);
}

#[test]
fn missing_rate_authority_does_not_implicitly_select_source_read() {
    let mut r = registry();
    r.catalog.as_mut().unwrap().sources[2].profile_id = None;
    let result = run(&r, 1);
    assert!(result.candidate.is_none());
    assert!(result.placements.is_empty());
    assert_eq!(
        result.conflicts[0].reason,
        PlacementConflictReason::MissingRateAuthority
    );
}

fn fallback_registry(two: bool) -> RegistryDocument {
    let mut c = catalog::tests::with_fallback_node();
    let mut ids = vec!["fb".to_string()];
    if two {
        let mut n = c.nodes.iter().find(|n| n.id == "fb").unwrap().clone();
        n.id = "fb2".into();
        n.parent = Some("wan2".into());
        c.nodes.push(n);
        ids.push("fb2".into());
    }
    c.fallback = Some(catalog::FallbackPolicy {
        node_ids: ids,
        default_plan: rate(10_000_000, 10_000_000),
    });
    RegistryDocument {
        catalog: Some(c),
        ..Default::default()
    }
}

fn unbound(number: u32, plan: Option<RateBps>) -> AccountObservation {
    let mut a = observed(number);
    a.policy.profile = ProfileObservation::Known(Some("unbound:new-plan".into()));
    a.policy.plan = plan;
    a
}

#[test]
fn an_unmatched_subscriber_goes_to_a_fallback_node_with_the_router_plan() {
    let r = fallback_registry(false);
    let result = resolve(collect(
        &r,
        vec![unbound(0, Some(rate(20_000_000, 5_000_000)))],
    ))
    .unwrap();
    assert!(result.conflicts.is_empty(), "{:?}", result.conflicts);
    assert_eq!(result.placements[0].target_id.as_deref(), Some("fb"));
    assert_eq!(result.placements[0].base, rate(20_000_000, 5_000_000));
}

#[test]
fn without_a_router_plan_the_default_plan_applies() {
    let r = fallback_registry(false);
    let result = resolve(collect(&r, vec![unbound(0, None)])).unwrap();
    assert_eq!(result.placements[0].base, rate(10_000_000, 10_000_000));
}

#[test]
fn an_attached_plan_wins_in_the_fallback() {
    let r = fallback_registry(false);
    let mut r = collect(&r, vec![unbound(0, None)]).registry;
    r.subscribers[0].policy.profile_id = Some("local-plan".into());
    let result = resolve(collect(&r, vec![unbound(0, None)])).unwrap();
    assert!(result.conflicts.is_empty(), "{:?}", result.conflicts);
    assert_eq!(result.placements[0].base, rate(50_000_000, 10_000_000));
}

#[test]
fn a_moved_customer_is_placed_again_instead_of_invalid_assignment() {
    let r = fallback_registry(false);
    let r = run(&r, 1).candidate.unwrap();
    assert!(r.subscribers[0].policy.assignment.as_deref() != Some("fb"));
    let result = resolve(collect(
        &r,
        vec![unbound(0, Some(rate(20_000_000, 5_000_000)))],
    ))
    .unwrap();
    assert!(result.conflicts.is_empty(), "{:?}", result.conflicts);
    assert_eq!(result.placements[0].target_id.as_deref(), Some("fb"));
}

#[test]
fn an_ineligible_pin_still_conflicts() {
    let r = fallback_registry(false);
    let mut r = run(&r, 1).candidate.unwrap();
    let target = r.subscribers[0].policy.assignment.clone();
    r.subscribers[0].policy.pin = target;
    let result = resolve(collect(&r, vec![unbound(0, None)])).unwrap();
    assert!(matches!(
        result.conflicts[0].reason,
        PlacementConflictReason::InvalidPin(_)
    ));
}

#[test]
fn the_fallback_balances_across_its_instances() {
    let r = fallback_registry(true);
    let result = resolve(collect(
        &r,
        (0..9)
            .map(|i| unbound(i, Some(rate(50_000_000, 10_000_000))))
            .collect(),
    ))
    .unwrap();
    assert!(result.conflicts.is_empty(), "{:?}", result.conflicts);
    assert_eq!((count(&result, "fb"), count(&result, "fb2")), (3, 6));
}

// Automatic placement (R1–R6). `unbound` accounts match no placed source of router `distro`.

fn add(
    r: &mut RegistryDocument,
    id: &str,
    parent: Option<&str>,
    kind: NodeKind,
    router: Option<&str>,
) {
    let capacity = match kind {
        NodeKind::Intermediate => catalog::CapacityPolicy::Inherit,
        _ => catalog::CapacityPolicy::Automatic {
            factors: FactorChoice::Inherit,
        },
    };
    r.catalog
        .as_mut()
        .unwrap()
        .nodes
        .push(catalog::CatalogNode {
            address_list: None,
            tree_excluded: false,
            router_id: router.map(Into::into),
            geo: None,
            minimum: Default::default(),
            id: id.into(),
            name: id.into(),
            parent: parent.map(Into::into),
            source_id: None,
            kind,
            capacity,
        });
}

fn auto_run(r: &RegistryDocument, count: u32) -> PlacementResolution {
    let accounts = (0..count)
        .map(|i| unbound(i, Some(rate(50_000_000, 10_000_000))))
        .collect();
    resolve(collect(r, accounts)).unwrap()
}

fn targets(result: &PlacementResolution) -> BTreeSet<String> {
    assert!(result.conflicts.is_empty(), "{:?}", result.conflicts);
    result
        .placements
        .iter()
        .map(|p| p.target_id.clone().unwrap_or_default())
        .collect()
}

fn set(ids: &[&str]) -> BTreeSet<String> {
    ids.iter().map(|s| s.to_string()).collect()
}

/// Router node `towerA` of `distro` below `wan1`, with `t1` and `t2` below it.
fn towered() -> RegistryDocument {
    let mut r = registry();
    add(
        &mut r,
        "towerA",
        Some("wan1"),
        NodeKind::Intermediate,
        Some("distro"),
    );
    add(
        &mut r,
        "t1",
        Some("towerA"),
        NodeKind::SubscriberGroup,
        None,
    );
    add(
        &mut r,
        "t2",
        Some("towerA"),
        NodeKind::SubscriberGroup,
        None,
    );
    r
}

#[test]
fn unplaced_source_with_router_node_goes_to_its_children() {
    assert_eq!(targets(&auto_run(&towered(), 6)), set(&["t1", "t2"]));
}

#[test]
fn router_node_without_children_takes_its_subscribers() {
    let mut r = registry();
    add(
        &mut r,
        "towerA",
        Some("wan1"),
        NodeKind::Intermediate,
        Some("distro"),
    );
    assert_eq!(targets(&auto_run(&r, 3)), set(&["towerA"]));
}

#[test]
fn router_without_node_goes_to_its_own_wan() {
    let mut r = registry();
    let c = r.catalog.as_mut().unwrap();
    let mut source = c.sources[0].clone();
    source.id = "wan-source-3".into();
    source.origin = catalog::SourceOrigin::Wan {
        router_id: "distro".into(),
        endpoint_ref: "endpoint-3".into(),
    };
    c.sources.push(source);
    c.routers[1].roles.push(catalog::RouterRole::Wan);
    let mut wan = c.nodes[0].clone();
    wan.id = "wan3".into();
    wan.source_id = Some("wan-source-3".into());
    c.nodes.push(wan);
    assert_eq!(targets(&auto_run(&r, 3)), set(&["wan3"]));
}

#[test]
fn router_without_node_or_wan_goes_to_the_roots() {
    assert_eq!(targets(&auto_run(&registry(), 9)), set(&["wan1", "wan2"]));
}

#[test]
fn nested_router_node_is_not_eligible_for_the_parent_router() {
    let mut r = registry();
    add(
        &mut r,
        "towerA",
        Some("wan1"),
        NodeKind::Intermediate,
        Some("distro"),
    );
    add(
        &mut r,
        "t1",
        Some("towerA"),
        NodeKind::SubscriberGroup,
        None,
    );
    add(
        &mut r,
        "towerB",
        Some("towerA"),
        NodeKind::Intermediate,
        Some("core"),
    );
    add(
        &mut r,
        "apB",
        Some("towerB"),
        NodeKind::SubscriberGroup,
        None,
    );
    assert_eq!(targets(&auto_run(&r, 6)), set(&["t1"]));
}

#[test]
fn adding_a_router_node_moves_its_subscribers_once() {
    let first = auto_run(&registry(), 4);
    assert_eq!(targets(&first), set(&["wan1", "wan2"]));
    let mut r = first.candidate.unwrap();
    add(
        &mut r,
        "towerA",
        Some("wan1"),
        NodeKind::Intermediate,
        Some("distro"),
    );
    add(
        &mut r,
        "t1",
        Some("towerA"),
        NodeKind::SubscriberGroup,
        None,
    );
    let second = auto_run(&r, 4);
    assert_eq!(targets(&second), set(&["t1"]));
    let third = auto_run(&second.candidate.unwrap(), 4);
    assert_eq!(third.placements, second.placements);
}

#[test]
fn saved_assignment_stays_when_a_new_child_appears() {
    let mut r = registry();
    add(
        &mut r,
        "towerA",
        Some("wan1"),
        NodeKind::Intermediate,
        Some("distro"),
    );
    add(
        &mut r,
        "t1",
        Some("towerA"),
        NodeKind::SubscriberGroup,
        None,
    );
    let mut r = auto_run(&r, 4).candidate.unwrap();
    add(
        &mut r,
        "t2",
        Some("towerA"),
        NodeKind::SubscriberGroup,
        None,
    );
    let result = auto_run(&r, 8);
    let old: Vec<_> = result
        .placements
        .iter()
        .filter(|p| r.subscribers.iter().any(|s| s.id == p.subscriber_id))
        .collect();
    assert_eq!(old.len(), 4);
    assert!(old.iter().all(|p| p.target_id.as_deref() == Some("t1")));
    assert_eq!(count(&result, "t2"), 4);
}

#[test]
fn accounts_on_two_routers_use_the_intersection() {
    let edge = SourceScope {
        router_id: "edge".into(),
        ..scope()
    };
    let mut r = towered();
    let c = r.catalog.as_mut().unwrap();
    c.routers.push(catalog::CatalogRouter {
        id: "edge".into(),
        name: "Edge".into(),
        roles: vec![catalog::RouterRole::Ppp],
        ..Default::default()
    });
    let mut source = c.sources[2].clone();
    source.id = "edge-source".into();
    source.targets.clear();
    if let catalog::SourceOrigin::Subscribers {
        scope, profile_ref, ..
    } = &mut source.origin
    {
        *scope = edge.clone();
        *profile_ref = Some("edge-profile".into());
    }
    c.sources.push(source);
    // A placed `edge` source, so the scope is read. The account matches no source of it.
    let mut group = c.nodes.iter().find(|n| n.id == "group2").unwrap().clone();
    group.id = "edge-group".into();
    group.source_id = Some("edge-source".into());
    c.nodes.push(group);
    r.subscribers.push(SubscriberRecord {
        id: "both".into(),
        identities: vec![
            IdentityRef {
                scope: scope(),
                account_ref: "account-00000".into(),
            },
            IdentityRef {
                scope: edge.clone(),
                account_ref: "account-00000".into(),
            },
        ],
        policy: Default::default(),
    });
    let account = || unbound(0, Some(rate(50_000_000, 10_000_000)));
    let mut other = account();
    other.endpoints[0].ips = vec!["192.0.2.200".parse().unwrap()];
    let snapshot = ObservationGate::default()
        .accept(
            1,
            &[scope(), edge.clone()],
            vec![
                SourceRead::Complete {
                    scope: scope(),
                    accounts: vec![account()],
                },
                SourceRead::Complete {
                    scope: edge,
                    accounts: vec![other],
                },
            ],
        )
        .unwrap();
    // `distro` gives {t1, t2}; `edge` has no router node or WAN and gives the roots.
    let result = resolve(reconcile(&r, snapshot).unwrap()).unwrap();
    assert!(matches!(
        result.conflicts[0].reason,
        PlacementConflictReason::NoEligibleTarget
    ));
}

#[test]
fn explicit_fallback_keeps_priority_and_obeys_the_family_rule() {
    let mut r = fallback_registry(false);
    add(
        &mut r,
        "towerA",
        Some("wan2"),
        NodeKind::Intermediate,
        Some("distro"),
    );
    assert_eq!(targets(&auto_run(&r, 2)), set(&["fb"]));
    // The fallback node moves into the branch of `core`. `distro` subscribers use their router node.
    add(
        &mut r,
        "towerB",
        Some("wan1"),
        NodeKind::Intermediate,
        Some("core"),
    );
    let c = r.catalog.as_mut().unwrap();
    c.nodes.iter_mut().find(|n| n.id == "fb").unwrap().parent = Some("towerB".into());
    assert_eq!(targets(&auto_run(&r, 2)), set(&["towerA"]));
}

#[test]
fn placed_source_ignores_automatic_sets() {
    let result = run(&towered(), 6);
    assert_eq!(targets(&result), set(&["group1", "group2"]));
}

// Flat output (R1, R6).

/// `catalog()` with no nodes and no source targets: an empty tree with one known source.
fn empty_tree() -> RegistryDocument {
    let mut r = registry();
    let c = r.catalog.as_mut().unwrap();
    c.nodes.clear();
    for source in &mut c.sources {
        source.targets.clear();
    }
    r
}

#[test]
fn empty_tree_places_every_subscriber_without_a_parent() {
    let result = run(&empty_tree(), 3);
    assert!(result.conflicts.is_empty(), "{:?}", result.conflicts);
    assert_eq!(result.placements.len(), 3);
    assert!(result.placements.iter().all(|p| p.target_id.is_none()));
    // The unplaced source still gives its profile rates.
    assert!(
        result
            .placements
            .iter()
            .all(|p| p.base == rate(50_000_000, 10_000_000))
    );
    let candidate = result.candidate.unwrap();
    assert!(
        candidate
            .subscribers
            .iter()
            .all(|s| s.policy.assignment.is_none())
    );
}

#[test]
fn empty_tree_without_a_plan_is_a_conflict() {
    let result = resolve(collect(&empty_tree(), vec![unbound(0, None)])).unwrap();
    assert_eq!(
        result.conflicts[0].reason,
        PlacementConflictReason::MissingPlan
    );
    assert!(result.candidate.is_none());
}

#[test]
fn roots_that_are_other_router_nodes_give_no_parent() {
    let mut r = empty_tree();
    r.catalog
        .as_mut()
        .unwrap()
        .nodes
        .push(catalog::CatalogNode {
            address_list: None,
            tree_excluded: false,
            router_id: Some("core".into()),
            geo: None,
            minimum: Default::default(),
            id: "coreSite".into(),
            name: "coreSite".into(),
            parent: None,
            source_id: None,
            kind: NodeKind::Intermediate,
            capacity: catalog::CapacityPolicy::Fixed {
                base: rate(1_000_000_000, 1_000_000_000),
                factors: FactorChoice::Inherit,
            },
        });
    let result = auto_run(&r, 2);
    assert!(result.conflicts.is_empty(), "{:?}", result.conflicts);
    assert!(result.placements.iter().all(|p| p.target_id.is_none()));
}

#[test]
fn missing_catalog_uses_the_default_catalog() {
    let r = RegistryDocument::default();
    let result = resolve(collect(
        &r,
        vec![unbound(0, Some(rate(20_000_000, 5_000_000)))],
    ))
    .unwrap();
    assert!(result.conflicts.is_empty(), "{:?}", result.conflicts);
    assert_eq!(result.placements[0].target_id, None);
}

// Allocation modes, quality, and failover.

use crate::topology_manager::wan_health::{WanSnapshot, WanState, WanStatus};

/// A WAN node ID, its state, and its quality as `(numerator, denominator)`.
type WanRow<'a> = (&'a str, WanState, Option<(u64, u64)>);

fn wan_of(states: &[WanRow<'_>]) -> WanSnapshot {
    WanSnapshot(
        states
            .iter()
            .map(|(id, state, q)| {
                let quality = q.map(|(n, d)| frac(n, d));
                (
                    id.to_string(),
                    WanStatus {
                        state: *state,
                        quality,
                    },
                )
            })
            .collect(),
    )
}

fn auto_with(r: &RegistryDocument, count: u32, wan: &[WanRow<'_>]) -> PlacementResolution {
    let accounts = (0..count)
        .map(|i| unbound(i, Some(rate(50_000_000, 10_000_000))))
        .collect();
    let mut c = collect(r, accounts);
    c.wan = wan_of(wan);
    resolve(c).unwrap()
}

fn mode(m: catalog::AllocationMode) -> RegistryDocument {
    let mut r = registry();
    r.catalog.as_mut().unwrap().allocation = m;
    r
}

#[test]
fn count_is_the_default_for_automatic_sets() {
    let result = auto_with(&mode(catalog::AllocationMode::Count), 9, &[]);
    let (a, b) = (count(&result, "wan1"), count(&result, "wan2"));
    assert_eq!(a + b, 9);
    assert!(a.abs_diff(b) <= 1, "{a} / {b}");
}

#[test]
fn capacity_mode_follows_capacity() {
    let result = auto_with(&mode(catalog::AllocationMode::Capacity), 9, &[]);
    assert_eq!((count(&result, "wan1"), count(&result, "wan2")), (3, 6));
}

#[test]
fn quality_mode_weights_capacity_by_quality() {
    let wan = [("wan2", WanState::Up, Some((1, 4)))];
    let result = auto_with(&mode(catalog::AllocationMode::Quality), 9, &wan);
    // 310 : 620 x 1/4 = 310 : 155.
    assert_eq!((count(&result, "wan1"), count(&result, "wan2")), (6, 3));
}

#[test]
fn stale_quality_uses_capacity() {
    let wan = [("wan2", WanState::Unknown, None)];
    let result = auto_with(&mode(catalog::AllocationMode::Quality), 9, &wan);
    assert_eq!((count(&result, "wan1"), count(&result, "wan2")), (3, 6));
}

#[test]
fn an_unavailable_wan_branch_leaves_every_set() {
    let wan = [("wan1", WanState::Unavailable, None)];
    assert_eq!(targets(&auto_with(&registry(), 4, &wan)), set(&["wan2"]));
    let mut c = collect(&registry(), (0..4).map(observed).collect());
    c.wan = wan_of(&wan);
    assert_eq!(targets(&resolve(c).unwrap()), set(&["group2"]));
}

/// One placed subscriber, its saved group, and the WAN above that group.
fn saved_one() -> (RegistryDocument, String, &'static str) {
    let r = run(&registry(), 1).candidate.unwrap();
    let group = r.subscribers[0].policy.assignment.clone().unwrap();
    let wan = if group == "group1" { "wan1" } else { "wan2" };
    (r, group, wan)
}

#[test]
fn a_failover_keeps_the_home_assignment() {
    let (r, group, wan) = saved_one();
    let mut c = collect(&r, vec![observed(0)]);
    c.wan = wan_of(&[(wan, WanState::Unavailable, None)]);
    let result = resolve(c).unwrap();
    let p = &result.placements[0];
    assert_ne!(p.target_id.as_deref(), Some(group.as_str()));
    assert_eq!(p.home.as_deref(), Some(group.as_str()));
    let saved = &result.candidate.unwrap().subscribers[0];
    assert_eq!(saved.policy.assignment.as_deref(), Some(group.as_str()));
}

#[test]
fn the_home_returns_when_the_wan_is_up() {
    let (r, group, wan) = saved_one();
    let mut c = collect(&r, vec![observed(0)]);
    c.wan = wan_of(&[(wan, WanState::Up, None)]);
    let result = resolve(c).unwrap();
    assert_eq!(
        result.placements[0].target_id.as_deref(),
        Some(group.as_str())
    );
    assert_eq!(result.placements[0].home, None);
}

#[test]
fn when_every_wan_is_unavailable_nobody_moves() {
    let (r, group, _) = saved_one();
    let mut c = collect(&r, vec![observed(0)]);
    c.wan = wan_of(&[
        ("wan1", WanState::Unavailable, None),
        ("wan2", WanState::Unavailable, None),
    ]);
    let result = resolve(c).unwrap();
    assert!(result.conflicts.is_empty(), "{:?}", result.conflicts);
    assert_eq!(
        result.placements[0].target_id.as_deref(),
        Some(group.as_str())
    );
    assert_eq!(result.placements[0].home, None);
}

#[test]
fn quality_changes_do_not_move_saved_assignments() {
    let r = auto_with(&mode(catalog::AllocationMode::Quality), 9, &[])
        .candidate
        .unwrap();
    let before: Vec<_> = r
        .subscribers
        .iter()
        .map(|s| s.policy.assignment.clone())
        .collect();
    let after = auto_with(&r, 9, &[("wan2", WanState::Up, Some((1, 10)))]);
    let placed: Vec<_> = after
        .placements
        .iter()
        .map(|p| p.target_id.clone())
        .collect();
    let mut expected = before.clone();
    expected.sort();
    let mut placed_sorted = placed.clone();
    placed_sorted.sort();
    assert_eq!(placed_sorted, expected);
}

#[test]
fn placement_never_targets_a_tree_excluded_router_node() {
    let mut r = registry();
    add(
        &mut r,
        "towerA",
        Some("wan1"),
        NodeKind::Intermediate,
        Some("distro"),
    );
    r.catalog
        .as_mut()
        .unwrap()
        .nodes
        .last_mut()
        .unwrap()
        .tree_excluded = true;
    assert_eq!(targets(&auto_run(&r, 6)), set(&["wan1", "wan2"]));
}

#[test]
fn placement_never_targets_a_tree_excluded_display_node() {
    let mut r = registry();
    add(&mut r, "lqos", Some("wan1"), NodeKind::Intermediate, None);
    r.catalog
        .as_mut()
        .unwrap()
        .nodes
        .last_mut()
        .unwrap()
        .tree_excluded = true;
    assert_eq!(targets(&auto_run(&r, 6)), set(&["wan1", "wan2"]));
}

#[test]
fn an_excluded_source_leaves_the_pair() {
    let mut r = registry();
    for s in &mut r.catalog.as_mut().unwrap().sources {
        if matches!(s.origin, catalog::SourceOrigin::Subscribers { .. }) {
            s.excluded = true;
        }
    }
    let result = run(&r, 3);
    assert!(result.placements.is_empty(), "{:?}", result.placements);
    assert!(result.active.is_empty());
    // The registry keeps the records, so a source that returns keeps its Circuit IDs.
    assert_eq!(result.candidate.as_ref().unwrap().subscribers.len(), 3);
}
