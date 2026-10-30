use anyhow::{Result, bail};
use lqosync_v3::{core_migration::*, core_plan::Binding};
use std::collections::BTreeMap;
fn now() -> u64 {
    lqosync_v3::orchestrator::now()
}
fn member(id: &str, list: &str, address: &str) -> Member {
    Member {
        id: id.into(),
        list: list.into(),
        address: address.into(),
        comment: "manual operator row".into(),
        dynamic: false,
        disabled: false,
        timeout: None,
    }
}
fn fixture() -> (Plan, Mock) {
    let evidence = Evidence {
        revision: "settings+sessions-v1".into(),
        captured_at: now(),
        rows: BTreeMap::from([(
            "core".into(),
            vec![
                member("*1", "wan-a", "192.0.2.0/29"),
                member("*2", "outside", "198.51.100.0/24"),
            ],
        )]),
    };
    let plan = prepare(evidence.clone(), "192.0.2.3/32", "*1").unwrap();
    (
        plan,
        Mock {
            evidence,
            writes: 0,
            fail_at: None,
            lose_reply: false,
            next_id: 10,
            reads: 0,
            fail_read_at: None,
            refresh_time: true,
            journal_fault: None,
            read_delay_ms: 0,
        },
    )
}
fn prepare(evidence: Evidence, address: &str, id: &str) -> Result<Plan> {
    Plan::prepare(
        evidence,
        vec![Binding {
            wan: "a".into(),
            router: "core".into(),
            list: "wan-a".into(),
            table: "table-a".into(),
        }],
        "00000000-0000-4000-8000-000000000001".into(),
        vec![Host {
            router: "core".into(),
            list: "wan-a".into(),
            wan: "a".into(),
            subscriber: "00000000-0000-4000-8000-000000000002".into(),
            address: address.into(),
        }],
        vec![RowKey {
            router: "core".into(),
            id: id.into(),
        }],
        120,
    )
}
struct Mock {
    evidence: Evidence,
    writes: usize,
    fail_at: Option<usize>,
    lose_reply: bool,
    next_id: usize,
    reads: usize,
    fail_read_at: Option<usize>,
    refresh_time: bool,
    journal_fault: Option<(usize, std::path::PathBuf)>,
    read_delay_ms: u64,
}
impl Mock {
    fn before_write(&mut self) -> Result<()> {
        self.writes += 1;
        if self.fail_at == Some(self.writes) && !self.lose_reply {
            bail!("injected failure")
        }
        Ok(())
    }
    fn after_write(&self) -> Result<()> {
        assert!(
            self.evidence.rows["core"].iter().any(|r| r.list == "wan-a"
                && (r.address == "192.0.2.0/29" || r.address == "192.0.2.3/32")),
            "Selected client lost membership during a write or rollback"
        );
        if let Some((at, path)) = &self.journal_fault
            && *at == self.writes
        {
            std::fs::rename(path, path.with_extension("saved"))?;
            std::fs::create_dir(path)?;
        }

        if self.fail_at == Some(self.writes) && self.lose_reply {
            bail!("reply lost after write")
        }
        Ok(())
    }
}
impl Transport for Mock {
    async fn observe(&mut self) -> Result<Evidence> {
        tokio::time::sleep(std::time::Duration::from_millis(self.read_delay_ms)).await;
        self.reads += 1;
        if self.fail_read_at == Some(self.reads) {
            bail!("injected read failure")
        }
        if self.refresh_time {
            self.evidence.captured_at = now();
        }
        Ok(self.evidence.clone())
    }
    async fn add(&mut self, router: &str, entry: &Member) -> Result<String> {
        self.before_write()?;
        let mut entry = entry.clone();
        entry.id = format!("*{}", self.next_id);
        self.next_id += 1;
        let id = entry.id.clone();
        self.evidence.rows.get_mut(router).unwrap().push(entry);
        self.after_write()?;
        Ok(id)
    }
    async fn set_address(&mut self, router: &str, expected: &Member, address: &str) -> Result<()> {
        self.before_write()?;
        let entry = self
            .evidence
            .rows
            .get_mut(router)
            .unwrap()
            .iter_mut()
            .find(|r| r.id == expected.id)
            .unwrap();
        assert_eq!(entry, expected);
        entry.address = address.into();
        self.after_write()
    }
    async fn remove(&mut self, router: &str, expected: &Member) -> Result<()> {
        self.before_write()?;
        let entries = self.evidence.rows.get_mut(router).unwrap();
        let index = entries
            .iter()
            .position(|r| r == expected)
            .expect("exact returned row");
        entries.remove(index);
        self.after_write()
    }
}
#[tokio::test]
async fn forward_rollback_and_duplicate_execution() {
    let (plan, mut io) = fixture();
    let original = io.evidence.rows.clone();
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("journal.json");
    let mut executor = Executor::create(&path, plan.clone(), &plan.revision().unwrap()).unwrap();
    executor.forward(&mut io).await.unwrap();
    assert_eq!(io.writes, 3);
    assert_eq!(executor.status(), Status::ForwardComplete);
    let writes = io.writes;
    executor.forward(&mut io).await.unwrap();
    assert_eq!(io.writes, writes);
    assert!(Executor::create(&path, plan.clone(), &plan.revision().unwrap()).is_err());
    executor.rollback(&mut io).await.unwrap();
    assert_eq!(io.evidence.rows, original);
    assert_eq!(executor.status(), Status::RolledBack);
    let writes = io.writes;
    executor.rollback(&mut io).await.unwrap();
    assert_eq!(io.writes, writes);
    assert!(executor.forward(&mut io).await.is_err());
}
#[tokio::test]
async fn drift_and_wrong_approval_reject_before_writes() {
    let (plan, mut io) = fixture();
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("journal.json");
    assert!(Executor::create(&path, plan.clone(), "wrong-plan").is_err());
    assert!(!path.exists());
    let mut executor = Executor::create(&path, plan.clone(), &plan.revision().unwrap()).unwrap();
    io.evidence.revision = "changed-session".into();
    assert!(executor.forward(&mut io).await.is_err());
    assert_eq!(io.writes, 0);
    io.evidence.revision = "settings+sessions-v1".into();
    io.evidence.rows.get_mut("core").unwrap()[1].comment = "foreign change".into();
    assert!(executor.forward(&mut io).await.is_err());
    assert_eq!(io.writes, 0);
}
#[test]
fn rejects_wrong_membership_scope_and_unsafe_rows() {
    let (_, io) = fixture();
    let evidence = io.evidence;
    for (address, id) in [("203.0.113.1/32", "*1"), ("192.0.2.3/32", "*2")] {
        assert!(prepare(evidence.clone(), address, id).is_err());
    }
    for kind in ["dynamic", "timeout", "owned", "disabled"] {
        let mut e = evidence.clone();
        let row = &mut e.rows.get_mut("core").unwrap()[0];
        match kind {
            "dynamic" => row.dynamic = true,
            "timeout" => row.timeout = Some("1m".into()),
            "owned" => row.comment = "lqosync3:someone".into(),
            _ => row.disabled = true,
        }
        assert!(prepare(e, "192.0.2.3/32", "*1").is_err());
    }
}
#[tokio::test]
async fn lost_reply_is_durable_and_never_retried_or_removed() {
    let (plan, mut io) = fixture();
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("journal.json");
    let mut executor = Executor::create(&path, plan.clone(), &plan.revision().unwrap()).unwrap();
    io.fail_at = Some(1);
    io.lose_reply = true;
    assert!(executor.forward(&mut io).await.is_err());
    drop(executor);
    assert_eq!(io.writes, 1);
    let mut executor = Executor::open(&path, &plan.revision().unwrap()).unwrap();
    assert!(executor.forward(&mut io).await.is_err());
    assert!(executor.rollback(&mut io).await.is_err());
    assert_eq!(io.writes, 1);
    assert!(std::fs::read_to_string(path).unwrap().contains("pending"));
}
#[tokio::test]
async fn reopened_rollback_refuses_foreign_edits() {
    let (plan, mut io) = fixture();
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("journal.json");
    let mut executor = Executor::create(&path, plan.clone(), &plan.revision().unwrap()).unwrap();
    executor.forward(&mut io).await.unwrap();
    drop(executor);
    io.evidence.rows.get_mut("core").unwrap()[0].comment = "edited after migration".into();
    let before = io.writes;
    let mut executor = Executor::open(&path, &plan.revision().unwrap()).unwrap();
    assert!(executor.rollback(&mut io).await.is_err());
    assert_eq!(io.writes, before);
}

