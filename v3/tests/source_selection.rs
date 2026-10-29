use lqosync_v3::{model::Settings, normalizer};
use serde_json::json;
use std::{collections::BTreeMap, sync::Arc};
mod common;
fn select(settings: &mut Settings, selections: serde_json::Value) {
    let mut value = serde_json::to_value(&*settings).unwrap();
    value["selections"] = selections;
    *settings = serde_json::from_value(value).unwrap();
}
fn binding(
    id: &str,
    kind: &str,
    selector: &str,
    value: &str,
    node: &str,
    exclude: bool,
) -> serde_json::Value {
    json!({"id":id,"router":"r1","kind":kind,"selector":selector,"value":value,"node":node,"exclude":exclude})
}
#[test]
fn individual_selection_overrides_legacy_group_and_accepts_wan_destination() {
    let (mut s, inventories) = common::fixture();
    select(
        &mut s,
        json!([binding(
            "alice",
            "pppoe",
            "subscriber",
            "alice",
            "wan",
            false
        )]),
    );
    let records = normalizer::normalize(&s, &inventories, &BTreeMap::new(), 101).unwrap();
    assert_eq!(
        records.iter().find(|r| r.name == "alice").unwrap().node,
        "wan"
    );
}
#[test]
fn exclusion_skips_a_legacy_group_member() {
    let (mut s, inventories) = common::fixture();
    select(
        &mut s,
        json!([binding("alice", "pppoe", "subscriber", "alice", "", true)]),
    );
    let records = normalizer::normalize(&s, &inventories, &BTreeMap::new(), 101).unwrap();
    assert_eq!(records.len(), 2);
    assert!(records.iter().all(|r| r.name != "alice"));
}
#[test]
fn duplicate_legacy_group_selection_and_invalid_references_fail_closed() {
    for selection in [
        binding("dup", "pppoe", "group", "Plan", "wan", false),
        binding("missing", "pppoe", "subscriber", "alice", "missing", false),
        binding("invalid", "site", "subscriber", "alice", "wan", false),
    ] {
        let (mut s, inventories) = common::fixture();
        select(&mut s, json!([selection]));
        assert!(normalizer::normalize(&s, &inventories, &BTreeMap::new(), 101).is_err());
    }
}
#[test]
fn individual_dhcp_selection_matches_canonical_mac() {
    let (mut s, inventories) = common::fixture();
    select(
        &mut s,
        json!([binding(
            "lease",
            "dhcp",
            "subscriber",
            "0200.0000.0002",
            "wan",
            false
        )]),
    );
    let records = normalizer::normalize(&s, &inventories, &BTreeMap::new(), 101).unwrap();
    assert_eq!(
        records
            .iter()
            .find(|r| r.name == "DHCP client")
            .unwrap()
            .node,
        "wan"
    );
}
#[test]
fn selected_individual_requires_existing_enabled_metadata_even_when_offline() {
    let (mut s, mut inventories) = common::fixture();
    select(
        &mut s,
        json!([binding(
            "missing",
            "pppoe",
            "subscriber",
            "ghost",
            "wan",
            false
        )]),
    );
    Arc::make_mut(inventories.get_mut("r1").unwrap())
        .tables
        .get_mut("/ppp/active/print")
        .unwrap()
        .clear();
    assert!(normalizer::normalize(&s, &inventories, &BTreeMap::new(), 101).is_err());
}
#[test]
fn empty_collection_is_healthy_only_with_complete_fresh_tables() {
    let (s, mut inventories) = common::fixture();
    let inv = Arc::make_mut(inventories.get_mut("r1").unwrap());
    for path in [
        "/ppp/active/print",
        "/ip/dhcp-server/lease/print",
        "/ip/hotspot/active/print",
    ] {
        inv.tables.get_mut(path).unwrap().clear();
    }
    assert!(
        normalizer::collect(&s, &inventories, &BTreeMap::new(), 101)
            .unwrap()
            .is_empty()
    );
    assert!(normalizer::normalize(&s, &inventories, &BTreeMap::new(), 101).is_err());
    Arc::make_mut(inventories.get_mut("r1").unwrap())
        .tables
        .remove("/ppp/active/print");
    assert!(normalizer::collect(&s, &inventories, &BTreeMap::new(), 101).is_err());
}
#[test]
fn independent_groups_target_site_nodes_and_excluded_only_is_inactive() {
    let (mut s, inventories) = common::fixture();
    for node in &mut s.nodes {
        node.router.clear();
        node.source.clear();
    }
    s.nodes[1].kind = "site".into();
    select(
        &mut s,
        json!([binding("plan", "pppoe", "group", "Plan", "ppp", false)]),
    );
    let records = normalizer::collect(&s, &inventories, &BTreeMap::new(), 101).unwrap();
    assert_eq!(records.len(), 1);
    assert_eq!(records[0].node, "ppp");
    assert!(normalizer::is_source_node(&s, "ppp"));
    assert_eq!(
        normalizer::selected_routers(&s)
            .into_iter()
            .collect::<Vec<_>>(),
        vec!["r1"]
    );
    select(
        &mut s,
        json!([binding("alice", "pppoe", "subscriber", "alice", "", true)]),
    );
    assert!(!normalizer::has_bindings(&s));
    assert!(normalizer::selected_routers(&s).is_empty());
}
#[test]
fn offline_individual_requires_valid_group_metadata() {
    let (mut s, mut inventories) = common::fixture();
    for n in &mut s.nodes {
        n.router.clear();
        n.source.clear();
    }
    select(
        &mut s,
        json!([binding(
            "alice",
            "pppoe",
            "subscriber",
            "alice",
            "wan",
            false
        )]),
    );
    let inv = Arc::make_mut(inventories.get_mut("r1").unwrap());
    inv.tables.get_mut("/ppp/active/print").unwrap().clear();
    inv.tables.get_mut("/ppp/profile/print").unwrap()[0].insert("disabled".into(), "true".into());
    assert!(normalizer::collect(&s, &inventories, &BTreeMap::new(), 101).is_err());
}
#[test]
fn exclusion_in_an_unselected_service_does_not_require_its_inventory() {
    let (mut s, mut inventories) = common::fixture();
    s.nodes.retain(|n| n.kind == "wan" || n.kind == "pppoe");
    select(
        &mut s,
        json!([binding("bob", "hotspot", "subscriber", "bob", "", true)]),
    );
    let inv = Arc::make_mut(inventories.get_mut("r1").unwrap());
    inv.tables.remove("/ip/hotspot/active/print");
    assert_eq!(
        normalizer::collect(&s, &inventories, &BTreeMap::new(), 101)
            .unwrap()
            .len(),
        1
    );
}

