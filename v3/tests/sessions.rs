use lqosync_v3::{
    assignment::{AssignmentState, Pin, Pool},
    model::{Settings, Subscriber},
    normalizer,
    routeros::Inventory,
    sessions::{self, Observation},
};
use std::{collections::BTreeMap, sync::Arc};
mod common;

#[test]
#[ignore = "manual session persistence and assignment performance workload"]
fn online_session_pipeline_50k() {
    use std::time::Instant;
    let (mut settings, mut inventories) = fixture();
    for node in settings.nodes.iter_mut().filter(|n| n.kind == "wan") {
        node.download = 10000.;
        node.upload = 10000.;
    }
    let inv = Arc::make_mut(inventories.get_mut("r1").unwrap());
    inv.tables.insert(
        "/ppp/active/print".into(),
        (0..50_000u32)
            .map(|i| {
                [
                    ("name".into(), format!("bench-{i}")),
                    (
                        "address".into(),
                        std::net::Ipv4Addr::from(0x0a000001 + i).to_string(),
                    ),
                    (".id".into(), format!("*{:X}", i + 1)),
                    ("uptime".into(), "1h".into()),
                ]
                .into_iter()
                .collect()
            })
            .collect(),
    );
    inv.tables.insert(
        "/ppp/secret/print".into(),
        (0..50_000u32)
            .map(|i| {
                [
                    ("name".into(), format!("bench-{i}")),
                    ("profile".into(), "Plan".into()),
                ]
                .into_iter()
                .collect()
            })
            .collect(),
    );
    let start = Instant::now();
    let mut rows = collect(&settings, &inventories, 101);
    let collect_elapsed = start.elapsed();
    let start = Instant::now();
    let first = sessions::plan(
        &settings,
        &mut rows,
        &AssignmentState::default(),
        None,
        &inventories,
        &BTreeMap::new(),
        101,
    )
    .unwrap();
    let assign_elapsed = start.elapsed();
    assert_eq!(first.active.len(), 50_000);
    assert_eq!(rows.iter().filter(|s| s.node == "ppp").count(), 25_000);
    let dir = tempfile::tempdir().unwrap();
    let start = Instant::now();
    sessions::save(dir.path(), &first).unwrap();
    let save_elapsed = start.elapsed();
    let start = Instant::now();
    let loaded = sessions::load(dir.path()).unwrap().unwrap();
    let load_elapsed = start.elapsed();
    let mut next = collect(&settings, &inventories, 101);
    let start = Instant::now();
    let retained = sessions::plan(
        &settings,
        &mut next,
        &AssignmentState::default(),
        Some(&loaded),
        &inventories,
        &BTreeMap::new(),
        101,
    )
    .unwrap();
    let retained_elapsed = start.elapsed();
    assert_eq!(retained.assignments, first.assignments);
    lqosync_v3::minimums::allocate(&settings, &mut rows).unwrap();
    assert_eq!(
        lqosync_v3::model::compile(&settings, &rows)
            .unwrap()
            .csv
            .lines()
            .count(),
        50_001
    );
    eprintln!(
        "50k online clients/2 WANs debug: collect={collect_elapsed:?}, assign={assign_elapsed:?}, retain={retained_elapsed:?}, candidate_save={save_elapsed:?}, candidate_load={load_elapsed:?}"
    );
}
type Inventories = BTreeMap<String, Arc<Inventory>>;
fn fixture() -> (Settings, Inventories) {
    let (mut s, mut inventories) = common::fixture();
    s.nodes.retain(|n| matches!(n.id.as_str(), "wan" | "ppp"));
    s.nodes[0].router = "r1".into();
    s.nodes[0].source = "wan-a".into();
    s.nodes[0].download = 100.;
    s.nodes[0].upload = 100.;
    s.nodes[1].mode = "computed".into();
    let mut wan = s.nodes[0].clone();
    wan.id = "wan2".into();
    wan.source = "wan-b".into();
    wan.download = 900.;
    wan.upload = 900.;
    let mut target = s.nodes[1].clone();
    target.id = "ppp2".into();
    target.parent = "wan2".into();
    target.router.clear();
    target.source.clear();
    s.nodes.extend([wan, target]);
    s.assignment.pools.push(Pool {
        source_node: "ppp".into(),
        targets: vec!["ppp".into(), "ppp2".into()],
        balance: true,
        metric: "online_clients".into(),
    });
    let inv = Arc::make_mut(inventories.get_mut("r1").unwrap());
    inv.tables.insert(
        "/interface/print".into(),
        ["wan-a", "wan-b"]
            .map(|name| {
                [
                    ("name".into(), name.into()),
                    ("running".into(), "true".into()),
                    ("disabled".into(), "false".into()),
                ]
                .into_iter()
                .collect()
            })
            .to_vec(),
    );
    inv.tables.get_mut("/ppp/active/print").unwrap()[0]
        .extend([(".id".into(), "*1".into()), ("uptime".into(), "1h".into())]);
    (s, inventories)
}
fn collect(s: &Settings, i: &Inventories, now: u64) -> Vec<Subscriber> {
    normalizer::collect(s, i, &BTreeMap::new(), now).unwrap()
}
fn plan(
    s: &Settings,
    i: &Inventories,
    p: Option<&Observation>,
    c: &AssignmentState,
    now: u64,
) -> Observation {
    sessions::plan(s, &mut collect(s, i, now), c, p, i, &BTreeMap::new(), now).unwrap()
}
fn advance(i: &mut Inventories, epoch: u64, handle: &str, uptime: &str) {
    let inv = Arc::make_mut(i.get_mut("r1").unwrap());
    inv.collected_at = epoch;
    if let Some(row) = inv.tables.get_mut("/ppp/active/print").unwrap().first_mut() {
        row.insert(".id".into(), handle.into());
        row.insert("uptime".into(), uptime.into());
    }
}
fn committed_on(target: &str) -> AssignmentState {
    AssignmentState {
        assignments: [(
            normalizer::subscriber_id("r1", "ppp", "alice").unwrap(),
            Pin {
                source_node: "ppp".into(),
                node: target.into(),
            },
        )]
        .into_iter()
        .collect(),
    }
}
#[test]
fn migration_and_continuous_sessions_keep_parent() {
    let (mut s, mut i) = fixture();
    let c = committed_on("ppp2");
    let first = plan(&s, &i, None, &c, 101);
    assert_eq!(first.assignments, c);
    assert_eq!(first.active.values().next().unwrap().uptime, Some(3600));
    advance(&mut i, 102, "*1", "1h1m");
    s.nodes[0].download = 10000.;
    s.nodes[0].upload = 10000.;
    let second = plan(&s, &i, Some(&first), &c, 103);
    assert_eq!(second.assignments, first.assignments);
    assert_eq!(plan(&s, &i, Some(&second), &c, 103), second);
}
#[test]
fn count_balancing_ignores_unequal_wan_capacity() {
    let (s, mut i) = fixture();
    let inv = Arc::make_mut(i.get_mut("r1").unwrap());
    inv.tables.insert(
        "/ppp/active/print".into(),
        (0..20)
            .map(|j| {
                [
                    ("name".into(), format!("client{j}")),
                    ("address".into(), format!("192.0.2.{}", j + 1)),
                    (".id".into(), format!("*{:X}", j + 1)),
                ]
                .into_iter()
                .collect()
            })
            .collect(),
    );
    inv.tables.insert(
        "/ppp/secret/print".into(),
        (0..20)
            .map(|j| {
                [
                    ("name".into(), format!("client{j}")),
                    ("profile".into(), "Plan".into()),
                ]
                .into_iter()
                .collect()
            })
            .collect(),
    );
    let result = plan(&s, &i, None, &AssignmentState::default(), 101);
    assert_eq!(
        result
            .assignments
            .assignments
            .values()
            .filter(|p| p.node == "ppp")
            .count(),
        10
    );
}
#[test]
fn healthy_absence_survives_restart_and_reconnect_can_move() {
    let (s, mut i) = fixture();
    let c = committed_on("ppp2");
    let first = plan(&s, &i, None, &c, 101);
    let row = Arc::make_mut(i.get_mut("r1").unwrap())
        .tables
        .get_mut("/ppp/active/print")
        .unwrap()
        .remove(0);
    advance(&mut i, 102, "*1", "1h");
    let absent = plan(&s, &i, Some(&first), &c, 103);
    assert!(absent.active.is_empty());
    assert!(absent.assignments.assignments.is_empty());
    let dir = tempfile::tempdir().unwrap();
    sessions::save(dir.path(), &absent).unwrap();
    let restored = sessions::load(dir.path()).unwrap().unwrap();
    assert_eq!(restored, absent);
    Arc::make_mut(i.get_mut("r1").unwrap())
        .tables
        .get_mut("/ppp/active/print")
        .unwrap()
        .push(row);
    advance(&mut i, 104, "*2", "2s");
    assert_eq!(
        plan(&s, &i, Some(&restored), &c, 105)
            .assignments
            .assignments
            .values()
            .next()
            .unwrap()
            .node,
        "ppp"
    );
}
#[test]
fn handle_changes_and_uptime_resets_release_parent() {
    for (handle, uptime) in [("*2", "1h1m"), ("*1", "30s")] {
        let (s, mut i) = fixture();
        let c = committed_on("ppp2");
        let first = plan(&s, &i, None, &c, 101);
        advance(&mut i, 102, handle, uptime);
        assert_eq!(
            plan(&s, &i, Some(&first), &c, 103)
                .assignments
                .assignments
                .values()
                .next()
                .unwrap()
                .node,
            "ppp"
        );
    }
}
#[test]
fn pins_win_after_reconnect() {
    let (mut s, mut i) = fixture();
    let c = committed_on("ppp2");
    let first = plan(&s, &i, None, &c, 101);
    s.assignment.pins = c.assignments.clone();
    advance(&mut i, 102, "*2", "1s");
    assert_eq!(plan(&s, &i, Some(&first), &c, 103).assignments, c);
}
#[test]
fn failed_stale_partial_out_of_order_reads_preserve_input() {
    let (s, i) = fixture();
    let first = plan(&s, &i, None, &committed_on("ppp2"), 101);
    for failure in 0..6 {
        let mut bad = i.clone();
        let mut errors = BTreeMap::new();
        let inv = Arc::make_mut(bad.get_mut("r1").unwrap());
        match failure {
            1 => {
                errors.insert("r1".into(), "failed".into());
            }
            2 => {
                inv.errors
                    .insert("/ppp/active/print".into(), "partial".into());
            }
            3 => {
                inv.tables.remove("/ppp/active/print");
            }
            4 => {
                inv.collected_at = 99;
            }
            5 => {
                inv.errors.insert("collection".into(), "partial".into());
            }
            _ => {}
        }
        let mut subs = collect(&s, &i, 101);
        let original = subs.clone();
        assert!(
            sessions::plan(
                &s,
                &mut subs,
                &AssignmentState::default(),
                Some(&first),
                &bad,
                &errors,
                if failure == 0 { 401 } else { 101 }
            )
            .is_err(),
            "failure {failure}"
        );
        assert_eq!(subs, original);
    }
}
#[test]
fn missing_handle_invalid_uptime_and_selected_duplicates_fail() {
    for failure in 0..3 {
        let (s, mut i) = fixture();
        let mut subs = collect(&s, &i, 101);
        let rows = Arc::make_mut(i.get_mut("r1").unwrap())
            .tables
            .get_mut("/ppp/active/print")
            .unwrap();
        match failure {
            0 => {
                rows[0].remove(".id");
            }
            1 => {
                rows[0].insert("uptime".into(), "nonsense".into());
            }
            _ => rows.push(rows[0].clone()),
        }
        assert!(
            sessions::plan(
                &s,
                &mut subs,
                &AssignmentState::default(),
                None,
                &i,
                &BTreeMap::new(),
                101
            )
            .is_err()
        );
    }
}
#[test]
fn candidate_save_is_durable_idempotent_and_malformed_state_fails() {
    let dir = tempfile::tempdir().unwrap();
    assert!(sessions::load(dir.path()).unwrap().is_none());
    let (s, i) = fixture();
    let state = plan(&s, &i, None, &AssignmentState::default(), 101);
    sessions::save(dir.path(), &state).unwrap();
    let path = dir.path().join("state/online-assignments.toml");
    let before = std::fs::metadata(&path).unwrap().modified().unwrap();
    sessions::save(dir.path(), &state).unwrap();
    assert_eq!(
        std::fs::metadata(&path).unwrap().modified().unwrap(),
        before
    );
    assert_eq!(sessions::load(dir.path()).unwrap().unwrap(), state);
    assert!(!dir.path().join("state/current.toml").exists());
    for text in [
        "not toml!",
        "unexpected = true",
        "[assignments.assignments.bad]\nsource_node='ppp'\nnode='ppp2'",
        "[collected]\n'../r1'=100",
    ] {
        std::fs::write(&path, text).unwrap();
        assert!(sessions::load(dir.path()).is_err(), "{text}");
    }
}

