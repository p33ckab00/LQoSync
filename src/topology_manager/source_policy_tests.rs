use super::super::normalize::AuthorityEvidence;
use super::super::observation::ProfileObservation;
use super::super::rates::RateBps;
use super::*;
use crate::config::{RateLimitOrder, UnitlessUnit};

fn enabled() -> RateRead {
    RateRead::RateLimit(RateFormat {
        order: RateLimitOrder::RouterosRxTx,
        unitless: UnitlessUnit::Bps,
    })
}
fn binding(name: &str, reference: &str, rate: RateRead) -> ProfileBinding {
    ProfileBinding {
        source_ref: reference.into(),
        current_name: name.into(),
        rate,
    }
}
fn profile(row_ref: &str, name: &str, rate: Option<&str>) -> CurrentProfile {
    CurrentProfile {
        kind: SourceKind::Ppp,
        row_ref: row_ref.into(),
        name: name.into(),
        rate_limit: rate.map(str::to_owned),
        comment: None,
        active: true,
    }
}
fn index(profiles: &[CurrentProfile]) -> SourceIndex<'_> {
    SourceIndex {
        services: HashMap::new(),
        services_by_ref: HashMap::new(),
        profiles_by_ref: profiles
            .iter()
            .map(|p| ((p.kind, p.row_ref.as_str()), p))
            .collect(),
        profiles: profiles
            .iter()
            .map(|p| ((p.kind, p.name.as_str()), p))
            .collect(),
    }
}
fn account(profile_ref: Option<&str>, direct: Option<&str>) -> CurrentAccount {
    CurrentAccount {
        kind: if profile_ref.is_some() {
            SourceKind::Ppp
        } else {
            SourceKind::Dhcp
        },
        row_ref: "*a".into(),
        authority: AuthorityEvidence::LocalAccount {
            name: "user".into(),
            comment: None,
        },
        profile_ref: profile_ref.map(str::to_owned),
        direct_rate_limit: direct.map(str::to_owned),
        service_refs: vec![],
        endpoints: vec![],
        active_comments: vec![],
    }
}

