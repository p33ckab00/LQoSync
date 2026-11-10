//! Bounded SOCK_SEQPACKET queries to the native DPDK telemetry v2 socket.
use super::{BASIC, Port, Snapshot, text};
use anyhow::{Context, ensure};
use nix::sys::socket::{self, AddressFamily, MsgFlags, SockFlag, SockType, UnixAddr};
use serde_json::Value;
use std::{
    collections::{BTreeMap, HashSet},
    os::fd::{AsRawFd, OwnedFd},
    path::Path,
};
use tokio::io::unix::AsyncFd;
const MAX_REPLY: usize = 65536;
struct Connection {
    fd: AsyncFd<OwnedFd>,
}
impl Connection {
    async fn receive(&self) -> anyhow::Result<Value> {
        let mut bytes = vec![0; MAX_REPLY];
        let n = loop {
            let mut ready = self.fd.readable().await?;
            match ready.try_io(|fd| {
                socket::recv(fd.get_ref().as_raw_fd(), &mut bytes, MsgFlags::MSG_TRUNC)
                    .map_err(std::io::Error::from)
            }) {
                Ok(result) => break result?,
                Err(_) => continue,
            }
        };
        ensure!(n > 0 && n <= MAX_REPLY, "invalid or oversized DPDK reply");
        serde_json::from_slice(&bytes[..n]).context("invalid DPDK JSON")
    }
    async fn ask(&self, command: &str) -> anyhow::Result<Value> {
        loop {
            let mut ready = self.fd.writable().await?;
            match ready.try_io(|fd| {
                socket::send(
                    fd.get_ref().as_raw_fd(),
                    command.as_bytes(),
                    MsgFlags::MSG_NOSIGNAL,
                )
                .map_err(std::io::Error::from)
            }) {
                Ok(result) => {
                    ensure!(result? == command.len(), "short DPDK request");
                    break;
                }
                Err(_) => continue,
            }
        }
        let reply = self.receive().await?;
        reply
            .get(command.split(',').next().unwrap())
            .cloned()
            .context("unexpected DPDK response")
    }
}
fn field(value: &Value, key: &str) -> anyhow::Result<Option<String>> {
    match value.get(key).filter(|v| !v.is_null()) {
        None => Ok(None),
        Some(Value::String(s)) if text(s) => Ok(Some(s.clone())),
        _ => anyhow::bail!("invalid NIC text field"),
    }
}
fn counters(value: &Value, basic: bool) -> anyhow::Result<(BTreeMap<String, u64>, bool)> {
    if value.is_null() {
        return Ok((BTreeMap::new(), false));
    }
    let map = value.as_object().context("invalid NIC counters")?;
    let mut result = BTreeMap::new();
    let capped = !basic && map.len() > 256;
    for (key, v) in map {
        if basic && !BASIC.contains(&key.as_str()) {
            continue;
        }
        ensure!(text(key), "invalid NIC counter name");
        if v.is_null() {
            continue;
        }
        let n = v.as_u64().context("invalid NIC counter value")?;
        if result.len() < 256 {
            result.insert(key.clone(), n);
        }
    }
    Ok((result, capped))
}
fn ports(value: Value) -> anyhow::Result<Vec<u16>> {
    let all: Vec<u16> = serde_json::from_value(value).context("invalid NIC port list")?;
    ensure!(all.len() <= 1024, "too many DPDK port IDs");
    let mut unique = HashSet::new();
    ensure!(
        all.iter().all(|v| unique.insert(*v)),
        "duplicate NIC port ID"
    );
    Ok(all)
}
pub async fn sample(path: &Path) -> anyhow::Result<Snapshot> {
    tokio::time::timeout(std::time::Duration::from_secs(3), async {
        let fd = socket::socket(
            AddressFamily::Unix,
            SockType::SeqPacket,
            SockFlag::SOCK_CLOEXEC | SockFlag::SOCK_NONBLOCK,
            None,
        )?;
        socket::connect(fd.as_raw_fd(), &UnixAddr::new(path)?)
            .context("DPDK NIC socket unavailable")?;
        let peer = socket::getsockopt(&fd, socket::sockopt::PeerCredentials)?;
        ensure!(peer.pid() > 0, "invalid DPDK peer PID");
        let connection = Connection {
            // SAFETY: AsyncFd owns this OwnedFd; it stays open and its descriptor never changes.
            fd: unsafe { AsyncFd::register(fd)? },
        };
        let welcome = connection.receive().await?;
        let version = field(&welcome, "version")?.context("DPDK version unavailable")?;
        let available: Vec<String> = serde_json::from_value(connection.ask("/").await?)
            .context("invalid DPDK endpoint list")?;
        ensure!(
            available.len() <= 256 && available.iter().all(|s| text(s)),
            "invalid DPDK endpoint names"
        );
        ensure!(
            available.iter().any(|s| s == "/ethdev/list"),
            "NIC endpoints unavailable"
        );
        let ids = ports(connection.ask("/ethdev/list").await?)?;
        let mut result = Vec::new();
        for id in ids.iter().copied().take(8) {
            let mut port = Port {
                id,
                ..Default::default()
            };
            for endpoint in ["info", "link_status", "stats", "xstats"] {
                let key = format!("/ethdev/{endpoint}");
                if !available.contains(&key) {
                    continue;
                }
                let v = connection.ask(&format!("{key},{id}")).await?;
                match endpoint {
                    "info" => port.name = field(&v, "name")?,
                    "link_status" => {
                        port.status = field(&v, "status")?;
                        port.duplex = field(&v, "duplex")?;
                        port.speed_mbps = v
                            .get("speed")
                            .filter(|v| !v.is_null())
                            .map(|v| {
                                v.as_u64()
                                    .and_then(|n| u32::try_from(n).ok())
                                    .context("invalid NIC link speed")
                            })
                            .transpose()?;
                    }
                    "stats" => port.stats = counters(&v, true)?.0,
                    _ => {
                        let (values, capped) = counters(&v, false)?;
                        port.xstats = values;
                        port.xstats_capped = capped;
                    }
                }
            }
            result.push(port);
        }
        ensure!(
            ports(connection.ask("/ethdev/list").await?)? == ids,
            "DPDK port list changed during read"
        );
        if available.iter().any(|s| s == "/ethdev/info") {
            for p in &result {
                ensure!(
                    field(
                        &connection.ask(&format!("/ethdev/info,{}", p.id)).await?,
                        "name"
                    )? == p.name,
                    "DPDK port identity changed during read"
                );
            }
        }
        Ok(Snapshot {
            pid: peer.pid() as u32,
            version,
            ports: result,
            ports_capped: ids.len() > 8,
        })
    })
    .await
    .context("DPDK NIC request timed out")?
}
