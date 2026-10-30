use lqosync_v3::{core_adapter, migration_fence::Lease, web::App};
#[test]
fn exclusive_lease_excludes_consoles_and_other_maintenance() {
    let dir = tempfile::tempdir().unwrap();
    let console = App::open(dir.path().into()).unwrap();
    assert!(Lease::maintenance(dir.path(), "journal-a", "revision-a").is_err());
    drop(console);
    let lease = Lease::maintenance(dir.path(), "journal-a", "revision-a").unwrap();
    assert!(App::open(dir.path().into()).is_err());
    assert!(Lease::maintenance(dir.path(), "journal-b", "revision-b").is_err());
    drop(lease);
    assert!(
        App::open(dir.path().into()).is_err(),
        "recovery marker must outlive a crashed migration"
    );
    let lease = Lease::maintenance(dir.path(), "journal-a", "revision-a").unwrap();
    lease.finish().unwrap();
    assert!(App::open(dir.path().into()).is_ok());
}
#[test]
fn real_session_bootstrap_preserves_settings_and_requires_matching_manual_pins() {
    let (mut settings, mut inputs, _, _) = core_fixture::fixture();
    settings = lqosync_v3::dynamic::migrate(&settings).unwrap();
    for p in &mut settings.dynamic.as_mut().unwrap().policies {
        p.balance = true;
        p.metric = "online_clients".into();
    }
    let now = lqosync_v3::orchestrator::now();
    let inv = std::sync::Arc::make_mut(inputs.get_mut("r1").unwrap());
    inv.collected_at = now;
    for path in ["/ppp/active/print", "/ip/hotspot/active/print"] {
        for r in inv.tables.get_mut(path).unwrap() {
            r.insert(".id".into(), "*A".into());
            r.insert("uptime".into(), "10m".into());
        }
    }
    inv.tables
        .get_mut("/ip/firewall/address-list/print")
        .unwrap()
        .push(core_fixture::row(&[
            (".id", "*F"),
            ("list", "lqos-wan"),
            ("address", "0.0.0.0/0"),
            ("comment", "manual"),
        ]));
    let before = settings.clone();
    let dir = tempfile::tempdir().unwrap();
    let seed = core_adapter::bootstrap(&settings, &inputs, dir.path()).unwrap();
    assert_eq!(settings, before);
    assert!(!seed.observation.active.is_empty());
    assert_eq!(seed.rows.len(), 3);
    assert!(
        seed.observation
            .active
            .values()
            .filter(|s| s.service != "dhcp")
            .all(|s| s.id.as_deref() == Some("*A") && s.uptime == Some(600))
    );
}
mod common;
#[path = "common/core.rs"]
mod core_fixture;

