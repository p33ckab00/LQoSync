//! Compatibility proof only. The live builder still owns the current output.

use lqosync3::config::Config;
use lqosync3::sources::GroupKey;
use lqosync3::speed::{Rate, circuit_rates, fmt_rate};
use lqosync3::topology::{GroupTotal, RouterGroups, build_nodes, group_node_name, to_network_json};
use lqosync3::topology_manager::rates::{
    AggregateRule, Fraction, RateBps, RateRule, SpeedOverrides, SpeedPolicy, resolve_aggregate,
    resolve_subscriber,
};
use std::collections::BTreeMap;

#[test]
fn aggregate_stage_preserves_legacy_ppp_factors_backstop_and_explicit_floor() {
    use lqosync3::topology_manager::aggregate::evaluate;
    use lqosync3::topology_manager::catalog::*;
    use lqosync3::topology_manager::layout::NodeKind;
    use lqosync3::topology_manager::model::{RegistryDocument, SourceKind, SourceScope};
    use lqosync3::topology_manager::observation::{
        AccountObservation, Endpoint, ObservationGate, ObservedPolicy, SourceRead, reconcile,
    };
    use lqosync3::topology_manager::resolver::resolve;

    for (cap, floor_mbps) in [(false, 0.128), (true, 0.128), (false, 80.0)] {
        let mut cfg = config();
        cfg.bandwidth_defaults.safe_min_mbps = floor_mbps;
        cfg.backstop_caps.pppoe.enabled = cap;
        cfg.backstop_caps.pppoe.download_mbps = 30.0;
        cfg.backstop_caps.pppoe.upload_mbps = 20.0;
        cfg.backstop_caps.pppoe.multiplier = 0.5;
        let legacy = rendered_group(
            &cfg,
            GroupKey::PppPlan("Gold".into()),
            GroupTotal {
                count: 3,
                sum: rate(150.0, 150.0),
                unit: rate(50.0, 50.0),
            },
        );
        assert_eq!(cfg.bandwidth_defaults.pppoe_aggregate_factor, 0.35);
        let scope = SourceScope {
            router_id: "Hub-1".into(),
            source_id: "ppp-service".into(),
            kind: SourceKind::Ppp,
        };
        let base = RateBps::try_from(rate(50.0, 50.0)).unwrap();
        let registry = RegistryDocument {
            catalog: Some(TopologyCatalog {
                routers: vec![CatalogRouter {
                    id: "Hub-1".into(),
                    name: "Hub-1".into(),
                    roles: vec![RouterRole::Ppp],
                    ..Default::default()
                }],
                sources: vec![CatalogSource {
                    excluded: false,
                    id: "source".into(),
                    name: "Source".into(),
                    origin: SourceOrigin::Subscribers {
                        scope: scope.clone(),
                        profile_ref: None,
                        group: GroupDefaults {
                            allocation: Default::default(),
                            formula: GroupFormula::PlanSum,
                            factors: Factors {
                                down: fraction(35, 100),
                                up: fraction(35, 100),
                            },
                            backstop: cap.then_some(BackstopPolicy {
                                base: RateBps::try_from(rate(30.0, 20.0)).unwrap(),
                                factor: fraction(1, 2),
                            }),
                            floor: Some(RateBps::try_from(rate(floor_mbps, floor_mbps)).unwrap()),
                        },
                    },
                    profile_id: Some("plan".into()),
                    targets: vec!["group".into()],
                }],
                profiles: vec![SubscriberProfile {
                    id: "plan".into(),
                    name: "Plan".into(),
                    base: BaseRateAuthority::Local(base),
                    speed: SpeedOverrides::default(),
                    eligible_targets: None,
                }],
                nodes: vec![CatalogNode {
                    address_list: None,
                    tree_excluded: false,
                    router_id: None,
                    geo: None,
                    minimum: Default::default(),
                    id: "group".into(),
                    name: "Group".into(),
                    parent: None,
                    source_id: Some("source".into()),
                    kind: NodeKind::SubscriberGroup,
                    capacity: CapacityPolicy::Automatic {
                        factors: FactorChoice::Inherit,
                    },
                }],
                ..Default::default()
            }),
            ..Default::default()
        };
        let accounts = (1..=3)
            .map(|i| AccountObservation {
                account_ref: format!("account-{i}"),
                policy: ObservedPolicy::default(),
                endpoints: vec![Endpoint {
                    session_id: format!("session-{i}"),
                    ips: vec![format!("192.0.2.{i}").parse().unwrap()],
                    mac: None,
                }],
            })
            .collect();
        let snapshot = ObservationGate::default()
            .accept(
                1,
                std::slice::from_ref(&scope),
                vec![SourceRead::Complete {
                    scope: scope.clone(),
                    accounts,
                }],
            )
            .unwrap();
        let result = evaluate(resolve(reconcile(&registry, snapshot).unwrap()).unwrap()).unwrap();
        assert!(result.conflicts.is_empty());
        assert_eq!(
            result.nodes[0].rates.local,
            RateBps::try_from(legacy).unwrap()
        );
        assert_eq!(result.nodes[0].rates.ceiling, result.nodes[0].rates.local);
    }
}

