use lqosync_v3::{
    core_plan::{self, Action},
    routeros::Inventory,
};
use std::{collections::BTreeMap, sync::Arc};
mod common;
#[path = "common/core.rs"]
mod core_fixture;
use core_fixture::{fixture, owned, row};
fn rows<'a>(
    invs: &'a mut BTreeMap<String, Arc<Inventory>>,
    path: &str,
) -> &'a mut Vec<lqosync_v3::routeros::Row> {
    Arc::make_mut(invs.get_mut("r1").unwrap())
        .tables
        .get_mut(path)
        .unwrap()
}
#[test]
fn exact_ownership_diff_is_deterministic_and_preserves_subscriber_data() {
    let (s, mut inv, mut subs, state) = fixture();
    let before = subs.clone();
    let old_id = uuid::Uuid::new_v5(&uuid::Uuid::NAMESPACE_OID, b"old subscriber").to_string();
    rows(&mut inv, "/ip/firewall/address-list/print").extend([
        owned("*1", &subs[0].ipv4, &subs[0].id, "wan"),
        owned("*2", "192.0.2.99/32", &old_id, "wan"),
        row(&[
            (".id", "*3"),
            ("list", "lqos-wan"),
            ("address", "198.51.100.1"),
            ("comment", "manual"),
            ("dynamic", "false"),
            ("disabled", "false"),
        ]),
    ]);
    let plan = core_plan::build(&s, &subs, &state, &inv, &BTreeMap::new(), 101).unwrap();
    assert_eq!(
        plan.entries
            .iter()
            .filter(|e| e.action == Action::Add)
            .count(),
        2
    );
    assert_eq!(
        plan.entries
            .iter()
            .filter(|e| e.action == Action::Keep)
            .count(),
        1
    );
    let removed: Vec<_> = plan
        .entries
        .iter()
        .filter(|e| e.action == Action::Remove)
        .collect();
    assert_eq!(removed.len(), 1);
    assert_eq!(removed[0].entry_id, "*2");
    assert_eq!(removed[0].subscriber, old_id);
    assert_eq!(plan.protected, 1);
    assert!(plan.entries.iter().all(|e| e.address.ends_with("/32")));
    assert_eq!(before, subs);
    subs.reverse();
    rows(&mut inv, "/ip/firewall/address-list/print").reverse();
    assert_eq!(
        plan,
        core_plan::build(&s, &subs, &state, &inv, &BTreeMap::new(), 101).unwrap()
    );
}
#[test]
fn manual_exact_cidr_range_dns_and_ownership_collisions_block_without_claiming_rows() {
    for address in [
        "192.0.2.1",
        "192.0.2.0/24",
        "192.0.2.1-192.0.2.30",
        "host.example",
    ] {
        let (s, mut inv, subs, state) = fixture();
        rows(&mut inv, "/ip/firewall/address-list/print").push(row(&[
            (".id", "*9"),
            ("list", "lqos-wan"),
            ("address", address),
            ("comment", "manual"),
        ]));
        let e = core_plan::build(&s, &subs, &state, &inv, &BTreeMap::new(), 101)
            .unwrap_err()
            .to_string();
        assert!(e.contains("protected") || e.contains("unresolved"), "{e}");
    }
    for field in ["dynamic", "disabled"] {
        let (s, mut inv, subs, state) = fixture();
        let mut entry = owned("*1", &subs[0].ipv4, &subs[0].id, "wan");
        entry.insert(field.into(), "true".into());
        rows(&mut inv, "/ip/firewall/address-list/print").push(entry);
        assert!(
            core_plan::build(&s, &subs, &state, &inv, &BTreeMap::new(), 101)
                .unwrap_err()
                .to_string()
                .contains("owned")
        );
    }
    let (s, mut inv, subs, state) = fixture();
    let entry = owned("*1", &subs[0].ipv4, &subs[0].id, "wan");
    rows(&mut inv, "/ip/firewall/address-list/print").extend([entry.clone(), entry]);
    assert!(
        core_plan::build(&s, &subs, &state, &inv, &BTreeMap::new(), 101)
            .unwrap_err()
            .to_string()
            .contains("Duplicate")
    );
}
#[test]
fn routing_conflicts_and_unsupported_matchers_are_explicitly_blocked() {
    for (path, key, value, expected) in [
        (
            "/ip/route/print",
            "immediate-gw",
            "203.0.113.1%wrong",
            "WAN",
        ),
        (
            "/ip/route/print",
            "immediate-gw",
            "203.0.113.1%uplink,203.0.113.2%other",
            "WAN",
        ),
        ("/ip/route/print", "active", "false", "default"),
        (
            "/ip/firewall/mangle/print",
            "new-routing-mark",
            "wrong",
            "table",
        ),
        (
            "/ip/firewall/mangle/print",
            "passthrough",
            "true",
            "passthrough",
        ),
        (
            "/ip/firewall/mangle/print",
            "connection-mark",
            "from-pcc",
            "matcher",
        ),
        (
            "/ip/firewall/mangle/print",
            "new-future-matcher",
            "unknown",
            "matcher",
        ),
        ("/routing/table/print", "fib", "false", "FIB"),
        ("/system/resource/print", "version", "6.49.10", "RouterOS 7"),
    ] {
        let (s, mut inv, subs, state) = fixture();
        rows(&mut inv, path)[0].insert(key.into(), value.into());
        let e = core_plan::build(&s, &subs, &state, &inv, &BTreeMap::new(), 101)
            .unwrap_err()
            .to_string();
        assert!(e.contains(expected), "{path}:{key}: {e}");
    }
    for (path, entry) in [
        (
            "/routing/rule/print",
            row(&[
                ("action", "lookup-only-in-table"),
                ("table", "other"),
                ("disabled", "false"),
            ]),
        ),
        (
            "/ip/firewall/filter/print",
            row(&[("action", "fasttrack-connection"), ("disabled", "false")]),
        ),
        (
            "/ip/firewall/mangle/print",
            row(&[
                ("chain", "prerouting"),
                ("action", "jump"),
                ("jump-target", "PCC"),
                ("disabled", "false"),
            ]),
        ),
    ] {
        let (s, mut inv, subs, state) = fixture();
        rows(&mut inv, path).push(entry);
        assert!(core_plan::build(&s, &subs, &state, &inv, &BTreeMap::new(), 101).is_err());
    }
}
#[test]
fn stale_partial_and_inconsistent_inputs_never_create_a_removal_plan() {
    let (s, mut inv, subs, state) = fixture();
    assert!(core_plan::build(&s, &subs, &state, &inv, &BTreeMap::new(), 401).is_err());
    for path in core_plan::CORE_TABLES {
        let mut broken = inv.clone();
        Arc::make_mut(broken.get_mut("r1").unwrap())
            .tables
            .remove(*path);
        assert!(
            core_plan::build(&s, &subs, &state, &broken, &BTreeMap::new(), 101).is_err(),
            "{path}"
        );
    }
    Arc::make_mut(inv.get_mut("r1").unwrap())
        .errors
        .insert("collection".into(), "timeout".into());
    assert!(core_plan::build(&s, &subs, &state, &inv, &BTreeMap::new(), 101).is_err());
    let (s, inv, mut subs, mut state) = fixture();
    state.assignments.values_mut().next().unwrap().node = "other".into();
    assert!(core_plan::build(&s, &subs, &state, &inv, &BTreeMap::new(), 101).is_err());
    let (s, inv, _, state) = fixture();
    subs[0].ipv4 = "192.0.2.0/24".into();
    assert!(core_plan::build(&s, &subs, &state, &inv, &BTreeMap::new(), 101).is_err());
}
#[test]
fn static_policy_rejects_duplicate_lists_tables_and_wrong_core() {
    let (mut s, _, _, _) = fixture();
    let mut another = s.nodes[0].clone();
    another.id = "wan2".into();
    another.source = "other-uplink".into();
    s.nodes.push(another);
    let mut binding = s.core_lists.bindings[0].clone();
    binding.wan = "wan2".into();
    s.core_lists.bindings.push(binding);
    assert!(lqosync_v3::model::validate(&s).is_err());
    s.core_lists.bindings[1].list = "other".into();
    assert!(lqosync_v3::model::validate(&s).is_err());
    s.core_lists.bindings[1].table = "other".into();
    assert!(lqosync_v3::model::validate(&s).is_ok());
    s.routers[0].role = "distro".into();
    assert!(lqosync_v3::model::validate(&s).is_err());
}

