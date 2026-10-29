use lqosync_v3::{
    core_diagnostics::{self, Health, Origin, Outcome},
    core_plan::Binding,
};
use std::{collections::BTreeMap, sync::Arc};
mod common;
#[path = "common/core.rs"]
mod core_fixture;
fn fixture() -> (
    lqosync_v3::model::Settings,
    BTreeMap<String, Arc<lqosync_v3::routeros::Inventory>>,
) {
    let (mut s, mut inv, _, _) = core_fixture::fixture();
    let mut wan = s.nodes[0].clone();
    wan.id = "wan2".into();
    wan.source = "uplink2".into();
    s.nodes.push(wan);
    s.core_lists.bindings.push(Binding {
        wan: "wan2".into(),
        router: "r1".into(),
        list: "lqos-wan2".into(),
        table: "to-wan2".into(),
    });
    let i = Arc::make_mut(inv.get_mut("r1").unwrap());
    i.tables
        .get_mut("/interface/print")
        .unwrap()
        .push(core_fixture::row(&[
            ("name", "uplink2"),
            ("running", "true"),
            ("disabled", "false"),
        ]));
    i.tables
        .get_mut("/routing/table/print")
        .unwrap()
        .push(core_fixture::row(&[("name", "to-wan2"), ("fib", "true")]));
    let mut rule = i.tables["/ip/firewall/mangle/print"][0].clone();
    rule.insert("src-address-list".into(), "lqos-wan2".into());
    rule.insert("new-routing-mark".into(), "to-wan2".into());
    i.tables
        .get_mut("/ip/firewall/mangle/print")
        .unwrap()
        .push(rule);
    let mut route = i.tables["/ip/route/print"][0].clone();
    route.insert("routing-table".into(), "to-wan2".into());
    route.insert("immediate-gw".into(), "203.0.113.2%uplink2".into());
    i.tables.get_mut("/ip/route/print").unwrap().push(route);
    (s, inv)
}
#[test]
fn per_wan_diagnostics_report_all_scoped_failures_without_poisoning_healthy_wan() {
    let (s, mut inv) = fixture();
    let i = Arc::make_mut(inv.get_mut("r1").unwrap());
    i.tables.get_mut("/ip/route/print").unwrap()[1]
        .insert("immediate-gw".into(), "203.0.113.2%wrong".into());
    i.tables.get_mut("/ip/firewall/mangle/print").unwrap()[1]
        .insert("new-routing-mark".into(), "wrong-table".into());
    let report = core_diagnostics::review(&s, &inv, &BTreeMap::new(), 101, Origin::Live);
    let core = &report.cores[0];
    assert_eq!(core.health, Health::Fresh);
    assert_eq!(
        core.wans.iter().find(|w| w.wan == "wan").unwrap().status,
        Outcome::Pass
    );
    assert_eq!(
        core.wans.iter().find(|w| w.wan == "wan2").unwrap().status,
        Outcome::Conflict
    );
    assert!(
        core.findings
            .iter()
            .any(|f| f.wan == "wan2" && f.code == "wrong-routing-table")
    );
    assert!(
        core.findings
            .iter()
            .any(|f| f.wan == "wan2" && f.code == "wrong-wan-default")
    );
}
#[test]
fn pcc_fasttrack_and_jumps_remain_unsupported_and_affect_every_wan() {
    let (s, mut inv) = fixture();
    let i = Arc::make_mut(inv.get_mut("r1").unwrap());
    i.tables
        .get_mut("/ip/firewall/mangle/print")
        .unwrap()
        .push(core_fixture::row(&[
            (".id", "*20"),
            ("chain", "prerouting"),
            ("action", "mark-connection"),
            ("per-connection-classifier", "both-addresses:2/0"),
        ]));
    i.tables
        .get_mut("/ip/firewall/filter/print")
        .unwrap()
        .push(core_fixture::row(&[
            (".id", "*21"),
            ("action", "fasttrack-connection"),
        ]));
    let report = core_diagnostics::review(&s, &inv, &BTreeMap::new(), 101, Origin::Live);
    let core = &report.cores[0];
    assert!(core.wans.iter().all(|w| w.status == Outcome::Unsupported));
    assert!(core.findings.iter().any(|f| f.code == "pcc-policy"));
    assert!(core.findings.iter().any(|f| f.code == "fasttrack"));
    assert_eq!(core.counts.pcc_rules, 1);
    assert_eq!(core.counts.fasttrack_rules, 1);
}
#[test]
fn historical_stale_partial_and_missing_inventory_are_explicit_and_never_look_live() {
    let (s, inv) = fixture();
    let historical = core_diagnostics::review(&s, &inv, &BTreeMap::new(), 101, Origin::Saved);
    assert_eq!(historical.cores[0].health, Health::Historical);
    let stale = core_diagnostics::review(&s, &inv, &BTreeMap::new(), 401, Origin::Live);
    assert_eq!(stale.cores[0].health, Health::Stale);
    let missing =
        core_diagnostics::review(&s, &BTreeMap::new(), &BTreeMap::new(), 101, Origin::Live);
    assert_eq!(missing.cores[0].health, Health::Missing);
    assert!(
        missing.cores[0]
            .wans
            .iter()
            .all(|w| w.status == Outcome::Unavailable)
    );
    let mut partial = inv;
    Arc::make_mut(partial.get_mut("r1").unwrap())
        .tables
        .remove("/ip/firewall/filter/print");
    let report = core_diagnostics::review(&s, &partial, &BTreeMap::new(), 101, Origin::Live);
    assert_eq!(report.cores[0].health, Health::Partial);
    assert!(
        report.cores[0]
            .wans
            .iter()
            .all(|w| w.status == Outcome::Unavailable)
    );
}
#[test]
fn finding_limits_are_explicit_and_never_convert_an_unsupported_policy_to_pass() {
    let (s, mut inv) = fixture();
    let i = Arc::make_mut(inv.get_mut("r1").unwrap());
    i.tables
        .get_mut("/ip/firewall/mangle/print")
        .unwrap()
        .extend((0..1000).map(|n| {
            core_fixture::row(&[
                (".id", &format!("*{:X}", n + 100)),
                ("chain", "prerouting"),
                ("action", "jump"),
            ])
        }));
    let report = core_diagnostics::review(&s, &inv, &BTreeMap::new(), 101, Origin::Live);
    let core = &report.cores[0];
    assert!(core.findings.len() <= 257);
    assert!(core.findings.iter().any(|f| f.code == "finding-limit"));
    assert!(core.wans.iter().all(|w| w.status != Outcome::Pass));
}