#[test]
fn rejects_unselected_overlapping_manual_rows() {
    let (_, mut io) = fixture();
    io.evidence
        .rows
        .get_mut("core")
        .unwrap()
        .push(member("*3", "wan-a", "192.0.2.0/28"));
    assert!(
        prepare(io.evidence, "192.0.2.3/32", "*1").is_err(),
        "a remaining manual overlap would still block publication"
    );
}
#[tokio::test]
async fn row_ids_are_local_to_each_router() {
    let (_, mut io) = fixture();
    io.evidence.rows.insert(
        "another-core".into(),
        vec![member("*10", "outside", "203.0.113.0/24")],
    );
    let plan = prepare(io.evidence.clone(), "192.0.2.3/32", "*1").unwrap();
    let temp = tempfile::tempdir().unwrap();
    let mut executor = Executor::create(
        &temp.path().join("journal.json"),
        plan.clone(),
        &plan.revision().unwrap(),
    )
    .unwrap();
    executor.forward(&mut io).await.unwrap();
    executor.rollback(&mut io).await.unwrap();
}
#[test]
fn rejects_inconsistent_completed_journal() {
    let (plan, _) = fixture();
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("journal.json");
    drop(Executor::create(&path, plan.clone(), &plan.revision().unwrap()).unwrap());
    let mut value: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
    value["status"] = "forward_complete".into();
    std::fs::write(&path, serde_json::to_vec(&value).unwrap()).unwrap();
    assert!(Executor::open(&path, &plan.revision().unwrap()).is_err());
}

