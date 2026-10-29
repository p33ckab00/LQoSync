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

#[tokio::test]
async fn speed_rule_form_is_persistent_and_does_not_touch_snapshot_or_generation() {
    let dir = tempfile::tempdir().unwrap();
    let state = Arc::new(Mutex::new(App::open(dir.path().into()).unwrap()));
    std::fs::create_dir_all(dir.path().join("state")).unwrap();
    let snapshot = b"subscribers = []\n";
    let generation = b"generation = 'keep'\nstatus = 'staged'\n";
    std::fs::write(dir.path().join("state/subscribers.toml"), snapshot).unwrap();
    std::fs::write(dir.path().join("state/current.toml"), generation).unwrap();
    let token = state.lock().await.token.clone();
    let app = routes(state.clone());
    for (auth, fraction, status) in [
        ("invalid", 0.25, StatusCode::FORBIDDEN),
        (token.as_str(), 0.25, StatusCode::SEE_OTHER),
        (token.as_str(), 1.5, StatusCode::UNPROCESSABLE_ENTITY),
    ] {
        let response=app.clone().oneshot(Request::builder().method("POST").uri("/speed-rules/save").header("host","localhost:9303").header("content-type","application/x-www-form-urlencoded").body(Body::from(format!("token={auth}&min_rate_fraction={fraction}&min_floor_mbps=0.064&profile_name_fallback=false"))).unwrap()).await.unwrap();
        assert_eq!(response.status(), status);
    }
    let settings = lqosync_v3::store::load_settings(dir.path()).unwrap();
    assert_eq!(settings.speed_rules.min_rate_fraction, 0.25);
    assert!(!settings.speed_rules.profile_name_fallback);
    assert_eq!(
        std::fs::read(dir.path().join("state/subscribers.toml")).unwrap(),
        snapshot
    );
    assert_eq!(
        std::fs::read(dir.path().join("state/current.toml")).unwrap(),
        generation
    );
    let response = app
        .oneshot(
            Request::builder()
                .uri("/speed-rules")
                .header("host", "localhost:9303")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
}

fn preview_inventory() -> lqosync_v3::routeros::Inventory {
    use std::collections::BTreeMap;
    let mut tables = BTreeMap::new();
    for path in [
        "/ppp/active/print",
        "/ppp/secret/print",
        "/ppp/profile/print",
        "/ip/dhcp-server/lease/print",
        "/ip/hotspot/active/print",
        "/ip/hotspot/user/print",
        "/ip/hotspot/user/profile/print",
    ] {
        tables.insert(path.into(), vec![]);
    }
    tables.insert(
        "/ip/dhcp-server/lease/print".into(),
        (0..210)
            .map(|i| {
                [
                    (
                        "mac-address".into(),
                        format!("02:00:00:00:{:02X}:{:02X}", i / 256, i % 256),
                    ),
                    ("server".into(), "dhcp1".into()),
                    ("status".into(), "bound".into()),
                    ("address".into(), format!("192.0.2.{i}")),
                    ("comment".into(), "<script>alert(1)</script>|25M/5M".into()),
                ]
                .into_iter()
                .collect()
            })
            .collect(),
    );
    lqosync_v3::routeros::Inventory {
        router_id: "test".into(),
        collected_at: 1,
        tables,
        errors: BTreeMap::new(),
    }
}
fn add_preview_router(app: &mut App) {
    app.settings.routers.push(toml::from_str("id='test'\nname='Test'\nhost='127.0.0.1'\nport=1\nusername='reader'\npassword_env='LQOSYNC_TEST_UNSET'\nrole='distro'\ntls=false\n").unwrap());
    app.inventories
        .insert("test".into(), Arc::new(preview_inventory()));
}
#[tokio::test]
async fn speed_preview_is_bounded_escaped_and_never_publishes() {
    let dir = tempfile::tempdir().unwrap();
    let mut app = App::open(dir.path().into()).unwrap();
    add_preview_router(&mut app);
    let response = routes(Arc::new(Mutex::new(app)))
        .oneshot(
            Request::builder()
                .uri("/routers/test/speeds")
                .header("host", "localhost:9303")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
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
    assert!(body.contains("210 resolved"));
    assert!(body.contains("dhcp_lease_comment"));
    assert!(body.contains("&lt;script&gt;"));
    assert!(!body.contains("<script>"));
    assert!(!body.contains("02:00:00:00:00:D1"));
    assert!(body.len() < 200_000);
    assert!(!dir.path().join("state/subscribers.toml").exists());
    assert!(!dir.path().join("state/current.toml").exists());
}
#[test]
fn preview_rejects_results_if_policy_changes_while_worker_is_queued() {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .max_blocking_threads(1)
        .build()
        .unwrap();
    runtime.block_on(async {
        let dir = tempfile::tempdir().unwrap();
        let mut app = App::open(dir.path().into()).unwrap();
        add_preview_router(&mut app);
        let state = Arc::new(Mutex::new(app));
        let request = routes(state.clone()).oneshot(
            Request::builder()
                .uri("/routers/test/speeds")
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
        // Poll the request until it queues the preview on the occupied blocking pool.
        tokio::pin!(request);
        assert!(futures_poll_once(request.as_mut()).await.is_none());
        state.lock().await.settings.speed_rules.min_rate_fraction = 0.25;
        release_tx.send(()).unwrap();
        blocker.await.unwrap();
        let response = request.await.unwrap();
        assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
    });
}
async fn futures_poll_once<F: std::future::Future>(
    mut future: std::pin::Pin<&mut F>,
) -> Option<F::Output> {
    std::future::poll_fn(|cx| {
        std::task::Poll::Ready(match future.as_mut().poll(cx) {
            std::task::Poll::Ready(value) => Some(value),
            std::task::Poll::Pending => None,
        })
    })
    .await
}
