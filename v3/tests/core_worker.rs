use lqosync_v3::core_worker::Approval;
#[test]
fn approval_starts_closed() {
    assert!(!Approval::default().active());
}

use lqosync_v3::{
    core_console::{self, Config},
    core_worker::{Prepared, RouterSource, Snapshot, Source},
    model::Settings,
    routeros::{Inventory, Row},
};
use std::{collections::BTreeMap, sync::Arc};
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
    for path in lqosync_v3::core_plan::EXTRA_TABLES
        .iter()
        .copied()
        .chain(["/interface/list/member/print"])
    {
        i.tables.entry(path.into()).or_default();
    }
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

fn warm(
    s: &Settings,
    inv: &mut BTreeMap<String, Arc<Inventory>>,
    root: &std::path::Path,
) -> core_console::Runtime {
    let mut state = core_console::Runtime::default();
    let base = lqosync_v3::orchestrator::now() - 40;
    for n in 1..=5 {
        let i = Arc::make_mut(inv.get_mut("r1").unwrap());
        i.collected_at = base + (n - 1) * 10;
        for r in i.tables.get_mut("/tool/netwatch/print").unwrap() {
            r.insert("done-tests".into(), n.to_string());
        }
        state = core_console::compute(
            s,
            inv,
            &Default::default(),
            root,
            &state,
            base + (n - 1) * 10,
        )
        .unwrap();
    }
    state
}

#[derive(Clone)]
struct MockSource {
    s: Settings,
    inv: BTreeMap<String, Arc<Inventory>>,
    root: std::path::PathBuf,
    previous: core_console::Runtime,
    change: String,
}
impl Source for MockSource {
    async fn capture(&mut self) -> anyhow::Result<Snapshot> {
        let i = Arc::make_mut(self.inv.get_mut("r1").unwrap());
        match self.change.as_str() {
            "settings" => self.s.interval_seconds += 1,
            "pause" => self.s.core_control.as_mut().unwrap().mode = core_console::Mode::Paused,
            "disconnect" => i.tables.get_mut("/ppp/active/print").unwrap().clear(),
            "reconnect" => {
                i.tables.get_mut("/ppp/active/print").unwrap()[0]
                    .insert(".id".into(), "*999".into());
            }
            "rates" => {
                i.tables.get_mut("/ppp/profile/print").unwrap()[0]
                    .insert("rate-limit".into(), "20M/60M".into());
            }
            "reboot" => {
                i.tables.get_mut("/system/resource/print").unwrap()[0]
                    .insert("uptime".into(), "1s".into());
            }
            "policy" => {
                i.tables
                    .get_mut("/ip/firewall/mangle/print")
                    .unwrap()
                    .push(core_fixture::row(&[
                        (".id", "*FF"),
                        ("chain", "prerouting"),
                        ("action", "accept"),
                    ]))
            }
            "stale" => i.collected_at -= 60,
            "error" => anyhow::bail!("synthetic collection error"),
            _ => {}
        }
        Snapshot::from_inventories(
            self.s.clone(),
            self.inv.clone(),
            &self.root,
            &self.previous,
            lqosync_v3::orchestrator::now(),
        )
    }
    async fn confirm(&mut self, revision: &str) -> anyhow::Result<()> {
        anyhow::ensure!(
            lqosync_v3::wan_health::settings_revision(&self.s)? == revision,
            "Settings changed after collection"
        );
        anyhow::ensure!(
            self.change != "late-revoke",
            "synthetic final enable revocation"
        );
        Ok(())
    }
}
fn prepared(
    s: Settings,
    mut inv: BTreeMap<String, Arc<Inventory>>,
    root: &std::path::Path,
    change: &str,
) -> (Prepared, MockSource) {
    let previous = warm(&s, &mut inv, root);
    let snapshot = Snapshot::from_inventories(
        s.clone(),
        inv.clone(),
        root,
        &previous,
        lqosync_v3::orchestrator::now(),
    )
    .unwrap();
    (
        Prepared::new(snapshot, 120).unwrap(),
        MockSource {
            s,
            inv,
            root: root.into(),
            previous,
            change: change.into(),
        },
    )
}
#[test]
fn exact_approval_root_and_exclusive_console_fence_are_required() {
    for condition in ["approval", "root", "console"] {
        let d = tempfile::tempdir().unwrap();
        let other = tempfile::tempdir().unwrap();
        let (s, inv) = fixture();
        let (p, source) = prepared(s, inv, d.path(), "");
        let revision = p.revision().unwrap();
        let console = if condition == "console" {
            Some(lqosync_v3::web::App::open(d.path().into()).unwrap())
        } else {
            None
        };
        let root = if condition == "root" {
            other.path()
        } else {
            d.path()
        };
        assert!(
            p.open(
                root,
                if condition == "approval" {
                    "wrong"
                } else {
                    &revision
                },
                source
            )
            .is_err()
        );
        assert!(!root.join("state/core-migration-active.json").exists());
        drop(console);
    }
}
#[test]
fn configured_batch_limit_rejects_before_creating_intent() {
    let d = tempfile::tempdir().unwrap();
    let (mut s, mut inv) = fixture();
    s.core_control.as_mut().unwrap().batch_size = 2;
    let previous = warm(&s, &mut inv, d.path());
    let snapshot =
        Snapshot::from_inventories(s, inv, d.path(), &previous, lqosync_v3::orchestrator::now())
            .unwrap();
    assert!(Prepared::new(snapshot, 120).is_err());
    assert!(!d.path().join("state/core-migration-active.json").exists());
}
#[tokio::test]
async fn source_drift_revocation_and_late_confirmation_fail_before_send() {
    for change in [
        "settings",
        "pause",
        "disconnect",
        "reconnect",
        "rates",
        "reboot",
        "policy",
        "stale",
        "error",
        "revoked",
        "late-revoke",
    ] {
        let d = tempfile::tempdir().unwrap();
        let (s, inv) = fixture();
        let (p, source) = prepared(s, inv, d.path(), change);
        let revision = p.revision().unwrap();
        let mut worker = p.open(d.path(), &revision, source).unwrap();
        let approval = worker.approval();
        if change == "revoked" {
            approval.revoke();
        }
        assert!(worker.forward().await.is_err(), "{change}");
        assert!(!approval.active(), "{change}");
        assert!(d.path().join("state/core-migration-active.json").exists());
        assert!(worker.forward().await.is_err());
        let path = std::fs::read_dir(d.path().join("state/core-writer"))
            .unwrap()
            .next()
            .unwrap()
            .unwrap()
            .path();
        let journal: serde_json::Value =
            serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap();
        assert!(journal["pending"].is_null());
    }
}
#[tokio::test]
async fn every_policy_mode_guards_session_handles() {
    for (balance, metric) in [
        (false, "online_clients"),
        (true, "online_clients"),
        (true, "planned_load"),
    ] {
        let d = tempfile::tempdir().unwrap();
        let (mut s, inv) = fixture();
        for policy in &mut s.dynamic.as_mut().unwrap().policies {
            policy.balance = balance;
            policy.metric = metric.into();
        }
        let (p, source) = prepared(s, inv, d.path(), "reconnect");
        let revision = p.revision().unwrap();
        let mut worker = p.open(d.path(), &revision, source).unwrap();
        let approval = worker.approval();
        assert!(worker.forward().await.is_err());
        assert!(!approval.active(), "{balance} {metric}");
    }
}

