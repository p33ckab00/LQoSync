use lqosync_v3::wan_health::{Config, Evidence, HealthStatus, State, evaluate};
fn evidence(at: u64, count: u64, up: bool) -> Evidence {
    Evidence {
        context: "policy-a".into(),
        observed_at: at,
        router_uptime: at + 1000,
        counters: [count; 2],
        passed: [up; 2],
    }
}
fn step(c: &Config, s: &State, at: u64, count: u64, up: bool) -> State {
    evaluate(c, Some(s), Ok(evidence(at, count, up)), at).unwrap()
}
#[test]
fn startup_needs_new_completed_rounds_and_elapsed_recovery_delay() {
    let c = Config::default();
    let mut s = evaluate(&c, None, Ok(evidence(100, 1, true)), 100).unwrap();
    assert_eq!(s.status, HealthStatus::Unknown);
    for (at, count) in [(110, 2), (120, 3), (130, 4)] {
        s = step(&c, &s, at, count, true);
        assert_eq!(s.status, HealthStatus::Unknown);
    }
    s = step(&c, &s, 140, 5, true);
    assert_eq!(s.status, HealthStatus::Healthy);
    for (at, count) in [(150, 6), (160, 7)] {
        s = step(&c, &s, at, count, false);
        assert_eq!(s.status, HealthStatus::Healthy);
    }
    s = step(&c, &s, 170, 8, false);
    assert_eq!(s.status, HealthStatus::Unhealthy);
}
#[test]
fn repeated_counters_cannot_refresh_health_or_accumulate_failures() {
    let c = Config::default();
    let mut s = evaluate(&c, None, Ok(evidence(100, 1, false)), 100).unwrap();
    for at in [110, 120, 130] {
        s = step(&c, &s, at, 1, false);
        assert_eq!(s.status, HealthStatus::Unknown);
    }
    s = step(&c, &s, 131, 1, false);
    assert_eq!(s.status, HealthStatus::Unknown);
    assert!(s.reason.contains("stale"));
}
#[test]
fn unknown_input_resets_failure_streak_and_does_not_evict() {
    let c = Config::default();
    let mut s = evaluate(&c, None, Ok(evidence(100, 1, false)), 100).unwrap();
    s = step(&c, &s, 110, 2, false);
    s = step(&c, &s, 120, 3, false);
    s = evaluate(&c, Some(&s), Err("read failed".into()), 130).unwrap();
    assert_eq!(s.status, HealthStatus::Unknown);
    s = step(&c, &s, 140, 4, false);
    s = step(&c, &s, 150, 5, false);
    assert_eq!(s.status, HealthStatus::Unknown);
}
#[test]
fn future_old_reordered_or_rebooted_evidence_is_unknown() {
    let c = Config::default();
    let s = evaluate(&c, None, Ok(evidence(100, 20, true)), 100).unwrap();
    for mut e in [
        evidence(102, 21, true),
        evidence(60, 21, true),
        evidence(101, 19, true),
    ] {
        let got = evaluate(&c, Some(&s), Ok(e.clone()), 101).unwrap();
        assert_eq!(got.status, HealthStatus::Unknown);
        e.router_uptime = 1;
        let got = evaluate(&c, Some(&s), Ok(e), 101).unwrap();
        assert_eq!(got.status, HealthStatus::Unknown);
    }
    let mut changed = evidence(110, 21, true);
    changed.context = "policy-b".into();
    let got = evaluate(&c, Some(&s), Ok(changed), 110).unwrap();
    assert_eq!(got.status, HealthStatus::Unknown);
}
#[test]
fn one_passing_target_is_enough_but_both_counters_must_advance() {
    let c = Config {
        recovery_seconds: 0,
        success_rounds: 1,
        ..Default::default()
    };
    let s = evaluate(&c, None, Ok(evidence(100, 1, false)), 100).unwrap();
    let mut e = evidence(110, 2, false);
    e.passed = [true, false];
    e.counters = [2, 1];
    let unchanged = evaluate(&c, Some(&s), Ok(e.clone()), 110).unwrap();
    assert_eq!(unchanged.status, HealthStatus::Unknown);
    e.counters = [2, 2];
    let healthy = evaluate(&c, Some(&unchanged), Ok(e), 120).unwrap();
    assert_eq!(healthy.status, HealthStatus::Healthy);
}
#[test]
fn config_rejects_invalid_limits_and_old_settings_keep_health_disabled() {
    let c = Config {
        failure_rounds: 0,
        ..Default::default()
    };
    assert!(c.validate().is_err());
    let c = Config {
        interval_seconds: 31,
        ..Default::default()
    };
    assert!(c.validate().is_err());
    let s: lqosync_v3::model::Settings = toml::from_str("").unwrap();
    assert!(s.wan_health.is_none());
    assert_eq!(
        lqosync_v3::routeros::read_properties("/tool/netwatch/print").unwrap(),
        ".id,host,type,disabled,status,done-tests,interval,timeout,src-address,up-script,down-script,test-script"
    );
    assert!(lqosync_v3::routeros::read_properties("/tool/netwatch/add").is_err());
}
fn row(fields: &[(&str, &str)]) -> lqosync_v3::routeros::Row {
    fields
        .iter()
        .map(|(k, v)| (k.to_string(), v.to_string()))
        .collect()
}
fn fixture() -> (
    lqosync_v3::wan_health::Monitor,
    lqosync_v3::routeros::Inventory,
) {
    use lqosync_v3::wan_health::{Monitor, Probe};
    let m = Monitor {
        wan: "wan1".into(),
        probes: [
            Probe {
                id: "*1".into(),
                target: "1.1.1.1".parse().unwrap(),
            },
            Probe {
                id: "*2".into(),
                target: "8.8.8.8".parse().unwrap(),
            },
        ],
    };
    let mut tables = std::collections::BTreeMap::new();
    tables.insert(
        "/system/resource/print".into(),
        vec![row(&[("version", "7.20"), ("uptime", "1h")])],
    );
    tables.insert(
        "/interface/print".into(),
        vec![row(&[
            ("name", "ether1"),
            ("running", "true"),
            ("disabled", "false"),
        ])],
    );
    tables.insert(
        "/ip/route/print".into(),
        m.probes
            .iter()
            .map(|p| {
                row(&[
                    ("dst-address", &format!("{}/32", p.target)),
                    ("routing-table", "main"),
                    ("active", "true"),
                    ("immediate-gw", "192.0.2.1%ether1"),
                ])
            })
            .collect(),
    );
    tables.insert("/routing/rule/print".into(), vec![]);
    tables.insert("/ip/firewall/mangle/print".into(), vec![]);
    tables.insert(
        "/tool/netwatch/print".into(),
        m.probes
            .iter()
            .map(|p| {
                row(&[
                    (".id", &p.id),
                    ("host", &p.target.to_string()),
                    ("type", "icmp"),
                    ("status", "up"),
                    ("done-tests", "1"),
                    ("interval", "10s"),
                    ("timeout", "1s"),
                ])
            })
            .collect(),
    );
    (
        m,
        lqosync_v3::routeros::Inventory {
            router_id: "core".into(),
            collected_at: 100,
            tables,
            errors: Default::default(),
        },
    )
}
#[test]
fn provider_requires_exact_probe_paths_and_completed_counters() {
    use lqosync_v3::wan_health::from_inventory;
    let (m, inv) = fixture();
    let c = Config::default();
    let e = from_inventory(&c, &m, &inv, "ether1", "revision").unwrap();
    assert_eq!(e.counters, [1, 1]);
    assert_eq!(e.passed, [true, true]);
    for key in ["done-tests", "status", "interval"] {
        let mut broken = inv.clone();
        broken.tables.get_mut("/tool/netwatch/print").unwrap()[0].remove(key);
        assert!(
            from_inventory(&c, &m, &broken, "ether1", "revision").is_err(),
            "{key}"
        );
    }
    let mut wrong = inv.clone();
    wrong.tables.get_mut("/ip/route/print").unwrap()[0]
        .insert("immediate-gw".into(), "192.0.2.2%ether2".into());
    assert!(from_inventory(&c, &m, &wrong, "ether1", "revision").is_err());
    let mut overlap = inv.clone();
    let duplicate = overlap.tables["/ip/route/print"][0].clone();
    overlap
        .tables
        .get_mut("/ip/route/print")
        .unwrap()
        .push(duplicate);
    assert!(from_inventory(&c, &m, &overlap, "ether1", "revision").is_err());
}
#[test]
fn provider_rejects_policy_routing_scripts_partial_inventory_and_unknown_status() {
    use lqosync_v3::wan_health::from_inventory;
    let (m, inv) = fixture();
    let c = Config::default();
    for (table, entry) in [
        ("/routing/rule/print", row(&[("action", "lookup")])),
        (
            "/ip/firewall/mangle/print",
            row(&[("chain", "output"), ("action", "mark-routing")]),
        ),
    ] {
        let mut bad = inv.clone();
        bad.tables.get_mut(table).unwrap().push(entry);
        assert!(from_inventory(&c, &m, &bad, "ether1", "revision").is_err());
    }
    for (key, value) in [
        ("up-script", "/ip/firewall/address-list/remove [find]"),
        ("status", "unknown"),
        ("host", "1.1.1.1@vrf1"),
        ("disabled", "true"),
        ("done-tests", "0"),
    ] {
        let mut bad = inv.clone();
        bad.tables.get_mut("/tool/netwatch/print").unwrap()[0].insert(key.into(), value.into());
        assert!(
            from_inventory(&c, &m, &bad, "ether1", "revision").is_err(),
            "{key}"
        );
    }
    let mut bad = inv.clone();
    bad.errors
        .insert("/tool/netwatch/print".into(), "denied".into());
    assert!(from_inventory(&c, &m, &bad, "ether1", "revision").is_err());
}
#[test]
fn threshold_change_resets_existing_evidence() {
    let c = Config {
        recovery_seconds: 0,
        success_rounds: 1,
        ..Default::default()
    };
    let s = evaluate(&c, None, Ok(evidence(100, 1, true)), 100).unwrap();
    let s = step(&c, &s, 110, 2, true);
    assert_eq!(s.status, HealthStatus::Healthy);
    let changed = Config {
        failure_rounds: 1,
        ..c
    };
    let s = step(&changed, &s, 120, 3, true);
    assert_eq!(s.status, HealthStatus::Unknown);
}
