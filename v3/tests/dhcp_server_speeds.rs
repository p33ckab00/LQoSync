use lqosync_v3::{
    routeros::Row,
    speeds::{self, SpeedRules},
};
mod common;
fn rules(mode: &str) -> SpeedRules {
    toml::from_str(&format!(
        "[[dhcp_servers]]\nrouter='r1'\nserver='dhcp1'\nmode='{mode}'\ndownload=35.0\nupload=7.0\n"
    ))
    .unwrap()
}
fn row(values: &[(&str, &str)]) -> Row {
    values
        .iter()
        .map(|(k, v)| (k.to_string(), v.to_string()))
        .collect()
}
#[test]
fn auto_uses_server_comment_then_name_then_manual_without_lease_overrides() {
    let (_, invs) = common::fixture();
    let mut inv = (*invs["r1"]).clone();
    inv.tables.insert(
        "/ip/dhcp-server/print".into(),
        vec![row(&[("name", "dhcp1"), ("comment", "Site|25M/5M")])],
    );
    let r = rules("auto");
    let report = speeds::preview(&inv, &r).unwrap();
    let speed = report
        .entries
        .iter()
        .find(|e| e.service == "dhcp")
        .unwrap()
        .speed
        .as_ref()
        .unwrap();
    assert_eq!(
        (speed.rates.download, speed.rates.upload, speed.source),
        (25., 5., "dhcp_server_comment")
    );
    inv.tables.get_mut("/ip/dhcp-server/print").unwrap()[0].remove("comment");
    let report = speeds::preview(&inv, &r).unwrap();
    let speed = report
        .entries
        .iter()
        .find(|e| e.service == "dhcp")
        .unwrap()
        .speed
        .as_ref()
        .unwrap();
    assert_eq!(
        (speed.rates.download, speed.rates.upload, speed.source),
        (35., 7., "dhcp_server_manual")
    );
    let speed =
        speeds::resolve_dhcp(&row(&[("name", "DHCP-40M/8M")]), &SpeedRules::default()).unwrap();
    assert_eq!(
        (speed.rates.download, speed.rates.upload, speed.source),
        (40., 8., "dhcp_server_name")
    );
}
#[test]
fn explicit_modes_and_bad_server_evidence_do_not_silently_fall_through() {
    let (_, invs) = common::fixture();
    let mut inv = (*invs["r1"]).clone();
    inv.tables.insert(
        "/ip/dhcp-server/print".into(),
        vec![row(&[("name", "dhcp1"), ("comment", "Site|25M/5M")])],
    );
    for (mode, expected) in [
        ("comment", Some(25.)),
        ("name", None),
        ("manual", Some(35.)),
    ] {
        let report = speeds::preview(&inv, &rules(mode)).unwrap();
        let e = report.entries.iter().find(|e| e.service == "dhcp").unwrap();
        assert_eq!(e.speed.as_ref().map(|s| s.rates.download), expected);
    }
    for mutation in [
        "invalid-comment",
        "disabled",
        "duplicate",
        "missing",
        "failed",
    ] {
        let mut i = inv.clone();
        match mutation {
            "invalid-comment" => {
                i.tables.get_mut("/ip/dhcp-server/print").unwrap()[0]
                    .insert("comment".into(), "25M/bad".into());
            }
            "disabled" => {
                i.tables.get_mut("/ip/dhcp-server/print").unwrap()[0]
                    .insert("disabled".into(), "true".into());
            }
            "duplicate" => {
                let r = i.tables["/ip/dhcp-server/print"][0].clone();
                i.tables.get_mut("/ip/dhcp-server/print").unwrap().push(r);
            }
            "missing" => {
                i.tables.remove("/ip/dhcp-server/print");
            }
            _ => {
                i.errors
                    .insert("/ip/dhcp-server/print".into(), "failed".into());
            }
        }
        let report = speeds::preview(&i, &rules("auto")).unwrap();
        assert!(
            report
                .entries
                .iter()
                .find(|e| e.service == "dhcp")
                .unwrap()
                .speed
                .is_none(),
            "{mutation}"
        );
    }
}
#[test]
fn rules_reject_duplicates_invalid_rates_and_unsupported_modes() {
    let mut text = toml::to_string(&rules("manual")).unwrap();
    text.push_str("\n[[dhcp_servers]]\nrouter='r1'\nserver='dhcp1'\nmode='comment'\n");
    assert!(
        toml::from_str::<SpeedRules>(&text)
            .unwrap()
            .validate()
            .is_err()
    );
    for text in [
        "mode='wrong'\ndownload=35.0\nupload=7.0",
        "mode='manual'\ndownload=0.0\nupload=7.0",
        "mode='auto'\ndownload=35.0",
    ] {
        let rules: SpeedRules = toml::from_str(&format!(
            "[[dhcp_servers]]\nrouter='r1'\nserver='dhcp1'\n{text}\n"
        ))
        .unwrap();
        assert!(rules.validate().is_err());
    }
}