use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{TcpListener, TcpStream},
};
async fn sentence(s: &mut TcpStream) -> Option<Vec<String>> {
    let mut words = vec![];
    loop {
        let first = s.read_u8().await.ok()?;
        let size = if first < 128 {
            first as usize
        } else {
            (((first & 0x3f) as usize) << 8) | s.read_u8().await.ok()? as usize
        };
        if size == 0 {
            return Some(words);
        }
        let mut b = vec![0; size];
        s.read_exact(&mut b).await.ok()?;
        words.push(String::from_utf8(b).ok()?);
    }
}
async fn reply(s: &mut TcpStream, words: Vec<String>) {
    let mut b = vec![];
    for word in words {
        let n = word.len();
        assert!(n < 16384);
        if n < 128 {
            b.push(n as u8)
        } else {
            b.push(((n >> 8) as u8) | 0x80);
            b.push(n as u8)
        }
        b.extend(word.as_bytes());
    }
    b.push(0);
    let _ = s.write_all(&b).await;
}

#[derive(Clone)]
struct WireSource {
    source: MockSource,
    db: Arc<tokio::sync::Mutex<Vec<Row>>>,
}
impl Source for WireSource {
    async fn capture(&mut self) -> anyhow::Result<Snapshot> {
        Arc::make_mut(self.source.inv.get_mut("r1").unwrap())
            .tables
            .insert(
                "/ip/firewall/address-list/print".into(),
                self.db.lock().await.clone(),
            );
        self.source.capture().await
    }
    async fn confirm(&mut self, revision: &str) -> anyhow::Result<()> {
        self.source.confirm(revision).await
    }
}
#[tokio::test]
async fn scoped_wire_worker_verifies_add_rollback_and_freezes_lost_reply() {
    use lqosync_v3::core_worker::{Recovery, RecoveryAction};
    for (coordinated, lost, cancelled, recovery) in [
        (false, false, false, None),
        (false, true, false, None),
        (true, false, false, None),
        (true, true, false, None),
        (true, false, true, None),
        (false, false, false, Some(RecoveryAction::Rollback)),
        (false, true, false, Some(RecoveryAction::Rollback)),
        (true, false, false, Some(RecoveryAction::Rollback)),
        (false, false, false, Some(RecoveryAction::Forward)),
    ] {
        let recover = recovery.is_some();
        let d = tempfile::tempdir().unwrap();
        let (mut s, inv) = fixture();
        for selection in &mut s.selections {
            if selection.kind != "pppoe" {
                selection.exclude = true;
            }
        }
        let active: std::collections::BTreeSet<_> = s
            .selections
            .iter()
            .filter(|v| !v.exclude)
            .map(|v| v.id.clone())
            .collect();
        for policy in &mut s.dynamic.as_mut().unwrap().policies {
            policy.selections.retain(|id| active.contains(id));
        }
        s.dynamic
            .as_mut()
            .unwrap()
            .policies
            .retain(|p| !p.selections.is_empty());
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        s.routers[0].port = listener.local_addr().unwrap().port();
        let db = Arc::new(tokio::sync::Mutex::new(Vec::<Row>::new()));
        let remote = db.clone();
        let commands = Arc::new(tokio::sync::Mutex::new(Vec::<String>::new()));
        let calls = commands.clone();
        let sent = Arc::new(tokio::sync::Notify::new());
        let received = sent.clone();
        let server = tokio::spawn(async move {
            loop {
                let (mut stream, _) = listener.accept().await.unwrap();
                while let Some(words) = sentence(&mut stream).await {
                    let command = words[0].as_str();
                    let tag = words
                        .iter()
                        .find(|v| v.starts_with(".tag="))
                        .unwrap()
                        .clone();
                    let attrs: BTreeMap<String, String> = words
                        .iter()
                        .filter_map(|w| w.strip_prefix('=')?.split_once('='))
                        .map(|(a, b)| (a.into(), b.into()))
                        .collect();
                    match command {
                        "/login" => {}
                        "/ip/firewall/address-list/print" => {
                            for row in remote.lock().await.iter() {
                                let mut response = vec!["!re".into(), tag.clone()];
                                response.extend(row.iter().map(|(k, v)| format!("={k}={v}")));
                                reply(&mut stream, response).await;
                            }
                        }
                        "/ip/firewall/address-list/add" => {
                            calls.lock().await.push(command.into());
                            assert_eq!(
                                attrs.keys().map(String::as_str).collect::<Vec<_>>(),
                                vec!["address", "comment", "disabled", "list"]
                            );
                            remote.lock().await.push(core_fixture::row(&[
                                (".id", "*AC"),
                                ("address", &attrs["address"]),
                                ("list", &attrs["list"]),
                                ("comment", &attrs["comment"]),
                                ("disabled", "false"),
                                ("dynamic", "false"),
                            ]));
                            if cancelled {
                                received.notify_one();
                                std::future::pending::<()>().await;
                            }
                            if lost {
                                break;
                            }
                            reply(&mut stream, vec!["!done".into(), tag, "=ret=*AC".into()]).await;
                            continue;
                        }
                        "/ip/firewall/address-list/remove" => {
                            calls.lock().await.push(command.into());
                            assert_eq!(attrs.get(".id").map(String::as_str), Some("*AC"));
                            assert_eq!(attrs.len(), 1);
                            remote.lock().await.clear();
                        }
                        _ => panic!("Unscoped worker command {command}"),
                    }
                    reply(&mut stream, vec!["!done".into(), tag]).await;
                }
            }
        });
        std::fs::write(
            d.path().join("credentials.toml"),
            "[routers.r1]\npassword='synthetic'\n",
        )
        .unwrap();
        let (p, source) = prepared(s, inv, d.path(), "");
        let revision = p.revision().unwrap();
        if coordinated || recover {
            lqosync_v3::store::save_settings(d.path(), &source.s).unwrap();
        }
        let console = if coordinated {
            Some(lqosync_v3::web::App::open(d.path().into()).unwrap())
        } else {
            None
        };
        let source = WireSource {
            source,
            db: db.clone(),
        };
        let recovery_source = source.clone();
        let mut worker = if let Some(console) = &console {
            p.open_coordinated(d.path(), &revision, source, &console.core_coordinator)
        } else {
            p.open(d.path(), &revision, source)
        }
        .unwrap();
        let approval = worker.approval();
        if recovery == Some(RecoveryAction::Forward) {
            drop(worker);
            drop(console);
            let review = Recovery::review(
                d.path(),
                &revision,
                RecoveryAction::Forward,
                recovery_source,
                20,
            )
            .await
            .unwrap();
            let approved = review.revision().unwrap();
            let mut recovered = review.approve(&approved).await.unwrap();
            let grant = recovered.approval();
            assert!(recovered.rollback().await.is_err());
            recovered.forward().await.unwrap();
            recovered.finish().unwrap();
            assert!(!grant.active());
            assert_eq!(commands.lock().await.len(), 1);
            assert_eq!(db.lock().await.len(), 1);
            assert!(!d.path().join("state/core-migration-active.json").exists());
            assert!(!d.path().join("state/current.toml").exists());
            server.abort();
            continue;
        }
        if cancelled {
            {
                let work = worker.forward();
                tokio::pin!(work);
                tokio::select! {
                    out = &mut work => panic!("Request completed before cancellation: {out:?}"),
                    _ = sent.notified() => {},
                    _ = tokio::time::sleep(std::time::Duration::from_secs(5)) => panic!("No mock request received"),
                }
            }
        }
        if lost || cancelled {
            if lost {
                assert!(worker.forward().await.is_err());
            }
            assert!(worker.forward().await.is_err());
            assert!(worker.rollback().await.is_err());
            assert_eq!(commands.lock().await.len(), 1);
            drop(worker);
            assert!(lqosync_v3::web::App::open(d.path().into()).is_err());
            if recover {
                use lqosync_v3::core_worker::{Recovery, RecoveryAction};
                drop(console);
                let error = Recovery::review(
                    d.path(),
                    &revision,
                    RecoveryAction::Rollback,
                    recovery_source,
                    20,
                )
                .await
                .err()
                .unwrap();
                assert!(error.to_string().contains("Unknown"), "{error}");
                assert_eq!(commands.lock().await.len(), 1);
            }
        } else {
            worker.forward().await.unwrap();
            assert_eq!(db.lock().await.len(), 1);
            if recover {
                use lqosync_v3::core_worker::{Recovery, RecoveryAction};
                drop(worker);
                drop(console);
                let review = Recovery::review(
                    d.path(),
                    &revision,
                    RecoveryAction::Rollback,
                    recovery_source,
                    20,
                )
                .await
                .unwrap();
                let approved = review.revision().unwrap();
                let mut recovered = review.approve(&approved).await.unwrap();
                let grant = recovered.approval();
                assert!(recovered.forward().await.is_err());
                recovered.rollback().await.unwrap();
                recovered.finish().unwrap();
                assert!(!grant.active());
            } else {
                worker.rollback().await.unwrap();
                worker.finish().unwrap();
            }
            assert!(db.lock().await.is_empty());
            assert!(!d.path().join("state/core-migration-active.json").exists());
            assert_eq!(commands.lock().await.len(), 2);
        }
        if coordinated {
            assert!(!approval.active());
        }
        assert!(!d.path().join("state/current.toml").exists());
        server.abort();
    }
}
#[tokio::test]
async fn fresh_router_source_reads_netwatch_and_rejects_mid_collection_settings_drift() {
    for changed in [false, true] {
        let d = tempfile::tempdir().unwrap();
        let (mut settings, inv) = fixture();
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        settings.routers[0].port = listener.local_addr().unwrap().port();
        settings.auto_stage = false;
        lqosync_v3::store::save_settings(d.path(), &settings).unwrap();
        std::fs::write(
            d.path().join("credentials.toml"),
            "[routers.r1]\npassword='synthetic'\n",
        )
        .unwrap();
        let root = d.path().to_path_buf();
        let mut updated = settings.clone();
        updated.interval_seconds += 1;
        let tables = inv["r1"].tables.clone();
        let during = updated.clone();
        let server = tokio::spawn(async move {
            let (mut stream, _) = listener.accept().await.unwrap();
            let login = sentence(&mut stream).await.unwrap();
            let tag = login
                .iter()
                .find(|s| s.starts_with(".tag="))
                .unwrap()
                .clone();
            reply(&mut stream, vec!["!done".into(), tag]).await;
            let mut paths = vec![];
            for _ in 0..(14 + lqosync_v3::core_plan::EXTRA_TABLES.len() + 1) {
                let command = sentence(&mut stream).await.unwrap();
                assert!(command[0].ends_with("/print"));
                paths.push(command[0].clone());
                let tag = command
                    .iter()
                    .find(|s| s.starts_with(".tag="))
                    .unwrap()
                    .clone();
                if let Some(rows) = tables.get(&command[0]) {
                    for row in rows {
                        let mut words = vec!["!re".into(), tag.clone()];
                        words.extend(row.iter().map(|(k, v)| format!("={k}={v}")));
                        reply(&mut stream, words).await;
                    }
                }
                if changed && command[0] == lqosync_v3::wan_health::NETWATCH {
                    lqosync_v3::store::save_settings(&root, &during).unwrap();
                }
                reply(&mut stream, vec!["!done".into(), tag]).await;
            }
            paths
        });
        let mut source = RouterSource::new(d.path()).unwrap();
        let result = source.capture().await;
        assert_eq!(result.is_err(), changed);
        if let Ok(snapshot) = result {
            assert!(snapshot.runtime().health.contains_key("wan"));
            assert!(snapshot.runtime().preview.is_none());
        }
        assert!(
            server
                .await
                .unwrap()
                .iter()
                .any(|p| p == lqosync_v3::wan_health::NETWATCH)
        );
        assert!(!d.path().join("state/current.toml").exists());
        assert!(!d.path().join("state/core-migration-active.json").exists());
        if !changed {
            lqosync_v3::store::save_settings(d.path(), &updated).unwrap();
            assert!(
                source
                    .confirm(&lqosync_v3::wan_health::settings_revision(&settings).unwrap())
                    .await
                    .is_err()
            );
        }
    }
}

