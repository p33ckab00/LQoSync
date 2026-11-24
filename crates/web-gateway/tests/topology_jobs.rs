use serde_json::{Value, json};
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt};
use web_contract::topology::WebCommand;
use web_gateway::topology_jobs::{Jobs, Refused};

const KEY: &str = "0f8b5a3c-1d2e-4f60-9a7b-000000000001";

fn create(name: &str) -> WebCommand {
    WebCommand::PlanCreate {
        expected_revision: 2,
        name: name.into(),
        down_mbps: 100,
        up_mbps: 50,
    }
}

fn receipt(state: &str) -> Value {
    json!({"key": KEY, "state": state, "base_revision": "2", "revision": "3",
           "subject_id": "plan-1", "error": null})
}

#[derive(Clone, Copy)]
enum Mode {
    /// Answer each mutation with a saved receipt.
    Save,
    /// Read the mutation, then close without an answer.
    Drop,
    /// Answer with another request ID.
    WrongId,
    /// Answer each mutation with a failed receipt, as the service does for a refused save.
    Fail,
}

/// A fake service API socket. It counts `topology.web.mutate` calls. `topology.web.operation`
/// answers with a saved receipt for `alice` and `null` for anyone else.
fn core(dir: &std::path::Path, mode: Mode) -> Arc<AtomicUsize> {
    let calls = Arc::new(AtomicUsize::new(0));
    let listener = tokio::net::UnixListener::bind(dir.join("api.sock")).unwrap();
    let counter = calls.clone();
    tokio::spawn(async move {
        while let Ok((stream, _)) = listener.accept().await {
            let counter = counter.clone();
            tokio::spawn(async move {
                let (read, mut write) = stream.into_split();
                let mut line = String::new();
                tokio::io::BufReader::new(read)
                    .read_line(&mut line)
                    .await
                    .unwrap();
                let v: Value = serde_json::from_str(&line).unwrap();
                let result = match v["op"].as_str().unwrap() {
                    "topology.web.mutate" => {
                        counter.fetch_add(1, Ordering::SeqCst);
                        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
                        match mode {
                            Mode::Drop => return,
                            Mode::Fail => {
                                json!({"key": KEY, "state": "failed", "base_revision": "2",
                                "revision": null, "subject_id": null,
                                "error": "stale_config_digest: the loaded configuration changed"})
                            }
                            _ => receipt("saved"),
                        }
                    }
                    "topology.web.operation" if v["args"]["actor"] == "alice" => receipt("saved"),
                    _ => Value::Null,
                };
                let id = match mode {
                    Mode::WrongId => json!("other"),
                    _ => v["id"].clone(),
                };
                let answer = json!({"v": 1, "id": id, "ok": true, "result": result});
                let mut bytes = serde_json::to_vec(&answer).unwrap();
                bytes.push(b'\n');
                let _ = write.write_all(&bytes).await;
            });
        }
    });
    calls
}

async fn settle(jobs: &Jobs, actor: &str) -> web_contract::topology::Operation {
    for _ in 0..100 {
        let op = jobs.status(actor, KEY).await;
        if !matches!(op.state.as_str(), "queued" | "running" | "reviewing") {
            return op;
        }
        tokio::time::sleep(std::time::Duration::from_millis(20)).await;
    }
    panic!("the operation did not finish");
}

#[tokio::test]
async fn a_repeated_submit_runs_the_command_once() {
    let dir = tempfile::tempdir().unwrap();
    let calls = core(dir.path(), Mode::Save);
    let jobs = Jobs::new(dir.path().into());
    let first = jobs.submit("alice", KEY, create("Gold")).await.unwrap();
    assert!(matches!(first.state.as_str(), "queued" | "running"));
    // A double click and a retry after a reload use the same key.
    jobs.submit("alice", KEY, create("Gold")).await.unwrap();
    let done = settle(&jobs, "alice").await;
    jobs.submit("alice", KEY, create("Gold")).await.unwrap();
    assert_eq!(done.state, "saved");
    assert_eq!(done.receipt.unwrap().revision.as_deref(), Some("3"));
    assert_eq!(calls.load(Ordering::SeqCst), 1);
}

