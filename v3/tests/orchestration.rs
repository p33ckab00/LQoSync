use lqosync_v3::{orchestrator, store, web::App};
use std::sync::Arc;
use tokio::sync::Mutex;
mod common;
fn app() -> (tempfile::TempDir, lqosync_v3::web::Shared) {
    let dir = tempfile::tempdir().unwrap();
    let (settings, mut inventories) = common::fixture();
    for inventory in inventories.values_mut() {
        Arc::make_mut(inventory).collected_at = orchestrator::now();
    }
    store::save_settings(dir.path(), &settings).unwrap();
    let mut app = App::open(dir.path().into()).unwrap();
    app.inventories = inventories;
    (dir, Arc::new(Mutex::new(app)))
}
#[tokio::test]
async fn healthy_selected_inventory_automatically_publishes_and_failed_reads_keep_last_good() {
    let (dir, state) = app();
    orchestrator::refresh(state.clone()).await;
    let snapshot = store::load_snapshot(dir.path()).unwrap();
    assert_eq!(snapshot.len(), 3);
    let pointer = std::fs::read(dir.path().join("state/current.toml")).unwrap();
    let mirror = std::fs::read(dir.path().join("state/subscribers.toml")).unwrap();
    state
        .lock()
        .await
        .inventory_errors
        .insert("r1".into(), "Timeout".into());
    orchestrator::refresh(state.clone()).await;
    assert_eq!(
        pointer,
        std::fs::read(dir.path().join("state/current.toml")).unwrap()
    );
    assert_eq!(
        mirror,
        std::fs::read(dir.path().join("state/subscribers.toml")).unwrap()
    );
    assert!(state.lock().await.status.contains("Last valid snapshot"));
}

#[tokio::test]
async fn mapped_wan_health_failure_retains_the_last_consistent_assignment_generation() {
    let (dir, state) = app();
    {
        let mut a = state.lock().await;
        a.settings.nodes[0].router = "r1".into();
        a.settings.nodes[0].source = "uplink".into();
        Arc::make_mut(a.inventories.get_mut("r1").unwrap())
            .tables
            .insert(
                "/interface/print".into(),
                vec![
                    [
                        ("name".into(), "uplink".into()),
                        ("running".into(), "true".into()),
                        ("disabled".into(), "false".into()),
                    ]
                    .into_iter()
                    .collect(),
                ],
            );
    }
    orchestrator::refresh(state.clone()).await;
    let pointer = std::fs::read(dir.path().join("state/current.toml")).unwrap();
    let before = store::load_assignments(dir.path()).unwrap();
    {
        let mut a = state.lock().await;
        Arc::make_mut(a.inventories.get_mut("r1").unwrap())
            .tables
            .get_mut("/interface/print")
            .unwrap()[0]
            .insert("running".into(), "false".into());
    }
    orchestrator::refresh(state.clone()).await;
    assert_eq!(
        pointer,
        std::fs::read(dir.path().join("state/current.toml")).unwrap()
    );
    assert_eq!(before, store::load_assignments(dir.path()).unwrap());
    assert!(
        state
            .lock()
            .await
            .status
            .contains("down, disabled or has unknown status")
    );
    let generation = store::current_generation(dir.path()).unwrap();
    std::fs::write(
        dir.path()
            .join("staging")
            .join(generation)
            .join("assignments.toml"),
        "invalid assignment file",
    )
    .unwrap();
    {
        let mut a = state.lock().await;
        Arc::make_mut(a.inventories.get_mut("r1").unwrap())
            .tables
            .get_mut("/interface/print")
            .unwrap()[0]
            .insert("running".into(), "true".into());
    }
    orchestrator::refresh(state.clone()).await;
    assert_eq!(
        pointer,
        std::fs::read(dir.path().join("state/current.toml")).unwrap()
    );
    assert!(state.lock().await.status.contains("Last valid snapshot"));
}
#[tokio::test]
async fn paused_sources_do_not_collect_or_publish() {
    let (dir, state) = app();
    state.lock().await.settings.auto_stage = false;
    orchestrator::refresh(state.clone()).await;
    orchestrator::cycle(state.clone()).await;
    assert!(state.lock().await.read_epochs.is_empty());
    assert!(!dir.path().join("state/current.toml").exists());
}
#[tokio::test]
async fn failed_scheduled_round_cannot_replace_healthy_snapshot() {
    let (dir, state) = app();
    orchestrator::refresh(state.clone()).await;
    let pointer = std::fs::read(dir.path().join("state/current.toml")).unwrap();
    // Fixture uses an unset password reference. The cycle fails before any network connection.
    orchestrator::cycle(state.clone()).await;
    assert!(state.lock().await.inventory_errors.contains_key("r1"));
    assert_eq!(
        pointer,
        std::fs::read(dir.path().join("state/current.toml")).unwrap()
    );
}
#[tokio::test]
async fn clearing_all_bindings_does_not_republish_managed_state_as_an_offline_snapshot() {
    let (dir, state) = app();
    orchestrator::refresh(state.clone()).await;
    let pointer = std::fs::read(dir.path().join("state/current.toml")).unwrap();
    {
        let mut a = state.lock().await;
        for node in &mut a.settings.nodes {
            node.source.clear();
        }
        a.reconcile();
    }
    assert_eq!(
        pointer,
        std::fs::read(dir.path().join("state/current.toml")).unwrap()
    );
}