#[tokio::test]
async fn coordinated_open_coexists_with_console_but_pause_is_final_for_that_worker() {
    let d = tempfile::tempdir().unwrap();
    let (s, inv) = fixture();
    let settings_revision = core_console::revision(&s).unwrap();
    let (p, source) = prepared(s, inv, d.path(), "");
    let revision = p.revision().unwrap();
    let a = lqosync_v3::web::App::open(d.path().into()).unwrap();
    lqosync_v3::store::save_settings(d.path(), &source.s).unwrap();
    let coordinator = a.core_coordinator.clone();
    coordinator.configure(&settings_revision, true);
    let mut worker = p
        .open_coordinated(d.path(), &revision, source, &coordinator)
        .unwrap();
    assert!(worker.approval().active());
    assert!(lqosync_v3::migration_fence::Lease::exclusive(d.path()).is_err());
    assert!(coordinator.begin(d.path(), &settings_revision).is_err());
    coordinator.configure(&settings_revision, false);
    assert!(!worker.approval().active());
    coordinator.configure(&settings_revision, true);
    assert!(!worker.approval().active());
    assert!(worker.forward().await.is_err());
    assert!(worker.rollback().await.is_err());
    assert!(worker.finish().is_err());
    assert!(d.path().join("state/core-migration-active.json").exists());
    assert!(lqosync_v3::web::App::open(d.path().into()).is_err());
}

