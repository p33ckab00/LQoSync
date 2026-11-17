use super::*;
use crate::topology_manager::rates::RateRule;
use crate::topology_manager::{
    catalog,
    identity::{ReferenceAuthority, SourceSelection},
    publication::{prepare, tests::input},
    settings::SourceSettings,
    source_policy::SourcePolicy,
};
use std::time::Duration;

fn fixture() -> (Config, RegistryDocument) {
    let cfg = Config::from_toml_str("[speed]\npppoe_sources=['default']\n[bandwidth_defaults]\npppoe_default_rate='50M/10M'\nmin_rate_percentage=0.1\npppoe_aggregate_factor=0.8\n[[routers]]\nname='Distro'\naddress='192.0.2.1'\nusername='u'\npassword_file='/not-used'\n[routers.pppoe]\nenabled=true\nper_plan_node=true\n").unwrap();
    let mut registry = input(0).reconciliation.registry;
    let catalog = registry.catalog.as_mut().unwrap();
    catalog.defaults.down_min = RateRule::BaseFraction(Fraction {
        numerator: 1,
        denominator: 10,
    });
    catalog.defaults.up_min = catalog.defaults.down_min;
    if let catalog::SourceOrigin::Subscribers { group, .. } = &mut catalog.sources[2].origin {
        group.factors.up = group.factors.down;
    }
    let scope = catalog.required_scopes().unwrap()[0].clone();
    registry.source_settings = Some(SourceSettings {
        selections: vec![SourceSelection {
            scope,
            service_names: vec![],
            authority: ReferenceAuthority::OperatorManagedComment,
            policy: SourcePolicy {
                profiles: vec![crate::topology_manager::source_policy::ProfileBinding {
                    source_ref: "provider-profile-id".into(),
                    current_name: "gold".into(),
                    rate: crate::topology_manager::source_policy::RateRead::Disabled,
                }],
                legacy_priority: Some(
                    LegacyPriority::ppp(&cfg.speed, &cfg.bandwidth_defaults.pppoe_default_rate)
                        .unwrap(),
                ),
                ..Default::default()
            },
        }],
    });
    (cfg, registry)
}
fn has(issues: &[PolicyIssue], kind: PolicyIssueKind) -> bool {
    issues.iter().any(|i| i.kind == kind)
}

#[test]
fn valid_source_filters_are_supported_and_malformed_filters_block() {
    for global in [true, false] {
        let (mut cfg, registry) = fixture();
        let exclude = if global {
            &mut cfg.exclude
        } else {
            &mut cfg.routers[0].exclude
        };
        exclude.ips.push("192.0.2.0/24".into());
        assert!(source_issues(&cfg, &registry).unwrap().is_empty());
        let exclude = if global {
            &mut cfg.exclude
        } else {
            &mut cfg.routers[0].exclude
        };
        exclude.ips.push("invalid-network".into());
        assert!(has(
            &source_issues(&cfg, &registry).unwrap(),
            PolicyIssueKind::IpExclusions
        ));
    }
    let (mut cfg, registry) = fixture();
    cfg.exclude.ppp_profiles.push("management-*".into());
    assert!(source_issues(&cfg, &registry).unwrap().is_empty());
    cfg.exclude.ppp_profiles.push(String::new());
    assert!(has(
        &source_issues(&cfg, &registry).unwrap(),
        PolicyIssueKind::PppProfileExclusions
    ));
    let (mut cfg, registry) = fixture();
    cfg.exclude.dhcp_servers.push("mgmt".into());
    assert!(source_issues(&cfg, &registry).unwrap().is_empty());
}

#[test]
fn matching_local_policy_has_no_configuration_blockers() {
    let (cfg, registry) = fixture();
    assert_eq!(legacy_issues(&cfg, &registry).unwrap(), vec![]);
}