#[test]
fn actual_api_presence_only_fib_switch_is_enabled_but_other_empty_flags_are_not_guessed() {
    let (s, mut inv) = fixture();
    for row in Arc::make_mut(inv.get_mut("r1").unwrap())
        .tables
        .get_mut("/routing/table/print")
        .unwrap()
    {
        row.insert("fib".into(), String::new());
    }
    let report = core_diagnostics::review(&s, &inv, &BTreeMap::new(), 101, Origin::Live);
    assert!(
        report.cores[0]
            .wans
            .iter()
            .all(|w| w.status == Outcome::Pass)
    );
    Arc::make_mut(inv.get_mut("r1").unwrap())
        .tables
        .get_mut("/routing/table/print")
        .unwrap()[0]
        .insert("invalid".into(), String::new());
    let report = core_diagnostics::review(&s, &inv, &BTreeMap::new(), 101, Origin::Live);
    assert_eq!(
        report.cores[0]
            .wans
            .iter()
            .find(|w| w.wan == "wan")
            .unwrap()
            .status,
        Outcome::Conflict
    );
}

#[test]
fn unknown_filter_actions_cannot_receive_a_structural_pass() {
    let (s, mut inv) = fixture();
    Arc::make_mut(inv.get_mut("r1").unwrap())
        .tables
        .get_mut("/ip/firewall/filter/print")
        .unwrap()
        .push(core_fixture::row(&[
            ("action", "future-policy-action"),
            ("disabled", "false"),
        ]));
    let report = core_diagnostics::review(&s, &inv, &BTreeMap::new(), 101, Origin::Live);
    assert!(
        report.cores[0]
            .wans
            .iter()
            .all(|w| w.status == Outcome::Unsupported)
    );
}

