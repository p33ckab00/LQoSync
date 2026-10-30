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

#[test]
fn durable_pause_invalidates_other_coordinators_and_survives_reconfiguration() {
    let d = tempfile::tempdir().unwrap();
    let first = Coordinator::for_root(d.path()).unwrap();
    let second = Coordinator::for_root(d.path()).unwrap();
    first.configure("revision", true);
    second.configure("revision", true);
    let ticket = first.begin(d.path(), "revision").unwrap();
    second.pause_all().unwrap();
    assert!(ticket.check("revision").is_err());
    first.configure("revision", true);
    assert!(ticket.check("revision").is_err());
    drop(ticket);
    assert!(first.begin(d.path(), "revision").is_err());
    let restarted = Coordinator::for_root(d.path()).unwrap();
    restarted.configure("revision", true);
    assert!(restarted.begin(d.path(), "revision").is_err());
}

#[test]
fn shadow_ticket_cannot_revive_after_observed_control_loss() {
    let d = tempfile::tempdir().unwrap();
    let c = Coordinator::for_root(d.path()).unwrap();
    c.configure("revision", true);
    let ticket = c.begin(d.path(), "revision").unwrap();
    let remote = Coordinator::for_root(d.path()).unwrap();
    remote.configure("revision", true);
    remote.pause_all().unwrap();
    assert!(ticket.check("revision").is_err());
    std::fs::remove_file(d.path().join("state/core-control-state.json")).unwrap();
    assert!(ticket.check("revision").is_err());
}

#[test]
fn distributed_process_probe() {
    use std::io::Write;
    let Ok(root) = std::env::var("LQOSYNC_DISTRIBUTED_PROBE_ROOT") else {
        return;
    };
    let c = Coordinator::for_root(std::path::Path::new(&root)).unwrap();
    c.configure("probe", true);
    let ticket = c.begin(std::path::Path::new(&root), "probe").unwrap();
    println!("DISTRIBUTED_READY");
    std::io::stdout().flush().unwrap();
    let mut line = String::new();
    std::io::stdin().read_line(&mut line).unwrap();
    assert!(ticket.check("probe").is_err());
}
#[test]
fn pause_reaches_a_ticket_in_another_process() {
    use std::io::{BufRead, Write};
    let d = tempfile::tempdir().unwrap();
    let mut child = std::process::Command::new(std::env::current_exe().unwrap())
        .args(["--exact", "distributed_process_probe", "--nocapture"])
        .env("LQOSYNC_DISTRIBUTED_PROBE_ROOT", d.path())
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .spawn()
        .unwrap();
    let mut output = std::io::BufReader::new(child.stdout.take().unwrap());
    let ready = (&mut output)
        .lines()
        .any(|line| line.unwrap() == "DISTRIBUTED_READY");
    assert!(ready);
    let c = Coordinator::for_root(d.path()).unwrap();
    c.configure("probe", true);
    c.pause_all().unwrap();
    child.stdin.take().unwrap().write_all(b"check\n").unwrap();
    assert!(child.wait().unwrap().success());
}