#[tokio::test]
async fn another_payload_actor_or_command_is_refused() {
    let dir = tempfile::tempdir().unwrap();
    core(dir.path(), Mode::Save);
    let jobs = Jobs::new(dir.path().into());
    jobs.submit("alice", KEY, create("Gold")).await.unwrap();
    assert_eq!(
        jobs.submit("alice", KEY, create("Silver"))
            .await
            .unwrap_err(),
        Refused::KeyReused
    );
    assert_eq!(
        jobs.submit("bob", KEY, create("Gold")).await.unwrap_err(),
        Refused::KeyReused
    );
    // The core proposal token never comes from the browser.
    let confirm = WebCommand::EditConfirm {
        proposal_id: "edit-2-1".into(),
    };
    let other = "0f8b5a3c-1d2e-4f60-9a7b-000000000002";
    assert_eq!(
        jobs.submit("alice", other, confirm).await.unwrap_err(),
        Refused::Invalid
    );
    assert_eq!(
        jobs.submit("alice", "short", create("Gold"))
            .await
            .unwrap_err(),
        Refused::Invalid
    );
    // Another actor sees no operation of alice.
    assert_eq!(jobs.status("bob", KEY).await.state, "not_found");
}

#[tokio::test]
async fn a_lost_reply_is_unknown_until_the_service_receipt_resolves_it() {
    for mode in [Mode::Drop, Mode::WrongId] {
        let dir = tempfile::tempdir().unwrap();
        let calls = core(dir.path(), mode);
        let jobs = Jobs::new(dir.path().into());
        jobs.submit("alice", KEY, create("Gold")).await.unwrap();
        // The status asks the service receipt store; it never sends the mutation again.
        let done = settle(&jobs, "alice").await;
        match mode {
            // The fake answers lookups only with a correct ID.
            Mode::Drop => assert_eq!(done.state, "saved"),
            _ => assert_eq!(done.state, "outcome_unknown"),
        }
        assert_eq!(calls.load(Ordering::SeqCst), 1);
    }
}

#[tokio::test]
async fn no_service_socket_fails_without_a_write() {
    let dir = tempfile::tempdir().unwrap();
    let jobs = Jobs::new(dir.path().into());
    jobs.submit("alice", KEY, create("Gold")).await.unwrap();
    let done = settle(&jobs, "alice").await;
    assert_eq!(done.state, "failed");
    assert_eq!(done.receipt, None);
}

#[tokio::test]
async fn the_queue_has_a_limit() {
    let dir = tempfile::tempdir().unwrap();
    // A socket that never answers keeps every job open.
    let _listener = tokio::net::UnixListener::bind(dir.path().join("api.sock")).unwrap();
    let jobs = Jobs::new(dir.path().into());
    let mut refused = None;
    for i in 0..(web_gateway::topology_jobs::MAX_OPEN + 1) {
        let key = format!("0f8b5a3c-1d2e-4f60-9a7b-{i:012}");
        if let Err(e) = jobs.submit("alice", &key, create("Gold")).await {
            refused = Some((i, e));
            break;
        }
    }
    assert_eq!(
        refused,
        Some((web_gateway::topology_jobs::MAX_OPEN, Refused::Busy))
    );
}

