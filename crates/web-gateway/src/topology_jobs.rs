//! Web mutations: one broker lane to the service, idempotent by request key.
//!
//! The service receipt store is the durable record. This broker map only tracks requests in
//! flight. A reply that is lost after the request left is never sent again: the status asks the
//! service for the receipt of that key instead.
//!
//! A Builder proposal is reviewed first. Its service token stays here, bound to the actor and
//! the service expiry; the browser confirms by the proposal key only.
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt};
use tokio::sync::{Mutex, Semaphore};
use web_contract::topology::{
    CapacityChange, DiscoveredRouter, Discovery, LayoutRequest, Moved, Node, NodeChange, Operation,
    ProposalRequest, RateChange, ReadJob, ReadRequest, Review, SourceCandidate, WanCandidate,
    WebCommand, WebReceipt,
};

/// Queued and running mutations at one time, for all actors.
pub const MAX_OPEN: usize = 16;
/// The service answers between cycles, so a mutation can wait for a whole cycle.
const MUTATE_LIMIT: Duration = Duration::from_secs(600);
const LOOKUP_LIMIT: Duration = Duration::from_secs(10);
/// Finished operations stay in memory this long; the service keeps the receipt longer.
const KEEP: Duration = Duration::from_secs(3600);
/// Moves listed in a review. The review counts all of them.
const MOVE_SAMPLE: usize = 20;

#[derive(Debug, PartialEq, Eq)]
pub enum Refused {
    /// Not a valid key, or a command that the browser cannot send.
    Invalid,
    /// The key belongs to another actor or command.
    KeyReused,
    /// Too many operations are open.
    Busy,
    /// No current proposal of this actor has that key.
    Expired,
    /// The saved layout has another revision. Load the view again.
    Conflict,
    /// The service did not answer.
    Unavailable,
}

#[derive(Clone)]
enum State {
    Queued,
    Running,
    Done(Box<Operation>),
    /// The request left, and no valid answer came back.
    Lost,
}

struct Job {
    actor: String,
    /// The request as JSON: a repeated key must carry the same request.
    payload: serde_json::Value,
    state: State,
    at: Instant,
}

/// A reviewed proposal: the service token, its owner, and the service expiry (Unix seconds).
struct Lease {
    actor: String,
    token: String,
    expires_at: u64,
}

#[derive(Clone)]
pub struct Jobs {
    root: PathBuf,
    jobs: Arc<Mutex<HashMap<String, Job>>>,
    leases: Arc<Mutex<HashMap<String, Lease>>>,
    lane: Arc<Semaphore>,
}

enum CallError {
    /// No connection: the service did not get the request.
    NotSent,
    /// The request may have arrived; no valid answer came back.
    Lost,
    /// The service refused the request before any change. A rule text when it is safe to show.
    Refused(Option<String>),
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
        review: None,
        error: None,
        discovery: None,
    }
}

fn unix_now() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_secs())
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
            .take(1024 * 1024)
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
            (serde_json::Value::Bool(false), Some(code)) => {
                // Rule and conflict texts name the rule; other errors can name internal paths.
                let safe = matches!(code, "invalid" | "conflict")
                    .then(|| v["error"]["message"].as_str())
                    .flatten()
                    .map(|m| m.chars().take(300).collect());
                Err(CallError::Refused(safe))
            }
            _ => Err(CallError::Lost),
        }
    };
    tokio::time::timeout(limit, exchange)
        .await
        .unwrap_or(Err(CallError::Lost))
}