#[tokio::test]
async fn lost_narrowing_and_rollback_replies_remain_pending_after_reopen() {
    for failing_write in [3, 5] {
        let (plan, mut io) = fixture();
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("journal.json");
        let mut e = Executor::create(&path, plan.clone(), &plan.revision().unwrap()).unwrap();
        io.fail_at = Some(failing_write);
        io.lose_reply = true;
        if failing_write == 3 {
            assert!(e.forward(&mut io).await.is_err());
        } else {
            e.forward(&mut io).await.unwrap();
            assert!(e.rollback(&mut io).await.is_err());
        }
        drop(e);
        let writes = io.writes;
        let mut e = Executor::open(&path, &plan.revision().unwrap()).unwrap();
        assert!(e.forward(&mut io).await.is_err());
        assert!(e.rollback(&mut io).await.is_err());
        assert_eq!(io.writes, writes);
    }
}
#[tokio::test]
async fn journal_failure_blocks_writes_and_poisoned_instance_cannot_continue() {
    let (plan, mut io) = fixture();
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("journal.json");
    let saved = temp.path().join("journal.saved");
    let mut e = Executor::create(&path, plan.clone(), &plan.revision().unwrap()).unwrap();
    std::fs::rename(&path, &saved).unwrap();
    std::fs::create_dir(&path).unwrap();
    assert!(e.forward(&mut io).await.is_err());
    assert_eq!(io.writes, 0);
    std::fs::remove_dir(&path).unwrap();
    std::fs::rename(&saved, &path).unwrap();
    assert!(e.forward(&mut io).await.is_err());
    assert_eq!(io.writes, 0);
    drop(e);
    let mut e = Executor::open(&path, &plan.revision().unwrap()).unwrap();
    e.forward(&mut io).await.unwrap();
    e.rollback(&mut io).await.unwrap();
}
#[tokio::test]
async fn expired_approval_cannot_start_writes() {
    let (plan, mut io) = fixture();
    let mut value = serde_json::to_value(&plan).unwrap();
    value["baseline"]["captured_at"] = (now() - 2).into();
    value["expires_at"] = (now() - 1).into();
    let plan: Plan = serde_json::from_value(value).unwrap();
    let temp = tempfile::tempdir().unwrap();
    let mut e = Executor::create(
        &temp.path().join("journal.json"),
        plan.clone(),
        &plan.revision().unwrap(),
    )
    .unwrap();
    assert!(e.forward(&mut io).await.is_err());
    assert_eq!(io.writes, 0);
}
#[tokio::test]
async fn returned_id_collision_does_not_unlock_further_writes() {
    let (plan, mut io) = fixture();
    io.next_id = 1;
    let temp = tempfile::tempdir().unwrap();
    let mut e = Executor::create(
        &temp.path().join("journal.json"),
        plan.clone(),
        &plan.revision().unwrap(),
    )
    .unwrap();
    assert!(e.forward(&mut io).await.is_err());
    assert!(e.rollback(&mut io).await.is_err());
    assert_eq!(io.writes, 1);
}
#[test]
fn private_journal_and_lock_are_exclusive_and_wrong_approval_cannot_reopen() {
    use std::os::unix::fs::PermissionsExt;
    let (plan, _) = fixture();
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("journal.json");
    let e = Executor::create(&path, plan.clone(), &plan.revision().unwrap()).unwrap();
    assert_eq!(
        std::fs::metadata(&path).unwrap().permissions().mode() & 0o777,
        0o600
    );
    assert!(Executor::open(&path, &plan.revision().unwrap()).is_err());
    drop(e);
    assert!(Executor::open(&path, "different approval").is_err());
    assert!(Executor::open(&path, &plan.revision().unwrap()).is_ok());
}
#[test]
fn rejects_stale_future_and_whole_row_removal() {
    let (_, io) = fixture();
    for at in [now() - 31, now() + 60] {
        let mut evidence = io.evidence.clone();
        evidence.captured_at = at;
        assert!(prepare(evidence, "192.0.2.3/32", "*1").is_err());
    }
    let mut evidence = io.evidence;
    evidence.rows.get_mut("core").unwrap()[0].address = "192.0.2.3/32".into();
    assert!(prepare(evidence, "192.0.2.3/32", "*1").is_err());
}
#[test]
fn prefix_boundaries_do_not_overflow() {
    let (_, io) = fixture();
    for host in ["0.0.0.0/32", "255.255.255.255/32"] {
        let mut evidence = io.evidence.clone();
        evidence.rows.get_mut("core").unwrap()[0].address = "0.0.0.0/0".into();
        assert!(prepare(evidence, host, "*1").is_ok());
    }
}