/// A fake service for Builder proposals. It records every request. `node.remove` of `busy` is
/// refused with a rule; any other edit gets a proposal that moves two subscribers.
fn builder_core(dir: &std::path::Path, ttl: u64) -> Arc<tokio::sync::Mutex<Vec<Value>>> {
    let seen = Arc::new(tokio::sync::Mutex::new(Vec::new()));
    let listener = tokio::net::UnixListener::bind(dir.join("api.sock")).unwrap();
    let log = seen.clone();
    tokio::spawn(async move {
        while let Ok((stream, _)) = listener.accept().await {
            let log = log.clone();
            tokio::spawn(async move {
                let (read, mut write) = stream.into_split();
                let mut line = String::new();
                tokio::io::BufReader::new(read)
                    .read_line(&mut line)
                    .await
                    .unwrap();
                let v: Value = serde_json::from_str(&line).unwrap();
                log.lock().await.push(v.clone());
                let now = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_secs();
                let answer = match v["op"].as_str().unwrap() {
                    "topology.edit.propose" if v["args"]["edit"]["node_id"] == "busy" => json!({
                        "v": 1, "id": v["id"], "ok": false,
                        "error": {"code": "invalid", "message": "a node with children cannot be removed"}}),
                    "topology.edit.propose" => {
                        json!({"v": 1, "id": v["id"], "ok": true, "result": {
                        "proposal_id": "edit-2-7", "expires_at": now + ttl, "edit": v["args"]["edit"],
                        "nodes": [{"id": "new-1", "name": "Tower 1", "parent": null, "kind": "intermediate",
                                   "source_id": null, "capacity": {"mode": "automatic"}, "alias": false}],
                        "moves": [{"subscriber_id": "s1", "label": "alice", "from": "a", "to": "b"},
                                  {"subscriber_id": "s2", "label": null, "from": "a", "to": "b"}],
                        "cleared": ["s3"], "assigned": 0, "new_records": 0, "sources": ["ppp-a"],
                        "rates_changed": 1, "rates": [{"subscriber_id": "s1", "label": "alice",
                            "before": {"min_mbps": [50.0, 25.0], "max_mbps": [100.0, 50.0]},
                            "after": {"min_mbps": [5.0, 2.5], "max_mbps": [10.0, 5.0]}}],
                        "capacities": [{"node_id": "g1", "before_mbps": [400.0, 200.0], "after_mbps": [40.0, 20.0]}]}})
                    }
                    "wan.discover" => json!({"v": 1, "id": v["id"], "ok": true, "result": {
                        "collected_at": now, "unconfigured": [], "routers": [
                        {"router_id": "core", "name": "Core", "state": "ok", "error": null, "missing": [],
                         "candidates": [{"name": "pppoe-out7", "type": "pppoe-out", "running": true,
                            "disabled": false, "comment": null, "suggested": true, "source_id": null, "used_by": []}]},
                        {"router_id": "edge", "name": "Edge", "state": "failed", "error": "timeout",
                         "candidates": [], "missing": []}]}}),
                    "topology.sources.candidates" => {
                        json!({"v": 1, "id": v["id"], "ok": true, "result": {
                        "candidates": [{"key": "access-ppp", "kind": "ppp", "router": "edge",
                            "name": "PPPoE Edge", "subscribers": null, "rebind": false}]}})
                    }
                    "topology.preview.refresh" => json!({"v": 1, "id": v["id"], "ok": true,
                        "result": {"state": "ok"}}),
                    "topology.web.mutate" => json!({"v": 1, "id": v["id"], "ok": true,
                        "result": {"key": v["args"]["key"], "state": "saved", "base_revision": "2",
                                   "revision": "3", "subject_id": null, "error": null}}),
                    _ => json!({"v": 1, "id": v["id"], "ok": true, "result": {}}),
                };
                let mut bytes = serde_json::to_vec(&answer).unwrap();
                bytes.push(b'\n');
                let _ = write.write_all(&bytes).await;
            });
        }
    });
    seen
}

fn proposal(node: &str) -> web_contract::topology::ProposalRequest {
    web_contract::topology::ProposalRequest {
        key: KEY.into(),
        expected_revision: 2,
        edit: web_contract::topology::BuilderEdit::NodeRemove {
            node_id: node.into(),
        },
    }
}

const SAVE: &str = "0f8b5a3c-1d2e-4f60-9a7b-00000000aaaa";

