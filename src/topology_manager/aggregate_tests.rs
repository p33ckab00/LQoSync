use super::*;
use crate::topology_manager::catalog::{
    self, BackstopPolicy, CapacityPolicy, CatalogNode, FactorChoice, Factors, GroupFormula,
    SourceOrigin,
};
use crate::topology_manager::layout::NodeKind;
use crate::topology_manager::model::SourceKind;
use crate::topology_manager::observation::{
    AccountObservation, Endpoint, ObservationGate, ObservedPolicy, ProfileObservation, SourceRead,
    reconcile,
};
use crate::topology_manager::rates::{Fraction, RateRule, RuleValue};
use crate::topology_manager::resolver::resolve;

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
    c.sources[2].targets = vec!["group1".into()];
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
fn placements(r: &RegistryDocument, accounts: Vec<AccountObservation>) -> PlacementResolution {
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
    resolve(reconcile(r, snapshot).unwrap()).unwrap()
}
fn run(r: &RegistryDocument, count: u32) -> AggregateResolution {
    evaluate(placements(r, (0..count).map(observed).collect())).unwrap()
}
fn node<'a>(result: &'a AggregateResolution, id: &str) -> &'a AggregateNode {
    result.nodes.iter().find(|n| n.node_id == id).unwrap()
}
fn fixed(base: RateBps) -> CapacityPolicy {
    CapacityPolicy::Fixed {
        base,
        factors: FactorChoice::Override(Factors::default()),
    }
}

#[test]
fn ten_fifty_megabit_plans_keep_maxima_and_stay_below_the_wan() {
    let result = run(&registry(), 10);
    let group = node(&result, "group1");
    assert_eq!(group.basis, Some(rate(500_000_000, 100_000_000)));
    assert_eq!(group.calculated, rate(400_000_000, 50_000_000));
    assert_eq!(group.rates.local, group.calculated);
    assert_eq!(group.rates.ceiling, rate(310_000_000, 50_000_000));
    assert_eq!(
        node(&result, "wan1").rates.local,
        rate(310_000_000, 100_000_000)
    );
    assert!(
        result
            .placements
            .iter()
            .all(|p| p.rates.max == rate(50_000_000, 10_000_000))
    );
    assert!(result.conflicts.is_empty());
    assert!(result.candidate.is_some());
    assert!(
        result
            .warnings
            .iter()
            .any(|w| w.node_id == "wan1" && w.plan_over.down)
    );
}

#[test]
fn wan_direct_subscribers_do_not_compute_wan_capacity() {
    let mut r = registry();
    let c = r.catalog.as_mut().unwrap();
    c.nodes.truncate(2);
    c.sources[2].targets = vec!["wan1".into()];
    let result = run(&r, 10);
    let wan = node(&result, "wan1");
    assert_eq!(wan.direct_circuits, 10);
    assert_eq!(wan.basis, None);
    assert_eq!(wan.rates.local, rate(310_000_000, 100_000_000));
}

#[test]
fn a_manual_group_uses_its_configured_factors_once() {
    let mut r = registry();
    r.catalog.as_mut().unwrap().nodes[2].capacity = CapacityPolicy::Fixed {
        base: rate(200_000_000, 100_000_000),
        factors: FactorChoice::Override(Factors {
            down: frac(1, 2),
            up: frac(1, 4),
        }),
    };
    let result = run(&r, 2);
    let group = node(&result, "group1");
    assert_eq!(group.basis, None);
    assert_eq!(group.calculated, rate(100_000_000, 25_000_000));
}

#[test]
fn automatic_override_replaces_shared_factors() {
    let mut r = registry();
    r.catalog.as_mut().unwrap().nodes[2].capacity = CapacityPolicy::Automatic {
        factors: FactorChoice::Override(Factors {
            down: frac(1, 2),
            up: frac(1, 4),
        }),
    };
    let result = run(&r, 2);
    assert_eq!(
        node(&result, "group1").calculated,
        rate(50_000_000, 5_000_000)
    );
}