#[tokio::test]
async fn known_partial_execution_reopens_and_rolls_back_without_repeating_adds() {
    let (plan, mut io) = fixture();
    let original = io.evidence.rows.clone();
    io.fail_read_at = Some(2);
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("journal.json");
    let mut e = Executor::create(&path, plan.clone(), &plan.revision().unwrap()).unwrap();
    assert!(e.forward(&mut io).await.is_err());
    assert_eq!(io.writes, 1);
    drop(e);
    io.fail_read_at = None;
    let mut e = Executor::open(&path, &plan.revision().unwrap()).unwrap();
    e.rollback(&mut io).await.unwrap();
    assert_eq!(io.writes, 2);
    assert_eq!(io.evidence.rows, original);
}
#[tokio::test]
async fn lost_durable_result_after_successful_add_blocks_retry_on_reopen() {
    let (plan, mut io) = fixture();
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("journal.json");
    let mut e = Executor::create(&path, plan.clone(), &plan.revision().unwrap()).unwrap();
    io.journal_fault = Some((1, path.clone()));
    assert!(e.forward(&mut io).await.is_err());
    assert!(e.rollback(&mut io).await.is_err());
    assert_eq!(io.writes, 1);
    drop(e);
    std::fs::remove_dir(&path).unwrap();
    std::fs::rename(path.with_extension("saved"), &path).unwrap();
    let mut e = Executor::open(&path, &plan.revision().unwrap()).unwrap();
    assert!(e.forward(&mut io).await.is_err());
    assert!(e.rollback(&mut io).await.is_err());
    assert_eq!(io.writes, 1);
}
#[tokio::test]
async fn transport_must_return_fresh_evidence_before_each_write() {
    let (plan, mut io) = fixture();
    io.refresh_time = false;
    io.evidence.captured_at = now() - 31;
    let temp = tempfile::tempdir().unwrap();
    let mut e = Executor::create(
        &temp.path().join("journal.json"),
        plan.clone(),
        &plan.revision().unwrap(),
    )
    .unwrap();
    assert!(e.forward(&mut io).await.is_err());
    assert_eq!(io.writes, 0);
}