#[tokio::test]
async fn a_reviewed_proposal_saves_once_through_its_private_token() {
    let dir = tempfile::tempdir().unwrap();
    let seen = builder_core(dir.path(), 120);
    let jobs = Jobs::new(dir.path().into());
    jobs.propose("alice", proposal("old")).await.unwrap();
    let review = settle(&jobs, "alice").await;
    assert_eq!(review.state, "review_ready");
    let r = review.review.unwrap();
    assert_eq!((r.proposal.as_str(), r.moves, r.cleared), (KEY, 2, 1));
    assert_eq!(r.moved[0].label.as_deref(), Some("alice"));
    assert_eq!(r.rates_changed, 1);
    assert_eq!(r.rates[0].before, "min 50 / 25 Mbps, max 100 / 50 Mbps");
    assert_eq!(r.rates[0].after, "min 5 / 2.5 Mbps, max 10 / 5 Mbps");
    assert_eq!(
        (
            r.capacities[0].node_id.as_str(),
            r.capacities[0].after.as_str()
        ),
        ("g1", "40 / 20 Mbps")
    );
    assert!(
        !serde_json::to_string(&r).unwrap().contains("edit-2-7"),
        "no service token"
    );
    // No cached view of revision 2: no node diff, and the review says why.
    assert!(r.nodes.is_empty());
    assert_eq!(r.warnings.len(), 1);
    // Another actor cannot confirm or cancel this proposal.
    assert_eq!(
        jobs.confirm("bob", KEY, SAVE).await.unwrap_err(),
        Refused::Expired
    );
    let saved = jobs.confirm("alice", KEY, SAVE).await.unwrap();
    assert!(matches!(saved.state.as_str(), "queued" | "running"));
    // A double click on Save uses the same key: no second commit.
    jobs.confirm("alice", KEY, SAVE).await.unwrap();
    let mut done = jobs.status("alice", SAVE).await;
    for _ in 0..100 {
        if done.state == "saved" {
            break;
        }
        tokio::time::sleep(std::time::Duration::from_millis(20)).await;
        done = jobs.status("alice", SAVE).await;
    }
    assert_eq!(done.state, "saved");
    let log = seen.lock().await;
    let confirms: Vec<&Value> = log
        .iter()
        .filter(|v| v["op"] == "topology.web.mutate")
        .collect();
    assert_eq!(confirms.len(), 1);
    assert_eq!(confirms[0]["args"]["command"]["proposal_id"], "edit-2-7");
    assert_eq!(confirms[0]["args"]["actor"], "alice");
}

#[tokio::test]
async fn an_expired_or_cancelled_proposal_cannot_save() {
    let dir = tempfile::tempdir().unwrap();
    let seen = builder_core(dir.path(), 0);
    let jobs = Jobs::new(dir.path().into());
    jobs.propose("alice", proposal("old")).await.unwrap();
    assert_eq!(settle(&jobs, "alice").await.state, "review_ready");
    tokio::time::sleep(std::time::Duration::from_millis(1100)).await;
    assert_eq!(
        jobs.confirm("alice", KEY, SAVE).await.unwrap_err(),
        Refused::Expired
    );

    let dir = tempfile::tempdir().unwrap();
    let seen2 = builder_core(dir.path(), 120);
    let jobs = Jobs::new(dir.path().into());
    jobs.propose("alice", proposal("old")).await.unwrap();
    settle(&jobs, "alice").await;
    assert!(!jobs.cancel("bob", KEY).await);
    assert!(jobs.cancel("alice", KEY).await);
    assert_eq!(
        jobs.confirm("alice", KEY, SAVE).await.unwrap_err(),
        Refused::Expired
    );
    tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    assert!(
        seen2
            .lock()
            .await
            .iter()
            .any(|v| v["op"] == "topology.edit.cancel" && v["args"]["proposal_id"] == "edit-2-7")
    );
    assert!(
        !seen
            .lock()
            .await
            .iter()
            .any(|v| v["op"] == "topology.web.mutate")
    );
}