#[test]
fn backstop_caps_preserve_the_unconstrained_calculation() {
    let mut r = registry();
    if let SourceOrigin::Subscribers { group, .. } =
        &mut r.catalog.as_mut().unwrap().sources[2].origin
    {
        group.backstop = Some(BackstopPolicy {
            base: rate(50_000_000, 10_000_000),
            factor: frac(1, 2),
        });
    }
    let result = run(&r, 2);
    let group = node(&result, "group1");
    assert_eq!(group.calculated, rate(80_000_000, 10_000_000));
    assert_eq!(group.rates.local, rate(25_000_000, 5_000_000));
    assert_eq!(group.rates.ceiling, group.rates.local);
}

#[test]
fn custom_groups_use_plan_sum_without_source_factor_inheritance() {
    let mut r = registry();
    r.catalog.as_mut().unwrap().nodes[2].source_id = None;
    let result = run(&r, 2);
    assert_eq!(
        node(&result, "group1").calculated,
        rate(100_000_000, 20_000_000)
    );
}

#[test]
fn dhcp_site_and_hotspot_formulas_count_current_circuits() {
    for kind in [SourceKind::Dhcp, SourceKind::Hotspot] {
        let mut r = registry();
        if let SourceOrigin::Subscribers { scope, group, .. } =
            &mut r.catalog.as_mut().unwrap().sources[2].origin
        {
            scope.kind = kind;
            group.formula = GroupFormula::CountTimesUnit(rate(20_000_000, 4_000_000));
        }
        let mut a = observed(0);
        a.endpoints.extend(observed(99).endpoints);
        let result = evaluate(placements(&r, vec![a, observed(1)])).unwrap();
        let group = node(&result, "group1");
        assert_eq!(group.direct_circuits, 2);
        assert_eq!(group.basis, Some(rate(40_000_000, 8_000_000)));
        assert_eq!(group.calculated, rate(32_000_000, 4_000_000));
        assert_eq!(group.plan_demand, rate(100_000_000, 20_000_000));
    }
}

#[test]
fn dhcp_per_plan_uses_authoritative_plan_sum() {
    let mut r = registry();
    if let SourceOrigin::Subscribers { scope, .. } =
        &mut r.catalog.as_mut().unwrap().sources[2].origin
    {
        scope.kind = SourceKind::Dhcp;
    }
    let result = run(&r, 2);
    assert_eq!(
        node(&result, "group1").basis,
        Some(rate(100_000_000, 20_000_000))
    );
}

#[test]
fn inherited_intermediates_do_not_recalculate_subscriber_plans() {
    let mut r = registry();
    let c = r.catalog.as_mut().unwrap();
    c.nodes.push(CatalogNode {
        router_id: None,
        geo: None,
        minimum: Default::default(),
        id: "middle".into(),
        name: "Middle".into(),
        parent: Some("wan1".into()),
        source_id: None,
        kind: NodeKind::Intermediate,
        capacity: CapacityPolicy::Inherit,
    });
    c.nodes[2].parent = Some("middle".into());
    let result = run(&r, 2);
    let middle = node(&result, "middle");
    assert_eq!(middle.direct_circuits, 0);
    assert_eq!(middle.active_circuits, 2);
    assert_eq!(middle.basis, None);
    assert_eq!(middle.rates.local, rate(310_000_000, 100_000_000));
    assert_eq!(middle.minimum_demand, rate(2_000_000, 2_000_000));
}

#[test]
fn nested_limits_clip_descendants_without_compounding_factors() {
    let mut r = registry();
    let c = r.catalog.as_mut().unwrap();
    c.nodes.push(CatalogNode {
        router_id: None,
        geo: None,
        minimum: Default::default(),
        id: "middle".into(),
        name: "Middle".into(),
        parent: Some("wan1".into()),
        source_id: None,
        kind: NodeKind::Intermediate,
        capacity: fixed(rate(40_000_000, 20_000_000)),
    });
    c.nodes[2].parent = Some("middle".into());
    let result = run(&r, 10);
    assert_eq!(
        node(&result, "group1").calculated,
        rate(400_000_000, 50_000_000)
    );
    assert_eq!(
        node(&result, "group1").rates.ceiling,
        rate(40_000_000, 20_000_000)
    );
}

#[test]
fn wan_percentages_use_true_capacity_before_ancestor_limits() {
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
        down: frac(3, 4),
        up: frac(3, 4),
    };
    let result = run(&r, 2);
    let group = node(&result, "group1");
    assert_eq!(group.calculated, rate(232_500_000, 75_000_000));
    assert_eq!(group.rates.ceiling, rate(155_000_000, 50_000_000));
}

