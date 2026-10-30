use lqosync_v3::{
    assignment::AssignmentState,
    dynamic::{self, Target},
    minimums, model, normalizer,
    selection::Selection,
    sessions,
};
use std::{collections::BTreeMap, sync::Arc, time::Instant};
mod common;
#[test]
#[ignore = "manual 50k independent selectors/session/capacity/persistence workload"]
fn fifty_thousand_independent_selectors_share_flat_roots_and_wan_budgets() {
    let (mut s, mut inventory) = common::fixture();
    s.nodes.retain(|n| n.id == "wan" || n.id == "ppp");
    s.nodes[0].router = "r1".into();
    s.nodes[0].source = "wan-a".into();
    s.nodes[0].download = 200000.;
    s.nodes[0].upload = 200000.;
    s = dynamic::migrate(&s).unwrap();
    s.nodes.truncate(1);
    s.selections = (0..50000)
        .map(|i| Selection {
            id: format!("source-{i}"),
            router: "r1".into(),
            kind: "pppoe".into(),
            selector: "subscriber".into(),
            value: format!("bench-{i}"),
            node: String::new(),
            exclude: false,
        })
        .collect();
    let c = s.dynamic.as_mut().unwrap();
    c.wans[0].download = 100000.;
    c.wans[0].upload = 100000.;
    let mut wan = c.wans[0].clone();
    wan.id = "wan2".into();
    wan.interface = "wan-b".into();
    c.wans.push(wan);
    let p = &mut c.policies[0];
    p.selections = s.selections.iter().map(|s| s.id.clone()).collect();
    p.balance = true;
    p.targets = vec![
        Target {
            node: "wan".into(),
            wan: "wan".into(),
        },
        Target {
            node: "wan".into(),
            wan: "wan2".into(),
        },
    ];
    p.default = p.targets[0].clone();
    let inv = Arc::make_mut(inventory.get_mut("r1").unwrap());
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
    inv.tables.insert(
        "/ppp/active/print".into(),
        (0..50000u32)
            .map(|i| {
                [
                    ("name".into(), format!("bench-{i}")),
                    (
                        "address".into(),
                        std::net::Ipv4Addr::from(0x0a000001 + i).to_string(),
                    ),
                    (".id".into(), format!("*{:X}", i + 1)),
                    ("uptime".into(), "1h".into()),
                ]
                .into_iter()
                .collect()
            })
            .collect(),
    );
    inv.tables.insert(
        "/ppp/secret/print".into(),
        (0..50000)
            .map(|i| {
                [
                    ("name".into(), format!("bench-{i}")),
                    ("profile".into(), "Plan".into()),
                ]
                .into_iter()
                .collect()
            })
            .collect(),
    );
    let runtime = tokio::runtime::Runtime::new().unwrap();
    let render_page = |root: &std::path::Path| {
        let mut app = lqosync_v3::web::App::open(root.into()).unwrap();
        app.settings = s.clone();
        let start = Instant::now();
        let bytes = runtime.block_on(async move {
            use tower::ServiceExt;
            let response = lqosync_v3::web::routes(Arc::new(tokio::sync::Mutex::new(app)))
                .oneshot(
                    axum::http::Request::builder()
                        .uri("/assignments")
                        .header("host", "localhost:9303")
                        .body(axum::body::Body::empty())
                        .unwrap(),
                )
                .await
                .unwrap();
            assert_eq!(response.status().as_u16(), 200);
            axum::body::to_bytes(response.into_body(), usize::MAX)
                .await
                .unwrap()
        });
        assert!(
            bytes.len() < 1_000_000,
            "policy summaries must stay bounded"
        );
        (start.elapsed(), bytes)
    };
    let render_dir = tempfile::tempdir().unwrap();
    let (render, bytes) = render_page(render_dir.path());
    assert!(
        render.as_secs() < 10,
        "50k source policy page took {render:?}"
    );
    eprintln!("50k policy HTTP render: {render:?}, {} bytes", bytes.len());
    let errors = BTreeMap::new();
    let t = Instant::now();
    let mut rows = normalizer::collect(&s, &inventory, &errors, 101).unwrap();
    let collect = t.elapsed();
    let t = Instant::now();
    let first = sessions::plan(
        &s,
        &mut rows,
        &AssignmentState::default(),
        None,
        &inventory,
        &errors,
        101,
    )
    .unwrap();
    let assign = t.elapsed();
    assert_eq!(
        first
            .assignments
            .routes
            .values()
            .filter(|r| r.wan == "wan")
            .count(),
        25000
    );
    let t = Instant::now();
    let mut next = normalizer::collect(&s, &inventory, &errors, 101).unwrap();
    let retained = sessions::plan(
        &s,
        &mut next,
        &AssignmentState::default(),
        Some(&first),
        &inventory,
        &errors,
        101,
    )
    .unwrap();
    let retain = t.elapsed();
    assert_eq!(first.assignments, retained.assignments);
    let t = Instant::now();
    minimums::allocate_assigned(&s, &mut rows, &first.assignments).unwrap();
    let output = model::compile_assigned(&s, &rows, &first.assignments).unwrap();
    let compile = t.elapsed();
    assert_eq!(output.csv.lines().count(), 50001);
    assert!(rows.iter().map(|r| r.download_min).sum::<f64>() <= 200000.0001);
    let dir = tempfile::tempdir().unwrap();
    let t = Instant::now();
    sessions::save(dir.path(), &first).unwrap();
    let save = t.elapsed();
    let t = Instant::now();
    let loaded = sessions::load(dir.path()).unwrap().unwrap();
    let load = t.elapsed();
    assert_eq!(loaded, first);
    let (candidate_render, bytes) = render_page(dir.path());
    assert!(String::from_utf8_lossy(&bytes).contains("stored candidate: 25000"));
    assert!(
        candidate_render.as_secs() < 15,
        "50k candidate evidence page took {candidate_render:?}"
    );
    eprintln!(
        "50k policy HTTP with stored candidate: {candidate_render:?}, {} bytes",
        bytes.len()
    );

    let inv = Arc::make_mut(inventory.get_mut("r1").unwrap());
    inv.collected_at = 102;
    for (i, row) in inv
        .tables
        .get_mut("/ppp/active/print")
        .unwrap()
        .iter_mut()
        .enumerate()
    {
        row.insert(".id".into(), format!("*{:X}", i + 50001));
        row.insert("uptime".into(), "1m".into());
    }
    let mut rows = normalizer::collect(&s, &inventory, &errors, 103).unwrap();
    let t = Instant::now();
    let reconnected = sessions::plan(
        &s,
        &mut rows,
        &AssignmentState::default(),
        Some(&loaded),
        &inventory,
        &errors,
        103,
    )
    .unwrap();
    let reconnect = t.elapsed();
    assert_eq!(
        reconnected
            .assignments
            .routes
            .values()
            .filter(|r| r.wan == "wan")
            .count(),
        25000
    );
    eprintln!(
        "50k independent selectors/flat root/2 WANs debug: collect={collect:?} assign={assign:?} retain_with_collect={retain:?} allocate_compile={compile:?} save={save:?} load={load:?} reconnect={reconnect:?}"
    );
}