/// The review of one service proposal, with node changes against the cached view.
fn review(key: &str, base: u64, v: &serde_json::Value, current: &[Node]) -> Review {
    let count = |k: &str| {
        v[k].as_array()
            .map_or(v[k].as_u64().unwrap_or(0), |a| a.len() as u64)
    };
    let text = |x: &serde_json::Value| x.as_str().map(str::to_owned);
    let after: Vec<Node> = serde_json::from_value(v["nodes"].clone()).unwrap_or_default();
    let mut nodes = Vec::new();
    for n in &after {
        let change = match current.iter().find(|c| c.id == n.id) {
            None => "added",
            Some(c) if c.parent != n.parent => "moved",
            Some(c) if c.name != n.name || c.capacity != n.capacity => "changed",
            Some(_) => continue,
        };
        nodes.push(NodeChange {
            id: n.id.clone(),
            name: n.name.clone(),
            change: change.into(),
        });
    }
    for c in current
        .iter()
        .filter(|c| !after.iter().any(|n| n.id == c.id))
    {
        nodes.push(NodeChange {
            id: c.id.clone(),
            name: c.name.clone(),
            change: "removed".into(),
        });
    }
    let list = |k: &str| -> Vec<String> {
        v[k].as_array()
            .map(|a| {
                a.iter()
                    .filter_map(|x| x.as_str().map(str::to_owned))
                    .collect()
            })
            .unwrap_or_default()
    };
    Review {
        proposal: key.into(),
        expires_at: v["expires_at"].as_u64().unwrap_or(0),
        base_revision: base.to_string(),
        moves: count("moves"),
        moved: v["moves"]
            .as_array()
            .into_iter()
            .flatten()
            .take(MOVE_SAMPLE)
            .map(|m| Moved {
                subscriber_id: text(&m["subscriber_id"]).unwrap_or_default(),
                label: text(&m["label"]),
                from: text(&m["from"]),
                to: text(&m["to"]),
            })
            .collect(),
        cleared: count("cleared"),
        assigned: count("assigned"),
        new_records: count("new_records"),
        sources: list("sources"),
        nodes,
        warnings: list("warnings"),
        rates_changed: v["rates_changed"].as_u64().unwrap_or(0),
        rates: v["rates"]
            .as_array()
            .into_iter()
            .flatten()
            .take(MOVE_SAMPLE)
            .map(|r| RateChange {
                subscriber_id: text(&r["subscriber_id"]).unwrap_or_default(),
                label: text(&r["label"]),
                before: rates(&r["before"]),
                after: rates(&r["after"]),
            })
            .collect(),
        capacities: v["capacities"]
            .as_array()
            .into_iter()
            .flatten()
            .map(|c| CapacityChange {
                node_id: text(&c["node_id"]).unwrap_or_default(),
                before: mbps(&c["before_mbps"]),
                after: mbps(&c["after_mbps"]),
            })
            .collect(),
    }
}

/// `[down, up]` in Mbps as text. The service value stays exact; this is display only.
fn mbps(v: &serde_json::Value) -> String {
    match (v[0].as_f64(), v[1].as_f64()) {
        (Some(d), Some(u)) => format!("{d} / {u} Mbps"),
        _ => "—".into(),
    }
}

fn rates(v: &serde_json::Value) -> String {
    format!("min {}, max {}", mbps(&v["min_mbps"]), mbps(&v["max_mbps"]))
}

/// The typed answer of one discovery. Unknown fields stay out; missing fields become empty.
fn discovery(v: &serde_json::Value) -> Discovery {
    let text = |x: &serde_json::Value| x.as_str().unwrap_or_default().to_owned();
    let flag = |x: &serde_json::Value| x.as_bool().unwrap_or(false);
    let items = |x: &serde_json::Value| x.as_array().cloned().unwrap_or_default();
    Discovery {
        // Source discovery has no time of its own; the read finished now.
        collected_at: v["collected_at"].as_u64().unwrap_or_else(unix_now),
        routers: items(&v["routers"])
            .iter()
            .map(|r| DiscoveredRouter {
                router_id: text(&r["router_id"]),
                name: text(&r["name"]),
                state: text(&r["state"]),
                error: r["error"].as_str().map(|e| e.chars().take(300).collect()),
                candidates: items(&r["candidates"])
                    .iter()
                    .map(|c| WanCandidate {
                        name: text(&c["name"]),
                        kind: text(&c["type"]),
                        running: flag(&c["running"]),
                        disabled: flag(&c["disabled"]),
                        suggested: flag(&c["suggested"]),
                        used_by: items(&c["used_by"]).iter().map(text).collect(),
                    })
                    .collect(),
            })
            .collect(),
        sources: items(&v["candidates"])
            .iter()
            .map(|c| SourceCandidate {
                key: text(&c["key"]),
                kind: text(&c["kind"]),
                router: text(&c["router"]),
                name: text(&c["name"]),
                rebind: flag(&c["rebind"]),
            })
            .collect(),
    }
}

impl Jobs {
    pub fn new(root: PathBuf) -> Self {
        Self {
            root,
            jobs: Arc::new(Mutex::new(HashMap::new())),
            leases: Arc::new(Mutex::new(HashMap::new())),
            lane: Arc::new(Semaphore::new(1)),
        }
    }

