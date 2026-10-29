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
async fn page(app: &axum::Router) -> String {
    let response = app
        .clone()
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
fn revision(html: &str) -> String {
    html.split("name=\"revision\" value=\"")
        .nth(1)
        .unwrap()
        .split('"')
        .next()
        .unwrap()
        .into()
}
async fn post(app: &axum::Router, path: &str, form: String) -> StatusCode {
    app.clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(path)
                .header("host", "localhost:9303")
                .header("content-type", "application/x-www-form-urlencoded")
                .body(Body::from(form))
                .unwrap(),
        )
        .await
        .unwrap()
        .status()
}
#[tokio::test]
async fn per_server_form_persists_rates_auto_refreshes_and_preserves_them_on_global_save() {
    let dir = tempfile::tempdir().unwrap();
    let (s, mut inv) = common::fixture();
    for i in inv.values_mut() {
        Arc::make_mut(i).collected_at = orchestrator::now();
    }
    store::save_settings(dir.path(), &s).unwrap();
    let mut a = App::open(dir.path().into()).unwrap();
    a.inventories = inv;
    let token = a.token.clone();
    let state = Arc::new(Mutex::new(a));
    let app = routes(state.clone());
    let html = page(&app).await;
    assert!(html.contains("DHCP server speeds"));
    let rev = revision(&html);
    let fields = format!(
        "revision={rev}&selection=%5B%22r1%22%2C%22dhcp1%22%5D&mode=manual&download=35&upload=7&action=save"
    );
    assert_eq!(
        post(&app, "/speed-rules/dhcp", format!("token=bad&{fields}")).await,
        StatusCode::FORBIDDEN
    );
    assert_eq!(
        post(&app, "/speed-rules/dhcp", format!("token={token}&{fields}")).await,
        StatusCode::SEE_OTHER
    );
    let saved = store::load_settings(dir.path()).unwrap();
    assert_eq!(saved.speed_rules.dhcp_servers.len(), 1);
    assert_eq!(saved.speed_rules.dhcp_servers[0].download, Some(35.));
    let snapshot = store::load_snapshot(dir.path()).unwrap();
    let sub = snapshot.iter().find(|s| s.node == "dhcp").unwrap();
    assert_eq!((sub.download, sub.upload), (35., 7.));
    assert_eq!(
        post(&app, "/speed-rules/dhcp", format!("token={token}&{fields}")).await,
        StatusCode::UNPROCESSABLE_ENTITY
    );
    assert_eq!(post(&app,"/speed-rules/save",format!("token={token}&min_rate_fraction=0.25&min_floor_mbps=0.064&profile_name_fallback=true")).await,StatusCode::SEE_OTHER);
    assert_eq!(
        store::load_settings(dir.path())
            .unwrap()
            .speed_rules
            .dhcp_servers,
        saved.speed_rules.dhcp_servers
    );
    let before = std::fs::read(dir.path().join("state/current.toml")).unwrap();
    let rev = revision(&page(&app).await);
    let fields = format!(
        "token={token}&revision={rev}&selection=%5B%22r1%22%2C%22dhcp1%22%5D&mode=manual&download=0&upload=7&action=save"
    );
    assert_eq!(
        post(&app, "/speed-rules/dhcp", fields).await,
        StatusCode::UNPROCESSABLE_ENTITY
    );
    assert_eq!(
        std::fs::read(dir.path().join("state/current.toml")).unwrap(),
        before
    );
    let fields = format!(
        "token={token}&revision={rev}&selection=%5B%22r1%22%2C%22dhcp1%22%5D&mode=auto&download=&upload=&action=remove"
    );
    assert_eq!(
        post(&app, "/speed-rules/dhcp", fields).await,
        StatusCode::SEE_OTHER
    );
    assert!(
        store::load_settings(dir.path())
            .unwrap()
            .speed_rules
            .dhcp_servers
            .is_empty()
    );
    let pointer = std::fs::read(dir.path().join("state/current.toml")).unwrap();
    {
        let mut a = state.lock().await;
        Arc::make_mut(a.inventories.get_mut("r1").unwrap())
            .tables
            .get_mut("/ip/dhcp-server/print")
            .unwrap()[0]
            .insert("comment".into(), "25M/bad".into());
    }
    orchestrator::refresh(state.clone()).await;
    assert_eq!(
        std::fs::read(dir.path().join("state/current.toml")).unwrap(),
        pointer
    );
}
