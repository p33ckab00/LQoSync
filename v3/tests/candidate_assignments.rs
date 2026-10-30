use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use http_body_util::BodyExt;
use lqosync_v3::{
    assignment::{AssignmentState, Pin, Pool},
    normalizer, orchestrator, sessions, store,
    web::{App, Shared, routes},
};
use std::{collections::BTreeMap, sync::Arc};
use tokio::sync::Mutex;
use tower::ServiceExt;
mod common;
fn request(path: &str) -> Request<Body> {
    Request::builder()
        .uri(path)
        .header("host", "localhost:9303")
        .body(Body::empty())
        .unwrap()
}
fn pin(token: &str, id: &str, target: &str) -> Request<Body> {
    Request::builder()
        .method("POST")
        .uri("/assignments/pin")
        .header("host", "localhost:9303")
        .header("content-type", "application/x-www-form-urlencoded")
        .body(Body::from(format!("token={token}&id={id}&target={target}")))
        .unwrap()
}
async fn body(response: axum::response::Response) -> String {
    String::from_utf8(
        response
            .into_body()
            .collect()
            .await
            .unwrap()
            .to_bytes()
            .to_vec(),
    )
    .unwrap()
}
fn fixture(root: &std::path::Path) -> (Shared, String, String) {
    let (mut settings, mut inventories) = common::fixture();
    settings.auto_stage = false;
    settings.nodes[0].router = "r1".into();
    settings.nodes[0].source = "wan-a".into();
    let mut target = settings.nodes[1].clone();
    target.id = "ppp2".into();
    target.router.clear();
    target.source.clear();
    target.mode = "computed".into();
    settings.nodes.push(target);
    settings.assignment.pools.push(Pool {
        source_node: "ppp".into(),
        targets: vec!["ppp".into(), "ppp2".into()],
        balance: true,
        metric: "online_clients".into(),
    });
    let inv = Arc::make_mut(inventories.get_mut("r1").unwrap());
    inv.collected_at = orchestrator::now() - 2;
    inv.tables.get_mut("/ppp/active/print").unwrap()[0].insert(".id".into(), "*1".into());
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
    let id = normalizer::subscriber_id("r1", "ppp", "alice").unwrap();
    let baseline = AssignmentState {
        routes: Default::default(),
        assignments: [(
            id.clone(),
            Pin {
                source_node: "ppp".into(),
                node: "ppp2".into(),
            },
        )]
        .into_iter()
        .collect(),
    };
    let mut subscribers = normalizer::collect(
        &settings,
        &inventories,
        &BTreeMap::new(),
        orchestrator::now(),
    )
    .unwrap();
    let observation = sessions::plan(
        &settings,
        &mut subscribers,
        &baseline,
        None,
        &inventories,
        &BTreeMap::new(),
        orchestrator::now(),
    )
    .unwrap();
    store::save_settings(root, &settings).unwrap();
    sessions::save(root, &observation).unwrap();
    let mut app = App::open(root.into()).unwrap();
    app.inventories = inventories;
    let token = app.token.clone();
    (Arc::new(Mutex::new(app)), token, id)
}
#[tokio::test]
async fn candidate_only_clients_are_visible_and_pinnable_without_a_committed_generation() {
    let dir = tempfile::tempdir().unwrap();
    let (state, token, id) = fixture(dir.path());
    let app = routes(state.clone());
    let candidate_path = dir.path().join("state/online-assignments.toml");
    let before = std::fs::read(&candidate_path).unwrap();
    let response = app
        .clone()
        .oneshot(request("/assignments?q=alice"))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let html = body(response).await;
    assert!(html.contains("<p>Candidate assignments ·"));
    assert!(html.contains("alice"));
    assert!(html.contains("ppp2"));
    assert!(html.contains(&id));
    assert_eq!(std::fs::read(&candidate_path).unwrap(), before);
    assert!(!dir.path().join("state/current.toml").exists());
    assert_eq!(
        app.clone()
            .oneshot(pin("bad", &id, "ppp"))
            .await
            .unwrap()
            .status(),
        StatusCode::FORBIDDEN
    );
    assert!(state.lock().await.settings.assignment.pins.is_empty());
    assert_eq!(
        app.clone()
            .oneshot(pin(&token, &id, "ppp"))
            .await
            .unwrap()
            .status(),
        StatusCode::SEE_OTHER
    );
    let stored = store::load_settings(dir.path()).unwrap();
    assert_eq!(
        stored.assignment.pins[&id],
        Pin {
            source_node: "ppp".into(),
            node: "ppp".into()
        }
    );
    assert!(!dir.path().join("state/current.toml").exists());
}
#[tokio::test]
async fn stale_candidate_only_client_cannot_be_pinned_and_stored_pin_can_be_cleared() {
    let dir = tempfile::tempdir().unwrap();
    let (state, token, id) = fixture(dir.path());
    let app = routes(state.clone());
    assert_eq!(
        app.clone()
            .oneshot(pin(&token, &id, "ppp"))
            .await
            .unwrap()
            .status(),
        StatusCode::SEE_OTHER
    );
    Arc::make_mut(state.lock().await.inventories.get_mut("r1").unwrap()).collected_at =
        orchestrator::now() - 301;
    let response = app.clone().oneshot(request("/assignments")).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let html = body(response).await;
    assert!(html.contains("Committed snapshot"));
    assert!(!html.contains("<p>Candidate assignments ·"));
    assert_eq!(
        app.clone()
            .oneshot(pin(&token, &id, "ppp2"))
            .await
            .unwrap()
            .status(),
        StatusCode::UNPROCESSABLE_ENTITY
    );
    assert_eq!(state.lock().await.settings.assignment.pins[&id].node, "ppp");
    assert_eq!(
        app.oneshot(pin(&token, &id, "")).await.unwrap().status(),
        StatusCode::SEE_OTHER
    );
    assert!(state.lock().await.settings.assignment.pins.is_empty());
}
#[tokio::test]
async fn healthy_empty_candidate_is_not_replaced_with_committed_subscribers() {
    let dir = tempfile::tempdir().unwrap();
    let (state, _, _) = fixture(dir.path());
    let (settings, mut inventory) = {
        let a = state.lock().await;
        (a.settings.clone(), a.inventories.clone())
    };
    let mut subscribers =
        normalizer::collect(&settings, &inventory, &BTreeMap::new(), orchestrator::now()).unwrap();
    let old = sessions::load(dir.path()).unwrap().unwrap();
    let candidate = sessions::plan(
        &settings,
        &mut subscribers,
        &old.assignments,
        Some(&old),
        &inventory,
        &BTreeMap::new(),
        orchestrator::now(),
    )
    .unwrap();
    lqosync_v3::minimums::allocate(&settings, &mut subscribers).unwrap();
    let compiled = lqosync_v3::model::compile(&settings, &subscribers).unwrap();
    let publication =
        store::publish_assigned(dir.path(), &compiled, &subscribers, &candidate.assignments)
            .unwrap();
    let inv = Arc::make_mut(inventory.get_mut("r1").unwrap());
    inv.collected_at = orchestrator::now();
    for path in [
        "/ppp/active/print",
        "/ip/hotspot/active/print",
        "/ip/dhcp-server/lease/print",
    ] {
        inv.tables.get_mut(path).unwrap().clear();
    }
    {
        let mut a = state.lock().await;
        a.inventories = inventory;
        a.generation = Some(publication.generation);
    }
    let response = routes(state)
        .oneshot(request("/assignments"))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let html = body(response).await;
    assert!(html.contains("Candidate assignments"));
    assert!(html.contains("Subscribers · 0 match"));
    assert!(!html.contains("alice"));
}
#[tokio::test]
async fn corrupt_candidate_state_does_not_fall_back_or_allow_pin_changes() {
    let dir = tempfile::tempdir().unwrap();
    let (state, token, id) = fixture(dir.path());
    std::fs::write(dir.path().join("state/online-assignments.toml"), "corrupt!").unwrap();
    let app = routes(state.clone());
    assert_eq!(
        app.clone()
            .oneshot(request("/assignments"))
            .await
            .unwrap()
            .status(),
        StatusCode::UNPROCESSABLE_ENTITY
    );
    assert_eq!(
        app.oneshot(pin(&token, &id, "ppp")).await.unwrap().status(),
        StatusCode::UNPROCESSABLE_ENTITY
    );
    assert!(state.lock().await.settings.assignment.pins.is_empty());
}