    /// Record a new job, or return the operation of a known key.
    async fn admit(
        &self,
        actor: &str,
        key: &str,
        payload: serde_json::Value,
    ) -> Result<Option<Operation>, Refused> {
        if !valid_key(key) || actor.is_empty() || actor.len() > 64 {
            return Err(Refused::Invalid);
        }
        let mut jobs = self.jobs.lock().await;
        jobs.retain(|_, j| !matches!(j.state, State::Done(_)) || j.at.elapsed() < KEEP);
        if let Some(job) = jobs.get(key) {
            if job.actor != actor || job.payload != payload {
                return Err(Refused::KeyReused);
            }
            return Ok(Some(Self::view(key, &job.state)));
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
                payload,
                state: State::Queued,
                at: Instant::now(),
            },
        );
        Ok(None)
    }

    /// Save all canvas card positions. Layout only: no receipt, no shaping change.
    pub async fn layout(&self, request: LayoutRequest) -> Result<u64, Refused> {
        let valid = request.positions.len() <= 4096
            && request.positions.iter().all(|(id, point)| {
                (1..=64).contains(&id.len())
                    && point.iter().all(|v| v.is_finite() && v.abs() <= 1e7)
            });
        if !valid {
            return Err(Refused::Invalid);
        }
        let args = serde_json::to_value(&request).map_err(|_| Refused::Invalid)?;
        match call(&self.root, "topology.layout.save", args, LOOKUP_LIMIT).await {
            Ok(v) => v["revision"].as_u64().ok_or(Refused::Unavailable),
            Err(CallError::Refused(_)) => Err(Refused::Conflict),
            Err(_) => Err(Refused::Unavailable),
        }
    }

    /// Queue one command, or return the operation of a known key. Never waits for the service.
    pub async fn submit(
        &self,
        actor: &str,
        key: &str,
        command: WebCommand,
    ) -> Result<Operation, Refused> {
        // The browser never holds a service proposal token.
        if matches!(command, WebCommand::EditConfirm { .. }) {
            return Err(Refused::Invalid);
        }
        self.run_command(actor, key, command).await
    }

    async fn run_command(
        &self,
        actor: &str,
        key: &str,
        command: WebCommand,
    ) -> Result<Operation, Refused> {
        let payload = serde_json::to_value(&command).map_err(|_| Refused::Invalid)?;
        if let Some(known) = self.admit(actor, key, payload).await? {
            return Ok(known);
        }
        let (this, task_key, actor) = (self.clone(), key.to_owned(), actor.to_owned());
        tokio::spawn(async move {
            let key = task_key;
            let _lane = this.lane.acquire().await;
            this.set(&key, State::Running).await;
            let args = serde_json::json!({"key": key, "actor": actor, "command": command});
            let state = match call(&this.root, "topology.web.mutate", args, MUTATE_LIMIT).await {
                Ok(v) => match serde_json::from_value::<WebReceipt>(v) {
                    Ok(r) if r.key == key => {
                        State::Done(Box::new(op(&key, &r.state.clone(), Some(r))))
                    }
                    _ => State::Lost,
                },
                Err(CallError::NotSent) => State::Done(Box::new(op(&key, "failed", None))),
                Err(CallError::Refused(error)) => State::Done(Box::new(Operation {
                    error,
                    ..op(&key, "failed", None)
                })),
                Err(CallError::Lost) => State::Lost,
            };
            this.set(&key, state).await;
        });
        Ok(op(key, "queued", None))
    }

    /// Ask the service for an impact review. The operation becomes `review_ready` with the
    /// review, or `failed` with the service rule. A proposal makes no commit.
    pub async fn propose(
        &self,
        actor: &str,
        request: ProposalRequest,
    ) -> Result<Operation, Refused> {
        let key = request.key.clone();
        let payload = serde_json::to_value(&request).map_err(|_| Refused::Invalid)?;
        if let Some(known) = self.admit(actor, &key, payload).await? {
            return Ok(known);
        }
        let (this, actor) = (self.clone(), actor.to_owned());
        tokio::spawn(async move {
            let _lane = this.lane.acquire().await;
            this.set(&request.key, State::Running).await;
            let args = serde_json::json!({
                "expected_revision": request.expected_revision, "edit": request.edit,
            });
            let state = match call(&this.root, "topology.edit.propose", args, MUTATE_LIMIT).await {
                Ok(v) => match v["proposal_id"].as_str() {
                    Some(token) => {
                        let root = this.root.clone();
                        let view =
                            tokio::task::spawn_blocking(move || crate::topology::load(&root))
                                .await
                                .ok()
                                .flatten();
                        let base = request.expected_revision;
                        // Node changes compare with the cached tree of the same revision only.
                        let current = view
                            .filter(|v| v.registry_revision == Some(base.to_string()))
                            .map(|v| v.nodes);
                        let mut r =
                            review(&request.key, base, &v, current.as_deref().unwrap_or(&[]));
                        if current.is_none() {
                            r.nodes.clear();
                            r.warnings.push(
                                "Node changes are not listed: the cached view is from another revision."
                                    .into(),
                            );
                        }
                        this.leases.lock().await.insert(
                            request.key.clone(),
                            Lease {
                                actor,
                                token: token.into(),
                                expires_at: r.expires_at,
                            },
                        );
                        State::Done(Box::new(Operation {
                            review: Some(r),
                            ..op(&request.key, "review_ready", None)
                        }))
                    }
                    None => State::Done(Box::new(op(&request.key, "failed", None))),
                },
                Err(CallError::Refused(error)) => State::Done(Box::new(Operation {
                    error,
                    ..op(&request.key, "failed", None)
                })),
                // No proposal is known; a new review is safe.
                Err(_) => State::Done(Box::new(op(&request.key, "failed", None))),
            };
            this.set(&request.key, state).await;
        });
        Ok(op(&key, "queued", None))
    }

    /// Start one explicit router read. The answer arrives on the operation (`done`).
    pub async fn read(&self, actor: &str, request: ReadRequest) -> Result<Operation, Refused> {
        let key = request.key.clone();
        let payload = serde_json::to_value(&request).map_err(|_| Refused::Invalid)?;
        if let Some(known) = self.admit(actor, &key, payload).await? {
            return Ok(known);
        }
        let this = self.clone();
        tokio::spawn(async move {
            let _lane = this.lane.acquire().await;
            this.set(&request.key, State::Running).await;
            let operation = match request.job {
                ReadJob::Refresh => "topology.preview.refresh",
                ReadJob::Wan => "wan.discover",
                ReadJob::Sources => "topology.sources.candidates",
            };
            let failed = |error: Option<String>| Operation {
                error,
                ..op(&request.key, "failed", None)
            };
            let state = match call(&this.root, operation, serde_json::json!({}), MUTATE_LIMIT).await
            {
                Ok(v) => Operation {
                    discovery: (request.job != ReadJob::Refresh).then(|| discovery(&v)),
                    ..op(&request.key, "done", None)
                },
                Err(CallError::Refused(error)) => failed(error),
                Err(_) => failed(Some("The service did not answer the read.".into())),
            };
            this.set(&request.key, State::Done(Box::new(state))).await;
        });
        Ok(op(&key, "queued", None))
    }

    /// Save the reviewed proposal of this actor once, under the receipt key `key`.
    pub async fn confirm(
        &self,
        actor: &str,
        proposal: &str,
        key: &str,
    ) -> Result<Operation, Refused> {
        let known = self
            .jobs
            .lock()
            .await
            .get(key)
            .map(|j| (j.actor.clone(), j.state.clone()));
        if let Some((owner, state)) = known {
            return if owner == actor {
                Ok(Self::view(key, &state))
            } else {
                Err(Refused::KeyReused)
            };
        }
        let token = {
            let mut leases = self.leases.lock().await;
            match leases.get(proposal) {
                Some(l) if l.actor == actor && unix_now() < l.expires_at => {
                    leases.remove(proposal).map(|l| l.token)
                }
                _ => None,
            }
        }
        .ok_or(Refused::Expired)?;
        self.run_command(actor, key, WebCommand::EditConfirm { proposal_id: token })
            .await
    }

    /// Drop the proposal of this actor and ask the service to drop its kept copy.
    pub async fn cancel(&self, actor: &str, proposal: &str) -> bool {
        let lease = {
            let mut leases = self.leases.lock().await;
            match leases.get(proposal) {
                Some(l) if l.actor == actor => leases.remove(proposal),
                _ => None,
            }
        };
        let Some(lease) = lease else {
            return false;
        };
        let this = self.clone();
        tokio::spawn(async move {
            let _lane = this.lane.acquire().await;
            let args = serde_json::json!({"proposal_id": lease.token});
            let _ = call(&this.root, "topology.edit.cancel", args, LOOKUP_LIMIT).await;
        });
        true
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
            State::Done(o) => (**o).clone(),
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
                        self.set(key, State::Done(Box::new(done.clone()))).await;
                    }
                    done
                }
                _ => op(key, "outcome_unknown", None),
            },
            Err(_) => op(key, "outcome_unknown", None),
        }
    }
}

/// Put a queue result into a broker reply.
pub fn answer(reply: &mut crate::broker::Reply, result: Result<Operation, Refused>) {
    match result {
        Ok(operation) => reply.operation = Some(operation),
        Err(refused) => {
            reply.refused = Some(
                match refused {
                    Refused::Invalid => "invalid",
                    Refused::KeyReused => "key_reused",
                    Refused::Busy => "busy",
                    Refused::Expired => "expired",
                    Refused::Conflict => "conflict",
                    Refused::Unavailable => "unavailable",
                }
                .into(),
            )
        }
    }
    reply.ok = reply.operation.is_some();
}
