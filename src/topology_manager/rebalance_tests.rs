use super::*;
use crate::topology_manager::catalog::{self, FactorChoice, Factors};
use crate::topology_manager::observation::{
    AccountObservation, Endpoint, ObservationGate, ObservedPolicy, ProfileObservation, SourceRead,
    reconcile,
};
use crate::topology_manager::rates::Fraction;

#[test]
fn indivisible_plan_rounding_keeps_the_normal_resolvers_balanced_placements() {
    for count in 1..=20 {
        let r = resolve(collect(&registry(), (0..count).map(observed).collect()))
            .unwrap()
            .candidate
            .unwrap();
        assert!(
            run(&r, count).proposed_moves.is_empty(),
            "equal bottleneck score changed {count} circuits"
        );
    }
}

fn rate(down: u64, up: u64) -> RateBps {
    RateBps { down, up }
}
fn frac(numerator: u64, denominator: u64) -> Fraction {
    Fraction {
        numerator,
        denominator,
    }
}
fn registry() -> RegistryDocument {
    let mut c = catalog::tests::catalog();
    if let SourceOrigin::Subscribers { group, .. } = &mut c.sources[2].origin {
        group.backstop = None;
    }
    RegistryDocument {
        catalog: Some(c),
        ..Default::default()
    }
}
fn observed(i: u32) -> AccountObservation {
    AccountObservation {
        account_ref: format!("account-{i:05}"),
        policy: ObservedPolicy {
            profile: ProfileObservation::Known(Some("provider-profile-id".into())),
            plan: Some(rate(50_000_000, 10_000_000)),
        },
        endpoints: vec![Endpoint {
            session_id: format!("session-{i}"),
            ips: vec![std::net::IpAddr::V4((0x0a000001 + i).into())],
            mac: None,
        }],
    }
}
fn collect(r: &RegistryDocument, accounts: Vec<AccountObservation>) -> Reconciliation {
    let scopes = r.catalog.as_ref().unwrap().required_scopes().unwrap();
    assert_eq!(scopes.len(), 1);
    let snapshot = ObservationGate::default()
        .accept(
            1,
            &scopes,
            vec![SourceRead::Complete {
                scope: scopes[0].clone(),
                accounts,
            }],
        )
        .unwrap();
    reconcile(r, snapshot).unwrap()
}
fn seeded(r: &RegistryDocument, count: u32, target: &str) -> RegistryDocument {
    let mut r = collect(r, (0..count).map(observed).collect()).registry;
    for subscriber in &mut r.subscribers {
        subscriber.policy.assignment = Some(target.into());
    }
    r
}
fn request(r: &RegistryDocument) -> RebalanceRequest {
    RebalanceRequest {
        expected_revision: r.revision,
        scope: RebalanceScope::SourcePool("profile-source".into()),
    }
}
fn run(r: &RegistryDocument, count: u32) -> RebalanceResult {
    rebalance(collect(r, (0..count).map(observed).collect()), request(r)).unwrap()
}
fn count(result: &RebalanceResult, target: &str) -> usize {
    result
        .evaluation
        .placements
        .iter()
        .filter(|p| p.target_id == target)
        .count()
}

#[test]
fn one_profile_rebalances_across_wans_with_only_the_required_homogeneous_moves() {
    let r = seeded(&registry(), 60, "group1");
    let result = run(&r, 60);
    assert_eq!(
        (count(&result, "group1"), count(&result, "group2")),
        (20, 40)
    );
    assert_eq!(result.proposed_moves.len(), 40);
    assert!(result.evaluation.candidate.is_some());
    assert!(
        result
            .evaluation
            .placements
            .iter()
            .all(|p| p.rates.max == rate(50_000_000, 10_000_000))
    );
    assert!(
        r.subscribers
            .iter()
            .all(|s| s.policy.assignment.as_deref() == Some("group1"))
    );
}

