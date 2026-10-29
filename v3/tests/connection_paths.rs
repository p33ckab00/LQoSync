use lqosync_v3::{
    core_diagnostics::{self, Origin, Outcome},
    core_plan,
    routeros::{Inventory, Row},
};
use std::{collections::BTreeMap, sync::Arc};
mod common;
#[path = "common/core.rs"]
mod core_fixture;
fn classifier(reply: bool) -> Row {
    let mut row = core_fixture::row(&[
        (".id", "*C"),
        ("chain", "prerouting"),
        ("action", "mark-connection"),
        ("connection-state", "new"),
        ("connection-mark", "no-mark"),
        ("new-connection-mark", "wan-connection"),
        ("passthrough", "true"),
    ]);
    row.insert(
        if reply {
            "in-interface"
        } else {
            "src-address-list"
        }
        .into(),
        if reply { "uplink" } else { "lqos-wan" }.into(),
    );
    row
}
fn marked(
    reply: bool,
) -> (
    lqosync_v3::model::Settings,
    BTreeMap<String, Arc<Inventory>>,
    Vec<lqosync_v3::model::Subscriber>,
    lqosync_v3::assignment::AssignmentState,
) {
    let (s, mut inv, subs, state) = core_fixture::fixture();
    let rows = Arc::make_mut(inv.get_mut("r1").unwrap())
        .tables
        .get_mut("/ip/firewall/mangle/print")
        .unwrap();
    let mut route = rows[0].clone();
    route.insert(".id".into(), "*R".into());
    route.insert("connection-mark".into(), "wan-connection".into());
    if reply {
        route.insert("chain".into(), "output".into());
        route.remove("src-address-list");
    } else {
        rows.clear();
    }
    rows.push(classifier(reply));
    rows.push(route);
    (s, inv, subs, state)
}
#[test]
fn source_list_connection_mark_chain_supports_subscriber_plans_and_reports_forward_direction() {
    let (s, inv, subs, state) = marked(false);
    assert!(core_plan::build(&s, &subs, &state, &inv, &BTreeMap::new(), 101).is_ok());
    let report = core_diagnostics::review(&s, &inv, &BTreeMap::new(), 101, Origin::Live);
    let value = serde_json::to_value(&report).unwrap();
    assert_eq!(
        value["cores"][0]["connection_paths"][0]["direction"],
        "subscriber_forward"
    );
    assert_eq!(report.cores[0].wans[0].status, Outcome::Pass);
}
#[test]
fn wan_ingress_to_output_marks_are_router_replies_and_do_not_replace_subscriber_routes() {
    let (s, mut inv, subs, state) = marked(true);
    assert!(core_plan::build(&s, &subs, &state, &inv, &BTreeMap::new(), 101).is_ok());
    let report = core_diagnostics::review(&s, &inv, &BTreeMap::new(), 101, Origin::Live);
    let value = serde_json::to_value(&report).unwrap();
    assert_eq!(
        value["cores"][0]["connection_paths"][0]["direction"],
        "router_reply"
    );
    Arc::make_mut(inv.get_mut("r1").unwrap())
        .tables
        .get_mut("/ip/firewall/mangle/print")
        .unwrap()
        .remove(0);
    assert!(core_plan::build(&s, &subs, &state, &inv, &BTreeMap::new(), 101).is_err());
}
#[test]
fn ambiguous_marks_wrong_interfaces_unknown_matchers_and_rule_order_remain_blocked() {
    for case in [
        "producer-duplicate",
        "consumer-duplicate",
        "wrong-list",
        "wrong-table",
        "unknown-matcher",
        "order",
        "classifier-stop",
        "route-pass",
        "pcc",
        "unknown-flag",
    ] {
        let (s, mut inv, subs, state) = marked(false);
        let rows = Arc::make_mut(inv.get_mut("r1").unwrap())
            .tables
            .get_mut("/ip/firewall/mangle/print")
            .unwrap();
        match case {
            "producer-duplicate" => rows.push(rows[0].clone()),
            "consumer-duplicate" => rows.push(rows[1].clone()),
            "wrong-list" => {
                rows[1].insert("src-address-list".into(), "wrong".into());
            }
            "wrong-table" => {
                rows[1].insert("new-routing-mark".into(), "wrong".into());
            }
            "unknown-matcher" => {
                rows[0].insert("future-matcher".into(), "condition".into());
            }
            "order" => rows.reverse(),
            "classifier-stop" => {
                rows[0].insert("passthrough".into(), "false".into());
            }
            "route-pass" => {
                rows[1].insert("passthrough".into(), "true".into());
            }
            "pcc" => {
                rows[0].insert(
                    "per-connection-classifier".into(),
                    "both-addresses:2/0".into(),
                );
            }
            _ => {
                rows[0].insert("disabled".into(), "unknown".into());
            }
        }
        assert!(
            core_plan::build(&s, &subs, &state, &inv, &BTreeMap::new(), 101).is_err(),
            "{case}"
        );
    }
    for case in [
        "wrong-interface",
        "wrong-table",
        "conditional-output",
        "producer-other-chain",
    ] {
        let (s, mut inv, subs, state) = marked(true);
        let rows = Arc::make_mut(inv.get_mut("r1").unwrap())
            .tables
            .get_mut("/ip/firewall/mangle/print")
            .unwrap();
        match case {
            "wrong-interface" => {
                rows[1].insert("in-interface".into(), "other".into());
            }
            "wrong-table" => {
                rows[2].insert("new-routing-mark".into(), "other".into());
            }
            "conditional-output" => {
                rows[2].insert("protocol".into(), "tcp".into());
            }
            _ => {
                rows[1].insert("chain".into(), "postrouting".into());
            }
        }
        assert!(
            core_plan::build(&s, &subs, &state, &inv, &BTreeMap::new(), 101).is_err(),
            "{case}"
        );
    }
}

