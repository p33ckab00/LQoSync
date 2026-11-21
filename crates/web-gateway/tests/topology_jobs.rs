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
        if !matches!(op.state.as_str(), "queued" | "running") {
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