#[test]
fn a_balanced_pool_does_not_move_and_a_single_excess_placement_moves_once() {
    let r = resolve(collect(&registry(), (0..60).map(observed).collect()))
        .unwrap()
        .candidate
        .unwrap();
    let result = run(&r, 60);
    assert!(result.proposed_moves.is_empty());
    assert_eq!(result.evaluation.candidate.unwrap(), r);
    let mut unbalanced = r;
    unbalanced
        .subscribers
        .iter_mut()
        .find(|s| s.policy.assignment.as_deref() == Some("group2"))
        .unwrap()
        .policy
        .assignment = Some("group1".into());
    assert_eq!(run(&unbalanced, 60).proposed_moves.len(), 1);
}

#[test]
fn equal_quality_prefers_the_existing_assignment() {
    let mut r = registry();
    r.catalog.as_mut().unwrap().nodes[1].capacity = CapacityPolicy::Fixed {
        base: rate(310_000_000, 100_000_000),
        factors: FactorChoice::Inherit,
    };
    let r = seeded(&r, 1, "group2");
    let result = run(&r, 1);
    assert!(result.proposed_moves.is_empty());
    assert_eq!(count(&result, "group2"), 1);
}

#[test]
fn pins_remain_fixed_and_contribute_to_plan_demand() {
    let mut r = seeded(&registry(), 60, "group1");
    for s in r.subscribers.iter_mut().take(30) {
        s.policy.pin = Some("group1".into());
    }
    let result = run(&r, 60);
    assert_eq!(
        (count(&result, "group1"), count(&result, "group2")),
        (30, 30)
    );
    assert_eq!(result.proposed_moves.len(), 30);
    let after = result.evaluation.candidate.unwrap();
    for before in r.subscribers.iter().filter(|s| s.policy.pin.is_some()) {
        assert_eq!(
            after
                .subscribers
                .iter()
                .find(|s| s.id == before.id)
                .unwrap(),
            before
        );
    }
}

#[test]
fn stale_revision_and_unknown_or_non_subscriber_scopes_fail_closed() {
    let r = seeded(&registry(), 2, "group1");
    let mut stale = request(&r);
    stale.expected_revision += 1;
    assert!(rebalance(collect(&r, vec![observed(0)]), stale).is_err());
    for scope in [
        RebalanceScope::Branch("missing".into()),
        RebalanceScope::SourcePool("missing".into()),
        RebalanceScope::SourcePool("wan-source-1".into()),
    ] {
        assert!(
            rebalance(
                collect(&r, vec![observed(0)]),
                RebalanceRequest {
                    expected_revision: r.revision,
                    scope
                }
            )
            .is_err()
        );
    }
}

#[test]
fn branch_scope_keeps_other_wan_placements_and_uses_internal_destinations_only() {
    let mut r = registry();
    let c = r.catalog.as_mut().unwrap();
    c.nodes[3].parent = Some("wan1".into());
    c.nodes[3].source_id = None;
    let mut outside = c.nodes[3].clone();
    outside.id = "outside".into();
    outside.parent = Some("wan2".into());
    c.nodes.push(outside);
    c.sources[2].targets.push("outside".into());
    let mut r = seeded(&r, 30, "group1");
    for s in r.subscribers.iter_mut().skip(20) {
        s.policy.assignment = Some("outside".into());
    }
    let request = RebalanceRequest {
        expected_revision: r.revision,
        scope: RebalanceScope::Branch("wan1".into()),
    };
    let result = rebalance(collect(&r, (0..30).map(observed).collect()), request).unwrap();
    assert_eq!(
        (
            count(&result, "group1"),
            count(&result, "group2"),
            count(&result, "outside")
        ),
        (10, 10, 10)
    );
    assert_eq!(result.proposed_moves.len(), 10);
    assert!(
        result
            .proposed_moves
            .iter()
            .all(|m| m.from_target_id != "outside" && m.to_target_id != "outside")
    );
}