#[test]
fn unmodeled_output_stops_and_implicit_mark_clears_cannot_hide_behind_a_valid_link() {
    for reply in [false, true] {
        let (s, mut inv, subs, state) = marked(reply);
        let rows = Arc::make_mut(inv.get_mut("r1").unwrap())
            .tables
            .get_mut("/ip/firewall/mangle/print")
            .unwrap();
        rows.insert(
            0,
            if reply {
                core_fixture::row(&[("chain", "output"), ("action", "accept")])
            } else {
                core_fixture::row(&[("chain", "postrouting"), ("action", "mark-connection")])
            },
        );
        assert!(
            core_plan::build(&s, &subs, &state, &inv, &BTreeMap::new(), 101).is_err(),
            "reply={reply}"
        );
    }
}

fn many_paths(
    count: usize,
) -> (
    lqosync_v3::model::Settings,
    BTreeMap<String, Arc<Inventory>>,
) {
    let (mut s, mut inv, _, _) = core_fixture::fixture();
    let root = s.nodes[0].clone();
    s.core_lists.bindings.clear();
    let i = Arc::make_mut(inv.get_mut("r1").unwrap());
    for path in [
        "/interface/print",
        "/ip/route/print",
        "/routing/table/print",
        "/ip/firewall/mangle/print",
    ] {
        i.tables.get_mut(path).unwrap().clear();
    }
    for n in 0..count {
        let mut wan = root.clone();
        if n > 0 {
            wan.id = format!("wan-{n}");
        }
        wan.source = format!("uplink-{n}");
        if n == 0 {
            s.nodes[0] = wan.clone();
        } else {
            s.nodes.push(wan.clone());
        }
        let list = format!("list-{n}");
        let table = format!("table-{n}");
        let mark = format!("mark-{n}");
        s.core_lists.bindings.push(lqosync_v3::core_plan::Binding {
            wan: wan.id,
            router: "r1".into(),
            list: list.clone(),
            table: table.clone(),
        });
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
            .get_mut("/ip/route/print")
            .unwrap()
            .push(core_fixture::row(&[
                ("dst-address", "0.0.0.0/0"),
                ("routing-table", &table),
                ("active", "true"),
                ("immediate-gw", &format!("203.0.113.1%{}", wan.source)),
            ]));
        let mut producer = classifier(false);
        producer.insert(".id".into(), format!("*C{n}"));
        producer.insert("src-address-list".into(), list.clone());
        producer.insert("new-connection-mark".into(), mark.clone());
        i.tables
            .get_mut("/ip/firewall/mangle/print")
            .unwrap()
            .extend([
                producer,
                core_fixture::row(&[
                    (".id", &format!("*R{n}")),
                    ("action", "mark-routing"),
                    ("chain", "prerouting"),
                    ("connection-mark", &mark),
                    ("src-address-list", &list),
                    ("new-routing-mark", &table),
                    ("passthrough", "false"),
                ]),
            ]);
    }
    (s, inv)
}
#[test]
fn connection_path_limits_are_visible_and_cannot_create_a_wan_pass() {
    let (s, inv) = many_paths(513);
    let report = core_diagnostics::review(&s, &inv, &BTreeMap::new(), 101, Origin::Live);
    assert_eq!(report.cores[0].connection_paths.len(), 512);
    assert!(
        report.cores[0]
            .findings
            .iter()
            .any(|f| f.code == "connection-path-limit")
    );
    assert!(
        report.cores[0]
            .wans
            .iter()
            .all(|w| w.status != Outcome::Pass)
    );
}
#[test]
#[ignore = "manual connection path/index performance measurement"]
fn review_500_marked_wans_and_100k_routes() {
    let (s, mut inv) = many_paths(500);
    let rows = Arc::make_mut(inv.get_mut("r1").unwrap())
        .tables
        .get_mut("/ip/route/print")
        .unwrap();
    rows.extend((0..99_500).map(|n| {
        core_fixture::row(&[
            ("routing-table", "main"),
            (
                "dst-address",
                &format!("10.{}.{}.{}/32", n / 65536, (n / 256) % 256, n % 256),
            ),
            ("active", "true"),
        ])
    }));
    let start = std::time::Instant::now();
    let report = core_diagnostics::review(&s, &inv, &BTreeMap::new(), 101, Origin::Live);
    let elapsed = start.elapsed();
    assert!(report.cores[0].findings.is_empty());
    assert_eq!(report.cores[0].connection_paths.len(), 500);
    assert!(
        report.cores[0]
            .wans
            .iter()
            .all(|w| w.status == Outcome::Pass)
    );
    for path in [
        "/interface/print",
        "/routing/table/print",
        "/ip/route/print",
    ] {
        Arc::make_mut(inv.get_mut("r1").unwrap())
            .tables
            .get_mut(path)
            .unwrap()
            .reverse();
    }
    assert_eq!(
        report,
        core_diagnostics::review(&s, &inv, &BTreeMap::new(), 101, Origin::Live)
    );
    eprintln!("500 marked WANs /100k routes: {elapsed:?}; debug, excludes collection and disk I/O");
}

#[test]
fn review_rejects_reply_classifier_that_excludes_local_destinations() {
    let (s, mut inv, subs, state) = marked(true);
    Arc::make_mut(inv.get_mut("r1").unwrap())
        .tables
        .get_mut("/ip/firewall/mangle/print")
        .unwrap()[1]
        .insert("dst-address-type".into(), "!local".into());
    assert!(core_plan::build(&s, &subs, &state, &inv, &BTreeMap::new(), 101).is_err());
    let report = core_diagnostics::review(&s, &inv, &BTreeMap::new(), 101, Origin::Live);
    assert!(
        report.cores[0]
            .connection_paths
            .iter()
            .all(|p| p.status != Outcome::Pass)
    );
}
