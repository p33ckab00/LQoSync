use lqosync_v3::core_coordination::Coordinator;
#[test]
fn shadow_ticket_cannot_authorize_a_mutation() {
    let d = tempfile::tempdir().unwrap();
    let coordinator = Coordinator::default();
    coordinator.configure("settings", true);
    let ticket = coordinator.begin(d.path(), "settings").unwrap();
    assert!(ticket.check("settings").is_ok());
    assert!(ticket.require_mutation().is_err());
}

#[test]
fn pause_then_resume_does_not_restore_an_old_ticket() {
    let d = tempfile::tempdir().unwrap();
    let coordinator = Coordinator::default();
    coordinator.configure("a", true);
    let ticket = coordinator.begin(d.path(), "a").unwrap();
    coordinator.configure("a", false);
    assert!(ticket.check("a").is_err());
    coordinator.configure("a", true);
    assert!(ticket.check("a").is_err());
    drop(ticket);
    assert!(coordinator.begin(d.path(), "a").is_ok());
}
#[test]
fn competing_consoles_and_maintenance_cannot_share_shadow_work() {
    let d = tempfile::tempdir().unwrap();
    let first = Coordinator::default();
    let second = Coordinator::default();
    first.configure("a", true);
    second.configure("a", true);
    let ticket = first.begin(d.path(), "a").unwrap();
    assert!(second.begin(d.path(), "a").is_err());
    assert!(lqosync_v3::migration_fence::Lease::exclusive(d.path()).is_err());
    first.configure("b", true);
    assert!(ticket.check("a").is_err());
    drop(ticket);
    assert!(second.begin(d.path(), "a").is_ok());
}
#[tokio::test]
async fn cancelled_async_owner_releases_the_shadow_slot() {
    let d = tempfile::tempdir().unwrap();
    let coordinator = Coordinator::default();
    coordinator.configure("a", true);
    let ticket = coordinator.begin(d.path(), "a").unwrap();
    let (started, ready) = tokio::sync::oneshot::channel();
    let task = tokio::spawn(async move {
        let _ticket = ticket;
        let _ = started.send(());
        std::future::pending::<()>().await;
    });
    ready.await.unwrap();
    task.abort();
    assert!(task.await.unwrap_err().is_cancelled());
    assert!(coordinator.begin(d.path(), "a").is_ok());
}

#[test]
fn shadow_process_probe() {
    let Ok(root) = std::env::var("LQOSYNC_SHADOW_PROBE_ROOT") else {
        return;
    };
    let coordinator = Coordinator::default();
    coordinator.configure("probe", true);
    let _ticket = coordinator
        .begin(std::path::Path::new(&root), "probe")
        .unwrap();
    println!("SHADOW_READY");
    loop {
        std::thread::park();
    }
}
#[test]
fn killed_process_releases_the_shadow_slot_without_marker_removal() {
    use std::io::BufRead;
    let d = tempfile::tempdir().unwrap();
    let mut child = std::process::Command::new(std::env::current_exe().unwrap())
        .args(["--exact", "shadow_process_probe", "--nocapture"])
        .env("LQOSYNC_SHADOW_PROBE_ROOT", d.path())
        .stdout(std::process::Stdio::piped())
        .spawn()
        .unwrap();
    let mut ready = false;
    for line in std::io::BufReader::new(child.stdout.take().unwrap()).lines() {
        if line.unwrap() == "SHADOW_READY" {
            ready = true;
            break;
        }
    }
    assert!(ready);
    let coordinator = Coordinator::default();
    coordinator.configure("probe", true);
    assert!(coordinator.begin(d.path(), "probe").is_err());
    child.kill().unwrap();
    child.wait().unwrap();
    assert!(coordinator.begin(d.path(), "probe").is_ok());
    assert!(!d.path().join("state/core-migration-active.json").exists());
}

#[test]
fn console_coordinator_cannot_be_reused_for_another_data_directory() {
    let d = tempfile::tempdir().unwrap();
    let other = tempfile::tempdir().unwrap();
    let coordinator = Coordinator::for_root(d.path()).unwrap();
    coordinator.configure("revision", true);
    assert!(coordinator.begin(other.path(), "revision").is_err());
    assert!(!other.path().join("state").exists());
    assert!(coordinator.begin(d.path(), "revision").is_ok());
}
