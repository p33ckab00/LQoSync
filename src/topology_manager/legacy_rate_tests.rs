use super::*;
use crate::topology_manager::normalize::{AuthorityEvidence, CurrentAccount, CurrentProfile};
fn account() -> CurrentAccount {
    CurrentAccount {
        kind: SourceKind::Ppp,
        row_ref: "*a".into(),
        authority: AuthorityEvidence::LocalAccount {
            name: "alice".into(),
            comment: Some("Alice |25M/5M".into()),
        },
        profile_ref: Some("*p".into()),
        direct_rate_limit: None,
        service_refs: vec![],
        endpoints: vec![],
        active_comments: vec![Some("50M/10M".into())],
    }
}
fn profile() -> CurrentProfile {
    CurrentProfile {
        kind: SourceKind::Ppp,
        row_ref: "*p".into(),
        name: "100M".into(),
        rate_limit: Some("3M/60M".into()),
        comment: Some("40M/8M".into()),
        active: true,
    }
}
#[test]
fn converted_ppp_preserves_priority_and_explicit_fallback() {
    let speed = crate::config::Speed {
        pppoe_sources: vec![
            crate::config::PppSource::SecretComment,
            crate::config::PppSource::ProfileRateLimit,
        ],
        ..Default::default()
    };
    let rule = LegacyPriority::ppp(&speed, "10M/2M").unwrap();
    let compiled = rule.compile(SourceKind::Ppp).unwrap();
    let mut account = account();
    assert_eq!(
        compiled
            .read(&account, Some(&profile()), &[])
            .unwrap()
            .unwrap()
            .down,
        25_000_000
    );
    if let AuthorityEvidence::LocalAccount { comment, .. } = &mut account.authority {
        *comment = None;
    }
    assert_eq!(
        compiled
            .read(&account, Some(&profile()), &[])
            .unwrap()
            .unwrap()
            .down,
        60_000_000
    );
    assert_eq!(
        compiled.read(&account, None, &[]).unwrap().unwrap().down,
        10_000_000
    );
}
#[test]
fn active_comment_disagreement_cannot_choose_an_arbitrary_session() {
    let speed = crate::config::Speed {
        pppoe_sources: vec![crate::config::PppSource::ActiveComment],
        ..Default::default()
    };
    let compiled = LegacyPriority::ppp(&speed, "10M/10M")
        .unwrap()
        .compile(SourceKind::Ppp)
        .unwrap();
    let mut account = account();
    account.active_comments.push(Some("70M/10M".into()));
    assert!(compiled.read(&account, None, &[]).is_err());
}
#[test]
fn source_rules_reject_another_protocol_and_lossy_defaults() {
    assert!(LegacyPriority::ppp(&crate::config::Speed::default(), "1.0000001M/1M").is_err());
    let rule = LegacyPriority::ppp(&crate::config::Speed::default(), "10M/10M").unwrap();
    assert!(rule.compile(SourceKind::Dhcp).is_err());
}

#[test]
fn converted_dhcp_uses_selected_server_and_has_no_implicit_fallback() {
    use crate::config::{DhcpMode, DhcpSource};
    let speed = crate::config::Speed::default();
    let server = crate::config::DhcpServer {
        name: "access".into(),
        mode: DhcpMode::PerPlan,
        speed_sources: Some(vec![
            DhcpSource::LeaseComment,
            DhcpSource::ServerComment,
            DhcpSource::Settings,
        ]),
        default_plan_down_mbps: Some(15.0),
        default_plan_up_mbps: Some(4.0),
        download_factor: None,
        upload_factor: None,
    };
    let defaults = crate::config::BandwidthDefaults::default();
    let mut rule = LegacyPriority::dhcp(&speed, &defaults, &server).unwrap();
    let mut current = account();
    current.kind = SourceKind::Dhcp;
    let service = CurrentService {
        kind: SourceKind::Dhcp,
        row_ref: "*server".into(),
        name: "access".into(),
        disabled: false,
        comment: Some("30M/8M".into()),
        active: true,
    };
    assert_eq!(
        rule.compile(SourceKind::Dhcp)
            .unwrap()
            .read(&current, None, &[&service])
            .unwrap()
            .unwrap()
            .down,
        25_000_000
    );
    if let AuthorityEvidence::LocalAccount { comment, .. } = &mut current.authority {
        *comment = None;
    }
    assert_eq!(
        rule.compile(SourceKind::Dhcp)
            .unwrap()
            .read(&current, None, &[&service])
            .unwrap()
            .unwrap()
            .down,
        30_000_000
    );
    rule.sources = vec![LegacyRateSource::AccountComment];
    assert!(
        rule.compile(SourceKind::Dhcp)
            .unwrap()
            .read(&current, None, &[])
            .unwrap()
            .is_none()
    );
}

#[test]
fn default_order_is_preserved_and_fractional_bps_in_a_comment_blocks() {
    let speed = crate::config::Speed {
        pppoe_sources: vec![
            crate::config::PppSource::Default,
            crate::config::PppSource::SecretComment,
        ],
        ..Default::default()
    };
    let rule = LegacyPriority::ppp(&speed, "10M/2M").unwrap();
    assert_eq!(
        rule.compile(SourceKind::Ppp)
            .unwrap()
            .read(&account(), Some(&profile()), &[])
            .unwrap()
            .unwrap()
            .down,
        10_000_000
    );
    let speed = crate::config::Speed {
        pppoe_sources: vec![crate::config::PppSource::SecretComment],
        ..Default::default()
    };
    let mut account = account();
    if let AuthorityEvidence::LocalAccount { comment, .. } = &mut account.authority {
        *comment = Some("1.0000001M/2M".into());
    }
    assert!(
        LegacyPriority::ppp(&speed, "10M/2M")
            .unwrap()
            .compile(SourceKind::Ppp)
            .unwrap()
            .read(&account, None, &[])
            .is_err()
    );
}

#[test]
fn hotspot_conversion_keeps_explicit_directional_capacity() {
    let rule = LegacyPriority::hotspot(
        &crate::config::Speed::default(),
        &crate::config::BandwidthDefaults::default(),
        &crate::config::HotspotCfg {
            enabled: true,
            include_mac: false,
            download_limit_mbps: Some(20.0),
            upload_limit_mbps: Some(7.0),
            download_factor: None,
            upload_factor: None,
        },
    )
    .unwrap();
    let mut current = account();
    current.kind = SourceKind::Hotspot;
    assert_eq!(
        rule.compile(SourceKind::Hotspot)
            .unwrap()
            .read(&current, None, &[])
            .unwrap(),
        Some(RateBps {
            down: 20_000_000,
            up: 7_000_000
        })
    );
}
