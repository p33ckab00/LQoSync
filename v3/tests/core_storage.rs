use lqosync_v3::{core_plan, model, orchestrator, store, web::App};
use std::{collections::BTreeMap, sync::Arc};
use tokio::sync::Mutex;
mod common;
#[path = "common/core.rs"]
mod core_fixture;
#[tokio::test]
async fn manual_core_conflict_preserves_output_but_advances_session_candidate_and_live_review() {
    let (mut settings, mut inventories, _, _) = core_fixture::fixture();
    settings
        .assignment
        .pools
        .push(lqosync_v3::assignment::Pool {
            source_node: "ppp".into(),
            targets: vec!["ppp".into()],
            balance: true,
            metric: "online_clients".into(),
        });
    let i = Arc::make_mut(inventories.get_mut("r1").unwrap());
    i.collected_at = orchestrator::now() - 2;
    i.tables.get_mut("/ppp/active/print").unwrap()[0].insert(".id".into(), "*1".into());
    let dir = tempfile::tempdir().unwrap();
    store::save_settings(dir.path(), &settings).unwrap();
    let mut app = App::open(dir.path().into()).unwrap();
    app.inventories = inventories;
    let state = Arc::new(Mutex::new(app));
    orchestrator::refresh(state.clone()).await;
    let pointer = std::fs::read(dir.path().join("state/current.toml")).unwrap();
    {
        let mut a = state.lock().await;
        let i = Arc::make_mut(a.inventories.get_mut("r1").unwrap());
        i.collected_at = orchestrator::now() - 1;
        i.tables.get_mut("/ppp/active/print").unwrap()[0].insert(".id".into(), "*2".into());
        i.tables
            .get_mut("/ip/firewall/address-list/print")
            .unwrap()
            .push(core_fixture::row(&[
                (".id", "*F"),
                ("list", "lqos-wan"),
                ("address", "192.0.2.1/32"),
                ("dynamic", "false"),
                ("disabled", "false"),
            ]));
    }
    orchestrator::refresh(state.clone()).await;
    assert_eq!(
        std::fs::read(dir.path().join("state/current.toml")).unwrap(),
        pointer
    );
    let candidate = lqosync_v3::sessions::load(dir.path()).unwrap().unwrap();
    assert_eq!(
        candidate.active.values().next().unwrap().id.as_deref(),
        Some("*2")
    );
    assert!(state.lock().await.status.contains("protected"));
    let a = state.lock().await;
    let review = lqosync_v3::validation::review(
        &a.settings,
        &a.inventories,
        &a.inventory_errors,
        dir.path(),
        orchestrator::now(),
        lqosync_v3::core_diagnostics::Origin::Live,
    );
    assert!(
        review
            .checks
            .iter()
            .any(|c| c.code == "subscriber-assignment"
                && c.status == lqosync_v3::core_diagnostics::Outcome::Pass)
    );
    assert!(review.checks.iter().any(|c| c.code == "core-membership"
        && c.status == lqosync_v3::core_diagnostics::Outcome::Conflict));
    assert_eq!(
        lqosync_v3::sessions::load(dir.path()).unwrap().unwrap(),
        candidate
    );
    assert_eq!(
        std::fs::read(dir.path().join("state/current.toml")).unwrap(),
        pointer
    );
}
#[test]
fn core_plan_commits_with_its_snapshot_reuses_identical_inputs_and_rejects_corruption() {
    let (s, mut inv, subs, state) = core_fixture::fixture();
    Arc::make_mut(inv.get_mut("r1").unwrap()).collected_at = orchestrator::now();
    let dir = tempfile::tempdir().unwrap();
    let shaping = model::compile(&s, &subs).unwrap();
    let core = core_plan::build(
        &s,
        &subs,
        &state,
        &inv,
        &BTreeMap::new(),
        orchestrator::now(),
    )
    .unwrap();
    store::publish_normalized(dir.path(), &shaping, &subs).unwrap();
    assert!(store::load_core_plan(dir.path()).unwrap().is_none());
    let publication =
        store::publish_orchestrated(dir.path(), &shaping, &subs, &state, Some(&core)).unwrap();
    let committed = store::load_core_plan(dir.path()).unwrap().unwrap();
    assert_eq!(committed.generation, publication.generation);
    assert_eq!(committed.plan, core);
    for file in [
        "core-plan.json",
        "subscribers.toml",
        "assignments.toml",
        "network.json",
        "ShapedDevices.csv",
    ] {
        assert!(
            dir.path()
                .join("staging")
                .join(&committed.generation)
                .join(file)
                .exists()
        );
    }
    assert_eq!(
        store::publish_orchestrated(dir.path(), &shaping, &subs, &state, Some(&core))
            .unwrap()
            .generation,
        committed.generation
    );
    assert!(store::publish_assigned(dir.path(), &shaping, &subs, &state).is_err());
    let before = std::fs::read(dir.path().join("state/current.toml")).unwrap();
    let mut bad = core.clone();
    bad.entries[0].address = "198.51.100.99/32".into();
    assert!(store::publish_orchestrated(dir.path(), &shaping, &subs, &state, Some(&bad)).is_err());
    assert_eq!(
        before,
        std::fs::read(dir.path().join("state/current.toml")).unwrap()
    );
    let path = dir
        .path()
        .join("staging")
        .join(&committed.generation)
        .join("core-plan.json");
    std::fs::write(&path, serde_json::to_string(&bad).unwrap()).unwrap();
    assert!(store::load_core_plan(dir.path()).is_err());
    assert!(store::load_assignments(dir.path()).is_err());
    std::fs::remove_file(&path).unwrap();
    assert!(store::load_core_plan(dir.path()).is_err());
    std::fs::write(&path, "not json").unwrap();
    assert!(store::load_core_plan(dir.path()).is_err());
}
#[tokio::test]
async fn core_health_and_route_failures_retain_the_entire_last_valid_generation() {
    let (mut settings, mut inventories, _, _) = core_fixture::fixture();
    let dir = tempfile::tempdir().unwrap();
    settings.auto_stage = true;
    Arc::make_mut(inventories.get_mut("r1").unwrap()).collected_at = orchestrator::now();
    store::save_settings(dir.path(), &settings).unwrap();
    let mut a = App::open(dir.path().into()).unwrap();
    a.inventories = inventories;
    let state = Arc::new(Mutex::new(a));
    orchestrator::refresh(state.clone()).await;
    let before = std::fs::read(dir.path().join("state/current.toml")).unwrap();
    let committed = store::load_core_plan(dir.path()).unwrap().unwrap();
    state
        .lock()
        .await
        .inventory_errors
        .insert("r1".into(), "failed".into());
    orchestrator::refresh(state.clone()).await;
    assert_eq!(
        before,
        std::fs::read(dir.path().join("state/current.toml")).unwrap()
    );
    {
        let mut a = state.lock().await;
        a.inventory_errors.clear();
        Arc::make_mut(a.inventories.get_mut("r1").unwrap())
            .tables
            .get_mut("/ip/route/print")
            .unwrap()[0]
            .insert("immediate-gw".into(), "203.0.113.1%wrong".into());
    }
    orchestrator::refresh(state.clone()).await;
    assert_eq!(
        before,
        std::fs::read(dir.path().join("state/current.toml")).unwrap()
    );
    assert!(state.lock().await.status.contains("different interface"));
    assert_eq!(
        committed.plan,
        store::load_core_plan(dir.path()).unwrap().unwrap().plan
    );
}