#[test]
fn unrelated_bad_session_rows_do_not_block_selected_sessions() {
    let (s, mut i) = fixture();
    let mut subscribers = collect(&s, &i, 101);
    Arc::make_mut(i.get_mut("r1").unwrap())
        .tables
        .get_mut("/ppp/active/print")
        .unwrap()
        .push(
            [
                ("name".into(), "unselected".into()),
                ("uptime".into(), "invalid".into()),
            ]
            .into_iter()
            .collect(),
        );
    assert_eq!(
        sessions::plan(
            &s,
            &mut subscribers,
            &AssignmentState::default(),
            None,
            &i,
            &BTreeMap::new(),
            101
        )
        .unwrap()
        .active
        .len(),
        1
    );
}
#[test]
fn distinct_successful_reads_in_one_second_can_observe_reconnect_and_absence() {
    let (s, mut i) = fixture();
    let first = plan(&s, &i, None, &committed_on("ppp2"), 101);
    advance(&mut i, 100, "*2", "1s");
    let changed = plan(&s, &i, Some(&first), &first.assignments, 101);
    assert_ne!(changed.assignments, first.assignments);
    Arc::make_mut(i.get_mut("r1").unwrap())
        .tables
        .get_mut("/ppp/active/print")
        .unwrap()
        .clear();
    let absent = plan(&s, &i, Some(&changed), &changed.assignments, 101);
    assert!(absent.active.is_empty());
}
#[test]
fn planned_load_assignments_use_previous_candidate_and_keep_offline_history() {
    let (mut s, mut i) = fixture();
    s.assignment.pools[0].metric = "planned_load".into();
    let first = plan(&s, &i, None, &committed_on("ppp2"), 101);
    assert!(first.active.is_empty());
    Arc::make_mut(i.get_mut("r1").unwrap())
        .tables
        .get_mut("/ppp/active/print")
        .unwrap()
        .clear();
    advance(&mut i, 102, "*1", "1h");
    assert_eq!(
        plan(&s, &i, Some(&first), &AssignmentState::default(), 103).assignments,
        first.assignments
    );
}
#[test]
fn healthy_empty_input_still_checks_active_tables_and_wan_health() {
    let (s, mut i) = fixture();
    let first = plan(&s, &i, None, &committed_on("ppp2"), 101);
    let inv = Arc::make_mut(i.get_mut("r1").unwrap());
    inv.collected_at = 102;
    inv.tables.remove("/ppp/active/print");
    assert!(
        sessions::plan(
            &s,
            &mut [],
            &first.assignments,
            Some(&first),
            &i,
            &BTreeMap::new(),
            103
        )
        .is_err()
    );
    let inv = Arc::make_mut(i.get_mut("r1").unwrap());
    inv.tables.insert("/ppp/active/print".into(), vec![]);
    inv.tables.get_mut("/interface/print").unwrap()[0].insert("running".into(), "false".into());
    assert!(
        sessions::plan(
            &s,
            &mut [],
            &first.assignments,
            Some(&first),
            &i,
            &BTreeMap::new(),
            103
        )
        .is_err()
    );
}
#[test]
fn dhcp_bound_lease_presence_uses_active_identity_and_rejects_unknown_status() {
    let (mut s, mut i) = common::fixture();
    s.nodes.retain(|n| matches!(n.id.as_str(), "wan" | "dhcp"));
    s.nodes[0].router = "r1".into();
    s.nodes[0].source = "wan-a".into();
    s.assignment.pools.push(Pool {
        source_node: "dhcp".into(),
        targets: vec!["dhcp".into()],
        balance: true,
        metric: "online_clients".into(),
    });
    let inv = Arc::make_mut(i.get_mut("r1").unwrap());
    inv.tables.insert(
        "/interface/print".into(),
        vec![
            [
                ("name".into(), "wan-a".into()),
                ("running".into(), "true".into()),
                ("disabled".into(), "false".into()),
            ]
            .into_iter()
            .collect(),
        ],
    );
    inv.tables.get_mut("/ip/dhcp-server/lease/print").unwrap()[0].extend([
        ("server".into(), "all".into()),
        ("active-server".into(), "dhcp1".into()),
        ("mac-address".into(), "02:00:00:00:00:99".into()),
        ("active-mac-address".into(), "02-00-00-00-00-02".into()),
        ("address".into(), "0.0.0.0".into()),
        ("active-address".into(), "192.0.2.2".into()),
    ]);
    let first = plan(&s, &i, None, &AssignmentState::default(), 101);
    assert_eq!(
        first.active.keys().next().unwrap(),
        &normalizer::subscriber_id("r1", "dhcp", "02:00:00:00:00:02").unwrap()
    );
    let session = first.active.values().next().unwrap();
    assert_eq!((session.id.as_deref(), session.uptime), (None, None));
    let leases = Arc::make_mut(i.get_mut("r1").unwrap())
        .tables
        .insert("/ip/dhcp-server/lease/print".into(), vec![])
        .unwrap();
    let absent = plan(&s, &i, Some(&first), &first.assignments, 101);
    assert!(absent.active.is_empty());
    Arc::make_mut(i.get_mut("r1").unwrap())
        .tables
        .insert("/ip/dhcp-server/lease/print".into(), leases);
    let returning = plan(&s, &i, Some(&absent), &first.assignments, 101);
    assert_eq!(returning.active.len(), 1);
    let mut subscribers = collect(&s, &i, 101);
    let inv = Arc::make_mut(i.get_mut("r1").unwrap());
    inv.collected_at = 102;
    inv.tables.get_mut("/ip/dhcp-server/lease/print").unwrap()[0].remove("status");
    assert!(
        sessions::plan(
            &s,
            &mut subscribers,
            &first.assignments,
            Some(&first),
            &i,
            &BTreeMap::new(),
            103
        )
        .is_err()
    );
}
#[test]
fn hotspot_handles_and_clock_uptime_are_observed() {
    let (mut s, mut i) = common::fixture();
    s.nodes.retain(|n| matches!(n.id.as_str(), "wan" | "hs"));
    s.nodes[0].router = "r1".into();
    s.nodes[0].source = "wan-a".into();
    s.assignment.pools.push(Pool {
        source_node: "hs".into(),
        targets: vec!["hs".into()],
        balance: true,
        metric: "online_clients".into(),
    });
    let inv = Arc::make_mut(i.get_mut("r1").unwrap());
    inv.tables.insert(
        "/interface/print".into(),
        vec![
            [
                ("name".into(), "wan-a".into()),
                ("running".into(), "true".into()),
                ("disabled".into(), "false".into()),
            ]
            .into_iter()
            .collect(),
        ],
    );
    inv.tables.get_mut("/ip/hotspot/active/print").unwrap()[0].extend([
        (".id".into(), "*A".into()),
        ("uptime".into(), "1w2d03:04:05".into()),
    ]);
    let first = plan(&s, &i, None, &AssignmentState::default(), 101);
    assert_eq!(first.active.values().next().unwrap().uptime, Some(788645));
}

