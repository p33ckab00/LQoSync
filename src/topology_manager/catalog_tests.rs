use super::*;

#[test]
fn web_capacity_uses_true_wan_bases_and_checks_overflow() {
    use crate::status::WanCapacity;
    let mut c = catalog();
    let sum = WanCapacity::from_catalog(&c, 9).unwrap();
    let mut down = 0;
    let mut up = 0;
    for node in c.nodes.iter_mut().filter(|n| n.kind == NodeKind::Wan) {
        if let CapacityPolicy::Fixed { base, factors } = &mut node.capacity {
            down += base.down;
            up += base.up;
            *factors = FactorChoice::Override(Factors {
                down: fraction(1, 2),
                up: fraction(1, 2),
            });
        }
    }
    assert_eq!(sum.down_bps, down);
    assert_eq!(sum.up_bps, up);
    assert_eq!(sum.wan_count, 2);
    assert_eq!(sum.wans.len(), 2);
    assert_eq!(sum.wans.iter().map(|w| w.down_bps).sum::<u64>(), down);
    assert_eq!(sum.wans.iter().map(|w| w.up_bps).sum::<u64>(), up);
    let names = c
        .nodes
        .iter()
        .filter(|n| n.kind == NodeKind::Wan)
        .map(|n| n.name.as_str());
    assert!(sum.wans.iter().map(|w| w.name.as_str()).eq(names));
    assert_eq!(sum.registry_revision, 9);
    assert_eq!(WanCapacity::from_catalog(&c, 9), Some(sum));
    assert!(WanCapacity::from_catalog(&TopologyCatalog::default(), 0).is_none());
    for node in c.nodes.iter_mut().filter(|n| n.kind == NodeKind::Wan) {
        node.capacity = CapacityPolicy::Fixed {
            base: rate(u64::MAX, u64::MAX),
            factors: FactorChoice::Inherit,
        };
    }
    assert!(WanCapacity::from_catalog(&c, 10).is_none());
}
use crate::topology_manager::model::{IdentityRef, SourceKind, SubscriberPolicy};
use crate::topology_manager::rates::{AggregateRule, RateRule, RuleValue, resolve_aggregate};

fn fraction(numerator: u64, denominator: u64) -> Fraction {
    Fraction {
        numerator,
        denominator,
    }
}
fn rate(down: u64, up: u64) -> RateBps {
    RateBps { down, up }
}
fn scope() -> SourceScope {
    SourceScope {
        router_id: "distro".into(),
        source_id: "ppp-service".into(),
        kind: SourceKind::Ppp,
    }
}
fn automatic() -> CapacityPolicy {
    CapacityPolicy::Automatic {
        factors: FactorChoice::Inherit,
    }
}
fn fixed(base: RateBps) -> CapacityPolicy {
    CapacityPolicy::Fixed {
        base,
        factors: FactorChoice::Inherit,
    }
}

