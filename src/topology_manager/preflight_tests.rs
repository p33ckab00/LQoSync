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

fn dhcp_override_fixture() -> (Config, RegistryDocument) {
    let (mut cfg, mut registry) = fixture();
    cfg.routers[0].dhcp.enabled = true;
    cfg.routers[0].dhcp.require_bound = true;
    cfg.routers[0].dhcp.shape_unassigned = false;
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
        scope.kind = SourceKind::Dhcp;
        *profile_ref = None;
    }
    registry.catalog.as_mut().unwrap().profiles[0].base = BaseRateAuthority::SourceRead;
    let selection = &mut registry.source_settings.as_mut().unwrap().selections[0];
    selection.scope.kind = SourceKind::Dhcp;
    selection.policy = serde_json::from_value(serde_json::json!({
        "profiles": [], "unprofiled_rate": "Disabled",
        "dhcp_overrides": [{"legacy_client_index": 0,
            "account_ref": "operator-comment/v1/customer-1",
            "rate": {"down": 20_000_000, "up": 10_000_000}}]
    }))
    .unwrap();
    registry
        .subscribers
        .push(crate::topology_manager::model::SubscriberRecord {
            id: "enrolled-offline".into(),
            identities: vec![crate::topology_manager::model::IdentityRef {
                scope: selection.scope.clone(),
                account_ref: "operator-comment/v1/customer-1".into(),
            }],
            policy: Default::default(),
        });
    (cfg, registry)
}

#[test]
fn dhcp_identity_override_accepts_enrolled_offline_subscriber_without_mac_lookup() {
    let (mut cfg, registry) = dhcp_override_fixture();
    assert_eq!(source_issues(&cfg, &registry).unwrap(), vec![]);
    let before = source_filter_digest(&cfg, &registry).unwrap();
    assert!(before.is_some());
    cfg.routers[0].dhcp.clients[0].mac = "AA:BB:CC:DD:EE:99".into();
    assert_eq!(source_issues(&cfg, &registry).unwrap(), vec![]);
    assert_eq!(source_filter_digest(&cfg, &registry).unwrap(), before);
    cfg.routers[0].dhcp.clients[0].download_mbps = 21.0;
    assert!(has(
        &source_issues(&cfg, &registry).unwrap(),
        PolicyIssueKind::DhcpAddressOverrides
    ));
    assert_ne!(source_filter_digest(&cfg, &registry).unwrap(), before);
}

#[test]
fn dhcp_identity_override_rejects_incomplete_duplicate_or_stale_bindings() {
    for case in 0..6 {
        let (mut cfg, mut registry) = dhcp_override_fixture();
        match case {
            0 => cfg.routers[0].dhcp.clients.clear(),
            1 => {
                let client = cfg.routers[0].dhcp.clients[0].clone();
                cfg.routers[0].dhcp.clients.push(client);
            }
            2 => registry.source_settings.as_mut().unwrap().selections[0]
                .policy
                .dhcp_overrides
                .clear(),
            3 => {
                registry.source_settings.as_mut().unwrap().selections[0]
                    .policy
                    .dhcp_overrides[0]
                    .legacy_client_index = 1
            }
            4 => {
                registry.catalog.as_mut().unwrap().profiles[0].base =
                    BaseRateAuthority::Local(RateBps {
                        down: 50_000_000,
                        up: 10_000_000,
                    })
            }
            _ => {
                let policy = &mut registry.source_settings.as_mut().unwrap().selections[0].policy;
                policy.dhcp_overrides.push(policy.dhcp_overrides[0].clone());
            }
        }
        assert!(
            source_issues(&cfg, &registry).map_or(true, |issues| !issues.is_empty()),
            "case {case}"
        );
    }
}

#[test]
fn dhcp_identity_override_requires_durable_identity_and_round_trips_without_endpoints() {
    let (_, registry) = dhcp_override_fixture();
    registry.validate().unwrap();
    let text = serde_json::to_string(&registry).unwrap();
    assert!(!text.contains("AA:BB:CC"));
    let restored: RegistryDocument = serde_json::from_str(&text).unwrap();
    assert_eq!(restored, registry);
    let mut unknown = registry.clone();
    unknown.subscribers.clear();
    assert!(unknown.validate().is_err());
    let mut alias = registry.clone();
    alias.source_settings.as_mut().unwrap().selections[0]
        .policy
        .dhcp_overrides[0]
        .account_ref = "unverified".into();
    assert!(alias.validate().is_err());
}

