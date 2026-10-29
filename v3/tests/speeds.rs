use lqosync_v3::{routeros::Row, speeds::*};
fn row(fields: &[(&str, &str)]) -> Row {
    fields
        .iter()
        .map(|(k, v)| (k.to_string(), v.to_string()))
        .collect()
}
fn assert_rates(speed: &ResolvedSpeed, down: f64, up: f64) {
    assert!((speed.rates.download - down).abs() < 1e-9);
    assert!((speed.rates.upload - up).abs() < 1e-9);
}
#[test]
fn router_rates_use_bits_and_subscriber_direction_and_accept_symmetric_rates() {
    for (raw, d, u) in [
        ("10M/50M", 50., 10.),
        ("512k/1000000", 1., 0.512),
        ("25M", 25., 25.),
        ("1G/2G", 2000., 1000.),
        ("1/2", 0.000002, 0.000001),
    ] {
        assert_rates(
            &parse_router_rate(raw, &SpeedRules::default()).unwrap(),
            d,
            u,
        );
    }
}
#[test]
fn explicit_minima_win_and_computed_minima_never_raise_router_maximum() {
    let r =
        parse_router_rate("10M/50M 20M/60M 5M/25M 5/5 8 2M/7M", &SpeedRules::default()).unwrap();
    assert_eq!((r.rates.download_min, r.rates.upload_min), (7., 2.));
    assert!(r.burst_ignored);
    let r = parse_router_rate("64k/100k", &SpeedRules::default()).unwrap();
    assert_eq!((r.rates.download, r.rates.upload), (0.1, 0.064));
    assert_eq!((r.rates.download_min, r.rates.upload_min), (0.1, 0.064));
}
#[test]
fn invalid_or_unlimited_router_rates_are_errors() {
    for raw in [
        "",
        "0/0",
        "0/10M",
        "-10M/20M",
        "NaN/20M",
        "1e9/10M",
        "10M/20M/30M",
        "10MB/20M",
        "10M/20M junk",
        "10M/20M 30M 10M 5G",
        "10M/20M 30M 10M 5 9",
        "10M/20M 30M 10M 5 8 11M/20M",
        "10M/20M 30M 10M 5 8 1M/2M extra",
    ] {
        assert!(
            parse_router_rate(raw, &SpeedRules::default()).is_err(),
            "{raw}"
        );
    }
}
#[test]
fn legacy_comment_precedence_keeps_customer_download_upload_order() {
    let secret = row(&[("comment", "Customer|25M/5M")]);
    let active = row(&[("comment", "30M/10M")]);
    let profile = row(&[
        ("name", "Plan100M"),
        ("comment", "40M/20M"),
        ("rate-limit", "10M/50M"),
    ]);
    let s = resolve_ppp(&secret, &active, Some(&profile), &SpeedRules::default()).unwrap();
    assert_rates(&s, 25., 5.);
    assert_eq!(s.source, "ppp_secret_comment");
    let s = resolve_ppp(&row(&[]), &row(&[]), Some(&profile), &SpeedRules::default()).unwrap();
    assert_rates(&s, 40., 20.);
    assert_eq!(s.source, "ppp_profile_comment");
    let profile = row(&[("name", "Plan100M"), ("rate-limit", "10M/50M")]);
    let s = resolve_ppp(&row(&[]), &row(&[]), Some(&profile), &SpeedRules::default()).unwrap();
    assert_rates(&s, 50., 10.);
}
#[test]
fn no_guessed_maximum_on_missing_or_invalid_rates() {
    for profile in [
        row(&[("name", "service2026")]),
        row(&[("name", "Plan100M"), ("rate-limit", "0/0")]),
        row(&[("name", "Plan100M"), ("rate-limit", "bad")]),
    ] {
        assert!(resolve_ppp(&row(&[]), &row(&[]), Some(&profile), &SpeedRules::default()).is_err());
    }
    assert!(resolve_ppp(&row(&[]), &row(&[]), None, &SpeedRules::default()).is_err());
    let profile = row(&[("name", "Plan100M")]);
    let s = resolve_ppp(&row(&[]), &row(&[]), Some(&profile), &SpeedRules::default()).unwrap();
    assert_rates(&s, 100., 100.);
}
#[test]
fn comments_are_strict_and_do_not_salvage_bad_pairs() {
    for text in [
        "Customer|25M/garbage",
        "Customer|0M/10M",
        "Customer|-25M/10M",
        "25M/5M then 50M/10M",
        "25M/10M/5M",
    ] {
        assert!(
            resolve_dhcp(
                &row(&[("comment", text), ("rate-limit", "1M/10M")]),
                &SpeedRules::default()
            )
            .is_err(),
            "{text}"
        );
    }
    let s = resolve_dhcp(
        &row(&[("comment", "Customer|25M/5M")]),
        &SpeedRules::default(),
    )
    .unwrap();
    assert_rates(&s, 25., 5.);
    let s = resolve_dhcp(
        &row(&[
            ("comment", "ordinary customer note"),
            ("rate-limit", "5M/25M"),
        ]),
        &SpeedRules::default(),
    )
    .unwrap();
    assert_rates(&s, 25., 5.);
    assert!(resolve_dhcp(&row(&[]), &SpeedRules::default()).is_err());
}
#[test]
fn hotspot_uses_router_user_and_profile_rules_without_local_defaults() {
    let profile = row(&[("rate-limit", "2M/10M")]);
    let s = resolve_hotspot(&row(&[]), Some(&profile), &SpeedRules::default()).unwrap();
    assert_rates(&s, 10., 2.);
    assert!(resolve_hotspot(&row(&[]), None, &SpeedRules::default()).is_err());
}