#[test]
fn explicit_name_selection_maps_current_locator_to_stable_reference() {
    let profiles = vec![profile("*new", "gold", Some("2M/50M"))];
    let policy = SourcePolicy {
        profiles: vec![binding("gold", "stable-gold", RateRead::Disabled)],
        ..Default::default()
    };
    let source_index = index(&profiles);
    let compiled = policy.compile(SourceKind::Ppp, &source_index).unwrap();
    assert!(compiled.has_reference("stable-gold"));
    let observed = compiled.observe(&account(Some("*new"), None)).unwrap();
    assert_eq!(
        observed.profile,
        ProfileObservation::Known(Some("stable-gold".into()))
    );
    assert!(observed.plan.is_none());
}
#[test]
fn recreated_profile_uses_fresh_rate_and_keeps_only_the_explicit_stable_reference() {
    let policy = SourcePolicy {
        profiles: vec![binding("gold", "stable-gold", enabled())],
        ..Default::default()
    };
    for (row_ref, text, down) in [
        ("*old", "2M/10M", 10_000_000),
        ("*new", "3M/50M", 50_000_000),
    ] {
        let profiles = vec![profile(row_ref, "gold", Some(text))];
        let source_index = index(&profiles);
        let compiled = policy.compile(SourceKind::Ppp, &source_index).unwrap();
        let observed = compiled.observe(&account(Some(row_ref), None)).unwrap();
        assert_eq!(
            observed.profile,
            ProfileObservation::Known(Some("stable-gold".into()))
        );
        assert_eq!(observed.plan.unwrap().down, down);
    }
}
#[test]
fn renamed_profile_without_revised_selector_cannot_reuse_old_locator_mapping() {
    let profiles = vec![profile("stable-gold", "renamed", Some("2M/10M"))];
    let policy = SourcePolicy {
        profiles: vec![binding("gold", "stable-gold", enabled())],
        ..Default::default()
    };
    let source_index = index(&profiles);
    let compiled = policy.compile(SourceKind::Ppp, &source_index).unwrap();
    assert_eq!(
        compiled.observe(&account(Some("stable-gold"), None)),
        Err(SourcePolicyIssue::MissingProfileMapping)
    );
}
#[test]
fn blank_or_ambiguous_bindings_fail_before_any_account_can_be_accepted() {
    let profiles = vec![profile("*p", "gold", None)];
    for bindings in [
        vec![binding("", "ref", RateRead::Disabled)],
        vec![binding("gold", "", RateRead::Disabled)],
        vec![
            binding("gold", "ref", RateRead::Disabled),
            binding("gold", "other", RateRead::Disabled),
        ],
        vec![
            binding("gold", "ref", RateRead::Disabled),
            binding("silver", "ref", RateRead::Disabled),
        ],
    ] {
        assert!(
            SourcePolicy {
                profiles: bindings,
                ..Default::default()
            }
            .compile(SourceKind::Ppp, &index(&profiles))
            .is_err()
        );
    }
}
#[test]
fn disabled_reads_ignore_malformed_rate_input_without_creating_evidence() {
    let profiles = vec![profile("*p", "gold", Some("bad-private-rate"))];
    let policy = SourcePolicy {
        profiles: vec![binding("gold", "ref", RateRead::Disabled)],
        ..Default::default()
    };
    assert!(
        policy
            .compile(SourceKind::Ppp, &index(&profiles))
            .unwrap()
            .observe(&account(Some("*p"), None))
            .unwrap()
            .plan
            .is_none()
    );
}
#[test]
fn enabled_reads_keep_missing_rates_absent_and_invalid_rates_typed() {
    let policy = SourcePolicy {
        profiles: vec![binding("gold", "ref", enabled())],
        ..Default::default()
    };
    for text in [None, Some("bad-private-rate")] {
        let profiles = vec![profile("*p", "gold", text)];
        let result = policy
            .compile(SourceKind::Ppp, &index(&profiles))
            .unwrap()
            .observe(&account(Some("*p"), None));
        if text.is_none() {
            assert!(result.unwrap().plan.is_none());
        } else {
            assert_eq!(result, Err(SourcePolicyIssue::InvalidSourceRate));
        }
    }
}
#[test]
fn unprofiled_dhcp_uses_only_its_explicit_direct_rate_mode() {
    let profiles = vec![];
    let source_index = index(&profiles);
    let compiled = SourcePolicy {
        unprofiled_rate: enabled(),
        ..Default::default()
    }
    .compile(SourceKind::Dhcp, &source_index)
    .unwrap();
    let observed = compiled.observe(&account(None, Some("2M/10M"))).unwrap();
    assert_eq!(observed.profile, ProfileObservation::Known(None));
    assert_eq!(
        observed.plan,
        Some(RateBps {
            down: 10_000_000,
            up: 2_000_000
        })
    );
}
#[test]
fn mixed_local_and_source_profiles_keep_separate_read_modes_in_one_scope() {
    let profiles = vec![
        profile("*local", "local", Some("invalid")),
        profile("*source", "source", Some("2M/10M")),
    ];
    let policy = SourcePolicy {
        profiles: vec![
            binding("local", "local-ref", RateRead::Disabled),
            binding("source", "source-ref", enabled()),
        ],
        ..Default::default()
    };
    let source_index = index(&profiles);
    let compiled = policy.compile(SourceKind::Ppp, &source_index).unwrap();
    assert!(
        compiled
            .observe(&account(Some("*local"), None))
            .unwrap()
            .plan
            .is_none()
    );
    assert_eq!(
        compiled
            .observe(&account(Some("*source"), None))
            .unwrap()
            .plan,
        Some(RateBps {
            down: 10_000_000,
            up: 2_000_000
        })
    );
}

#[test]
fn converted_priority_uses_current_account_and_profile_data_without_another_authority() {
    use crate::topology_manager::legacy_rate::LegacyPriority;
    let speed = crate::config::Speed {
        rate_limit_order: RateLimitOrder::RouterosRxTx,
        pppoe_sources: vec![
            crate::config::PppSource::SecretComment,
            crate::config::PppSource::ProfileRateLimit,
        ],
        ..Default::default()
    };
    let mut policy = SourcePolicy {
        profiles: vec![binding("gold", "stable", RateRead::Disabled)],
        legacy_priority: Some(LegacyPriority::ppp(&speed, "10M/2M").unwrap()),
        ..Default::default()
    };
    let profiles = vec![profile("*p", "gold", Some("3M/60M"))];
    let index = index(&profiles);
    let compiled = policy.compile(SourceKind::Ppp, &index).unwrap();
    let mut account = account(Some("*p"), None);
    if let AuthorityEvidence::LocalAccount { comment, .. } = &mut account.authority {
        *comment = Some("Plan 25M/5M".into());
    }
    assert_eq!(
        compiled.observe(&account).unwrap().plan.unwrap(),
        RateBps {
            down: 25_000_000,
            up: 5_000_000
        }
    );
    if let AuthorityEvidence::LocalAccount { comment, .. } = &mut account.authority {
        *comment = None;
    }
    assert_eq!(
        compiled.observe(&account).unwrap().plan.unwrap().down,
        60_000_000
    );
    policy.profiles[0].rate = enabled();
    assert!(policy.compile(SourceKind::Ppp, &index).is_err());
}