#[test]
fn dhcp_override_settings_survive_store_reopen_and_stale_edit_is_rejected() {
    let (_, registry) = dhcp_override_fixture();
    let root = tempfile::tempdir().unwrap();
    let mut store = crate::topology_manager::store::RegistryStore::open(root.path()).unwrap();
    store.commit(0, registry).unwrap();
    let persisted = store.document().unwrap().clone();
    drop(store);
    let mut reopened = crate::topology_manager::store::RegistryStore::open(root.path()).unwrap();
    assert_eq!(reopened.document().unwrap(), &persisted);
    assert!(reopened.commit(0, persisted.clone()).is_err());
    assert_eq!(reopened.document().unwrap(), &persisted);
}

#[test]
fn dhcp_aliases_cannot_hold_conflicting_override_rates() {
    let (_, mut registry) = dhcp_override_fixture();
    let mut alias = registry.subscribers[0].identities[0].clone();
    alias.account_ref = "operator-comment/v1/alias".into();
    registry.subscribers[0].identities.push(alias.clone());
    let policy = &mut registry.source_settings.as_mut().unwrap().selections[0].policy;
    let mut binding = policy.dhcp_overrides[0].clone();
    binding.legacy_client_index = 1;
    binding.account_ref = alias.account_ref;
    policy.dhcp_overrides.push(binding);
    registry.validate().unwrap();
    registry.source_settings.as_mut().unwrap().selections[0]
        .policy
        .dhcp_overrides[1]
        .rate
        .down += 1;
    assert!(registry.validate().is_err());
}

#[test]
fn dhcp_override_coverage_is_shared_across_disjoint_service_scopes() {
    let (mut cfg, mut registry) = dhcp_override_fixture();
    cfg.routers[0].dhcp.clients.push(crate::config::DhcpClient {
        mac: "AA:BB:CC:DD:EE:02".into(),
        download_mbps: 30.0,
        upload_mbps: 11.0,
        name: None,
    });
    let selections = &mut registry.source_settings.as_mut().unwrap().selections;
    selections[0].service_names = vec!["lan1".into()];
    let mut second = selections[0].clone();
    second.scope.source_id = "dhcp-second".into();
    second.service_names = vec!["lan2".into()];
    second.policy.dhcp_overrides[0].legacy_client_index = 1;
    second.policy.dhcp_overrides[0].account_ref = "operator-comment/v1/customer-2".into();
    second.policy.dhcp_overrides[0].rate = RateBps {
        down: 30_000_000,
        up: 11_000_000,
    };
    let mut subscriber = registry.subscribers[0].clone();
    subscriber.id = "enrolled-second".into();
    subscriber.identities[0].scope = second.scope.clone();
    subscriber.identities[0].account_ref = "operator-comment/v1/customer-2".into();
    registry.subscribers.push(subscriber);
    let mut source = registry.catalog.as_ref().unwrap().sources[2].clone();
    source.id = "second-source".into();
    if let catalog::SourceOrigin::Subscribers { scope, .. } = &mut source.origin {
        *scope = second.scope.clone();
    }
    source.targets = vec!["group1".into(), "group2".into()];
    registry.catalog.as_mut().unwrap().sources.push(source);
    selections.push(second);
    assert_eq!(source_issues(&cfg, &registry).unwrap(), vec![]);
    registry.source_settings.as_mut().unwrap().selections[1]
        .policy
        .dhcp_overrides[0]
        .legacy_client_index = 0;
    assert!(has(
        &source_issues(&cfg, &registry).unwrap(),
        PolicyIssueKind::DhcpAddressOverrides
    ));
}

