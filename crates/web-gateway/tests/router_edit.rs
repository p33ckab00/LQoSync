use axum::{body::Body, http::Request, http::StatusCode};
use http_body_util::BodyExt;
use std::os::unix::fs::PermissionsExt;
use tower::ServiceExt;
use web_gateway::{
    accounts::Accounts,
    broker,
    http::{WebState, router},
    sessions::Sessions,
};

const ORIGIN: &str = "https://lqosync.test";

/// A fake core command: it records the request and gives one fixed answer.
fn editor(dir: &std::path::Path, answer: &str) -> broker::Editor {
    let bin = dir.join("editor.sh");
    std::fs::write(
        &bin,
        format!(
            "#!/bin/sh\ncat > '{}'\nprintf '%s\\n' '{answer}'\n",
            dir.join("request.json").display()
        ),
    )
    .unwrap();
    std::fs::set_permissions(&bin, std::fs::Permissions::from_mode(0o700)).unwrap();
    broker::Editor {
        bin,
        config: dir.join("unused.toml"),
    }
}

async fn app(answer: &str) -> (tempfile::TempDir, axum::Router) {
    let temp = tempfile::tempdir().unwrap();
    std::fs::set_permissions(temp.path(), std::fs::Permissions::from_mode(0o700)).unwrap();
    let accounts = Accounts::new(temp.path().join("accounts.db")).unwrap();
    accounts
        .create("operator-a", "test-password-only-123")
        .unwrap();
    let path = temp.path().join("broker.sock");
    let listener = tokio::net::UnixListener::bind(&path).unwrap();
    let state = temp.path().to_path_buf();
    let fake = editor(temp.path(), answer);
    tokio::spawn(async move {
        let _ = broker::serve(
            listener,
            unsafe { libc::geteuid() },
            accounts,
            state,
            None,
            Some(fake),
        )
        .await;
    });
    let app = router(
        WebState {
            broker: path,
            sessions: Sessions::new(temp.path().join("sessions.db")).unwrap(),
            origin: ORIGIN.into(),
            secure: true,
            index: std::sync::Arc::new("<html></html>".into()),
        },
        temp.path().into(),
    );
    (temp, app)
}

fn req(method: &str, path: &str, cookie: &str, csrf: &str, body: &str) -> Request<Body> {
    Request::builder()
        .method(method)
        .uri(path)
        .header("cookie", cookie)
        .header("origin", ORIGIN)
        .header("x-csrf-token", csrf)
        .header("content-type", "application/json")
        .body(Body::from(body.to_owned()))
        .unwrap()
}

async fn sign_in(app: &axum::Router) -> (String, String) {
    let r = app
        .clone()
        .oneshot(req(
            "POST",
            "/api/v1/session",
            "",
            "",
            r#"{"username":"operator-a","password":"test-password-only-123"}"#,
        ))
        .await
        .unwrap();
    let cookie = r.headers()["set-cookie"].to_str().unwrap().to_owned();
    let json: serde_json::Value =
        serde_json::from_slice(&r.into_body().collect().await.unwrap().to_bytes()).unwrap();
    (cookie, json["csrf"].as_str().unwrap().to_owned())
}