#[test]
fn empty_automatic_nodes_are_omitted_but_manual_nodes_and_intent_remain() {
    let r = registry();
    let result = run(&r, 0);
    assert_eq!(result.nodes.len(), 2);
    assert_eq!(result.omitted_nodes, vec!["group1", "group2"]);
    assert_eq!(result.candidate.unwrap(), r);
}

#[test]
fn profile_only_root_has_no_synthetic_capacity_constraint() {
    let mut r = registry();
    let c = r.catalog.as_mut().unwrap();
    c.nodes.retain(|n| n.id == "group1");
    c.nodes[0].parent = None;
    let result = run(&r, 10);
    assert_eq!(result.nodes.len(), 1);
    assert_eq!(result.nodes[0].rates.ceiling, rate(400_000_000, 50_000_000));
}

#[test]
fn excessive_minimums_block_the_candidate_and_identify_each_ancestor() {
    let mut r = registry();
    r.catalog.as_mut().unwrap().profiles[0].speed.down_min =
        RuleValue::Override(RateRule::FixedBps(40_000_000));
    let result = run(&r, 10);
    assert!(result.candidate.is_none());
    assert_eq!(
        result
            .conflicts
            .iter()
            .map(|c| c.node_id.as_str())
            .collect::<Vec<_>>(),
        vec!["group1", "wan1"]
    );
    assert!(
        matches!(result.conflicts[0].reason, AggregateConflictReason::MinimumExceedsCeiling { minimum, ceiling }
        if minimum.down == 400_000_000 && ceiling.down == 310_000_000)
    );
    assert!(
        result
            .placements
            .iter()
            .all(|p| p.rates.min.down == 40_000_000)
    );
}

#[test]
fn minimums_exactly_at_the_ceiling_are_valid() {
    let mut r = registry();
    r.catalog.as_mut().unwrap().profiles[0].speed.down_min =
        RuleValue::Override(RateRule::FixedBps(40_000_000));
    let result = run(&r, 2);
    assert!(result.conflicts.is_empty());
    assert!(result.candidate.is_some());
    assert_eq!(node(&result, "group1").minimum_demand.down, 80_000_000);
}

#[test]
fn aliases_and_endpoints_count_once_for_minimum_commitments() {
    let mut input = placements(&registry(), vec![observed(0), observed(1)])
        .candidate
        .unwrap();
    let second = input.subscribers.pop().unwrap();
    input.subscribers[0].identities.extend(second.identities);
    let result = evaluate(placements(&input, vec![observed(0), observed(1)])).unwrap();
    assert_eq!(node(&result, "wan1").active_circuits, 1);
    assert_eq!(
        node(&result, "wan1").minimum_demand,
        rate(1_000_000, 1_000_000)
    );
}

#[test]
fn sibling_ceilings_can_exceed_a_parent_if_their_minimums_fit() {
    let mut r = registry();
    let c = r.catalog.as_mut().unwrap();
    c.nodes[3].parent = Some("wan1".into());
    c.nodes[3].source_id = None;
    c.sources[2].targets = vec!["group1".into(), "group2".into()];
    let result = run(&r, 20);
    assert!(
        node(&result, "group1").rates.ceiling.down + node(&result, "group2").rates.ceiling.down
            > 310_000_000
    );
    assert!(result.conflicts.is_empty());
    assert!(result.candidate.is_some());
}

#[test]
fn combined_sibling_minimums_are_checked_at_their_common_ancestor() {
    let mut r = registry();
    let c = r.catalog.as_mut().unwrap();
    c.nodes[3].parent = Some("wan1".into());
    c.nodes[3].source_id = None;
    c.sources[2].targets = vec!["group1".into(), "group2".into()];
    c.profiles[0].speed.down_min = RuleValue::Override(RateRule::FixedBps(30_000_000));
    let result = run(&r, 12);
    assert_eq!(result.conflicts.len(), 1);
    assert_eq!(result.conflicts[0].node_id, "wan1");
    assert!(result.candidate.is_none());
}