pub(crate) fn catalog() -> TopologyCatalog {
    let wan = |id: &str, source: &str, base| CatalogNode {
        geo: None,
        minimum: Default::default(),
        id: id.into(),
        name: id.into(),
        parent: None,
        source_id: Some(source.into()),
        kind: NodeKind::Wan,
        capacity: fixed(base),
    };
    let group = |id: &str, parent: &str| CatalogNode {
        geo: None,
        minimum: Default::default(),
        id: id.into(),
        name: "Shared label".into(),
        parent: Some(parent.into()),
        source_id: Some("profile-source".into()),
        kind: NodeKind::SubscriberGroup,
        capacity: automatic(),
    };
    TopologyCatalog {
        routers: vec![
            CatalogRouter {
                id: "core".into(),
                name: "Core".into(),
                roles: vec![RouterRole::Wan],
            },
            CatalogRouter {
                id: "distro".into(),
                name: "Distro".into(),
                roles: vec![RouterRole::Ppp, RouterRole::Dhcp, RouterRole::Hotspot],
            },
        ],
        sources: vec![
            CatalogSource {
                id: "wan-source-1".into(),
                name: "WAN".into(),
                origin: SourceOrigin::Wan {
                    router_id: "core".into(),
                    endpoint_ref: "endpoint-1".into(),
                },
                profile_id: None,
                targets: Vec::new(),
            },
            CatalogSource {
                id: "wan-source-2".into(),
                name: "WAN".into(),
                origin: SourceOrigin::Wan {
                    router_id: "core".into(),
                    endpoint_ref: "endpoint-2".into(),
                },
                profile_id: None,
                targets: Vec::new(),
            },
            CatalogSource {
                id: "profile-source".into(),
                name: "One shared profile".into(),
                origin: SourceOrigin::Subscribers {
                    scope: scope(),
                    profile_ref: Some("provider-profile-id".into()),
                    group: GroupDefaults {
                        floor: None,
                        formula: GroupFormula::PlanSum,
                        factors: Factors {
                            down: fraction(4, 5),
                            up: fraction(1, 2),
                        },
                        backstop: Some(BackstopPolicy {
                            base: rate(500_000_000, 100_000_000),
                            factor: fraction(9, 10),
                        }),
                    },
                },
                profile_id: Some("local-plan".into()),
                targets: vec!["group1".into(), "group2".into()],
            },
        ],
        profiles: vec![SubscriberProfile {
            id: "local-plan".into(),
            name: "Gold".into(),
            base: BaseRateAuthority::Local(rate(50_000_000, 10_000_000)),
            speed: SpeedOverrides::default(),
            eligible_targets: None,
        }],
        nodes: vec![
            wan("wan1", "wan-source-1", rate(310_000_000, 100_000_000)),
            wan("wan2", "wan-source-2", rate(620_000_000, 200_000_000)),
            group("group1", "wan1"),
            group("group2", "wan2"),
        ],
        ..TopologyCatalog::default()
    }
}

pub(crate) fn subscriber(target: &str) -> SubscriberRecord {
    SubscriberRecord {
        id: "subscriber-id".into(),
        identities: vec![IdentityRef {
            scope: scope(),
            account_ref: "account-id".into(),
        }],
        policy: SubscriberPolicy {
            profile_id: Some("local-plan".into()),
            assignment: Some(target.into()),
            pin: Some(target.into()),
        },
    }
}

#[test]
fn one_profile_supports_multiple_wans_with_distinct_instances_and_equal_labels() {
    let c = catalog();
    c.validate().unwrap();
    c.validate_references(&[subscriber("group1")]).unwrap();
    assert_eq!(c.layout().unwrap().len(), 4);
    assert_eq!(c.required_scopes().unwrap(), vec![scope()]);
    assert_eq!(c.nodes[2].source_id, c.nodes[3].source_id);
    assert_ne!(c.nodes[2].id, c.nodes[3].id);
}

#[test]
fn profile_only_and_wan_only_topologies_keep_their_explicit_structure() {
    let mut c = catalog();
    c.nodes = vec![CatalogNode {
        geo: None,
        minimum: Default::default(),
        parent: None,
        ..c.nodes[2].clone()
    }];
    c.sources.retain(|s| s.id == "profile-source");
    c.sources[0].targets = vec!["group1".into()];
    c.validate().unwrap();
    let mut c = catalog();
    c.nodes.truncate(2);
    c.sources[2].targets = vec!["wan1".into(), "wan2".into()];
    c.validate_references(&[subscriber("wan2")]).unwrap();
}

#[test]
fn factor_overrides_replace_shared_values_and_preserve_manual_bases() {
    let mut c = catalog();
    let inherited = c.layout().unwrap();
    assert_eq!(
        inherited[2].capacity,
        AggregateRule::Automatic {
            down: fraction(4, 5),
            up: fraction(1, 2)
        }
    );
    c.nodes[2].capacity = CapacityPolicy::Fixed {
        base: rate(200_000_000, 100_000_000),
        factors: FactorChoice::Override(Factors {
            down: fraction(1, 2),
            up: fraction(1, 4),
        }),
    };
    let compiled = c.layout().unwrap();
    assert_eq!(
        compiled[2].capacity,
        AggregateRule::Fixed(rate(100_000_000, 25_000_000))
    );
    assert!(matches!(
        c.nodes[2].capacity,
        CapacityPolicy::Fixed {
            base: RateBps {
                down: 200_000_000,
                ..
            },
            ..
        }
    ));
    assert_eq!(
        rate(200_000_000, 100_000_000)
            .apply_factors(fraction(1, 2), fraction(1, 4))
            .unwrap(),
        rate(100_000_000, 25_000_000)
    );
}