#[test]
fn coordinated_open_rejects_foreign_scope_busy_slot_and_prior_records_without_replacing_them() {
    for change in [
        "batch",
        "root",
        "paused",
        "settings",
        "busy",
        "journal",
        "marker",
        "persisted_pause",
    ] {
        let d = tempfile::tempdir().unwrap();
        let other = tempfile::tempdir().unwrap();
        let (s, inv) = fixture();
        let (p, source) = prepared(s, inv, d.path(), "");
        let revision = p.revision().unwrap();
        let settings_revision = core_console::revision(&source.s).unwrap();
        lqosync_v3::store::save_settings(d.path(), &source.s).unwrap();
        let a = lqosync_v3::web::App::open(d.path().into()).unwrap();
        let marker = d.path().join("state/core-migration-active.json");
        let journal = d
            .path()
            .join("state/core-writer")
            .join(format!("{revision}.json"));
        let mut busy = None;
        match change {
            "paused" => a.core_coordinator.configure(&settings_revision, false),
            "settings" => a.core_coordinator.configure("another", true),
            "busy" => {
                busy = Some(
                    a.core_coordinator
                        .begin(d.path(), &settings_revision)
                        .unwrap(),
                )
            }
            "journal" => {
                std::fs::create_dir_all(journal.parent().unwrap()).unwrap();
                std::fs::write(&journal, "preserve journal").unwrap();
            }
            "marker" => std::fs::write(&marker, "preserve marker").unwrap(),
            "persisted_pause" => {
                let mut next = source.s.clone();
                next.core_control.as_mut().unwrap().mode = core_console::Mode::Paused;
                lqosync_v3::store::save_settings(d.path(), &next).unwrap();
            }
            _ => {}
        }
        let before = std::fs::read(&marker).ok();
        let journal_before = std::fs::read(&journal).ok();
        let root = if change == "root" {
            other.path()
        } else {
            d.path()
        };
        assert!(
            p.open_coordinated(
                root,
                if change == "batch" {
                    "wrong"
                } else {
                    &revision
                },
                source,
                &a.core_coordinator
            )
            .is_err(),
            "{change}"
        );
        assert_eq!(std::fs::read(&marker).ok(), before, "{change}");
        assert_eq!(std::fs::read(&journal).ok(), journal_before, "{change}");
        drop(busy);
    }
}

