use super::*;
use crate::topology_manager::catalog::{
    CapacityPolicy, CatalogNode, CatalogRouter, CatalogSource, RouterRole, SourceOrigin,
    TopologyCatalog,
};
use crate::topology_manager::layout::NodeKind;
use serde_json::json;

fn iface(name: &str, kind: &str) -> RosRow {
    [
        (".id", "*1"),
        ("name", name),
        ("type", kind),
        ("running", "true"),
        ("disabled", "false"),
    ]
    .iter()
    .map(|(k, v)| (k.to_string(), v.to_string()))
    .collect()
}

fn ok(name: &str, rows: Vec<RosRow>) -> RouterInterfaces {
    RouterInterfaces {
        router_id: format!("id-{name}"),
        name: name.into(),
        read: Ok(rows),
    }
}

fn names(router: &Value) -> Vec<&str> {
    router["candidates"]
        .as_array()
        .unwrap()
        .iter()
        .map(|c| c["name"].as_str().unwrap())
        .collect()
}

/// A catalog with a WAN source on router `router_name` (ID `router_id`) for `endpoint`.
fn catalog(router_id: &str, router_name: &str, endpoint: &str, nodes: &[&str]) -> TopologyCatalog {
    TopologyCatalog {
        routers: vec![CatalogRouter {
            id: router_id.into(),
            name: router_name.into(),
            roles: vec![RouterRole::Wan],
        }],
        sources: vec![CatalogSource {
            excluded: false,
            id: "wan-1".into(),
            name: "WAN1".into(),
            origin: SourceOrigin::Wan {
                router_id: router_id.into(),
                endpoint_ref: endpoint.into(),
            },
            profile_id: None,
            targets: vec![],
        }],
        nodes: nodes
            .iter()
            .map(|id| CatalogNode {
                address_list: None,
                tree_excluded: false,
                router_id: None,
                geo: None,
                minimum: Default::default(),
                id: (*id).into(),
                name: (*id).into(),
                parent: None,
                source_id: Some("wan-1".into()),
                kind: NodeKind::Wan,
                capacity: CapacityPolicy::Inherit,
            })
            .collect(),
        ..TopologyCatalog::default()
    }
}

#[test]
fn pppoe_out_comes_first_and_in_types_are_left_out() {
    let rows = vec![
        iface("ether1", "ether"),
        iface("pppoe-out2", "pppoe-out"),
        iface("pppoe-out1", "pppoe-out"),
        iface("<pppoe-a>", "pppoe-in"),
        iface("l2tp-x", "l2tp-in"),
    ];
    let v = answer(&[ok("Core", rows)], None, 7);
    let r = &v["routers"][0];
    assert_eq!(v["collected_at"], json!(7));
    assert_eq!(r["state"], json!("ok"));
    assert_eq!(r["error"], Value::Null);
    assert_eq!(names(r), ["pppoe-out1", "pppoe-out2", "ether1"]);
    let c = &r["candidates"];
    assert_eq!(
        (&c[0]["suggested"], &c[1]["suggested"], &c[2]["suggested"]),
        (&json!(true), &json!(true), &json!(false))
    );
    assert_eq!(c[0]["type"], json!("pppoe-out"));
    assert_eq!(c[0]["running"], json!(true));
    assert_eq!(c[0]["disabled"], json!(false));
    assert_eq!(c[0]["comment"], Value::Null);
}

#[test]
fn ten_thousand_dynamic_sessions_give_ok_and_no_in_candidates() {
    let mut rows: Vec<_> = (0..10_000)
        .map(|i| iface(&format!("<pppoe-u{i}>"), "pppoe-in"))
        .collect();
    rows.push(iface("vlan10", "vlan"));
    let v = answer(&[ok("Distro", rows)], None, 0);
    assert_eq!(v["routers"][0]["state"], json!("ok"));
    assert_eq!(names(&v["routers"][0]), ["vlan10"]);
}

