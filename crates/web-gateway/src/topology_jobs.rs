//! Web mutations: one broker lane to the service, idempotent by request key.
//!
//! The service receipt store is the durable record. This broker map only tracks requests in
//! flight. A reply that is lost after the request left is never sent again: the status asks the
//! service for the receipt of that key instead.
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt};
use tokio::sync::{Mutex, Semaphore};
use web_contract::topology::{Operation, WebCommand, WebReceipt};

/// Queued and running mutations at one time, for all actors.
pub const MAX_OPEN: usize = 16;
/// The service answers between cycles, so a mutation can wait for a whole cycle.
const MUTATE_LIMIT: Duration = Duration::from_secs(600);
const LOOKUP_LIMIT: Duration = Duration::from_secs(10);
/// Finished operations stay in memory this long; the service keeps the receipt longer.
const KEEP: Duration = Duration::from_secs(3600);

#[derive(Debug, PartialEq, Eq)]
pub enum Refused {
    /// Not a valid key, or a command that the browser cannot send.
    Invalid,
    /// The key belongs to another actor or command.
    KeyReused,
    /// Too many operations are open.
    Busy,
}

#[derive(Clone)]
enum State {
    Queued,
    Running,
    Done(Operation),
    /// The request left, and no valid answer came back.
    Lost,
}

struct Job {
    actor: String,
    command: WebCommand,
    state: State,
    at: Instant,
}

#[derive(Clone)]
pub struct Jobs {
    root: PathBuf,
    jobs: Arc<Mutex<HashMap<String, Job>>>,
    lane: Arc<Semaphore>,
}

enum CallError {
    /// No connection: the service did not get the request.
    NotSent,
    /// The request may have arrived; no valid answer came back.
    Lost,
    /// The service answered with an error: it refused the request before any change.
    Refused,
}

/// Same rule as the service: 16 to 64 letters, digits, or `-`.
fn valid_key(key: &str) -> bool {
    (16..=64).contains(&key.len()) && key.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-')
}

fn op(key: &str, state: &str, receipt: Option<WebReceipt>) -> Operation {
    Operation {
        key: key.into(),
        state: state.into(),
        receipt,
    }
}

/// One request on the service API socket. The answer must carry protocol 1, the same request
/// ID, and a complete success or error shape.
async fn call(
    root: &std::path::Path,
    operation: &str,
    args: serde_json::Value,
    limit: Duration,
) -> Result<serde_json::Value, CallError> {
    static N: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let id = format!(
        "web-{}-{}",
        std::process::id(),
        N.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
    );
    let stream = tokio::net::UnixStream::connect(root.join("api.sock"))
        .await
        .map_err(|_| CallError::NotSent)?;
    let exchange = async {
        let (read, mut write) = stream.into_split();
        let mut line = serde_json::to_vec(
            &serde_json::json!({"v": 1, "id": id, "op": operation, "args": args}),
        )
        .map_err(|_| CallError::NotSent)?;
        line.push(b'\n');
        write.write_all(&line).await.map_err(|_| CallError::Lost)?;
        let mut answer = String::new();
        tokio::io::BufReader::new(read)
            .take(64 * 1024)
            .read_line(&mut answer)
            .await
            .map_err(|_| CallError::Lost)?;
        let v: serde_json::Value = serde_json::from_str(&answer).map_err(|_| CallError::Lost)?;
        if v["v"] != 1 || v["id"] != id.as_str() {
            return Err(CallError::Lost);
        }
        match (&v["ok"], v["error"]["code"].as_str()) {
            (serde_json::Value::Bool(true), _) if v.get("result").is_some() => {
                Ok(v["result"].clone())
            }
            (serde_json::Value::Bool(false), Some(_)) => Err(CallError::Refused),
            _ => Err(CallError::Lost),
        }
    };
    tokio::time::timeout(limit, exchange)
        .await
        .unwrap_or(Err(CallError::Lost))
}