#[test]
fn dhcp_overrides_require_unique_references_positions_and_positive_rates() {
    let binding = DhcpOverrideBinding {
        legacy_client_index: 0,
        account_ref: "operator-comment/v1/customer-1".into(),
        rate: RateBps {
            down: 20_000_000,
            up: 7_000_000,
        },
    };
    let policy = SourcePolicy {
        dhcp_overrides: vec![binding.clone()],
        ..Default::default()
    };
    assert!(policy.validate(SourceKind::Dhcp).is_ok());
    assert!(policy.validate(SourceKind::Ppp).is_err());
    assert!(policy.validate(SourceKind::Hotspot).is_err());
    for case in 0..4 {
        let mut changed = policy.clone();
        match case {
            0 => changed.dhcp_overrides[0].account_ref.clear(),
            1 => changed.dhcp_overrides[0].rate.down = 0,
            2 => {
                let mut another = binding.clone();
                another.account_ref = "another".into();
                changed.dhcp_overrides.push(another);
            }
            _ => {
                let mut another = binding.clone();
                another.legacy_client_index = 1;
                changed.dhcp_overrides.push(another);
            }
        }
        assert!(changed.validate(SourceKind::Dhcp).is_err(), "case {case}");
    }
}

#[test]
fn authorized_override_precedes_source_rate_parsing_and_never_matches_a_locator() {
    let policy = SourcePolicy {
        dhcp_overrides: vec![DhcpOverrideBinding {
            legacy_client_index: 0,
            account_ref: "verified".into(),
            rate: RateBps {
                down: 20_000_000,
                up: 7_000_000,
            },
        }],
        unprofiled_rate: enabled(),
        ..Default::default()
    };
    let source_index = index(&[]);
    let compiled = policy.compile(SourceKind::Dhcp, &source_index).unwrap();
    let account = account(None, Some("malformed-private-rate"));
    assert_eq!(
        compiled
            .observe_identified(&account, "verified")
            .unwrap()
            .plan,
        Some(RateBps {
            down: 20_000_000,
            up: 7_000_000
        })
    );
    assert_eq!(
        compiled.observe_identified(&account, &account.row_ref),
        Err(SourcePolicyIssue::InvalidSourceRate)
    );
}

fn unknown_setup() -> Vec<CurrentProfile> {
    vec![
        profile("*gold", "gold", Some("2M/50M")),
        profile("*silver", "silver", Some("5M/10M")),
        profile("*bronze", "bronze", None),
    ]
}

fn bound() -> SourcePolicy {
    SourcePolicy {
        profiles: vec![binding("gold", "stable-gold", RateRead::Disabled)],
        ..Default::default()
    }
}

#[test]
fn an_unknown_profile_with_a_fallback_reads_its_rate_limit() {
    let profiles = unknown_setup();
    let source_index = index(&profiles);
    let compiled = bound()
        .compile(SourceKind::Ppp, &source_index)
        .unwrap()
        .accept_unknown();
    let observed = compiled.observe(&account(Some("*silver"), None)).unwrap();
    assert_eq!(
        observed.profile,
        ProfileObservation::Known(Some("unbound:silver".into()))
    );
    let expected =
        super::super::source_rate::parse_rate_limit(Some("5M/10M"), UNKNOWN_FORMAT).unwrap();
    assert_eq!(observed.plan, expected);
    assert!(observed.plan.is_some());
}

#[test]
fn an_unknown_profile_without_rate_limit_gives_no_plan() {
    let profiles = unknown_setup();
    let source_index = index(&profiles);
    let compiled = bound()
        .compile(SourceKind::Ppp, &source_index)
        .unwrap()
        .accept_unknown();
    assert_eq!(
        compiled
            .observe(&account(Some("*bronze"), None))
            .unwrap()
            .plan,
        None
    );
}

#[test]
fn an_unknown_profile_without_a_fallback_is_still_missing() {
    let profiles = unknown_setup();
    let source_index = index(&profiles);
    let compiled = bound().compile(SourceKind::Ppp, &source_index).unwrap();
    assert_eq!(
        compiled
            .observe(&account(Some("*silver"), None))
            .unwrap_err(),
        SourcePolicyIssue::MissingProfileMapping
    );
}

#[test]
fn known_profiles_are_unchanged() {
    let profiles = unknown_setup();
    let source_index = index(&profiles);
    let compiled = bound()
        .compile(SourceKind::Ppp, &source_index)
        .unwrap()
        .accept_unknown();
    assert_eq!(
        compiled
            .observe(&account(Some("*gold"), None))
            .unwrap()
            .profile,
        ProfileObservation::Known(Some("stable-gold".into()))
    );
}