#[test]
fn a_failed_router_gives_its_error_and_no_missing_list() {
    let failed = RouterInterfaces {
        router_id: "id-Core".into(),
        name: "Core".into(),
        read: Err("login refused".into()),
    };
    let cat = catalog("id-Core", "Core", "pppoe-out1", &["n1"]);
    let v = answer(&[failed], Some(&cat), 0);
    let r = &v["routers"][0];
    assert_eq!(r["state"], json!("failed"));
    assert_eq!(r["error"], json!("login refused"));
    assert_eq!(r["candidates"], json!([]));
    assert_eq!(r["missing"], json!([]));
    assert_eq!(v["unconfigured"], json!([]));
}

#[test]
fn a_catalog_wan_source_gives_source_id_and_sorted_used_by() {
    let cat = catalog("id-Core", "Core", "pppoe-out1", &["n2", "n1"]);
    let v = answer(
        &[ok("Core", vec![iface("pppoe-out1", "pppoe-out")])],
        Some(&cat),
        0,
    );
    let c = &v["routers"][0]["candidates"][0];
    assert_eq!(c["source_id"], json!("wan-1"));
    assert_eq!(c["used_by"], json!(["n1", "n2"]));
    assert_eq!(v["routers"][0]["missing"], json!([]));
}

#[test]
fn a_renamed_interface_shows_missing_and_a_new_candidate() {
    let cat = catalog("id-Core", "Core", "pppoe-out1", &["n1"]);
    let v = answer(
        &[ok("Core", vec![iface("pppoe-isp-a", "pppoe-out")])],
        Some(&cat),
        0,
    );
    let r = &v["routers"][0];
    assert_eq!(
        r["missing"],
        json!([{"source_id": "wan-1", "endpoint_ref": "pppoe-out1"}])
    );
    assert_eq!(r["candidates"][0]["source_id"], Value::Null);
    assert_eq!(r["candidates"][0]["used_by"], json!([]));
}

#[test]
fn catalog_routers_match_by_name() {
    let cat = catalog("custom-core", "Core", "pppoe-out1", &["n1"]);
    let v = answer(
        &[ok("Core", vec![iface("pppoe-out1", "pppoe-out")])],
        Some(&cat),
        0,
    );
    assert_eq!(
        v["routers"][0]["candidates"][0]["source_id"],
        json!("wan-1")
    );
    assert_eq!(v["unconfigured"], json!([]));
}

#[test]
fn a_wan_source_on_an_unconfigured_router_is_listed() {
    let cat = catalog("id-Old", "OldCore", "pppoe-out1", &["n1"]);
    let v = answer(&[ok("Core", vec![])], Some(&cat), 0);
    assert_eq!(
        v["unconfigured"],
        json!([{"source_id": "wan-1", "router_id": "id-Old", "endpoint_ref": "pppoe-out1"}])
    );
    assert_eq!(v["routers"][0]["missing"], json!([]));
}

#[test]
fn every_router_remains_visible_when_one_has_no_wan_candidates() {
    let v = answer(
        &[
            ok("Distro", vec![]),
            ok("Core", vec![iface("pppoe-out1", "pppoe-out")]),
        ],
        None,
        1_000,
    );
    let routers = v["routers"].as_array().unwrap();
    assert_eq!(routers.len(), 2);
    assert_eq!(routers[0]["name"], "Distro");
    assert_eq!(routers[0]["state"], "ok");
    assert_eq!(routers[0]["candidates"], json!([]));
    assert_eq!(routers[1]["name"], "Core");
    assert_eq!(names(&routers[1]), ["pppoe-out1"]);
}

#[test]
fn without_a_catalog_lists_are_empty() {
    let v = answer(
        &[ok("Core", vec![iface("pppoe-out1", "pppoe-out")])],
        None,
        0,
    );
    let c = &v["routers"][0]["candidates"][0];
    assert_eq!((&c["source_id"], &c["used_by"]), (&Value::Null, &json!([])));
    assert_eq!(v["routers"][0]["missing"], json!([]));
    assert_eq!(v["unconfigured"], json!([]));
}