#[test]
fn expired_source_provenance_cannot_switch_the_pointer_even_when_the_core_is_fresh() {
    let (mut s, mut inv, subs, state) = core_fixture::fixture();
    let dir = tempfile::tempdir().unwrap();
    let now = orchestrator::now();
    Arc::make_mut(inv.get_mut("r1").unwrap()).collected_at = now;
    let mut core_router = s.routers[0].clone();
    core_router.id = "core".into();
    core_router.role = "core".into();
    s.routers.push(core_router);
    s.routers[0].role = "distro".into();
    s.nodes[0].router = "core".into();
    s.core_lists.bindings[0].router = "core".into();
    let mut core_inventory = (*inv["r1"]).clone();
    core_inventory.router_id = "core".into();
    inv.insert("core".into(), Arc::new(core_inventory));
    let shape = model::compile(&s, &subs).unwrap();
    let mut core = core_plan::build(&s, &subs, &state, &inv, &BTreeMap::new(), now).unwrap();
    assert_eq!(core.source_routers, vec!["r1"]);
    assert_eq!(core.inventory_at.len(), 2);
    store::publish_orchestrated(dir.path(), &shape, &subs, &state, Some(&core)).unwrap();
    let before = std::fs::read(dir.path().join("state/current.toml")).unwrap();
    core.inventory_at.insert("r1".into(), now - 301);
    core_plan::validate_snapshot(&core, &subs).unwrap();
    assert!(
        store::publish_orchestrated(dir.path(), &shape, &subs, &state, Some(&core)).is_err(),
        "expired source authorized a new generation"
    );
    assert_eq!(
        before,
        std::fs::read(dir.path().join("state/current.toml")).unwrap()
    );
}