#[tokio::test]
async fn pausing_a_read_round_stops_queued_routers() {
    use std::sync::atomic::{AtomicUsize, Ordering};
    let dir = tempfile::tempdir().unwrap();
    let (mut settings, _) = common::fixture();
    settings.routers.clear();
    settings.nodes.retain(|n| n.kind == "wan");
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();
    let accepted = Arc::new(AtomicUsize::new(0));
    let accepted_server = accepted.clone();
    let (tx, mut rx) = tokio::sync::mpsc::channel(10);
    let server = tokio::spawn(async move {
        let mut sockets = vec![];
        loop {
            let (socket, _) = listener.accept().await.unwrap();
            sockets.push(socket);
            accepted_server.fetch_add(1, Ordering::SeqCst);
            tx.send(()).await.unwrap();
        }
    });
    for i in 0..5 {
        let mut router = common::fixture().0.routers[0].clone();
        router.id = format!("r{i}");
        router.port = port;
        router.timeout_seconds = 1;
        // Use a non-secret existing environment value for the local mock login only.
        router.password_env = "PATH".into();
        settings.routers.push(router);
        let mut node = common::fixture().0.nodes[1].clone();
        node.id = format!("ppp{i}");
        node.router = format!("r{i}");
        settings.nodes.push(node);
    }
    store::save_settings(dir.path(), &settings).unwrap();
    let state = Arc::new(Mutex::new(App::open(dir.path().into()).unwrap()));
    let cycle = tokio::spawn(orchestrator::cycle(state.clone()));
    for _ in 0..4 {
        tokio::time::timeout(std::time::Duration::from_secs(3), rx.recv())
            .await
            .unwrap()
            .unwrap();
    }
    state.lock().await.settings.auto_stage = false;
    tokio::time::timeout(std::time::Duration::from_secs(3), cycle)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(accepted.load(Ordering::SeqCst), 4);
    assert!(!dir.path().join("state/current.toml").exists());
    server.abort();
}

#[test]
fn normalization_discards_a_calculation_if_a_new_read_starts_while_it_is_queued() {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .max_blocking_threads(1)
        .build()
        .unwrap();
    runtime.block_on(async {
        let (dir, state) = app();
        let (started_tx, started_rx) = tokio::sync::oneshot::channel();
        let (release_tx, release_rx) = std::sync::mpsc::channel();
        let blocker = tokio::task::spawn_blocking(move || {
            started_tx.send(()).unwrap();
            release_rx.recv().unwrap();
        });
        started_rx.await.unwrap();
        let work = orchestrator::refresh(state.clone());
        tokio::pin!(work);
        let pending =
            std::future::poll_fn(|cx| std::task::Poll::Ready(work.as_mut().poll(cx).is_pending()))
                .await;
        assert!(pending);
        state.lock().await.begin_read("r1");
        release_tx.send(()).unwrap();
        blocker.await.unwrap();
        work.await;
        assert!(!dir.path().join("state/current.toml").exists());
        assert!(state.lock().await.status.contains("discarded"));
    });
}

async fn read_sentence(stream: &mut tokio::net::TcpStream) -> Vec<String> {
    use tokio::io::AsyncReadExt;
    let mut words = vec![];
    loop {
        let first = stream.read_u8().await.unwrap();
        let len = if first < 128 {
            usize::from(first)
        } else {
            (usize::from(first & 0x7f) << 8) | usize::from(stream.read_u8().await.unwrap())
        };
        if len == 0 {
            return words;
        }
        let mut bytes = vec![0; len];
        stream.read_exact(&mut bytes).await.unwrap();
        words.push(String::from_utf8(bytes).unwrap());
    }
}
async fn send_sentence(stream: &mut tokio::net::TcpStream, words: Vec<String>) {
    use tokio::io::AsyncWriteExt;
    let mut bytes = vec![];
    for word in words {
        assert!(word.len() < 128);
        bytes.push(word.len() as u8);
        bytes.extend(word.as_bytes());
    }
    bytes.push(0);
    stream.write_all(&bytes).await.unwrap();
}