impl Jobs {
    pub fn new(root: PathBuf) -> Self {
        Self {
            root,
            jobs: Arc::new(Mutex::new(HashMap::new())),
            lane: Arc::new(Semaphore::new(1)),
        }
    }

    /// Queue one command, or return the operation of a known key. Never waits for the service.
    pub async fn submit(
        &self,
        actor: &str,
        key: &str,
        command: WebCommand,
    ) -> Result<Operation, Refused> {
        if !valid_key(key)
            || actor.is_empty()
            || actor.len() > 64
            || matches!(command, WebCommand::EditConfirm { .. })
        {
            return Err(Refused::Invalid);
        }
        let mut jobs = self.jobs.lock().await;
        jobs.retain(|_, j| !matches!(j.state, State::Done(_)) || j.at.elapsed() < KEEP);
        if let Some(job) = jobs.get(key) {
            if job.actor != actor || job.command != command {
                return Err(Refused::KeyReused);
            }
            return Ok(Self::view(key, &job.state));
        }
        let open = jobs
            .values()
            .filter(|j| matches!(j.state, State::Queued | State::Running))
            .count();
        if open >= MAX_OPEN {
            return Err(Refused::Busy);
        }
        jobs.insert(
            key.into(),
            Job {
                actor: actor.into(),
                command: command.clone(),
                state: State::Queued,
                at: Instant::now(),
            },
        );
        drop(jobs);
        let (this, task_key, actor) = (self.clone(), key.to_owned(), actor.to_owned());
        tokio::spawn(async move {
            let key = task_key;
            let _lane = this.lane.acquire().await;
            this.set(&key, State::Running).await;
            let args = serde_json::json!({"key": key, "actor": actor, "command": command});
            let state = match call(&this.root, "topology.web.mutate", args, MUTATE_LIMIT).await {
                Ok(v) => match serde_json::from_value::<WebReceipt>(v) {
                    Ok(r) if r.key == key => State::Done(op(&key, &r.state.clone(), Some(r))),
                    _ => State::Lost,
                },
                Err(CallError::NotSent) => State::Done(op(&key, "failed", None)),
                Err(CallError::Refused) => State::Done(op(&key, "failed", None)),
                Err(CallError::Lost) => State::Lost,
            };
            this.set(&key, state).await;
        });
        Ok(op(key, "queued", None))
    }

    async fn set(&self, key: &str, state: State) {
        if let Some(job) = self.jobs.lock().await.get_mut(key) {
            job.state = state;
            job.at = Instant::now();
        }
    }

    fn view(key: &str, state: &State) -> Operation {
        match state {
            State::Queued => op(key, "queued", None),
            State::Running => op(key, "running", None),
            State::Done(o) => o.clone(),
            State::Lost => op(key, "outcome_unknown", None),
        }
    }

    /// The operation of `key` for `actor`. A lost or unknown request asks the service receipt.
    pub async fn status(&self, actor: &str, key: &str) -> Operation {
        let known = self
            .jobs
            .lock()
            .await
            .get(key)
            .map(|j| (j.actor.clone(), j.state.clone()));
        match known {
            Some((owner, _)) if owner != actor => return op(key, "not_found", None),
            Some((_, state @ (State::Queued | State::Running | State::Done(_)))) => {
                return Self::view(key, &state);
            }
            _ => {}
        }
        let lost = known.is_some();
        if !valid_key(key) {
            return op(key, "not_found", None);
        }
        let args = serde_json::json!({"key": key, "actor": actor});
        match call(&self.root, "topology.web.operation", args, LOOKUP_LIMIT).await {
            Ok(serde_json::Value::Null) if !lost => op(key, "not_found", None),
            Ok(v) => match serde_json::from_value::<WebReceipt>(v) {
                Ok(r) if r.key == key && r.state != "outcome_unknown" => {
                    let done = op(key, &r.state.clone(), Some(r));
                    if lost {
                        self.set(key, State::Done(done.clone())).await;
                    }
                    done
                }
                _ => op(key, "outcome_unknown", None),
            },
            Err(_) => op(key, "outcome_unknown", None),
        }
    }
}
