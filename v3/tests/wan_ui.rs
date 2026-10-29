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
fn post(path: &str, fields: &[(&str, &str)]) -> Request<Body> {
    Request::builder()
        .method("POST")
        .uri(path)
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
#[tokio::test]
async fn mapped_wans_pool_and_pins_auto_publish_consistent_parents_and_reject_invalid_edits() {
    let dir = tempfile::tempdir().unwrap();
    let (mut settings, mut inventories) = common::fixture();
    let mut wan = settings.nodes[0].clone();
    wan.id = "wan2".into();
    settings.nodes.push(wan);
    let mut target = settings.nodes[1].clone();
    target.id = "ppp2".into();
    target.parent = "wan2".into();
    target.router.clear();
    target.source.clear();
    target.mode = "computed".into();
    settings.nodes.push(target);
    let inv = Arc::make_mut(inventories.get_mut("r1").unwrap());
    inv.collected_at = orchestrator::now();
    inv.tables.insert(
        "/interface/print".into(),
        ["wan-a", "wan-b"]
            .map(|name| {
                [
                    ("name".into(), name.into()),
                    ("running".into(), "true".into()),
                    ("disabled".into(), "false".into()),
                ]
                .into_iter()
                .collect()
            })
            .to_vec(),
    );
    store::save_settings(dir.path(), &settings).unwrap();
    let mut state = App::open(dir.path().into()).unwrap();
    state.inventories = inventories;
    let token = state.token.clone();
    let state = Arc::new(Mutex::new(state));
    let app = routes(state.clone());
    for path in ["/wans", "/assignments"] {
        assert_eq!(
            app.clone()
                .oneshot(
                    Request::builder()
                        .uri(path)
                        .header("host", "localhost:9303")
                        .body(Body::empty())
                        .unwrap()
                )
                .await
                .unwrap()
                .status(),
            StatusCode::OK
        );
    }
    assert_eq!(
        app.clone()
            .oneshot(post(
                "/wans/save",
                &[
                    ("token", "bad"),
                    ("node", "wan"),
                    ("selection", r#"["r1","wan-a"]"#),
                    ("download", "1000"),
                    ("upload", "1000")
                ]
            ))
            .await
            .unwrap()
            .status(),
        StatusCode::FORBIDDEN
    );
    for (node, selection) in [("wan", r#"["r1","wan-a"]"#), ("wan2", r#"["r1","wan-b"]"#)] {
        assert_eq!(
            app.clone()
                .oneshot(post(
                    "/wans/save",
                    &[
                        ("token", &token),
                        ("node", node),
                        ("selection", selection),
                        ("download", "1000"),
                        ("upload", "1000")
                    ]
                ))
                .await
                .unwrap()
                .status(),
            StatusCode::SEE_OTHER
        );
    }
    let before = std::fs::read(dir.path().join("settings.toml")).unwrap();
    let pointer = std::fs::read(dir.path().join("state/current.toml")).unwrap();
    assert_eq!(
        app.clone()
            .oneshot(post(
                "/wans/save",
                &[
                    ("token", &token),
                    ("node", "wan2"),
                    ("selection", r#"["r1","wan-a"]"#),
                    ("download", "1000"),
                    ("upload", "1000")
                ]
            ))
            .await
            .unwrap()
            .status(),
        StatusCode::UNPROCESSABLE_ENTITY
    );
    assert_eq!(
        before,
        std::fs::read(dir.path().join("settings.toml")).unwrap()
    );
    assert_eq!(
        pointer,
        std::fs::read(dir.path().join("state/current.toml")).unwrap()
    );
    assert_eq!(
        app.clone()
            .oneshot(post(
                "/assignments/pool",
                &[
                    ("token", &token),
                    ("source", "ppp"),
                    ("target", "ppp2"),
                    ("action", "add")
                ]
            ))
            .await
            .unwrap()
            .status(),
        StatusCode::SEE_OTHER
    );
    let snapshot = store::load_snapshot(dir.path()).unwrap();
    let alice = snapshot.iter().find(|s| s.name == "alice").unwrap().clone();
    assert_eq!(
        app.clone()
            .oneshot(post(
                "/assignments/pin",
                &[("token", &token), ("id", &alice.id), ("target", "ppp2")]
            ))
            .await
            .unwrap()
            .status(),
        StatusCode::SEE_OTHER
    );
    let (snapshot, assignments) = store::load_assignment_snapshot(dir.path()).unwrap();
    let moved = snapshot.iter().find(|s| s.id == alice.id).unwrap();
    assert_eq!(moved.node, "ppp2");
    assert_eq!(moved.download, alice.download);
    assert_eq!(assignments.assignments[&alice.id].source_node, "ppp");
    let pointer = std::fs::read(dir.path().join("state/current.toml")).unwrap();
    assert_eq!(
        app.clone()
            .oneshot(post(
                "/assignments/pool",
                &[("token", &token), ("source", "ppp"), ("action", "delete")]
            ))
            .await
            .unwrap()
            .status(),
        StatusCode::UNPROCESSABLE_ENTITY
    );
    assert_eq!(
        pointer,
        std::fs::read(dir.path().join("state/current.toml")).unwrap()
    );
    assert_eq!(
        app.clone()
            .oneshot(post(
                "/assignments/pin",
                &[("token", &token), ("id", &alice.id), ("target", "dhcp")]
            ))
            .await
            .unwrap()
            .status(),
        StatusCode::UNPROCESSABLE_ENTITY
    );
    assert_eq!(
        pointer,
        std::fs::read(dir.path().join("state/current.toml")).unwrap()
    );
    assert_eq!(
        app.clone()
            .oneshot(post(
                "/assignments/pin",
                &[("token", &token), ("id", &alice.id), ("target", "")]
            ))
            .await
            .unwrap()
            .status(),
        StatusCode::SEE_OTHER
    );
    assert!(state.lock().await.settings.assignment.pins.is_empty());
    assert_eq!(
        store::load_snapshot(dir.path())
            .unwrap()
            .iter()
            .find(|s| s.id == alice.id)
            .unwrap()
            .node,
        "ppp2"
    );
    let response = app
        .oneshot(
            Request::builder()
                .uri("/assignments")
                .header("host", "localhost:9303")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
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
    assert!(body.contains("ppp2"));
    assert!(body.contains("alice"));
}

#[tokio::test]
async fn assignment_pagination_preserves_encoded_search_and_bounds_large_offsets() {
    use lqosync_v3::model::{Settings, Subscriber};
    let dir = tempfile::tempdir().unwrap();
    let settings: Settings = toml::from_str(include_str!("../examples/settings.toml")).unwrap();
    store::save_settings(dir.path(), &settings).unwrap();
    let subscribers: Vec<_> = (0..130)
        .map(|i| Subscriber {
            id: format!("user-{i:03}"),
            name: if i < 120 {
                format!("A&B +✓ {i:03}")
            } else {
                "unrelated".into()
            },
            node: "ppp50".into(),
            ipv4: format!("192.0.2.{}", i + 1),
            download: 10.,
            upload: 5.,
            download_min: 1.,
            upload_min: 1.,
        })
        .collect();
    let plan = lqosync_v3::model::compile(&settings, &subscribers).unwrap();
    store::publish_normalized(dir.path(), &plan, &subscribers).unwrap();
    let app = routes(Arc::new(Mutex::new(App::open(dir.path().into()).unwrap())));
    let query = encode("A&B +✓");
    for (offset, expected, link) in [(0, "user-000", 100), (usize::MAX, "user-100", 0)] {
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri(format!("/assignments?q={query}&offset={offset}"))
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
        assert!(body.contains(expected));
        assert!(!body.contains("unrelated"));
        assert!(
            body.contains(&format!("/assignments?q={query}&amp;offset={link}")),
            "search missing from pagination"
        );
    }
}