struct PausingSource {
    inner: MockSource,
    coordinator: lqosync_v3::core_coordination::Coordinator,
    settings_revision: String,
    phase: &'static str,
}
impl Source for PausingSource {
    async fn capture(&mut self) -> anyhow::Result<Snapshot> {
        let result = self.inner.capture().await?;
        tokio::task::yield_now().await;
        if self.phase == "capture" {
            self.coordinator.configure(&self.settings_revision, false);
        }
        Ok(result)
    }
    async fn confirm(&mut self, revision: &str) -> anyhow::Result<()> {
        self.inner.confirm(revision).await?;
        tokio::task::yield_now().await;
        if self.phase == "confirm" {
            self.coordinator.configure(&self.settings_revision, false);
        }
        Ok(())
    }
}
#[tokio::test]
async fn coordinated_pause_during_awaited_verification_prevents_any_connection_or_send() {
    for phase in ["capture", "confirm"] {
        let d = tempfile::tempdir().unwrap();
        let (mut s, inv) = fixture();
        // An unused loopback listener detects any unexpected connection attempt.
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        s.routers[0].port = listener.local_addr().unwrap().port();
        let (p, inner) = prepared(s, inv, d.path(), "");
        lqosync_v3::store::save_settings(d.path(), &inner.s).unwrap();
        let a = lqosync_v3::web::App::open(d.path().into()).unwrap();
        let source = PausingSource {
            settings_revision: core_console::revision(&inner.s).unwrap(),
            inner,
            coordinator: a.core_coordinator.clone(),
            phase,
        };
        let revision = p.revision().unwrap();
        let mut worker = p
            .open_coordinated(d.path(), &revision, source, &a.core_coordinator)
            .unwrap();
        let error = worker.forward().await.unwrap_err().to_string();
        assert!(error.contains("revoked"), "{phase}: {error}");
        assert!(!worker.approval().active());
        assert!(
            tokio::time::timeout(std::time::Duration::from_millis(20), listener.accept())
                .await
                .is_err()
        );
        let report = lqosync_v3::core_recovery::inspect(d.path()).unwrap();
        assert_eq!(report.journals[0].completed, 0);
        assert!(report.journals[0].pending.is_none());
        drop(worker);
        assert!(d.path().join("state/core-migration-active.json").exists());
    }
}