#[test]
fn profile_and_source_target_restrictions_remain_effective() {
    let mut r = registry();
    r.catalog.as_mut().unwrap().profiles[0].eligible_targets = Some(vec!["group1".into()]);
    let r = seeded(&r, 12, "group1");
    assert!(run(&r, 12).proposed_moves.is_empty());
    let mut r = registry();
    r.catalog.as_mut().unwrap().sources[2].targets = vec!["group1".into()];
    let r = seeded(&r, 12, "group1");
    assert!(run(&r, 12).proposed_moves.is_empty());
}

#[test]
fn current_source_membership_limits_pool_scope_even_with_shared_destinations() {
    let mut r = registry();
    let c = r.catalog.as_mut().unwrap();
    c.nodes[2].source_id = None;
    c.nodes[3].source_id = None;
    let mut other = c.sources[2].clone();
    other.id = "other-source".into();
    if let SourceOrigin::Subscribers { profile_ref, .. } = &mut other.origin {
        *profile_ref = Some("other-profile".into());
    }
    c.sources.push(other);
    let accounts = || {
        (0..60)
            .map(|i| {
                let mut a = observed(i);
                if i >= 30 {
                    a.policy.profile = ProfileObservation::Known(Some("other-profile".into()));
                }
                a
            })
            .collect()
    };
    let mut r = collect(&r, accounts()).registry;
    for s in &mut r.subscribers {
        s.policy.assignment = Some("group1".into());
    }
    let result = rebalance(collect(&r, accounts()), request(&r)).unwrap();
    assert_eq!(result.proposed_moves.len(), 30);
    let after = result.evaluation.candidate.unwrap();
    for before in r.subscribers.iter().skip(30) {
        assert_eq!(
            after
                .subscribers
                .iter()
                .find(|s| s.id == before.id)
                .unwrap(),
            before
        );
    }
}

#[test]
fn automatic_group_capacities_are_not_allocation_weights_and_conflicts_hold_the_proposal() {
    let mut r = registry();
    r.catalog.as_mut().unwrap().nodes[3].capacity = CapacityPolicy::Automatic {
        factors: FactorChoice::Override(Factors {
            down: frac(1, 100),
            up: frac(1, 100),
        }),
    };
    let r = seeded(&r, 60, "group1");
    let result = run(&r, 60);
    assert_eq!(
        (count(&result, "group1"), count(&result, "group2")),
        (20, 40)
    );
    assert_eq!(result.proposed_moves.len(), 40);
    assert!(result.evaluation.candidate.is_none());
    assert!(
        result
            .evaluation
            .conflicts
            .iter()
            .any(|c| c.node_id == "group2")
    );
}

#[test]
fn direct_wan_destinations_and_asymmetric_upload_capacity_are_supported() {
    let mut r = registry();
    let c = r.catalog.as_mut().unwrap();
    c.nodes.truncate(2);
    c.sources[2].targets = vec!["wan1".into(), "wan2".into()];
    c.profiles[0].base = BaseRateAuthority::Local(rate(50_000_000, 20_000_000));
    c.nodes[1].capacity = CapacityPolicy::Fixed {
        base: rate(620_000_000, 100_000_000),
        factors: FactorChoice::Inherit,
    };
    let r = seeded(&r, 60, "wan1");
    let result = run(&r, 60);
    assert_eq!((count(&result, "wan1"), count(&result, "wan2")), (30, 30));
}

#[test]
fn no_wan_groups_use_equal_plan_demand_without_a_synthetic_capacity() {
    let mut r = registry();
    let c = r.catalog.as_mut().unwrap();
    c.nodes.drain(..2);
    for n in &mut c.nodes {
        n.parent = None;
    }
    c.nodes[1].source_id = None;
    let r = seeded(&r, 20, "group1");
    let result = run(&r, 20);
    assert_eq!(
        (count(&result, "group1"), count(&result, "group2")),
        (10, 10)
    );
}

