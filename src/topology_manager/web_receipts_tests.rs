use super::*;
use crate::topology_manager::store::RegistryStore;
use std::os::unix::fs::PermissionsExt;

fn command(name: &str) -> WebCommand {
    WebCommand::PlanCreate {
        expected_revision: 0,
        name: name.into(),
        down_mbps: 100,
        up_mbps: 50,
    }
}

const KEY: &str = "0f8b5a3c-1d2e-4f60-9a7b-123456789abc";

/// A store, its receipts, and a changed candidate document.
fn setup() -> (tempfile::TempDir, RegistryStore, Receipts, RegistryDocument) {
    let root = tempfile::tempdir().unwrap();
    let store = RegistryStore::open(root.path()).unwrap();
    let receipts = Receipts::open(store.owner_directory()).unwrap();
    let mut next = store.document().unwrap().clone();
    next.next_id = 20;
    (root, store, receipts, next)
}

fn begin_new(receipts: &Receipts, store: &RegistryStore) -> Record {
    match receipts
        .begin(
            KEY,
            "alice",
            &command("Gold"),
            store.document().unwrap().revision,
            1,
        )
        .unwrap()
    {
        Begin::New(record) => record,
        Begin::Existing(r) => panic!("existing {r:?}"),
    }
}

fn commit_prepared(
    receipts: &Receipts,
    store: &mut RegistryStore,
    record: &Record,
    next: RegistryDocument,
) {
    let mut hook = |doc: &RegistryDocument| receipts.prepare(record, doc);
    store.commit_observed(0, next, &mut hook).unwrap();
}

#[test]
fn the_receipt_directory_is_private_and_keys_are_checked() {
    let (root, _store, receipts, _) = setup();
    let dir = root.path().join("topology-manager/web-receipts");
    assert_eq!(
        std::fs::metadata(&dir).unwrap().permissions().mode() & 0o777,
        0o700
    );
    for bad in [
        "",
        "short",
        "../../etc/passwd",
        "a b c d e f g h i j k l m n",
        &"x".repeat(65),
    ] {
        assert!(
            receipts
                .begin(bad, "alice", &command("Gold"), 0, 1)
                .is_err(),
            "{bad}"
        );
    }
    assert!(receipts.get("../registry").unwrap().is_none());
}

#[test]
fn the_same_key_and_payload_returns_the_same_receipt_without_a_second_commit() {
    let (_root, mut store, receipts, next) = setup();
    let record = begin_new(&receipts, &store);
    commit_prepared(&receipts, &mut store, &record, next);
    let saved = receipts
        .finish_saved(&record, 1, Some("plan-1".into()))
        .unwrap();
    assert_eq!(saved.view().state, "saved");
    match receipts
        .begin(KEY, "alice", &command("Gold"), 1, 2)
        .unwrap()
    {
        Begin::Existing(r) => assert_eq!(r.view(), saved.view()),
        Begin::New(_) => panic!("a repeated key must not start a second mutation"),
    }
    assert_eq!(store.document().unwrap().revision, 1);
}

#[test]
fn a_reused_key_with_another_payload_or_actor_is_refused() {
    let (_root, store, receipts, _) = setup();
    begin_new(&receipts, &store);
    for (actor, cmd) in [("alice", command("Silver")), ("mallory", command("Gold"))] {
        let error = receipts.begin(KEY, actor, &cmd, 0, 2).unwrap_err();
        assert!(error.chain().any(|c| c.is::<KeyReused>()), "{error:#}");
    }
    // Another actor cannot read the receipt.
    assert!(
        receipts
            .lookup(KEY, "mallory", store.document().unwrap())
            .unwrap()
            .is_none()
    );
}

#[test]
fn a_crash_before_the_registry_write_is_not_committed() {
    // Crash after the intent; and crash after `prepared` but before the registry rename.
    for prepared in [false, true] {
        let (_root, store, receipts, next) = setup();
        let record = begin_new(&receipts, &store);
        if prepared {
            receipts
                .prepare(&record, &store.preview_for_test(0, next))
                .unwrap();
        }
        assert_eq!(receipts.recover(store.document().unwrap()).unwrap(), 1);
        let r = receipts
            .lookup(KEY, "alice", store.document().unwrap())
            .unwrap()
            .unwrap();
        assert_eq!(r.view().state, "not_committed", "prepared={prepared}");
        assert_eq!(store.document().unwrap().revision, 0);
    }
}

#[test]
fn a_crash_after_the_registry_write_resolves_to_saved_once() {
    let (root, mut store, receipts, next) = setup();
    let record = begin_new(&receipts, &store);
    commit_prepared(&receipts, &mut store, &record, next);
    // The response and the `saved` receipt are lost. The service restarts.
    drop(store);
    let store = RegistryStore::open(root.path()).unwrap();
    assert_eq!(receipts.recover(store.document().unwrap()).unwrap(), 1);
    let view = receipts
        .lookup(KEY, "alice", store.document().unwrap())
        .unwrap()
        .unwrap()
        .view();
    assert_eq!(
        (view.state.as_str(), view.revision.as_deref()),
        ("saved", Some("1"))
    );
    // Recovery is repeatable, and a retry finds the receipt, not a new mutation.
    assert_eq!(receipts.recover(store.document().unwrap()).unwrap(), 0);
    assert!(matches!(
        receipts
            .begin(KEY, "alice", &command("Gold"), 1, 3)
            .unwrap(),
        Begin::Existing(_)
    ));
}

#[test]
fn a_later_commit_after_a_lost_receipt_gives_outcome_unknown() {
    let (_root, mut store, receipts, next) = setup();
    let record = begin_new(&receipts, &store);
    commit_prepared(&receipts, &mut store, &record, next);
    let mut later = store.document().unwrap().clone();
    later.next_id = 30;
    store.commit(1, later).unwrap();
    receipts.recover(store.document().unwrap()).unwrap();
    let view = receipts
        .lookup(KEY, "alice", store.document().unwrap())
        .unwrap()
        .unwrap()
        .view();
    assert_eq!(
        (view.state.as_str(), view.revision.as_deref()),
        ("outcome_unknown", None)
    );
}

#[test]
fn a_failed_mutation_keeps_its_safe_reason() {
    let (_root, store, receipts, _) = setup();
    let record = begin_new(&receipts, &store);
    let failed = receipts.finish_failed(&record, "conflict").unwrap();
    let view = failed.view();
    assert_eq!(
        (view.state.as_str(), view.error.as_deref()),
        ("failed", Some("conflict"))
    );
    assert_eq!(view.base_revision, "0");
}

#[test]
fn old_resolved_receipts_are_pruned_and_unresolved_ones_stay() {
    let (_root, store, receipts, _) = setup();
    let record = begin_new(&receipts, &store);
    receipts.finish_failed(&record, "conflict").unwrap();
    let other = "1f8b5a3c-1d2e-4f60-9a7b-123456789abc";
    receipts
        .begin(other, "alice", &command("Gold"), 0, 1)
        .unwrap();
    assert_eq!(receipts.prune(1 + KEEP_SECS + 1).unwrap(), 1);
    assert!(receipts.get(KEY).unwrap().is_none());
    assert!(receipts.get(other).unwrap().is_some());
}

#[test]
fn the_fingerprint_follows_the_document() {
    let a = RegistryDocument::default();
    let mut b = a.clone();
    assert_eq!(fingerprint(&a), fingerprint(&b));
    b.next_id = 2;
    assert_ne!(fingerprint(&a), fingerprint(&b));
}