#[test]
fn wan_factors_keep_true_capacity_and_reject_more_than_one_hundred_percent() {
    let mut c = catalog();
    c.nodes[0].capacity = CapacityPolicy::Fixed {
        base: rate(310_000_000, 100_000_000),
        factors: FactorChoice::Override(Factors {
            down: fraction(9, 10),
            up: fraction(4, 5),
        }),
    };
    assert_eq!(
        c.layout().unwrap()[0].capacity,
        AggregateRule::Fixed(rate(279_000_000, 80_000_000))
    );
    c.nodes[0].capacity = CapacityPolicy::Fixed {
        base: rate(310_000_000, 100_000_000),
        factors: FactorChoice::Override(Factors {
            down: fraction(11, 10),
            up: fraction(1, 1),
        }),
    };
    assert!(c.validate().is_err());
}

#[test]
fn wan_percentages_do_not_compound_group_defaults() {
    let mut c = catalog();
    c.nodes[2].capacity = CapacityPolicy::WanPercentage {
        down: fraction(1, 2),
        up: fraction(1, 2),
    };
    let rule = c.layout().unwrap()[2].capacity;
    let resolved = resolve_aggregate(
        &rule,
        rate(0, 0),
        Some(rate(310_000_000, 100_000_000)),
        Some(rate(100_000_000, 80_000_000)),
    )
    .unwrap();
    assert_eq!(resolved.local, rate(155_000_000, 50_000_000));
    assert_eq!(resolved.ceiling, rate(100_000_000, 50_000_000));
    c.nodes[2].parent = None;
    assert!(c.validate().is_err());
}

#[test]
fn custom_group_defaults_are_independent_of_discovered_group_factors() {
    let mut c = catalog();
    c.nodes[2].source_id = None;
    assert_eq!(
        c.layout().unwrap()[2].capacity,
        AggregateRule::Automatic {
            down: fraction(1, 1),
            up: fraction(1, 1)
        }
    );
}

#[test]
fn duplicate_ids_origins_and_source_parent_instances_are_rejected() {
    for table in 0..5 {
        let mut c = catalog();
        match table {
            0 => c.routers.push(c.routers[0].clone()),
            1 => c.sources.push(c.sources[0].clone()),
            2 => c.profiles.push(c.profiles[0].clone()),
            3 => c.nodes.push(c.nodes[0].clone()),
            _ => {
                let mut source = c.sources[0].clone();
                source.id = "duplicate-origin".into();
                c.sources.push(source);
            }
        }
        assert!(c.validate().is_err());
    }
    let mut c = catalog();
    let mut node = c.nodes[2].clone();
    node.id = "another-instance".into();
    c.nodes.push(node);
    assert!(c.validate().is_err());
}

#[test]
fn selected_sources_require_known_routers_and_matching_read_roles() {
    let mut c = catalog();
    c.routers[1].roles = vec![RouterRole::Wan];
    assert!(c.validate().is_err());
    let mut c = catalog();
    c.routers[1].id = "missing-router".into();
    assert!(c.validate().is_err());
    let mut c = catalog();
    c.nodes[2].source_id = Some("missing-source".into());
    assert!(c.validate().is_err());
    let mut c = catalog();
    c.nodes[2].source_id = Some("wan-source-1".into());
    assert!(c.validate().is_err());
    let mut c = catalog();
    c.nodes[0].source_id = None;
    assert!(c.validate().is_err());
}

#[test]
fn invalid_parent_graphs_and_destinations_are_rejected_without_promotion() {
    let mut c = catalog();
    c.nodes[2].parent = Some("missing-parent".into());
    assert!(c.validate().is_err());
    let mut c = catalog();
    c.sources[2].targets = vec!["wan1".into()];
    assert!(c.validate().is_err());
    let mut c = catalog();
    c.sources[2].targets = vec!["missing-node".into()];
    assert!(c.validate().is_err());
    let mut c = catalog();
    c.profiles[0].eligible_targets = Some(Vec::new());
    assert!(c.validate().is_err());
}