#[tokio::test]
async fn approval_that_expires_during_observation_cannot_authorize_a_write() {
    let (plan, mut io) = fixture();
    let mut value = serde_json::to_value(&plan).unwrap();
    value["expires_at"] = (now() + 1).into();
    let plan: Plan = serde_json::from_value(value).unwrap();
    io.read_delay_ms = 2100;
    let temp = tempfile::tempdir().unwrap();
    let mut e = Executor::create(
        &temp.path().join("journal.json"),
        plan.clone(),
        &plan.revision().unwrap(),
    )
    .unwrap();
    assert!(e.forward(&mut io).await.is_err());
    assert_eq!(io.writes, 0);
}
#[test]
fn marker_inputs_match_core_planner_identity_rules() {
    let (plan, _) = fixture();
    for (key, invalid) in [
        ("subscriber", "00000000000040008000000000000002"),
        ("wan", "contains:colon"),
        ("router", "contains space"),
    ] {
        let mut value = serde_json::to_value(&plan).unwrap();
        if key == "subscriber" {
            value["hosts"][0][key] = invalid.into();
        } else if key == "wan" {
            value["hosts"][0][key] = invalid.into();
            value["bindings"][0][key] = invalid.into();
        } else {
            let rows = value["baseline"]["rows"]["core"].take();
            value["baseline"]["rows"]
                .as_object_mut()
                .unwrap()
                .remove("core");
            value["baseline"]["rows"][invalid] = rows;
            value["hosts"][0][key] = invalid.into();
            value["bindings"][0][key] = invalid.into();
            value["protected"][0][key] = invalid.into();
        }
        let plan: Plan = serde_json::from_value(value).unwrap();
        let temp = tempfile::tempdir().unwrap();
        assert!(
            Executor::create(
                &temp.path().join("journal.json"),
                plan.clone(),
                &plan.revision().unwrap()
            )
            .is_err(),
            "invalid {key} accepted"
        );
    }
}

#[tokio::test]
async fn two_wan_migration_preserves_membership_and_rejects_ambiguous_or_moved_clients() {
    let (_, mut io) = fixture();
    io.evidence
        .rows
        .get_mut("core")
        .unwrap()
        .push(member("*3", "wan-b", "203.0.113.0/29"));
    let bindings = vec![
        Binding {
            wan: "a".into(),
            router: "core".into(),
            list: "wan-a".into(),
            table: "table-a".into(),
        },
        Binding {
            wan: "b".into(),
            router: "core".into(),
            list: "wan-b".into(),
            table: "table-b".into(),
        },
    ];
    let hosts = vec![
        Host {
            router: "core".into(),
            list: "wan-a".into(),
            wan: "a".into(),
            subscriber: "00000000-0000-4000-8000-000000000002".into(),
            address: "192.0.2.3/32".into(),
        },
        Host {
            router: "core".into(),
            list: "wan-b".into(),
            wan: "b".into(),
            subscriber: "00000000-0000-4000-8000-000000000003".into(),
            address: "203.0.113.3/32".into(),
        },
    ];
    let selected = vec![
        RowKey {
            router: "core".into(),
            id: "*1".into(),
        },
        RowKey {
            router: "core".into(),
            id: "*3".into(),
        },
    ];
    let owner = "00000000-0000-4000-8000-000000000001".to_string();
    let plan = Plan::prepare(
        io.evidence.clone(),
        bindings.clone(),
        owner.clone(),
        hosts.clone(),
        selected.clone(),
        120,
    )
    .unwrap();
    let mut moved = hosts.clone();
    moved[0].wan = "b".into();
    moved[0].list = "wan-b".into();
    assert!(
        Plan::prepare(
            io.evidence.clone(),
            bindings.clone(),
            owner.clone(),
            moved,
            selected.clone(),
            120
        )
        .is_err()
    );
    let mut ambiguous = io.evidence.clone();
    ambiguous
        .rows
        .get_mut("core")
        .unwrap()
        .push(member("*4", "wan-b", "192.0.2.0/29"));
    assert!(Plan::prepare(ambiguous, bindings, owner, hosts, selected, 120).is_err());
    let original = io.evidence.rows.clone();
    let temp = tempfile::tempdir().unwrap();
    let mut e = Executor::create(
        &temp.path().join("journal.json"),
        plan.clone(),
        &plan.revision().unwrap(),
    )
    .unwrap();
    e.forward(&mut io).await.unwrap();
    assert_eq!(io.writes, 6);
    e.rollback(&mut io).await.unwrap();
    assert_eq!(io.evidence.rows, original);
}
