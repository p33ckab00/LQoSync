use lqosync3::{
    system_health::{Hub, Phase},
    write_receipt::{FileOutcome, Meta, Tracker},
};
use std::path::PathBuf;
fn meta() -> Meta {
    Meta {
        cycle_seq: 1,
        config_digest: "config-1".into(),
        notice: None,
    }
}
fn outputs(dir: &std::path::Path) -> Vec<(PathBuf, String)> {
    vec![
        (dir.join("network.json"), "{}".into()),
        (dir.join("ShapedDevices.csv"), "csv".into()),
    ]
}
#[test]
fn unchanged_cycle_keeps_the_operation_identity_and_does_not_emit_a_write() {
    let d = tempfile::tempdir().unwrap();
    let out = outputs(&d.path().join("live"));
    let mut tracker = Tracker::load(d.path()).unwrap();
    let result = tracker.write(&out, meta(), 10, &Hub::default());
    assert!(result.changed);
    assert!(result.error.is_none());
    let id = tracker.current().unwrap().write_id.clone();
    let second = tracker.write(&out, meta(), 11, &Hub::default());
    assert!(!second.changed);
    assert!(second.error.is_none());
    assert_eq!(tracker.current().unwrap().write_id, id);
    assert_eq!(tracker.current().unwrap().phase, Phase::Pending);
}
#[test]
fn partial_pair_failure_retains_the_completed_file_outcome() {
    let d = tempfile::tempdir().unwrap();
    let mut out = outputs(&d.path().join("live"));
    std::fs::write(d.path().join("blocked"), "file").unwrap();
    out[1].0 = d.path().join("blocked/child");
    let mut tracker = Tracker::load(d.path()).unwrap();
    let r = tracker.write(&out, meta(), 10, &Hub::default());
    assert!(r.changed);
    assert!(r.error.is_some());
    let current = tracker.current().unwrap();
    assert_eq!(current.phase, Phase::WriteFailed);
    assert_eq!(current.files[0].outcome, FileOutcome::Written);
    assert_eq!(current.files[1].outcome, FileOutcome::Failed);
}
#[test]
fn restart_recovers_identity_but_interrupted_intent_never_becomes_applied() {
    let d = tempfile::tempdir().unwrap();
    let out = outputs(&d.path().join("live"));
    let mut tracker = Tracker::load(d.path()).unwrap();
    tracker.write(&out, meta(), 10, &Hub::default());
    let id = tracker.current().unwrap().write_id.clone();
    let mut reloaded = Tracker::load(d.path()).unwrap();
    assert_eq!(reloaded.current().unwrap().write_id, id);
    assert_ne!(reloaded.current().unwrap().phase, Phase::Applied);
    reloaded
        .transition(Phase::ImportConfirmed, 11, &Hub::default())
        .unwrap();
    assert_eq!(reloaded.current().unwrap().phase, Phase::ImportConfirmed);
}
#[test]
fn superseding_update_keeps_a_terminal_record_of_the_previous_operation() {
    let d = tempfile::tempdir().unwrap();
    let mut out = outputs(&d.path().join("live"));
    let hub = Hub::default();
    let mut tracker = Tracker::load(d.path()).unwrap();
    tracker.write(&out, meta(), 10, &hub);
    let old = tracker.current().unwrap().write_id.clone();
    out[0].1 = "{\"new\":1}".into();
    tracker.write(&out, meta(), 11, &hub);
    assert_ne!(tracker.current().unwrap().write_id, old);
    let old_event = std::fs::read_to_string(
        d.path()
            .join("write-events")
            .join(format!("{old}-SUPERSEDED.json")),
    )
    .unwrap();
    assert!(old_event.contains("SUPERSEDED"));
}
#[test]
fn receipts_are_private_and_hashes_are_sha256() {
    use std::os::unix::fs::PermissionsExt;
    let d = tempfile::tempdir().unwrap();
    let mut tracker = Tracker::load(d.path()).unwrap();
    tracker.write(
        &outputs(&d.path().join("live")),
        meta(),
        10,
        &Hub::default(),
    );
    let path = d.path().join("write-receipt.json");
    assert_eq!(
        std::fs::metadata(path).unwrap().permissions().mode() & 0o777,
        0o600
    );
    assert_eq!(
        tracker.current().unwrap().files[0].sha256,
        "44136fa355b3678a1146ad16f7e8649e94fb4fc21fe77e8310c060f61caaff8a"
    );
}