#[test]
fn malformed_markers_do_not_claim_rows_and_other_scopes_are_preserved() {
    let (s, mut inv, subs, state) = fixture();
    let mut invalid = owned("*9", "192.0.2.99", &subs[0].id, "wan");
    invalid.insert(
        "comment".into(),
        format!("lqosync3:{}:bad-id:wan", s.core_lists.owner),
    );
    rows(&mut inv, "/ip/firewall/address-list/print").push(invalid);
    assert!(
        core_plan::build(&s, &subs, &state, &inv, &BTreeMap::new(), 101)
            .unwrap_err()
            .to_string()
            .contains("Malformed")
    );
    rows(&mut inv, "/ip/firewall/address-list/print").clear();
    let mut retired = owned("*1", &subs[0].ipv4, &subs[0].id, "retired-wan");
    retired.insert("list".into(), "retired-list".into());
    let mut foreign = owned("*2", "198.51.100.9", &subs[0].id, "wan");
    foreign.insert(
        "comment".into(),
        core_plan::marker("a68d3230-0265-40dc-8c0b-572808cb51aa", &subs[0].id, "wan"),
    );
    rows(&mut inv, "/ip/firewall/address-list/print").extend([retired, foreign]);
    let plan = core_plan::build(&s, &subs, &state, &inv, &BTreeMap::new(), 101).unwrap();
    assert_eq!(plan.protected, 1);
    assert!(plan.entries.iter().all(|e| e.action == Action::Add));
}