#[test]
fn dormant_local_profiles_cannot_hide_minimum_or_maximum_policy_changes() {
    let (mut cfg, registry) = fixture();
    cfg.bandwidth_defaults.min_rate_percentage = 0.5;
    assert!(has(
        &legacy_issues(&cfg, &registry).unwrap(),
        PolicyIssueKind::SubscriberBounds
    ));
    let (mut cfg, registry) = fixture();
    cfg.bandwidth_defaults.max_floor_mbps = 100.0;
    assert!(has(
        &legacy_issues(&cfg, &registry).unwrap(),
        PolicyIssueKind::SubscriberBounds
    ));
}

#[test]
fn source_priorities_and_dynamic_authority_need_structural_proof() {
    let (mut cfg, registry) = fixture();
    cfg.speed
        .pppoe_sources
        .insert(0, crate::config::PppSource::SecretComment);
    let issues = legacy_issues(&cfg, &registry).unwrap();
    assert!(has(&issues, PolicyIssueKind::SourcePriority));
    assert!(has(&issues, PolicyIssueKind::BaseAuthority));
    let (cfg, mut registry) = fixture();
    registry.catalog.as_mut().unwrap().profiles[0].base = catalog::BaseRateAuthority::SourceRead;
    assert!(has(
        &legacy_issues(&cfg, &registry).unwrap(),
        PolicyIssueKind::DynamicSubscriberBounds
    ));
}

#[test]
fn group_factors_caps_and_minimums_are_checked_without_current_members() {
    for case in 0..4 {
        let (mut cfg, mut registry) = fixture();
        let kind = match case {
            0 => {
                cfg.bandwidth_defaults.pppoe_aggregate_factor = 0.5;
                PolicyIssueKind::GroupFactors
            }
            1 => {
                cfg.backstop_caps.pppoe.enabled = true;
                PolicyIssueKind::GroupBackstop
            }
            2 => {
                cfg.nodes.ppp_plan.download_min_mbps = Some(10);
                PolicyIssueKind::GroupMinimum
            }
            _ => {
                registry.catalog.as_mut().unwrap().nodes[2].capacity =
                    catalog::CapacityPolicy::Fixed {
                        base: RateBps {
                            down: 100_000_000,
                            up: 20_000_000,
                        },
                        factors: catalog::FactorChoice::Inherit,
                    };
                PolicyIssueKind::GroupCapacity
            }
        };
        assert!(has(&legacy_issues(&cfg, &registry).unwrap(), kind));
    }
}

#[test]
fn dhcp_endpoint_overrides_require_identity_policy_even_when_no_matching_lease_is_online() {
    let (mut cfg, mut registry) = fixture();
    let kind = SourceKind::Dhcp;
    cfg.routers[0].dhcp.enabled = true;
    cfg.routers[0].pppoe.enabled = false;
    cfg.routers[0].dhcp.clients.push(crate::config::DhcpClient {
        mac: "AA:BB:CC:DD:EE:01".into(),
        download_mbps: 20.0,
        upload_mbps: 10.0,
        name: None,
    });
    if let catalog::SourceOrigin::Subscribers {
        scope, profile_ref, ..
    } = &mut registry.catalog.as_mut().unwrap().sources[2].origin
    {
        scope.kind = kind;
        *profile_ref = None;
    }
    let selection = &mut registry.source_settings.as_mut().unwrap().selections[0];
    selection.scope.kind = kind;
    selection.policy = SourcePolicy::default();
    let issues = source_issues(&cfg, &registry).unwrap();
    assert!(has(&issues, PolicyIssueKind::DhcpAddressOverrides));
    assert!(!format!("{issues:?}").contains("AA:BB:CC:DD:EE:01"));
}