use lqosync_v3::routeros::Inventory;
fn inventory() -> Inventory {
    Inventory {
        router_id: "test".into(),
        collected_at: 1,
        errors: Default::default(),
        tables: [
            (
                "/ppp/active/print",
                vec![row(&[("name", "alice"), ("address", "192.0.2.1")])],
            ),
            (
                "/ppp/secret/print",
                vec![row(&[("name", "alice"), ("profile", "Plan100M")])],
            ),
            (
                "/ppp/profile/print",
                vec![row(&[("name", "Plan100M"), ("rate-limit", "10M/50M")])],
            ),
            (
                "/ip/dhcp-server/lease/print",
                vec![row(&[
                    ("mac-address", "AA:BB:CC:DD:EE:FF"),
                    ("server", "dhcp1"),
                    ("status", "bound"),
                    ("address", "192.0.2.2"),
                    ("comment", "Client|25M/5M"),
                ])],
            ),
            (
                "/ip/hotspot/active/print",
                vec![row(&[("user", "bob"), ("address", "192.0.2.3")])],
            ),
            (
                "/ip/hotspot/user/print",
                vec![row(&[("name", "bob"), ("profile", "HS10M")])],
            ),
            (
                "/ip/hotspot/user/profile/print",
                vec![row(&[("name", "HS10M"), ("rate-limit", "2M/10M")])],
            ),
        ]
        .into_iter()
        .map(|(k, v)| (k.to_string(), v))
        .collect(),
    }
}
#[test]
fn inventory_preview_joins_all_sources_and_records_provenance() {
    let report = preview(&inventory(), &SpeedRules::default()).unwrap();
    assert!(report.errors.is_empty());
    assert_eq!(report.resolved(), 3);
    assert_eq!(report.unresolved(), 0);
    let ppp = report.entries.iter().find(|e| e.service == "ppp").unwrap();
    assert_rates(ppp.speed.as_ref().unwrap(), 50., 10.);
    assert_eq!(ppp.group, "Plan100M");
    assert_eq!(ppp.speed.as_ref().unwrap().source, "ppp_profile_rate_limit");
}
#[test]
fn partial_or_duplicate_sources_do_not_choose_an_arbitrary_profile() {
    let mut inv = inventory();
    let duplicate = inv.tables["/ppp/profile/print"][0].clone();
    inv.tables
        .get_mut("/ppp/profile/print")
        .unwrap()
        .push(duplicate);
    let report = preview(&inv, &SpeedRules::default()).unwrap();
    assert!(
        report
            .entries
            .iter()
            .find(|e| e.service == "ppp")
            .unwrap()
            .error
            .as_ref()
            .unwrap()
            .contains("Duplicate")
    );
    let mut inv = inventory();
    inv.errors
        .insert("/ppp/secret/print".into(), "Denied".into());
    let report = preview(&inv, &SpeedRules::default()).unwrap();
    assert!(!report.errors.is_empty());
    assert!(
        report
            .entries
            .iter()
            .find(|e| e.service == "ppp")
            .unwrap()
            .speed
            .is_none()
    );
}
#[test]
fn inactive_dhcp_and_unknown_radius_users_do_not_get_default_speeds() {
    let mut inv = inventory();
    inv.tables.get_mut("/ip/dhcp-server/lease/print").unwrap()[0]
        .insert("status".into(), "waiting".into());
    inv.tables
        .get_mut("/ip/hotspot/user/print")
        .unwrap()
        .clear();
    let report = preview(&inv, &SpeedRules::default()).unwrap();
    assert_eq!(report.entries.len(), 2);
    assert!(
        report
            .entries
            .iter()
            .find(|e| e.service == "hotspot")
            .unwrap()
            .speed
            .is_none()
    );
}
#[test]
fn invalid_minimum_policy_is_rejected_even_for_empty_inventory() {
    let mut rules = SpeedRules {
        min_rate_fraction: 1.1,
        ..Default::default()
    };
    assert!(preview(&inventory(), &rules).is_err());
    rules.min_rate_fraction = f64::NAN;
    assert!(rules.validate().is_err());
    rules.min_rate_fraction = 0.5;
    rules.min_floor_mbps = -1.0;
    assert!(rules.validate().is_err());
}

