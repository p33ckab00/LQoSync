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
                    "source_studio.collect" if studio => Some(json!({
                        "id": "snap-1", "router_id": v["args"]["router_id"], "config_digest": "d1",
                        "collected_from": 1, "collected_to": 2, "expires_at": 3,
                        "tables": [{"table": "routes", "state": "failed", "records": 0,
                                    "collected_at": 2, "issue_code": "trap"}]})),
                    "source_studio.evaluate" if studio => Some(json!({
                        "id": "cand-b1-1", "base_revision": v["args"]["expected_revision"],
                        "config_digest": "d1", "snapshot_id": v["args"]["snapshot_id"],
                        "draft_digest": "x", "expires_at": 9})),
                    "source_studio.candidate" if studio => Some(
                        match (
                            v["args"]["id"].as_str().unwrap(),
                            v["args"]["actor"].as_str().unwrap(),
                            v["args"]["part"].as_str().unwrap(),
                        ) {
                            ("cand-b1-1", "operator-a", "summary") => {
                                json!({"status": "ok", "value": {
                            "reference": {"id": "cand-b1-1", "base_revision": "7", "config_digest": "d1",
                                          "snapshot_id": "snap-1", "draft_digest": "x", "expires_at": 9},
                            "graph": {"nodes": [], "flows": [], "nodes_truncated": false, "flows_truncated": false},
                            "diff": {"added": 0, "removed": 0, "changed": 0, "unchanged": 0, "markers": [], "markers_truncated": false},
                            "issues": {"blockers": 1, "warnings": 0, "information": 0},
                            "publishable": false, "complete": true}})
                            }
                            ("cand-b1-1", "operator-a", "issues") => {
                                json!({"status": "ok", "value": {
                            "candidate_id": "cand-b1-1", "next_cursor": null, "total": 1, "items": [
                                {"code": "missing_ppp_join", "severity": "blocker", "entity_ref": "table:ppp_active#0",
                                 "field": null, "message": "m", "remedy": null}]}})
                            }
                            ("cand-b1-2", _, _) => json!({"status": "expired"}),
                            _ => json!({"status": "not_found"}),
                        },
                    ),
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
async fn source_jobs_refuse_bad_bodies() {
    let (_dir, app, seen) = app(true).await;
    let (cookie, csrf) = sign_in(&app).await;
    let jobs = "/api/v1/topology/source-jobs";
    let mut unknown: Value = serde_json::from_str(&job("studio")).unwrap();
    unknown["job"]["path"] = json!("/system/reboot");
    let big = "x".repeat(33 * 1024);
    for (body, want, code) in [
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

async fn settle(app: &axum::Router, cookie: &str, key: &str) -> Value {
    let path = format!("/api/v1/topology/operations/{key}");
    for _ in 0..100 {
        let (_, op) = send(app, req("GET", &path, cookie, "", String::new())).await;
        if op["state"] == "done" || op["state"] == "failed" {
            return op;
        }
        tokio::time::sleep(std::time::Duration::from_millis(20)).await;
    }
    panic!("the operation did not finish");
}

#[tokio::test]
async fn an_authenticated_collect_job_answers_a_snapshot() {
    let (_dir, app, seen) = app(true).await;
    let (cookie, csrf) = sign_in(&app).await;
    let body = json!({"key": KEY, "job": {"kind": "collect", "router_id": "lq3-00000000000000a1",
        "expected_config_digest": "d1", "sources": ["wan"]}})
    .to_string();
    let jobs = "/api/v1/topology/source-jobs";
    let (status, _) = send(&app, req("POST", jobs, "", "", body.clone())).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED, "no session, no job");
    let (status, _) = send(&app, req("POST", jobs, &cookie, &csrf, body)).await;
    assert_eq!(status, StatusCode::ACCEPTED);
    let op = settle(&app, &cookie, KEY).await;
    assert_eq!(op["state"], json!("done"), "{op}");
    assert_eq!(op["source_result"]["kind"], json!("snapshot"));
    // The failed table stays failed: it is not an empty complete table.
    assert_eq!(op["source_result"]["tables"][0]["state"], json!("failed"));
    let calls: Vec<Value> = seen
        .lock()
        .unwrap()
        .iter()
        .filter(|v| v["op"] == "source_studio.collect")
        .cloned()
        .collect();
    assert_eq!(calls.len(), 1);
    assert_eq!(
        calls[0]["args"],
        json!({"router_id": "lq3-00000000000000a1", "expected_config_digest": "d1", "sources": ["wan"]})
    );
}

#[tokio::test]
async fn source_config_reads_the_cache_without_router_io() {
    let (dir, app, seen) = app(true).await;
    let view = json!({"version": 1, "router_id": "lq3-00000000000000a1", "config_digest": "d2",
        "loaded_config_digest": "d1", "policy_revision": null, "enabled": true,
        "access_profiles": [{"id": "default", "label": "Router credential", "credential_set": true,
                             "supported_writes": []}],
        "policies": [], "snapshots": [], "wan_candidates": []});
    let cache = json!({"version": 1, "updated": 1, "routers": [view]});
    std::fs::write(dir.path().join("source-studio.json"), cache.to_string()).unwrap();
    let path = "/api/v1/routers/lq3-00000000000000a1/source-config";
    let (status, _) = send(&app, req("GET", path, "", "", String::new())).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    let (cookie, _) = sign_in(&app).await;
    let before = seen.lock().unwrap().len();
    let (status, body) = send(&app, req("GET", path, &cookie, "", String::new())).await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(body, view);
    for other in ["lq3-00000000000000b2", "..%2Fetc"] {
        let path = format!("/api/v1/routers/{other}/source-config");
        let (status, _) = send(&app, req("GET", &path, &cookie, "", String::new())).await;
        assert_eq!(status, StatusCode::NOT_FOUND);
    }
    assert_eq!(
        seen.lock().unwrap().len(),
        before,
        "a GET sends nothing to the service"
    );
}

#[tokio::test]
async fn an_evaluate_job_keeps_a_candidate_that_only_its_actor_reads() {
    let (_dir, app, seen) = app(true).await;
    let (cookie, csrf) = sign_in(&app).await;
    let body = json!({"key": KEY, "job": {"kind": "evaluate", "router_id": "lq3-00000000000000a1",
        "expected_revision": "18446744073709551615", "expected_config_digest": "d1",
        "snapshot_id": "snap-1", "policies": []}})
    .to_string();
    let (status, _) = send(
        &app,
        req("POST", "/api/v1/topology/source-jobs", &cookie, &csrf, body),
    )
    .await;
    assert_eq!(status, StatusCode::ACCEPTED);
    let op = settle(&app, &cookie, KEY).await;
    assert_eq!(op["source_result"]["kind"], json!("candidate"), "{op}");
    assert_eq!(
        op["source_result"]["base_revision"],
        json!("18446744073709551615")
    );
    let sent: Vec<Value> = seen
        .lock()
        .unwrap()
        .iter()
        .filter(|v| v["op"] == "source_studio.evaluate")
        .cloned()
        .collect();
    assert_eq!(
        sent[0]["args"]["actor"],
        json!("operator-a"),
        "the session names the actor"
    );
    let base = "/api/v1/topology/source-candidates";
    let get = |path: String| req("GET", &path, &cookie, "", String::new());
    let (status, body) = send(&app, get(format!("{base}/cand-b1-1"))).await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(body["issues"]["blockers"], json!(1));
    let (status, body) = send(&app, get(format!("{base}/cand-b1-1/issues?limit=10"))).await;
    assert_eq!(
        (status, &body["total"]),
        (StatusCode::OK, &json!(1)),
        "{body}"
    );
    for (path, want, code) in [
        (
            format!("{base}/cand-b1-9"),
            StatusCode::NOT_FOUND,
            "not_found",
        ),
        (format!("{base}/cand-b1-2"), StatusCode::GONE, "expired"),
        (
            format!("{base}/cand-b1-1/records?limit=500"),
            StatusCode::BAD_REQUEST,
            "invalid",
        ),
        (
            format!("{base}/cand-b1-1/issues?cursor=x&extra=1"),
            StatusCode::BAD_REQUEST,
            "invalid",
        ),
        (
            format!("{base}/not-a-candidate"),
            StatusCode::NOT_FOUND,
            "not_found",
        ),
    ] {
        let (status, body) = send(&app, get(path.clone())).await;
        assert_eq!(
            (status, &body["code"]),
            (want, &json!(code)),
            "{path}: {body}"
        );
    }
    let (status, _) = send(
        &app,
        req("GET", &format!("{base}/cand-b1-1"), "", "", String::new()),
    )
    .await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn large_policy_uses_scoped_body_limit() {
    let (_dir, app, _) = app(true).await;
    let (cookie, csrf) = sign_in(&app).await;
    // About 12 KiB: a policy set of several policies.
    let names: Vec<String> = (0..1200).map(|i| format!("n{i:05}")).collect();
    let edit = json!({"key": "studio-save-0000000001", "expected_revision": 1, "edit": {
        "kind": "router.sources_set", "router_id": "lq3-00000000000000a1",
        "expected_config_digest": "d1", "policies": [], "padding": names}})
    .to_string();
    assert!(edit.len() > 8 * 1024 && edit.len() < 32 * 1024);
    let (status, _) = send(
        &app,
        req(
            "POST",
            "/api/v1/topology/edits",
            &cookie,
            &csrf,
            edit.clone(),
        ),
    )
    .await;
    assert_ne!(
        status,
        StatusCode::PAYLOAD_TOO_LARGE,
        "the edit route takes 32 KiB"
    );
    let (status, _) = send(
        &app,
        req("POST", "/api/v1/topology/reads", &cookie, &csrf, edit),
    )
    .await;
    assert_eq!(
        status,
        StatusCode::PAYLOAD_TOO_LARGE,
        "other routes keep 4 KiB"
    );
}

#[tokio::test]
async fn studio_posts_need_the_same_origin() {
    let (_dir, app, seen) = app(true).await;
    let (cookie, csrf) = sign_in(&app).await;
    let edit = json!({"key": "studio-save-0000000001", "expected_revision": 1, "edit": {
        "kind": "router.sources_set", "router_id": "lq3-00000000000000a1",
        "expected_config_digest": "d1", "policies": []}})
    .to_string();
    for (path, body) in [
        ("/api/v1/topology/source-jobs", job("studio")),
        ("/api/v1/topology/edits", edit),
    ] {
        let r = Request::builder()
            .method("POST")
            .uri(path)
            .header("cookie", &cookie)
            .header("origin", "https://other.test")
            .header("x-csrf-token", &csrf)
            .header("content-type", "application/json")
            .body(Body::from(body))
            .unwrap();
        let (status, _) = send(&app, r).await;
        assert_eq!(status, StatusCode::FORBIDDEN, "{path}");
    }
    assert!(
        seen.lock()
            .unwrap()
            .iter()
            .all(|v| v["op"] != "source_studio.test")
    );
}

#[tokio::test]
async fn throughput_joins_the_two_service_caches_without_a_service_call() {
    let (dir, app, seen) = app(true).await;
    let path = "/api/v1/topology/throughput";
    let (status, _) = send(&app, req("GET", path, "", "", String::new())).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    let (cookie, _) = sign_in(&app).await;
    let (status, _) = send(&app, req("GET", path, &cookie, "", String::new())).await;
    assert_eq!(status, StatusCode::NOT_FOUND, "no topology view yet");
    let now = web_gateway::broker::now();
    let node = |id: &str, parent: Option<&str>, card: &str| {
        json!({"id": id, "name": id, "parent": parent, "kind": "group", "source_id": null,
               "capacity": {"mode": "automatic"}, "alias": false, "calculated_mbps": null,
               "exported_mbps": null, "members": null, "card_id": card})
    };
    let view = json!({"version": 1, "updated": now, "fresh_for": 60, "snapshot_id": "s",
        "registry_revision": "2", "mode": "apply", "writer": "topology", "enrolled": true,
        "observed": {"revision": "2", "collected_at": now, "generation": 1, "state": "ok", "stale": false},
        "nodes": [node("wan-1", None, "wan-1"), node("ppp-1", Some("wan-1"), "source:ppp")],
        "sources": [], "plans": [], "observations": [], "collection": [], "conflicts": [],
        "subscribers": [{"id": "s1", "identities": [], "plan_id": null, "pin": null, "assignment": null,
            "active": {"node": "ppp-1", "ips": ["100.64.0.1"], "min_mbps": [1.0, 1.0], "max_mbps": [9.0, 9.0], "fallback": false}}]});
    let telemetry = json!({"version": 1, "updated": now, "stale": false, "reason": null,
        "hosts": {"alice": {"100.64.0.1": {"down_mbps": 12.5, "up_mbps": 2.0, "rtt_ms": 4.0}}}});
    std::fs::write(dir.path().join("topology.json"), view.to_string()).unwrap();
    std::fs::write(
        dir.path().join("subscriber-telemetry.json"),
        telemetry.to_string(),
    )
    .unwrap();
    let before = seen.lock().unwrap().len();
    let (status, body) = send(&app, req("GET", path, &cookie, "", String::new())).await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(
        (body["stale"].clone(), body["complete"].clone()),
        (json!(false), json!(true)),
        "{body}"
    );
    assert_eq!(
        body["flows"],
        json!([{"from": "wan-1", "to": "source:ppp", "down_bps": 12_500_000, "up_bps": 2_000_000}])
    );
    assert_eq!(seen.lock().unwrap().len(), before, "no service call");
}