#[test]
fn candidate_parent_survives_count_changes_even_when_committed_parent_differs() {
    let (s, mut i) = fixture();
    let first = plan(&s, &i, None, &AssignmentState::default(), 101);
    let alice = normalizer::subscriber_id("r1", "ppp", "alice").unwrap();
    assert_eq!(first.assignments.assignments[&alice].node, "ppp");
    let inv = Arc::make_mut(i.get_mut("r1").unwrap());
    inv.tables.get_mut("/ppp/active/print").unwrap().push(
        [
            ("name".into(), "charlie".into()),
            ("address".into(), "192.0.2.99".into()),
            (".id".into(), "*99".into()),
            ("uptime".into(), "1s".into()),
        ]
        .into_iter()
        .collect(),
    );
    inv.tables.get_mut("/ppp/secret/print").unwrap().push(
        [
            ("name".into(), "charlie".into()),
            ("profile".into(), "Plan".into()),
        ]
        .into_iter()
        .collect(),
    );
    advance(&mut i, 102, "*1", "1h1m");
    let second = plan(&s, &i, Some(&first), &committed_on("ppp2"), 103);
    assert_eq!(second.assignments.assignments[&alice].node, "ppp");
    assert_eq!(
        second
            .assignments
            .assignments
            .values()
            .filter(|p| p.node == "ppp2")
            .count(),
        1
    );
}

