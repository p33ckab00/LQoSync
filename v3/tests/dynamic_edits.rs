use lqosync_v3::{
    dynamic, model,
    topology_edits::{self, Operation},
};
mod common;
fn settings() -> model::Settings {
    let (mut s, _) = common::fixture();
    s.nodes[0].router = "r1".into();
    s.nodes[0].source = "uplink".into();
    dynamic::migrate(&s).unwrap()
}
#[test]
fn promotion_and_demotion_preserve_wan_sources_and_node_ids() {
    let s = settings();
    let up = topology_edits::preview(
        &s,
        Operation::Reparent {
            id: "ppp".into(),
            parent: String::new(),
        },
    )
    .unwrap();
    assert!(
        up.settings
            .nodes
            .iter()
            .find(|n| n.id == "ppp")
            .unwrap()
            .parent
            .is_empty()
    );
    assert_eq!(up.settings.dynamic, s.dynamic);
    assert_eq!(up.settings.selections, s.selections);
    let down = topology_edits::preview(
        &up.settings,
        Operation::Reparent {
            id: "ppp".into(),
            parent: "dhcp".into(),
        },
    )
    .unwrap();
    assert_eq!(
        down.settings
            .nodes
            .iter()
            .find(|n| n.id == "ppp")
            .unwrap()
            .parent,
        "dhcp"
    );
    assert!(
        topology_edits::preview(
            &down.settings,
            Operation::Reparent {
                id: "dhcp".into(),
                parent: "ppp".into()
            }
        )
        .is_err()
    );
}
#[test]
fn referenced_node_removal_requires_an_explicit_replacement_and_preserves_sources() {
    let s = settings();
    assert!(
        topology_edits::preview(
            &s,
            Operation::Remove {
                id: "ppp".into(),
                replacement: None,
                child_parent: None
            }
        )
        .is_err()
    );
    let edit = topology_edits::preview(
        &s,
        Operation::Remove {
            id: "ppp".into(),
            replacement: Some("wan".into()),
            child_parent: None,
        },
    )
    .unwrap();
    assert!(!edit.settings.nodes.iter().any(|n| n.id == "ppp"));
    assert_eq!(edit.settings.selections, s.selections);
    let p = edit
        .settings
        .dynamic
        .as_ref()
        .unwrap()
        .policies
        .iter()
        .find(|p| p.id == "ppp")
        .unwrap();
    assert_eq!(p.default.node, "wan");
    assert!(p.targets.iter().all(|t| t.node != "ppp"));
    assert_eq!(p.default.wan, "wan");
    assert_ne!(
        edit.revision,
        topology_edits::revision(&edit.settings).unwrap()
    );
    assert_eq!(s.nodes.len(), 4);
}

#[tokio::test]
async fn hierarchy_posts_reject_stale_revisions_and_save_valid_promotions() {
    use axum::{body::Body, http::Request};
    use std::sync::Arc;
    use tokio::sync::Mutex;
    use tower::ServiceExt;
    let d = tempfile::tempdir().unwrap();
    let mut a = lqosync_v3::web::App::open(d.path().into()).unwrap();
    a.settings = settings();
    a.settings.auto_stage = false;
    let token = a.token.clone();
    let rev = topology_edits::revision(&a.settings).unwrap();
    let state = Arc::new(Mutex::new(a));
    let app = lqosync_v3::web::routes(state.clone());
    for (revision, status) in [("stale", 422), (rev.as_str(), 303)] {
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/dynamic/node")
                    .header("host", "localhost:9303")
                    .header("content-type", "application/x-www-form-urlencoded")
                    .body(Body::from(format!(
                        "token={token}&revision={revision}&node=ppp&action=promote"
                    )))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status().as_u16(), status);
    }
    assert!(
        state
            .lock()
            .await
            .settings
            .nodes
            .iter()
            .find(|n| n.id == "ppp")
            .unwrap()
            .parent
            .is_empty()
    );
}

#[tokio::test]
async fn editing_a_source_policy_preserves_its_selection_identity() {
    use axum::{body::Body, http::Request};
    use std::sync::Arc;
    use tokio::sync::Mutex;
    use tower::ServiceExt;
    let d = tempfile::tempdir().unwrap();
    let mut a = lqosync_v3::web::App::open(d.path().into()).unwrap();
    a.settings = settings();
    a.settings.auto_stage = false;
    let (_, mut inventory) = common::fixture();
    Arc::make_mut(inventory.get_mut("r1").unwrap()).collected_at = lqosync_v3::orchestrator::now();
    a.inventories = inventory;
    let id = a
        .settings
        .dynamic
        .as_ref()
        .unwrap()
        .policies
        .iter()
        .find(|p| p.id == "ppp")
        .unwrap()
        .selections[0]
        .clone();
    let body = format!(
        "token={}&revision={}&original_id={id}&selection=[\"r1\",\"pppoe\",\"group\",\"Plan\"]&policy=ppp",
        a.token,
        topology_edits::revision(&a.settings).unwrap()
    );
    let state = Arc::new(Mutex::new(a));
    let response = lqosync_v3::web::routes(state.clone())
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/sources/selection")
                .header("host", "localhost:9303")
                .header("content-type", "application/x-www-form-urlencoded")
                .body(Body::from(body))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status().as_u16(), 303);
    assert_eq!(state.lock().await.settings.selections.len(), 3);
    assert!(
        state
            .lock()
            .await
            .settings
            .selections
            .iter()
            .any(|s| s.id == id)
    );
}