#[test]
fn offline_intent_is_preserved_and_only_current_circuits_enter_the_action() {
    let r = seeded(&registry(), 60, "group1");
    let result = run(&r, 30);
    assert_eq!(
        (count(&result, "group1"), count(&result, "group2")),
        (10, 20)
    );
    let after = result.evaluation.candidate.unwrap();
    for before in r.subscribers.iter().skip(30) {
        assert_eq!(
            after
                .subscribers
                .iter()
                .find(|s| s.id == before.id)
                .unwrap(),
            before
        );
    }
}

#[test]
fn collection_and_catalog_order_do_not_change_proposed_moves() {
    let r = seeded(&registry(), 60, "group1");
    let forward = run(&r, 60);
    let mut r = r;
    let c = r.catalog.as_mut().unwrap();
    c.nodes.reverse();
    c.sources.reverse();
    let reverse = rebalance(
        collect(&r, (0..60).rev().map(observed).collect()),
        request(&r),
    )
    .unwrap();
    assert_eq!(forward.proposed_moves, reverse.proposed_moves);
    assert_eq!(forward.evaluation.placements, reverse.evaluation.placements);
}

#[test]
fn an_unselected_discovery_cannot_become_a_rebalance_scope() {
    let mut r = registry();
    let c = r.catalog.as_mut().unwrap();
    let mut source = c.sources[2].clone();
    source.id = "unselected".into();
    source.targets.clear();
    if let SourceOrigin::Subscribers { profile_ref, .. } = &mut source.origin {
        *profile_ref = None;
    }
    c.sources.push(source);
    let r = seeded(&r, 2, "group1");
    assert!(
        rebalance(
            collect(&r, vec![observed(0)]),
            RebalanceRequest {
                expected_revision: r.revision,
                scope: RebalanceScope::SourcePool("unselected".into())
            }
        )
        .is_err()
    );
}

#[test]
fn mixed_bounded_and_unbounded_destinations_require_an_explicit_capacity_policy() {
    let mut r = registry();
    let c = r.catalog.as_mut().unwrap();
    c.nodes.drain(..2);
    for n in &mut c.nodes {
        n.parent = None;
    }
    c.nodes[1].source_id = None;
    c.nodes[0].capacity = CapacityPolicy::Fixed {
        base: rate(310_000_000, 100_000_000),
        factors: FactorChoice::Inherit,
    };
    let r = seeded(&r, 10, "group1");
    assert!(rebalance(collect(&r, (0..10).map(observed).collect()), request(&r)).is_err());
}

#[test]
fn explicit_source_instances_work_without_a_shared_target_list() {
    let mut r = registry();
    r.catalog.as_mut().unwrap().sources[2].targets.clear();
    let r = seeded(&r, 60, "group1");
    let result = run(&r, 60);
    assert_eq!(
        (count(&result, "group1"), count(&result, "group2")),
        (20, 40)
    );
}

#[test]
fn a_pool_action_does_not_pull_placements_from_outside_its_destinations() {
    let mut r = registry();
    let c = r.catalog.as_mut().unwrap();
    c.nodes[2].source_id = None;
    c.nodes[3].source_id = None;
    let mut broad = c.sources[2].clone();
    broad.id = "broad".into();
    if let SourceOrigin::Subscribers { profile_ref, .. } = &mut broad.origin {
        *profile_ref = None;
    }
    c.sources.push(broad);
    c.sources[2].targets = vec!["group1".into()];
    let r = seeded(&r, 10, "group2");
    let result = run(&r, 10);
    assert!(result.proposed_moves.is_empty());
    assert_eq!(count(&result, "group2"), 10);
}