#[tokio::test]
async fn a_refused_proposal_shows_the_service_rule() {
    let dir = tempfile::tempdir().unwrap();
    builder_core(dir.path(), 120);
    let jobs = Jobs::new(dir.path().into());
    jobs.propose("alice", proposal("busy")).await.unwrap();
    let done = settle(&jobs, "alice").await;
    assert_eq!(done.state, "failed");
    assert_eq!(
        done.error.as_deref(),
        Some("a node with children cannot be removed")
    );
}

#[tokio::test]
async fn explicit_reads_return_their_typed_answers() {
    use web_contract::topology::{ReadJob, ReadRequest};
    let dir = tempfile::tempdir().unwrap();
    let seen = builder_core(dir.path(), 120);
    let jobs = Jobs::new(dir.path().into());
    for (i, job) in [ReadJob::Wan, ReadJob::Sources, ReadJob::Refresh]
        .into_iter()
        .enumerate()
    {
        let key = format!("0f8b5a3c-1d2e-4f60-9a7b-00000000r00{i}");
        let request = ReadRequest {
            key: key.clone(),
            job,
            routers: Vec::new(),
        };
        jobs.read("alice", request.clone()).await.unwrap();
        // A double click returns the same job.
        jobs.read("alice", request).await.unwrap();
        let mut done = jobs.status("alice", &key).await;
        for _ in 0..100 {
            if done.state != "queued" && done.state != "running" {
                break;
            }
            tokio::time::sleep(std::time::Duration::from_millis(20)).await;
            done = jobs.status("alice", &key).await;
        }
        assert_eq!(done.state, "done", "{job:?}");
        let d = done.discovery.unwrap_or_default();
        match job {
            ReadJob::Wan => {
                assert_eq!(d.routers.len(), 2);
                assert_eq!(d.routers[0].candidates[0].name, "pppoe-out7");
                assert_eq!(d.routers[1].error.as_deref(), Some("timeout"));
            }
            ReadJob::Sources => assert_eq!(d.sources[0].key, "access-ppp"),
            ReadJob::Refresh => assert!(d.routers.is_empty() && d.sources.is_empty()),
        }
    }
    let ops: Vec<String> = seen
        .lock()
        .await
        .iter()
        .map(|v| v["op"].as_str().unwrap().to_owned())
        .collect();
    assert_eq!(
        ops,
        [
            "wan.discover",
            "topology.sources.candidates",
            "topology.preview.refresh"
        ]
    );
}

/// A fake service that saves layout revision 1 from base 0 and refuses any other base.
fn layout_core(dir: &std::path::Path) -> Arc<AtomicUsize> {
    let calls = Arc::new(AtomicUsize::new(0));
    let listener = tokio::net::UnixListener::bind(dir.join("api.sock")).unwrap();
    let counter = calls.clone();
    tokio::spawn(async move {
        while let Ok((stream, _)) = listener.accept().await {
            counter.fetch_add(1, Ordering::SeqCst);
            tokio::spawn(async move {
                let (read, mut write) = stream.into_split();
                let mut line = String::new();
                tokio::io::BufReader::new(read)
                    .read_line(&mut line)
                    .await
                    .unwrap();
                let v: Value = serde_json::from_str(&line).unwrap();
                assert_eq!(v["op"], "topology.layout.save");
                let answer = if v["args"]["expected_revision"] == 0 {
                    json!({"v": 1, "id": v["id"], "ok": true, "result": {"revision": 1}})
                } else {
                    json!({"v": 1, "id": v["id"], "ok": false,
                           "error": {"code": "conflict", "message": "revision conflict"}})
                };
                let mut bytes = serde_json::to_vec(&answer).unwrap();
                bytes.push(b'\n');
                let _ = write.write_all(&bytes).await;
            });
        }
    });
    calls
}