fn rate(down: f64, up: f64) -> Rate {
    Rate { down, up }
}

fn fraction(numerator: u64, denominator: u64) -> Fraction {
    Fraction {
        numerator,
        denominator,
    }
}

fn config() -> Config {
    Config::from_toml_str(include_str!("scenarios/root_promotion/config.toml")).unwrap()
}

fn rendered_group(cfg: &Config, key: GroupKey, total: GroupTotal) -> Rate {
    let name = group_node_name(cfg, "Hub-1", &key);
    let online = [RouterGroups {
        router: "Hub-1".into(),
        ppp_enabled: true,
        per_plan_node: true,
        totals: BTreeMap::from([(key, total)]),
    }];
    let nodes = build_nodes(cfg, true, &online, &BTreeMap::new(), &[]).unwrap();
    let json = to_network_json(&nodes, None, &BTreeMap::new());
    rate(
        json[&name]["downloadBandwidthMbps"].as_f64().unwrap(),
        json[&name]["uploadBandwidthMbps"].as_f64().unwrap(),
    )
}

#[test]
fn ppp_and_dhcp_basis_fixtures_match_current_rendering_and_golden_output() {
    let cfg = config();
    let golden: serde_json::Value = serde_json::from_str(include_str!(
        "scenarios/root_promotion/expected/network.json"
    ))
    .unwrap();
    let fixtures = [
        (
            GroupKey::PppPlan("Gold".into()),
            GroupTotal {
                count: 3,
                sum: rate(150.0, 150.0),
                unit: rate(50.0, 50.0),
            },
            rate(150.0, 150.0),
            fraction(35, 100),
            fraction(35, 100),
            rate(52.5, 52.5),
        ),
        (
            GroupKey::PppPlan("Silver".into()),
            GroupTotal {
                count: 1,
                sum: rate(50.0, 5.0),
                unit: rate(9.0, 9.0),
            },
            rate(50.0, 5.0),
            fraction(35, 100),
            fraction(35, 100),
            rate(17.5, 1.75),
        ),
        (
            GroupKey::DhcpSite("LAN".into()),
            GroupTotal {
                count: 2,
                sum: rate(999.0, 999.0),
                unit: rate(50.0, 50.0),
            },
            rate(100.0, 100.0),
            fraction(1, 1),
            fraction(1, 1),
            rate(100.0, 100.0),
        ),
        (
            GroupKey::DhcpSite("IOT".into()),
            GroupTotal {
                count: 1,
                sum: rate(999.0, 999.0),
                unit: rate(20.0, 10.0),
            },
            rate(20.0, 10.0),
            fraction(1, 1),
            fraction(1, 1),
            rate(20.0, 10.0),
        ),
    ];
    for (key, total, basis, down, up, expected) in fixtures {
        let legacy = rendered_group(&cfg, key.clone(), total);
        assert_eq!(legacy, expected);
        let name = group_node_name(&cfg, "Hub-1", &key);
        assert_eq!(
            golden[&name]["downloadBandwidthMbps"].as_f64(),
            Some(expected.down)
        );
        assert_eq!(
            golden[&name]["uploadBandwidthMbps"].as_f64(),
            Some(expected.up)
        );
        let resolved = resolve_aggregate(
            &AggregateRule::Automatic { down, up },
            RateBps::try_from(basis).unwrap(),
            None,
            None,
        )
        .unwrap();
        let converted = Rate::try_from(resolved.ceiling).unwrap();
        assert_eq!(converted, legacy);
        assert_eq!(fmt_rate(converted.down), fmt_rate(legacy.down));
        assert_eq!(fmt_rate(converted.up), fmt_rate(legacy.up));
    }
}

#[test]
fn hotspot_uses_count_times_unit_and_independent_factors() {
    let mut cfg = config();
    cfg.bandwidth_defaults.hotspot_download_factor = 0.8;
    cfg.bandwidth_defaults.hotspot_upload_factor = 0.6;
    cfg.backstop_caps.hotspot.enabled = false;
    let legacy = rendered_group(
        &cfg,
        GroupKey::Hotspot,
        GroupTotal {
            count: 3,
            sum: rate(999.0, 999.0),
            unit: rate(7.0, 3.0),
        },
    );
    assert_eq!(legacy, rate(16.8, 5.4));
    let resolved = resolve_aggregate(
        &AggregateRule::Automatic {
            down: fraction(4, 5),
            up: fraction(3, 5),
        },
        RateBps::try_from(rate(21.0, 9.0)).unwrap(),
        None,
        None,
    )
    .unwrap();
    assert_eq!(Rate::try_from(resolved.ceiling).unwrap(), legacy);
}

