use lqosync_v3::{
    core_diagnostics::{self, Origin, Outcome},
    core_plan,
    routeros::Inventory,
};
use std::{collections::BTreeMap, sync::Arc};
mod common;
#[path = "common/core.rs"]
mod core_fixture;
use core_fixture::row;

fn fixture() -> (
    lqosync_v3::model::Settings,
    BTreeMap<String, Arc<Inventory>>,
) {
    let (s, mut inv, _, _) = core_fixture::fixture();
    let i = Arc::make_mut(inv.get_mut("r1").unwrap());
    i.tables.get_mut("/interface/print").unwrap().push(row(&[
        ("name", "downstream"),
        ("running", "true"),
        ("disabled", "false"),
    ]));
    i.tables.insert(
        "/interface/list/print".into(),
        vec![row(&[
            ("name", "SUBSCRIBERS"),
            ("include", ""),
            ("exclude", ""),
        ])],
    );
    i.tables.insert(
        "/interface/list/member/print".into(),
        vec![row(&[
            ("list", "SUBSCRIBERS"),
            ("interface", "downstream"),
            ("disabled", "false"),
            ("dynamic", "false"),
        ])],
    );
    i.tables
        .get_mut("/ip/firewall/address-list/print")
        .unwrap()
        .push(row(&[
            ("list", "INTERNAL"),
            ("address", "10.0.0.0/8"),
            ("disabled", "false"),
            ("dynamic", "false"),
        ]));
    let routing = &mut i.tables.get_mut("/ip/firewall/mangle/print").unwrap()[0];
    routing.insert("in-interface-list".into(), "SUBSCRIBERS".into());
    routing.insert("dst-address-list".into(), "!INTERNAL".into());
    let rule = i.tables["/ip/firewall/mangle/print"][0].clone();
    i.tables.insert(
        "/ip/firewall/mangle/print".into(),
        vec![
            row(&[
                (".id", "*10"),
                ("chain", "prerouting"),
                ("action", "accept"),
                ("dst-address-list", "INTERNAL"),
            ]),
            row(&[
                (".id", "*11"),
                ("chain", "prerouting"),
                ("action", "accept"),
                ("protocol", "ospf"),
            ]),
            row(&[
                (".id", "*12"),
                ("chain", "prerouting"),
                ("action", "accept"),
                ("protocol", "udp"),
                ("dst-port", "3784,3785"),
            ]),
            rule,
        ],
    );
    (s, inv)
}
fn passing(s: &lqosync_v3::model::Settings, inv: &BTreeMap<String, Arc<Inventory>>) -> bool {
    let r = core_diagnostics::review(s, inv, &BTreeMap::new(), 101, Origin::Live);
    r.cores[0].wans.iter().all(|w| w.status == Outcome::Pass)
}
#[test]
fn exact_static_ingress_and_destination_scope_supports_local_and_control_bypasses() {
    let (s, inv) = fixture();
    assert!(
        passing(&s, &inv),
        "Supported static forwarding scope remained unsupported"
    );
    let (_, _, subs, state) = core_fixture::fixture();
    assert!(core_plan::build(&s, &subs, &state, &inv, &BTreeMap::new(), 101).is_ok());
}
#[test]
fn composed_missing_dynamic_ambiguous_and_wan_ingress_lists_never_pass() {
    for case in 0..9 {
        let (s, mut inv) = fixture();
        let i = Arc::make_mut(inv.get_mut("r1").unwrap());
        match case {
            0 => {
                i.tables.remove("/interface/list/print");
            }
            1 => {
                i.tables.get_mut("/interface/list/print").unwrap()[0]
                    .insert("include".into(), "all".into());
            }
            2 => {
                i.tables.get_mut("/interface/list/print").unwrap()[0]
                    .insert("exclude".into(), "WAN".into());
            }
            3 => {
                i.tables.get_mut("/interface/list/member/print").unwrap()[0]
                    .insert("dynamic".into(), "true".into());
            }
            4 => {
                let r = i.tables["/interface/list/member/print"][0].clone();
                i.tables
                    .get_mut("/interface/list/member/print")
                    .unwrap()
                    .push(r);
            }
            5 => {
                i.tables.get_mut("/interface/list/member/print").unwrap()[0]
                    .insert("interface".into(), "uplink".into());
            }
            6 => {
                i.tables
                    .get_mut("/interface/list/member/print")
                    .unwrap()
                    .clear();
            }
            7 => {
                i.tables.get_mut("/interface/list/member/print").unwrap()[0]
                    .insert("interface".into(), "missing".into());
            }
            _ => {
                i.errors
                    .insert("/interface/list/print".into(), "read failed".into());
            }
        }
        assert!(!passing(&s, &inv), "Unsafe interface scope passed: {case}");
    }
}
#[test]
fn unbounded_mutable_or_unresolved_destination_exclusions_never_pass() {
    for (key, value) in [
        ("address", "0.0.0.0/0"),
        ("address", "host.example.invalid"),
        ("dynamic", "true"),
        ("disabled", "true"),
        ("timeout", "5m"),
    ] {
        let (s, mut inv) = fixture();
        let i = Arc::make_mut(inv.get_mut("r1").unwrap());
        i.tables.get_mut("/ip/firewall/address-list/print").unwrap()[0]
            .insert(key.into(), value.into());
        assert!(
            !passing(&s, &inv),
            "Unsafe destination scope passed: {key}={value}"
        );
    }
}
#[test]
fn combined_destination_exclusions_cannot_hide_an_empty_forwarding_scope() {
    let (s, mut inv) = fixture();
    let i = Arc::make_mut(inv.get_mut("r1").unwrap());
    i.tables.insert(
        "/ip/firewall/address-list/print".into(),
        vec![
            row(&[("list", "INTERNAL"), ("address", "0.0.0.0/1")]),
            row(&[("list", "INTERNAL"), ("address", "128.0.0.0/1")]),
        ],
    );
    assert!(
        !passing(&s, &inv),
        "Exclusion union covered every destination but received a pass"
    );
}
#[test]
fn active_firewall_writers_cannot_make_a_checked_destination_exclusion_mutable() {
    for path in ["/ip/firewall/filter/print", "/ip/firewall/mangle/print"] {
        let (s, mut inv) = fixture();
        let i = Arc::make_mut(inv.get_mut("r1").unwrap());
        i.tables.get_mut(path).unwrap().push(row(&[
            ("chain", "forward"),
            ("action", "add-src-to-address-list"),
            ("address-list", "INTERNAL"),
        ]));
        assert!(
            !passing(&s, &inv),
            "Destination-list writer was ignored in {path}"
        );
    }
    let (s, mut inv) = fixture();
    let i = Arc::make_mut(inv.get_mut("r1").unwrap());
    i.tables
        .get_mut("/ip/firewall/filter/print")
        .unwrap()
        .push(row(&[
            ("chain", "forward"),
            ("action", "add-dst-to-address-list"),
            ("address-list", "INTERNAL"),
            ("disabled", "true"),
        ]));
    assert!(
        passing(&s, &inv),
        "Disabled writer poisoned the active scope"
    );
}
#[test]
fn unknown_conditions_broad_accepts_and_wrong_bypass_order_remain_blocked() {
    for case in 0..8 {
        let (s, mut inv) = fixture();
        let i = Arc::make_mut(inv.get_mut("r1").unwrap());
        let rules = i.tables.get_mut("/ip/firewall/mangle/print").unwrap();
        match case {
            0 => {
                rules[0].remove("dst-address-list");
            }
            1 => {
                rules[1].insert("protocol".into(), "tcp".into());
            }
            2 => {
                rules[2].insert("dst-port".into(), "53".into());
            }
            3 => {
                rules[0].insert("src-address-list".into(), "extra".into());
            }
            4 => {
                rules.swap(0, 3);
            }
            5 => {
                rules[3].insert("dst-address-list".into(), "INTERNAL".into());
            }
            6 => {
                rules[3].insert("in-interface-list".into(), "!SUBSCRIBERS".into());
            }
            _ => {
                rules[3].insert("src-port".into(), "1234".into());
            }
        }
        assert!(
            !passing(&s, &inv),
            "Unsupported bypass/condition passed: {case}"
        );
    }
}
#[test]
fn rejected_scope_candidates_receive_an_explicit_budget_finding() {
    let (s, mut inv) = fixture();
    let i = Arc::make_mut(inv.get_mut("r1").unwrap());
    let mut route = i.tables["/ip/firewall/mangle/print"][3].clone();
    route.insert("in-interface-list".into(), "missing".into());
    i.tables
        .insert("/ip/firewall/mangle/print".into(), vec![route; 513]);
    let report = core_diagnostics::review(&s, &inv, &BTreeMap::new(), 101, Origin::Live);
    assert!(
        report.cores[0]
            .findings
            .iter()
            .any(|f| f.code == "scope-limit" && f.wan.is_empty())
    );
}
#[test]
#[ignore = "manual scope index performance workload"]
fn review_five_thousand_static_ingress_members() {
    let (s, mut inv) = fixture();
    let i = Arc::make_mut(inv.get_mut("r1").unwrap());
    for n in 0..5000 {
        let name = format!("access-{n}");
        i.tables.get_mut("/interface/print").unwrap().push(row(&[
            ("name", &name),
            ("running", "true"),
            ("disabled", "false"),
        ]));
        i.tables
            .get_mut("/interface/list/member/print")
            .unwrap()
            .push(row(&[("list", "SUBSCRIBERS"), ("interface", &name)]));
    }
    let started = std::time::Instant::now();
    let report = core_diagnostics::review(&s, &inv, &BTreeMap::new(), 101, Origin::Live);
    eprintln!("5001-member scope review: {:?}", started.elapsed());
    assert_eq!(
        report.cores[0].forward_scopes[0].ingress_interfaces.len(),
        5001
    );
    assert!(report.cores[0].findings.is_empty());
}