use std::{collections::BTreeMap, sync::Arc};
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
struct Wire {
    tables: BTreeMap<String, Vec<lqosync_v3::routeros::Row>>,
    writes: Vec<String>,
    next: usize,
    lose_add: bool,
    omit_id: bool,
    tag_first: bool,
    reject_pending_read: Option<std::path::PathBuf>,
}
async fn mock_router(
    lose_add: bool,
) -> (
    tempfile::TempDir,
    Arc<tokio::sync::Mutex<Wire>>,
    tokio::task::JoinHandle<()>,
) {
    let (mut settings, mut inv, _, _) = core_fixture::fixture();
    settings = lqosync_v3::dynamic::migrate(&settings).unwrap();
    for p in &mut settings.dynamic.as_mut().unwrap().policies {
        p.balance = true;
        p.metric = "online_clients".into();
    }
    let inv = Arc::make_mut(inv.get_mut("r1").unwrap());
    for path in ["/ppp/active/print", "/ip/hotspot/active/print"] {
        for row in inv.tables.get_mut(path).unwrap() {
            row.insert(".id".into(), "*A".into());
            row.insert("uptime".into(), "10m".into());
        }
    }
    inv.tables
        .get_mut("/ip/firewall/address-list/print")
        .unwrap()
        .push(core_fixture::row(&[
            (".id", "*F"),
            ("list", "lqos-wan"),
            ("address", "0.0.0.0/0"),
            ("comment", "protected manual"),
        ]));
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    settings.routers[0].port = listener.local_addr().unwrap().port();
    settings.routers[0].timeout_seconds = 1;
    let root = tempfile::tempdir().unwrap();
    lqosync_v3::store::save_settings(root.path(), &settings).unwrap();
    std::fs::write(
        root.path().join("credentials.toml"),
        "[routers.r1]\npassword='synthetic'\n",
    )
    .unwrap();
    let state = Arc::new(tokio::sync::Mutex::new(Wire {
        tables: inv.tables.clone(),
        writes: vec![],
        next: 256,
        lose_add,
        omit_id: false,
        tag_first: false,
        reject_pending_read: None,
    }));
    let shared = state.clone();
    let task = tokio::spawn(async move {
        loop {
            let Ok((mut s, _)) = listener.accept().await else {
                break;
            };
            let state = shared.clone();
            s.set_nodelay(true).unwrap();
            tokio::spawn(async move {
                while let Some(words) = sentence(&mut s).await {
                    let tag = words
                        .iter()
                        .find(|w| w.starts_with(".tag="))
                        .unwrap()
                        .clone();
                    let command = words[0].as_str();
                    if command == "/login" {
                        reply(&mut s, vec!["!done".into(), tag]).await;
                        continue;
                    }
                    let mut db = state.lock().await;
                    if command.ends_with("/print") {
                        let reject = db.reject_pending_read.as_ref().is_some_and(|p| {
                            std::fs::read_to_string(p)
                                .ok()
                                .and_then(|s| serde_json::from_str::<serde_json::Value>(&s).ok())
                                .is_some_and(|v| !v["pending"].is_null())
                        });
                        if reject {
                            db.reject_pending_read = None;
                            drop(db);
                            reply(
                                &mut s,
                                vec![
                                    "!trap".into(),
                                    "=message=injected preflight read failure".into(),
                                    tag,
                                ],
                            )
                            .await;
                            continue;
                        }

                        let rows = db.tables.get(command).cloned().unwrap_or_default();
                        drop(db);
                        for row in rows {
                            let mut words = vec!["!re".into(), tag.clone()];
                            words.extend(row.into_iter().map(|(k, v)| format!("={k}={v}")));
                            reply(&mut s, words).await;
                        }
                        reply(&mut s, vec!["!done".into(), tag]).await;
                        continue;
                    }
                    let attrs = words
                        .iter()
                        .filter_map(|w| {
                            w.strip_prefix('=')
                                .and_then(|v| v.split_once('='))
                                .map(|(k, v)| (k.to_string(), v.to_string()))
                        })
                        .collect::<BTreeMap<_, _>>();
                    db.writes.push(command.into());
                    let mut ret = None;
                    match command {
                        "/ip/firewall/address-list/add" => {
                            let mut row = attrs;
                            row.insert("disabled".into(), "false".into());
                            row.insert("dynamic".into(), "false".into());
                            let id = format!("*{:X}", db.next);
                            db.next += 1;
                            row.insert(".id".into(), id.clone());
                            db.tables
                                .get_mut("/ip/firewall/address-list/print")
                                .unwrap()
                                .push(row);
                            if db.lose_add {
                                return;
                            }
                            ret = Some(id);
                        }
                        "/ip/firewall/address-list/set" => {
                            let row = db
                                .tables
                                .get_mut("/ip/firewall/address-list/print")
                                .unwrap()
                                .iter_mut()
                                .find(|r| r[".id"] == attrs[".id"])
                                .unwrap();
                            row.insert("address".into(), attrs["address"].clone());
                        }
                        "/ip/firewall/address-list/remove" => db
                            .tables
                            .get_mut("/ip/firewall/address-list/print")
                            .unwrap()
                            .retain(|r| r[".id"] != attrs[".id"]),
                        _ => panic!("Unexpected mutation {command}"),
                    }
                    let omit_id = db.omit_id;
                    let tag_first = db.tag_first;
                    drop(db);
                    let mut done = vec!["!done".into()];
                    if tag_first {
                        done.push(tag.clone());
                    }
                    if let Some(id) = ret.filter(|_| !omit_id) {
                        done.push(format!("=ret={id}"));
                    }
                    if !tag_first {
                        done.push(tag);
                    }
                    reply(&mut s, done).await;
                }
            });
        }
    });
    (root, state, task)
}
#[tokio::test]
async fn production_wire_adapter_bootstraps_applies_validates_and_rolls_back() {
    let (root, state, task) = mock_router(false).await;
    let original = state.lock().await.tables["/ip/firewall/address-list/print"].clone();
    let prepared = core_adapter::prepare(
        root.path(),
        vec![lqosync_v3::core_migration::RowKey {
            router: "r1".into(),
            id: "*F".into(),
        }],
    )
    .await
    .unwrap();
    let revision = prepared.revision().unwrap();
    let ack = format!("EXCLUSIVE_ROUTER_ACCESS:{revision}");
    assert!(
        core_adapter::execute(root.path(), &prepared, "wrong", &ack, false)
            .await
            .is_err()
    );
    assert!(state.lock().await.writes.is_empty());
    core_adapter::execute(root.path(), &prepared, &revision, &ack, false)
        .await
        .unwrap();
    assert!(App::open(root.path().into()).is_ok());
    assert_eq!(state.lock().await.writes.len(), 5);
    let candidate = lqosync_v3::sessions::load(root.path()).unwrap().unwrap();
    assert_eq!(candidate.active.len(), 3);
    assert!(
        candidate
            .active
            .values()
            .filter(|s| s.service != "dhcp")
            .all(|s| s.id.as_deref() == Some("*A"))
    );
    state.lock().await.reject_pending_read = Some(
        root.path()
            .join(format!("state/core-migrations/{revision}.json")),
    );
    assert!(
        core_adapter::execute(root.path(), &prepared, &revision, &ack, true)
            .await
            .is_err()
    );
    assert_eq!(state.lock().await.writes.len(), 5);
    core_adapter::execute(root.path(), &prepared, &revision, &ack, true)
        .await
        .unwrap();
    let actual = state.lock().await.tables["/ip/firewall/address-list/print"].clone();
    assert_eq!(actual, original);
    task.abort();
}
#[tokio::test]
async fn uncertain_network_add_keeps_marker_and_never_retries() {
    let (root, state, task) = mock_router(true).await;
    let prepared = core_adapter::prepare(
        root.path(),
        vec![lqosync_v3::core_migration::RowKey {
            router: "r1".into(),
            id: "*F".into(),
        }],
    )
    .await
    .unwrap();
    let revision = prepared.revision().unwrap();
    let ack = format!("EXCLUSIVE_ROUTER_ACCESS:{revision}");
    assert!(
        core_adapter::execute(root.path(), &prepared, &revision, &ack, false)
            .await
            .is_err()
    );
    assert!(App::open(root.path().into()).is_err());
    assert_eq!(state.lock().await.writes.len(), 1);
    assert!(
        core_adapter::execute(root.path(), &prepared, &revision, &ack, false)
            .await
            .is_err()
    );
    assert!(
        core_adapter::execute(root.path(), &prepared, &revision, &ack, true)
            .await
            .is_err()
    );
    assert_eq!(state.lock().await.writes.len(), 1);
    task.abort();
}

