use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use http_body_util::BodyExt;
use lqosync_v3::{
    orchestrator, store,
    web::{App, Shared, routes},
};
use std::sync::Arc;
use tokio::sync::Mutex;
use tower::ServiceExt;
mod common;
#[path = "common/core.rs"]
mod core_fixture;
fn encode(s: &str) -> String {
    s.bytes()
        .map(|b| {
            if b.is_ascii_alphanumeric() || b"-_.~".contains(&b) {
                (b as char).to_string()
            } else {
                format!("%{b:02X}")
            }
        })
        .collect()
}
fn post(fields: &[(&str, &str)]) -> Request<Body> {
    Request::builder()
        .method("POST")
        .uri("/core-lists/save")
        .header("host", "localhost:9303")
        .header("content-type", "application/x-www-form-urlencoded")
        .body(Body::from(
            fields
                .iter()
                .map(|(k, v)| format!("{}={}", encode(k), encode(v)))
                .collect::<Vec<_>>()
                .join("&"),
        ))
        .unwrap()
}
async fn get(app: &axum::Router, path: &str) -> String {
    let r = app
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
    assert_eq!(r.status(), StatusCode::OK);
    String::from_utf8(r.into_body().collect().await.unwrap().to_bytes().to_vec()).unwrap()
}
fn revision(html: &str) -> String {
    html.split("name=\"revision\" value=\"")
        .nth(1)
        .expect("revision field")
        .split('"')
        .next()
        .unwrap()
        .into()
}
async fn fixture() -> (tempfile::TempDir, Shared, axum::Router, String) {
    let dir = tempfile::tempdir().unwrap();
    let (mut s, mut inv, _, _) = core_fixture::fixture();
    s.core_lists = Default::default();
    Arc::make_mut(inv.get_mut("r1").unwrap()).collected_at = orchestrator::now();
    store::save_settings(dir.path(), &s).unwrap();
    let mut a = App::open(dir.path().into()).unwrap();
    a.inventories = inv;
    let token = a.token.clone();
    let state = Arc::new(Mutex::new(a));
    orchestrator::refresh(state.clone()).await;
    let app = routes(state.clone());
    (dir, state, app, token)
}
#[tokio::test]
async fn ui_saves_bindings_auto_plans_and_protects_invalid_or_stale_edits() {
    let (dir, state, app, token) = fixture().await;
    let page = get(&app, "/core-lists").await;
    let rev = revision(&page);
    assert!(page.contains("Core address-list planner"));
    let fields = [
        ("token", token.as_str()),
        ("revision", rev.as_str()),
        ("action", "save"),
        ("wan", "wan"),
        ("router", "r1"),
        ("list", "lqos-wan"),
        ("table", "to-wan"),
    ];
    let mut bad = fields;
    bad[0].1 = "bad";
    assert_eq!(
        app.clone().oneshot(post(&bad)).await.unwrap().status(),
        StatusCode::FORBIDDEN
    );
    assert_eq!(
        app.clone().oneshot(post(&fields)).await.unwrap().status(),
        StatusCode::SEE_OTHER
    );
    let core = store::load_core_plan(dir.path()).unwrap().unwrap();
    assert_eq!(core.plan.entries.len(), 3);
    assert_eq!(state.lock().await.settings.core_lists.bindings.len(), 1);
    let pointer = std::fs::read(dir.path().join("state/current.toml")).unwrap();
    let settings = std::fs::read(dir.path().join("settings.toml")).unwrap();
    assert_eq!(
        app.clone().oneshot(post(&fields)).await.unwrap().status(),
        StatusCode::UNPROCESSABLE_ENTITY
    );
    let page = get(&app, "/core-lists").await;
    let rev = revision(&page);
    let mut bad = fields;
    bad[1].1 = &rev;
    bad[5].1 = "!negated";
    assert_eq!(
        app.clone().oneshot(post(&bad)).await.unwrap().status(),
        StatusCode::UNPROCESSABLE_ENTITY
    );
    assert_eq!(
        settings,
        std::fs::read(dir.path().join("settings.toml")).unwrap()
    );
    assert_eq!(
        pointer,
        std::fs::read(dir.path().join("state/current.toml")).unwrap()
    );
    {
        let mut a = state.lock().await;
        Arc::make_mut(a.inventories.get_mut("r1").unwrap())
            .tables
            .get_mut("/ip/route/print")
            .unwrap()[0]
            .insert("immediate-gw".into(), "1.1.1.1%wrong".into());
    }
    orchestrator::refresh(state.clone()).await;
    let body = get(&app, "/core-lists").await;
    assert!(body.contains("different interface"));
    assert!(body.contains(&core.generation));
    assert!(body.contains("Last committed local plan"));
    assert_eq!(
        pointer,
        std::fs::read(dir.path().join("state/current.toml")).unwrap()
    );
}
#[tokio::test]
async fn preview_is_paginated_search_is_encoded_and_markup_is_escaped() {
    use lqosync_v3::{assignment, core_plan, model, normalizer};
    use std::collections::BTreeMap;
    let (dir, state, app, _) = fixture().await;
    let (mut s, inv, _, _) = core_fixture::fixture();
    let mut subs = normalizer::normalize(&s, &inv, &BTreeMap::new(), 101).unwrap();
    let template = subs[0].clone();
    subs = (0..130)
        .map(|n| {
            let mut sub = template.clone();
            sub.id = uuid::Uuid::new_v5(
                &uuid::Uuid::NAMESPACE_OID,
                format!("subscriber-{n}").as_bytes(),
            )
            .to_string();
            sub.ipv4 = format!("192.0.2.{}", n + 1);
            sub
        })
        .collect();
    s.core_lists.bindings[0].list = "A&B +✓ <script>".into();
    let mut inv = inv;
    Arc::make_mut(inv.get_mut("r1").unwrap())
        .tables
        .get_mut("/ip/firewall/mangle/print")
        .unwrap()[0]
        .insert(
            "src-address-list".into(),
            s.core_lists.bindings[0].list.clone(),
        );
    let assignments = assignment::assign(
        &s,
        &mut subs,
        &Default::default(),
        &inv,
        &BTreeMap::new(),
        101,
    )
    .unwrap();
    Arc::make_mut(inv.get_mut("r1").unwrap()).collected_at = orchestrator::now();
    let core = core_plan::build(
        &s,
        &subs,
        &assignments,
        &inv,
        &BTreeMap::new(),
        orchestrator::now(),
    )
    .unwrap();
    lqosync_v3::minimums::allocate(&s, &mut subs).unwrap();
    let shape = model::compile(&s, &subs).unwrap();
    let p =
        store::publish_orchestrated(dir.path(), &shape, &subs, &assignments, Some(&core)).unwrap();
    {
        let mut a = state.lock().await;
        a.settings = s;
        a.generation = Some(p.generation);
    }
    let query = encode("A&B +✓");
    let page = get(&app, &format!("/core-lists?q={query}")).await;
    assert!(page.contains("130 match"));
    assert!(page.contains(&format!("/core-lists?q={query}&amp;offset=100")));
    assert!(!page.contains("<script>"));
    assert!(page.contains("&lt;script&gt;"));
    let page = get(
        &app,
        &format!("/core-lists?q={query}&offset={}", usize::MAX),
    )
    .await;
    assert!(page.contains(&format!("/core-lists?q={query}&amp;offset=0")));
}
