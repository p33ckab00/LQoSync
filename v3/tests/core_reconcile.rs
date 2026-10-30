use lqosync_v3::{
    assignment::AssignmentState,
    core_reconcile::{self, Cleanup, Decision, Input, Options},
    dynamic::{self, Target},
    normalizer,
    routeros::Inventory,
    sessions,
    wan_health::{self, Config, Evidence, State},
};
use std::{collections::BTreeMap, sync::Arc};
mod common;
#[path = "common/core.rs"]
mod core_fixture;
fn fixture() -> (
    lqosync_v3::model::Settings,
    BTreeMap<String, Arc<Inventory>>,
    sessions::Observation,
) {
    let (s, mut inv, _, _) = core_fixture::fixture();
    let mut s = dynamic::migrate(&s).unwrap();
    s.nodes.truncate(1);
    let c = s.dynamic.as_mut().unwrap();
    let mut second = c.wans[0].clone();
    second.id = "wan2".into();
    second.interface = "uplink2".into();
    c.wans.push(second);
    for p in &mut c.policies {
        p.default = Target {
            node: "wan".into(),
            wan: "wan".into(),
        };
        p.targets = vec![
            p.default.clone(),
            Target {
                node: "wan".into(),
                wan: "wan2".into(),
            },
        ];
        p.balance = true;
    }
    let mut b = s.core_lists.bindings[0].clone();
    b.wan = "wan2".into();
    b.list = "lqos-wan2".into();
    b.table = "to-wan2".into();
    s.core_lists.bindings.push(b);
    let i = Arc::make_mut(inv.get_mut("r1").unwrap());
    for (table, entry) in [
        (
            "/interface/print",
            core_fixture::row(&[
                ("name", "uplink2"),
                ("running", "true"),
                ("disabled", "false"),
            ]),
        ),
        (
            "/routing/table/print",
            core_fixture::row(&[("name", "to-wan2"), ("fib", "true")]),
        ),
        (
            "/ip/route/print",
            core_fixture::row(&[
                ("dst-address", "0.0.0.0/0"),
                ("routing-table", "to-wan2"),
                ("immediate-gw", "203.0.113.2%uplink2"),
                ("active", "true"),
            ]),
        ),
        (
            "/ip/firewall/mangle/print",
            core_fixture::row(&[
                (".id", "*2"),
                ("chain", "prerouting"),
                ("action", "mark-routing"),
                ("src-address-list", "lqos-wan2"),
                ("new-routing-mark", "to-wan2"),
                ("passthrough", "false"),
                ("dst-address-type", "!local"),
            ]),
        ),
    ] {
        i.tables.get_mut(table).unwrap().push(entry);
    }
    for path in ["/ppp/active/print", "/ip/hotspot/active/print"] {
        i.tables.get_mut(path).unwrap()[0]
            .extend([(".id".into(), "*1".into()), ("uptime".into(), "1h".into())]);
    }
    let mut raw = normalizer::collect(&s, &inv, &BTreeMap::new(), 101).unwrap();
    let obs = sessions::plan(
        &s,
        &mut raw,
        &AssignmentState::default(),
        None,
        &inv,
        &BTreeMap::new(),
        101,
    )
    .unwrap();
    (s, inv, obs)
}
fn health(s: &lqosync_v3::model::Settings, wan: &str, up: bool) -> State {
    let c = Config {
        failure_rounds: 1,
        success_rounds: 1,
        recovery_seconds: 0,
        interval_seconds: 1,
        ..Default::default()
    };
    let e = |n| Evidence {
        settings_revision: wan_health::settings_revision(s).unwrap(),
        wan: wan.into(),
        context: "test".into(),
        observed_at: n,
        router_uptime: 1000 + n,
        counters: [n; 2],
        passed: [up; 2],
    };
    let first = wan_health::evaluate(&c, None, Ok(e(99)), 99).unwrap();
    wan_health::evaluate(&c, Some(&first), Ok(e(100)), 100).unwrap()
}
fn run(
    s: &lqosync_v3::model::Settings,
    inv: &BTreeMap<String, Arc<Inventory>>,
    obs: &sessions::Observation,
    prev: Option<&sessions::Observation>,
    h: &BTreeMap<String, State>,
    cleanup: &Cleanup,
    now: u64,
) -> core_reconcile::ReconcilePlan {
    core_reconcile::plan(Input {
        settings: s,
        inventories: inv,
        observation: obs,
        previous: prev,
        health: h,
        cleanup,
        options: Options::default(),
        now,
    })
    .unwrap()
}
fn owned(
    s: &lqosync_v3::model::Settings,
    inv: &mut BTreeMap<String, Arc<Inventory>>,
    obs: &sessions::Observation,
) {
    let raw = normalizer::collect(s, inv, &BTreeMap::new(), 101).unwrap();
    let table = Arc::make_mut(inv.get_mut("r1").unwrap())
        .tables
        .get_mut("/ip/firewall/address-list/print")
        .unwrap();
    for (n, row) in raw.iter().enumerate() {
        let wan = &obs.assignments.routes[&row.id].wan;
        let mut r = core_fixture::owned(
            &format!("*{:X}", n + 10),
            &format!("{}/32", row.ipv4),
            &row.id,
            wan,
        );
        r.insert(
            "list".into(),
            s.core_lists
                .bindings
                .iter()
                .find(|b| b.wan == *wan)
                .unwrap()
                .list
                .clone(),
        );
        table.push(r);
    }
}
#[test]
fn arrivals_balance_healthy_wans_and_keep_input_unchanged() {
    let (s, inv, obs) = fixture();
    let h = BTreeMap::from([
        ("wan".into(), health(&s, "wan", true)),
        ("wan2".into(), health(&s, "wan2", true)),
    ]);
    let c = Cleanup::default();
    let p = run(&s, &inv, &obs, None, &h, &c, 101);
    assert_eq!(
        p.decisions
            .iter()
            .filter(|d| matches!(d, Decision::Add { .. }))
            .count(),
        3
    );
    let counts = p
        .assignments
        .routes
        .values()
        .fold(BTreeMap::new(), |mut m, r| {
            *m.entry(&r.wan).or_insert(0) += 1;
            m
        });
    assert_eq!(
        counts.values().max().unwrap() - counts.values().min().unwrap(),
        1
    );
    assert!(inv["r1"].tables["/ip/firewall/address-list/print"].is_empty());
}
#[test]
fn failed_wan_evacuates_same_core_and_recovery_does_not_force_failback() {
    let (s, mut inv, obs) = fixture();
    owned(&s, &mut inv, &obs);
    let h = BTreeMap::from([
        ("wan".into(), health(&s, "wan", false)),
        ("wan2".into(), health(&s, "wan2", true)),
    ]);
    let p = run(&s, &inv, &obs, Some(&obs), &h, &Cleanup::default(), 101);
    assert!(p.decisions.iter().any(|d|matches!(d,Decision::Move{before,after,..} if before.id==after.id && before.list!=after.list)));
    assert!(p.assignments.routes.values().all(|r| r.wan == "wan2"));
    let mut applied = inv.clone();
    for r in Arc::make_mut(applied.get_mut("r1").unwrap())
        .tables
        .get_mut("/ip/firewall/address-list/print")
        .unwrap()
    {
        let id = r["comment"].split(':').nth(2).unwrap().to_owned();
        r.insert("list".into(), "lqos-wan2".into());
        r.insert(
            "comment".into(),
            lqosync_v3::core_plan::marker(&s.core_lists.owner, &id, "wan2"),
        );
    }
    let mut retained = obs.clone();
    retained.assignments = p.assignments;
    let recovered = BTreeMap::from([
        ("wan".into(), health(&s, "wan", true)),
        ("wan2".into(), health(&s, "wan2", true)),
    ]);
    let p = run(
        &s,
        &applied,
        &retained,
        Some(&retained),
        &recovered,
        &Cleanup::default(),
        101,
    );
    assert!(
        p.decisions
            .iter()
            .all(|d| matches!(d, Decision::Keep { .. }))
    );
}
#[test]
fn unknown_health_blocks_arrivals_and_pins_survive_failure_without_override() {
    let (mut s, mut inv, obs) = fixture();
    let h = BTreeMap::new();
    let p = run(&s, &inv, &obs, None, &h, &Cleanup::default(), 101);
    assert!(
        p.decisions
            .iter()
            .all(|d| matches!(d, Decision::Blocked { .. }))
    );
    owned(&s, &mut inv, &obs);
    let (id, route) = obs
        .assignments
        .routes
        .iter()
        .find(|(_, r)| r.wan == "wan")
        .unwrap();
    s.dynamic.as_mut().unwrap().pins.insert(
        id.clone(),
        dynamic::DynamicPin {
            policy: route.policy.clone(),
            target: Target {
                node: route.node.clone(),
                wan: route.wan.clone(),
            },
        },
    );
    let h = BTreeMap::from([
        ("wan".into(), health(&s, "wan", false)),
        ("wan2".into(), health(&s, "wan2", true)),
    ]);
    let p = run(&s, &inv, &obs, Some(&obs), &h, &Cleanup::default(), 101);
    assert!(
        p.decisions
            .iter()
            .any(|d| matches!(d,Decision::Blocked{subscriber,..} if subscriber==id))
    );
}
#[test]
fn a_recent_health_read_cannot_extend_an_old_completed_sample() {
    let (s, mut inv, mut obs) = fixture();
    for i in inv.values_mut() {
        Arc::make_mut(i).collected_at = 140;
    }
    for at in obs.collected.values_mut() {
        *at = 140;
    }
    for a in obs.active.values_mut() {
        a.collected_at = 140;
    }
    let cfg = Config {
        interval_seconds: 1,
        failure_rounds: 1,
        success_rounds: 1,
        recovery_seconds: 0,
        ..Default::default()
    };
    let fresh = health(&s, "wan", true);
    let frozen = wan_health::evaluate(
        &cfg,
        Some(&fresh),
        Ok(Evidence {
            settings_revision: wan_health::settings_revision(&s).unwrap(),
            wan: "wan".into(),
            context: "test".into(),
            observed_at: 129,
            router_uptime: 1129,
            counters: [100; 2],
            passed: [true; 2],
        }),
        129,
    )
    .unwrap();
    assert_eq!(frozen.status, wan_health::HealthStatus::Healthy);
    let h = BTreeMap::from([
        ("wan".into(), frozen),
        ("wan2".into(), health(&s, "wan2", true)),
    ]);
    let p = run(&s, &inv, &obs, None, &h, &Cleanup::default(), 140);
    assert!(
        p.decisions
            .iter()
            .all(|d| matches!(d, Decision::Blocked { .. }))
    );
}
#[test]
fn inactive_failed_source_default_is_allowed_but_bad_destination_routes_are_not() {
    let (s, mut inv, obs) = fixture();
    owned(&s, &mut inv, &obs);
    let h = BTreeMap::from([
        ("wan".into(), health(&s, "wan", false)),
        ("wan2".into(), health(&s, "wan2", true)),
    ]);
    Arc::make_mut(inv.get_mut("r1").unwrap())
        .tables
        .get_mut("/ip/route/print")
        .unwrap()[0]
        .insert("active".into(), "false".into());
    let p = run(&s, &inv, &obs, Some(&obs), &h, &Cleanup::default(), 101);
    assert!(
        p.decisions
            .iter()
            .any(|d| matches!(d, Decision::Move { .. }))
    );
    Arc::make_mut(inv.get_mut("r1").unwrap())
        .tables
        .get_mut("/ip/route/print")
        .unwrap()[1]
        .insert("immediate-gw".into(), "203.0.113.1%uplink".into());
    assert!(
        core_reconcile::plan(Input {
            settings: &s,
            inventories: &inv,
            observation: &obs,
            previous: Some(&obs),
            health: &h,
            cleanup: &Cleanup::default(),
            options: Options::default(),
            now: 101
        })
        .is_err()
    );
}
fn advance(inv: &mut BTreeMap<String, Arc<Inventory>>, obs: &mut sessions::Observation, at: u64) {
    for i in inv.values_mut() {
        Arc::make_mut(i).collected_at = at;
    }
    for value in obs.collected.values_mut() {
        *value = at;
    }
    for session in obs.active.values_mut() {
        session.collected_at = at;
    }
}
#[test]
fn offline_cleanup_needs_two_complete_observations_and_exact_grace_boundary() {
    let (s, mut inv, old) = fixture();
    owned(&s, &mut inv, &old);
    let mut obs = old.clone();
    let id = old
        .active
        .iter()
        .find(|(_, s)| s.service == "ppp")
        .unwrap()
        .0
        .clone();
    obs.active.remove(&id);
    Arc::make_mut(inv.get_mut("r1").unwrap())
        .tables
        .get_mut("/ppp/active/print")
        .unwrap()
        .clear();
    advance(&mut inv, &mut obs, 110);
    let h = BTreeMap::from([
        ("wan".into(), health(&s, "wan", true)),
        ("wan2".into(), health(&s, "wan2", true)),
    ]);
    let first = run(&s, &inv, &obs, Some(&old), &h, &Cleanup::default(), 110);
    assert!(
        !first
            .decisions
            .iter()
            .any(|d| matches!(d, Decision::Remove { .. }))
    );
    advance(&mut inv, &mut obs, 139);
    let before = run(&s, &inv, &obs, Some(&old), &h, &first.cleanup, 139);
    assert!(
        !before
            .decisions
            .iter()
            .any(|d| matches!(d, Decision::Remove { .. }))
    );
    advance(&mut inv, &mut obs, 140);
    let after = run(&s, &inv, &obs, Some(&old), &h, &before.cleanup, 140);
    assert!(
        after
            .decisions
            .iter()
            .any(|d| matches!(d,Decision::Remove{subscriber,..} if subscriber==&id))
    );
    let repeated = run(&s, &inv, &obs, Some(&old), &h, &after.cleanup, 140);
    assert!(
        !repeated
            .decisions
            .iter()
            .any(|d| matches!(d, Decision::Remove { .. }))
    );
}
#[test]
fn residual_coverage_ip_reuse_and_ip_changes_never_produce_unsafe_moves() {
    let (s, mut inv, obs) = fixture();
    owned(&s, &mut inv, &obs);
    let raw = normalizer::collect(&s, &inv, &BTreeMap::new(), 101).unwrap();
    let id = &raw[0].id;
    let ip = &raw[0].ipv4;
    let h = BTreeMap::from([
        ("wan".into(), health(&s, "wan", false)),
        ("wan2".into(), health(&s, "wan2", true)),
    ]);
    let table = Arc::make_mut(inv.get_mut("r1").unwrap())
        .tables
        .get_mut("/ip/firewall/address-list/print")
        .unwrap();
    table.push(core_fixture::row(&[
        (".id", "*FF"),
        ("list", "lqos-wan"),
        ("address", &format!("{ip}/32")),
        ("comment", "manual"),
    ]));
    let p = run(&s, &inv, &obs, Some(&obs), &h, &Cleanup::default(), 101);
    assert!(p.decisions.iter().any(|d|matches!(d,Decision::Blocked{subscriber,reason} if subscriber==id && reason.contains("Protected"))));
    let table = Arc::make_mut(inv.get_mut("r1").unwrap())
        .tables
        .get_mut("/ip/firewall/address-list/print")
        .unwrap();
    table.pop();
    let entry = table
        .iter_mut()
        .find(|r| r["comment"].contains(id))
        .unwrap();
    entry.insert("address".into(), "192.0.2.254/32".into());
    let p = run(&s, &inv, &obs, Some(&obs), &h, &Cleanup::default(), 101);
    assert!(p.decisions.iter().any(|d|matches!(d,Decision::Blocked{subscriber,reason} if subscriber==id && reason.contains("IP change"))));
    let table = Arc::make_mut(inv.get_mut("r1").unwrap())
        .tables
        .get_mut("/ip/firewall/address-list/print")
        .unwrap();
    let entry = table
        .iter_mut()
        .find(|r| r["comment"].contains(id))
        .unwrap();
    entry.insert("address".into(), format!("{ip}/32"));
    entry.insert(
        "comment".into(),
        lqosync_v3::core_plan::marker(
            &s.core_lists.owner,
            &uuid::Uuid::new_v4().to_string(),
            &obs.assignments.routes[id].wan,
        ),
    );
    let p = run(&s, &inv, &obs, Some(&obs), &h, &Cleanup::default(), 101);
    assert!(p.decisions.iter().any(|d|matches!(d,Decision::Blocked{subscriber,reason} if subscriber==id && reason.contains("another subscriber"))));
}
#[test]
fn pin_override_is_explicit_and_manual_arrivals_follow_default() {
    let (mut s, mut inv, obs) = fixture();
    owned(&s, &mut inv, &obs);
    let (id, r) = obs
        .assignments
        .routes
        .iter()
        .find(|(_, r)| r.wan == "wan")
        .unwrap();
    s.dynamic.as_mut().unwrap().pins.insert(
        id.clone(),
        dynamic::DynamicPin {
            policy: r.policy.clone(),
            target: Target {
                node: r.node.clone(),
                wan: r.wan.clone(),
            },
        },
    );
    let h = BTreeMap::from([
        ("wan".into(), health(&s, "wan", false)),
        ("wan2".into(), health(&s, "wan2", true)),
    ]);
    let p = core_reconcile::plan(Input {
        settings: &s,
        inventories: &inv,
        observation: &obs,
        previous: Some(&obs),
        health: &h,
        cleanup: &Cleanup::default(),
        options: Options {
            pin_failover: true,
            ..Default::default()
        },
        now: 101,
    })
    .unwrap();
    assert!(
        p.decisions
            .iter()
            .any(|d| matches!(d,Decision::Move{subscriber,..} if subscriber==id))
    );
    let (mut s, inv, obs) = fixture();
    for p in &mut s.dynamic.as_mut().unwrap().policies {
        p.balance = false;
    }
    let h = BTreeMap::from([
        ("wan".into(), health(&s, "wan", true)),
        ("wan2".into(), health(&s, "wan2", true)),
    ]);
    let p = run(&s, &inv, &obs, None, &h, &Cleanup::default(), 101);
    assert!(p.assignments.routes.values().all(|r| r.wan == "wan"));
}
#[test]
fn settings_changes_invalidate_health_and_incomplete_or_empty_scans_cannot_delete() {
    let (mut s, mut inv, mut obs) = fixture();
    owned(&s, &mut inv, &obs);
    let h = BTreeMap::from([
        ("wan".into(), health(&s, "wan", true)),
        ("wan2".into(), health(&s, "wan2", true)),
    ]);
    s.dynamic.as_mut().unwrap().wans[0].name = "renamed".into();
    let p = run(&s, &inv, &obs, None, &h, &Cleanup::default(), 101);
    assert!(
        !p.decisions
            .iter()
            .any(|d| matches!(d, Decision::Add { .. } | Decision::Move { .. }))
    );
    let old = obs.clone();
    for path in [
        "/ppp/active/print",
        "/ip/dhcp-server/lease/print",
        "/ip/hotspot/active/print",
    ] {
        Arc::make_mut(inv.get_mut("r1").unwrap())
            .tables
            .get_mut(path)
            .unwrap()
            .clear();
    }
    obs.active.clear();
    let p = run(&s, &inv, &obs, Some(&old), &h, &Cleanup::default(), 101);
    assert!(
        p.decisions
            .iter()
            .all(|d| matches!(d, Decision::Blocked { .. }))
    );
    assert!(p.cleanup.absent.is_empty());
    Arc::make_mut(inv.get_mut("r1").unwrap())
        .errors
        .insert("/ppp/active/print".into(), "failed".into());
    assert!(
        core_reconcile::plan(Input {
            settings: &s,
            inventories: &inv,
            observation: &obs,
            previous: Some(&old),
            health: &h,
            cleanup: &Cleanup::default(),
            options: Options::default(),
            now: 101
        })
        .is_err()
    );
}
#[test]
fn cross_core_evacuation_is_blocked_and_reconnects_can_choose_a_new_healthy_wan() {
    let (mut s, mut inv, obs) = fixture();
    owned(&s, &mut inv, &obs);
    let old_id = obs
        .assignments
        .routes
        .iter()
        .find(|(_, r)| r.wan == "wan")
        .unwrap()
        .0
        .clone();
    let mut router = s.routers[0].clone();
    router.id = "r2".into();
    router.role = "core".into();
    s.routers.push(router);
    s.dynamic.as_mut().unwrap().wans[1].router = "r2".into();
    s.core_lists.bindings[1].router = "r2".into();
    let mut second = (*inv["r1"]).clone();
    second.router_id = "r2".into();
    second
        .tables
        .get_mut("/ip/firewall/address-list/print")
        .unwrap()
        .clear();
    inv.insert("r2".into(), Arc::new(second));
    for (router, list) in [("r1", "lqos-wan"), ("r2", "lqos-wan2")] {
        Arc::make_mut(inv.get_mut(router).unwrap())
            .tables
            .get_mut("/ip/firewall/mangle/print")
            .unwrap()
            .retain(|r| r.get("src-address-list").is_some_and(|v| v == list));
    }

    let h = BTreeMap::from([
        ("wan".into(), health(&s, "wan", false)),
        ("wan2".into(), health(&s, "wan2", true)),
    ]);
    let p = run(&s, &inv, &obs, Some(&obs), &h, &Cleanup::default(), 101);
    assert!(p.decisions.iter().any(|d|matches!(d,Decision::Blocked{subscriber,reason} if subscriber==&old_id && reason.contains("Cross-core"))));
    let (s, mut inv, old) = fixture();
    owned(&s, &mut inv, &old);
    let mut obs = old.clone();
    for session in obs.active.values_mut() {
        if session.id.is_some() {
            session.id = Some("*AA".into());
            session.uptime = Some(1);
        }
    }
    let h = BTreeMap::from([
        ("wan".into(), health(&s, "wan", true)),
        ("wan2".into(), health(&s, "wan2", true)),
    ]);
    let p = run(&s, &inv, &obs, Some(&old), &h, &Cleanup::default(), 101);
    assert_eq!(p.assignments.routes.len(), 3);
    // Deterministic scoring includes the retained DHCP subscriber before pending reconnects.
    let p2 = run(&s, &inv, &obs, Some(&old), &h, &Cleanup::default(), 101);
    assert_eq!(p.revision, p2.revision);
}
#[test]
fn first_session_metadata_does_not_rebalance_previously_retained_clients() {
    let (s, mut inv, obs) = fixture();
    let mut prior = obs.clone();
    prior.active.clear();
    for r in prior.assignments.routes.values_mut() {
        r.wan = "wan".into();
    }
    owned(&s, &mut inv, &prior);
    let h = BTreeMap::from([
        ("wan".into(), health(&s, "wan", true)),
        ("wan2".into(), health(&s, "wan2", true)),
    ]);
    let p = run(&s, &inv, &obs, Some(&prior), &h, &Cleanup::default(), 101);
    assert!(
        p.decisions
            .iter()
            .all(|d| matches!(d, Decision::Keep { .. }))
    );
}
#[test]
fn manual_policy_cleanup_uses_its_selected_source_when_session_metadata_is_absent() {
    let (mut s, mut inv, mut prior) = fixture();
    for p in &mut s.dynamic.as_mut().unwrap().policies {
        p.balance = false;
    }
    prior.active.clear();
    owned(&s, &mut inv, &prior);
    let id = prior
        .assignments
        .routes
        .iter()
        .find(|(_, r)| {
            s.selections
                .iter()
                .any(|sel| sel.id == r.selection && sel.kind == "pppoe")
        })
        .unwrap()
        .0
        .clone();
    Arc::make_mut(inv.get_mut("r1").unwrap())
        .tables
        .get_mut("/ppp/active/print")
        .unwrap()
        .clear();
    let mut obs = prior.clone();
    advance(&mut inv, &mut obs, 110);
    let h = BTreeMap::new();
    let first = run(&s, &inv, &obs, Some(&prior), &h, &Cleanup::default(), 110);
    advance(&mut inv, &mut obs, 140);
    let second = run(&s, &inv, &obs, Some(&prior), &h, &first.cleanup, 140);
    assert!(
        second
            .decisions
            .iter()
            .any(|d| matches!(d,Decision::Remove{subscriber,..} if subscriber==&id))
    );
}
#[test]
fn healthy_destination_requires_a_current_running_interface() {
    let (s, mut inv, obs) = fixture();
    let h = BTreeMap::from([
        ("wan".into(), health(&s, "wan", false)),
        ("wan2".into(), health(&s, "wan2", true)),
    ]);
    Arc::make_mut(inv.get_mut("r1").unwrap())
        .tables
        .get_mut("/interface/print")
        .unwrap()[1]
        .insert("running".into(), "false".into());
    assert!(
        core_reconcile::plan(Input {
            settings: &s,
            inventories: &inv,
            observation: &obs,
            previous: None,
            health: &h,
            cleanup: &Cleanup::default(),
            options: Options::default(),
            now: 101
        })
        .is_err()
    );
}