#[test]
fn unresolved_placement_candidates_cannot_enter_aggregate_evaluation() {
    let mut a = observed(0);
    a.policy.profile = ProfileObservation::Unread;
    assert!(evaluate(placements(&registry(), vec![a])).is_err());
}

#[test]
fn duplicate_or_missing_current_circuit_placements_fail_closed() {
    let r = registry();
    let mut duplicate = placements(&r, vec![observed(0)]);
    duplicate.placements.push(duplicate.placements[0].clone());
    assert!(evaluate(duplicate).is_err());
    let mut missing = placements(&r, vec![observed(0)]);
    missing.placements.clear();
    assert!(evaluate(missing).is_err());
}

#[test]
fn a_deep_reversed_graph_uses_iterative_evaluation() {
    let mut r = registry();
    let c = r.catalog.as_mut().unwrap();
    let mut parent = "wan1".to_string();
    for i in 0..20_000 {
        let id = format!("middle-{i:05}");
        c.nodes.push(CatalogNode {
            router_id: None,
            geo: None,
            minimum: Default::default(),
            id: id.clone(),
            name: id.clone(),
            parent: Some(parent),
            source_id: None,
            kind: NodeKind::Intermediate,
            capacity: CapacityPolicy::Inherit,
        });
        parent = id;
    }
    c.nodes
        .iter_mut()
        .find(|n| n.id == "group1")
        .unwrap()
        .parent = Some(parent);
    c.nodes.reverse();
    let result = run(&r, 2);
    assert!(result.conflicts.is_empty());
    assert_eq!(node(&result, "middle-19999").active_circuits, 2);
    assert_eq!(
        node(&result, "group1").calculated,
        rate(80_000_000, 10_000_000)
    );
}

fn with_floor(r: &RegistryDocument, floor: RateBps) -> RegistryDocument {
    let mut json = serde_json::to_value(r).unwrap();
    json["catalog"]["sources"][2]["origin"]["Subscribers"]["group"]["floor"] =
        serde_json::to_value(floor).unwrap();
    serde_json::from_value(json).unwrap()
}

#[test]
fn an_explicit_source_floor_raises_local_capacity_without_changing_the_calculation() {
    let r = with_floor(&registry(), rate(60_000_000, 8_000_000));
    let result = run(&r, 1);
    let group = node(&result, "group1");
    assert_eq!(group.calculated, rate(40_000_000, 5_000_000));
    assert_eq!(group.rates.local, rate(60_000_000, 8_000_000));
    assert_eq!(group.rates.ceiling, group.rates.local);
    assert!(result.conflicts.is_empty());
}

#[test]
fn a_floor_cannot_increase_the_wan_or_bypass_a_backstop() {
    for backstop in [false, true] {
        let mut r = with_floor(&registry(), rate(60_000_000, 8_000_000));
        let c = r.catalog.as_mut().unwrap();
        if backstop {
            if let SourceOrigin::Subscribers { group, .. } = &mut c.sources[2].origin {
                group.backstop = Some(BackstopPolicy {
                    base: rate(30_000_000, 6_000_000),
                    factor: frac(1, 1),
                });
            }
        } else {
            c.nodes[0].capacity = fixed(rate(30_000_000, 6_000_000));
        }
        let result = run(&r, 1);
        assert!(result.candidate.is_none());
        assert_eq!(
            node(&result, "group1").rates.ceiling,
            rate(30_000_000, 6_000_000)
        );
        assert!(result.conflicts.iter().any(|c| c.node_id == "group1"
            && matches!(
                c.reason,
                AggregateConflictReason::FloorExceedsCeiling { .. }
            )));
    }
}

#[test]
fn a_floor_does_not_create_an_empty_automatic_branch() {
    let r = with_floor(&registry(), rate(60_000_000, 8_000_000));
    let result = run(&r, 0);
    assert!(result.omitted_nodes.contains(&"group1".into()));
    assert!(result.conflicts.is_empty());
}

#[test]
fn manual_capacity_does_not_inherit_an_automatic_backstop_or_floor() {
    let mut r = with_floor(&registry(), rate(60_000_000, 8_000_000));
    r.catalog.as_mut().unwrap().nodes[2].capacity = fixed(rate(20_000_000, 4_000_000));
    let result = run(&r, 1);
    assert_eq!(
        node(&result, "group1").rates.local,
        rate(20_000_000, 4_000_000)
    );
    assert!(result.conflicts.is_empty());
}

