use axum::{body::Body, http::Request};
use http_body_util::BodyExt;
use lqosync_v3::{
    core_diagnostics, orchestrator, store,
    web::{App, routes},
};
use std::{collections::BTreeMap, sync::Arc};
use tokio::sync::Mutex;
use tower::ServiceExt;
mod common;
#[path = "common/core.rs"]
mod core_fixture;
async fn get(app: &axum::Router, path: &str) -> String {
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
    assert_eq!(response.status(), 200);
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
#[tokio::test]
async fn live_and_saved_policy_diagnostics_are_distinct_and_get_never_publishes() {
    let (s, mut inv, _, _) = core_fixture::fixture();
    let dir = tempfile::tempdir().unwrap();
    Arc::make_mut(inv.get_mut("r1").unwrap()).collected_at = orchestrator::now();
    store::save_settings(dir.path(), &s).unwrap();
    store::atomic_write(
        &dir.path().join("state/router-inventory/r1.toml"),
        &toml::to_string(&*inv["r1"]).unwrap(),
    )
    .unwrap();
    let mut a = App::open(dir.path().into()).unwrap();
    a.inventories = inv;
    let state = Arc::new(Mutex::new(a));
    orchestrator::refresh(state.clone()).await;
    let pointer = std::fs::read(dir.path().join("state/current.toml")).unwrap();
    let app = routes(state.clone());
    let live = get(&app, "/core-review").await;
    assert!(live.contains("Core policy review"));
    assert!(live.contains("Fresh"));
    assert!(live.contains("Pass"));
    let saved = get(&app, "/core-review?origin=saved").await;
    assert!(saved.contains("Historical"));
    assert!(saved.contains("Saved inventory"));
    assert_eq!(
        pointer,
        std::fs::read(dir.path().join("state/current.toml")).unwrap()
    );
    let (snapshots, errors) = core_diagnostics::saved(&s, dir.path());
    assert_eq!(snapshots.len(), 1);
    assert!(errors.is_empty());
    assert_eq!(state.lock().await.inventories.len(), 1);
}
#[tokio::test]
async fn review_shows_multiple_findings_escapes_policy_names_and_hides_passwords() {
    let (mut s, mut inv, _, _) = core_fixture::fixture();
    let dir = tempfile::tempdir().unwrap();
    s.core_lists.bindings[0].list = "<script>bad</script>".into();
    let inventory = Arc::make_mut(inv.get_mut("r1").unwrap());
    inventory.collected_at = orchestrator::now();
    inventory
        .tables
        .get_mut("/ip/firewall/mangle/print")
        .unwrap()[0]
        .insert(
            "src-address-list".into(),
            s.core_lists.bindings[0].list.clone(),
        );
    inventory
        .tables
        .get_mut("/ip/firewall/mangle/print")
        .unwrap()[0]
        .insert("new-routing-mark".into(), "wrong".into());
    inventory.tables.get_mut("/ip/route/print").unwrap()[0]
        .insert("immediate-gw".into(), "1.1.1.1%wrong".into());
    store::save_settings(dir.path(), &s).unwrap();
    store::atomic_write(
        &dir.path().join("credentials.toml"),
        "[routers.r1]\npassword='PRIVATE_TEST_SECRET'\n",
    )
    .unwrap();
    let mut a = App::open(dir.path().into()).unwrap();
    a.inventories = inv;
    let app = routes(Arc::new(Mutex::new(a)));
    let html = get(&app, "/core-review").await;
    assert!(html.contains("wrong-routing-table"));
    assert!(html.contains("wrong-wan-default"));
    assert!(html.contains("Conflict"));
    assert!(!html.contains("<script>"));
    assert!(html.contains("&lt;script&gt;"));
    assert!(!html.contains("PRIVATE_TEST_SECRET"));
    assert!(!dir.path().join("state/current.toml").exists());
    let missing = core_diagnostics::review(
        &s,
        &BTreeMap::new(),
        &BTreeMap::new(),
        101,
        core_diagnostics::Origin::Live,
    );
    assert_eq!(missing.cores.len(), 1);
}

#[test]
fn queued_review_rejects_changed_settings_or_started_read() {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .max_blocking_threads(1)
        .build()
        .unwrap();
    runtime.block_on(async {
        for change_settings in [true, false] {
            let dir = tempfile::tempdir().unwrap();
            let (s, inv, _, _) = core_fixture::fixture();
            store::save_settings(dir.path(), &s).unwrap();
            let mut a = App::open(dir.path().into()).unwrap();
            a.inventories = inv;
            let state = Arc::new(Mutex::new(a));
            let request = routes(state.clone()).oneshot(
                Request::builder()
                    .uri("/core-review")
                    .header("host", "localhost:9303")
                    .body(Body::empty())
                    .unwrap(),
            );
            let (started_tx, started_rx) = tokio::sync::oneshot::channel();
            let (release_tx, release_rx) = std::sync::mpsc::channel();
            let blocker = tokio::task::spawn_blocking(move || {
                started_tx.send(()).unwrap();
                release_rx.recv().unwrap();
            });
            started_rx.await.unwrap();
            tokio::pin!(request);
            let pending = std::future::poll_fn(|cx| {
                std::task::Poll::Ready(request.as_mut().poll(cx).is_pending())
            })
            .await;
            assert!(pending);
            {
                let mut a = state.lock().await;
                if change_settings {
                    a.settings.speed_rules.min_rate_fraction = 0.25;
                } else {
                    *a.read_epochs.entry("r1".into()).or_default() += 1;
                }
            }
            release_tx.send(()).unwrap();
            blocker.await.unwrap();
            let response = request.await.unwrap();
            assert_eq!(
                response.status(),
                axum::http::StatusCode::UNPROCESSABLE_ENTITY
            );
            let body = response.into_body().collect().await.unwrap().to_bytes();
            assert!(String::from_utf8_lossy(&body).contains("Core review inputs changed"));
            assert!(!dir.path().join("state/current.toml").exists());
        }
    });
}
