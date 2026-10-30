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
    for lost in [false, true] {
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
        let server = tokio::spawn(async move {
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
        });
        std::fs::write(
            d.path().join("credentials.toml"),
            "[routers.r1]\npassword='synthetic'\n",
        )
        .unwrap();
        let (p, source) = prepared(s, inv, d.path(), "");
        let revision = p.revision().unwrap();
        let mut worker = p
            .open(
                d.path(),
                &revision,
                WireSource {
                    source,
                    db: db.clone(),
                },
            )
            .unwrap();
        if lost {
            assert!(worker.forward().await.is_err());
            assert!(worker.forward().await.is_err());
            assert!(worker.rollback().await.is_err());
            assert_eq!(commands.lock().await.len(), 1);
            drop(worker);
            assert!(lqosync_v3::web::App::open(d.path().into()).is_err());
        } else {
            worker.forward().await.unwrap();
            assert_eq!(db.lock().await.len(), 1);
            worker.rollback().await.unwrap();
            worker.finish().unwrap();
            assert!(db.lock().await.is_empty());
            assert!(!d.path().join("state/core-migration-active.json").exists());
            assert_eq!(commands.lock().await.len(), 2);
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
