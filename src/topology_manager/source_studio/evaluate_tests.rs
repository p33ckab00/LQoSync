use super::*;
use web_contract::source_studio::{IdentityMode, WriteGrant, WriteScope};

fn cfg(extra: &str) -> Config {
    Config::from_toml_str(&format!(
        "[[routers]]\nname='Core'\naddress='192.0.2.1'\nusername='u'\npassword_file='/x'\n{extra}"
    ))
    .unwrap()
}

fn legacy() -> RateSource {
    RateSource::AccountComment {
        format: CommentRateFormat::LegacyConfigured,
    }
}

/// The default PPP order of `config/defaults.toml`, with the subscriber override first.
fn ppp_rates() -> Vec<RateSource> {
    vec![
        RateSource::SubscriberOverride,
        legacy(),
        RateSource::ActiveComment {
            format: CommentRateFormat::LegacyConfigured,
        },
        RateSource::ProfileComment {
            format: CommentRateFormat::LegacyConfigured,
        },
        RateSource::ProfileRateLimit,
        RateSource::ProfileName,
        RateSource::Settings,
    ]
}

/// The default per-site DHCP order: server comment, then settings.
fn dhcp_rates() -> Vec<RateSource> {
    vec![
        RateSource::SubscriberOverride,
        RateSource::ServerComment {
            format: CommentRateFormat::LegacyConfigured,
        },
        RateSource::Settings,
    ]
}

fn ppp() -> SourcePolicyDraft {
    SourcePolicyDraft {
        id: Some("policy-ppp".into()),
        name: "PPPoE".into(),
        enabled: true,
        priority: 10,
        source: SourceKind::Ppp,
        read_access_ref: "default".into(),
        members: MemberMode::Joined,
        filter: FilterExpr::All { rules: vec![] },
        group: GroupRule::ByField {
            field: FieldKey::AccountProfile,
        },
        identity: IdentityMode::Registry,
        rates: ppp_rates(),
        parent: ParentRule::Router,
        capacity: CapacityRule::FromPlans {
            down_factor: "0.5".into(),
            up_factor: "1".into(),
            minimum_down_bps: "0".into(),
            minimum_up_bps: "0".into(),
        },
        writes: vec![],
    }
}

fn dhcp(id: &str, priority: u16, servers: &[&str]) -> SourcePolicyDraft {
    SourcePolicyDraft {
        id: Some(id.into()),
        source: SourceKind::Dhcp,
        members: MemberMode::Active,
        priority,
        filter: FilterExpr::Match {
            field: FieldKey::LeaseServer,
            op: FilterOp::In,
            value: FilterValue::Texts {
                values: servers.iter().map(|s| s.to_string()).collect(),
            },
        },
        group: GroupRule::ByField {
            field: FieldKey::LeaseServer,
        },
        rates: dhcp_rates(),
        ..ppp()
    }
}

fn codes(issues: &[ValidationIssue]) -> Vec<(&str, &str)> {
    issues
        .iter()
        .map(|i| (i.code.as_str(), i.entity_ref.as_str()))
        .collect()
}

#[test]
fn the_default_configuration_translates_without_issues() {
    let c = cfg("");
    let (intent, issues) = translate(
        &c,
        &c.routers[0],
        &[ppp(), dhcp("policy-dhcp", 20, &["LAN"])],
    );
    assert!(issues.is_empty(), "{issues:?}");
    let ppp = intent.ppp.unwrap();
    assert!(ppp.per_plan);
    assert_eq!(ppp.policy_id, "policy-ppp");
    assert_eq!(intent.dhcp.len(), 1);
    assert_eq!(
        intent.dhcp[0].servers.as_deref(),
        Some(&["LAN".to_string()][..])
    );
}

#[test]
fn hotspot_authority_is_not_invented() {
    let c = cfg("");
    let hotspot = SourcePolicyDraft {
        id: None,
        source: SourceKind::Hotspot,
        members: MemberMode::Active,
        group: GroupRule::ByField {
            field: FieldKey::UserProfile,
        },
        ..ppp()
    };
    let (intent, issues) = translate(&c, &c.routers[0], &[hotspot]);
    assert_eq!(intent, RouterIntent::default());
    assert!(
        codes(&issues).contains(&("unsupported_in_v1", "draft:0")),
        "{issues:?}"
    );
    assert!(issues.iter().all(|i| i.severity == Severity::Blocker));
}