#[test]
fn pins_require_known_eligible_instances_and_profile_references() {
    let c = catalog();
    assert!(
        c.validate_references(&[subscriber("missing-node")])
            .is_err()
    );
    let mut record = subscriber("group1");
    record.policy.profile_id = Some("missing-plan".into());
    assert!(c.validate_references(&[record]).is_err());
    let mut c = catalog();
    c.profiles[0].eligible_targets = Some(vec!["group1".into()]);
    assert!(c.validate_references(&[subscriber("group2")]).is_err());
    let mut c = catalog();
    c.sources[2].targets = vec!["group2".into()];
    assert!(c.validate_references(&[subscriber("group1")]).is_err());
}

#[test]
fn rate_authority_is_explicit_and_invalid_speed_controls_are_rejected() {
    let mut c = catalog();
    c.profiles[0].base = BaseRateAuthority::SourceRead;
    c.validate().unwrap();
    let restored: TopologyCatalog =
        serde_json::from_str(&serde_json::to_string(&c).unwrap()).unwrap();
    assert_eq!(restored.profiles[0].base, BaseRateAuthority::SourceRead);
    c.profiles[0].speed.down_min = RuleValue::Override(RateRule::BaseFraction(fraction(2, 1)));
    assert!(c.validate().is_err());
    let mut c = catalog();
    c.profiles[0].base = BaseRateAuthority::Local(rate(500_000, 500_000));
    assert!(c.validate().is_err());
    let mut c = catalog();
    c.defaults.up_max = RateRule::BaseFraction(fraction(1, 0));
    assert!(c.validate().is_err());
}

#[test]
fn extra_wan_inventory_does_not_expand_existing_targets_or_instances() {
    let mut c = catalog();
    let targets = c.sources[2].targets.clone();
    let nodes = c.nodes.clone();
    c.sources.push(CatalogSource {
        id: "new-wan".into(),
        name: "Candidate WAN".into(),
        origin: SourceOrigin::Wan {
            router_id: "core".into(),
            endpoint_ref: "new-endpoint".into(),
        },
        profile_id: None,
        targets: Vec::new(),
    });
    c.validate().unwrap();
    assert_eq!(c.sources[2].targets, targets);
    assert_eq!(c.nodes, nodes);
}

#[test]
fn nested_unknown_endpoint_fields_are_not_accepted_as_policy_data() {
    let mut json = serde_json::to_value(catalog()).unwrap();
    json["profiles"][0]["base"]["Local"]["ipv4"] = serde_json::json!("192.0.2.1");
    assert!(serde_json::from_value::<TopologyCatalog>(json).is_err());
}

#[test]
fn a_large_catalog_uses_iterative_validation() {
    let mut c = TopologyCatalog {
        nodes: (0..10_000)
            .map(|i| CatalogNode {
                geo: None,
                minimum: Default::default(),
                id: format!("group-{i}"),
                name: "Same label".into(),
                parent: None,
                source_id: None,
                kind: NodeKind::SubscriberGroup,
                capacity: automatic(),
            })
            .collect(),
        ..TopologyCatalog::default()
    };
    c.validate().unwrap();
    assert_eq!(c.layout().unwrap().len(), 10_000);
    c.routers.push(CatalogRouter {
        id: "distro".into(),
        name: "Distro".into(),
        roles: vec![RouterRole::Ppp],
    });
    c.sources.push(CatalogSource {
        id: "subscribers".into(),
        name: "Source".into(),
        origin: SourceOrigin::Subscribers {
            scope: scope(),
            profile_ref: None,
            group: GroupDefaults {
                floor: None,
                formula: GroupFormula::PlanSum,
                factors: Factors::default(),
                backstop: None,
            },
        },
        profile_id: None,
        targets: c.nodes.iter().map(|node| node.id.clone()).collect(),
    });
    let subscribers: Vec<_> = (0..10_000)
        .map(|i| SubscriberRecord {
            id: format!("subscriber-{i}"),
            identities: vec![IdentityRef {
                scope: scope(),
                account_ref: format!("account-{i}"),
            }],
            policy: SubscriberPolicy {
                profile_id: None,
                assignment: Some(format!("group-{i}")),
                pin: None,
            },
        })
        .collect();
    c.validate_references(&subscribers).unwrap();
}

