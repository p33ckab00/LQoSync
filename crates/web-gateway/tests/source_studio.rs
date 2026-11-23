//! Topology Studio routes through the real broker and job queue, with a fake service socket.
use axum::{body::Body, http::Request, http::StatusCode};
use http_body_util::BodyExt;
use serde_json::{Value, json};
use std::os::unix::fs::PermissionsExt;
use std::sync::{Arc, Mutex};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt};
use tower::ServiceExt;
use web_gateway::{
    accounts::Accounts,
    broker,
    http::{WebState, router},
    sessions::Sessions,
};

const ORIGIN: &str = "https://lqosync.test";
const SECRET: &str = "s3cret-draft-value";
const KEY: &str = "studio-test-0000000001";

/// A fake service API socket. It records each request line. `studio = false` is an older
/// service without Studio operations.
fn core(dir: &std::path::Path, studio: bool) -> Arc<Mutex<Vec<Value>>> {
    let seen = Arc::new(Mutex::new(Vec::new()));
    let listener = tokio::net::UnixListener::bind(dir.join("api.sock")).unwrap();
    let log = seen.clone();
    tokio::spawn(async move {
        while let Ok((stream, _)) = listener.accept().await {
            let log = log.clone();
            tokio::spawn(async move {
                let (read, mut write) = stream.into_split();
                let mut line = String::new();
                tokio::io::BufReader::new(read)
                    .read_line(&mut line)
                    .await
                    .unwrap();
                let v: Value = serde_json::from_str(&line).unwrap();
                log.lock().unwrap().push(v.clone());
                let result = match v["op"].as_str().unwrap() {
                    "source_studio.capabilities" if studio => Some(json!({
                        "version": 1, "features": ["draft_test"], "fields": [],
                        "limits": web_contract::source_studio::StudioLimits::default()})),
                    "source_studio.test" if studio => Some(json!({
                        "connection_fingerprint": "f1", "at": 1, "elapsed_ms": 2,
                        "checks": [{"stage": "transport", "state": "passed", "code": "ok"}]})),
                    "topology.web.operation" => Some(Value::Null),
                    _ => None,
                };
                let answer = match result {
                    Some(r) => json!({"v": 1, "id": v["id"], "ok": true, "result": r}),
                    None => json!({"v": 1, "id": v["id"], "ok": false,
                                   "error": {"code": "unknown_op", "message": "unknown op"}}),
                };
                let mut bytes = serde_json::to_vec(&answer).unwrap();
                bytes.push(b'\n');
                let _ = write.write_all(&bytes).await;
            });
        }
    });
    seen
}

async fn app(studio: bool) -> (tempfile::TempDir, axum::Router, Arc<Mutex<Vec<Value>>>) {
    let temp = tempfile::tempdir().unwrap();
    std::fs::set_permissions(temp.path(), std::fs::Permissions::from_mode(0o700)).unwrap();
    let seen = core(temp.path(), studio);
    let accounts = Accounts::new(temp.path().join("accounts.db")).unwrap();
    accounts
        .create("operator-a", "test-password-only-123")
        .unwrap();
    let path = temp.path().join("broker.sock");
    let listener = tokio::net::UnixListener::bind(&path).unwrap();
    let state = temp.path().to_path_buf();
    tokio::spawn(async move {
        let uid = unsafe { libc::geteuid() };
        let _ = broker::serve(listener, uid, accounts, state, None, None).await;
    });
    let app = router(
        WebState {
            broker: path,
            sessions: Sessions::new(temp.path().join("sessions.db")).unwrap(),
            origin: ORIGIN.into(),
            secure: true,
            index: Arc::new("<html></html>".into()),
        },
        temp.path().into(),
    );
    (temp, app, seen)
}

fn req(method: &str, path: &str, cookie: &str, csrf: &str, body: String) -> Request<Body> {
    Request::builder()
        .method(method)
        .uri(path)
        .header("cookie", cookie)
        .header("origin", ORIGIN)
        .header("x-csrf-token", csrf)
        .header("content-type", "application/json")
        .body(Body::from(body))
        .unwrap()
}

async fn send(app: &axum::Router, r: Request<Body>) -> (StatusCode, Value) {
    let r = app.clone().oneshot(r).await.unwrap();
    let status = r.status();
    let bytes = r.into_body().collect().await.unwrap().to_bytes();
    (
        status,
        serde_json::from_slice(&bytes).unwrap_or(Value::Null),
    )
}

async fn sign_in(app: &axum::Router) -> (String, String) {
    let body = r#"{"username":"operator-a","password":"test-password-only-123"}"#;
    let r = app
        .clone()
        .oneshot(req("POST", "/api/v1/session", "", "", body.into()))
        .await
        .unwrap();
    let cookie = r.headers()["set-cookie"].to_str().unwrap().to_owned();
    let json: Value =
        serde_json::from_slice(&r.into_body().collect().await.unwrap().to_bytes()).unwrap();
    (cookie, json["csrf"].as_str().unwrap().to_owned())
}