fn encode(raw: &str) -> String {
    raw.bytes()
        .map(|b| {
            if b.is_ascii_alphanumeric() || b"-_.~".contains(&b) {
                (b as char).to_string()
            } else {
                format!("%{b:02X}")
            }
        })
        .collect()
}
fn post(path: &str, fields: &[(&str, &str)]) -> axum::http::Request<axum::body::Body> {
    axum::http::Request::builder()
        .method("POST")
        .uri(path)
        .header("host", "localhost:9303")
        .header("content-type", "application/x-www-form-urlencoded")
        .body(axum::body::Body::from(
            fields
                .iter()
                .map(|(k, v)| format!("{}={}", encode(k), encode(v)))
                .collect::<Vec<_>>()
                .join("&"),
        ))
        .unwrap()
}
#[tokio::test]
async fn independent_source_forms_check_freshness_and_token_then_create_and_remove_branch_binding()
{
    use axum::http::StatusCode;
    use lqosync_v3::{
        orchestrator, store,
        web::{App, routes},
    };
    use tokio::sync::Mutex;
    use tower::ServiceExt;
    let dir = tempfile::tempdir().unwrap();
    let (mut settings, mut inventories) = common::fixture();
    for node in &mut settings.nodes {
        node.router.clear();
        node.source.clear();
    }
    store::save_settings(dir.path(), &settings).unwrap();
    let mut state = App::open(dir.path().into()).unwrap();
    state.inventories = inventories.clone();
    let token = state.token.clone();
    let state = Arc::new(Mutex::new(state));
    let app = routes(state.clone());
    let source = r#"["r1","pppoe","group","Plan"]"#;
    let before = std::fs::read(dir.path().join("settings.toml")).unwrap();
    for (auth, status) in [
        ("invalid", StatusCode::FORBIDDEN),
        (token.as_str(), StatusCode::UNPROCESSABLE_ENTITY),
    ] {
        assert_eq!(
            app.clone()
                .oneshot(post(
                    "/sources/selection",
                    &[("token", auth), ("selection", source), ("parent", "wan")]
                ))
                .await
                .unwrap()
                .status(),
            status
        );
        assert_eq!(
            std::fs::read(dir.path().join("settings.toml")).unwrap(),
            before
        );
    }
    for inv in inventories.values_mut() {
        Arc::make_mut(inv).collected_at = orchestrator::now();
    }
    state.lock().await.inventories = inventories;
    let response = app
        .clone()
        .oneshot(post(
            "/sources/selection",
            &[("token", &token), ("selection", source), ("parent", "wan")],
        ))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::SEE_OTHER);
    let saved = store::load_settings(dir.path()).unwrap();
    assert_eq!(saved.selections.len(), 1);
    let selection = &saved.selections[0];
    let node = saved.nodes.iter().find(|n| n.id == selection.node).unwrap();
    assert_eq!(
        (&node.name, &node.parent, &node.kind, &node.mode),
        (
            &"Plan".to_owned(),
            &"wan".to_owned(),
            &"pppoe".to_owned(),
            &"computed".to_owned()
        )
    );
    assert_eq!((node.download_factor, node.upload_factor), (1., 1.));
    assert_eq!(store::load_snapshot(dir.path()).unwrap()[0].node, node.id);
    let path = format!("/sources/selection/{}/delete", selection.id);
    assert_eq!(
        app.oneshot(post(&path, &[("token", &token)]))
            .await
            .unwrap()
            .status(),
        StatusCode::SEE_OTHER
    );
    let removed = store::load_settings(dir.path()).unwrap();
    assert!(removed.selections.is_empty());
    assert_eq!(removed.nodes, saved.nodes);
    assert_eq!(state.lock().await.inventories["r1"].tables.len(), 8);
}
#[tokio::test]
async fn inventory_selector_choices_are_searchable_and_bounded() {
    use axum::{body::Body, http::Request};
    use http_body_util::BodyExt;
    use lqosync_v3::{
        orchestrator, store,
        web::{App, routes},
    };
    use tokio::sync::Mutex;
    use tower::ServiceExt;
    let dir = tempfile::tempdir().unwrap();
    let (settings, mut inventories) = common::fixture();
    store::save_settings(dir.path(), &settings).unwrap();
    let inv = Arc::make_mut(inventories.get_mut("r1").unwrap());
    inv.collected_at = orchestrator::now();
    inv.tables.insert(
        "/ppp/secret/print".into(),
        (0..500)
            .map(|i| {
                [
                    ("name".into(), format!("account{i:03}")),
                    ("profile".into(), "Plan".into()),
                ]
                .into_iter()
                .collect()
            })
            .collect(),
    );
    let mut state = App::open(dir.path().into()).unwrap();
    state.inventories = inventories;
    let app = routes(Arc::new(Mutex::new(state)));
    for (path, expected) in [
        ("/sources", "account000"),
        ("/sources?q=account499", "account499"),
    ] {
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri(path)
                    .header("host", "localhost:9303")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        let body = String::from_utf8(
            response
                .into_body()
                .collect()
                .await
                .unwrap()
                .to_bytes()
                .to_vec(),
        )
        .unwrap();
        assert!(body.contains(expected));
        assert!(body.matches(" · subscriber · ").count() <= 200);
        if path == "/sources" {
            assert!(!body.contains("account499"));
        }
    }
}
#[test]
#[ignore = "manual indexed selector performance measurement"]
fn collect_50k_individual_selections_uses_indexed_matching() {
    let (mut s, mut inventories) = common::fixture();
    s.nodes.retain(|n| n.kind == "wan");
    s.nodes[0].download = 10_000.;
    s.nodes[0].upload = 10_000.;
    let inv = Arc::make_mut(inventories.get_mut("r1").unwrap());
    inv.tables.insert(
        "/ppp/secret/print".into(),
        (0..50_000)
            .map(|i| {
                [
                    ("name".into(), format!("user{i}")),
                    ("profile".into(), "Plan".into()),
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
    select(
        &mut s,
        serde_json::Value::Array(
            (0..50_000)
                .map(|i| {
                    binding(
                        &format!("pick{i}"),
                        "pppoe",
                        "subscriber",
                        &format!("user{i}"),
                        "wan",
                        false,
                    )
                })
                .collect(),
        ),
    );
    let started = std::time::Instant::now();
    let records = normalizer::collect(&s, &inventories, &BTreeMap::new(), 101).unwrap();
    eprintln!(
        "50k explicit individual selectors: collect={:?} (debug={})",
        started.elapsed(),
        cfg!(debug_assertions)
    );
    assert_eq!(records.len(), 50_000);
    assert!(records.iter().all(|r| r.node == "wan"));
}

#[test]
fn removed_excluded_groups_do_not_block_healthy_selected_services() {
    for kind in ["pppoe", "dhcp", "hotspot"] {
        let (mut settings, inventories) = common::fixture();
        select(
            &mut settings,
            json!([binding("removed-group", kind, "group", "removed", "", true)]),
        );
        assert_eq!(
            normalizer::collect(&settings, &inventories, &BTreeMap::new(), 101)
                .unwrap()
                .len(),
            3
        );
    }
}
#[test]
fn removed_excluded_accounts_and_leases_do_not_block_healthy_selected_services() {
    for (kind, value) in [
        ("pppoe", "removed"),
        ("dhcp", "020000000099"),
        ("hotspot", "removed"),
    ] {
        let (mut settings, inventories) = common::fixture();
        select(
            &mut settings,
            json!([binding(
                "removed-account",
                kind,
                "subscriber",
                value,
                "",
                true
            )]),
        );
        assert_eq!(
            normalizer::collect(&settings, &inventories, &BTreeMap::new(), 101)
                .unwrap()
                .len(),
            3
        );
    }
}
#[test]
fn selected_dynamic_dhcp_lease_disappearance_is_healthy_absence_with_complete_tables() {
    let (mut settings, mut inventories) = common::fixture();
    settings.nodes.retain(|n| n.kind == "wan");
    select(
        &mut settings,
        json!([binding(
            "dynamic",
            "dhcp",
            "subscriber",
            "020000000002",
            "wan",
            false
        )]),
    );
    assert_eq!(
        normalizer::collect(&settings, &inventories, &BTreeMap::new(), 101)
            .unwrap()
            .len(),
        1
    );
    Arc::make_mut(inventories.get_mut("r1").unwrap())
        .tables
        .get_mut("/ip/dhcp-server/lease/print")
        .unwrap()
        .clear();
    assert!(
        normalizer::collect(&settings, &inventories, &BTreeMap::new(), 101)
            .unwrap()
            .is_empty()
    );
    assert!(normalizer::normalize(&settings, &inventories, &BTreeMap::new(), 101).is_err());
    Arc::make_mut(inventories.get_mut("r1").unwrap())
        .tables
        .remove("/ip/dhcp-server/lease/print");
    assert!(normalizer::collect(&settings, &inventories, &BTreeMap::new(), 101).is_err());
}
#[test]
fn existing_selected_dhcp_records_still_require_enabled_valid_server_mapping() {
    for (field, value) in [("disabled", "true"), ("server", "unknown")] {
        let (mut settings, mut inventories) = common::fixture();
        settings.nodes.retain(|n| n.kind == "wan");
        select(
            &mut settings,
            json!([binding(
                "dynamic",
                "dhcp",
                "subscriber",
                "020000000002",
                "wan",
                false
            )]),
        );
        Arc::make_mut(inventories.get_mut("r1").unwrap())
            .tables
            .get_mut("/ip/dhcp-server/lease/print")
            .unwrap()[0]
            .insert(field.into(), value.into());
        assert!(normalizer::collect(&settings, &inventories, &BTreeMap::new(), 101).is_err());
    }
}

#[test]
fn active_excluded_account_is_skipped_before_missing_local_mapping_or_ip_validation() {
    let (mut settings, mut inventories) = common::fixture();
    select(
        &mut settings,
        json!([binding(
            "exclude-alice",
            "pppoe",
            "subscriber",
            "alice",
            "",
            true
        )]),
    );
    let inv = Arc::make_mut(inventories.get_mut("r1").unwrap());
    inv.tables.get_mut("/ppp/secret/print").unwrap().clear();
    inv.tables.get_mut("/ppp/active/print").unwrap()[0].insert("address".into(), "invalid".into());
    assert_eq!(
        normalizer::collect(&settings, &inventories, &BTreeMap::new(), 101)
            .unwrap()
            .len(),
        2
    );
    Arc::make_mut(inventories.get_mut("r1").unwrap())
        .tables
        .remove("/ppp/secret/print");
    assert!(normalizer::collect(&settings, &inventories, &BTreeMap::new(), 101).is_err());
}

#[test]
fn active_excluded_group_is_skipped_before_deleted_profile_validation() {
    let (mut settings, mut inventories) = common::fixture();
    settings
        .nodes
        .iter_mut()
        .find(|n| n.id == "ppp")
        .unwrap()
        .source = "Other".into();
    select(
        &mut settings,
        json!([binding("exclude-plan", "pppoe", "group", "Plan", "", true)]),
    );
    let inv = Arc::make_mut(inventories.get_mut("r1").unwrap());
    inv.tables.get_mut("/ppp/profile/print").unwrap()[0].insert("name".into(), "Other".into());
    assert_eq!(
        normalizer::collect(&settings, &inventories, &BTreeMap::new(), 101)
            .unwrap()
            .len(),
        2
    );
    select(&mut settings, json!([]));
    assert!(normalizer::collect(&settings, &inventories, &BTreeMap::new(), 101).is_err());
}

#[test]
fn malformed_existing_dhcp_identity_is_not_treated_as_selected_lease_absence() {
    let (mut settings, mut inventories) = common::fixture();
    settings.nodes.retain(|n| n.kind == "wan");
    select(
        &mut settings,
        json!([binding(
            "lease",
            "dhcp",
            "subscriber",
            "020000000002",
            "wan",
            false
        )]),
    );
    Arc::make_mut(inventories.get_mut("r1").unwrap())
        .tables
        .get_mut("/ip/dhcp-server/lease/print")
        .unwrap()[0]
        .insert("mac-address".into(), "malformed".into());
    assert!(normalizer::collect(&settings, &inventories, &BTreeMap::new(), 101).is_err());
}
#[test]
fn explicitly_excluded_dhcp_group_can_skip_a_malformed_identity() {
    let (mut settings, mut inventories) = common::fixture();
    settings
        .nodes
        .iter_mut()
        .find(|n| n.id == "dhcp")
        .unwrap()
        .source = "Other".into();
    select(
        &mut settings,
        json!([binding(
            "exclude-server",
            "dhcp",
            "group",
            "dhcp1",
            "",
            true
        )]),
    );
    let inv = Arc::make_mut(inventories.get_mut("r1").unwrap());
    inv.tables.get_mut("/ip/dhcp-server/print").unwrap()[0].insert("name".into(), "Other".into());
    inv.tables.get_mut("/ip/dhcp-server/lease/print").unwrap()[0]
        .insert("mac-address".into(), "malformed".into());
    assert_eq!(
        normalizer::collect(&settings, &inventories, &BTreeMap::new(), 101)
            .unwrap()
            .len(),
        2
    );
}