#[test]
fn network_only_update_keeps_the_csv_outcome_unchanged() {
    let d = tempfile::tempdir().unwrap();
    let mut out = outputs(&d.path().join("live"));
    let hub = Hub::default();
    let mut tracker = Tracker::load(d.path()).unwrap();
    tracker.write(&out, meta(), 10, &hub);
    out[0].1 = "{\"changed\":true}".into();
    let r = tracker.write(&out, meta(), 11, &hub);
    assert!(r.changed);
    assert_eq!(
        tracker.current().unwrap().files[1].outcome,
        FileOutcome::Unchanged
    );
}
#[test]
fn event_capacity_is_visible_and_does_not_block_file_replacement() {
    let d = tempfile::tempdir().unwrap();
    let events = d.path().join("write-events");
    std::fs::create_dir(&events).unwrap();
    for i in 0..lqosync3::write_receipt::MAX_EVENTS {
        std::fs::write(events.join(format!("{i}.json")), "{}").unwrap();
    }
    let hub = Hub::default();
    let mut tracker = Tracker::load(d.path()).unwrap();
    let r = tracker.write(&outputs(&d.path().join("live")), meta(), 10, &hub);
    assert!(r.changed);
    assert!(r.error.is_none());
    assert!(r.tracking_error.is_some());
    assert!(
        hub.snapshot(10, None)
            .components
            .iter()
            .any(|c| c.reason == "write_receipt_storage_failed")
    );
}
#[test]
fn interrupted_intent_cannot_be_promoted_without_reconciliation() {
    let d = tempfile::tempdir().unwrap();
    let hub = Hub::default();
    let mut tracker = Tracker::load(d.path()).unwrap();
    tracker.write(&outputs(&d.path().join("live")), meta(), 10, &hub);
    let path = d.path().join("write-receipt.json");
    let mut value: serde_json::Value =
        serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
    value["phase"] = serde_json::json!("INTENT");
    value["files"][1]["outcome"] = serde_json::json!("PENDING");
    std::fs::write(&path, value.to_string()).unwrap();
    let mut recovered = Tracker::load(d.path()).unwrap();
    assert_eq!(recovered.current().unwrap().phase, Phase::Interrupted);
    assert!(
        recovered
            .transition(Phase::ImportConfirmed, 11, &hub)
            .is_err()
    );
    assert!(recovered.transition(Phase::Applied, 11, &hub).is_err());
}

#[test]
fn interrupted_complete_files_can_be_reconciled_without_another_write() {
    let d = tempfile::tempdir().unwrap();
    let hub = Hub::default();
    let mut tracker = Tracker::load(d.path()).unwrap();
    let out = outputs(&d.path().join("live"));
    tracker.write(&out, meta(), 10, &hub);
    let path = d.path().join("write-receipt.json");
    let mut value: serde_json::Value =
        serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
    value["phase"] = serde_json::json!("INTENT");
    value["files"][1]["outcome"] = serde_json::json!("PENDING");
    std::fs::write(&path, value.to_string()).unwrap();
    let mut recovered = Tracker::load(d.path()).unwrap();
    assert!(recovered.reconcile(11, &hub).unwrap());
    assert_eq!(
        recovered.current().unwrap().files[1].outcome,
        FileOutcome::Reconciled
    );
    assert_eq!(recovered.current().unwrap().phase, Phase::Pending);
    assert!(!recovered.reconcile(12, &hub).unwrap());
}

#[test]
fn a_crash_between_receipt_and_event_can_recover_the_same_identity() {
    let d = tempfile::tempdir().unwrap();
    let hub = Hub::default();
    let mut tracker = Tracker::load(d.path()).unwrap();
    tracker.write(&outputs(&d.path().join("live")), meta(), 10, &hub);
    let id = tracker.current().unwrap().write_id.clone();
    let event = d
        .path()
        .join("write-events")
        .join(format!("{id}-PENDING.json"));
    std::fs::remove_file(&event).unwrap();
    let recovered = Tracker::load(d.path()).unwrap();
    recovered.recover_events(&hub).unwrap();
    assert!(event.exists());
    assert_eq!(recovered.current().unwrap().write_id, id);
}

#[test]
fn csv_only_update_does_not_claim_a_network_replacement() {
    let dir = tempfile::tempdir().unwrap();
    let hub = Hub::default();
    let mut tracker = Tracker::load(dir.path()).unwrap();
    let mut pair = outputs(&dir.path().join("live"));
    tracker.write(&pair, meta(), 10, &hub);
    pair[1].1 = "changed csv".into();
    tracker.write(&pair, meta(), 11, &hub);
    assert_eq!(
        tracker.current().unwrap().files[0].outcome,
        FileOutcome::Unchanged
    );
    assert_eq!(
        tracker.current().unwrap().files[1].outcome,
        FileOutcome::Written
    );
}
