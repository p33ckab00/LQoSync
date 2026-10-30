use lqosync_v3::{
    core_migration::Member,
    core_recovery,
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
    unverified: bool,
    calls: usize,
}
impl Transport for Mock {
    async fn observe(&mut self) -> anyhow::Result<Evidence> {
        let now = lqosync_v3::orchestrator::now();
        Ok(Evidence {
            context: if self.unverified && self.calls > 0 {
                "changed"
            } else {
                "context"
            }
            .into(),
            captured_at: now,
            valid_until: now + 30,
            rows: self.rows.clone(),
        })
    }
    async fn execute(
        &mut self,
        operation: &Operation,
        _: &Evidence,
    ) -> anyhow::Result<Option<String>> {
        self.calls += 1;
        if self.lost {
            anyhow::bail!("synthetic unknown outcome");
        }
        match operation {
            Operation::Add { router, member } => {
                let mut row = member.clone();
                row.id = "*AA".into();
                self.rows.get_mut(router).unwrap().push(row);
                Ok(Some("*AA".into()))
            }
            Operation::Remove { router, before } => {
                self.rows
                    .get_mut(router)
                    .unwrap()
                    .retain(|r| r.id != before.id);
                Ok(None)
            }
            _ => unreachable!(),
        }
    }
}
async fn journal(root: &std::path::Path, mode: &str) -> std::path::PathBuf {
    let batch = batch(
        vec![Operation::Add {
            router: "core".into(),
            member: member("", "one", "wan1"),
        }],
        vec![],
    );
    let revision = batch.revision().unwrap();
    let mut worker = Executor::open(root, batch, &revision).unwrap();
    let mut io = Mock {
        rows: BTreeMap::from([("core".into(), vec![])]),
        lost: mode == "lost",
        unverified: mode == "unverified",
        calls: 0,
    };
    if mode == "saved" {
        worker.forward(&mut io).await.unwrap();
        worker.rollback(&mut io).await.unwrap();
        worker.finish().unwrap();
    } else {
        assert!(worker.forward(&mut io).await.is_err());
        drop(worker);
    }
    root.join("state/core-writer")
        .join(format!("{revision}.json"))
}
#[test]
fn empty_directory_inspection_creates_no_files() {
    let d = tempfile::tempdir().unwrap();
    let report = core_recovery::inspect(d.path()).unwrap();
    assert!(report.journals.is_empty());
    assert_eq!(std::fs::read_dir(d.path()).unwrap().count(), 0);
}
#[tokio::test]
async fn saved_and_uncertain_results_remain_distinct_and_bytes_unchanged() {
    for mode in ["lost", "unverified", "saved"] {
        let d = tempfile::tempdir().unwrap();
        let path = journal(d.path(), mode).await;
        let bytes = std::fs::read(&path).unwrap();
        let marker = d.path().join("state/core-migration-active.json");
        let before = std::fs::read(&marker).ok();
        let report = core_recovery::inspect(d.path()).unwrap();
        assert_eq!(report.journals.len(), 1);
        let j = &report.journals[0];
        assert_eq!(j.pending.is_some(), mode == "lost");
        assert_eq!(j.saved_verified, mode == "saved");
        assert_eq!(j.last_known.is_some(), mode != "lost");
        if mode == "saved" {
            assert_eq!(j.direction, "Rollback");
            assert_eq!(j.last_known.as_ref().unwrap().kind, "Remove");
        }
        assert_eq!(std::fs::read(&path).unwrap(), bytes);
        assert_eq!(std::fs::read(&marker).ok(), before);
    }
}
#[tokio::test]
async fn corrupt_history_and_missing_active_journal_do_not_claim_verification() {
    let d = tempfile::tempdir().unwrap();
    let path = journal(d.path(), "lost").await;
    let mut value: serde_json::Value =
        serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
    value["verified"] = true.into();
    std::fs::write(&path, serde_json::to_vec(&value).unwrap()).unwrap();
    let report = core_recovery::inspect(d.path()).unwrap();
    assert!(report.journals.is_empty());
    assert!(!report.issues.is_empty());
    std::fs::remove_file(&path).unwrap();
    let report = core_recovery::inspect(d.path()).unwrap();
    assert!(report.active_revision.is_some());
    assert!(!report.issues.is_empty());
}
#[test]
fn symlinks_fifo_and_oversized_files_are_not_followed() {
    use std::os::unix::fs::symlink;
    let d = tempfile::tempdir().unwrap();
    let elsewhere = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(d.path().join("state/core-writer")).unwrap();
    let name = "80dff65e-3400-4c10-a702-46b19a9d8666.json";
    std::fs::write(elsewhere.path().join("secret"), "NEVER_DISPLAY_THIS").unwrap();
    let path = d.path().join("state/core-writer").join(name);
    symlink(elsewhere.path().join("secret"), &path).unwrap();
    assert!(!core_recovery::inspect(d.path()).unwrap().issues.is_empty());
    std::fs::remove_file(&path).unwrap();
    let c = std::ffi::CString::new(path.to_str().unwrap()).unwrap();
    // SAFETY: c is a valid terminated pathname in a private temporary test directory.
    assert_eq!(unsafe { libc::mkfifo(c.as_ptr(), 0o600) }, 0);
    assert!(!core_recovery::inspect(d.path()).unwrap().issues.is_empty());
    std::fs::remove_file(&path).unwrap();
    let f = std::fs::File::create(&path).unwrap();
    f.set_len(64 * 1024 * 1024 + 1).unwrap();
    assert!(!core_recovery::inspect(d.path()).unwrap().issues.is_empty());
    std::fs::remove_dir_all(d.path().join("state")).unwrap();
    symlink(elsewhere.path(), d.path().join("state")).unwrap();
    assert!(core_recovery::inspect(d.path()).is_err());
}
#[tokio::test]
async fn recovery_viewer_is_get_only_even_when_normal_console_startup_is_blocked() {
    use axum::{
        body::Body,
        http::{Request, StatusCode},
    };
    use http_body_util::BodyExt;
    use tower::ServiceExt;
    let d = tempfile::tempdir().unwrap();
    let path = journal(d.path(), "lost").await;
    let before = std::fs::read(&path).unwrap();
    assert!(lqosync_v3::web::App::open(d.path().into()).is_err());
    let viewer = lqosync_v3::core_recovery_web::viewer(d.path().into());
    let out = viewer
        .clone()
        .oneshot(
            Request::builder()
                .uri("/")
                .header("host", "127.0.0.1:19306")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(out.status(), StatusCode::OK);
    assert_eq!(out.headers()["cache-control"], "no-store");
    let text =
        String::from_utf8(out.into_body().collect().await.unwrap().to_bytes().to_vec()).unwrap();
    assert!(text.contains("Approve / resume — locked"));
    assert!(text.contains("Pending intent"));
    for endpoint in [
        "/",
        "/core-recovery",
        "/approve",
        "/resume",
        "/rollback",
        "/marker/remove",
    ] {
        let out = viewer
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(endpoint)
                    .header("host", "127.0.0.1")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert!([StatusCode::METHOD_NOT_ALLOWED, StatusCode::NOT_FOUND].contains(&out.status()));
    }
    let out = viewer
        .oneshot(
            Request::builder()
                .uri("/")
                .header("host", "attacker.example")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(out.status(), StatusCode::FORBIDDEN);
    assert_eq!(std::fs::read(&path).unwrap(), before);
    assert!(d.path().join("state/core-migration-active.json").exists());
}

#[test]
fn invalid_utf8_exhausts_the_remaining_inspection_budget() {
    let d = tempfile::tempdir().unwrap();
    let journals = d.path().join("state/core-writer");
    std::fs::create_dir_all(&journals).unwrap();
    for _ in 0..2 {
        std::fs::write(
            journals.join(format!("{}.json", uuid::Uuid::new_v4())),
            [255],
        )
        .unwrap();
    }
    let report = core_recovery::inspect(d.path()).unwrap();
    assert!(report.limited);
    assert_eq!(report.issues.len(), 1);
}

#[tokio::test]
async fn recovery_assessment_binds_exact_history_and_fresh_evidence_without_approval() {
    let d = tempfile::tempdir().unwrap();
    let path = journal(d.path(), "unverified").await;
    let raw = std::fs::read_to_string(&path).unwrap();
    let revision = path.file_stem().unwrap().to_str().unwrap();
    let now = lqosync_v3::orchestrator::now();
    let fresh = || Evidence {
        context: "context".into(),
        captured_at: now,
        valid_until: now + 30,
        rows: BTreeMap::from([("core".into(), vec![member("*AA", "one", "wan1")])]),
    };
    let review = lqosync_v3::core_writer::assess_recovery(&raw, revision, fresh(), now).unwrap();
    assert!(review.next_forward.is_none());
    assert_eq!(
        review.next_rollback.as_ref().unwrap().row_id.as_deref(),
        Some("*AA")
    );
    assert!(review.require_execution_approval().is_err());
    assert!(!review.is_fresh(now + 31));
    for change in ["context", "row", "stale", "future", "expired", "incomplete"] {
        let mut e = fresh();
        match change {
            "context" => e.context = "changed".into(),
            "row" => e.rows.get_mut("core").unwrap()[0].list = "two".into(),
            "stale" => e.captured_at = now - 31,
            "future" => e.captured_at = now + 1,
            "expired" => e.valid_until = now - 1,
            "incomplete" => {
                e.rows.clear();
            }
            _ => unreachable!(),
        }
        assert!(
            lqosync_v3::core_writer::assess_recovery(&raw, revision, e, now).is_err(),
            "{change}"
        );
    }
    assert_eq!(std::fs::read_to_string(&path).unwrap(), raw);
    assert!(d.path().join("state/core-migration-active.json").exists());
}

#[tokio::test]
async fn matching_rows_cannot_resolve_an_unknown_recovery_outcome() {
    let d = tempfile::tempdir().unwrap();
    let path = journal(d.path(), "lost").await;
    let raw = std::fs::read_to_string(&path).unwrap();
    let now = lqosync_v3::orchestrator::now();
    let evidence = Evidence {
        context: "context".into(),
        captured_at: now,
        valid_until: now + 30,
        rows: BTreeMap::from([("core".into(), vec![])]),
    };
    let err = lqosync_v3::core_writer::assess_recovery(
        &raw,
        path.file_stem().unwrap().to_str().unwrap(),
        evidence,
        now,
    )
    .unwrap_err();
    assert!(err.to_string().contains("Unknown"));
}