#[test]
fn coordinated_drop_revokes_detached_approval_and_keeps_recovery_fenced() {
    let d = tempfile::tempdir().unwrap();
    let (s, inv) = fixture();
    let (p, source) = prepared(s, inv, d.path(), "");
    lqosync_v3::store::save_settings(d.path(), &source.s).unwrap();
    let a = lqosync_v3::web::App::open(d.path().into()).unwrap();
    let revision = p.revision().unwrap();
    let worker = p
        .open_coordinated(d.path(), &revision, source, &a.core_coordinator)
        .unwrap();
    let approval = worker.approval();
    assert!(approval.active());
    drop(worker);
    assert!(!approval.active());
    assert!(
        a.core_coordinator
            .begin(d.path(), &core_console::revision(&a.settings).unwrap())
            .is_err()
    );
    assert!(d.path().join("state/core-migration-active.json").exists());
}

#[tokio::test]
async fn another_console_durable_pause_revokes_worker_without_a_router_request() {
    let d = tempfile::tempdir().unwrap();
    let (s, inv) = fixture();
    let (p, source) = prepared(s, inv, d.path(), "");
    lqosync_v3::store::save_settings(d.path(), &source.s).unwrap();
    let a = lqosync_v3::web::App::open(d.path().into()).unwrap();
    let revision = p.revision().unwrap();
    let mut worker = p
        .open_coordinated(d.path(), &revision, source, &a.core_coordinator)
        .unwrap();
    let remote = lqosync_v3::core_coordination::Coordinator::for_root(d.path()).unwrap();
    remote.configure(&core_console::revision(&a.settings).unwrap(), true);
    remote.pause_all().unwrap();
    assert!(!worker.approval().active());
    assert!(worker.forward().await.is_err());
    assert!(worker.rollback().await.is_err());
    assert!(d.path().join("state/core-migration-active.json").exists());
}