#[test]
fn inactive_catalog_sources_do_not_change_the_effective_override_base() {
    let (cfg, mut registry) = dhcp_override_fixture();
    let catalog = registry.catalog.as_mut().unwrap();
    let mut profile = catalog.profiles[0].clone();
    profile.id = "unused-local-plan".into();
    profile.base = BaseRateAuthority::Local(RateBps {
        down: 40_000_000,
        up: 10_000_000,
    });
    let mut source = catalog.sources[2].clone();
    source.id = "unused-source".into();
    if let catalog::SourceOrigin::Subscribers { profile_ref, .. } = &mut source.origin {
        *profile_ref = Some("unused-provider-profile".into());
    }
    source.targets.clear();
    source.profile_id = Some(profile.id.clone());
    catalog.profiles.push(profile);
    catalog.sources.push(source);
    assert_eq!(source_issues(&cfg, &registry).unwrap(), vec![]);
}

fn imported_bounds() -> crate::topology_manager::rates::SpeedOverrides {
    serde_json::from_value(serde_json::json!({
        "down_min": {"Override":{"LegacyRoundedBase":{"factor":{"numerator":1,"denominator":10},"floor_bps":128_000}}},
        "up_min": {"Override":{"LegacyRoundedBase":{"factor":{"numerator":1,"denominator":10},"floor_bps":128_000}}},
        "down_max": {"Override":{"LegacyRoundedBase":{"factor":{"numerator":1,"denominator":1},"floor_bps":200_000}}},
        "up_max": {"Override":{"LegacyRoundedBase":{"factor":{"numerator":1,"denominator":1},"floor_bps":200_000}}}
    })).unwrap()
}

#[test]
fn imported_dynamic_bounds_prove_dormant_policy_instead_of_current_sample_rates() {
    let (cfg, mut registry) = fixture();
    let profile = &mut registry.catalog.as_mut().unwrap().profiles[0];
    profile.base = BaseRateAuthority::SourceRead;
    profile.speed = imported_bounds();
    assert_eq!(legacy_issues(&cfg, &registry).unwrap(), vec![]);
}

#[test]
fn imported_dynamic_bounds_reject_changed_controls_and_config_even_when_offline() {
    for case in 0..5 {
        let (mut cfg, mut registry) = fixture();
        let profile = &mut registry.catalog.as_mut().unwrap().profiles[0];
        profile.base = BaseRateAuthority::SourceRead;
        profile.speed = imported_bounds();
        match case {
            0 => cfg.bandwidth_defaults.min_rate_percentage = 0.2,
            1 => cfg.bandwidth_defaults.safe_min_mbps = 0.256,
            2 => cfg.bandwidth_defaults.max_floor_mbps = 0.5,
            3 => {
                profile.speed.down_max = crate::topology_manager::rates::RuleValue::Override(
                    RateRule::FixedBps(50_000_000),
                )
            }
            _ => profile.speed.up_min = crate::topology_manager::rates::RuleValue::Inherit,
        }
        assert!(
            has(
                &legacy_issues(&cfg, &registry).unwrap(),
                PolicyIssueKind::DynamicSubscriberBounds
            ),
            "case {case}"
        );
    }
}

#[test]
fn inherited_imported_bounds_survive_a_revision_checked_store_edit() {
    let (cfg, mut registry) = fixture();
    let controls = crate::topology_manager::legacy_bounds::import(&cfg.bandwidth_defaults).unwrap();
    let catalog = registry.catalog.as_mut().unwrap();
    catalog.defaults = crate::topology_manager::rates::SpeedPolicy {
        down_min: controls.down_min.select(catalog.defaults.down_min),
        up_min: controls.up_min.select(catalog.defaults.up_min),
        down_max: controls.down_max.select(catalog.defaults.down_max),
        up_max: controls.up_max.select(catalog.defaults.up_max),
    };
    catalog.profiles[0].base = BaseRateAuthority::SourceRead;
    let root = tempfile::tempdir().unwrap();
    let mut store = crate::topology_manager::store::RegistryStore::open(root.path()).unwrap();
    store.commit(0, registry).unwrap();
    drop(store);
    let store = crate::topology_manager::store::RegistryStore::open(root.path()).unwrap();
    assert_eq!(
        legacy_issues(&cfg, store.document().unwrap()).unwrap(),
        vec![]
    );
    let mut changed = cfg;
    changed.nodes.ppp_plan.download_min_mbps = Some(1);
    assert!(has(
        &legacy_issues(&changed, store.document().unwrap()).unwrap(),
        PolicyIssueKind::GroupMinimum
    ));
}
