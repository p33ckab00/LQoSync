use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use http_body_util::BodyExt;
use lqosync_v3::{
    core_plan, model, orchestrator, store,
    web::{App, routes},
};
use std::{collections::BTreeMap, sync::Arc};
use tokio::sync::Mutex;
use tower::ServiceExt;
mod common;
#[path = "common/core.rs"]
mod core_fixture;
async fn page(app: &axum::Router, path: &str) -> String {
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
    assert_eq!(response.status(), StatusCode::OK);
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
async fn end_to_end_review_checks_current_outputs_without_publishing_and_detects_corruption() {
    let dir = tempfile::tempdir().unwrap();
    let (mut s, mut inv, subs, state) = core_fixture::fixture();
    s.auto_stage = false;
    for i in inv.values_mut() {
        Arc::make_mut(i).collected_at = orchestrator::now();
    }
    store::save_settings(dir.path(), &s).unwrap();
    let shaping = model::compile(&s, &subs).unwrap();
    let core = core_plan::build(
        &s,
        &subs,
        &state,
        &inv,
        &BTreeMap::new(),
        orchestrator::now(),
    )
    .unwrap();
    let p = store::publish_orchestrated(dir.path(), &shaping, &subs, &state, Some(&core)).unwrap();
    let mut a = App::open(dir.path().into()).unwrap();
    a.inventories = inv;
    let shared = Arc::new(Mutex::new(a));
    let app = routes(shared.clone());
    let pointer = std::fs::read(dir.path().join("state/current.toml")).unwrap();
    let text = page(&app, "/validation").await;
    assert!(text.contains("Subscriber and output validation"));
    assert!(text.contains("3 subscribers"));
    assert!(text.contains("Current output matches"));
    assert_eq!(
        std::fs::read(dir.path().join("state/current.toml")).unwrap(),
        pointer
    );
    {
        let mut a = shared.lock().await;
        for (id, inventory) in &a.inventories {
            store::atomic_write(
                &dir.path()
                    .join("state/router-inventory")
                    .join(format!("{id}.toml")),
                &toml::to_string(&**inventory).unwrap(),
            )
            .unwrap();
        }
        a.inventories.clear();
    }
    let historical = page(&app, "/validation?origin=saved").await;
    assert!(historical.contains("Historical review"));
    assert!(historical.contains("Unavailable"));
    assert!(shared.lock().await.inventories.is_empty());
    let (_, mut fresh, _, _) = core_fixture::fixture();
    for i in fresh.values_mut() {
        Arc::make_mut(i).collected_at = core.inventory_at["r1"];
    }
    shared.lock().await.inventories = fresh;
    let csv = dir
        .path()
        .join("staging")
        .join(p.generation)
        .join("ShapedDevices.csv");
    std::fs::write(csv, "corrupt output").unwrap();
    let text = page(&app, "/validation").await;
    assert!(text.contains("Conflict"));
    assert!(text.contains("ShapedDevices.csv differs"));
    let network = dir
        .path()
        .join("staging")
        .join(store::current_generation(dir.path()).unwrap())
        .join("network.json");
    std::fs::write(&network, "{}").unwrap();
    assert!(
        page(&app, "/validation")
            .await
            .contains("network.json differs")
    );

    assert_eq!(
        std::fs::read(dir.path().join("state/current.toml")).unwrap(),
        pointer
    );
    {
        let mut a = shared.lock().await;
        Arc::make_mut(a.inventories.get_mut("r1").unwrap()).collected_at = 0;
    }
    let text = page(&app, "/validation").await;
    assert!(text.contains("Unavailable"));
}
#[tokio::test]
async fn missing_setup_and_saved_inputs_do_not_report_live_validation_success() {
    let dir = tempfile::tempdir().unwrap();
    let state = Arc::new(Mutex::new(App::open(dir.path().into()).unwrap()));
    let app = routes(state);
    let text = page(&app, "/validation").await;
    assert!(text.contains("Configure WAN roots and subscriber sources"));
    assert!(text.contains("Unavailable"));
    let text = page(&app, "/validation?origin=saved").await;
    assert!(text.contains("Historical review"));
    assert!(text.contains("Unavailable"));
    assert!(!dir.path().join("state/current.toml").exists());
}

#[test]
fn queued_validation_rejects_changed_settings_read_epochs_and_generation_pointer() {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .max_blocking_threads(1)
        .build()
        .unwrap();
    runtime.block_on(async {
        for change in ["settings", "read", "pointer"] {
            let dir = tempfile::tempdir().unwrap();
            let state = Arc::new(Mutex::new(App::open(dir.path().into()).unwrap()));
            let request = routes(state.clone()).oneshot(
                Request::builder()
                    .uri("/validation")
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
            match change {
                "settings" => state.lock().await.settings.speed_rules.min_rate_fraction = 0.25,
                "read" => {
                    state.lock().await.read_epochs.insert("r1".into(), 1);
                }
                _ => {
                    std::fs::create_dir_all(dir.path().join("state")).unwrap();
                    std::fs::write(
                        dir.path().join("state/current.toml"),
                        "generation='changed-generation'\nstatus='staged'\n",
                    )
                    .unwrap();
                }
            }
            release_tx.send(()).unwrap();
            blocker.await.unwrap();
            let response = request.await.unwrap();
            assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
            let bytes = response.into_body().collect().await.unwrap().to_bytes();
            assert!(String::from_utf8_lossy(&bytes).contains("Validation inputs changed"));
        }
    });
}

#[test]
fn unknown_cli_flags_fail_before_settings_or_inventory_are_accessed() {
    let dir = tempfile::tempdir().unwrap();
    let output = std::process::Command::new(env!("CARGO_BIN_EXE_core-validate"))
        .arg("--unknown-option")
        .env("LQOSYNC3_DATA", dir.path())
        .output()
        .unwrap();
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("Unknown core-validate option"));
    assert!(std::fs::read_dir(dir.path()).unwrap().next().is_none());
}

#[test]
fn review_cli_exits_unsuccessfully_for_historical_or_unconfigured_inputs() {
    let dir = tempfile::tempdir().unwrap();
    let (s, mut inv, _, _) = core_fixture::fixture();
    store::save_settings(dir.path(), &s).unwrap();
    for (id, i) in &mut inv {
        Arc::make_mut(i).collected_at = orchestrator::now();
        store::atomic_write(
            &dir.path()
                .join("state/router-inventory")
                .join(format!("{id}.toml")),
            &toml::to_string(&**i).unwrap(),
        )
        .unwrap();
    }
    let output = std::process::Command::new(env!("CARGO_BIN_EXE_core-validate"))
        .args(["--saved", "--end-to-end"])
        .env("LQOSYNC3_DATA", dir.path())
        .output()
        .unwrap();
    assert!(!output.status.success());
    assert!(
        dir.path()
            .join("state/end-to-end-validation.json")
            .is_file()
    );
    assert!(!dir.path().join("state/current.toml").exists());
}