#[tokio::test]
async fn persisted_guard_allows_exact_read_only_recovery_review_after_drop() {
    use lqosync_v3::core_worker::{Recovery, RecoveryAction};
    let d = tempfile::tempdir().unwrap();
    let (s, inv) = fixture();
    let (p, source) = prepared(s, inv, d.path(), "");
    lqosync_v3::store::save_settings(d.path(), &source.s).unwrap();
    let revision = p.revision().unwrap();
    let resumed_source = MockSource {
        s: source.s.clone(),
        inv: source.inv.clone(),
        root: source.root.clone(),
        previous: source.previous.clone(),
        change: String::new(),
    };
    drop(p.open(d.path(), &revision, source).unwrap());
    let path = d.path().join(format!("state/core-worker/{revision}.json"));
    assert!(path.exists());
    let journal = d.path().join(format!("state/core-writer/{revision}.json"));
    let before = std::fs::read(&journal).unwrap();
    let review = Recovery::review(
        d.path(),
        &revision,
        RecoveryAction::Forward,
        resumed_source,
        20,
    )
    .await
    .unwrap();
    assert_ne!(review.revision().unwrap(), revision);
    assert_eq!(review.proposal().approval, review.revision().unwrap());
    assert_eq!(review.proposal().action, RecoveryAction::Forward);
    assert!(review.proposal().assessment.next_forward.is_some());
    assert_eq!(std::fs::read(&journal).unwrap(), before);
    drop(review);
    assert!(d.path().join("state/core-migration-active.json").exists());
}

#[tokio::test]
async fn recovery_rejects_missing_corrupt_foreign_and_unknown_records_without_writes() {
    use lqosync_v3::core_worker::{Recovery, RecoveryAction};
    for change in [
        "manifest_missing",
        "manifest_symlink",
        "manifest_schema",
        "manifest_guard",
        "journal_unknown",
        "marker_missing",
        "console",
        "root",
    ] {
        let d = tempfile::tempdir().unwrap();
        let other = tempfile::tempdir().unwrap();
        let (s, inv) = fixture();
        let (p, source) = prepared(s, inv, d.path(), "");
        lqosync_v3::store::save_settings(d.path(), &source.s).unwrap();
        let revision = p.revision().unwrap();
        let copy = source.clone();
        drop(p.open(d.path(), &revision, source).unwrap());
        let manifest = d.path().join(format!("state/core-worker/{revision}.json"));
        let journal = d.path().join(format!("state/core-writer/{revision}.json"));
        let marker = d.path().join("state/core-migration-active.json");
        match change {
            "manifest_missing" => std::fs::remove_file(&manifest).unwrap(),
            "manifest_symlink" => {
                std::fs::remove_file(&manifest).unwrap();
                let target = other.path().join("guard.json");
                std::fs::write(&target, "synthetic private record").unwrap();
                std::os::unix::fs::symlink(target, &manifest).unwrap();
            }
            "manifest_schema" | "manifest_guard" => {
                let mut m: serde_json::Value =
                    serde_json::from_slice(&std::fs::read(&manifest).unwrap()).unwrap();
                if change == "manifest_schema" {
                    m["schema"] = 9.into();
                } else {
                    m["guard"]["revision"] = "changed".into();
                }
                std::fs::write(&manifest, serde_json::to_vec(&m).unwrap()).unwrap();
            }
            "journal_unknown" => {
                let mut j: serde_json::Value =
                    serde_json::from_slice(&std::fs::read(&journal).unwrap()).unwrap();
                j["pending"] = j["batch"]["operations"][0].clone();
                std::fs::write(&journal, serde_json::to_vec(&j).unwrap()).unwrap();
            }
            "marker_missing" => std::fs::remove_file(&marker).unwrap(),
            _ => {}
        }
        // A raw shared lease models an already-running console despite the recovery marker.
        let lock = if change == "console" {
            Some(
                std::fs::OpenOptions::new()
                    .read(true)
                    .write(true)
                    .open(d.path().join("state/application-writer.lock"))
                    .unwrap(),
            )
        } else {
            None
        };
        if let Some(lock) = &lock {
            use std::os::fd::AsRawFd;
            // SAFETY: lock owns a live descriptor; flock retains no Rust pointer.
            assert_eq!(
                unsafe { libc::flock(lock.as_raw_fd(), libc::LOCK_SH | libc::LOCK_NB) },
                0
            );
        }
        let before = std::fs::read(&journal).unwrap();
        let marker_before = std::fs::read(&marker).ok();
        assert!(
            Recovery::review(
                if change == "root" {
                    other.path()
                } else {
                    d.path()
                },
                &revision,
                RecoveryAction::Forward,
                copy,
                20
            )
            .await
            .is_err(),
            "{change}"
        );
        assert_eq!(std::fs::read(&journal).unwrap(), before);
        assert_eq!(std::fs::read(&marker).ok(), marker_before);
        drop(lock);
    }
}