#[tokio::test]
async fn selected_read_round_includes_a_mapped_core_router_without_subscriber_bindings() {
    let dir = tempfile::tempdir().unwrap();
    let (mut settings, inventories) = common::fixture();
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let mut core = settings.routers[0].clone();
    core.id = "core".into();
    core.role = "core".into();
    core.port = listener.local_addr().unwrap().port();
    core.password_env = "PATH".into();
    settings.routers.push(core);
    settings.nodes[0].router = "core".into();
    settings.nodes[0].source = "uplink".into();
    let server = tokio::spawn(async move {
        let (mut stream, _) = listener.accept().await.unwrap();
        let login = read_sentence(&mut stream).await;
        assert_eq!(login[0], "/login");
        let mut done = vec!["!done".into()];
        done.extend(login.into_iter().filter(|w| w.starts_with(".tag=")));
        send_sentence(&mut stream, done).await;
        for _ in 0..6 {
            let command = read_sentence(&mut stream).await;
            assert!(command[0].ends_with("/print"));
            let tag = command
                .iter()
                .find(|w| w.starts_with(".tag="))
                .unwrap()
                .clone();
            if command[0] == "/interface/print" {
                send_sentence(
                    &mut stream,
                    vec![
                        "!re".into(),
                        tag.clone(),
                        "=name=uplink".into(),
                        "=running=true".into(),
                        "=disabled=false".into(),
                    ],
                )
                .await;
            }
            send_sentence(&mut stream, vec!["!done".into(), tag]).await;
        }
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
    });
    store::save_settings(dir.path(), &settings).unwrap();
    let mut a = App::open(dir.path().into()).unwrap();
    a.inventories = inventories;
    let state = Arc::new(Mutex::new(a));
    tokio::time::timeout(
        std::time::Duration::from_secs(5),
        orchestrator::cycle(state.clone()),
    )
    .await
    .unwrap();
    let a = state.lock().await;
    assert!(a.read_epochs.contains_key("core"));
    assert!(
        a.inventories["core"].tables["/interface/print"]
            .iter()
            .any(|r| r.get("name").is_some_and(|n| n == "uplink"))
    );
    assert!(!a.inventory_errors.contains_key("core"));
    // The unconfigured distro credential fails before connecting; no generation may publish.
    assert!(a.inventory_errors.contains_key("r1"));
    assert!(!dir.path().join("state/current.toml").exists());
    drop(a);
    server.await.unwrap();
}
#[tokio::test]
async fn scheduled_mock_router_collection_reaches_a_committed_subscriber_generation() {
    let dir = tempfile::tempdir().unwrap();
    let (mut settings, inventories) = common::fixture();
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    settings.routers[0].port = listener.local_addr().unwrap().port();
    settings.routers[0].password_env = "PATH".into();
    let tables = inventories["r1"].tables.clone();
    let server = tokio::spawn(async move {
        let (mut stream, _) = listener.accept().await.unwrap();
        let login = read_sentence(&mut stream).await;
        assert_eq!(login[0], "/login");
        let mut done = vec!["!done".into()];
        done.extend(login.into_iter().filter(|w| w.starts_with(".tag=")));
        send_sentence(&mut stream, done).await;
        for _ in 0..14 {
            let command = read_sentence(&mut stream).await;
            assert!(command[0].ends_with("/print"));
            let tag = command
                .iter()
                .find(|w| w.starts_with(".tag="))
                .unwrap()
                .clone();
            if let Some(rows) = tables.get(&command[0]) {
                for row in rows {
                    let mut reply = vec!["!re".into(), tag.clone()];
                    reply.extend(row.iter().map(|(k, v)| format!("={k}={v}")));
                    send_sentence(&mut stream, reply).await;
                }
            }
            send_sentence(&mut stream, vec!["!done".into(), tag]).await;
        }
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
    });
    store::save_settings(dir.path(), &settings).unwrap();
    let state = Arc::new(Mutex::new(App::open(dir.path().into()).unwrap()));
    tokio::time::timeout(
        std::time::Duration::from_secs(5),
        orchestrator::cycle(state.clone()),
    )
    .await
    .unwrap();
    assert_eq!(store::load_snapshot(dir.path()).unwrap().len(), 3);
    assert!(state.lock().await.inventory_errors.is_empty());
    assert!(
        state
            .lock()
            .await
            .status
            .contains("3 normalized subscribers")
    );
    server.await.unwrap();
}
