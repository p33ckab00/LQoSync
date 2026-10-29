use lqosync_v3::normalizer::normalize;
use std::{collections::BTreeMap, sync::Arc};
mod common;
use common::fixture;
#[test]
fn selected_sources_produce_correct_nodes_and_stable_ids() {
    let (mut settings, mut inventories) = fixture();
    let first = normalize(&settings, &inventories, &BTreeMap::new(), 101).unwrap();
    assert_eq!(first.len(), 3);
    let alice = first.iter().find(|s| s.name == "alice").unwrap();
    assert_eq!((alice.download, alice.upload), (50., 10.));
    assert_eq!(alice.node, "ppp");
    let inv = Arc::make_mut(inventories.get_mut("r1").unwrap());
    inv.tables.get_mut("/ppp/active/print").unwrap()[0]
        .insert("address".into(), "192.0.2.100".into());
    inv.tables.get_mut("/ppp/secret/print").unwrap()[0].insert("profile".into(), "New Plan".into());
    inv.tables.get_mut("/ppp/profile/print").unwrap()[0].insert("name".into(), "New Plan".into());
    settings.nodes[1].source = "New Plan".into();
    let second = normalize(&settings, &inventories, &BTreeMap::new(), 101).unwrap();
    assert_eq!(
        alice.id,
        second.iter().find(|s| s.name == "alice").unwrap().id
    );
}
#[test]
fn partial_failed_stale_and_missing_selected_metadata_block_publication() {
    let (s, mut invs) = fixture();
    assert!(normalize(&s, &invs, &BTreeMap::new(), 401).is_err());
    assert!(
        normalize(
            &s,
            &invs,
            &[("r1".into(), "Latest read failed".into())]
                .into_iter()
                .collect(),
            101
        )
        .is_err()
    );
    Arc::make_mut(invs.get_mut("r1").unwrap())
        .errors
        .insert("/ppp/active/print".into(), "Denied".into());
    assert!(normalize(&s, &invs, &BTreeMap::new(), 101).is_err());
    let (s, mut invs) = fixture();
    Arc::make_mut(invs.get_mut("r1").unwrap())
        .tables
        .get_mut("/ip/dhcp-server/print")
        .unwrap()
        .clear();
    assert!(normalize(&s, &invs, &BTreeMap::new(), 101).is_err());
}
#[test]
fn hotspot_supersedes_dhcp_only_on_same_router_ip_and_mac() {
    let (s, mut invs) = fixture();
    let inv = Arc::make_mut(invs.get_mut("r1").unwrap());
    let hs = &mut inv.tables.get_mut("/ip/hotspot/active/print").unwrap()[0];
    hs.insert("address".into(), "192.0.2.2".into());
    hs.insert("mac-address".into(), "02:00:00:00:00:02".into());
    let records = normalize(&s, &invs, &BTreeMap::new(), 101).unwrap();
    assert_eq!(records.len(), 2);
    assert!(records.iter().all(|r| r.node != "dhcp"));
    Arc::make_mut(invs.get_mut("r1").unwrap())
        .tables
        .get_mut("/ip/hotspot/active/print")
        .unwrap()[0]
        .insert("mac-address".into(), "02:00:00:00:00:09".into());
    assert!(normalize(&s, &invs, &BTreeMap::new(), 101).is_err());
}
#[test]
fn duplicate_binding_and_invalid_addresses_or_unresolved_rates_are_rejected() {
    let (mut s, invs) = fixture();
    let mut duplicate = s.nodes[1].clone();
    duplicate.id = "duplicate".into();
    s.nodes.push(duplicate);
    assert!(normalize(&s, &invs, &BTreeMap::new(), 101).is_err());
    for (field, value) in [
        ("address", "192.0.2.0/24"),
        ("address", "0.0.0.0"),
        ("name", ""),
    ] {
        let (s, mut invs) = fixture();
        Arc::make_mut(invs.get_mut("r1").unwrap())
            .tables
            .get_mut("/ppp/active/print")
            .unwrap()[0]
            .insert(field.into(), value.into());
        assert!(normalize(&s, &invs, &BTreeMap::new(), 101).is_err());
    }
}
#[test]
fn unselected_service_failure_does_not_override_selected_service_health() {
    let (mut s, mut invs) = fixture();
    s.nodes.retain(|n| n.kind == "wan" || n.kind == "pppoe");
    let inv = Arc::make_mut(invs.get_mut("r1").unwrap());
    inv.tables.remove("/ip/hotspot/active/print");
    inv.errors
        .insert("/ip/hotspot/active/print".into(), "Disabled source".into());
    assert_eq!(
        normalize(&s, &invs, &BTreeMap::new(), 101).unwrap().len(),
        1
    );
}