#[tokio::test]
async fn interrupted_bootstrap_recovers_from_either_exact_candidate_state() {
    for saved in [false, true] {
        let (root, state, task) = mock_router(false).await;
        let prepared = core_adapter::prepare(
            root.path(),
            vec![lqosync_v3::core_migration::RowKey {
                router: "r1".into(),
                id: "*F".into(),
            }],
        )
        .await
        .unwrap();
        let revision = prepared.revision().unwrap();
        let value = serde_json::to_value(&prepared).unwrap();
        let seed: lqosync_v3::sessions::Observation =
            serde_json::from_value(value["baseline"].clone()).unwrap();
        let bytes = toml::to_string(&seed).unwrap().into_bytes();
        let after = uuid::Uuid::new_v5(
            &uuid::Uuid::NAMESPACE_OID,
            &serde_json::to_vec(&bytes).unwrap(),
        )
        .to_string();
        let folder = root.path().join("state/core-migrations");
        std::fs::create_dir_all(&folder).unwrap();
        let path = folder.join(format!("{revision}.json"));
        let lease = Lease::maintenance(root.path(), path.to_str().unwrap(), &revision).unwrap();
        drop(
            lqosync_v3::core_migration::Executor::create(
                &path,
                prepared.plan.clone(),
                &prepared.plan.revision().unwrap(),
            )
            .unwrap(),
        );
        lqosync_v3::store::atomic_write(&path.with_extension("bootstrap.json"),&serde_json::to_string(&serde_json::json!({"before":value["candidate_revision"],"after":after,"observation":seed})).unwrap()).unwrap();
        if saved {
            lqosync_v3::sessions::save(root.path(), &seed).unwrap();
            state.lock().await.tag_first = true;
        }
        drop(lease);
        let ack = format!("EXCLUSIVE_ROUTER_ACCESS:{revision}");
        core_adapter::execute(root.path(), &prepared, &revision, &ack, false)
            .await
            .unwrap();
        assert_eq!(state.lock().await.writes.len(), 5);
        core_adapter::execute(root.path(), &prepared, &revision, &ack, true)
            .await
            .unwrap();
        task.abort();
    }
}