#[tokio::test(start_paused = true)]
async fn configured_adoption_requires_both_policy_and_exact_snapshot_checks() {
    let (cfg, registry) = fixture();
    let mut observed = input(1);
    observed.reconciliation.registry.catalog = registry.catalog;
    observed.reconciliation.registry.source_settings = registry.source_settings;
    let pair = prepare(observed).unwrap();
    let mut legacy_rows = pair.rows().to_vec();
    legacy_rows[0].device_id = "legacy-device".into();
    legacy_rows[0].circuit_id = "legacy-circuit".into();
    let rows = legacy_rows.as_slice();
    let manifest = super::super::migration::MigrationManifest {
        expected_revision: pair.expected_revision(),
        generation: pair.generation(),
        baseline_digest: super::super::migration::fingerprint(rows, pair.network()),
        candidate_digest: super::super::migration::fingerprint(pair.rows(), pair.network()),
        rows: rows
            .iter()
            .map(|r| super::super::migration::RowMapping {
                legacy: super::super::migration::RowIdentity {
                    circuit_id: r.circuit_id.clone(),
                    device_id: r.device_id.clone(),
                },
                current: super::super::migration::RowIdentity {
                    circuit_id: pair.rows()[0].circuit_id.clone(),
                    device_id: pair.rows()[0].device_id.clone(),
                },
                legacy_circuit_name: r.circuit_name.clone(),
                legacy_device_name: r.device_name.clone(),
            })
            .collect(),
        nodes: vec![],
        protected_rows: vec![],
    };
    super::super::ownership::adopt_configured_legacy(
        &cfg,
        &pair,
        rows,
        pair.network(),
        &manifest,
        &[],
    )
    .unwrap();
    let mut filtered = cfg.clone();
    filtered.exclude.ips.push("198.51.100.0/24".into());
    assert!(
        super::super::ownership::adopt_configured_legacy(
            &filtered,
            &pair,
            rows,
            pair.network(),
            &manifest,
            &[]
        )
        .is_err()
    );
    let mut changed = cfg.clone();
    changed.bandwidth_defaults.min_rate_percentage = 0.5;
    assert!(
        super::super::ownership::adopt_configured_legacy(
            &changed,
            &pair,
            rows,
            pair.network(),
            &manifest,
            &[]
        )
        .is_err()
    );
    let mut stale = manifest.clone();
    stale.generation += 1;
    assert!(
        super::super::ownership::adopt_configured_legacy(
            &cfg,
            &pair,
            rows,
            pair.network(),
            &stale,
            &[]
        )
        .is_err()
    );
    tokio::time::advance(Duration::from_secs(11)).await;
    assert!(
        super::super::ownership::adopt_configured_legacy(
            &cfg,
            &pair,
            rows,
            pair.network(),
            &manifest,
            &[]
        )
        .is_err()
    );
}

fn protocol_fixture(kind: SourceKind) -> (Config, RegistryDocument) {
    let (mut cfg, mut registry) = fixture();
    let router = &mut cfg.routers[0];
    router.pppoe.enabled = false;
    router.dhcp.enabled = kind == SourceKind::Dhcp;
    router.dhcp.require_bound = true;
    router.dhcp.shape_unassigned = false;
    router.hotspot.enabled = kind == SourceKind::Hotspot;
    let selection = &mut registry.source_settings.as_mut().unwrap().selections[0];
    selection.scope.kind = kind;
    if kind == SourceKind::Dhcp {
        use crate::config::{DhcpMode, DhcpServer, DhcpSource};
        router.dhcp.server_mode = DhcpMode::PerPlan;
        router.dhcp.servers = vec![DhcpServer {
            name: "lan".into(),
            mode: DhcpMode::PerPlan,
            speed_sources: Some(vec![DhcpSource::Settings]),
            default_plan_down_mbps: Some(50.0),
            default_plan_up_mbps: Some(10.0),
            download_factor: Some(0.8),
            upload_factor: Some(0.8),
        }];
        selection.service_names = vec!["lan".into()];
        selection.policy.profiles.clear();
        selection.policy.legacy_priority = Some(
            LegacyPriority::dhcp(&cfg.speed, &cfg.bandwidth_defaults, &router.dhcp.servers[0])
                .unwrap(),
        );
    } else {
        router.hotspot.download_limit_mbps = Some(50.0);
        router.hotspot.upload_limit_mbps = Some(10.0);
        router.hotspot.download_factor = Some(0.8);
        router.hotspot.upload_factor = Some(0.8);
        selection.policy.legacy_priority = Some(
            LegacyPriority::hotspot(&cfg.speed, &cfg.bandwidth_defaults, &router.hotspot).unwrap(),
        );
    }
    if let catalog::SourceOrigin::Subscribers {
        scope,
        profile_ref,
        group,
    } = &mut registry.catalog.as_mut().unwrap().sources[2].origin
    {
        scope.kind = kind;
        if kind == SourceKind::Dhcp {
            *profile_ref = None;
        } else {
            group.formula = catalog::GroupFormula::CountTimesUnit(RateBps {
                down: 50_000_000,
                up: 10_000_000,
            });
        }
    }
    (cfg, registry)
}