#[test]
fn optional_floor_policy_loads_old_schema_two_and_rejects_zero() {
    let r = registry();
    let old = serde_json::to_value(&r).unwrap();
    assert!(
        old["catalog"]["sources"][2]["origin"]["Subscribers"]["group"]
            .get("floor")
            .is_none()
    );
    let decoded: RegistryDocument = serde_json::from_value(old).unwrap();
    assert_eq!(decoded, r);
    let invalid = with_floor(&r, rate(0, 1));
    assert!(invalid.validate().is_err());
}

#[test]
fn floor_policy_survives_a_checked_private_store_restart() {
    use crate::topology_manager::store::RegistryStore;
    let r = with_floor(&registry(), rate(60_000_000, 8_000_000));
    let dir = tempfile::tempdir().unwrap();
    let mut store = RegistryStore::open(dir.path()).unwrap();
    store.commit(0, r).unwrap();
    drop(store);
    let store = RegistryStore::open(dir.path()).unwrap();
    let result = run(store.document().unwrap(), 1);
    assert_eq!(
        node(&result, "group1").rates.local,
        rate(60_000_000, 8_000_000)
    );
}

#[test]
fn upload_minimum_conflicts_do_not_change_download_rates() {
    let mut r = registry();
    r.catalog.as_mut().unwrap().profiles[0].speed.up_min =
        RuleValue::Override(RateRule::FixedBps(9_000_000));
    let result = run(&r, 12);
    assert!(result.conflicts.iter().any(|c| c.node_id == "wan1" && matches!(c.reason,
        AggregateConflictReason::MinimumExceedsCeiling { minimum, ceiling } if minimum.up > ceiling.up && minimum.down < ceiling.down)));
    assert!(
        result
            .placements
            .iter()
            .all(|p| p.rates.max.down == 50_000_000)
    );
}

#[test]
fn warning_tracks_policy_maximums_separately_from_base_plan_demand() {
    let mut r = registry();
    let c = r.catalog.as_mut().unwrap();
    c.nodes[2].capacity = fixed(rate(100_000_000, 20_000_000));
    c.profiles[0].speed.down_max = RuleValue::Override(RateRule::FixedBps(80_000_000));
    let result = run(&r, 2);
    let warning = result
        .warnings
        .iter()
        .find(|w| w.node_id == "group1")
        .unwrap();
    assert!(!warning.plan_over.down);
    assert!(warning.maximum_over.down);
    assert!(result.candidate.is_some());
}

#[test]
fn count_unit_and_maximum_demand_overflow_fail_closed() {
    let mut r = registry();
    if let SourceOrigin::Subscribers { scope, group, .. } =
        &mut r.catalog.as_mut().unwrap().sources[2].origin
    {
        scope.kind = SourceKind::Dhcp;
        group.formula = GroupFormula::CountTimesUnit(rate(u64::MAX, u64::MAX));
    }
    assert!(evaluate(placements(&r, vec![observed(0), observed(1)])).is_err());
    let mut r = registry();
    r.catalog.as_mut().unwrap().profiles[0].speed.down_max =
        RuleValue::Override(RateRule::FixedBps(u64::MAX));
    assert!(evaluate(placements(&r, vec![observed(0), observed(1)])).is_err());
}

#[test]
fn confirmed_absence_recomputes_capacity_from_the_remaining_fresh_circuits() {
    let first = run(&registry(), 10);
    let r = first.candidate.unwrap();
    let result = run(&r, 3);
    assert_eq!(node(&result, "group1").direct_circuits, 3);
    assert_eq!(
        node(&result, "group1").calculated,
        rate(120_000_000, 15_000_000)
    );
    assert_eq!(result.candidate.unwrap().subscribers.len(), 10);
}

#[test]
fn node_and_source_order_do_not_change_aggregate_results() {
    let r = registry();
    let first = run(&r, 10);
    let mut reversed = r.clone();
    let c = reversed.catalog.as_mut().unwrap();
    c.nodes.reverse();
    c.sources.reverse();
    c.profiles.reverse();
    c.routers.reverse();
    let second = run(&reversed, 10);
    assert_eq!(first.nodes, second.nodes);
    assert_eq!(first.warnings, second.warnings);
    assert_eq!(first.conflicts, second.conflicts);
    assert_eq!(first.omitted_nodes, second.omitted_nodes);
}

