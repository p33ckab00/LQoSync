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
        let _ = broker::serve(listener, unsafe { libc::geteuid() }, a, state).await;
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