#[tokio::test]
async fn source_ip_or_handle_drift_stops_before_candidate_or_router_writes() {
    for key in ["address", ".id"] {
        let (root, state, task) = mock_router(false).await;
        let prepared = core_adapter::prepare(
            root.path(),
            vec![lqosync_v3::core_migration::RowKey {
                router: "r1".into(),
                id: "*F".into(),
            }],
        )
        .await
        .unwrap();
        let revision = prepared.revision().unwrap();
        let ack = format!("EXCLUSIVE_ROUTER_ACCESS:{revision}");
        state
            .lock()
            .await
            .tables
            .get_mut("/ppp/active/print")
            .unwrap()[0]
            .insert(
                key.into(),
                if key == "address" { "192.0.2.4" } else { "*B" }.into(),
            );
        assert!(
            core_adapter::execute(root.path(), &prepared, &revision, &ack, false)
                .await
                .is_err()
        );
        assert!(state.lock().await.writes.is_empty());
        assert!(!root.path().join("state/online-assignments.toml").exists());
        task.abort();
    }
}
#[test]
fn cli_fence_and_private_file_checks_work_across_processes() {
    let root = tempfile::tempdir().unwrap();
    let app = App::open(root.path().into()).unwrap();
    let scope = root.path().join("scope.json");
    lqosync_v3::store::atomic_write(&scope, "[]").unwrap();
    let out = std::process::Command::new(env!("CARGO_BIN_EXE_core-migrate"))
        .arg("prepare")
        .arg(root.path())
        .arg(&scope)
        .arg(root.path().join("plan.json"))
        .output()
        .unwrap();
    assert!(!out.status.success());
    assert!(String::from_utf8_lossy(&out.stderr).contains("writer fence is busy"));
    drop(app);
    use std::os::unix::fs::PermissionsExt;
    std::fs::set_permissions(&scope, std::fs::Permissions::from_mode(0o644)).unwrap();
    let out = std::process::Command::new(env!("CARGO_BIN_EXE_core-migrate"))
        .arg("prepare")
        .arg(root.path())
        .arg(&scope)
        .arg(root.path().join("plan.json"))
        .output()
        .unwrap();
    assert!(!out.status.success());
    assert!(String::from_utf8_lossy(&out.stderr).contains("private regular files"));
}
#[test]
fn legacy_console_helper() {
    if std::env::var_os("LQOSYNC_TEST_LEGACY").is_none() {
        return;
    }
    std::fs::write("/proc/self/comm", "lqosync-v3").unwrap();
    println!("LEGACY_READY");
    std::thread::sleep(std::time::Duration::from_secs(5));
}
#[test]
fn legacy_console_without_an_os_lease_still_blocks_maintenance() {
    use std::io::BufRead;
    let root = tempfile::tempdir().unwrap();
    let mut child = std::process::Command::new(std::env::current_exe().unwrap())
        .args(["--exact", "legacy_console_helper", "--nocapture"])
        .env("LQOSYNC_TEST_LEGACY", "1")
        .env("LQOSYNC3_DATA", root.path())
        .stdout(std::process::Stdio::piped())
        .spawn()
        .unwrap();
    let mut output = std::io::BufReader::new(child.stdout.take().unwrap());
    let mut ready = false;
    for _ in 0..8 {
        let mut line = String::new();
        if output.read_line(&mut line).unwrap() == 0 {
            break;
        }
        if line.contains("LEGACY_READY") {
            ready = true;
            break;
        }
    }
    let result = Lease::exclusive(root.path());
    let _ = child.kill();
    let _ = child.wait();
    assert!(ready);
    assert!(result.is_err());
}
#[test]
fn two_wan_bootstrap_keeps_observed_wan_and_rejects_conflicting_manual_pin() {
    let (mut settings, mut inputs, _, _) = core_fixture::fixture();
    settings = lqosync_v3::dynamic::migrate(&settings).unwrap();
    let config = settings.dynamic.as_mut().unwrap();
    let mut second = config.wans[0].clone();
    second.id = "wan2".into();
    second.interface = "backup".into();
    config.wans.push(second);
    for p in &mut config.policies {
        p.balance = true;
        p.metric = "online_clients".into();
        let mut target = p.targets[0].clone();
        target.wan = "wan2".into();
        p.targets.push(target);
    }
    settings
        .core_lists
        .bindings
        .push(lqosync_v3::core_plan::Binding {
            wan: "wan2".into(),
            router: "r1".into(),
            list: "second-list".into(),
            table: "second-table".into(),
        });
    let inv = Arc::make_mut(inputs.get_mut("r1").unwrap());
    inv.collected_at = lqosync_v3::orchestrator::now();
    inv.tables
        .get_mut("/interface/print")
        .unwrap()
        .push(core_fixture::row(&[
            ("name", "backup"),
            ("running", "true"),
            ("disabled", "false"),
        ]));
    for path in ["/ppp/active/print", "/ip/hotspot/active/print"] {
        inv.tables.get_mut(path).unwrap()[0].insert(".id".into(), "*A".into());
        inv.tables.get_mut(path).unwrap()[0].insert("uptime".into(), "10m".into());
    }
    inv.tables
        .get_mut("/ip/firewall/address-list/print")
        .unwrap()
        .push(core_fixture::row(&[
            (".id", "*F"),
            ("list", "lqos-wan"),
            ("address", "0.0.0.0/0"),
        ]));
    let root = tempfile::tempdir().unwrap();
    let seed = core_adapter::bootstrap(&settings, &inputs, root.path()).unwrap();
    assert!(
        seed.observation
            .assignments
            .routes
            .values()
            .all(|r| r.wan == "wan")
    );
    let (id, route) = seed.observation.assignments.routes.iter().next().unwrap();
    let config = settings.dynamic.as_mut().unwrap();
    let policy = config
        .policies
        .iter()
        .find(|p| p.id == route.policy)
        .unwrap();
    let target = policy
        .targets
        .iter()
        .find(|t| t.wan == "wan2")
        .unwrap()
        .clone();
    config.pins.insert(
        id.clone(),
        lqosync_v3::dynamic::DynamicPin {
            policy: route.policy.clone(),
            target,
        },
    );
    assert!(core_adapter::bootstrap(&settings, &inputs, root.path()).is_err());
}

