use lqosync_v3::{
    core_migration::Member,
    core_writer::{Batch, Evidence, Executor, Operation, Transport},
};
use std::collections::BTreeMap;
fn member(id: &str, list: &str, wan: &str) -> Member {
    Member {
        id: id.into(),
        list: list.into(),
        address: "192.0.2.1/32".into(),
        comment: lqosync_v3::core_plan::marker(
            "80dff65e-3400-4c10-a702-46b19a9d8666",
            "8f67fc01-84f9-4db0-8ef2-b223a16c4d99",
            wan,
        ),
        dynamic: false,
        disabled: false,
        timeout: None,
    }
}
fn batch(ops: Vec<Operation>, rows: Vec<Member>) -> Batch {
    Batch::new(
        "80dff65e-3400-4c10-a702-46b19a9d8666".into(),
        vec![
            lqosync_v3::core_plan::Binding {
                wan: "wan1".into(),
                router: "core".into(),
                list: "one".into(),
                table: "t1".into(),
            },
            lqosync_v3::core_plan::Binding {
                wan: "wan2".into(),
                router: "core".into(),
                list: "two".into(),
                table: "t2".into(),
            },
        ],
        "context".into(),
        lqosync_v3::orchestrator::now() + 60,
        BTreeMap::from([("core".into(), rows)]),
        ops,
    )
    .unwrap()
}
struct Mock {
    rows: BTreeMap<String, Vec<Member>>,
    lost: bool,
    unsent: bool,
    calls: usize,
    context: String,
}
impl Mock {
    fn new(rows: Vec<Member>) -> Self {
        Self {
            rows: BTreeMap::from([("core".into(), rows)]),
            lost: false,
            unsent: false,
            calls: 0,
            context: "context".into(),
        }
    }
}
impl Transport for Mock {
    async fn observe(&mut self) -> anyhow::Result<Evidence> {
        Ok(Evidence {
            context: self.context.clone(),
            captured_at: lqosync_v3::orchestrator::now(),
            valid_until: lqosync_v3::orchestrator::now() + 30,
            rows: self.rows.clone(),
        })
    }
    async fn execute(
        &mut self,
        op: &Operation,
        _guard: &Evidence,
    ) -> anyhow::Result<Option<String>> {
        if self.unsent {
            return Err(
                anyhow::anyhow!("preflight").context(lqosync_v3::core_migration::RequestNotSent)
            );
        }
        self.calls += 1;
        let rows = self.rows.get_mut("core").unwrap();
        let ret = match op {
            Operation::Add { member, .. } => {
                let mut m = member.clone();
                m.id = format!("*{:X}", self.calls + 100);
                let id = m.id.clone();
                rows.push(m);
                Some(id)
            }
            Operation::Remove { before, .. } => {
                rows.retain(|r| r != before);
                None
            }
            Operation::Move { before, after, .. } => {
                *rows.iter_mut().find(|r| *r == before).unwrap() = after.clone();
                None
            }
        };
        if self.lost {
            anyhow::bail!("reply lost");
        }
        Ok(ret)
    }
}
#[tokio::test]
async fn move_keeps_id_and_verified_rollback_restores_exact_row() {
    let before = member("*1", "one", "wan1");
    let after = member("*1", "two", "wan2");
    let b = batch(
        vec![Operation::Move {
            router: "core".into(),
            before: before.clone(),
            after,
        }],
        vec![before.clone()],
    );
    let d = tempfile::tempdir().unwrap();
    let revision = b.revision().unwrap();
    let mut io = Mock::new(vec![before.clone()]);
    let mut x = Executor::open(d.path(), b, &revision).unwrap();
    x.forward(&mut io).await.unwrap();
    assert_eq!(io.rows["core"][0].list, "two");
    x.rollback(&mut io).await.unwrap();
    assert_eq!(io.rows["core"], vec![before]);
    x.finish().unwrap();
    assert!(!d.path().join("state/core-migration-active.json").exists());
}
#[tokio::test]
async fn lost_reply_never_retries_or_guesses_and_keeps_recovery_fence() {
    let m = member("", "one", "wan1");
    let b = batch(
        vec![Operation::Add {
            router: "core".into(),
            member: m,
        }],
        vec![],
    );
    let d = tempfile::tempdir().unwrap();
    let rev = b.revision().unwrap();
    let mut io = Mock::new(vec![]);
    io.lost = true;
    {
        let mut x = Executor::open(d.path(), b.clone(), &rev).unwrap();
        assert!(x.forward(&mut io).await.is_err());
    }
    let mut x = Executor::open(d.path(), b, &rev).unwrap();
    io.lost = false;
    assert!(x.forward(&mut io).await.is_err());
    assert!(x.rollback(&mut io).await.is_err());
    assert_eq!(io.calls, 1);
    assert!(d.path().join("state/core-migration-active.json").exists());
}
#[tokio::test]
async fn known_unsent_can_resume_but_foreign_drift_stops_before_send() {
    let m = member("", "one", "wan1");
    let b = batch(
        vec![Operation::Add {
            router: "core".into(),
            member: m,
        }],
        vec![],
    );
    let d = tempfile::tempdir().unwrap();
    let rev = b.revision().unwrap();
    let mut io = Mock::new(vec![]);
    let mut x = Executor::open(d.path(), b, &rev).unwrap();
    io.unsent = true;
    assert!(x.forward(&mut io).await.is_err());
    io.unsent = false;
    io.context = "changed".into();
    assert!(x.forward(&mut io).await.is_err());
    assert_eq!(io.calls, 0);
    io.context = "context".into();
    x.forward(&mut io).await.unwrap();
    x.rollback(&mut io).await.unwrap();
    assert!(io.rows["core"].is_empty());
    x.finish().unwrap();
}
#[tokio::test]
async fn removal_rollback_records_actual_replacement_id() {
    let m = member("*1", "one", "wan1");
    let b = batch(
        vec![Operation::Remove {
            router: "core".into(),
            before: m.clone(),
        }],
        vec![m],
    );
    let d = tempfile::tempdir().unwrap();
    let rev = b.revision().unwrap();
    let mut io = Mock::new(b.baseline().clone()["core"].clone());
    {
        let mut x = Executor::open(d.path(), b.clone(), &rev).unwrap();
        x.forward(&mut io).await.unwrap();
    }
    let mut x = Executor::open(d.path(), b, &rev).unwrap();
    x.rollback(&mut io).await.unwrap();
    assert_ne!(io.rows["core"][0].id, "*1");
    assert_eq!(io.rows["core"][0].address, "192.0.2.1/32");
    x.finish().unwrap();
}
#[test]
fn move_rejects_changed_row_id_or_address() {
    let before = member("*1", "one", "wan1");
    for (id, address) in [("*2", "192.0.2.1/32"), ("*1", "192.0.2.2/32")] {
        let mut after = member(id, "two", "wan2");
        after.address = address.into();
        let mut value = serde_json::to_value(batch(
            vec![Operation::Move {
                router: "core".into(),
                before: before.clone(),
                after: member("*1", "two", "wan2"),
            }],
            vec![before.clone()],
        ))
        .unwrap();
        value["operations"][0]["after"] = serde_json::to_value(after).unwrap();
        let invalid: Batch = serde_json::from_value(value).unwrap();
        let rev = invalid.revision().unwrap();
        let root = tempfile::tempdir().unwrap();
        assert!(Executor::open(root.path(), invalid, &rev).is_err());
    }
}
#[test]
fn serialized_batches_cannot_increase_mutation_limit_or_change_owner() {
    let b = batch(
        vec![Operation::Add {
            router: "core".into(),
            member: member("", "one", "wan1"),
        }],
        vec![],
    );
    for mutation in ["too_many", "foreign"] {
        let mut value = serde_json::to_value(&b).unwrap();
        if mutation == "too_many" {
            let op = value["operations"][0].clone();
            value["operations"] = serde_json::Value::Array(vec![op; 21]);
        } else {
            value["operations"][0]["member"]["comment"] = serde_json::json!("manual");
        }
        let tampered: Batch = serde_json::from_value(value).unwrap();
        let rev = tampered.revision().unwrap();
        let dir = tempfile::tempdir().unwrap();
        assert!(Executor::open(dir.path(), tampered, &rev).is_err());
        assert!(!dir.path().join("state/core-migration-active.json").exists());
    }
}
#[test]
fn leases_exclude_consoles_competing_batches_and_maintenance() {
    let b = batch(
        vec![Operation::Add {
            router: "core".into(),
            member: member("", "one", "wan1"),
        }],
        vec![],
    );
    let root = tempfile::tempdir().unwrap();
    let rev = b.revision().unwrap();
    let console = lqosync_v3::migration_fence::Lease::console(root.path()).unwrap();
    assert!(Executor::open(root.path(), b.clone(), &rev).is_err());
    drop(console);
    let executor = Executor::open(root.path(), b.clone(), &rev).unwrap();
    assert!(Executor::open(root.path(), b.clone(), &rev).is_err());
    assert!(lqosync_v3::migration_fence::Lease::console(root.path()).is_err());
    drop(executor);
    assert!(lqosync_v3::migration_fence::Lease::exclusive(root.path()).is_err());
}
#[tokio::test]
async fn missing_returned_id_and_post_write_drift_remain_fenced() {
    struct NoId(Mock);
    impl Transport for NoId {
        async fn observe(&mut self) -> anyhow::Result<Evidence> {
            self.0.observe().await
        }
        async fn execute(
            &mut self,
            op: &Operation,
            e: &Evidence,
        ) -> anyhow::Result<Option<String>> {
            self.0.execute(op, e).await?;
            Ok(None)
        }
    }
    let b = batch(
        vec![Operation::Add {
            router: "core".into(),
            member: member("", "one", "wan1"),
        }],
        vec![],
    );
    let root = tempfile::tempdir().unwrap();
    let rev = b.revision().unwrap();
    let mut x = Executor::open(root.path(), b, &rev).unwrap();
    let mut io = NoId(Mock::new(vec![]));
    assert!(x.forward(&mut io).await.is_err());
    assert!(x.rollback(&mut io).await.is_err());
    assert_eq!(io.0.calls, 1);
}
#[test]
fn production_transport_requires_explicit_fresh_verifier() {
    use lqosync_v3::core_writer_transport::{RouterTransport, Verifier};
    struct Verify;
    impl Verifier for Verify {
        async fn verify(&mut self) -> anyhow::Result<Evidence> {
            anyhow::bail!("no approved settings/source/health evidence")
        }
    }
    let dir = tempfile::tempdir().unwrap();
    let settings = lqosync_v3::model::Settings::default();
    assert!(RouterTransport::new(dir.path(), settings, Verify).is_ok());
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

struct WireState {
    rows: Vec<Member>,
    commands: Vec<(String, BTreeMap<String, String>)>,
    next: u64,
    mode: String,
}
async fn wire_server() -> (
    lqosync_v3::model::Settings,
    std::sync::Arc<tokio::sync::Mutex<WireState>>,
    tokio::task::JoinHandle<()>,
) {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let mut settings = lqosync_v3::policy_fixture::settings();
    settings.routers[0].id = "core".into();
    settings.routers[0].port = listener.local_addr().unwrap().port();
    settings.routers[0].timeout_seconds = 1;
    settings.core_lists = lqosync_v3::core_plan::Policy {
        owner: "80dff65e-3400-4c10-a702-46b19a9d8666".into(),
        bindings: vec![],
    };
    let mut wan:lqosync_v3::model::Node=toml::from_str("id='wan1'\nname='WAN1'\nparent=''\nkind='wan'\nrouter='core'\nsource='uplink'\nmode='manual'\ndownload=100.0\nupload=100.0\ndownload_factor=1.0\nupload_factor=1.0\n").unwrap();
    settings.nodes.push(wan.clone());
    wan.id = "wan2".into();
    wan.source = "uplink2".into();
    settings.nodes.push(wan);
    settings.core_lists.bindings = vec![
        lqosync_v3::core_plan::Binding {
            wan: "wan1".into(),
            router: "core".into(),
            list: "one".into(),
            table: "t1".into(),
        },
        lqosync_v3::core_plan::Binding {
            wan: "wan2".into(),
            router: "core".into(),
            list: "two".into(),
            table: "t2".into(),
        },
    ];
    let db = std::sync::Arc::new(tokio::sync::Mutex::new(WireState {
        rows: vec![],
        commands: vec![],
        next: 256,
        mode: String::new(),
    }));
    let shared = db.clone();
    let task = tokio::spawn(async move {
        loop {
            let (mut socket, _) = listener.accept().await.unwrap();
            socket.set_nodelay(true).unwrap();
            let state = shared.clone();
            tokio::spawn(async move {
                while let Some(words) = sentence(&mut socket).await {
                    let tag = words
                        .iter()
                        .find(|s| s.starts_with(".tag="))
                        .unwrap()
                        .clone();
                    let cmd = words[0].clone();
                    if cmd == "/login" {
                        reply(&mut socket, vec!["!done".into(), tag]).await;
                        continue;
                    }
                    let attrs: BTreeMap<String, String> = words
                        .iter()
                        .filter_map(|s| {
                            s.strip_prefix('=')
                                .and_then(|v| v.split_once('='))
                                .map(|(k, v)| (k.into(), v.into()))
                        })
                        .collect();
                    let mut state = state.lock().await;
                    if cmd.ends_with("/print") {
                        let rows = state.rows.clone();
                        drop(state);
                        for row in rows {
                            let mut attrs = vec![
                                "!re".into(),
                                tag.clone(),
                                format!("=.id={}", row.id),
                                format!("=list={}", row.list),
                                format!("=address={}", row.address),
                                format!("=comment={}", row.comment),
                                "=disabled=false".into(),
                                "=dynamic=false".into(),
                            ];
                            if let Some(t) = row.timeout {
                                attrs.push(format!("=timeout={t}"));
                            }
                            reply(&mut socket, attrs).await;
                        }
                        reply(&mut socket, vec!["!done".into(), tag]).await;
                        continue;
                    }
                    state.commands.push((cmd.clone(), attrs.clone()));
                    if state.mode == "trap" {
                        drop(state);
                        reply(
                            &mut socket,
                            vec!["!trap".into(), "=message=denied".into(), tag],
                        )
                        .await;
                        continue;
                    }
                    let mut returned = None;
                    match cmd.as_str() {
                        "/ip/firewall/address-list/add" => {
                            let mut row = member("", &attrs["list"], "wan1");
                            row.address = attrs["address"].clone();
                            row.comment = attrs["comment"].clone();
                            row.id = format!("*{:X}", state.next);
                            state.next += 1;
                            returned = Some(row.id.clone());
                            state.rows.push(row);
                        }
                        "/ip/firewall/address-list/set" => {
                            let r = state
                                .rows
                                .iter_mut()
                                .find(|r| r.id == attrs[".id"])
                                .unwrap();
                            assert_eq!(attrs.len(), 3);
                            r.list = attrs["list"].clone();
                            r.comment = attrs["comment"].clone();
                        }
                        "/ip/firewall/address-list/remove" => {
                            state.rows.retain(|r| r.id != attrs[".id"])
                        }
                        _ => panic!("Unexpected command"),
                    }
                    let mode = state.mode.clone();
                    drop(state);
                    if mode == "lost" {
                        break;
                    }
                    let mut words = vec!["!done".into(), tag];
                    if mode != "missing"
                        && let Some(id) = returned
                    {
                        words.push(format!("=ret={id}"));
                    }
                    reply(&mut socket, words).await;
                }
            });
        }
    });
    (settings, db, task)
}
struct WireVerifier {
    settings: lqosync_v3::model::Settings,
    context: String,
}
impl lqosync_v3::core_writer_transport::Verifier for WireVerifier {
    async fn verify(&mut self) -> anyhow::Result<Evidence> {
        let c = lqosync_v3::routeros::Client::connect_with_password(
            &self.settings.routers[0],
            "synthetic",
        )
        .await?;
        let rows = lqosync_v3::core_writer_transport::members(
            &c.read("/ip/firewall/address-list/print").await?,
        )?;
        let now = lqosync_v3::orchestrator::now();
        Ok(Evidence {
            context: self.context.clone(),
            captured_at: now,
            valid_until: now + 30,
            rows: BTreeMap::from([("core".into(), rows)]),
        })
    }
}
#[tokio::test]
async fn wire_transport_moves_only_list_comment_and_uses_actual_add_ids() {
    use lqosync_v3::core_writer_transport::RouterTransport;
    let (settings, db, task) = wire_server().await;
    db.lock().await.rows = vec![member("*1", "one", "wan1")];
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(
        dir.path().join("credentials.toml"),
        "[routers.core]\npassword='synthetic'\n",
    )
    .unwrap();
    let verifier = WireVerifier {
        settings: settings.clone(),
        context: "context".into(),
    };
    let mut io = RouterTransport::new(dir.path(), settings, verifier).unwrap();
    let before = member("*1", "one", "wan1");
    let b = batch(
        vec![Operation::Move {
            router: "core".into(),
            before: before.clone(),
            after: member("*1", "two", "wan2"),
        }],
        vec![before.clone()],
    );
    let rev = b.revision().unwrap();
    let mut x = Executor::open(dir.path(), b, &rev).unwrap();
    x.forward(&mut io).await.unwrap();
    x.rollback(&mut io).await.unwrap();
    x.finish().unwrap();
    assert_eq!(db.lock().await.rows, vec![before]);
    let mut m = member("", "one", "wan1");
    m.address = "192.0.2.2/32".into();
    m.comment = lqosync_v3::core_plan::marker(
        "80dff65e-3400-4c10-a702-46b19a9d8666",
        "9f67fc01-84f9-4db0-8ef2-b223a16c4d99",
        "wan1",
    );
    let b = batch(
        vec![Operation::Add {
            router: "core".into(),
            member: m,
        }],
        db.lock().await.rows.clone(),
    );
    let rev = b.revision().unwrap();
    let mut x = Executor::open(dir.path(), b, &rev).unwrap();
    x.forward(&mut io).await.unwrap();
    assert!(db.lock().await.rows.iter().any(|r| r.id == "*100"));
    x.rollback(&mut io).await.unwrap();
    x.finish().unwrap();
    assert!(
        db.lock()
            .await
            .commands
            .iter()
            .all(|(c, a)| c != "/ip/firewall/address-list/set"
                || a.keys().cloned().collect::<Vec<_>>() == vec![".id", "comment", "list"])
    );
    task.abort();
}
#[tokio::test]
async fn wire_trap_lost_reply_and_absent_returned_id_block_replay() {
    use lqosync_v3::core_writer_transport::RouterTransport;
    for mode in ["trap", "lost", "missing"] {
        let (settings, db, task) = wire_server().await;
        db.lock().await.mode = mode.into();
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("credentials.toml"),
            "[routers.core]\npassword='synthetic'\n",
        )
        .unwrap();
        let verifier = WireVerifier {
            settings: settings.clone(),
            context: "context".into(),
        };
        let mut io = RouterTransport::new(dir.path(), settings, verifier).unwrap();
        let b = batch(
            vec![Operation::Add {
                router: "core".into(),
                member: member("", "one", "wan1"),
            }],
            vec![],
        );
        let rev = b.revision().unwrap();
        let mut x = Executor::open(dir.path(), b, &rev).unwrap();
        assert!(x.forward(&mut io).await.is_err());
        assert!(x.forward(&mut io).await.is_err());
        assert!(x.rollback(&mut io).await.is_err());
        assert_eq!(db.lock().await.commands.len(), 1);
        task.abort();
    }
}
#[tokio::test]
async fn reopening_a_verified_journal_requires_a_fresh_verification_before_finish() {
    let b = batch(
        vec![Operation::Add {
            router: "core".into(),
            member: member("", "one", "wan1"),
        }],
        vec![],
    );
    let root = tempfile::tempdir().unwrap();
    let rev = b.revision().unwrap();
    let mut io = Mock::new(vec![]);
    {
        let mut x = Executor::open(root.path(), b.clone(), &rev).unwrap();
        x.forward(&mut io).await.unwrap();
    }
    let x = Executor::open(root.path(), b, &rev).unwrap();
    assert!(x.finish().is_err());
    assert!(
        root.path()
            .join("state/core-migration-active.json")
            .exists()
    );
}
#[test]
fn an_unrelated_malformed_owned_row_blocks_a_batch() {
    let m = member("", "one", "wan1");
    let mut broken = member("*1", "two", "wan2");
    broken.address = "198.51.100.0/24".into();
    broken.comment = lqosync_v3::core_plan::marker(
        "80dff65e-3400-4c10-a702-46b19a9d8666",
        "9f67fc01-84f9-4db0-8ef2-b223a16c4d99",
        "wan2",
    );
    let mut b = serde_json::to_value(batch(
        vec![Operation::Add {
            router: "core".into(),
            member: m,
        }],
        vec![],
    ))
    .unwrap();
    b["baseline"]["core"] = serde_json::json!([broken]);
    let b: Batch = serde_json::from_value(b).unwrap();
    let rev = b.revision().unwrap();
    let root = tempfile::tempdir().unwrap();
    assert!(Executor::open(root.path(), b, &rev).is_err());
}