#[test]
fn pinned_wan_move_plans_only_the_owned_old_host_removal_and_new_list_add() {
    use lqosync_v3::{
        assignment::{self, Pin, Pool},
        core_plan::Binding,
        normalizer,
    };
    let (mut s, mut inv, _, previous) = fixture();
    let mut wan = s.nodes[0].clone();
    wan.id = "wan2".into();
    wan.source = "uplink2".into();
    s.nodes.push(wan);
    let mut node = s.nodes[1].clone();
    node.id = "ppp2".into();
    node.parent = "wan2".into();
    node.router.clear();
    node.source.clear();
    s.nodes.push(node);
    s.core_lists.bindings.push(Binding {
        wan: "wan2".into(),
        router: "r1".into(),
        list: "lqos-wan2".into(),
        table: "to-wan2".into(),
    });
    s.assignment.pools.push(Pool {
        source_node: "ppp".into(),
        targets: vec!["ppp".into(), "ppp2".into()],
        balance: false,
    });
    rows(&mut inv, "/interface/print").push(row(&[
        ("name", "uplink2"),
        ("running", "true"),
        ("disabled", "false"),
    ]));
    rows(&mut inv, "/routing/table/print").push(row(&[("name", "to-wan2"), ("fib", "true")]));
    let mut rule = rows(&mut inv, "/ip/firewall/mangle/print")[0].clone();
    rule.insert("src-address-list".into(), "lqos-wan2".into());
    rule.insert("new-routing-mark".into(), "to-wan2".into());
    rows(&mut inv, "/ip/firewall/mangle/print").push(rule);
    let mut route = rows(&mut inv, "/ip/route/print")[0].clone();
    route.insert("routing-table".into(), "to-wan2".into());
    route.insert("immediate-gw".into(), "203.0.113.2%uplink2".into());
    rows(&mut inv, "/ip/route/print").push(route);
    let mut subs = normalizer::normalize(&s, &inv, &BTreeMap::new(), 101).unwrap();
    let alice = subs.iter().find(|s| s.node == "ppp").unwrap().clone();
    s.assignment.pins.insert(
        alice.id.clone(),
        Pin {
            source_node: "ppp".into(),
            node: "ppp2".into(),
        },
    );
    rows(&mut inv, "/ip/firewall/address-list/print").push(owned(
        "*1",
        &alice.ipv4,
        &alice.id,
        "wan",
    ));
    let state = assignment::assign(&s, &mut subs, &previous, &inv, &BTreeMap::new(), 101).unwrap();
    let plan = core_plan::build(&s, &subs, &state, &inv, &BTreeMap::new(), 101).unwrap();
    let changes: Vec<_> = plan
        .entries
        .iter()
        .filter(|e| e.subscriber == alice.id)
        .collect();
    assert_eq!(changes.len(), 2);
    assert!(
        changes
            .iter()
            .any(|e| e.action == Action::Remove && e.list == "lqos-wan" && e.entry_id == "*1")
    );
    assert!(
        changes
            .iter()
            .any(|e| e.action == Action::Add && e.list == "lqos-wan2" && e.node == "ppp2")
    );
    core_plan::validate_snapshot(&plan, &subs).unwrap();
    assert_eq!(
        subs.iter().find(|s| s.id == alice.id).unwrap().download,
        alice.download
    );
}