#[tokio::test]
async fn a_layout_save_returns_the_revision_or_a_conflict() {
    use web_contract::topology::LayoutRequest;
    let dir = tempfile::tempdir().unwrap();
    let calls = layout_core(dir.path());
    let jobs = Jobs::new(dir.path().into());
    let request = |rev| LayoutRequest {
        direction: Default::default(),
        expected_revision: rev,
        positions: [("n1".to_string(), [1.0, 2.0])].into(),
    };
    assert_eq!(jobs.layout(request(0)).await, Ok(1));
    assert_eq!(jobs.layout(request(5)).await, Err(Refused::Conflict));
    let mut far = request(0);
    far.positions.insert("n2".into(), [f64::INFINITY, 0.0]);
    assert_eq!(jobs.layout(far).await, Err(Refused::Invalid));
    assert_eq!(
        calls.load(Ordering::SeqCst),
        2,
        "an invalid layout never reaches the service"
    );
}

#[tokio::test]
async fn an_automatic_edit_is_a_submitted_command() {
    let dir = tempfile::tempdir().unwrap();
    let calls = core(dir.path(), Mode::Save);
    let jobs = Jobs::new(dir.path().into());
    let edit = WebCommand::Edit {
        expected_revision: 2,
        edit: web_contract::topology::BuilderEdit::NodeRemove {
            node_id: "n1".into(),
        },
    };
    jobs.submit("alice", KEY, edit).await.unwrap();
    assert_eq!(settle(&jobs, "alice").await.state, "saved");
    assert_eq!(calls.load(Ordering::SeqCst), 1);
}

#[tokio::test]
async fn a_read_with_too_many_or_empty_router_ids_is_refused() {
    use web_contract::topology::{ReadJob, ReadRequest};
    let dir = tempfile::tempdir().unwrap();
    let jobs = Jobs::new(dir.path().into());
    for routers in [vec!["r".to_string(); 65], vec![String::new()]] {
        let request = ReadRequest {
            key: KEY.into(),
            job: ReadJob::Wan,
            routers,
        };
        assert_eq!(
            jobs.read("alice", request).await.unwrap_err(),
            Refused::Invalid
        );
    }
}

fn sources_set() -> WebCommand {
    serde_json::from_value(json!({"command": "edit", "expected_revision": 2, "edit": {
        "kind": "router.sources_set", "router_id": "lq3-00000000000000a1",
        "expected_config_digest": "d1", "policies": []}}))
    .unwrap()
}

#[tokio::test]
async fn lost_policy_reply_recovers_receipt() {
    let dir = tempfile::tempdir().unwrap();
    let calls = core(dir.path(), Mode::Drop);
    let jobs = Jobs::new(dir.path().into());
    jobs.submit("alice", KEY, sources_set()).await.unwrap();
    // The reply is lost after the service saved: the receipt decides, with no second save.
    let done = settle(&jobs, "alice").await;
    assert_eq!(done.state, "saved");
    assert_eq!(done.receipt.unwrap().revision.as_deref(), Some("3"));
    jobs.submit("alice", KEY, sources_set()).await.unwrap();
    assert_eq!(calls.load(Ordering::SeqCst), 1);
    // Another actor cannot read that receipt.
    assert_eq!(jobs.status("bob", KEY).await.state, "not_found");
}

#[tokio::test]
async fn apply_failure_never_reports_applied() {
    let dir = tempfile::tempdir().unwrap();
    core(dir.path(), Mode::Fail);
    let jobs = Jobs::new(dir.path().into());
    jobs.submit("alice", KEY, sources_set()).await.unwrap();
    let done = settle(&jobs, "alice").await;
    assert_eq!(done.state, "failed");
    let receipt = done.receipt.unwrap();
    assert_eq!((receipt.state.as_str(), receipt.revision), ("failed", None));
    assert!(receipt.error.unwrap().starts_with("stale_config_digest"));
    // An operation state is never "applied": only runtime evidence shows an applied output.
    for state in [&done.state, &receipt.state] {
        assert!(!["saved", "applied"].contains(&state.as_str()));
    }
}