#[tokio::test]
async fn context_change_during_final_target_read_is_known_unsent() {
    use lqosync_v3::core_writer_transport::{RouterTransport, Verifier};
    struct Changing {
        inner: WireVerifier,
        calls: usize,
    }
    impl Verifier for Changing {
        async fn verify(&mut self) -> anyhow::Result<Evidence> {
            self.calls += 1;
            let mut e = self.inner.verify().await?;
            if self.calls >= 3 {
                e.context = "authorization revoked".into();
            }
            Ok(e)
        }
    }
    let (settings, db, task) = wire_server().await;
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(
        dir.path().join("credentials.toml"),
        "[routers.core]\npassword='synthetic'\n",
    )
    .unwrap();
    let verifier = Changing {
        inner: WireVerifier {
            settings: settings.clone(),
            context: "context".into(),
        },
        calls: 0,
    };
    let mut io = RouterTransport::new(dir.path(), settings, verifier).unwrap();
    let b = batch(
        vec![Operation::Add {
            router: "core".into(),
            member: member("", "one", "wan1"),
        }],
        vec![],
    );
    let revision = b.revision().unwrap();
    let mut x = Executor::open(dir.path(), b, &revision).unwrap();
    let error = x.forward(&mut io).await.unwrap_err();
    assert!(
        error
            .downcast_ref::<lqosync_v3::core_migration::RequestNotSent>()
            .is_some()
    );
    assert!(db.lock().await.commands.is_empty());
    let journal: serde_json::Value = serde_json::from_str(
        &std::fs::read_to_string(
            dir.path()
                .join("state/core-writer")
                .join(format!("{revision}.json")),
        )
        .unwrap(),
    )
    .unwrap();
    assert!(journal["pending"].is_null());
    assert!(x.rollback(&mut io).await.is_err());
    task.abort();
}
#[test]
fn batch_requires_complete_binding_core_scope() {
    let mut b = serde_json::to_value(batch(
        vec![Operation::Add {
            router: "core".into(),
            member: member("", "one", "wan1"),
        }],
        vec![],
    ))
    .unwrap();
    b["bindings"][1]["router"] = serde_json::json!("other-core");
    let b: Batch = serde_json::from_value(b).unwrap();
    let rev = b.revision().unwrap();
    let root = tempfile::tempdir().unwrap();
    assert!(Executor::open(root.path(), b, &rev).is_err());
}
#[tokio::test]
async fn exact_twenty_operation_limit_is_enforced_and_rollback_is_bounded() {
    let mut operations = vec![];
    for i in 1..=20 {
        let mut m = member("", "one", "wan1");
        m.address = format!("198.51.100.{i}/32");
        m.comment = lqosync_v3::core_plan::marker(
            "80dff65e-3400-4c10-a702-46b19a9d8666",
            &uuid::Uuid::new_v5(&uuid::Uuid::NAMESPACE_OID, &[i]).to_string(),
            "wan1",
        );
        operations.push(Operation::Add {
            router: "core".into(),
            member: m,
        });
    }
    let b = batch(operations, vec![]);
    let root = tempfile::tempdir().unwrap();
    let rev = b.revision().unwrap();
    let mut io = Mock::new(vec![]);
    let mut x = Executor::open(root.path(), b, &rev).unwrap();
    x.forward(&mut io).await.unwrap();
    assert_eq!(io.calls, 20);
    x.rollback(&mut io).await.unwrap();
    assert_eq!(io.calls, 40);
    assert!(io.rows["core"].is_empty());
    x.finish().unwrap();
}
#[tokio::test]
async fn foreign_row_drift_prevents_write_and_finish_before_verification() {
    let before = member("*1", "one", "wan1");
    let b = batch(
        vec![Operation::Move {
            router: "core".into(),
            before: before.clone(),
            after: member("*1", "two", "wan2"),
        }],
        vec![before.clone()],
    );
    let root = tempfile::tempdir().unwrap();
    let rev = b.revision().unwrap();
    let mut io = Mock::new(vec![before]);
    io.rows.get_mut("core").unwrap()[0].comment = "manual".into();
    let mut x = Executor::open(root.path(), b, &rev).unwrap();
    assert!(x.forward(&mut io).await.is_err());
    assert_eq!(io.calls, 0);
    assert!(x.finish().is_err());
}
#[tokio::test]
async fn corrupt_pending_journal_does_not_enable_retry() {
    let b = batch(
        vec![Operation::Add {
            router: "core".into(),
            member: member("", "one", "wan1"),
        }],
        vec![],
    );
    let root = tempfile::tempdir().unwrap();
    let rev = b.revision().unwrap();
    let mut io = Mock::new(vec![]);
    io.lost = true;
    {
        let mut x = Executor::open(root.path(), b.clone(), &rev).unwrap();
        assert!(x.forward(&mut io).await.is_err());
    }
    let path = root
        .path()
        .join("state/core-writer")
        .join(format!("{rev}.json"));
    let mut json: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
    json["forward"] = serde_json::json!(1);
    std::fs::write(path, serde_json::to_string(&json).unwrap()).unwrap();
    assert!(Executor::open(root.path(), b, &rev).is_err());
    assert_eq!(io.calls, 1);
    assert!(
        root.path()
            .join("state/core-migration-active.json")
            .exists()
    );
}
#[test]
fn expired_batch_rejects_before_creating_a_recovery_marker() {
    let mut value = serde_json::to_value(batch(
        vec![Operation::Add {
            router: "core".into(),
            member: member("", "one", "wan1"),
        }],
        vec![],
    ))
    .unwrap();
    value["expires_at"] = serde_json::json!(1);
    let b: Batch = serde_json::from_value(value).unwrap();
    let rev = b.revision().unwrap();
    let root = tempfile::tempdir().unwrap();
    assert!(Executor::open(root.path(), b, &rev).is_err());
    assert!(
        !root
            .path()
            .join("state/core-migration-active.json")
            .exists()
    );
}
#[tokio::test]
async fn failed_intent_persistence_never_sends_and_poisoned_writer_cannot_continue() {
    struct BrokenStore {
        inner: Mock,
        path: std::path::PathBuf,
    }
    impl Transport for BrokenStore {
        async fn observe(&mut self) -> anyhow::Result<Evidence> {
            let _ = std::fs::remove_file(&self.path);
            std::fs::create_dir(&self.path)?;
            self.inner.observe().await
        }
        async fn execute(&mut self, o: &Operation, g: &Evidence) -> anyhow::Result<Option<String>> {
            self.inner.execute(o, g).await
        }
    }
    let b = batch(
        vec![Operation::Add {
            router: "core".into(),
            member: member("", "one", "wan1"),
        }],
        vec![],
    );
    let root = tempfile::tempdir().unwrap();
    let rev = b.revision().unwrap();
    let mut x = Executor::open(root.path(), b, &rev).unwrap();
    let mut io = BrokenStore {
        inner: Mock::new(vec![]),
        path: root
            .path()
            .join("state/core-writer")
            .join(format!("{rev}.json")),
    };
    assert!(x.forward(&mut io).await.is_err());
    assert_eq!(io.inner.calls, 0);
    assert!(x.rollback(&mut io).await.is_err());
    assert!(x.finish().is_err());
    assert!(
        root.path()
            .join("state/core-migration-active.json")
            .exists()
    );
}
#[tokio::test]
async fn failed_reverification_revokes_previous_completion() {
    let b = batch(
        vec![Operation::Add {
            router: "core".into(),
            member: member("", "one", "wan1"),
        }],
        vec![],
    );
    let root = tempfile::tempdir().unwrap();
    let rev = b.revision().unwrap();
    let mut io = Mock::new(vec![]);
    let mut x = Executor::open(root.path(), b, &rev).unwrap();
    x.forward(&mut io).await.unwrap();
    io.context = "revoked".into();
    assert!(x.forward(&mut io).await.is_err());
    assert!(x.finish().is_err());
    assert!(
        root.path()
            .join("state/core-migration-active.json")
            .exists()
    );
}
