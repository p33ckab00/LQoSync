use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use http_body_util::BodyExt;
use lqosync_v3::web::{App, routes};
use std::sync::Arc;
use tokio::sync::Mutex;
use tower::ServiceExt;

#[tokio::test]
async fn all_pages_render_and_reject_mutation_without_token() {
    let dir = tempfile::tempdir().unwrap();
    let state = Arc::new(Mutex::new(App::open(dir.path().into()).unwrap()));
    let app = routes(state);
    for path in ["/", "/routers", "/topology", "/subscribers", "/settings"] {
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
        assert_eq!(response.headers()["cache-control"], "no-store");
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
        assert!(body.contains("Rust backend and Rust-rendered UI"));
    }
    let before = std::fs::read(dir.path().join("settings.toml")).unwrap();
    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/settings/save")
                .header("host", "localhost:9303")
                .header("content-type", "application/x-www-form-urlencoded")
                .body(Body::from(
                    "token=invalid&interval_seconds=2&auto_stage=true",
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::FORBIDDEN);
    assert_eq!(
        before,
        std::fs::read(dir.path().join("settings.toml")).unwrap()
    );
}

#[tokio::test]
async fn settings_save_is_persistent_but_invalid_interval_is_not() {
    let dir = tempfile::tempdir().unwrap();
    let state = Arc::new(Mutex::new(App::open(dir.path().into()).unwrap()));
    let token = state.lock().await.token.clone();
    let app = routes(state);
    for (interval, status) in [
        (12, StatusCode::SEE_OTHER),
        (0, StatusCode::UNPROCESSABLE_ENTITY),
    ] {
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/settings/save")
                    .header("host", "localhost:9303")
                    .header("content-type", "application/x-www-form-urlencoded")
                    .body(Body::from(format!(
                        "token={token}&interval_seconds={interval}&auto_stage=true"
                    )))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), status);
    }
    assert_eq!(
        lqosync_v3::store::load_settings(dir.path())
            .unwrap()
            .interval_seconds,
        12
    );
}

#[tokio::test]
async fn rejects_untrusted_host_headers() {
    let dir = tempfile::tempdir().unwrap();
    let state = Arc::new(Mutex::new(App::open(dir.path().into()).unwrap()));
    let response = routes(state)
        .oneshot(
            Request::builder()
                .uri("/")
                .header("host", "attacker.example")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn manual_child_over_computed_parent_does_not_change_settings() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::create_dir(dir.path().join("state")).unwrap();
    std::fs::write(
        dir.path().join("settings.toml"),
        include_str!("../examples/settings.toml"),
    )
    .unwrap();
    std::fs::write(
        dir.path().join("state/subscribers.toml"),
        include_str!("../examples/subscribers.toml"),
    )
    .unwrap();
    let state = Arc::new(Mutex::new(App::open(dir.path().into()).unwrap()));
    let token = state.lock().await.token.clone();
    let before = std::fs::read(dir.path().join("settings.toml")).unwrap();
    let body = format!(
        "token={token}&original_id=&id=child&name=Child&parent=ppp50&kind=site&router=&source=&mode=manual&download=50&upload=10&download_factor=1&upload_factor=1"
    );
    let response = routes(state)
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/nodes/save")
                .header("host", "localhost:9303")
                .header("content-type", "application/x-www-form-urlencoded")
                .body(Body::from(body))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(
        before,
        std::fs::read(dir.path().join("settings.toml")).unwrap()
    );
}

#[tokio::test]
async fn router_inventory_requires_token_and_keeps_snapshot_after_connection_error() {
    let dir = tempfile::tempdir().unwrap();
    let state = Arc::new(Mutex::new(App::open(dir.path().into()).unwrap()));
    let token = state.lock().await.token.clone();
    let app = routes(state.clone());
    std::fs::create_dir_all(dir.path().join("state")).unwrap();
    let snapshot = b"subscribers = []\n";
    std::fs::write(dir.path().join("state/subscribers.toml"), snapshot).unwrap();
    // Omit the new TLS fields to verify compatibility with earlier forms/settings.
    let form = format!(
        "token={token}&original_id=&id=test&name=Test&host=127.0.0.1&port=1&username=reader&password_env=LQOSYNC_TEST_UNSET_INVENTORY_PASSWORD_93&role=core&tls=false"
    );
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/routers/save")
                .header("host", "localhost:9303")
                .header("content-type", "application/x-www-form-urlencoded")
                .body(Body::from(form))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::SEE_OTHER);
    let stored = lqosync_v3::store::load_settings(dir.path()).unwrap();
    assert_eq!(stored.routers[0].timeout_seconds, 10);
    assert!(stored.routers[0].tls_ca_file.is_empty());
    for (value, status) in [
        ("invalid", StatusCode::FORBIDDEN),
        (token.as_str(), StatusCode::SEE_OTHER),
    ] {
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/routers/test/inspect")
                    .header("host", "localhost:9303")
                    .header("content-type", "application/x-www-form-urlencoded")
                    .body(Body::from(format!("token={value}")))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), status);
    }
    assert!(
        state.lock().await.inventory_errors["test"].contains("environment variable is missing")
    );
    assert_eq!(
        std::fs::read(dir.path().join("state/subscribers.toml")).unwrap(),
        snapshot
    );
    assert!(!dir.path().join("state/router-inventory/test.toml").exists());
}