#[tokio::test]
async fn edits_need_csrf_use_the_session_actor_and_map_core_codes() {
    let ok = r#"{"code":"ok","message":"","reply":{"router_id":"lq3-0000000000000001","digest":"d2","reload":"sent","warnings":[]}}"#;
    let (dir, app) = app(ok).await;
    let (cookie, csrf) = sign_in(&app).await;
    let create = r#"{"edit":{"expected_digest":"d1","name":"Core-01","address":"192.0.2.9","username":"api","password":"x"},"current_password":null}"#;
    // No CSRF token: refused before the core command runs.
    let r = app
        .clone()
        .oneshot(req("POST", "/api/v1/routers", &cookie, "wrong", create))
        .await
        .unwrap();
    assert_eq!(r.status(), StatusCode::FORBIDDEN);
    assert!(!dir.path().join("request.json").exists());
    // The browser cannot choose the actor: an unknown field is rejected.
    let forged = r#"{"edit":{"expected_digest":"d1"},"current_password":null,"actor":"root"}"#;
    let r = app
        .clone()
        .oneshot(req("POST", "/api/v1/routers", &cookie, &csrf, forged))
        .await
        .unwrap();
    assert!(r.status().is_client_error());
    let r = app
        .clone()
        .oneshot(req("POST", "/api/v1/routers", &cookie, &csrf, create))
        .await
        .unwrap();
    assert_eq!(r.status(), StatusCode::OK);
    let sent: web_contract::routers::RouterEditRequest =
        serde_json::from_slice(&std::fs::read(dir.path().join("request.json")).unwrap()).unwrap();
    assert_eq!(sent.actor, "operator-a");
    assert_eq!(sent.router_id, None);
    assert_eq!(sent.edit.name.as_deref(), Some("Core-01"));
    // An unsafe router ID never reaches the core command.
    let r = app
        .clone()
        .oneshot(req(
            "PATCH",
            "/api/v1/routers/..%2Fetc",
            &cookie,
            &csrf,
            create,
        ))
        .await
        .unwrap();
    assert_eq!(r.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn a_credential_replacement_needs_the_current_password() {
    let ok = r#"{"code":"ok","message":"","reply":{"router_id":"lq3-0000000000000001","digest":"d2","reload":"sent","warnings":[]}}"#;
    let (dir, app) = app(ok).await;
    let (cookie, csrf) = sign_in(&app).await;
    let path = "/api/v1/routers/lq3-0000000000000001";
    let replace = |pw: &str| {
        format!(r#"{{"edit":{{"expected_digest":"d1","password":"new"}},"current_password":{pw}}}"#)
    };
    for pw in ["null", "\"wrong-password\""] {
        let r = app
            .clone()
            .oneshot(req("PATCH", path, &cookie, &csrf, &replace(pw)))
            .await
            .unwrap();
        assert_eq!(r.status(), StatusCode::FORBIDDEN);
        let out: web_contract::routers::RouterEditOutcome =
            serde_json::from_slice(&r.into_body().collect().await.unwrap().to_bytes()).unwrap();
        assert_eq!(out.code, "reauth");
        assert!(
            !dir.path().join("request.json").exists(),
            "the core command did not run"
        );
    }
    let r = app
        .clone()
        .oneshot(req(
            "PATCH",
            path,
            &cookie,
            &csrf,
            &replace("\"test-password-only-123\""),
        ))
        .await
        .unwrap();
    assert_eq!(r.status(), StatusCode::OK);
    // A disable needs the password too; an enable does not.
    std::fs::remove_file(dir.path().join("request.json")).unwrap();
    let disable = |pw: &str| {
        format!(r#"{{"edit":{{"expected_digest":"d1","enabled":false}},"current_password":{pw}}}"#)
    };
    let r = app
        .clone()
        .oneshot(req("PATCH", path, &cookie, &csrf, &disable("null")))
        .await
        .unwrap();
    assert_eq!(r.status(), StatusCode::FORBIDDEN);
    assert!(
        !dir.path().join("request.json").exists(),
        "no disable without the password"
    );
    let r = app
        .clone()
        .oneshot(req(
            "PATCH",
            path,
            &cookie,
            &csrf,
            &disable("\"test-password-only-123\""),
        ))
        .await
        .unwrap();
    assert_eq!(r.status(), StatusCode::OK);
    let r = app
        .clone()
        .oneshot(req(
            "PATCH",
            path,
            &cookie,
            &csrf,
            r#"{"edit":{"expected_digest":"d1","enabled":true},"current_password":null}"#,
        ))
        .await
        .unwrap();
    assert_eq!(r.status(), StatusCode::OK);
    // An edit without a new credential needs no password.
    let r = app
        .clone()
        .oneshot(req(
            "PATCH",
            path,
            &cookie,
            &csrf,
            r#"{"edit":{"expected_digest":"d1","port":8728},"current_password":null}"#,
        ))
        .await
        .unwrap();
    assert_eq!(r.status(), StatusCode::OK);
}

#[tokio::test]
async fn core_conflicts_and_activity_reach_the_browser() {
    let conflict = r#"{"code":"conflict","message":"The configuration changed after this page loaded.","reply":null}"#;
    let (dir, app) = app(conflict).await;
    let (cookie, csrf) = sign_in(&app).await;
    let r = app
        .clone()
        .oneshot(req(
            "PATCH",
            "/api/v1/routers/lq3-0000000000000001",
            &cookie,
            &csrf,
            r#"{"edit":{"expected_digest":"old","port":8728},"current_password":null}"#,
        ))
        .await
        .unwrap();
    assert_eq!(r.status(), StatusCode::CONFLICT);
    let event = |id: &str, at: u64| {
        format!(
            r#"{{"at":{at},"actor":"operator-a","router_id":"{id}","router":"R","action":"update","changes":["port"],"digest":"d"}}"#
        )
    };
    std::fs::write(
        dir.path().join("router-audit.jsonl"),
        format!(
            "{}\n{}\nnot json\n{}\n",
            event("lq3-0000000000000001", 1),
            event("lq3-0000000000000002", 2),
            event("lq3-0000000000000001", 3)
        ),
    )
    .unwrap();
    let r = app
        .clone()
        .oneshot(req(
            "GET",
            "/api/v1/routers/lq3-0000000000000001/activity",
            &cookie,
            "",
            "",
        ))
        .await
        .unwrap();
    assert_eq!(r.status(), StatusCode::OK);
    let a: web_contract::routers::RouterActivity =
        serde_json::from_slice(&r.into_body().collect().await.unwrap().to_bytes()).unwrap();
    let at: Vec<u64> = a.events.iter().map(|e| e.at).collect();
    assert_eq!(
        at,
        [3, 1],
        "only this router, newest first, bad lines skipped"
    );
}

#[tokio::test]
async fn a_connection_test_goes_to_the_service_socket_and_sync_now_runs_the_command() {
    let ok = r#"{"code":"ok","message":"sent","reply":null}"#;
    let (dir, app) = app(ok).await;
    let (cookie, csrf) = sign_in(&app).await;
    let path = "/api/v1/routers/lq3-0000000000000001/test";
    // No service socket: the test answers with the class `service`, not an error page.
    let r = app
        .clone()
        .oneshot(req("POST", path, &cookie, &csrf, ""))
        .await
        .unwrap();
    assert_eq!(r.status(), StatusCode::OK);
    let t: web_contract::routers::RouterTestResult =
        serde_json::from_slice(&r.into_body().collect().await.unwrap().to_bytes()).unwrap();
    assert_eq!(t.class, "service");
    // A fake service: it checks the request and answers one test.
    let listener = tokio::net::UnixListener::bind(dir.path().join("api.sock")).unwrap();
    let service = tokio::spawn(async move {
        use tokio::io::{AsyncBufReadExt, AsyncWriteExt};
        let (stream, _) = listener.accept().await.unwrap();
        let (read, mut write) = stream.into_split();
        let mut line = String::new();
        tokio::io::BufReader::new(read)
            .read_line(&mut line)
            .await
            .unwrap();
        let v: serde_json::Value = serde_json::from_str(&line).unwrap();
        assert_eq!(v["op"], "router.test");
        assert_eq!(
            v["args"]["actor"], "operator-a",
            "the actor comes from the session"
        );
        let answer = serde_json::json!({"v":1,"id":v["id"],"ok":true,"result":{
            "router_id":"lq3-0000000000000001","class":"ok","identity":"edge-r1","elapsed_ms":12,"at":1}});
        write
            .write_all(format!("{answer}\n").as_bytes())
            .await
            .unwrap();
    });
    let r = app
        .clone()
        .oneshot(req("POST", path, &cookie, "wrong", ""))
        .await
        .unwrap();
    assert_eq!(
        r.status(),
        StatusCode::FORBIDDEN,
        "a test needs the CSRF token"
    );
    let r = app
        .clone()
        .oneshot(req("POST", path, &cookie, &csrf, ""))
        .await
        .unwrap();
    let t: web_contract::routers::RouterTestResult =
        serde_json::from_slice(&r.into_body().collect().await.unwrap().to_bytes()).unwrap();
    assert_eq!(
        (t.class.as_str(), t.identity.as_deref()),
        ("ok", Some("edge-r1"))
    );
    service.await.unwrap();
    let r = app
        .clone()
        .oneshot(req("POST", "/api/v1/routers/sync", &cookie, &csrf, ""))
        .await
        .unwrap();
    assert_eq!(r.status(), StatusCode::OK);
}
