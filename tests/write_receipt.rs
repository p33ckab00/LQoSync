use lqosync3::{
    system_health::{Hub, Phase},
    write_receipt::{FileOutcome, Meta, Tracker},
};
use std::path::PathBuf;
fn meta() -> Meta {
    Meta {
        topology: None,
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
    assert!(!r.changed, "a staging failure replaces no file");
    assert!(r.error.is_some());
    let current = tracker.current().unwrap();
    assert_eq!(current.phase, Phase::WriteFailed);
    assert_eq!(current.files[0].outcome, FileOutcome::Pending);
    assert_eq!(current.files[1].outcome, FileOutcome::Failed);
    assert!(!out[0].0.exists(), "the first file is not replaced alone");
    assert!(stages(&d.path().join("live")).is_empty());
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

#[test]
fn failed_durable_intent_stops_both_file_replacements() {
    let d = tempfile::tempdir().unwrap();
    let mut tracker = Tracker::load(d.path()).unwrap();
    let live = d.path().join("live");
    let out = outputs(&live);
    std::fs::create_dir(d.path().join("write-receipt.json")).unwrap();
    let result = tracker.write(&out, meta(), 10, &Hub::default());
    assert!(result.error.is_some());
    assert!(!result.changed);
    assert!(!out[0].0.exists());
    assert!(!out[1].0.exists());
}

#[test]
fn unchanged_inputs_without_a_matching_receipt_get_a_trackable_attempt() {
    let d = tempfile::tempdir().unwrap();
    let out = outputs(d.path());
    for (path, text) in &out {
        std::fs::write(path, text).unwrap();
    }
    let mut tracker = Tracker::load(d.path()).unwrap();
    let result = tracker.write(&out, meta(), 10, &Hub::default());
    assert!(!result.changed);
    let receipt = tracker
        .current()
        .expect("matching inputs still need a receipt");
    assert!(
        receipt
            .files
            .iter()
            .all(|f| f.outcome == FileOutcome::Unchanged)
    );
    let id = receipt.write_id.clone();
    tracker.write(&out, meta(), 11, &Hub::default());
    assert_eq!(tracker.current().unwrap().write_id, id);
}

#[test]
fn old_metadata_has_no_invented_topology_context_and_invalid_context_cannot_load() {
    let old: Meta =
        serde_json::from_str(r#"{"cycle_seq":1,"config_digest":"old","notice":null}"#).unwrap();
    assert!(old.topology.is_none());
    let d = tempfile::tempdir().unwrap();
    let mut tracker = Tracker::load(d.path()).unwrap();
    tracker.write(&outputs(d.path()), old, 10, &Hub::default());
    let path = d.path().join("write-receipt.json");
    let mut value: serde_json::Value =
        serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
    assert!(value["meta"].get("topology").is_none());
    value["meta"]["topology"] = serde_json::json!({"policy_revision":1,"observation_generation":0,"registry_digest":"invalid"});
    std::fs::write(&path, value.to_string()).unwrap();
    assert!(Tracker::load(d.path()).is_err());
    value["meta"]["topology"] = serde_json::json!({"policy_revision":1,"observation_generation":1,"registry_digest":"a".repeat(64)});
    std::fs::write(&path, value.to_string()).unwrap();
    assert!(
        Tracker::load(d.path())
            .unwrap()
            .current()
            .unwrap()
            .meta
            .topology
            .as_ref()
            .unwrap()
            .static_source
            .is_none()
    );
    value["meta"]["topology"]["static_source"] = serde_json::json!({
        "path":"/owner/topology-manager/static-input.json","revision":0,"digest":"b".repeat(64)
    });
    std::fs::write(&path, value.to_string()).unwrap();
    assert!(Tracker::load(d.path()).is_err());
}

#[test]
fn compiler_binding_changes_create_new_receipts_without_replacing_matching_bytes() {
    let d = tempfile::tempdir().unwrap();
    let out = outputs(d.path());
    let hub = Hub::default();
    let mut tracker = Tracker::load(d.path()).unwrap();
    tracker.write(&out, meta(), 10, &hub);
    let legacy = tracker.current().unwrap().write_id.clone();
    let mut bound = meta();
    bound.topology = Some(lqosync3::write_receipt::TopologyContext {
        static_source: None,
        policy_revision: 1,
        observation_generation: 1,
        registry_digest: "a".repeat(64),
        compiler: Some(lqosync3::topology_manager::compiler::CompilerBinding {
            library: "/native/liblqos_python.so".into(),
            config: "/native/lqos.conf".into(),
            digest: "b".repeat(64),
            effective_config_digest: "d".repeat(64),
        }),
    });
    let result = tracker.write(&out, bound.clone(), 11, &hub);
    assert!(!result.changed && result.error.is_none());
    let first = tracker.current().unwrap().write_id.clone();
    assert_ne!(first, legacy);
    assert!(
        tracker
            .current()
            .unwrap()
            .files
            .iter()
            .all(|f| f.outcome == FileOutcome::Unchanged)
    );
    tracker.write(&out, bound.clone(), 12, &hub);
    assert_eq!(tracker.current().unwrap().write_id, first);
    bound
        .topology
        .as_mut()
        .unwrap()
        .compiler
        .as_mut()
        .unwrap()
        .digest = "c".repeat(64);
    tracker.write(&out, bound, 13, &hub);
    assert_ne!(tracker.current().unwrap().write_id, first);
}

#[test]
fn unchanged_outputs_keep_the_current_static_source_revision_after_restart() {
    let d = tempfile::tempdir().unwrap();
    let out = outputs(d.path());
    let hub = Hub::default();
    let mut tracker = Tracker::load(d.path()).unwrap();
    let mut bound = meta();
    bound.topology = Some(lqosync3::write_receipt::TopologyContext {
        compiler: None,
        static_source: Some(lqosync3::topology_manager::static_input::StaticBinding {
            path: "/owner/topology-manager/static-input.json".into(),
            revision: 1,
            digest: "a".repeat(64),
        }),
        policy_revision: 1,
        observation_generation: 1,
        registry_digest: "b".repeat(64),
    });
    assert!(tracker.write(&out, bound.clone(), 10, &hub).error.is_none());
    let first = tracker.current().unwrap().write_id.clone();
    bound
        .topology
        .as_mut()
        .unwrap()
        .static_source
        .as_mut()
        .unwrap()
        .revision = 2;
    let result = tracker.write(&out, bound, 11, &hub);
    assert!(!result.changed && result.error.is_none());
    assert_ne!(tracker.current().unwrap().write_id, first);
    drop(tracker);
    let tracker = Tracker::load(d.path()).unwrap();
    assert_eq!(
        tracker
            .current()
            .unwrap()
            .meta
            .topology
            .as_ref()
            .unwrap()
            .static_source
            .as_ref()
            .unwrap()
            .revision,
        2
    );
}

/// Stage files left in a directory.
fn stages(dir: &std::path::Path) -> Vec<String> {
    std::fs::read_dir(dir)
        .map(|r| {
            r.filter_map(|e| e.ok()?.file_name().into_string().ok())
                .filter(|n| n.contains(".stage-"))
                .collect()
        })
        .unwrap_or_default()
}

/// Write pair A, then pair B, then rewind the disk to a crash during B: `replaced` files hold B,
/// the others hold A, `staged` files have a B stage (or `corrupt` bytes), and the receipt is Intent.
fn crash(
    d: &std::path::Path,
    replaced: &[usize],
    staged: &[usize],
    corrupt: bool,
) -> Vec<(PathBuf, String)> {
    let live = d.join("live");
    let a = outputs(&live);
    let b = vec![
        (live.join("network.json"), "{\"b\":1}".to_string()),
        (live.join("ShapedDevices.csv"), "csv-b".to_string()),
    ];
    let mut tracker = Tracker::load(d).unwrap();
    tracker.write(&a, meta(), 10, &Hub::default());
    tracker.write(&b, meta(), 11, &Hub::default());
    let id = tracker.current().unwrap().write_id.clone();
    drop(tracker);
    let receipt_path = d.join("write-receipt.json");
    let mut receipt: serde_json::Value =
        serde_json::from_slice(&std::fs::read(&receipt_path).unwrap()).unwrap();
    receipt["phase"] = "INTENT".into();
    for i in 0..2 {
        if !replaced.contains(&i) {
            std::fs::write(&a[i].0, &a[i].1).unwrap();
            receipt["files"][i]["outcome"] = "PENDING".into();
        }
        if staged.contains(&i) {
            let bytes = if corrupt { "garbage" } else { b[i].1.as_str() };
            std::fs::write(lqosync3::files::stage_path(&b[i].0, &id), bytes).unwrap();
        }
    }
    lqosync3::files::write_secret(&receipt_path, &serde_json::to_vec(&receipt).unwrap()).unwrap();
    b
}

#[test]
fn a_write_stages_both_files_and_leaves_no_stage_behind() {
    let d = tempfile::tempdir().unwrap();
    let out = outputs(&d.path().join("live"));
    let mut tracker = Tracker::load(d.path()).unwrap();
    assert!(
        tracker
            .write(&out, meta(), 10, &Hub::default())
            .error
            .is_none()
    );
    assert!(stages(&d.path().join("live")).is_empty());
    assert_eq!(std::fs::read_to_string(&out[1].0).unwrap(), "csv");
}

#[test]
fn restart_completes_a_pair_interrupted_after_the_first_replacement() {
    let d = tempfile::tempdir().unwrap();
    let b = crash(d.path(), &[0], &[1], false);
    let mut tracker = Tracker::load(d.path()).unwrap();
    assert!(tracker.recovered_pair());
    for (path, text) in &b {
        assert_eq!(&std::fs::read_to_string(path).unwrap(), text);
    }
    assert!(stages(&d.path().join("live")).is_empty());
    assert!(tracker.reconcile(12, &Hub::default()).unwrap());
    assert_eq!(tracker.current().unwrap().phase, Phase::Pending);
}

#[test]
fn restart_completes_a_pair_interrupted_before_any_replacement() {
    let d = tempfile::tempdir().unwrap();
    let b = crash(d.path(), &[], &[0, 1], false);
    let tracker = Tracker::load(d.path()).unwrap();
    assert!(tracker.recovered_pair());
    for (path, text) in &b {
        assert_eq!(&std::fs::read_to_string(path).unwrap(), text);
    }
}

#[test]
fn restart_after_an_interrupted_staging_keeps_the_previous_pair() {
    let d = tempfile::tempdir().unwrap();
    let a = outputs(&d.path().join("live"));
    crash(d.path(), &[], &[0], false);
    let mut tracker = Tracker::load(d.path()).unwrap();
    assert!(!tracker.recovered_pair());
    for (path, text) in &a {
        assert_eq!(
            &std::fs::read_to_string(path).unwrap(),
            text,
            "the old pair stays whole"
        );
    }
    assert!(stages(&d.path().join("live")).is_empty());
    assert!(!tracker.reconcile(12, &Hub::default()).unwrap());
    assert_ne!(tracker.current().unwrap().phase, Phase::Applied);
}

#[test]
fn a_corrupt_stage_is_never_installed() {
    let d = tempfile::tempdir().unwrap();
    let a = outputs(&d.path().join("live"));
    crash(d.path(), &[0], &[1], true);
    let mut tracker = Tracker::load(d.path()).unwrap();
    assert!(!tracker.recovered_pair());
    assert_eq!(std::fs::read_to_string(&a[1].0).unwrap(), a[1].1);
    assert!(stages(&d.path().join("live")).is_empty());
    assert!(
        !tracker.reconcile(12, &Hub::default()).unwrap(),
        "a mixed pair is never confirmed"
    );
}