#[test]
fn invalid_stored_evidence_and_oversized_state_fail_closed() {
    let (s, i) = fixture();
    let mut state = plan(&s, &i, None, &AssignmentState::default(), 101);
    state.active.values_mut().next().unwrap().id = None;
    let dir = tempfile::tempdir().unwrap();
    assert!(sessions::save(dir.path(), &state).is_err());
    std::fs::create_dir(dir.path().join("state")).unwrap();
    let path = dir.path().join("state/online-assignments.toml");
    std::fs::write(&path, toml::to_string(&state).unwrap()).unwrap();
    assert!(sessions::load(dir.path()).is_err());
    std::fs::File::create(&path)
        .unwrap()
        .set_len(64 * 1024 * 1024 + 1)
        .unwrap();
    assert!(sessions::load(dir.path()).is_err());
}

#[test]
fn uptime_rejects_duplicate_units_and_counts_subseconds_correctly() {
    for (raw, expected) in [
        ("1500ms", Some(1)),
        ("1s500ms", Some(1)),
        ("00:01:02", Some(62)),
        ("1h1h", None),
        ("1m2h", None),
        ("1s00:00:01", None),
        ("18446744073709551615w", None),
    ] {
        let (s, mut i) = fixture();
        let mut subscribers = collect(&s, &i, 101);
        Arc::make_mut(i.get_mut("r1").unwrap())
            .tables
            .get_mut("/ppp/active/print")
            .unwrap()[0]
            .insert("uptime".into(), raw.into());
        let result = sessions::plan(
            &s,
            &mut subscribers,
            &AssignmentState::default(),
            None,
            &i,
            &BTreeMap::new(),
            101,
        );
        if let Some(expected) = expected {
            assert_eq!(
                result.unwrap().active.values().next().unwrap().uptime,
                Some(expected),
                "{raw}"
            );
        } else {
            assert!(result.is_err(), "{raw}");
        }
    }
}