#[test]
fn invalid_current_membership_and_pins_do_not_trigger_a_fallback() {
    let mut r = seeded(&registry(), 2, "group1");
    r.subscribers[0].policy.pin = Some("group1".into());
    let before = r.clone();
    let mut a = observed(0);
    a.policy.profile = ProfileObservation::Known(Some("other".into()));
    assert!(rebalance(collect(&r, vec![a, observed(1)]), request(&r)).is_err());
    assert_eq!(r, before);
}

#[test]
fn multiple_current_endpoints_do_not_add_plan_demand_or_duplicate_moves() {
    let r = seeded(&registry(), 6, "group1");
    let mut a = observed(0);
    a.endpoints.extend(observed(99).endpoints);
    let result = rebalance(
        collect(&r, std::iter::once(a).chain((1..6).map(observed)).collect()),
        request(&r),
    )
    .unwrap();
    assert_eq!(result.proposed_moves.len(), 4);
    assert_eq!(result.evaluation.placements.len(), 6);
    assert_eq!(result.evaluation.active[0].endpoints.len(), 2);
}

#[test]
fn different_plan_sizes_rebalance_demand_instead_of_circuit_count() {
    let mut r = registry();
    r.catalog.as_mut().unwrap().profiles[0].base = BaseRateAuthority::SourceRead;
    let accounts = || {
        (0..30)
            .map(|i| {
                let mut a = observed(i);
                a.policy.plan = Some(if i < 10 {
                    rate(100_000_000, 20_000_000)
                } else {
                    rate(10_000_000, 2_000_000)
                });
                a
            })
            .collect()
    };
    let mut r = collect(&r, accounts()).registry;
    for s in &mut r.subscribers {
        s.policy.assignment = Some("group1".into());
    }
    let result = rebalance(collect(&r, accounts()), request(&r)).unwrap();
    let demand: u64 = result
        .evaluation
        .placements
        .iter()
        .filter(|p| p.target_id == "group1")
        .map(|p| p.base.down)
        .sum();
    assert_eq!(demand, 400_000_000);
    assert_ne!(count(&result, "group1"), 10);
    assert!(result.evaluation.candidate.is_some());
}

#[test]
fn checked_rebalance_persists_without_replaying_session_data_after_restart() {
    use crate::topology_manager::store::RegistryStore;
    let dir = tempfile::tempdir().unwrap();
    let mut store = RegistryStore::open(dir.path()).unwrap();
    store.commit(0, seeded(&registry(), 60, "group1")).unwrap();
    let result = run(store.document().unwrap(), 60);
    store
        .commit(1, result.evaluation.candidate.unwrap())
        .unwrap();
    drop(store);
    let store = RegistryStore::open(dir.path()).unwrap();
    let mut accounts: Vec<_> = (0..60).map(observed).collect();
    accounts[0].endpoints = observed(99).endpoints;
    let again = rebalance(
        collect(store.document().unwrap(), accounts),
        request(store.document().unwrap()),
    )
    .unwrap();
    assert!(again.proposed_moves.is_empty());
    assert_eq!(
        again.evaluation.active[0].endpoints[0].endpoint.session_id,
        "session-99"
    );
    let bytes = std::fs::read_to_string(dir.path().join("topology-manager/registry.json")).unwrap();
    assert!(!bytes.contains("session-99"));
    assert!(!bytes.contains("10.0.0."));
}

#[test]
fn synthetic_ten_thousand_circuit_action_is_bounded_and_stable_on_repeat() {
    let mut r = registry();
    let c = r.catalog.as_mut().unwrap();
    c.nodes[0].capacity = CapacityPolicy::Fixed {
        base: rate(31_000_000_000, 10_000_000_000),
        factors: FactorChoice::Inherit,
    };
    c.nodes[1].capacity = CapacityPolicy::Fixed {
        base: rate(62_000_000_000, 20_000_000_000),
        factors: FactorChoice::Inherit,
    };
    let r = seeded(&r, 10_000, "group1");
    let result = run(&r, 10_000);
    assert_eq!(
        (count(&result, "group1"), count(&result, "group2")),
        (3333, 6667)
    );
    assert_eq!(result.proposed_moves.len(), 6667);
    let after = result.evaluation.candidate.unwrap();
    assert!(run(&after, 10_000).proposed_moves.is_empty());
}

