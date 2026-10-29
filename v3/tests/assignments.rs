use lqosync_v3::{assignment::*, model::*, normalizer};
use std::{collections::BTreeMap, sync::Arc};
mod common;
fn fixture() -> (
    Settings,
    BTreeMap<String, Arc<lqosync_v3::routeros::Inventory>>,
    Vec<Subscriber>,
) {
    let (mut s, mut invs) = common::fixture();
    s.nodes[0].router = "r1".into();
    s.nodes[0].source = "wan-a".into();
    s.nodes[0].download = 100.;
    s.nodes[0].upload = 100.;
    let mut wan = s.nodes[0].clone();
    wan.id = "wan2".into();
    wan.source = "wan-b".into();
    wan.download = 300.;
    wan.upload = 300.;
    s.nodes.push(wan);
    let mut target = s.nodes[1].clone();
    target.id = "ppp2".into();
    target.parent = "wan2".into();
    target.router.clear();
    target.source.clear();
    target.mode = "computed".into();
    s.nodes.push(target);
    Arc::make_mut(invs.get_mut("r1").unwrap()).tables.insert(
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
    let subs = normalizer::normalize(&s, &invs, &BTreeMap::new(), 101).unwrap();
    (s, invs, subs)
}
#[test]
fn pins_move_only_the_parent_and_stable_assignments_survive_reordered_scans() {
    let (mut s, invs, mut subs) = fixture();
    s.assignment.pools.push(Pool {
        source_node: "ppp".into(),
        targets: vec!["ppp".into(), "ppp2".into()],
        balance: false,
    });
    let subscriber = subs.iter().find(|s| s.node == "ppp").unwrap().clone();
    s.assignment.pins.insert(
        subscriber.id.clone(),
        Pin {
            source_node: "ppp".into(),
            node: "ppp2".into(),
        },
    );
    let state = assign(
        &s,
        &mut subs,
        &AssignmentState::default(),
        &invs,
        &BTreeMap::new(),
        101,
    )
    .unwrap();
    let moved = subs.iter().find(|s| s.id == subscriber.id).unwrap();
    assert_eq!(moved.node, "ppp2");
    assert_eq!(
        (moved.download, moved.upload, moved.ipv4.as_str()),
        (
            subscriber.download,
            subscriber.upload,
            subscriber.ipv4.as_str()
        )
    );
    s.assignment.pins.clear();
    let mut again = normalizer::normalize(&s, &invs, &BTreeMap::new(), 101).unwrap();
    again.reverse();
    let second = assign(&s, &mut again, &state, &invs, &BTreeMap::new(), 101).unwrap();
    assert_eq!(state, second);
}
#[test]
fn capacity_balancing_is_deterministic_and_weighted_and_keeps_existing_subscribers() {
    let (mut s, invs, _) = fixture();
    s.nodes[1].mode = "computed".into();
    s.assignment.pools.push(Pool {
        source_node: "ppp".into(),
        targets: vec!["ppp2".into(), "ppp".into()],
        balance: true,
    });
    let mut subscribers: Vec<_> = (0..40)
        .map(|i| Subscriber {
            id: format!("user{i:03}"),
            name: "Test".into(),
            node: "ppp".into(),
            ipv4: format!("192.0.2.{}", i + 1),
            download: 10.,
            upload: 10.,
            download_min: 1.,
            upload_min: 1.,
        })
        .collect();
    let state = assign(
        &s,
        &mut subscribers,
        &AssignmentState::default(),
        &invs,
        &BTreeMap::new(),
        101,
    )
    .unwrap();
    assert_eq!(subscribers.iter().filter(|s| s.node == "ppp2").count(), 30);
    let mut reversed: Vec<_> = subscribers
        .iter()
        .cloned()
        .map(|mut s| {
            s.node = "ppp".into();
            s
        })
        .collect();
    reversed.reverse();
    assert_eq!(
        assign(
            &s,
            &mut reversed,
            &AssignmentState::default(),
            &invs,
            &BTreeMap::new(),
            101
        )
        .unwrap(),
        state
    );
    s.nodes[0].download = 1000.;
    s.nodes[0].upload = 1000.;
    let mut original: Vec<_> = subscribers
        .into_iter()
        .map(|mut s| {
            s.node = "ppp".into();
            s
        })
        .collect();
    assert_eq!(
        assign(&s, &mut original, &state, &invs, &BTreeMap::new(), 101).unwrap(),
        state
    );
}
#[test]
fn invalid_targets_duplicate_mapping_and_failed_wan_health_are_rejected() {
    let (mut s, invs, mut subs) = fixture();
    s.assignment.pools.push(Pool {
        source_node: "ppp".into(),
        targets: vec!["ppp".into(), "dhcp".into()],
        balance: true,
    });
    assert!(lqosync_v3::assignment::validate(&s).is_err());
    s.assignment.pools[0].targets[1] = "ppp2".into();
    s.nodes[4].source = "wan-a".into();
    assert!(lqosync_v3::assignment::validate(&s).is_err());
    s.nodes[4].source = "wan-b".into();
    assert!(
        assign(
            &s,
            &mut subs,
            &AssignmentState::default(),
            &invs,
            &[("r1".into(), "failed".into())].into_iter().collect(),
            101
        )
        .is_err()
    );
    assert!(
        assign(
            &s,
            &mut subs,
            &AssignmentState::default(),
            &invs,
            &BTreeMap::new(),
            401
        )
        .is_err()
    );
}

#[test]
fn offline_assignments_survive_publication_restart_and_capacity_changes() {
    use lqosync_v3::{model::compile, store};
    let (mut settings, inventories, original) = fixture();
    for node in settings.nodes.iter_mut().filter(|n| n.kind != "wan") {
        node.mode = "computed".into();
    }
    settings.assignment.pools.push(Pool {
        source_node: "ppp".into(),
        targets: vec!["ppp".into(), "ppp2".into()],
        balance: true,
    });
    let alice = original.iter().find(|s| s.node == "ppp").unwrap();
    let mut active = original.clone();
    let first = assign(
        &settings,
        &mut active,
        &AssignmentState::default(),
        &inventories,
        &BTreeMap::new(),
        101,
    )
    .unwrap();
    assert_eq!(first.assignments[&alice.id].node, "ppp2");
    let mut offline: Vec<_> = original
        .iter()
        .filter(|s| s.id != alice.id)
        .cloned()
        .collect();
    let absent = assign(
        &settings,
        &mut offline,
        &first,
        &inventories,
        &BTreeMap::new(),
        101,
    )
    .unwrap();
    assert_eq!(
        absent.assignments.get(&alice.id),
        first.assignments.get(&alice.id)
    );
    let dir = tempfile::tempdir().unwrap();
    let plan = compile(&settings, &offline).unwrap();
    store::publish_assigned(dir.path(), &plan, &offline, &absent).unwrap();
    let restored = store::load_assignments(dir.path()).unwrap();
    settings.nodes[0].download = 10000.;
    settings.nodes[0].upload = 10000.;
    let mut returning = original.clone();
    let after = assign(
        &settings,
        &mut returning,
        &restored,
        &inventories,
        &BTreeMap::new(),
        101,
    )
    .unwrap();
    assert_eq!(after.assignments[&alice.id], first.assignments[&alice.id]);
    assert_eq!(
        returning.iter().find(|s| s.id == alice.id).unwrap().node,
        "ppp2"
    );
    settings.assignment.pools.clear();
    let pruned = assign(
        &settings,
        &mut offline,
        &restored,
        &inventories,
        &BTreeMap::new(),
        101,
    )
    .unwrap();
    assert!(!pruned.assignments.contains_key(&alice.id));
}

#[test]
#[ignore = "manual synthetic performance measurement"]
fn assign_50k_subscribers_across_eight_wans() {
    use std::time::Instant;
    let (mut settings, mut inventories, _) = fixture();
    settings.nodes.retain(|n| n.id == "wan" || n.id == "ppp");
    settings.nodes[0].download = 1000.;
    settings.nodes[0].upload = 1000.;
    settings.nodes[1].mode = "computed".into();
    let mut targets = vec!["ppp".into()];
    let mut interfaces =
        Arc::make_mut(inventories.get_mut("r1").unwrap()).tables["/interface/print"].clone();
    for i in 1..8 {
        let mut wan = settings.nodes[0].clone();
        wan.id = format!("wan{i}");
        wan.source = format!("uplink-{i}");
        let mut node = settings.nodes[1].clone();
        node.id = format!("ppp{i}");
        node.parent = wan.id.clone();
        node.router.clear();
        node.source.clear();
        targets.push(node.id.clone());
        settings.nodes.extend([wan, node]);
        interfaces.push(
            [
                ("name".into(), format!("uplink-{i}")),
                ("running".into(), "true".into()),
                ("disabled".into(), "false".into()),
            ]
            .into_iter()
            .collect(),
        );
    }
    Arc::make_mut(inventories.get_mut("r1").unwrap())
        .tables
        .insert("/interface/print".into(), interfaces);
    settings.assignment.pools.push(Pool {
        source_node: "ppp".into(),
        targets,
        balance: true,
    });
    let mut subscribers: Vec<_> = (0..50000)
        .map(|i| Subscriber {
            id: format!("user-{i:05}"),
            name: format!("Test {i}"),
            node: "ppp".into(),
            ipv4: std::net::Ipv4Addr::from(0x0a000001 + i).to_string(),
            download: 50.,
            upload: 10.,
            download_min: 5.,
            upload_min: 1.,
        })
        .collect();
    let start = Instant::now();
    let first = assign(
        &settings,
        &mut subscribers,
        &AssignmentState::default(),
        &inventories,
        &BTreeMap::new(),
        101,
    )
    .unwrap();
    let new_elapsed = start.elapsed();
    let mut counts = BTreeMap::new();
    for s in &subscribers {
        *counts.entry(s.node.clone()).or_insert(0) += 1;
        assert_eq!(
            (s.download, s.upload, s.download_min, s.upload_min),
            (50., 10., 5., 1.)
        );
    }
    assert_eq!(counts.len(), 8);
    assert!(counts.values().all(|n| *n == 6250));
    for s in &mut subscribers {
        s.node = "ppp".into();
    }
    let start = Instant::now();
    let retained = assign(
        &settings,
        &mut subscribers,
        &first,
        &inventories,
        &BTreeMap::new(),
        101,
    )
    .unwrap();
    let retained_elapsed = start.elapsed();
    assert_eq!(first, retained);
    let plan = lqosync_v3::model::compile(&settings, &subscribers).unwrap();
    assert_eq!(plan.csv.lines().count(), 50001);
    eprintln!(
        "50k subscribers / 8 WANs: new={new_elapsed:?}, retained={retained_elapsed:?}; debug, excludes collection and disk publication"
    );
}