#[test]
fn configured_backstop_limits_preserve_local_target_and_rendered_ceiling() {
    let mut cfg = config();
    cfg.backstop_caps.pppoe.enabled = true;
    cfg.backstop_caps.pppoe.download_mbps = 100.0;
    cfg.backstop_caps.pppoe.upload_mbps = 10.0;
    cfg.backstop_caps.pppoe.multiplier = 0.5;
    let legacy = rendered_group(
        &cfg,
        GroupKey::Ppp,
        GroupTotal {
            count: 10,
            sum: rate(500.0, 500.0),
            unit: rate(50.0, 50.0),
        },
    );
    assert_eq!(legacy, rate(50.0, 5.0));
    let resolved = resolve_aggregate(
        &AggregateRule::Automatic {
            down: fraction(35, 100),
            up: fraction(35, 100),
        },
        RateBps::try_from(rate(500.0, 500.0)).unwrap(),
        None,
        Some(RateBps::try_from(rate(50.0, 5.0)).unwrap()),
    )
    .unwrap();
    assert_eq!(Rate::try_from(resolved.local).unwrap(), rate(175.0, 175.0));
    assert_eq!(Rate::try_from(resolved.ceiling).unwrap(), legacy);
}

#[test]
fn migration_preserves_resolved_minima_and_floors_instead_of_new_defaults() {
    for base in [rate(50.0, 5.0), rate(0.5, 0.5), rate(0.1, 0.1)] {
        let legacy = circuit_rates(base, 0.1, 0.128, 0.2);
        let min = RateBps::try_from(rate(legacy.down_min, legacy.up_min)).unwrap();
        let max = RateBps::try_from(rate(legacy.down_max, legacy.up_max)).unwrap();
        let migrated = SpeedPolicy {
            down_min: RateRule::FixedBps(min.down),
            up_min: RateRule::FixedBps(min.up),
            down_max: RateRule::FixedBps(max.down),
            up_max: RateRule::FixedBps(max.up),
        };
        let resolved = resolve_subscriber(
            RateBps::try_from(base).unwrap(),
            &migrated,
            &SpeedOverrides::default(),
        )
        .unwrap();
        let actual_min = Rate::try_from(resolved.min).unwrap();
        let actual_max = Rate::try_from(resolved.max).unwrap();
        assert_eq!(
            (
                fmt_rate(actual_min.down),
                fmt_rate(actual_min.up),
                fmt_rate(actual_max.down),
                fmt_rate(actual_max.up)
            ),
            (
                fmt_rate(legacy.down_min),
                fmt_rate(legacy.up_min),
                fmt_rate(legacy.down_max),
                fmt_rate(legacy.up_max)
            )
        );
    }
}

#[test]
fn empty_legacy_group_floor_is_an_explicit_migration_target() {
    let cfg = config();
    let legacy = rendered_group(
        &cfg,
        GroupKey::Ppp,
        GroupTotal {
            count: 0,
            sum: rate(0.0, 0.0),
            unit: rate(0.0, 0.0),
        },
    );
    assert_eq!(legacy, rate(0.128, 0.128));
    let migrated = RateBps::try_from(legacy).unwrap();
    let resolved = resolve_aggregate(
        &AggregateRule::Fixed(migrated),
        RateBps { down: 0, up: 0 },
        None,
        None,
    )
    .unwrap();
    assert_eq!(Rate::try_from(resolved.ceiling).unwrap(), legacy);
    // New empty automatic branches are omitted by a later export adapter, not given a floor here.
}

#[test]
fn legacy_boundary_rejects_invalid_or_lossy_rates() {
    for value in [
        f64::NAN,
        f64::INFINITY,
        f64::NEG_INFINITY,
        -1.0,
        0.0,
        0.000_000_1,
        1.000_000_1,
        1e20,
    ] {
        assert!(RateBps::try_from(rate(value, 50.0)).is_err(), "{value}");
        assert!(RateBps::try_from(rate(50.0, value)).is_err(), "{value}");
    }
    assert!(
        Rate::try_from(RateBps {
            down: u64::MAX,
            up: 1
        })
        .is_err()
    );
    assert!(Rate::try_from(RateBps { down: 1, up: 0 }).is_err());
}

#[test]
fn decimal_mbps_round_trips_preserve_integer_bps() {
    for value in [
        0.000_001,
        0.128,
        0.2,
        1.000_001,
        17.5,
        157.5,
        310.0,
        100_000.123_456,
    ] {
        let input = rate(value, value);
        let converted = RateBps::try_from(input).unwrap();
        assert_eq!(Rate::try_from(converted).unwrap(), input);
        assert_eq!(
            RateBps::try_from(Rate::try_from(converted).unwrap()).unwrap(),
            converted
        );
    }
}