#[test]
fn source_backed_group_formulas_preserve_protocol_and_group_basis() {
    let mut c = catalog();
    let source = &mut c.sources[2];
    if let SourceOrigin::Subscribers { group, .. } = &mut source.origin {
        group.formula = GroupFormula::CountTimesUnit(rate(50_000_000, 10_000_000));
    }
    assert!(c.validate().is_err());

    for kind in [SourceKind::Dhcp, SourceKind::Hotspot] {
        let mut c = catalog();
        if let SourceOrigin::Subscribers {
            scope,
            profile_ref,
            group,
        } = &mut c.sources[2].origin
        {
            scope.kind = kind;
            *profile_ref = None;
            group.formula = GroupFormula::CountTimesUnit(rate(50_000_000, 10_000_000));
        }
        c.validate().unwrap();
        if let SourceOrigin::Subscribers { group, .. } = &mut c.sources[2].origin {
            group.formula = GroupFormula::PlanSum;
        }
        if kind == SourceKind::Hotspot {
            assert!(c.validate().is_err());
        } else {
            c.validate().unwrap();
        }
    }
    let mut c = catalog();
    if let SourceOrigin::Subscribers { scope, .. } = &mut c.sources[2].origin {
        scope.kind = SourceKind::Dhcp;
    }
    c.validate().unwrap();
}

#[test]
fn imported_dynamic_controls_reject_impossible_factors_and_floors_without_live_rates() {
    for floors in [false, true] {
        let mut c = catalog();
        c.profiles[0].base = BaseRateAuthority::SourceRead;
        c.profiles[0].speed.down_min = RuleValue::Override(RateRule::LegacyRoundedBase {
            factor: fraction(1, if floors { 10 } else { 2 }),
            floor_bps: if floors { 300_000 } else { 100_000 },
        });
        c.profiles[0].speed.down_max = RuleValue::Override(RateRule::LegacyRoundedBase {
            factor: fraction(1, if floors { 1 } else { 4 }),
            floor_bps: 200_000,
        });
        assert!(c.validate().is_err(), "floors {floors}");
    }
}

#[test]
fn node_minimums_require_positive_exact_whole_mbps_and_keep_absent_policy_compatible() {
    let original = catalog();
    let text = serde_json::to_string(&original).unwrap();
    assert!(!text.contains("minimum"));
    assert_eq!(
        serde_json::from_str::<TopologyCatalog>(&text).unwrap(),
        original
    );
    for value in [0, 1, 1_500_000, u64::MAX] {
        let mut c = original.clone();
        c.nodes[2].minimum.down = Some(value);
        assert!(c.validate().is_err(), "{value}");
    }
}

pub(crate) fn with_fallback_node() -> TopologyCatalog {
    let mut c = catalog();
    c.nodes.push(CatalogNode {
        geo: None,
        minimum: Default::default(),
        id: "fb".into(),
        name: "Fallback".into(),
        parent: Some("wan1".into()),
        source_id: None,
        kind: NodeKind::SubscriberGroup,
        capacity: automatic(),
    });
    c
}

#[test]
fn a_registry_without_fallback_round_trips_unchanged() {
    let c = catalog();
    let text = serde_json::to_string(&c).unwrap();
    assert!(!text.contains("fallback"), "{text}");
    let back: TopologyCatalog = serde_json::from_str(&text).unwrap();
    assert_eq!(back, c);
    assert_eq!(serde_json::to_string(&back).unwrap(), text);
}

#[test]
fn fallback_validation_rules() {
    let policy = |ids: &[&str], plan: RateBps| FallbackPolicy {
        node_ids: ids.iter().map(|s| s.to_string()).collect(),
        default_plan: plan,
    };
    let ten = rate(10_000_000, 10_000_000);
    let mut c = with_fallback_node();
    c.fallback = Some(policy(&["fb"], ten));
    c.validate().unwrap();
    for bad in [
        policy(&[], ten),
        policy(&["nope"], ten),
        policy(&["group1"], ten),
        policy(&["wan1"], ten),
        policy(&["fb", "fb"], ten),
        policy(&["fb"], rate(0, 10_000_000)),
    ] {
        let mut c = with_fallback_node();
        c.fallback = Some(bad.clone());
        assert!(c.validate().is_err(), "{bad:?}");
    }
}