#[test]
#[ignore = "manual normalization/compiler performance measurement"]
fn normalize_and_compile_50k_subscribers() {
    let (mut settings, mut inventories) = fixture();
    settings.nodes.retain(|n| n.kind == "wan");
    let base = common::fixture().0.nodes[1].clone();
    for i in 0..100 {
        let mut node = base.clone();
        node.id = format!("ppp{i}");
        node.source = format!("Plan{i}");
        node.mode = "computed".into();
        node.download = 0.;
        node.upload = 0.;
        settings.nodes.push(node);
    }
    let inv = Arc::make_mut(inventories.get_mut("r1").unwrap());
    inv.tables.insert(
        "/ppp/profile/print".into(),
        (0..100)
            .map(|i| {
                [
                    ("name".into(), format!("Plan{i}")),
                    ("rate-limit".into(), "10M/50M".into()),
                ]
                .into_iter()
                .collect()
            })
            .collect(),
    );
    inv.tables.insert(
        "/ppp/secret/print".into(),
        (0..50_000)
            .map(|i| {
                [
                    ("name".into(), format!("user{i}")),
                    ("profile".into(), format!("Plan{}", i % 100)),
                ]
                .into_iter()
                .collect()
            })
            .collect(),
    );
    inv.tables.insert(
        "/ppp/active/print".into(),
        (0..50_000)
            .map(|i| {
                [
                    ("name".into(), format!("user{i}")),
                    ("address".into(), format!("10.1.{}.{}", i / 256, i % 256)),
                ]
                .into_iter()
                .collect()
            })
            .collect(),
    );
    let start = std::time::Instant::now();
    let subscribers = normalize(&settings, &inventories, &BTreeMap::new(), 101).unwrap();
    let normalized = start.elapsed();
    let plan = lqosync_v3::model::compile(&settings, &subscribers).unwrap();
    let total = start.elapsed();
    assert_eq!(subscribers.len(), 50_000);
    assert_eq!(plan.capacities["wan"].subscribers, 50_000);
    assert_eq!(plan.capacities.len(), 101);
    eprintln!(
        "50k subscribers /100 selected profiles: normalize={normalized:?}, normalize+compile={total:?} (debug={})",
        cfg!(debug_assertions)
    );
}

#[test]
fn wildcard_dhcp_uses_active_server_and_device_mac_and_never_silently_drops_unknown_mapping() {
    let (settings, mut inventories) = fixture();
    let lease = &mut Arc::make_mut(inventories.get_mut("r1").unwrap())
        .tables
        .get_mut("/ip/dhcp-server/lease/print")
        .unwrap()[0];
    lease.insert("server".into(), "all".into());
    lease.insert("mac-address".into(), "00:00:00:00:00:00".into());
    lease.insert("active-server".into(), "dhcp1".into());
    lease.insert("active-mac-address".into(), "02:00:00:00:00:02".into());
    let subscribers = normalize(&settings, &inventories, &BTreeMap::new(), 101).unwrap();
    assert_eq!(subscribers.len(), 3);
    assert!(subscribers.iter().any(|s| s.node == "dhcp"));
    Arc::make_mut(inventories.get_mut("r1").unwrap())
        .tables
        .get_mut("/ip/dhcp-server/lease/print")
        .unwrap()[0]
        .remove("active-server");
    assert!(normalize(&settings, &inventories, &BTreeMap::new(), 101).is_err());
}