fn job(username: &str) -> String {
    json!({"key": KEY, "job": {"kind": "draft_test", "required_sources": ["ppp"],
        "connection": {"address": "198.51.100.7", "port": 8729, "tls": true, "tls_verify": true,
                       "username": username, "password": SECRET, "ca_pem": null}}})
    .to_string()
}

#[tokio::test]
async fn a_draft_test_job_reaches_the_service_and_answers_on_the_operation() {
    let (_dir, app, seen) = app(true).await;
    let caps = "/api/v1/topology/source-capabilities";
    let (status, body) = send(&app, req("GET", caps, "", "", String::new())).await;
    assert_eq!(
        (status, &body["code"]),
        (StatusCode::UNAUTHORIZED, &json!("unauthorized"))
    );
    let (cookie, csrf) = sign_in(&app).await;
    let (status, body) = send(&app, req("GET", caps, &cookie, "", String::new())).await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(body["features"], json!(["draft_test"]));
    let jobs = "/api/v1/topology/source-jobs";
    let (status, _) = send(&app, req("POST", jobs, &cookie, "wrong", job("studio"))).await;
    assert_eq!(status, StatusCode::FORBIDDEN);
    let r = app
        .clone()
        .oneshot(req("POST", jobs, &cookie, &csrf, job("studio")))
        .await
        .unwrap();
    assert_eq!(r.status(), StatusCode::ACCEPTED);
    let location = r.headers()["location"].to_str().unwrap().to_owned();
    assert_eq!(location, format!("/api/v1/topology/operations/{KEY}"));
    let mut done = Value::Null;
    for _ in 0..100 {
        (_, done) = send(&app, req("GET", &location, &cookie, "", String::new())).await;
        if done["state"] == "done" || done["state"] == "failed" {
            break;
        }
        tokio::time::sleep(std::time::Duration::from_millis(20)).await;
    }
    assert_eq!(done["state"], json!("done"), "{done}");
    assert_eq!(done["source_result"]["kind"], json!("draft_test"));
    assert!(!done.to_string().contains(SECRET));
    // The same key and payload is a retry; a changed payload needs a new key.
    let (status, _) = send(&app, req("POST", jobs, &cookie, &csrf, job("studio"))).await;
    assert_eq!(status, StatusCode::ACCEPTED);
    let (status, body) = send(&app, req("POST", jobs, &cookie, &csrf, job("other"))).await;
    assert_eq!(
        (status, &body["code"]),
        (StatusCode::CONFLICT, &json!("key_reused"))
    );
    let tests: Vec<Value> = seen
        .lock()
        .unwrap()
        .iter()
        .filter(|v| v["op"] == "source_studio.test")
        .cloned()
        .collect();
    assert_eq!(tests.len(), 1, "a retry runs the test once");
    assert_eq!(tests[0]["args"]["connection"]["password"], json!(SECRET));
    assert!(tests[0]["args"].get("actor").is_none());
}

#[tokio::test]
async fn source_jobs_refuse_bad_bodies_and_unreleased_jobs() {
    let (_dir, app, seen) = app(true).await;
    let (cookie, csrf) = sign_in(&app).await;
    let jobs = "/api/v1/topology/source-jobs";
    let collect = json!({"key": KEY, "job": {"kind": "collect", "router_id": "lq3-a1",
        "expected_config_digest": "d", "sources": ["ppp"]}});
    let mut unknown: Value = serde_json::from_str(&job("studio")).unwrap();
    unknown["job"]["path"] = json!("/system/reboot");
    let big = "x".repeat(33 * 1024);
    for (body, want, code) in [
        (collect.to_string(), StatusCode::NOT_FOUND, "unavailable"),
        (unknown.to_string(), StatusCode::BAD_REQUEST, "invalid"),
        (big, StatusCode::PAYLOAD_TOO_LARGE, "too_large"),
    ] {
        let (status, body) = send(&app, req("POST", jobs, &cookie, &csrf, body)).await;
        assert_eq!((status, &body["code"]), (want, &json!(code)), "{body}");
    }
    assert!(
        !seen
            .lock()
            .unwrap()
            .iter()
            .any(|v| v["op"] == "source_studio.test"),
        "no refused job reaches the service"
    );
}

#[tokio::test]
async fn an_older_service_has_no_studio_capabilities() {
    let (_dir, app, _) = app(false).await;
    let (cookie, _) = sign_in(&app).await;
    let caps = "/api/v1/topology/source-capabilities";
    let (status, body) = send(&app, req("GET", caps, &cookie, "", String::new())).await;
    assert_eq!(
        (status, &body["code"]),
        (StatusCode::NOT_FOUND, &json!("unavailable"))
    );
}