#[tokio::test]
async fn recovery_approval_binds_exact_bytes_scope_expiry_and_pause() {
    use lqosync_v3::core_worker::{Recovery, RecoveryAction};
    for change in [
        "approval",
        "journal",
        "manifest",
        "pause",
        "expiry",
        "direction",
    ] {
        let d = tempfile::tempdir().unwrap();
        let (s, inv) = fixture();
        let (p, source) = prepared(s, inv, d.path(), "");
        lqosync_v3::store::save_settings(d.path(), &source.s).unwrap();
        let revision = p.revision().unwrap();
        let copy = source.clone();
        let settings_revision = core_console::revision(&source.s).unwrap();
        drop(p.open(d.path(), &revision, source).unwrap());
        let review = Recovery::review(
            d.path(),
            &revision,
            RecoveryAction::Forward,
            copy,
            if change == "expiry" { 1 } else { 20 },
        )
        .await
        .unwrap();
        let approved = review.revision().unwrap();
        let journal = d.path().join(format!("state/core-writer/{revision}.json"));
        let manifest = d.path().join(format!("state/core-worker/{revision}.json"));
        match change {
            "journal" | "manifest" => {
                use std::io::Write;
                std::fs::OpenOptions::new()
                    .append(true)
                    .open(if change == "journal" {
                        &journal
                    } else {
                        &manifest
                    })
                    .unwrap()
                    .write_all(b" ")
                    .unwrap();
            }
            "pause" => {
                let c = lqosync_v3::core_coordination::Coordinator::for_root(d.path()).unwrap();
                c.configure(&settings_revision, true);
                c.pause_all().unwrap();
            }
            "expiry" => tokio::time::sleep(std::time::Duration::from_millis(2100)).await,
            _ => {}
        }
        let before = std::fs::read(&journal).unwrap();
        let result = review
            .approve(if change == "approval" {
                &revision
            } else {
                &approved
            })
            .await;
        if change == "direction" {
            let mut worker = result.unwrap();
            let grant = worker.approval();
            assert!(worker.rollback().await.is_err());
            drop(worker);
            assert!(!grant.active());
        } else {
            assert!(result.is_err(), "{change}");
        }
        assert_eq!(std::fs::read(&journal).unwrap(), before);
        assert!(d.path().join("state/core-migration-active.json").exists());
    }
}

#[derive(Clone)]
struct ChangingRecoverySource {
    inner: MockSource,
    signal: Arc<std::sync::Mutex<String>>,
}
impl Source for ChangingRecoverySource {
    async fn capture(&mut self) -> anyhow::Result<Snapshot> {
        self.inner.change = self.signal.lock().unwrap().clone();
        self.inner.capture().await
    }
    async fn confirm(&mut self, revision: &str) -> anyhow::Result<()> {
        self.inner.confirm(revision).await
    }
}
#[tokio::test]
async fn recovery_recollects_sources_before_approval() {
    use lqosync_v3::core_worker::{Recovery, RecoveryAction};
    for change in [
        "settings",
        "reconnect",
        "rates",
        "policy",
        "reboot",
        "error",
        "late-revoke",
    ] {
        let d = tempfile::tempdir().unwrap();
        let (s, inv) = fixture();
        let (p, source) = prepared(s, inv, d.path(), "");
        lqosync_v3::store::save_settings(d.path(), &source.s).unwrap();
        let revision = p.revision().unwrap();
        let copy = source.clone();
        drop(p.open(d.path(), &revision, source).unwrap());
        let signal = Arc::new(std::sync::Mutex::new(String::new()));
        let review = Recovery::review(
            d.path(),
            &revision,
            RecoveryAction::Forward,
            ChangingRecoverySource {
                inner: copy,
                signal: signal.clone(),
            },
            20,
        )
        .await
        .unwrap();
        let approved = review.revision().unwrap();
        *signal.lock().unwrap() = change.into();
        let path = d.path().join(format!("state/core-writer/{revision}.json"));
        let before = std::fs::read(&path).unwrap();
        assert!(review.approve(&approved).await.is_err(), "{change}");
        assert_eq!(std::fs::read(&path).unwrap(), before);
    }
}

#[tokio::test]
async fn read_only_router_source_does_not_recreate_missing_settings() {
    let d = tempfile::tempdir().unwrap();
    let mut source = RouterSource::new(d.path()).unwrap();
    assert!(source.preview().await.is_err());
    assert!(!d.path().join("settings.toml").exists());
    assert_eq!(std::fs::read_dir(d.path()).unwrap().count(), 0);
}