#[test]
fn unsupported_constructs_are_blockers_and_change_nothing() {
    let c = cfg("");
    let mut cases: Vec<SourcePolicyDraft> = Vec::new();
    let mut p = ppp();
    p.filter = FilterExpr::Match {
        field: FieldKey::AccountComment,
        op: FilterOp::Contains,
        value: FilterValue::Text {
            value: "vip".into(),
        },
    };
    cases.push(p);
    let mut p = ppp();
    p.rates.swap(1, 2);
    cases.push(p);
    let mut p = ppp();
    p.rates.remove(0);
    cases.push(p);
    let mut p = ppp();
    p.rates.push(RateSource::Fallback {
        plan_id: "plan-1".into(),
    });
    cases.push(p);
    let mut p = ppp();
    p.group = GroupRule::Attach {
        node_id: "tm-node-1".into(),
    };
    cases.push(p);
    let mut p = ppp();
    p.members = MemberMode::Configured;
    cases.push(p);
    let mut p = ppp();
    p.writes = vec![WriteGrant {
        access_ref: "default".into(),
        scope: WriteScope::WanAddressLists,
        enabled: true,
    }];
    cases.push(p);
    let mut p = ppp();
    p.read_access_ref = "nobody".into();
    cases.push(p);
    let mut p = ppp();
    p.capacity = CapacityRule::Fixed {
        down_bps: "0".into(),
        up_bps: "-5".into(),
    };
    cases.push(p);
    let mut p = dhcp("d", 1, &["LAN"]);
    p.rates.swap(1, 2);
    cases.push(p);
    for (n, p) in cases.into_iter().enumerate() {
        let (intent, issues) = translate(&c, &c.routers[0], &[p]);
        assert_eq!(intent, RouterIntent::default(), "case {n}");
        assert!(
            issues.iter().any(|i| i.severity == Severity::Blocker),
            "case {n}: {issues:?}"
        );
    }
}

#[test]
fn equal_priority_overlap_is_a_blocker_and_a_lower_value_wins() {
    let c = cfg("");
    let (_, issues) = translate(
        &c,
        &c.routers[0],
        &[dhcp("a", 5, &["LAN", "Guest"]), dhcp("b", 5, &["Guest"])],
    );
    assert_eq!(codes(&issues), [("priority_overlap", "b")]);
    let (intent, issues) = translate(
        &c,
        &c.routers[0],
        &[dhcp("a", 9, &["LAN", "Guest"]), dhcp("b", 5, &["Guest"])],
    );
    assert!(issues.is_empty(), "{issues:?}");
    let order: Vec<_> = intent.dhcp.iter().map(|d| d.policy_id.as_str()).collect();
    assert_eq!(order, ["b", "a"]);
    // Two enabled PPP policies cannot share the one PPP scope of a router.
    let mut second = ppp();
    second.id = Some("policy-ppp-2".into());
    second.priority = 1;
    let (_, issues) = translate(&c, &c.routers[0], &[ppp(), second]);
    assert_eq!(codes(&issues), [("unsupported_in_v1", "policy-ppp-2")]);
}

#[test]
fn a_disabled_policy_is_checked_but_not_translated() {
    let c = cfg("");
    let mut p = ppp();
    p.enabled = false;
    let (intent, issues) = translate(&c, &c.routers[0], &[p]);
    assert!(intent.ppp.is_none() && issues.is_empty());
}

#[test]
fn input_limits_reject_the_request() {
    assert!(check_limits(&vec![ppp(); MAX_POLICIES + 1]).is_err());
    let mut deep = ppp();
    let leaf = || FilterExpr::Match {
        field: FieldKey::AccountProfile,
        op: FilterOp::Eq,
        value: FilterValue::Text { value: "a".into() },
    };
    deep.filter = FilterExpr::All {
        rules: vec![FilterExpr::Any {
            rules: vec![FilterExpr::All {
                rules: vec![FilterExpr::Any {
                    rules: vec![leaf()],
                }],
            }],
        }],
    };
    assert!(check_limits(&[deep]).is_err());
    let mut wide = ppp();
    wide.filter = FilterExpr::Any {
        rules: (0..=MAX_FILTER_LEAVES).map(|_| leaf()).collect(),
    };
    assert!(check_limits(&[wide]).is_err());
    assert!(check_limits(&[ppp(), dhcp("d", 1, &["LAN"])]).is_ok());
}