#[test]
fn hotspot_and_explicit_dhcp_policies_are_checked_without_current_sessions() {
    for kind in [SourceKind::Hotspot, SourceKind::Dhcp] {
        let (cfg, registry) = protocol_fixture(kind);
        assert_eq!(legacy_issues(&cfg, &registry).unwrap(), vec![]);
        let mut changed = registry.clone();
        if let catalog::SourceOrigin::Subscribers { group, .. } =
            &mut changed.catalog.as_mut().unwrap().sources[2].origin
        {
            group.formula = if kind == SourceKind::Hotspot {
                catalog::GroupFormula::CountTimesUnit(RateBps {
                    down: 10_000_000,
                    up: 10_000_000,
                })
            } else {
                catalog::GroupFormula::CountTimesUnit(RateBps {
                    down: 50_000_000,
                    up: 10_000_000,
                })
            };
        }
        assert!(has(
            &legacy_issues(&cfg, &changed).unwrap(),
            PolicyIssueKind::GroupFormula
        ));
    }
}

#[test]
fn dhcp_membership_and_server_rules_cannot_be_silently_dropped() {
    let (mut cfg, registry) = protocol_fixture(SourceKind::Dhcp);
    cfg.routers[0].dhcp.discover = false;
    assert!(source_issues(&cfg, &registry).unwrap().is_empty());
    let mut broad = registry.clone();
    broad.source_settings.as_mut().unwrap().selections[0]
        .service_names
        .clear();
    assert!(has(
        &source_issues(&cfg, &broad).unwrap(),
        PolicyIssueKind::DhcpMembership
    ));
    cfg.routers[0].dhcp.require_bound = false;
    assert!(has(
        &source_issues(&cfg, &registry).unwrap(),
        PolicyIssueKind::DhcpMembership
    ));
    cfg.routers[0].dhcp.require_bound = true;
    cfg.routers[0].dhcp.shape_unassigned = true;
    assert!(has(
        &source_issues(&cfg, &registry).unwrap(),
        PolicyIssueKind::DhcpMembership
    ));
    cfg.routers[0].exclude.dhcp_servers.push(String::new());
    assert!(has(
        &source_issues(&cfg, &registry).unwrap(),
        PolicyIssueKind::DhcpServerExclusions
    ));
}

#[test]
fn dhcp_service_policies_that_differ_require_separate_scopes() {
    let (mut cfg, mut registry) = protocol_fixture(SourceKind::Dhcp);
    let mut other = cfg.routers[0].dhcp.servers[0].clone();
    other.name = "lan2".into();
    other.default_plan_down_mbps = Some(20.0);
    cfg.routers[0].dhcp.servers.push(other);
    registry.source_settings.as_mut().unwrap().selections[0]
        .service_names
        .push("lan2".into());
    assert!(has(
        &legacy_issues(&cfg, &registry).unwrap(),
        PolicyIssueKind::DhcpServicePolicy
    ));
}

#[test]
fn factors_require_exact_rational_equality_and_do_not_use_float_tolerance() {
    let (cfg, mut registry) = fixture();
    if let catalog::SourceOrigin::Subscribers { group, .. } =
        &mut registry.catalog.as_mut().unwrap().sources[2].origin
    {
        group.factors.down = Fraction {
            numerator: 8_000_000_000_000_000_001,
            denominator: 10_000_000_000_000_000_000,
        };
    }
    assert!(has(
        &legacy_issues(&cfg, &registry).unwrap(),
        PolicyIssueKind::GroupFactors
    ));
}