#[tokio::test]
async fn known_unsent_preflight_failure_does_not_block_rollback() {
    let (root, state, task) = mock_router(false).await;
    let prepared = core_adapter::prepare(
        root.path(),
        vec![lqosync_v3::core_migration::RowKey {
            router: "r1".into(),
            id: "*F".into(),
        }],
    )
    .await
    .unwrap();
    let revision = prepared.revision().unwrap();
    let ack = format!("EXCLUSIVE_ROUTER_ACCESS:{revision}");
    state.lock().await.reject_pending_read = Some(
        root.path()
            .join(format!("state/core-migrations/{revision}.json")),
    );
    assert!(
        core_adapter::execute(root.path(), &prepared, &revision, &ack, false)
            .await
            .is_err()
    );
    assert!(state.lock().await.writes.is_empty());
    core_adapter::execute(root.path(), &prepared, &revision, &ack, true)
        .await
        .unwrap();
    assert!(App::open(root.path().into()).is_ok());
    task.abort();
}

#[tokio::test]
async fn missing_returned_id_blocks_recovery_without_guessing_rows() {
    let (root, state, task) = mock_router(false).await;
    let prepared = core_adapter::prepare(
        root.path(),
        vec![lqosync_v3::core_migration::RowKey {
            router: "r1".into(),
            id: "*F".into(),
        }],
    )
    .await
    .unwrap();
    let revision = prepared.revision().unwrap();
    let ack = format!("EXCLUSIVE_ROUTER_ACCESS:{revision}");
    state.lock().await.omit_id = true;
    assert!(
        core_adapter::execute(root.path(), &prepared, &revision, &ack, false)
            .await
            .is_err()
    );
    assert_eq!(state.lock().await.writes.len(), 1);
    assert!(
        core_adapter::execute(root.path(), &prepared, &revision, &ack, true)
            .await
            .is_err()
    );
    assert_eq!(state.lock().await.writes.len(), 1);
    assert!(App::open(root.path().into()).is_err());
    task.abort();
}