#[test]
fn exact_source_and_account_names_with_spaces_are_never_silently_omitted() {
    let (mut settings, mut inventories) = fixture();
    settings.nodes[1].source = " Plan ".into();
    let inv = Arc::make_mut(inventories.get_mut("r1").unwrap());
    inv.tables.get_mut("/ppp/profile/print").unwrap()[0].insert("name".into(), " Plan ".into());
    inv.tables.get_mut("/ppp/secret/print").unwrap()[0].insert("profile".into(), " Plan ".into());
    inv.tables.get_mut("/ppp/secret/print").unwrap()[0].insert("name".into(), " alice ".into());
    inv.tables.get_mut("/ppp/active/print").unwrap()[0].insert("name".into(), " alice ".into());
    let subscribers = normalize(&settings, &inventories, &BTreeMap::new(), 101).unwrap();
    assert_eq!(subscribers.len(), 3);
    assert!(
        subscribers
            .iter()
            .any(|s| s.name == " alice " && s.node == "ppp")
    );
}

#[test]
fn nonexistent_nonempty_source_mapping_is_not_treated_as_a_known_unselected_profile() {
    let (settings, mut inventories) = fixture();
    Arc::make_mut(inventories.get_mut("r1").unwrap())
        .tables
        .get_mut("/ppp/secret/print")
        .unwrap()[0]
        .insert("profile".into(), "Ghost".into());
    assert!(normalize(&settings, &inventories, &BTreeMap::new(), 101).is_err());
    Arc::make_mut(inventories.get_mut("r1").unwrap())
        .tables
        .get_mut("/ppp/profile/print")
        .unwrap()
        .push(
            [
                ("name".into(), "Ghost".into()),
                ("rate-limit".into(), "5M/25M".into()),
            ]
            .into_iter()
            .collect(),
        );
    assert_eq!(
        normalize(&settings, &inventories, &BTreeMap::new(), 101)
            .unwrap()
            .len(),
        2
    );
}

#[test]
fn ids_are_distinct_across_routers_and_dhcp_ids_ignore_ip_hostname_and_mac_case() {
    let (mut settings, mut inventories) = fixture();
    let first = normalize(&settings, &inventories, &BTreeMap::new(), 101).unwrap();
    let id = first.iter().find(|s| s.node == "dhcp").unwrap().id.clone();
    let lease = &mut Arc::make_mut(inventories.get_mut("r1").unwrap())
        .tables
        .get_mut("/ip/dhcp-server/lease/print")
        .unwrap()[0];
    lease.insert("address".into(), "192.0.2.123".into());
    lease.insert("host-name".into(), "renamed".into());
    lease.insert("mac-address".into(), "0200.0000.0002".into());
    assert_eq!(
        normalize(&settings, &inventories, &BTreeMap::new(), 101)
            .unwrap()
            .iter()
            .find(|s| s.node == "dhcp")
            .unwrap()
            .id,
        id
    );
    settings
        .nodes
        .retain(|n| n.kind == "wan" || n.kind == "pppoe");
    let mut node = settings.nodes[1].clone();
    node.id = "ppp2".into();
    node.router = "r2".into();
    settings.nodes.push(node);
    let mut router = settings.routers[0].clone();
    router.id = "r2".into();
    settings.routers.push(router);
    let mut second = (*inventories["r1"]).clone();
    second.router_id = "r2".into();
    second.tables.get_mut("/ppp/active/print").unwrap()[0]
        .insert("address".into(), "192.0.2.250".into());
    inventories.insert("r2".into(), Arc::new(second));
    let subscribers = normalize(&settings, &inventories, &BTreeMap::new(), 101).unwrap();
    assert_eq!(subscribers.len(), 2);
    assert_ne!(subscribers[0].id, subscribers[1].id);
}