#[test]
fn observed_path_limit_is_explicit_instead_of_silently_omitting_wans() {
    let (mut s, mut inv) = fixture();
    s.core_lists.bindings.clear();
    let i = Arc::make_mut(inv.get_mut("r1").unwrap());
    let template = i.tables["/ip/route/print"][0].clone();
    i.tables.insert(
        "/ip/route/print".into(),
        (0..513)
            .map(|n| {
                let mut row = template.clone();
                row.insert("routing-table".into(), format!("observed-table-{n}"));
                row
            })
            .collect(),
    );
    let report = core_diagnostics::review(&s, &inv, &BTreeMap::new(), 101, Origin::Live);
    let core = &report.cores[0];
    assert_eq!(core.observed_paths_omitted, 1);
    assert_eq!(core.wans.len(), 512);
    assert!(
        core.findings
            .iter()
            .any(|f| f.code == "observed-path-limit")
    );
    assert!(core.wans.iter().all(|w| w.status != Outcome::Pass));
}

#[test]
#[ignore = "manual indexed policy performance measurement"]
fn review_1000_wans_and_100k_routes() {
    use std::time::Instant;
    let (mut s, mut inv, _, _) = core_fixture::fixture();
    let root = s.nodes[0].clone();
    s.nodes.clear();
    s.core_lists.bindings.clear();
    let i = Arc::make_mut(inv.get_mut("r1").unwrap());
    for path in [
        "/interface/print",
        "/ip/firewall/mangle/print",
        "/routing/table/print",
        "/ip/route/print",
    ] {
        i.tables.get_mut(path).unwrap().clear();
    }
    for n in 0..1000 {
        let mut wan = root.clone();
        wan.id = format!("wan-{n}");
        wan.source = format!("uplink-{n}");
        let list = format!("list-{n}");
        let table = format!("table-{n}");
        s.core_lists.bindings.push(Binding {
            wan: wan.id.clone(),
            router: "r1".into(),
            list: list.clone(),
            table: table.clone(),
        });
        s.nodes.push(wan.clone());
        i.tables
            .get_mut("/interface/print")
            .unwrap()
            .push(core_fixture::row(&[
                ("name", &wan.source),
                ("running", "true"),
                ("disabled", "false"),
            ]));
        i.tables
            .get_mut("/routing/table/print")
            .unwrap()
            .push(core_fixture::row(&[("name", &table), ("fib", "")]));
        i.tables
            .get_mut("/ip/firewall/mangle/print")
            .unwrap()
            .push(core_fixture::row(&[
                ("chain", "prerouting"),
                ("action", "mark-routing"),
                ("src-address-list", &list),
                ("new-routing-mark", &table),
                ("passthrough", "false"),
            ]));
        i.tables
            .get_mut("/ip/route/print")
            .unwrap()
            .push(core_fixture::row(&[
                ("dst-address", "0.0.0.0/0"),
                ("routing-table", &table),
                ("immediate-gw", &format!("203.0.113.1%{}", wan.source)),
                ("active", "true"),
            ]));
    }
    i.tables
        .get_mut("/ip/route/print")
        .unwrap()
        .extend((0..99000).map(|_| {
            core_fixture::row(&[
                ("dst-address", "192.0.2.0/24"),
                ("routing-table", "main"),
                ("active", "true"),
            ])
        }));
    i.tables
        .get_mut("/ip/firewall/address-list/print")
        .unwrap()
        .extend((0..50000).map(|n| {
            core_fixture::row(&[
                ("list", "manual"),
                (
                    "address",
                    &std::net::Ipv4Addr::from(0x0a000001 + n).to_string(),
                ),
            ])
        }));
    let started = Instant::now();
    let report = core_diagnostics::review(&s, &inv, &BTreeMap::new(), 101, Origin::Live);
    let elapsed = started.elapsed();
    assert_eq!(report.cores[0].wans.len(), 1000);
    assert!(
        report.cores[0]
            .wans
            .iter()
            .all(|w| w.status == Outcome::Pass)
    );
    assert!(report.cores[0].findings.is_empty());
    assert_eq!(report.cores[0].counts.routes, 100000);
    for rows in Arc::make_mut(inv.get_mut("r1").unwrap())
        .tables
        .values_mut()
    {
        rows.reverse();
    }
    assert_eq!(
        report,
        core_diagnostics::review(&s, &inv, &BTreeMap::new(), 101, Origin::Live)
    );
    eprintln!(
        "1000 WANs /100k routes /50k list members: indexed review={elapsed:?}; debug, excludes collection and disk I/O"
    );
}
