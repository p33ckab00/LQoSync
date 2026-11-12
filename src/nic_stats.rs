//! Native DPDK NIC counters. Port RX/TX stays separate from subscriber traffic and shaper drops.
mod ipc;
use anyhow::ensure;
pub use ipc::sample;
use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, path::Path};

pub const CACHE: &str = "nic-telemetry.json";
pub const POLL_SECS: u64 = 10;
pub const MAX_AGE: u64 = 15;
const MAX_BYTES: u64 = 1024 * 1024;
pub const BASIC: [&str; 8] = [
    "ipackets",
    "opackets",
    "ibytes",
    "obytes",
    "imissed",
    "ierrors",
    "oerrors",
    "rx_nombuf",
];
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct Port {
    pub id: u16,
    pub name: Option<String>,
    pub status: Option<String>,
    pub speed_mbps: Option<u32>,
    pub duplex: Option<String>,
    pub stats: BTreeMap<String, u64>,
    pub xstats: BTreeMap<String, u64>,
    pub xstats_capped: bool,
    pub deltas: BTreeMap<String, u64>,
    pub reset: bool,
}
#[derive(Debug)]
pub struct Snapshot {
    pub pid: u32,
    pub version: String,
    pub ports: Vec<Port>,
    pub ports_capped: bool,
}
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct Report {
    pub version: u32,
    pub updated: u64,
    pub stale: bool,
    pub reason: Option<String>,
    pub dpdk_version: Option<String>,
    pub pid: Option<u32>,
    pub ports: Vec<Port>,
    pub ports_capped: bool,
    pub interval_ms: Option<u64>,
}
fn text(s: &str) -> bool {
    !s.is_empty() && s.len() <= 128 && !s.chars().any(char::is_control)
}
impl Report {
    fn valid(&self) -> bool {
        let mut ids = std::collections::HashSet::new();
        self.version == 1
            && self.ports.len() <= 8
            && self
                .reason
                .as_ref()
                .is_none_or(|v| v.len() <= 256 && !v.chars().any(char::is_control))
            && self.dpdk_version.as_ref().is_none_or(|v| text(v))
            && self.ports.iter().all(|p| {
                ids.insert(p.id)
                    && p.name.as_ref().is_none_or(|v| text(v))
                    && p.status.as_ref().is_none_or(|v| text(v))
                    && p.duplex.as_ref().is_none_or(|v| text(v))
                    && p.stats.len() <= BASIC.len()
                    && p.stats.keys().all(|k| BASIC.contains(&k.as_str()))
                    && p.xstats.len() <= 256
                    && p.xstats.keys().all(|k| text(k))
                    && p.deltas.len() <= BASIC.len()
                    && p.deltas.keys().all(|k| p.stats.contains_key(k))
            })
    }
    pub fn live(&self, now: u64) -> bool {
        self.version == 1
            && !self.stale
            && self.updated > 0
            && now
                .checked_sub(self.updated)
                .is_some_and(|age| age <= MAX_AGE)
    }
    pub fn alerts(&self, now: u64) -> Vec<String> {
        if !self.live(now) {
            return Vec::new();
        }
        let mut alerts = Vec::new();
        for p in &self.ports {
            if p.status
                .as_deref()
                .is_some_and(|s| s.eq_ignore_ascii_case("down"))
            {
                alerts.push(format!("[WARN] NIC port {} link DOWN", p.id));
            }
            for k in ["ierrors", "oerrors", "imissed", "rx_nombuf"] {
                if let Some(n) = p.deltas.get(k).filter(|n| **n > 0) {
                    alerts.push(format!(
                        "[WARN] NIC port {} {k}: +{n} in observed interval",
                        p.id
                    ));
                }
            }
        }
        alerts
    }
    pub fn measurements(&self, now: u64, detail: bool) -> Vec<String> {
        let live = self.live(now);
        let state = if live {
            "LIVE"
        } else if self.updated > 0 {
            "STALE"
        } else {
            "UNAVAILABLE"
        };
        let age = now
            .checked_sub(self.updated)
            .filter(|_| self.updated > 0)
            .map_or("?".into(), |v| format!("{v}s"));
        let mut lines = vec![format!(
            "NIC diagnostics {state} | age {age} | whole bridge; port RX/TX"
        )];
        if let Some(reason) = &self.reason {
            lines.push(format!("NIC note: {reason}"));
        }
        if let Some(version) = &self.dpdk_version {
            lines.push(format!(
                "{version} | {} ports{}",
                self.ports.len(),
                if self.ports_capped {
                    " (capped at 8)"
                } else {
                    ""
                }
            ));
        }
        for p in &self.ports {
            let value = |v: Option<u64>| {
                if live {
                    v.map_or("unknown".into(), |v| v.to_string())
                } else {
                    "unknown".into()
                }
            };
            lines.push(format!(
                "Port {} {} | {} | {} Mbps | {}",
                p.id,
                p.name.as_deref().unwrap_or("unknown"),
                if live {
                    p.status.as_deref().unwrap_or("unknown")
                } else {
                    "unknown"
                },
                value(p.speed_mbps.map(u64::from)),
                if live {
                    p.duplex.as_deref().unwrap_or("unknown")
                } else {
                    "unknown"
                }
            ));
            lines.push(format!(
                "RX errors {} | TX errors {} | missed {} | no mbuf {}",
                value(p.stats.get("ierrors").copied()),
                value(p.stats.get("oerrors").copied()),
                value(p.stats.get("imissed").copied()),
                value(p.stats.get("rx_nombuf").copied())
            ));
            lines.push(format!(
                "New/interval: RX errors {} | TX errors {} | missed {} | no mbuf {}",
                value(p.deltas.get("ierrors").copied()),
                value(p.deltas.get("oerrors").copied()),
                value(p.deltas.get("imissed").copied()),
                value(p.deltas.get("rx_nombuf").copied())
            ));
            if p.reset {
                lines.push("NIC counters decreased/reset; interval unknown".into());
            }
            if detail {
                for k in ["ipackets", "opackets", "ibytes", "obytes"] {
                    lines.push(format!(
                        "{k}: {} | new {}",
                        value(p.stats.get(k).copied()),
                        value(p.deltas.get(k).copied())
                    ));
                }
                if p.xstats.is_empty() {
                    lines.push("Extended stats unavailable or empty".into());
                } else {
                    lines.push(format!(
                    "Extended stats: {}{} (reported raw; may overlap basic counters; do not sum)",
                    p.xstats.len(),
                    if p.xstats_capped {
                        "; capped at 256"
                    } else {
                        ""
                    }
                ));
                }
                for (k, v) in &p.xstats {
                    lines.push(format!("{k}: {}", value(Some(*v))));
                }
            }
        }
        if let Some(ms) = self.interval_ms.filter(|_| live) {
            lines.push(format!("NIC interval {ms} ms; local receive times"));
        }
        lines
    }
}
#[derive(Default)]
pub struct Tracker {
    pub report: Report,
    previous: Option<(u32, u64, Vec<Port>)>,
}
impl Tracker {
    pub fn record(&mut self, result: anyhow::Result<Snapshot>, now_ms: u64) {
        let snapshot = match result {
            Ok(s) => s,
            Err(e) => {
                self.previous = None;
                self.report.version = 1;
                self.report.stale = true;
                self.report.interval_ms = None;
                self.report.reason = Some(
                    e.to_string()
                        .chars()
                        .filter(|c| !c.is_control())
                        .take(256)
                        .collect(),
                );
                return;
            }
        };
        let mut ports = snapshot.ports;
        let previous = self.previous.as_ref().filter(|(pid, at, _)| {
            *pid == snapshot.pid
                && now_ms
                    .checked_sub(*at)
                    .is_some_and(|ms| ms > 0 && ms <= MAX_AGE * 1000)
        });
        for p in &mut ports {
            p.deltas.clear();
            p.reset = false;
            if let Some(old) = previous.and_then(|(_, _, rows)| {
                rows.iter()
                    .find(|old| p.name.is_some() && old.id == p.id && old.name == p.name)
            }) {
                p.reset = p
                    .stats
                    .iter()
                    .any(|(key, value)| old.stats.get(key).is_some_and(|before| value < before));
                if !p.reset {
                    for (key, value) in &p.stats {
                        if let Some(delta) =
                            old.stats.get(key).and_then(|old| value.checked_sub(*old))
                        {
                            p.deltas.insert(key.clone(), delta);
                        }
                    }
                }
            }
        }
        let interval_ms = previous.map(|(_, at, _)| now_ms - *at);
        self.previous = Some((snapshot.pid, now_ms, ports.clone()));
        self.report = Report {
            version: 1,
            updated: now_ms / 1000,
            stale: false,
            reason: None,
            dpdk_version: Some(snapshot.version),
            pid: Some(snapshot.pid),
            ports,
            ports_capped: snapshot.ports_capped,
            interval_ms,
        };
    }
}
pub fn read_cache(path: &Path) -> Report {
    use std::io::Read;
    let read = || -> anyhow::Result<Report> {
        let mut bytes = Vec::new();
        std::fs::File::open(path)?
            .take(MAX_BYTES + 1)
            .read_to_end(&mut bytes)?;
        ensure!(bytes.len() as u64 <= MAX_BYTES, "NIC cache too large");
        let r: Report = serde_json::from_slice(&bytes)?;
        ensure!(r.valid(), "invalid NIC cache");
        Ok(r)
    };
    read().unwrap_or_else(|_| Report {
        version: 1,
        stale: true,
        reason: Some("NIC cache unavailable or invalid".into()),
        ..Default::default()
    })
}
fn publish(dir: &Path, report: &Report) -> anyhow::Result<()> {
    use std::os::unix::fs::PermissionsExt;
    ensure!(report.valid(), "invalid NIC report");
    let bytes = serde_json::to_vec(report)?;
    ensure!(bytes.len() as u64 <= MAX_BYTES, "NIC cache too large");
    let tmp = dir.join(format!(".{CACHE}.tmp-{}", std::process::id()));
    std::fs::write(&tmp, bytes)?;
    std::fs::set_permissions(&tmp, std::fs::Permissions::from_mode(0o644))?;
    std::fs::rename(tmp, dir.join(CACHE))?;
    Ok(())
}
fn record_system_health(hub: &crate::system_health::Hub, report: &Report, now: u64, socket: &str) {
    use crate::system_health::{Domain, Observation, State};
    let live = report.live(now);
    let epoch = format!(
        "{}:{}",
        crate::cli::config_digest(socket.as_bytes()),
        report.pid.unwrap_or(0)
    );
    let telemetry = if !live {
        State::Unknown
    } else if report.ports_capped {
        State::Degraded
    } else {
        State::Healthy
    };
    let links = if !live || report.ports.is_empty() {
        State::Unknown
    } else if report.ports.iter().any(|p| {
        p.status
            .as_deref()
            .is_some_and(|s| s.eq_ignore_ascii_case("down"))
            || ["ierrors", "oerrors", "imissed", "rx_nombuf"]
                .iter()
                .any(|k| p.deltas.get(*k).is_some_and(|n| *n > 0))
    }) {
        State::Degraded
    } else if report.ports_capped
        || report.ports.iter().any(|p| {
            !p.status
                .as_deref()
                .is_some_and(|s| s.eq_ignore_ascii_case("up"))
        })
    {
        State::Unknown
    } else {
        State::Healthy
    };
    for (domain, component, state, reason) in [
        (
            Domain::Telemetry,
            "nic",
            telemetry,
            if !live {
                "nic_sample_unavailable"
            } else if report.ports_capped {
                "nic_sample_capped"
            } else {
                "nic_sample_fresh"
            },
        ),
        (
            Domain::Dataplane,
            "nic_links",
            links,
            match links {
                State::Healthy => "nic_links_up",
                State::Degraded => "nic_link_or_interval_error",
                _ => "nic_links_unproven",
            },
        ),
    ] {
        let _ = hub.observe(Observation {
            domain,
            component: component.into(),
            state,
            reason: reason.into(),
            observed_at: report.updated,
            max_age: MAX_AGE,
            source_epoch: epoch.clone(),
            last_success: None,
        });
    }
}
pub(crate) fn start(
    mut changes: tokio::sync::watch::Receiver<crate::worker_util::Settings>,
    hub: crate::system_health::Hub,
) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        let mut current = changes.borrow().clone();
        let mut tracker = Tracker::default();
        let mut warned = false;
        let mut tick = tokio::time::interval(std::time::Duration::from_secs(POLL_SECS));
        tick.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
        loop {
            tokio::select! {
                changed=changes.changed()=>{if changed.is_err(){break;}let next=changes.borrow_and_update().clone();if !current.same_nic(&next){tracker=Tracker::default();tick.reset_immediately();}current=next;}
                _=tick.tick()=>{if !current.v3{continue;}let result=sample(Path::new(&current.nic_socket)).await;if !changes.borrow().same_nic(&current){continue;}
                    let now=std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_millis().try_into().unwrap_or(u64::MAX);
                    tracker.record(result,now);
                    record_system_health(&hub,&tracker.report,now/1000,&current.nic_socket);
                    if let Err(e)=publish(Path::new(&current.state),&tracker.report){if !warned{eprintln!("NIC telemetry cache: {e}");}warned=true;}else{warned=false;}
                }
            }
        }
    })
}
#[cfg(test)]
mod tests;
