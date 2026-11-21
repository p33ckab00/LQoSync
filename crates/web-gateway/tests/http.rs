use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use http_body_util::BodyExt;
use std::{os::unix::fs::PermissionsExt, sync::Arc};
use tower::ServiceExt;
use web_gateway::{
    accounts::Accounts,
    broker,
    http::{WebState, router},
    sessions::Sessions,
};
async fn fixture() -> (
    tempfile::TempDir,
    axum::Router,
    Accounts,
    tokio::task::JoinHandle<()>,
) {
    let temp = tempfile::tempdir().unwrap();
    std::fs::set_permissions(temp.path(), std::fs::Permissions::from_mode(0o700)).unwrap();
    let accounts = Accounts::new(temp.path().join("accounts.db")).unwrap();
    accounts
        .create("operator-a", "test-password-only-123")
        .unwrap();
    accounts
        .create("operator-b", "test-password-only-456")
        .unwrap();
    let path = temp.path().join("broker.sock");
    let listener = tokio::net::UnixListener::bind(&path).unwrap();
    let a = accounts.clone();
    let state = temp.path().to_path_buf();
    let job = tokio::spawn(async move {
        let _ = broker::serve(listener, unsafe { libc::geteuid() }, a, state, None, None).await;
    });
    let app = router(
        WebState {
            broker: path,
            sessions: Sessions::new(temp.path().join("sessions.db")).unwrap(),
            origin: "https://lqosync.test".into(),
            secure: true,
            index: Arc::new("<html data-theme=\"system\"></html>".into()),
        },
        temp.path().into(),
    );
    (temp, app, accounts, job)
}
async fn status_owned(app: &axum::Router, path: &str, cookie: &str) -> axum::response::Response {
    app.clone()
        .oneshot(req("GET", path, cookie, "", ""))
        .await
        .unwrap()
}
fn req(method: &str, path: &str, cookie: &str, origin: &str, body: &str) -> Request<Body> {
    Request::builder()
        .method(method)
        .uri(path)
        .header("cookie", cookie)
        .header("origin", origin)
        .header("content-type", "application/json")
        .body(Body::from(body.to_owned()))
        .unwrap()
}
#[tokio::test]
async fn live_stream_authenticates_emits_each_second_and_stops_after_revocation() {
    let (_dir, app, accounts, job) = fixture().await;
    assert_eq!(
        app.clone()
            .oneshot(req("GET", "/api/v1/overview/live", "", "", ""))
            .await
            .unwrap()
            .status(),
        StatusCode::UNAUTHORIZED
    );
    let signed = app
        .clone()
        .oneshot(req(
            "POST",
            "/api/v1/session",
            "",
            "https://lqosync.test",
            r#"{"username":"operator-a","password":"test-password-only-123"}"#,
        ))
        .await
        .unwrap();
    let cookie = signed.headers()["set-cookie"].to_str().unwrap().to_owned();
    let response = app
        .clone()
        .oneshot(req("GET", "/api/v1/overview/live", &cookie, "", ""))
        .await
        .unwrap();
    assert_eq!(response.headers()["content-type"], "text/event-stream");
    assert_eq!(response.headers()["cache-control"], "no-store");
    assert_eq!(response.headers()["x-accel-buffering"], "no");
    let mut body = response.into_body();
    for _ in 0..2 {
        let frame = tokio::time::timeout(std::time::Duration::from_secs(2), body.frame())
            .await
            .unwrap()
            .unwrap()
            .unwrap();
        let text = String::from_utf8(frame.into_data().unwrap().to_vec()).unwrap();
        assert!(text.contains("data: {") && text.contains("traffic_stale"));
    }
    accounts.disable("operator-a", true).unwrap();
    let frame = tokio::time::timeout(std::time::Duration::from_secs(2), body.frame())
        .await
        .unwrap()
        .unwrap()
        .unwrap();
    assert!(
        String::from_utf8(frame.into_data().unwrap().to_vec())
            .unwrap()
            .contains("data: unauthorized")
    );
    assert!(body.frame().await.is_none());
    accounts.disable("operator-a", false).unwrap();
    // A broker outage must terminate the stream, rather than replay current claims.
    let signed = app
        .clone()
        .oneshot(req(
            "POST",
            "/api/v1/session",
            "",
            "https://lqosync.test",
            r#"{"username":"operator-a","password":"test-password-only-123"}"#,
        ))
        .await
        .unwrap();
    let cookie = signed.headers()["set-cookie"].to_str().unwrap().to_owned();
    let response = app
        .clone()
        .oneshot(req("GET", "/api/v1/overview/live", &cookie, "", ""))
        .await
        .unwrap();
    let mut body = response.into_body();
    assert!(body.frame().await.is_some());
    job.abort();
    tokio::task::yield_now().await;
    let frame = tokio::time::timeout(std::time::Duration::from_secs(2), body.frame())
        .await
        .unwrap()
        .unwrap()
        .unwrap();
    assert!(
        String::from_utf8(frame.into_data().unwrap().to_vec())
            .unwrap()
            .contains("data: unavailable")
    );
    assert!(body.frame().await.is_none());
}
#[tokio::test]
async fn sign_in_dashboard_revocation_and_no_account_routes() {
    let (_dir, app, accounts, job) = fixture().await;
    assert_eq!(
        app.clone()
            .oneshot(req("GET", "/api/v1/overview", "", "", " "))
            .await
            .unwrap()
            .status(),
        StatusCode::UNAUTHORIZED
    );
    let body = r#"{"username":"operator-a","password":"test-password-only-123"}"#;
    assert_eq!(
        app.clone()
            .oneshot(req(
                "POST",
                "/api/v1/session",
                "",
                "https://attacker.test",
                body
            ))
            .await
            .unwrap()
            .status(),
        StatusCode::FORBIDDEN
    );
    let signed = app
        .clone()
        .oneshot(req(
            "POST",
            "/api/v1/session",
            "",
            "https://lqosync.test",
            body,
        ))
        .await
        .unwrap();
    assert_eq!(signed.status(), StatusCode::OK);
    let cookie = signed.headers()["set-cookie"].to_str().unwrap().to_owned();
    assert!(
        cookie.contains("HttpOnly")
            && cookie.contains("Secure")
            && cookie.contains("SameSite=Strict")
    );
    let json: serde_json::Value =
        serde_json::from_slice(&signed.into_body().collect().await.unwrap().to_bytes()).unwrap();
    let dashboard = app
        .clone()
        .oneshot(req("GET", "/api/v1/overview", &cookie, "", ""))
        .await
        .unwrap();
    assert_eq!(dashboard.status(), StatusCode::OK);
    assert_eq!(dashboard.headers()["cache-control"], "no-store");
    // History windows: authenticated, and only the three bounded windows exist.
    let status = |path: &'static str, cookie: String| {
        let app = app.clone();
        async move {
            app.oneshot(req("GET", path, &cookie, "", ""))
                .await
                .unwrap()
        }
    };
    assert_eq!(
        status("/api/v1/traffic?window=1h", String::new())
            .await
            .status(),
        StatusCode::UNAUTHORIZED
    );
    assert_eq!(
        status("/api/v1/traffic?window=7d", cookie.clone())
            .await
            .status(),
        StatusCode::BAD_REQUEST
    );
    let window = status("/api/v1/traffic?window=24h", cookie.clone()).await;
    assert_eq!(window.status(), StatusCode::OK);
    let window: web_contract::TrafficWindow =
        serde_json::from_slice(&window.into_body().collect().await.unwrap().to_bytes()).unwrap();
    assert_eq!((window.window.as_str(), window.bucket_secs), ("24h", 360));
    // The router inventory: authenticated, read from the service cache, no router I/O.
    assert_eq!(
        status("/api/v1/routers", String::new()).await.status(),
        StatusCode::UNAUTHORIZED
    );
    assert_eq!(
        status("/api/v1/routers", cookie.clone()).await.status(),
        StatusCode::SERVICE_UNAVAILABLE,
        "no cache yet"
    );
    let inventory = web_contract::routers::RouterInventory {
        version: web_contract::routers::VERSION,
        updated: 990,
        fresh_for: 300,
        routers: vec![web_contract::routers::RouterSummary {
            id: "router-core".into(),
            name: "Core".into(),
            ..Default::default()
        }],
        config_digest: String::new(),
        shaping: Default::default(),
        truncated: false,
    };
    std::fs::write(
        _dir.path().join("routers.json"),
        serde_json::to_vec(&inventory).unwrap(),
    )
    .unwrap();
    let listed = status("/api/v1/routers", cookie.clone()).await;
    assert_eq!(listed.status(), StatusCode::OK);
    assert_eq!(listed.headers()["cache-control"], "no-store");
    let listed: web_contract::routers::RouterInventory =
        serde_json::from_slice(&listed.into_body().collect().await.unwrap().to_bytes()).unwrap();
    assert_eq!(listed, inventory);
    // Topology: authenticated reads of the service cache. No router I/O, no write route.
    assert_eq!(
        status("/api/v1/topology", String::new()).await.status(),
        StatusCode::UNAUTHORIZED
    );
    assert_eq!(
        status("/api/v1/topology", cookie.clone()).await.status(),
        StatusCode::SERVICE_UNAVAILABLE,
        "no cache yet"
    );
    let view = web_contract::topology::TopologyView {
        version: web_contract::topology::VERSION,
        updated: 990,
        fresh_for: 300,
        snapshot_id: "r1-c1".into(),
        subscribers: vec![web_contract::topology::Subscriber {
            id: "sub-1".into(),
            identities: Vec::new(),
            plan_id: None,
            pin: None,
            assignment: None,
            active: None,
        }],
        ..Default::default()
    };
    std::fs::write(
        _dir.path().join(web_contract::topology::CACHE),
        serde_json::to_vec(&view).unwrap(),
    )
    .unwrap();
    let summary = status("/api/v1/topology", cookie.clone()).await;
    assert_eq!(summary.status(), StatusCode::OK);
    let summary: web_contract::topology::TopologySummary =
        serde_json::from_slice(&summary.into_body().collect().await.unwrap().to_bytes()).unwrap();
    assert_eq!(summary.subscribers_total, 1);
    for (path, code) in [
        ("/api/v1/topology/assignments?state=offline", StatusCode::OK),
        ("/api/v1/topology/observations?kind=ppp", StatusCode::OK),
        ("/api/v1/topology/subscribers/sub-1", StatusCode::OK),
        ("/api/v1/topology/subscribers/sub-2", StatusCode::NOT_FOUND),
        (
            "/api/v1/topology/assignments?cursor=old~0~0",
            StatusCode::CONFLICT,
        ),
    ] {
        assert_eq!(status(path, cookie.clone()).await.status(), code, "{path}");
    }
    // A topology change: same origin and CSRF, then 202 and an operation URL.
    let key = "0f8b5a3c-1d2e-4f60-9a7b-000000000001";
    let change =
        serde_json::json!({"key": key, "expected_revision": 1, "edit": {"kind": "plan.create",
        "name": "Gold", "down_mbps": 100, "up_mbps": 50}})
        .to_string();
    // Every web change is reviewed: no route commits without a proposal.
    let unreviewed = app
        .clone()
        .oneshot(req(
            "POST",
            "/api/v1/topology/mutations",
            &cookie,
            "https://lqosync.test",
            &change,
        ))
        .await
        .unwrap();
    assert_eq!(unreviewed.status(), StatusCode::NOT_FOUND);
    // A canvas edit saves without a review: same origin and CSRF, then 202.
    let edit_key = "0f8b5a3c-1d2e-4f60-9a7b-000000000009";
    let edit = serde_json::json!({"key": edit_key, "expected_revision": 1,
        "edit": {"kind": "node.remove", "node_id": "n1"}})
    .to_string();
    let layout = r#"{"expected_revision":0,"positions":{"n1":[10.0,20.5]}}"#;
    for (method, path, body) in [
        ("POST", "/api/v1/topology/edits", edit.as_str()),
        ("PUT", "/api/v1/topology/layout", layout),
    ] {
        let refused = app
            .clone()
            .oneshot(req(method, path, &cookie, "https://lqosync.test", body))
            .await
            .unwrap();
        assert_eq!(refused.status(), StatusCode::FORBIDDEN, "{path}");
    }
    let csrf: axum::http::HeaderValue = json["csrf"].as_str().unwrap().parse().unwrap();
    let mut post = req(
        "POST",
        "/api/v1/topology/edits",
        &cookie,
        "https://lqosync.test",
        &edit,
    );
    post.headers_mut().insert("x-csrf-token", csrf.clone());
    let queued = app.clone().oneshot(post).await.unwrap();
    assert_eq!(queued.status(), StatusCode::ACCEPTED);
    assert_eq!(
        queued.headers()["location"],
        format!("/api/v1/topology/operations/{edit_key}")
    );
    // No service socket in this fixture: the layout save is unavailable and writes nothing.
    let mut put = req(
        "PUT",
        "/api/v1/topology/layout",
        &cookie,
        "https://lqosync.test",
        layout,
    );
    put.headers_mut().insert("x-csrf-token", csrf);
    assert_eq!(
        app.clone().oneshot(put).await.unwrap().status(),
        StatusCode::SERVICE_UNAVAILABLE
    );
    let no_csrf = app
        .clone()
        .oneshot(req(
            "POST",
            "/api/v1/topology/proposals",
            &cookie,
            "https://lqosync.test",
            &change,
        ))
        .await
        .unwrap();
    assert_eq!(no_csrf.status(), StatusCode::FORBIDDEN);
    let mut mutate = req(
        "POST",
        "/api/v1/topology/proposals",
        &cookie,
        "https://lqosync.test",
        &change,
    );
    mutate.headers_mut().insert(
        "x-csrf-token",
        json["csrf"].as_str().unwrap().parse().unwrap(),
    );
    let queued = app.clone().oneshot(mutate).await.unwrap();
    assert_eq!(queued.status(), StatusCode::ACCEPTED);
    assert_eq!(
        queued.headers()["location"],
        format!("/api/v1/topology/operations/{key}")
    );
    // No service socket in this fixture: the review fails and nothing is written.
    let mut state = String::new();
    for _ in 0..50 {
        let op = status_owned(&app, &format!("/api/v1/topology/operations/{key}"), &cookie).await;
        assert_eq!(op.status(), StatusCode::OK);
        let op: web_contract::topology::Operation =
            serde_json::from_slice(&op.into_body().collect().await.unwrap().to_bytes()).unwrap();
        state = op.state;
        if state != "queued" && state != "running" {
            break;
        }
        tokio::time::sleep(std::time::Duration::from_millis(20)).await;
    }
    assert_eq!(state, "failed");
    let posted = app
        .clone()
        .oneshot(req(
            "POST",
            "/api/v1/topology",
            &cookie,
            "https://lqosync.test",
            "{}",
        ))
        .await
        .unwrap();
    assert_eq!(posted.status(), StatusCode::METHOD_NOT_ALLOWED);
    // Deep links load the page shell; the page reads data through the API.
    for path in [
        "/routers",
        "/routers/router-core",
        "/topology",
        "/pppoe",
        "/ipoe",
    ] {
        let page = status(path, String::new()).await;
        assert_eq!(page.status(), StatusCode::OK, "{path}");
    }
    for route in ["/api/v1/users", "/api/v1/signup", "/api/v1/reset-password"] {
        for method in ["GET", "POST", "PATCH", "DELETE"] {
            assert_eq!(
                app.clone()
                    .oneshot(req(method, route, &cookie, "https://lqosync.test", "{}"))
                    .await
                    .unwrap()
                    .status(),
                StatusCode::NOT_FOUND
            );
        }
    }
    assert_eq!(
        app.clone()
            .oneshot(req(
                "DELETE",
                "/api/v1/session",
                &cookie,
                "https://lqosync.test",
                ""
            ))
            .await
            .unwrap()
            .status(),
        StatusCode::FORBIDDEN
    );
    let request = Request::builder()
        .method("DELETE")
        .uri("/api/v1/session")
        .header("cookie", &cookie)
        .header("origin", "https://lqosync.test")
        .header("x-csrf-token", json["csrf"].as_str().unwrap())
        .body(Body::empty())
        .unwrap();
    assert_eq!(
        app.clone().oneshot(request).await.unwrap().status(),
        StatusCode::NO_CONTENT
    );
    assert_eq!(
        app.clone()
            .oneshot(req("GET", "/api/v1/session", &cookie, "", ""))
            .await
            .unwrap()
            .status(),
        StatusCode::UNAUTHORIZED
    );
    let signed = app
        .clone()
        .oneshot(req(
            "POST",
            "/api/v1/session",
            "",
            "https://lqosync.test",
            body,
        ))
        .await
        .unwrap();
    let cookie = signed.headers()["set-cookie"].to_str().unwrap().to_owned();
    accounts.disable("operator-a", true).unwrap();
    assert_eq!(
        app.clone()
            .oneshot(req("GET", "/api/v1/overview", &cookie, "", ""))
            .await
            .unwrap()
            .status(),
        StatusCode::UNAUTHORIZED
    );
    job.abort();
}
#[tokio::test]
async fn theme_is_safely_restored_before_wasm_and_broker_failure_fails_closed() {
    let (_dir, app, _a, job) = fixture().await;
    let page = app
        .clone()
        .oneshot(req("GET", "/", "lqosync_theme=dark", "", ""))
        .await
        .unwrap();
    let text = String::from_utf8(
        page.into_body()
            .collect()
            .await
            .unwrap()
            .to_bytes()
            .to_vec(),
    )
    .unwrap();
    assert!(text.contains("data-theme=\"dark\""));
    let page = app
        .clone()
        .oneshot(req("GET", "/", "lqosync_theme=evil", "", ""))
        .await
        .unwrap();
    let text = String::from_utf8(
        page.into_body()
            .collect()
            .await
            .unwrap()
            .to_bytes()
            .to_vec(),
    )
    .unwrap();
    assert!(text.contains("data-theme=\"system\""));
    let body = r#"{"username":"operator-a","password":"test-password-only-123"}"#;
    let signed = app
        .clone()
        .oneshot(req(
            "POST",
            "/api/v1/session",
            "",
            "https://lqosync.test",
            body,
        ))
        .await
        .unwrap();
    let cookie = signed.headers()["set-cookie"].to_str().unwrap().to_owned();
    job.abort();
    tokio::task::yield_now().await;
    assert_eq!(
        app.oneshot(req("GET", "/api/v1/overview", &cookie, "", ""))
            .await
            .unwrap()
            .status(),
        StatusCode::SERVICE_UNAVAILABLE
    );
}
