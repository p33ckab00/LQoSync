use lqosync_v3::{core_plan, model, orchestrator, store, web::App};
use std::{collections::BTreeMap, sync::Arc};
use tokio::sync::Mutex;
mod common;
#[path = "common/core.rs"]
mod core_fixture;
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