#[test]
fn sparse_score_deltas_match_full_world_comparison_and_shared_ancestor_loads() {
    fn loads(index: &Index<'_>, assigned: &[(usize, RateBps)]) -> Loads {
        let mut loads = Loads {
            nodes: vec![ZERO; index.catalog.nodes.len()],
            targets: vec![ZERO; index.targets.len()],
            paths: index
                .targets
                .iter()
                .map(|t| t.independent_path.iter().copied().collect())
                .collect(),
        };
        for &(target, base) in assigned {
            loads.targets[target] = add(loads.targets[target], base).unwrap();
            for &n in &index.targets[target].independent_path {
                loads.nodes[n] = add(loads.nodes[n], base).unwrap();
            }
        }
        loads
    }
    fn quality(index: &Index<'_>, loads: &Loads) -> (Vec<Ratio>, Vec<Ratio>) {
        let independent: HashSet<_> = index
            .targets
            .iter()
            .flat_map(|t| t.independent_path.iter().copied())
            .collect();
        let mut independent: Vec<_> = independent
            .into_iter()
            .map(|n| ratio(loads.nodes[n], index.capacities[n]))
            .collect();
        let mut targets: Vec<_> = loads
            .targets
            .iter()
            .enumerate()
            .map(|(t, load)| ratio(*load, index.targets[t].capacity))
            .collect();
        independent.sort_unstable_by(|a, b| b.cmp(a));
        targets.sort_unstable_by(|a, b| b.cmp(a));
        (independent, targets)
    }
    let mut r = registry();
    let c = r.catalog.as_mut().unwrap();
    c.nodes[2].capacity = CapacityPolicy::Fixed {
        base: rate(100_000_000, 20_000_000),
        factors: FactorChoice::Override(Factors::default()),
    };
    let mut extra = c.nodes[2].clone();
    extra.id = "group3".into();
    extra.source_id = None;
    extra.capacity = CapacityPolicy::Fixed {
        base: rate(80_000_000, 16_000_000),
        factors: FactorChoice::Override(Factors::default()),
    };
    c.nodes.push(extra);
    c.sources[2].targets.push("group3".into());
    let index = Index::new(c).unwrap();
    for seed in 0..128u64 {
        let assigned: Vec<_> = (0..12)
            .map(|i| {
                (
                    ((seed + i) % 3) as usize,
                    rate(
                        ((seed * 17 + i * 13) % 41 + 1) * 1_000_000,
                        ((seed * 3 + i * 7) % 19 + 1) * 1_000_000,
                    ),
                )
            })
            .collect();
        let before = loads(&index, &assigned);
        let before_quality = quality(&index, &before);
        for circuit in 0..3 {
            let (from, base) = assigned[circuit];
            let mut trials = Vec::new();
            for to in 0..3 {
                if to == from {
                    continue;
                }
                let (delta, improves) = before.trial(&index, from, to, base).unwrap();
                let mut changed = assigned.clone();
                changed[circuit].0 = to;
                let expected = loads(&index, &changed);
                let after_quality = quality(&index, &expected);
                assert_eq!(
                    delta.compare(&Delta::default()),
                    after_quality.cmp(&before_quality)
                );
                if improves {
                    assert_eq!(after_quality.cmp(&before_quality), Ordering::Less);
                }
                let mut applied = loads(&index, &assigned);
                applied.apply(from, to, base).unwrap();
                assert_eq!(applied.nodes, expected.nodes);
                assert_eq!(applied.targets, expected.targets);
                trials.push((delta, after_quality));
            }
            assert_eq!(
                trials[0].0.compare(&trials[1].0),
                trials[0].1.cmp(&trials[1].1)
            );
        }
    }
}