async fn post(app: axum::Router, uri: &str, body: String) -> axum::response::Response {
    use axum::{body::Body, http::Request};
    use tower::ServiceExt;
    app.oneshot(
        Request::builder()
            .method("POST")
            .uri(uri)
            .header("host", "localhost:9303")
            .header("content-type", "application/x-www-form-urlencoded")
            .body(Body::from(body))
            .unwrap(),
    )
    .await
    .unwrap()
}
#[tokio::test]
async fn stale_source_deletion_and_pin_clear_do_not_overwrite_new_settings() {
    use std::sync::Arc;
    use tokio::sync::Mutex;
    let d = tempfile::tempdir().unwrap();
    let mut a = lqosync_v3::web::App::open(d.path().into()).unwrap();
    a.settings = settings();
    let id = a.settings.selections[0].id.clone();
    let before = a.settings.clone();
    let token = a.token.clone();
    let state = Arc::new(Mutex::new(a));
    let app = lqosync_v3::web::routes(state.clone());
    for uri in [
        format!("/sources/selection/{id}/delete"),
        "/assignments/pin".into(),
    ] {
        let response = post(
            app.clone(),
            &uri,
            format!("token={token}&revision=stale&id=missing&target="),
        )
        .await;
        let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        assert!(String::from_utf8_lossy(&bytes).contains("Settings changed"));
        assert_eq!(state.lock().await.settings, before);
    }
}
#[tokio::test]
async fn unpublished_legacy_candidate_allows_replacement_but_rejects_unresolvable_source_removal() {
    use lqosync_v3::{assignment, normalizer, sessions};
    use std::{collections::BTreeMap, sync::Arc};
    use tokio::sync::Mutex;
    let d = tempfile::tempdir().unwrap();
    let mut a = lqosync_v3::web::App::open(d.path().into()).unwrap();
    let (mut legacy, mut inventory) = common::fixture();
    let mut rows = normalizer::collect(&legacy, &inventory, &BTreeMap::new(), 101).unwrap();
    let assignments = assignment::assign(
        &legacy,
        &mut rows,
        &Default::default(),
        &inventory,
        &BTreeMap::new(),
        101,
    )
    .unwrap();
    let output = model::compile(&legacy, &rows).unwrap();
    lqosync_v3::store::publish_assigned(d.path(), &output, &rows, &assignments).unwrap();
    a.generation = lqosync_v3::store::current_generation(d.path());
    sessions::save(
        d.path(),
        &sessions::Observation {
            assignments,
            ..Default::default()
        },
    )
    .unwrap();
    legacy.nodes[0].router = "r1".into();
    legacy.nodes[0].source = "uplink".into();
    a.settings = dynamic::migrate(&legacy).unwrap();
    a.settings.auto_stage = false;
    Arc::make_mut(inventory.get_mut("r1").unwrap()).collected_at = lqosync_v3::orchestrator::now();
    a.inventories = inventory;
    let token = a.token.clone();
    let rev = topology_edits::revision(&a.settings).unwrap();
    let state = Arc::new(Mutex::new(a));
    let app = lqosync_v3::web::routes(state.clone());
    let response = post(
        app.clone(),
        "/dynamic/node",
        format!("token={token}&revision={rev}&node=ppp&action=remove&replacement=wan"),
    )
    .await;
    assert_eq!(response.status().as_u16(), 303);
    let a = state.lock().await;
    let current_rows = normalizer::collect(
        &a.settings,
        &a.inventories,
        &BTreeMap::new(),
        lqosync_v3::orchestrator::now(),
    )
    .unwrap();
    let evidence = sessions::load(d.path()).unwrap().unwrap();
    let adopted = dynamic::adopt(&a.settings, &current_rows, &evidence.assignments).unwrap();
    assert!(adopted.routes.values().all(|r| r.node != "ppp"));
    let id = a
        .settings
        .dynamic
        .as_ref()
        .unwrap()
        .policies
        .iter()
        .find(|p| p.id == "ppp")
        .unwrap()
        .selections[0]
        .clone();
    let before = a.settings.clone();
    let rev = topology_edits::revision(&a.settings).unwrap();
    drop(a);
    let response = post(
        app,
        &format!("/sources/selection/{id}/delete"),
        format!("token={token}&revision={rev}"),
    )
    .await;
    assert_eq!(response.status().as_u16(), 422);
    assert_eq!(state.lock().await.settings, before);
}

