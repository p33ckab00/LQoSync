use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use http_body_util::BodyExt;
use lqosync_v3::{
    core_console::{self, Config, Mode},
    web::{App, routes},
};
use std::sync::Arc;
use tokio::sync::Mutex;
use tower::ServiceExt;
fn form(token: &str, revision: &str, mode: &str) -> String {
    format!(
        "token={token}&revision={revision}&mode={mode}&offline_seconds=30&evidence_seconds=30&batch_size=20&evacuate_active=true&failure_rounds=3&success_rounds=3&interval_seconds=10&recovery_seconds=30"
    )
}
#[tokio::test]
async fn preview_page_is_read_only_and_configuration_is_modal() {
    let d = tempfile::tempdir().unwrap();
    let state = Arc::new(Mutex::new(App::open(d.path().into()).unwrap()));
    let out = routes(state)
        .oneshot(
            Request::builder()
                .header("host", "127.0.0.1:9303")
                .uri("/core-control")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(out.status(), StatusCode::OK);
    let text =
        String::from_utf8(out.into_body().collect().await.unwrap().to_bytes().to_vec()).unwrap();
    assert!(text.contains("Automatic writes blocked"));
    assert!(text.contains("data-config"));
    assert!(text.contains("No preview evidence"));
    assert!(!d.path().join("state/core-migration-active.json").exists());
}
#[tokio::test]
async fn token_and_revision_checks_prevent_overwrites_and_automatic_enable() {
    let d = tempfile::tempdir().unwrap();
    let mut a = App::open(d.path().into()).unwrap();
    a.settings.auto_stage = false;
    let token = a.token.clone();
    let rev = core_console::revision(&a.settings).unwrap();
    let state = Arc::new(Mutex::new(a));
    for (t, r, m) in [
        ("wrong", rev.as_str(), "paused"),
        (token.as_str(), "stale", "paused"),
        (token.as_str(), rev.as_str(), "automatic"),
    ] {
        let response = routes(state.clone())
            .oneshot(
                Request::builder()
                    .header("host", "127.0.0.1:9303")
                    .method("POST")
                    .uri("/core-control/save")
                    .header("content-type", "application/x-www-form-urlencoded")
                    .body(Body::from(form(t, r, m)))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert!(!response.status().is_redirection());
        assert!(state.lock().await.settings.core_control.is_none());
    }
    let out = routes(state.clone())
        .oneshot(
            Request::builder()
                .header("host", "127.0.0.1:9303")
                .method("POST")
                .uri("/core-control/save")
                .header("content-type", "application/x-www-form-urlencoded")
                .body(Body::from(form(&token, &rev, "paused")))
                .unwrap(),
        )
        .await
        .unwrap();
    assert!(out.status().is_redirection());
    assert_eq!(
        state
            .lock()
            .await
            .settings
            .core_control
            .as_ref()
            .unwrap()
            .mode,
        Mode::Paused
    );
    assert!(d.path().join("settings.toml").exists());
    assert!(!d.path().join("state/core-migration-active.json").exists());
}
#[test]
fn legacy_settings_preserve_disabled_preview_and_invalid_controls_reject() {
    let s: lqosync_v3::model::Settings = toml::from_str("").unwrap();
    assert!(s.core_control.is_none());
    let c = Config {
        batch_size: 21,
        ..Default::default()
    };
    assert!(c.validate().is_err());
    let c = Config {
        mode: Mode::Automatic,
        ..Default::default()
    };
    assert!(c.validate().is_err());
}
#[tokio::test]
async fn paused_preview_discards_previous_health_and_cleanup_without_reads() {
    let d = tempfile::tempdir().unwrap();
    let mut a = App::open(d.path().into()).unwrap();
    a.settings.core_control = Some(Config {
        mode: Mode::Paused,
        ..Default::default()
    });
    let shared = Arc::new(Mutex::new(a));
    core_console::refresh(shared.clone()).await;
    let a = shared.lock().await;
    assert!(a.core_runtime.preview.is_none());
    assert!(a.core_runtime.health.is_empty());
    assert!(a.core_runtime.cleanup.absent.is_empty());
    assert!(!d.path().join("state/core-migration-active.json").exists());
}

mod common;
#[path = "common/core.rs"]
mod core_fixture;
fn fixture() -> (
    lqosync_v3::model::Settings,
    std::collections::BTreeMap<String, Arc<lqosync_v3::routeros::Inventory>>,
) {
    use lqosync_v3::{
        dynamic,
        wan_health::{Monitor, Probe},
    };
    let (s, mut inv, _, _) = core_fixture::fixture();
    let mut s = dynamic::migrate(&s).unwrap();
    s.core_control = Some(Config::default());
    let monitor = Monitor {
        wan: "wan".into(),
        probes: [
            Probe {
                id: "*AA".into(),
                target: "1.1.1.1".parse().unwrap(),
            },
            Probe {
                id: "*BB".into(),
                target: "8.8.8.8".parse().unwrap(),
            },
        ],
    };
    let i = Arc::make_mut(inv.get_mut("r1").unwrap());
    i.tables.get_mut("/system/resource/print").unwrap()[0].insert("uptime".into(), "1h".into());
    for path in ["/ppp/active/print", "/ip/hotspot/active/print"] {
        i.tables.get_mut(path).unwrap()[0]
            .extend([(".id".into(), "*1".into()), ("uptime".into(), "1h".into())]);
    }
    for p in &monitor.probes {
        i.tables
            .get_mut("/ip/route/print")
            .unwrap()
            .push(core_fixture::row(&[
                ("dst-address", &format!("{}/32", p.target)),
                ("routing-table", "main"),
                ("active", "true"),
                ("immediate-gw", "203.0.113.1%uplink"),
            ]));
    }
    i.tables.insert(
        "/tool/netwatch/print".into(),
        monitor
            .probes
            .iter()
            .map(|p| {
                core_fixture::row(&[
                    (".id", &p.id),
                    ("host", &p.target.to_string()),
                    ("type", "icmp"),
                    ("status", "up"),
                    ("done-tests", "1"),
                    ("interval", "10s"),
                    ("timeout", "1s"),
                ])
            })
            .collect(),
    );
    s.wan_health = Some(lqosync_v3::wan_health::Config {
        monitors: vec![monitor],
        ..Default::default()
    });
    (s, inv)
}
#[test]
fn completed_health_rounds_feed_preview_without_persisting_candidates() {
    use lqosync_v3::{core_reconcile::Decision, wan_health::HealthStatus};
    let (s, mut inv) = fixture();
    let d = tempfile::tempdir().unwrap();
    let mut state = core_console::Runtime::default();
    for n in 1..=5 {
        let now = 90 + n * 10;
        let i = Arc::make_mut(inv.get_mut("r1").unwrap());
        i.collected_at = now;
        for r in i.tables.get_mut("/tool/netwatch/print").unwrap() {
            r.insert("done-tests".into(), n.to_string());
        }
        state =
            core_console::compute(&s, &inv, &Default::default(), d.path(), &state, now).unwrap();
        assert!(state.preview.is_some(), "{}", state.status);
        if n == 1 {
            assert_eq!(state.health["wan"].status, HealthStatus::Unknown);
        }
    }
    assert_eq!(state.health["wan"].status, HealthStatus::Healthy);
    assert_eq!(state.ambiguous, 3);
    assert_eq!(
        state
            .preview
            .as_ref()
            .unwrap()
            .decisions
            .iter()
            .filter(|d| matches!(d, Decision::Add { .. }))
            .count(),
        3
    );
    Arc::make_mut(inv.get_mut("r1").unwrap())
        .tables
        .get_mut("/ip/firewall/address-list/print")
        .unwrap()
        .push(core_fixture::row(&[
            (".id", "*CC"),
            ("list", "lqos-wan"),
            ("address", "192.0.2.0/24"),
            ("comment", "manual protected"),
        ]));
    state = core_console::compute(&s, &inv, &Default::default(), d.path(), &state, 140).unwrap();
    assert_eq!(state.observed.get("wan"), Some(&3));
    assert_eq!(state.ambiguous, 0);
    assert!(
        state
            .preview
            .as_ref()
            .unwrap()
            .decisions
            .iter()
            .all(|d| matches!(d, Decision::Blocked { .. }))
    );
    assert_eq!(std::fs::read_dir(d.path()).unwrap().count(), 0);
    assert!(core_console::monitor_router(&s, "r1"));
    assert!(!core_console::monitor_router(&s, "other"));
    assert_eq!(core_console::interval(&s), 10);
}
#[test]
fn failed_source_keeps_health_diagnostics_but_discards_plan_and_cleanup() {
    let (s, mut inv) = fixture();
    inv.get_mut("r1")
        .map(Arc::make_mut)
        .unwrap()
        .tables
        .remove("/ppp/active/print");
    let d = tempfile::tempdir().unwrap();
    let state = core_console::compute(
        &s,
        &inv,
        &Default::default(),
        d.path(),
        &Default::default(),
        100,
    )
    .unwrap();
    assert!(state.health.contains_key("wan"));
    assert!(state.preview.is_none());
    assert!(state.cleanup.absent.is_empty());
    assert!(state.status.contains("blocked"));
}

#[test]
fn queued_preview_rejects_changed_settings_reads_and_read_errors() {
    use std::future::Future;
    for change in [
        "settings",
        "read",
        "error",
        "inventory",
        "pause",
        "external_pause",
    ] {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .max_blocking_threads(1)
            .build()
            .unwrap();
        runtime.block_on(async {
            let d = tempfile::tempdir().unwrap();
            let (settings, inventories) = fixture();
            let mut a = App::open(d.path().into()).unwrap();
            a.settings = settings;
            lqosync_v3::store::save_settings(d.path(), &a.settings).unwrap();
            a.inventories = inventories;
            let shared = Arc::new(Mutex::new(a));
            let (started_tx, started_rx) = tokio::sync::oneshot::channel();
            let (release_tx, release_rx) = std::sync::mpsc::channel();
            let blocker = tokio::task::spawn_blocking(move || {
                started_tx.send(()).unwrap();
                release_rx.recv().unwrap();
            });
            started_rx.await.unwrap();
            let work = core_console::refresh(shared.clone());
            tokio::pin!(work);
            assert!(
                std::future::poll_fn(|cx| std::task::Poll::Ready(
                    work.as_mut().poll(cx).is_pending()
                ))
                .await
            );
            {
                let mut a = shared.lock().await;
                match change {
                    "settings" => a.settings.interval_seconds += 1,
                    "read" => {
                        a.begin_read("r1");
                    }
                    "error" => {
                        a.inventory_errors.insert("r1".into(), "failed".into());
                    }
                    "inventory" => {
                        let inventory = (**a.inventories.get("r1").unwrap()).clone();
                        a.inventories.insert("r1".into(), Arc::new(inventory));
                    }
                    "pause" => a.settings.core_control.as_mut().unwrap().mode = Mode::Paused,
                    "external_pause" => {
                        let mut next = a.settings.clone();
                        next.core_control.as_mut().unwrap().mode = Mode::Paused;
                        lqosync_v3::store::save_settings(d.path(), &next).unwrap();
                    }
                    _ => unreachable!(),
                }
            }
            if change == "pause" {
                core_console::refresh(shared.clone()).await;
            }
            release_tx.send(()).unwrap();
            blocker.await.unwrap();
            work.await;
            let a = shared.lock().await;
            assert!(a.core_runtime.preview.is_none());
            assert!(a.core_runtime.health.is_empty());
            assert!(a.core_runtime.status.contains(if change == "pause" {
                "paused"
            } else if change == "external_pause" {
                "blocked"
            } else {
                "superseded"
            }));
        });
    }
}

#[test]
fn cancelled_preview_caller_does_not_leave_the_worker_permanently_busy() {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .max_blocking_threads(1)
        .build()
        .unwrap();
    runtime.block_on(async {
        let d = tempfile::tempdir().unwrap();
        let (settings, inventories) = fixture();
        let revision = core_console::revision(&settings).unwrap();
        let mut a = App::open(d.path().into()).unwrap();
        a.settings = settings;
        lqosync_v3::store::save_settings(d.path(), &a.settings).unwrap();
        a.inventories = inventories;
        let shared = Arc::new(Mutex::new(a));
        let (started_tx, started_rx) = tokio::sync::oneshot::channel();
        let (release_tx, release_rx) = std::sync::mpsc::channel();
        let blocker = tokio::task::spawn_blocking(move || {
            started_tx.send(()).unwrap();
            release_rx.recv().unwrap();
        });
        started_rx.await.unwrap();
        let work = tokio::spawn(core_console::refresh(shared.clone()));
        tokio::task::yield_now().await;
        work.abort();
        assert!(work.await.unwrap_err().is_cancelled());
        release_tx.send(()).unwrap();
        blocker.await.unwrap();
        tokio::time::timeout(std::time::Duration::from_secs(3), async {
            loop {
                if shared.lock().await.core_runtime.settings_revision == revision {
                    break;
                }
                tokio::task::yield_now().await;
            }
        })
        .await
        .expect("cancelled caller left preview stuck");
    });
}

#[tokio::test]
async fn pause_requires_current_revision_and_token_and_invalidates_shadow_ticket() {
    let d = tempfile::tempdir().unwrap();
    let mut a = App::open(d.path().into()).unwrap();
    a.settings.auto_stage = false;
    let token = a.token.clone();
    let revision = core_console::revision(&a.settings).unwrap();
    a.core_coordinator.configure(&revision, true);
    let ticket = a.core_coordinator.begin(d.path(), &revision).unwrap();
    let shared = Arc::new(Mutex::new(a));
    for (t, r, accepted) in [
        ("wrong", revision.as_str(), false),
        (token.as_str(), "stale", false),
        (token.as_str(), revision.as_str(), true),
    ] {
        let out = routes(shared.clone())
            .oneshot(
                Request::builder()
                    .header("host", "127.0.0.1:9303")
                    .method("POST")
                    .uri("/core-control/pause")
                    .header("content-type", "application/x-www-form-urlencoded")
                    .body(Body::from(format!("token={t}&revision={r}")))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(out.status().is_redirection(), accepted);
        assert_eq!(ticket.check(&revision).is_err(), accepted);
    }
    drop(ticket);
    drop(shared);
    let reopened = App::open(d.path().into()).unwrap();
    assert_eq!(reopened.settings.core_control.unwrap().mode, Mode::Paused);
    assert!(!d.path().join("state/core-migration-active.json").exists());
}