#[test]
fn a_stage_input_without_current_endpoint_evidence_cannot_supply_a_candidate() {
    let mut input = placements(&registry(), vec![observed(0)]);
    input.active[0].endpoints.clear();
    assert!(evaluate(input).is_err());
    let mut input = placements(&registry(), vec![observed(0)]);
    input.active[0].accounts.clear();
    assert!(evaluate(input).is_err());
}

#[test]
fn conflict_order_stays_canonical_when_multiple_checks_fail_for_one_node() {
    let mut r = with_floor(&registry(), rate(400_000_000, 120_000_000));
    r.catalog.as_mut().unwrap().profiles[0].speed.down_min =
        RuleValue::Override(RateRule::FixedBps(40_000_000));
    let first = run(&r, 10);
    r.catalog.as_mut().unwrap().nodes.reverse();
    let second = run(&r, 10);
    assert_eq!(first.conflicts, second.conflicts);
    assert_eq!(first.conflicts.len(), 3);
}

#[test]
fn explicit_node_minimums_reserve_parent_budget_and_never_double_count_descendants() {
    let mut r = registry();
    let catalog = r.catalog.as_mut().unwrap();
    let group = catalog.nodes.iter_mut().find(|n| n.id == "group1").unwrap();
    group.minimum = super::super::rates::NodeMinimum {
        down: Some(20_000_000),
        up: Some(3_000_000),
    };
    let result = run(&r, 1);
    assert!(result.conflicts.is_empty());
    assert_eq!(
        node(&result, "group1").minimum_commitment,
        rate(20_000_000, 3_000_000)
    );
    assert_eq!(
        node(&result, "wan1").minimum_commitment,
        rate(20_000_000, 3_000_000)
    );
    let wan = r
        .catalog
        .as_mut()
        .unwrap()
        .nodes
        .iter_mut()
        .find(|n| n.id == "wan1")
        .unwrap();
    wan.minimum = super::super::rates::NodeMinimum {
        down: Some(25_000_000),
        up: Some(4_000_000),
    };
    let result = run(&r, 1);
    assert!(result.conflicts.is_empty());
    assert_eq!(
        node(&result, "wan1").minimum_commitment,
        rate(25_000_000, 4_000_000)
    );
}

#[test]
fn explicit_minimums_cannot_understate_subscribers_or_exceed_ceilings() {
    for (down, up) in [(1_000_000, 1_000_000), (90_000_000, 11_000_000)] {
        let mut r = registry();
        let group = r
            .catalog
            .as_mut()
            .unwrap()
            .nodes
            .iter_mut()
            .find(|n| n.id == "group1")
            .unwrap();
        group.minimum = super::super::rates::NodeMinimum {
            down: Some(down),
            up: Some(up),
        };
        let result = run(&r, 2);
        assert!(!result.conflicts.is_empty());
        assert!(result.candidate.is_none());
    }
}

#[test]
fn sibling_minimums_must_fit_the_wan_even_when_each_child_fits() {
    let mut r = registry();
    let c = r.catalog.as_mut().unwrap();
    for child in &mut c.nodes[2..] {
        child.parent = Some("wan1".into());
        child.capacity = fixed(rate(200_000_000, 50_000_000));
        child.minimum.down = Some(160_000_000);
    }
    c.nodes[3].source_id = None;
    let result = run(&r, 0);
    assert!(result.candidate.is_none());
    assert_eq!(node(&result, "wan1").minimum_commitment.down, 320_000_000);
    assert!(result.conflicts.iter().any(|c| c.node_id == "wan1"));
}

#[test]
fn an_explicit_parent_guarantee_must_cover_child_guarantees() {
    let mut r = registry();
    let c = r.catalog.as_mut().unwrap();
    c.nodes[2].minimum.down = Some(20_000_000);
    c.nodes[0].minimum.down = Some(19_000_000);
    let result = run(&r, 1);
    assert!(result.candidate.is_none());
    assert!(result.conflicts.iter().any(|c| c.node_id == "wan1"
        && matches!(
            c.reason,
            AggregateConflictReason::MinimumBelowDescendants { .. }
        )));
}