#[tokio::test]
async fn pin_form_selects_exact_wan_pair_and_removal_shows_both_evidence_counts() {
    use lqosync_v3::{
        assignment::AssignmentState, dynamic::Target, minimums, normalizer, sessions, store,
    };
    use std::{collections::BTreeMap, sync::Arc};
    use tokio::sync::Mutex;
    use tower::ServiceExt;
    let d = tempfile::tempdir().unwrap();
    let mut a = lqosync_v3::web::App::open(d.path().into()).unwrap();
    let (_, inventory) = common::fixture();
    a.settings = settings();
    let c = a.settings.dynamic.as_mut().unwrap();
    let mut second = c.wans[0].clone();
    second.id = "second".into();
    second.interface = "other".into();
    c.wans.push(second);
    let policy = c.policies.iter_mut().find(|p| p.id == "ppp").unwrap();
    let target = Target {
        node: "ppp".into(),
        wan: "second".into(),
    };
    policy.targets.push(target.clone());
    policy.default = target.clone();
    let mut rows = normalizer::collect(&a.settings, &inventory, &BTreeMap::new(), 101).unwrap();
    let state =
        dynamic::assign_targets(&a.settings, &mut rows, &AssignmentState::default()).unwrap();
    minimums::allocate_assigned(&a.settings, &mut rows, &state).unwrap();
    let output = model::compile_assigned(&a.settings, &rows, &state).unwrap();
    store::publish_assigned(d.path(), &output, &rows, &state).unwrap();
    sessions::save(
        d.path(),
        &sessions::Observation {
            assignments: state,
            ..Default::default()
        },
    )
    .unwrap();
    a.generation = store::current_generation(d.path());
    let app = lqosync_v3::web::routes(Arc::new(Mutex::new(a)));
    let response = app
        .oneshot(
            axum::http::Request::builder()
                .uri("/assignments")
                .header("host", "localhost:9303")
                .body(axum::body::Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status().as_u16(), 200);
    let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let html = String::from_utf8(bytes.to_vec()).unwrap();
    let selected = format!(
        "value=\"{}\" selected",
        serde_json::to_string(&target)
            .unwrap()
            .replace('"', "&quot;")
    );
    let pin_form = html
        .split("data-editor-title=\"Pin alice\"")
        .nth(1)
        .unwrap()
        .split("</form>")
        .next()
        .unwrap();
    assert!(
        pin_form.contains(&selected),
        "current node/WAN pair must be selected"
    );
    assert!(html.contains("committed: 1 · stored candidate: 1"));
    assert!(html.contains("data-editor-title=\"Pin alice\""));
}

#[tokio::test]
async fn policy_page_bounds_source_summaries_and_labels_missing_evidence() {
    use std::sync::Arc;
    use tokio::sync::Mutex;
    use tower::ServiceExt;
    let d = tempfile::tempdir().unwrap();
    let mut a = lqosync_v3::web::App::open(d.path().into()).unwrap();
    a.settings = settings();
    let source = a.settings.selections[0].clone();
    a.settings.selections = (0..500)
        .map(|i| {
            let mut s = source.clone();
            s.id = format!("source-{i}");
            s.value = format!("member-{i}");
            s
        })
        .collect();
    let c = a.settings.dynamic.as_mut().unwrap();
    c.policies.truncate(1);
    c.policies[0].selections = a.settings.selections.iter().map(|s| s.id.clone()).collect();
    let response = lqosync_v3::web::routes(Arc::new(Mutex::new(a)))
        .oneshot(
            axum::http::Request::builder()
                .uri("/assignments")
                .header("host", "localhost:9303")
                .body(axum::body::Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status().as_u16(), 200);
    let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let html = String::from_utf8(bytes.to_vec()).unwrap();
    assert!(html.contains("500 selected sources"));
    assert!(!html.contains("member-499"));
    assert!(html.contains("committed: unavailable · stored candidate: unavailable"));
    assert!(html.contains("name=\"policy_q\""));
}

#[tokio::test]
async fn unadopted_legacy_snapshot_does_not_offer_a_guessed_pin_pair() {
    use lqosync_v3::{assignment, normalizer, store};
    use std::{collections::BTreeMap, sync::Arc};
    use tokio::sync::Mutex;
    use tower::ServiceExt;
    let d = tempfile::tempdir().unwrap();
    let mut a = lqosync_v3::web::App::open(d.path().into()).unwrap();
    let (legacy, inventory) = common::fixture();
    let mut rows = normalizer::collect(&legacy, &inventory, &BTreeMap::new(), 101).unwrap();
    let state = assignment::assign(
        &legacy,
        &mut rows,
        &Default::default(),
        &inventory,
        &BTreeMap::new(),
        101,
    )
    .unwrap();
    store::publish_assigned(
        d.path(),
        &model::compile(&legacy, &rows).unwrap(),
        &rows,
        &state,
    )
    .unwrap();
    a.settings = settings();
    a.generation = store::current_generation(d.path());
    let response = lqosync_v3::web::routes(Arc::new(Mutex::new(a)))
        .oneshot(
            axum::http::Request::builder()
                .uri("/assignments")
                .header("host", "localhost:9303")
                .body(axum::body::Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let html = String::from_utf8(bytes.to_vec()).unwrap();
    assert!(html.contains("Routing pair unavailable"));
    assert!(!html.contains("data-editor-title=\"Pin alice\""));
}