#[test]
fn server_names_are_strict_and_router_scopes_and_active_server_names_are_exact() {
    for name in [
        "DHCP-25M/bad",
        "DHCP-1.2.3M",
        "DHCP-1e3M",
        "DHCP-100/20M",
        "DHCP-100/ 20M",
        "DHCP-100/\t20M",
        "DHCP-25M-50M",
        "DHCP-0M",
    ] {
        assert!(
            speeds::resolve_dhcp(&row(&[("name", name)]), &SpeedRules::default()).is_err(),
            "{name}"
        );
    }
    let (_, invs) = common::fixture();
    let mut inv = (*invs["r1"]).clone();
    inv.tables.insert(
        "/ip/dhcp-server/print".into(),
        vec![row(&[("name", " site A "), ("comment", "Site|44M/9M")])],
    );
    let lease = &mut inv.tables.get_mut("/ip/dhcp-server/lease/print").unwrap()[0];
    lease.insert("server".into(), "all".into());
    lease.insert("active-server".into(), " site A ".into());
    let report = speeds::preview(&inv, &rules("manual")).unwrap();
    let speed = report
        .entries
        .iter()
        .find(|e| e.service == "dhcp")
        .unwrap()
        .speed
        .as_ref()
        .unwrap();
    assert_eq!(speed.rates.download, 44.);
    inv.router_id = "r2".into();
    inv.tables.insert(
        "/ip/dhcp-server/print".into(),
        vec![row(&[("name", "dhcp1")])],
    );
    inv.tables.get_mut("/ip/dhcp-server/lease/print").unwrap()[0]
        .insert("active-server".into(), "dhcp1".into());
    let report = speeds::preview(&inv, &rules("manual")).unwrap();
    assert!(
        report
            .entries
            .iter()
            .find(|e| e.service == "dhcp")
            .unwrap()
            .speed
            .is_none()
    );
}
#[test]
#[ignore = "manual DHCP server cache performance measurement"]
fn preview_50k_dhcp_leases_and_1000_server_rules() {
    let (_, invs) = common::fixture();
    let mut inv = (*invs["r1"]).clone();
    let mut rules = SpeedRules::default();
    let servers = (0..1000)
        .map(|n| {
            let name = format!("dhcp-{n}");
            rules.dhcp_servers.push(speeds::DhcpServerRule {
                router: "r1".into(),
                server: name.clone(),
                mode: "manual".into(),
                download: Some(35. + (n % 5) as f64),
                upload: Some(7.),
            });
            row(&[("name", &name)])
        })
        .collect();
    inv.tables.insert("/ip/dhcp-server/print".into(), servers);
    inv.tables.insert(
        "/ip/dhcp-server/lease/print".into(),
        (0..50_000)
            .map(|n| {
                row(&[
                    ("server", &format!("dhcp-{}", n % 1000)),
                    ("status", "bound"),
                    (
                        "address",
                        &format!("10.{}.{}.{}", n / 65536, (n / 256) % 256, n % 256),
                    ),
                    (
                        "mac-address",
                        &format!(
                            "02:00:00:{:02X}:{:02X}:{:02X}",
                            n / 65536,
                            (n / 256) % 256,
                            n % 256
                        ),
                    ),
                    ("comment", "ignored|999M/999M"),
                ])
            })
            .collect(),
    );
    let start = std::time::Instant::now();
    let report = speeds::preview(&inv, &rules).unwrap();
    let elapsed = start.elapsed();
    assert_eq!(report.resolved(), 50_002);
    assert!(report.errors.is_empty());
    for e in report.entries.iter().filter(|e| e.service == "dhcp") {
        let n = e
            .group
            .strip_prefix("dhcp-")
            .unwrap()
            .parse::<usize>()
            .unwrap();
        let speed = e.speed.as_ref().unwrap();
        assert_eq!(speed.rates.download, 35. + (n % 5) as f64);
        assert_eq!(speed.source, "dhcp_server_manual");
    }
    for rows in inv.tables.values_mut() {
        rows.reverse();
    }
    let reversed = speeds::preview(&inv, &rules).unwrap();
    assert_eq!(reversed.resolved(), report.resolved());
    let rates = |r: speeds::SpeedPreview| {
        r.entries
            .into_iter()
            .map(|e| (e.identity, e.speed.unwrap().rates.download.to_bits()))
            .collect::<std::collections::BTreeMap<_, _>>()
    };
    assert_eq!(rates(reversed), rates(report));
    eprintln!(
        "50k DHCP leases /1000 server rules: {elapsed:?}; debug, excludes collection and disk I/O"
    );
}