#[test]
fn malformed_comment_left_sides_cannot_be_salvaged_as_symmetric_rates() {
    for text in ["NaNM/10M", "bad/10M", "-inf/10M", "25M/bad"] {
        assert!(
            resolve_dhcp(
                &row(&[("comment", text), ("rate-limit", "5M/25M")]),
                &SpeedRules::default()
            )
            .is_err(),
            "{text}"
        );
    }
}
#[test]
#[ignore = "manual performance measurement"]
fn inventory_50k_performance() {
    let mut inv = inventory();
    let profiles = (0..100)
        .map(|i| row(&[("name", &format!("Plan{i}M")), ("rate-limit", "10M/50M")]))
        .collect();
    inv.tables.insert("/ppp/profile/print".into(), profiles);
    inv.tables.insert(
        "/ppp/secret/print".into(),
        (0..50_000)
            .map(|i| {
                row(&[
                    ("name", &format!("user{i:05}")),
                    ("profile", &format!("Plan{}M", i % 100)),
                ])
            })
            .collect(),
    );
    inv.tables.insert(
        "/ppp/active/print".into(),
        (0..50_000)
            .map(|i| {
                row(&[
                    ("name", &format!("user{i:05}")),
                    (
                        "address",
                        &format!("10.{}.{}.{}", i / 65536, (i / 256) % 256, i % 256),
                    ),
                ])
            })
            .collect(),
    );
    let start = std::time::Instant::now();
    let report = preview(&inv, &SpeedRules::default()).unwrap();
    let elapsed = start.elapsed();
    assert_eq!(report.resolved(), 50_002);
    assert_eq!(report.unresolved(), 0);
    assert!(report.errors.is_empty());
    eprintln!(
        "50,000 PPP sessions / 100 shared profiles: {elapsed:?} ({:?} build)",
        if cfg!(debug_assertions) {
            "debug"
        } else {
            "release"
        }
    );
}

#[test]
fn numeric_token_boundaries_never_change_the_value() {
    for text in ["1e3M", "1.2.3M", "100/20M"] {
        assert!(
            resolve_dhcp(&row(&[("comment", text)]), &SpeedRules::default()).is_err(),
            "{text}"
        );
    }
    let p = row(&[("name", "Plan.5M")]);
    assert_rates(
        &resolve_ppp(&row(&[]), &row(&[]), Some(&p), &SpeedRules::default()).unwrap(),
        0.5,
        0.5,
    );
    for name in [
        "Plan1.2.3M",
        "Plan1e3M",
        "Plan1e-3M",
        "Plan1e+3M",
        "Plan50M/10M",
    ] {
        assert!(
            resolve_ppp(
                &row(&[]),
                &row(&[]),
                Some(&row(&[("name", name)])),
                &SpeedRules::default()
            )
            .is_err(),
            "{name}"
        );
    }
}
#[test]
fn all_duplicate_sessions_are_unresolved_independent_of_input_order() {
    let mut inv = inventory();
    for path in [
        "/ppp/active/print",
        "/ip/dhcp-server/lease/print",
        "/ip/hotspot/active/print",
    ] {
        let mut duplicate = inv.tables[path][0].clone();
        duplicate.insert("comment".into(), "100M/100M".into());
        inv.tables.get_mut(path).unwrap().push(duplicate);
    }
    for _ in 0..2 {
        let report = preview(&inv, &SpeedRules::default()).unwrap();
        assert_eq!(report.resolved(), 0);
        assert_eq!(report.unresolved(), 6);
        for rows in inv.tables.values_mut() {
            rows.reverse();
        }
    }
}

#[test]
fn free_text_comment_with_whitespace_keeps_legacy_rates() {
    for text in ["Alice Smith 25M/5M", "Plan 25M", "Customer | 25M/5M"] {
        let s = resolve_dhcp(&row(&[("comment", text)]), &SpeedRules::default()).unwrap();
        assert_eq!(s.rates.download, 25.);
    }
}