#[test]
fn same_inventory_can_move_selected_subscriber_to_a_new_source_group() {
    let (mut settings, inventories) = fixture();
    let first = plan(&settings, &inventories, None, &committed_on("ppp2"), 101);
    let mut group = settings.nodes[1].clone();
    group.id = "custom".into();
    group.kind = "group".into();
    group.router.clear();
    group.source.clear();
    settings.nodes.push(group);
    settings.selections.push(lqosync_v3::selection::Selection {
        id: "alice-override".into(),
        router: "r1".into(),
        kind: "pppoe".into(),
        selector: "subscriber".into(),
        value: "alice".into(),
        node: "custom".into(),
        exclude: false,
    });
    settings.assignment.pools.push(Pool {
        source_node: "custom".into(),
        targets: vec!["custom".into()],
        balance: true,
        metric: "online_clients".into(),
    });
    let second = plan(
        &settings,
        &inventories,
        Some(&first),
        &first.assignments,
        101,
    );
    assert_eq!(
        second
            .assignments
            .assignments
            .values()
            .next()
            .unwrap()
            .source_node,
        "custom"
    );
    assert_eq!(
        second.assignments.assignments.values().next().unwrap().node,
        "custom"
    );
    assert_eq!(
        plan(
            &settings,
            &inventories,
            Some(&second),
            &first.assignments,
            101
        ),
        second
    );
}

