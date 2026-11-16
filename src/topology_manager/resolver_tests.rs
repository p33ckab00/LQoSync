use super::*;
use crate::topology_manager::catalog::{self, FactorChoice, Factors};
use crate::topology_manager::model::{SourceKind, SourceScope};
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
        .filter(|p| p.target_id == target)
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
    assert_eq!(result.placements[0].target_id, "group1");
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
    for (profile, expected) in [
        (
            ProfileObservation::Unread,
            PlacementConflictReason::MissingProfileEvidence,
        ),
        (
            ProfileObservation::Known(None),
            PlacementConflictReason::NoEligibleTarget,
        ),
    ] {
        let mut a = observed(0);
        a.policy.profile = profile;
        let result = resolve(collect(&registry(), vec![a])).unwrap();
        assert_eq!(result.conflicts[0].reason, expected);
        assert!(result.candidate.is_none());
    }
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
fn invalid_saved_assignment_blocks_automatic_fallback() {
    let r = run(&registry(), 1).candidate.unwrap();
    let mut a = observed(0);
    a.policy.profile = ProfileObservation::Known(None);
    let result = resolve(collect(&r, vec![a])).unwrap();
    assert!(matches!(
        result.conflicts[0].reason,
        PlacementConflictReason::InvalidAssignment(_)
    ));
    assert!(result.candidate.is_none());
    // Intent remains intact while the candidate is held.
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
        .filter(|p| p.target_id == "group1")
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
