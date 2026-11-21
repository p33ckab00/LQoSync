//! Fixed private IPC: credential checks and cached overview only, never arbitrary operations.
use crate::accounts::{Account, Accounts};
use crate::history::History;
use anyhow::{Result, ensure};
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    sync::Arc,
    time::{Duration, SystemTime, UNIX_EPOCH},
};
use tokio::{
    io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader},
    net::{UnixListener, UnixStream},
    sync::{Mutex, RwLock, Semaphore},
};
use web_contract::routers::{
    RouterActivity, RouterEditForm, RouterEditOutcome, RouterEditRequest, RouterTestResult,
};
use web_contract::{Overview, TrafficWindow};

/// The request line limit. A router edit can carry a CA certificate (16 KiB) or 500 client overrides.
const MAX_REQUEST: usize = 64 * 1024;

/// The core command that saves router edits: `<bin> router-edit --config <config>`.
#[derive(Clone, Debug)]
pub struct Editor {
    pub bin: PathBuf,
    pub config: PathBuf,
}
pub fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}
#[derive(Serialize, Deserialize)]
#[serde(tag = "op", deny_unknown_fields)]
pub enum Request {
    Login {
        username: String,
        password: String,
    },
    Active {
        id: i64,
        epoch: i64,
    },
    Overview,
    /// One bucketed chart window: `1h`, `6h`, or `24h`.
    Traffic {
        window: String,
    },
    /// The cached router inventory. No router I/O.
    Routers,
    /// One local router change. The gateway sets `actor` from the session, never from the browser.
    RouterEdit {
        actor: String,
        router_id: Option<String>,
        form: Box<RouterEditForm>,
    },
    /// The audit events of one router.
    RouterActivity {
        router_id: String,
    },
    /// One router with its lists.
    RouterDetail {
        router_id: String,
    },
    /// One connection test, run by the service through its API socket.
    RouterTest {
        actor: String,
        router_id: String,
    },
    /// Start a cycle now for all enabled routers.
    SyncNow,
}
#[derive(Serialize, Deserialize)]
pub struct Reply {
    pub ok: bool,
    pub account: Option<Account>,
    pub overview: Option<Overview>,
    #[serde(default)]
    pub traffic: Option<TrafficWindow>,
    #[serde(default)]
    pub routers: Option<web_contract::routers::RouterInventory>,
    #[serde(default)]
    pub edit: Option<RouterEditOutcome>,
    #[serde(default)]
    pub activity: Option<RouterActivity>,
    #[serde(default)]
    pub test: Option<RouterTestResult>,
    #[serde(default)]
    pub router: Option<web_contract::routers::RouterSummary>,
}
impl Reply {
    fn no() -> Self {
        Self {
            ok: false,
            account: None,
            overview: None,
            traffic: None,
            routers: None,
            edit: None,
            activity: None,
            test: None,
            router: None,
        }
    }
}
pub async fn exchange(path: &Path, request: &Request) -> Result<Reply> {
    exchange_within(path, request, Duration::from_secs(10)).await
}
/// A router edit runs the core command, so it gets a longer limit than a cache read.
pub async fn exchange_within(path: &Path, request: &Request, limit: Duration) -> Result<Reply> {
    tokio::time::timeout(limit, async {
        let mut stream = UnixStream::connect(path).await?;
        let mut body = serde_json::to_vec(request)?;
        body.push(b'\n');
        ensure!(body.len() <= MAX_REQUEST, "request too large");
        stream.write_all(&body).await?;
        let mut line = String::new();
        BufReader::new(stream)
            .take(65537)
            .read_line(&mut line)
            .await?;
        ensure!(
            line.len() <= 65536 && line.ends_with('\n'),
            "invalid broker response"
        );
        Ok(serde_json::from_str(&line)?)
    })
    .await?
}
pub async fn serve(
    listener: UnixListener,
    uid: u32,
    accounts: Accounts,
    state_dir: PathBuf,
    history_path: Option<PathBuf>,
    editor: Option<Editor>,
) -> Result<()> {
    let cached = Arc::new(RwLock::new(Overview {
        stale: true,
        traffic_stale: true,
        ..Default::default()
    }));
    let history = Arc::new(RwLock::new(
        history_path
            .as_deref()
            .map(History::load)
            .unwrap_or_default(),
    ));
    let cache = cached.clone();
    let recorder = history.clone();
    let inventory_root = state_dir.clone();
    tokio::spawn(async move {
        let mut tick = tokio::time::interval(Duration::from_secs(1));
        tick.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
        let mut cycle = 0u8;
        let mut seconds = 0u32;
        // Seed once from the full core history (about one hour), so the 1h window starts filled.
        let path = state_dir.clone();
        if let Ok(seed) =
            tokio::task::spawn_blocking(move || crate::overview::history_samples(&path, now()))
                .await
        {
            let mut h = recorder.write().await;
            for (source, points) in seed.iter().enumerate() {
                h.observe(source, points);
            }
        }
        loop {
            tick.tick().await;
            let path = state_dir.clone();
            let mut previous = cache.read().await.clone();
            let full = cycle == 0;
            cycle = (cycle + 1) % 5;
            if let Ok(v) = tokio::task::spawn_blocking(move || {
                if full {
                    crate::overview::project(&path, now())
                } else {
                    crate::overview::refresh_traffic(&path, now(), &mut previous);
                    previous
                }
            })
            .await
            {
                {
                    let mut h = recorder.write().await;
                    h.observe(0, &v.traffic);
                    if let Some(c) = &v.classification {
                        h.observe(1, &c.mapped.points);
                        h.observe(2, &c.unmapped.points);
                    }
                }
                *cache.write().await = v;
            }
            seconds = seconds.wrapping_add(1);
            if seconds.is_multiple_of(60)
                && let Some(path) = history_path.clone()
            {
                let snapshot = recorder.read().await.clone();
                // A failed save keeps the history in memory. The next save tries again.
                let _ = tokio::task::spawn_blocking(move || snapshot.save(&path)).await;
            }
        }
    });
    let connections = Arc::new(Semaphore::new(16));
    let hashes = Arc::new(Semaphore::new(2));
    let limits = Arc::new(Mutex::new(HashMap::<String, (u64, u32)>::new()));
    // One config writer at a time: router edits are serialized.
    let edits = Arc::new(Semaphore::new(1));
    // One connection test at a time; the service runs it between cycles.
    let tests = Arc::new(Semaphore::new(1));
    let editor = editor.map(Arc::new);
    loop {
        let (stream, _) = listener.accept().await?;
        if stream.peer_cred()?.uid() != uid {
            continue;
        }
        let Ok(permit) = connections.clone().try_acquire_owned() else {
            continue;
        };
        let accounts = accounts.clone();
        let cache = cached.clone();
        let history = history.clone();
        let hashes = hashes.clone();
        let limits = limits.clone();
        let root = inventory_root.clone();
        let edits = edits.clone();
        let tests = tests.clone();
        let editor = editor.clone();
        tokio::spawn(async move {
            let _permit = permit;
            // A connection test waits for the current cycle, then for its own router timeouts.
            let result = tokio::time::timeout(Duration::from_secs(75), async {
                let (read, mut write) = stream.into_split();
                let mut line = String::new();
                BufReader::new(read)
                    .take(MAX_REQUEST as u64 + 1)
                    .read_line(&mut line)
                    .await?;
                ensure!(
                    line.len() <= MAX_REQUEST && line.ends_with('\n'),
                    "invalid request"
                );
                let request: Request = serde_json::from_str(&line)?;
                let mut reply = Reply::no();
                match request {
                    Request::Overview => {
                        reply.ok = true;
                        reply.overview = Some(cache.read().await.clone());
                    }
                    Request::Traffic { window } => {
                        ensure!(window.len() <= 8, "invalid input");
                        reply.traffic = history.read().await.window(&window, now());
                        reply.ok = reply.traffic.is_some();
                    }
                    Request::Routers => {
                        reply.routers =
                            tokio::task::spawn_blocking(move || crate::routers::load(&root))
                                .await?;
                        reply.ok = reply.routers.is_some();
                    }
                    Request::RouterDetail { router_id } => {
                        ensure!(router_id.len() <= 64, "invalid input");
                        reply.router = tokio::task::spawn_blocking(move || {
                            crate::routers::load_one(&root, &router_id)
                        })
                        .await?;
                        reply.ok = reply.router.is_some();
                    }
                    Request::RouterTest { actor, router_id } => {
                        ensure!(actor.len() <= 64 && router_id.len() <= 64, "invalid input");
                        reply.test = Some(match tests.clone().try_acquire_owned() {
                            Ok(_one) => crate::routers::core_test(&root, &actor, &router_id).await,
                            Err(_) => RouterTestResult {
                                router_id,
                                class: "busy".into(),
                                at: now(),
                                ..Default::default()
                            },
                        });
                        reply.ok = true;
                    }
                    Request::SyncNow => {
                        let Some(editor) = editor.as_deref() else {
                            anyhow::bail!("no router editor");
                        };
                        reply.edit = Some(crate::routers::run_sync(editor).await);
                        reply.ok = true;
                    }
                    Request::RouterActivity { router_id } => {
                        ensure!(router_id.len() <= 64, "invalid input");
                        reply.activity = tokio::task::spawn_blocking(move || {
                            crate::routers::activity(&root, &router_id)
                        })
                        .await
                        .ok();
                        reply.ok = reply.activity.is_some();
                    }
                    Request::RouterEdit {
                        actor,
                        router_id,
                        form,
                    } => {
                        ensure!(
                            actor.len() <= 64 && router_id.as_ref().is_none_or(|r| r.len() <= 64),
                            "invalid input"
                        );
                        let Some(editor) = editor.as_deref() else {
                            anyhow::bail!("no router editor");
                        };
                        // A credential replacement or a disable needs the operator password again (owner decision D2).
                        let replaces = router_id.is_some()
                            && (form.edit.password.is_some() || form.edit.enabled == Some(false));
                        let confirmed = match (replaces, form.current_password.clone()) {
                            (false, _) => true,
                            // Only a real password try counts against the login rate limit.
                            (true, Some(password)) if password.len() <= 256 => {
                                let allowed = {
                                    let mut map = limits.lock().await;
                                    map.retain(|_, (at, _)| now().saturating_sub(*at) < 60);
                                    let counter = map.entry(actor.clone()).or_insert((now(), 0));
                                    counter.1 += 1;
                                    counter.1 <= 5
                                };
                                match (allowed, hashes.try_acquire_owned()) {
                                    (true, Ok(_hash)) => {
                                        let (a, user) = (accounts.clone(), actor.clone());
                                        tokio::task::spawn_blocking(move || {
                                            a.verify(&user, &password)
                                        })
                                        .await?
                                        .is_ok()
                                    }
                                    _ => false,
                                }
                            }
                            (true, _) => false,
                        };
                        reply.edit = Some(if confirmed {
                            let _one = edits.acquire().await?;
                            let request = RouterEditRequest {
                                actor,
                                router_id,
                                edit: form.edit,
                            };
                            crate::routers::run_editor(editor, &request).await
                        } else {
                            RouterEditOutcome {
                                code: "reauth".into(),
                                message: "Enter your current password to confirm this change."
                                    .into(),
                                reply: None,
                            }
                        });
                        reply.ok = true;
                    }
                    Request::Active { id, epoch } => {
                        reply.ok = tokio::task::spawn_blocking(move || accounts.active(id, epoch))
                            .await??;
                    }
                    Request::Login { username, password } => {
                        ensure!(
                            username.len() <= 64 && password.len() <= 256,
                            "invalid input"
                        );
                        let allowed = {
                            let mut map = limits.lock().await;
                            map.retain(|_, (at, _)| now().saturating_sub(*at) < 60);
                            if map.len() >= 1024 && !map.contains_key(&username) {
                                false
                            } else {
                                let counter = map.entry(username.clone()).or_insert((now(), 0));
                                counter.1 += 1;
                                counter.1 <= 5
                            }
                        };
                        if allowed && let Ok(_hash) = hashes.try_acquire_owned() {
                            reply.account = tokio::task::spawn_blocking(move || {
                                accounts.verify(&username, &password)
                            })
                            .await?
                            .ok();
                            reply.ok = reply.account.is_some();
                        }
                    }
                }
                let mut body = serde_json::to_vec(&reply)?;
                body.push(b'\n');
                ensure!(body.len() <= 65536, "broker response too large");
                write.write_all(&body).await?;
                Ok::<(), anyhow::Error>(())
            })
            .await;
            // Never log the request body or credentials, even when parsing fails.
            let _ = result;
        });
    }
}