#[test]
fn lease_scope_releases_kernel_lock_even_with_an_inherited_descriptor() {
    use std::os::fd::FromRawFd;
    let root = tempfile::tempdir().unwrap();
    let lease = Lease::exclusive(root.path()).unwrap();
    let path = root.path().join("state/application-writer.lock");
    let fd = std::fs::read_dir("/proc/self/fd")
        .unwrap()
        .filter_map(Result::ok)
        .find_map(|e| {
            (std::fs::read_link(e.path()).ok().as_ref() == Some(&path))
                .then(|| e.file_name().to_string_lossy().parse::<i32>().unwrap())
        })
        .unwrap();
    // SAFETY: dup returns a new owned descriptor for a live File. File takes ownership once.
    let duplicate = unsafe { libc::dup(fd) };
    assert!(duplicate >= 0);
    let inherited = unsafe { std::fs::File::from_raw_fd(duplicate) };
    drop(lease);
    let result = Lease::exclusive(root.path());
    drop(inherited);
    assert!(
        result.is_ok(),
        "a fork-inherited descriptor must not extend the owner's lease scope: {:?}",
        result.err()
    );
}

#[tokio::test]
async fn expired_review_cannot_bootstrap_or_create_a_recovery_marker() {
    let (root, state, task) = mock_router(false).await;
    let prepared = core_adapter::prepare(
        root.path(),
        vec![lqosync_v3::core_migration::RowKey {
            router: "r1".into(),
            id: "*F".into(),
        }],
    )
    .await
    .unwrap();
    let mut value = serde_json::to_value(prepared).unwrap();
    let now = lqosync_v3::orchestrator::now();
    value["plan"]["baseline"]["captured_at"] = (now - 2).into();
    value["plan"]["expires_at"] = (now - 1).into();
    let prepared: core_adapter::Prepared = serde_json::from_value(value).unwrap();
    let revision = prepared.revision().unwrap();
    let ack = format!("EXCLUSIVE_ROUTER_ACCESS:{revision}");
    assert!(
        core_adapter::execute(root.path(), &prepared, &revision, &ack, false)
            .await
            .is_err()
    );
    assert!(state.lock().await.writes.is_empty());
    assert!(!root.path().join("state/online-assignments.toml").exists());
    assert!(
        !root
            .path()
            .join("state/core-migration-active.json")
            .exists()
    );
    task.abort();
}