#[test]
#[ignore = "manual synthetic performance measurement"]
fn plan_50k_subscribers_and_60k_existing_members() {
    use std::time::Instant;
    let (s, mut inv, original, _) = fixture();
    let template = original[0].clone();
    let mut subs: Vec<_> = (0..50000)
        .map(|n| {
            let mut sub = template.clone();
            sub.id =
                uuid::Uuid::new_v5(&uuid::Uuid::NAMESPACE_OID, format!("bench-{n}").as_bytes())
                    .to_string();
            sub.ipv4 = std::net::Ipv4Addr::from(0x0a000001 + n).to_string();
            sub
        })
        .collect();
    let state = lqosync_v3::assignment::assign(
        &s,
        &mut subs,
        &Default::default(),
        &inv,
        &BTreeMap::new(),
        101,
    )
    .unwrap();
    let members = rows(&mut inv, "/ip/firewall/address-list/print");
    for (n, sub) in subs.iter().take(25000).enumerate() {
        members.push(owned(&format!("*{n:X}"), &sub.ipv4, &sub.id, "wan"));
    }
    for n in 0..10000 {
        members.push(owned(
            &format!("*{:X}", n + 25000),
            &std::net::Ipv4Addr::from(0x64400001 + n).to_string(),
            &uuid::Uuid::new_v5(
                &uuid::Uuid::NAMESPACE_OID,
                format!("offline-{n}").as_bytes(),
            )
            .to_string(),
            "wan",
        ));
    }
    for n in 0..25000 {
        members.push(row(&[
            (".id", &format!("*{:X}", n + 35000)),
            ("list", "lqos-wan"),
            (
                "address",
                &std::net::Ipv4Addr::from(0xc6120001 + n).to_string(),
            ),
            ("comment", "manual"),
        ]));
    }
    let before = subs.clone();
    let start = Instant::now();
    let plan = core_plan::build(&s, &subs, &state, &inv, &BTreeMap::new(), 101).unwrap();
    let build = start.elapsed();
    let start = Instant::now();
    core_plan::validate_snapshot(&plan, &subs).unwrap();
    let integrity = start.elapsed();
    let start = Instant::now();
    let encoded = serde_json::to_vec(&plan).unwrap();
    let encode = start.elapsed();
    assert_eq!(
        plan.entries
            .iter()
            .filter(|e| e.action == Action::Add)
            .count(),
        25000
    );
    assert_eq!(
        plan.entries
            .iter()
            .filter(|e| e.action == Action::Keep)
            .count(),
        25000
    );
    assert_eq!(
        plan.entries
            .iter()
            .filter(|e| e.action == Action::Remove)
            .count(),
        10000
    );
    assert_eq!(plan.protected, 25000);
    assert_eq!(subs, before);
    subs.reverse();
    rows(&mut inv, "/ip/firewall/address-list/print").reverse();
    assert_eq!(
        plan,
        core_plan::build(&s, &subs, &state, &inv, &BTreeMap::new(), 101).unwrap()
    );
    eprintln!(
        "50k subscribers / 60k existing members: plan={build:?}, integrity={integrity:?}, JSON={encode:?} ({} bytes); debug, excludes collection and disk publication",
        encoded.len()
    );
}
