use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use http_body_util::BodyExt;
use lqosync_v3::{
    orchestrator, store,
    web::{App, routes},
};
use std::sync::Arc;
use tokio::sync::Mutex;
use tower::ServiceExt;
mod common;
fn encode(raw: &str) -> String {
    raw.bytes()
        .map(|b| {
            if b.is_ascii_alphanumeric() || b"-_.~".contains(&b) {
                (b as char).to_string()
            } else {
                format!("%{b:02X}")
            }
        })
        .collect()
}
fn request(token: &str, node: &str, source: &str) -> Request<Body> {
    Request::builder()
        .method("POST")
        .uri("/sources/save")
        .header("host", "localhost:9303")
        .header("content-type", "application/x-www-form-urlencoded")
        .body(Body::from(format!(
            "token={}&node={}&selection={}",
            encode(token),
            encode(node),
            encode(source)
        )))
        .unwrap()
}
#[tokio::test]
async fn discovered_source_form_checks_token_persists_and_publishes_without_an_apply_button() {
    let dir = tempfile::tempdir().unwrap();
    let (mut settings, mut inventories) = common::fixture();
    for n in &mut settings.nodes {
        n.source.clear();
    }
    for i in inventories.values_mut() {
        Arc::make_mut(i).collected_at = orchestrator::now();
    }
    store::save_settings(dir.path(), &settings).unwrap();
    let mut state = App::open(dir.path().into()).unwrap();
    state.inventories = inventories;
    let token = state.token.clone();
    let state = Arc::new(Mutex::new(state));
    let app = routes(state.clone());
    let selected = r#"["r1","Plan"]"#;
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/sources")
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
    assert!(body.contains("Router · Plan"));
    let before = std::fs::read(dir.path().join("settings.toml")).unwrap();
    for (auth, selection, status) in [
        ("invalid", selected, StatusCode::FORBIDDEN),
        (
            token.as_str(),
            r#"["r1","Missing"]"#,
            StatusCode::UNPROCESSABLE_ENTITY,
        ),
    ] {
        assert_eq!(
            app.clone()
                .oneshot(request(auth, "ppp", selection))
                .await
                .unwrap()
                .status(),
            status
        );
        assert_eq!(
            before,
            std::fs::read(dir.path().join("settings.toml")).unwrap()
        );
        assert!(!dir.path().join("state/current.toml").exists());
    }
    assert_eq!(
        app.clone()
            .oneshot(request(&token, "ppp", selected))
            .await
            .unwrap()
            .status(),
        StatusCode::SEE_OTHER
    );
    assert_eq!(
        store::load_settings(dir.path()).unwrap().nodes[1].source,
        "Plan"
    );
    let subscribers = store::load_snapshot(dir.path()).unwrap();
    assert_eq!(subscribers.len(), 1);
    assert_eq!(subscribers[0].node, "ppp");
    let pointer = std::fs::read(dir.path().join("state/current.toml")).unwrap();
    let mut duplicate = state.lock().await.settings.nodes[1].clone();
    duplicate.id = "duplicate".into();
    duplicate.source.clear();
    state.lock().await.settings.nodes.push(duplicate);
    assert_eq!(
        app.oneshot(request(&token, "duplicate", selected))
            .await
            .unwrap()
            .status(),
        StatusCode::UNPROCESSABLE_ENTITY
    );
    assert_eq!(
        pointer,
        std::fs::read(dir.path().join("state/current.toml")).unwrap()
    );
}
