use lqosync_v3::model::{self, Node, Settings, Subscriber};
mod common;
fn node(id: &str, parent: &str, download: f64, upload: f64) -> Node {
    Node {
        id: id.into(),
        name: id.into(),
        parent: parent.into(),
        kind: if parent.is_empty() { "wan" } else { "pppoe" }.into(),
        router: String::new(),
        source: String::new(),
        mode: "manual".into(),
        download,
        upload,
        download_factor: 1.0,
        upload_factor: 1.0,
    }
}
fn settings() -> Settings {
    Settings {
        nodes: vec![
            node("wan", "", 320.0, 320.0),
            node("ppp", "wan", 320.0, 320.0),
        ],
        ..Default::default()
    }
}
fn subscriber(id: usize, minimum: f64) -> Subscriber {
    Subscriber {
        id: id.to_string(),
        name: format!("Test {id}"),
        ipv4: format!("198.18.{}.{}", id / 250, id % 250 + 1),
        node: "ppp".into(),
        download: 400.0,
        upload: 400.0,
        download_min: minimum,
        upload_min: minimum,
    }
}
#[test]
fn compiler_rejects_aggregate_minimums_above_any_ancestor() {
    let s = settings();
    let subs = vec![subscriber(1, 162.5), subscriber(2, 162.5)];
    assert!(
        model::compile(&s, &subs).is_err(),
        "325 Mbps of minimums exceeded the 320 Mbps ancestor"
    );
}
#[test]
fn overloaded_minimums_fit_both_directions_without_changing_maxima_or_identity() {
    let s = settings();
    let mut subs = vec![subscriber(1, 162.5), subscriber(2, 162.5)];
    let before = subs.clone();
    let report = lqosync_v3::minimums::allocate(&s, &mut subs).unwrap();
    assert_eq!(report.adjusted_subscribers, 2);
    for (old, new) in before.iter().zip(&subs) {
        assert_eq!(
            (&old.id, &old.name, &old.ipv4, &old.node),
            (&new.id, &new.name, &new.ipv4, &new.node)
        );
        assert_eq!((old.download, old.upload), (new.download, new.upload));
        assert!(
            new.download_min >= s.speed_rules.min_floor_mbps
                && new.upload_min >= s.speed_rules.min_floor_mbps
        );
    }
    assert!((subs.iter().map(|s| s.download_min).sum::<f64>() - 320.0).abs() < 1e-7);
    assert!(model::compile(&s, &subs).is_ok());
    let once = subs.clone();
    let again = lqosync_v3::minimums::allocate(&s, &mut subs).unwrap();
    assert_eq!(again.adjusted_subscribers, 0);
    assert_eq!(subs, once);
}
#[test]
fn nested_asymmetric_caps_and_proportional_headroom_are_respected() {
    let mut s = settings();
    s.nodes[0].download = 100.;
    s.nodes[0].upload = 80.;
    s.nodes[1].download = 20.;
    s.nodes[1].upload = 10.;
    s.speed_rules.min_floor_mbps = 1.;
    let mut subs = vec![subscriber(1, 60.), subscriber(2, 40.)];
    lqosync_v3::minimums::allocate(&s, &mut subs).unwrap();
    assert!((subs.iter().map(|s| s.download_min).sum::<f64>() - 20.).abs() < 1e-7);
    assert!((subs.iter().map(|s| s.upload_min).sum::<f64>() - 10.).abs() < 1e-7);
    assert!(((subs[0].download_min - 1.) / (subs[1].download_min - 1.) - 59. / 39.).abs() < 1e-7);
    assert!(model::compile(&s, &subs).is_ok());
}
#[test]
fn exhausted_floors_fail_without_partial_input_mutation() {
    let mut s = settings();
    s.nodes[0].upload = 0.1;
    s.nodes[1].upload = 0.1;
    let mut subs = vec![subscriber(1, 162.5), subscriber(2, 162.5)];
    let before = subs.clone();
    let err = lqosync_v3::minimums::allocate(&s, &mut subs)
        .unwrap_err()
        .to_string();
    assert!(err.contains("floor"));
    assert_eq!(subs, before);
}
#[test]
fn floors_at_the_exact_budget_do_not_fail_due_to_float_sum_rounding() {
    let mut s = settings();
    for node in &mut s.nodes {
        node.download = 640.;
        node.upload = 640.;
    }
    let mut subs: Vec<_> = (0..5000).map(|i| subscriber(i, 0.128)).collect();
    lqosync_v3::minimums::allocate(&s, &mut subs).unwrap();
    assert!(model::compile(&s, &subs).is_ok());
}
#[test]
fn tiny_positive_capacities_do_not_receive_an_absolute_tolerance_bypass() {
    let mut s = settings();
    for node in &mut s.nodes {
        node.download = 1e-12;
        node.upload = 1e-12;
    }
    let mut subs = vec![subscriber(1, 0.)];
    subs[0].download = 1e-10;
    subs[0].upload = 1e-10;
    assert!(lqosync_v3::minimums::allocate(&s, &mut subs).is_err());
}
#[test]
fn allocation_is_deterministic_and_floors_never_exceed_a_subscriber_maximum() {
    let s = settings();
    let mut a = vec![subscriber(1, 300.), subscriber(2, 25.)];
    let mut b = a.iter().rev().cloned().collect::<Vec<_>>();
    lqosync_v3::minimums::allocate(&s, &mut a).unwrap();
    lqosync_v3::minimums::allocate(&s, &mut b).unwrap();
    b.reverse();
    assert_eq!(a, b);
    let mut low = vec![subscriber(3, 0.)];
    low[0].download = 0.05;
    low[0].upload = 0.02;
    lqosync_v3::minimums::allocate(&s, &mut low).unwrap();
    assert_eq!(low[0].download_min, 0.05);
    assert_eq!(low[0].upload_min, 0.02);
}
#[tokio::test]
async fn automatic_publication_and_validation_share_effective_minimums_and_retain_on_floor_failure()
{
    use lqosync_v3::{core_diagnostics::Origin, orchestrator, store, validation, web::App};
    use std::{collections::BTreeMap, sync::Arc};
    let dir = tempfile::tempdir().unwrap();
    let (mut s, mut inv) = common::fixture();
    for node in &mut s.nodes {
        node.download = 30.;
        node.upload = 10.;
    }
    for i in inv.values_mut() {
        Arc::make_mut(i).collected_at = orchestrator::now();
    }
    store::save_settings(dir.path(), &s).unwrap();
    let mut app = App::open(dir.path().into()).unwrap();
    app.inventories = inv.clone();
    let shared = Arc::new(tokio::sync::Mutex::new(app));
    orchestrator::refresh(shared.clone()).await;
    let snapshot = store::load_snapshot(dir.path()).unwrap();
    assert_eq!(snapshot.len(), 3);
    let shaping = model::compile(&s, &snapshot).unwrap();
    assert!((snapshot.iter().map(|s| s.download_min).sum::<f64>() - 30.).abs() < 1e-7);
    let generation = store::current_generation(dir.path()).unwrap();
    assert_eq!(
        std::fs::read_to_string(
            dir.path()
                .join("staging")
                .join(&generation)
                .join("ShapedDevices.csv")
        )
        .unwrap(),
        shaping.csv
    );
    let report = validation::review(
        &s,
        &inv,
        &BTreeMap::new(),
        dir.path(),
        orchestrator::now(),
        Origin::Live,
    );
    assert!(report.checks.iter().any(|c| c.code == "subscriber-minimums"
        && c.status == lqosync_v3::core_diagnostics::Outcome::Pass));
    for code in ["snapshot-consistency", "ShapedDevices.csv", "network.json"] {
        assert!(
            report
                .checks
                .iter()
                .any(|c| c.code == code && c.status == lqosync_v3::core_diagnostics::Outcome::Pass),
            "Effective output mismatch: {code}"
        );
    }
    shared.lock().await.settings.speed_rules.min_floor_mbps = 20.;
    orchestrator::refresh(shared.clone()).await;
    assert_eq!(store::current_generation(dir.path()).unwrap(), generation);
    assert!(shared.lock().await.status.contains("floor"));
}
#[test]
#[ignore = "manual performance workload"]
fn allocate_50k_subscribers_without_repeated_global_scans() {
    let mut s = settings();
    for n in &mut s.nodes {
        n.download = 10000.;
        n.upload = 10000.;
    }
    let mut subs: Vec<_> = (0..50000)
        .map(|i| {
            let mut x = subscriber(i, 0.5);
            x.ipv4 = format!("198.{}.{}.{}", 18 + i / 65536, (i / 256) % 256, i % 256);
            x.download = 1.;
            x.upload = 0.5;
            x.upload_min = 0.25;
            x
        })
        .collect();
    let start = std::time::Instant::now();
    let report = lqosync_v3::minimums::allocate(&s, &mut subs).unwrap();
    eprintln!("50k minimum allocation: {:?}", start.elapsed());
    assert_eq!(report.adjusted_subscribers, 50000);
    assert!(model::compile(&s, &subs).is_ok());
}
#[tokio::test]
async fn source_managed_topology_edits_validate_an_allocated_copy_then_publish_effective_minimums()
{
    use axum::{body::Body, http::Request};
    use lqosync_v3::{
        orchestrator, store,
        web::{App, routes},
    };
    use std::sync::Arc;
    use tower::ServiceExt;
    let dir = tempfile::tempdir().unwrap();
    let (s, mut inv) = common::fixture();
    for i in inv.values_mut() {
        Arc::make_mut(i).collected_at = orchestrator::now();
    }
    store::save_settings(dir.path(), &s).unwrap();
    let mut a = App::open(dir.path().into()).unwrap();
    a.inventories = inv;
    let token = a.token.clone();
    let shared = Arc::new(tokio::sync::Mutex::new(a));
    orchestrator::refresh(shared.clone()).await;
    let old = store::current_generation(dir.path()).unwrap();
    let app = routes(shared.clone());
    let body = format!(
        "token={token}&original_id=ppp&id=ppp&name=PPP&parent=wan&kind=pppoe&router=r1&source=Plan&mode=manual&download=10&upload=10&download_factor=1&upload_factor=1"
    );
    let response = app
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
    assert_eq!(response.status(), axum::http::StatusCode::SEE_OTHER);
    assert_ne!(store::current_generation(dir.path()).unwrap(), old);
    let rows = store::load_snapshot(dir.path()).unwrap();
    let ppp = rows.iter().find(|s| s.node == "ppp").unwrap();
    assert!(ppp.download_min <= 10. && ppp.download_min >= 0.128);
    assert_eq!(ppp.download, 50.);
}