#[test]
fn same_inventory_can_include_a_previously_excluded_subscriber() {
    let (mut settings, mut inventories) = fixture();
    let inv = Arc::make_mut(inventories.get_mut("r1").unwrap());
    inv.tables.get_mut("/ppp/active/print").unwrap().push(
        [
            ("name".into(), "bob".into()),
            ("address".into(), "192.0.2.99".into()),
            (".id".into(), "*2".into()),
        ]
        .into_iter()
        .collect(),
    );
    inv.tables.get_mut("/ppp/secret/print").unwrap().push(
        [
            ("name".into(), "bob".into()),
            ("profile".into(), "Plan".into()),
        ]
        .into_iter()
        .collect(),
    );
    settings.selections.push(lqosync_v3::selection::Selection {
        id: "bob-override".into(),
        router: "r1".into(),
        kind: "pppoe".into(),
        selector: "subscriber".into(),
        value: "bob".into(),
        node: "ppp".into(),
        exclude: true,
    });
    let first = plan(&settings, &inventories, None, &committed_on("ppp2"), 101);
    assert_eq!(first.active.len(), 1);
    settings.selections[0].exclude = false;
    let second = plan(
        &settings,
        &inventories,
        Some(&first),
        &first.assignments,
        101,
    );
    assert_eq!(second.active.len(), 2);
    let alice = normalizer::subscriber_id("r1", "ppp", "alice").unwrap();
    assert_eq!(
        second.assignments.assignments[&alice],
        first.assignments.assignments[&alice]
    );
    settings.selections[0].exclude = true;
    assert_eq!(
        plan(
            &settings,
            &inventories,
            Some(&second),
            &first.assignments,
            101
        )
        .active
        .len(),
        1
    );
}

#[test]
fn capacity_name_and_pin_edits_keep_same_session_continuity() {
    let (mut settings, inventories) = fixture();
    let first = plan(&settings, &inventories, None, &committed_on("ppp2"), 101);
    settings.nodes[0].download = 10000.;
    settings.nodes[0].upload = 10000.;
    settings.nodes[1].name = "New display name".into();
    let second = plan(
        &settings,
        &inventories,
        Some(&first),
        &AssignmentState::default(),
        101,
    );
    assert_eq!(second.assignments, first.assignments);
    settings.assignment.pins = first.assignments.assignments.clone();
    let third = plan(
        &settings,
        &inventories,
        Some(&second),
        &AssignmentState::default(),
        101,
    );
    assert_eq!(third.assignments, first.assignments);
}
