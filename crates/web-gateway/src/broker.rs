//! Fixed private IPC: credential checks and cached overview only, never arbitrary operations.
use crate::accounts::{Account, Accounts};
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
use web_contract::Overview;
pub fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}
#[derive(Serialize, Deserialize)]
#[serde(tag = "op", deny_unknown_fields)]
pub enum Request {
    Login { username: String, password: String },
    Active { id: i64, epoch: i64 },
    Overview,
}
#[derive(Serialize, Deserialize)]
pub struct Reply {
    pub ok: bool,
    pub account: Option<Account>,
    pub overview: Option<Overview>,
}
impl Reply {
    fn no() -> Self {
        Self {
            ok: false,
            account: None,
            overview: None,
        }
    }
}
pub async fn exchange(path: &Path, request: &Request) -> Result<Reply> {
    tokio::time::timeout(Duration::from_secs(10), async {
        let mut stream = UnixStream::connect(path).await?;
        let mut body = serde_json::to_vec(request)?;
        body.push(b'\n');
        ensure!(body.len() <= 4096, "request too large");
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
) -> Result<()> {
    let cached = Arc::new(RwLock::new(Overview {
        stale: true,
        traffic_stale: true,
        ..Default::default()
    }));
    let cache = cached.clone();
    tokio::spawn(async move {
        let mut tick = tokio::time::interval(Duration::from_secs(1));
        tick.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
        let mut cycle = 0u8;
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
                *cache.write().await = v;
            }
        }
    });
    let connections = Arc::new(Semaphore::new(16));
    let hashes = Arc::new(Semaphore::new(2));
    let limits = Arc::new(Mutex::new(HashMap::<String, (u64, u32)>::new()));
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
        let hashes = hashes.clone();
        let limits = limits.clone();
        tokio::spawn(async move {
            let _permit = permit;
            let result = tokio::time::timeout(Duration::from_secs(12), async {
                let (read, mut write) = stream.into_split();
                let mut line = String::new();
                BufReader::new(read).take(4097).read_line(&mut line).await?;
                ensure!(
                    line.len() <= 4096 && line.ends_with('\n'),
                    "invalid request"
                );
                let request: Request = serde_json::from_str(&line)?;
                let mut reply = Reply::no();
                match request {
                    Request::Overview => {
                        reply.ok = true;
                        reply.overview = Some(cache.read().await.clone());
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
                write.write_all(&body).await?;
                Ok::<(), anyhow::Error>(())
            })
            .await;
            // Never log the request body or credentials, even when parsing fails.
            let _ = result;
        });
    }
}
