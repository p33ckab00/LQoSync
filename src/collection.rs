//! Sanitized subscriber evidence from the existing shaping-cycle reads. No background router polling.
use crate::{
    config::{Config, RouterCfg},
    cycle::RouterResult,
    model::{RosRow, RouterSnapshot},
};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

pub const CACHE: &str = "collection.json";
pub const MAX_BYTES: u64 = 32 * 1024 * 1024;
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum State {
    Online,
    Offline,
    Bound,
    Unbound,
    Disabled,
    Unknown,
    Stale,
}
impl State {
    pub fn label(self) -> &'static str {
        match self {
            Self::Online => "ONLINE",
            Self::Offline => "OFFLINE",
            Self::Bound => "BOUND",
            Self::Unbound => "UNBOUND",
            Self::Disabled => "DISABLED",
            Self::Unknown => "UNKNOWN",
            Self::Stale => "STALE",
        }
    }
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Subscriber {
    pub key: String,
    pub source: String,
    pub name: String,
    pub circuit: String,
    pub ipv4: String,
    pub mac: String,
    pub state: State,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Source {
    pub enabled: bool,
    pub records: usize,
    pub online: Option<usize>,
    pub offline: Option<usize>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Router {
    pub name: String,
    pub state: String,
    pub observed_at: u64,
    pub sources: BTreeMap<String, Source>,
    pub subscribers: Vec<Subscriber>,
}
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Report {
    pub version: u32,
    pub pid: u32,
    #[serde(default)]
    pub cycle_seq: u64,
    pub updated: u64,
    pub fresh_for: u64,
    pub routers: Vec<Router>,
}
impl Report {
    pub fn collect(
        cfg: &Config,
        outcomes: &[(String, RouterResult)],
        previous: Option<&Self>,
        now: u64,
    ) -> Self {
        let mut routers = Vec::new();
        for (name, result) in outcomes {
            let Some(config) = cfg.routers.iter().find(|r| r.name == *name) else {
                continue;
            };
            let state = match result {
                RouterResult::Online(_) => "online",
                RouterResult::Failed => "failed",
                RouterResult::Unreachable => "unreachable",
                RouterResult::Offline => "offline",
            };
            let mut router = match result {
                RouterResult::Online(snapshot) => observe(cfg, config, snapshot, now),
                _ => previous
                    .and_then(|r| r.routers.iter().find(|r| r.name == *name))
                    .cloned()
                    .unwrap_or(Router {
                        name: name.clone(),
                        state: state.into(),
                        observed_at: 0,
                        sources: BTreeMap::new(),
                        subscribers: Vec::new(),
                    }),
            };
            router.state = state.into();
            for (key, enabled) in [
                ("ppp", config.pppoe.enabled),
                ("hotspot", config.hotspot.enabled),
                ("dhcp", config.dhcp.enabled),
            ] {
                router
                    .sources
                    .entry(key.into())
                    .or_insert(Source {
                        enabled,
                        records: 0,
                        online: None,
                        offline: None,
                    })
                    .enabled = enabled;
            }
            routers.push(router);
        }
        Self {
            version: 1,
            pid: std::process::id(),
            cycle_seq: 0,
            updated: now,
            fresh_for: cfg
                .runtime
                .scan_interval_idle
                .max(cfg.runtime.scan_interval_active)
                .max(cfg.runtime.error_retry_interval)
                .saturating_add(cfg.router_access.api_timeout.saturating_mul(7).max(15)),
            routers,
        }
    }
    pub fn fresh(&self, now: u64) -> bool {
        self.updated > 0 && now >= self.updated && now - self.updated <= self.fresh_for
    }
}

fn field<'a>(r: &'a RosRow, k: &str) -> &'a str {
    r.get(k).map_or("", |s| s.trim())
}
fn disabled(r: &RosRow) -> bool {
    ["disabled", "inactive"]
        .iter()
        .any(|k| matches!(field(r, k).to_ascii_lowercase().as_str(), "true" | "yes"))
}
fn mac_key(mac: &str) -> String {
    mac.replace([':', '-'], "").to_ascii_uppercase()
}
fn observe(cfg: &Config, config: &RouterCfg, snapshot: &RouterSnapshot, now: u64) -> Router {
    let mut subscribers = BTreeMap::new();
    if config.pppoe.enabled {
        for (rows, active) in [(&snapshot.ppp_secret, false), (&snapshot.ppp_active, true)] {
            for row in rows {
                let user = field(row, "name");
                if user.is_empty() {
                    continue;
                }
                let key = serde_json::to_string(&("ppp", user)).unwrap();
                let state = if active {
                    State::Online
                } else if disabled(row) {
                    State::Disabled
                } else {
                    State::Offline
                };
                subscribers.insert(
                    key.clone(),
                    Subscriber {
                        key,
                        source: "ppp".into(),
                        name: user.into(),
                        circuit: cfg
                            .circuits
                            .pppoe
                            .replace("{user}", user)
                            .replace("{router}", &config.name),
                        ipv4: field(row, "address").into(),
                        mac: field(row, "caller-id").into(),
                        state,
                    },
                );
            }
        }
    }
    if config.hotspot.enabled {
        let active: std::collections::HashSet<_> = snapshot
            .hotspot_active
            .iter()
            .map(|r| field(r, "user"))
            .collect();
        for row in snapshot.hotspot_user.iter().flatten() {
            let user = field(row, "name");
            if user.is_empty() || active.contains(user) {
                continue;
            }
            let key = serde_json::to_string(&("hotspot_account", user)).unwrap();
            subscribers.insert(
                key.clone(),
                Subscriber {
                    key,
                    source: "hotspot".into(),
                    name: user.into(),
                    circuit: cfg
                        .circuits
                        .hotspot
                        .replace("{key}", user)
                        .replace("{router}", &config.name),
                    ipv4: String::new(),
                    mac: String::new(),
                    state: if disabled(row) {
                        State::Disabled
                    } else {
                        State::Offline
                    },
                },
            );
        }
        for row in &snapshot.hotspot_active {
            let mac = field(row, "mac-address");
            let user = field(row, "user");
            let identity = if config.hotspot.include_mac && !mac.is_empty() {
                mac_key(mac)
            } else {
                user.into()
            };
            if identity.is_empty() {
                continue;
            }
            let key = serde_json::to_string(&("hotspot", &identity)).unwrap();
            subscribers.insert(
                key.clone(),
                Subscriber {
                    key,
                    source: "hotspot".into(),
                    name: if user.is_empty() {
                        identity.clone()
                    } else {
                        user.into()
                    },
                    circuit: cfg
                        .circuits
                        .hotspot
                        .replace("{key}", &identity)
                        .replace("{router}", &config.name),
                    ipv4: field(row, "address").into(),
                    mac: mac.into(),
                    state: State::Online,
                },
            );
        }
    }
    if config.dhcp.enabled {
        for row in &snapshot.dhcp_lease {
            let mac = field(row, "mac-address");
            let ip = match field(row, "active-address") {
                "" => field(row, "address"),
                ip => ip,
            };
            if mac.is_empty() && ip.is_empty() {
                continue;
            }
            let identity =
                serde_json::to_string(&("dhcp", field(row, "server"), mac_key(mac), ip)).unwrap();
            let host = field(row, "host-name");
            let name = if host.is_empty() {
                if mac.is_empty() { ip } else { mac }
            } else {
                host
            };
            let key = if host.is_empty() {
                mac_key(mac)
            } else {
                host.into()
            };
            let state = match field(row, "status") {
                "bound" => State::Bound,
                "waiting" | "testing" | "declined" | "offered" | "authorizing" | "conflict" => {
                    State::Unbound
                }
                _ => State::Unknown,
            };
            subscribers.insert(
                identity.clone(),
                Subscriber {
                    key: identity,
                    source: "dhcp".into(),
                    name: name.into(),
                    circuit: cfg
                        .circuits
                        .dhcp
                        .replace("{key}", &key)
                        .replace("{router}", &config.name),
                    ipv4: ip.into(),
                    mac: mac.into(),
                    state,
                },
            );
        }
    }
    let subscribers: Vec<Subscriber> = subscribers.into_values().collect();
    let sources = [
        ("ppp", config.pppoe.enabled, snapshot.ppp_active.len()),
        (
            "hotspot",
            config.hotspot.enabled,
            snapshot.hotspot_active.len(),
        ),
        ("dhcp", config.dhcp.enabled, snapshot.dhcp_lease.len()),
    ]
    .into_iter()
    .map(|(key, enabled, records)| {
        let count = |state| {
            subscribers
                .iter()
                .filter(|s| s.source == key && s.state == state)
                .count()
        };
        (
            key.into(),
            Source {
                enabled,
                records: if enabled { records } else { 0 },
                online: enabled.then(|| {
                    count(if key == "dhcp" {
                        State::Bound
                    } else {
                        State::Online
                    })
                }),
                offline: (enabled && (key != "hotspot" || snapshot.hotspot_user.is_some())).then(
                    || {
                        count(if key == "dhcp" {
                            State::Unbound
                        } else {
                            State::Offline
                        })
                    },
                ),
            },
        )
    })
    .collect();
    Router {
        name: config.name.clone(),
        state: "online".into(),
        observed_at: now,
        sources,
        subscribers,
    }
}

pub fn publish(directory: &std::path::Path, report: &Report) -> anyhow::Result<()> {
    use std::os::unix::fs::PermissionsExt;
    let bytes = serde_json::to_vec(report)?;
    anyhow::ensure!(
        bytes.len() as u64 <= MAX_BYTES,
        "collection cache exceeds 32 MiB"
    );
    let temp = directory.join(format!(".{CACHE}.tmp-{}", std::process::id()));
    std::fs::write(&temp, bytes)?;
    std::fs::set_permissions(&temp, std::fs::Permissions::from_mode(0o644))?;
    std::fs::rename(temp, directory.join(CACHE))?;
    Ok(())
}
pub fn read(path: &std::path::Path) -> anyhow::Result<Report> {
    use std::io::Read;
    let mut bytes = Vec::new();
    std::fs::File::open(path)?
        .take(MAX_BYTES + 1)
        .read_to_end(&mut bytes)?;
    anyhow::ensure!(
        bytes.len() as u64 <= MAX_BYTES,
        "collection cache exceeds 32 MiB"
    );
    let report: Report = serde_json::from_slice(&bytes)?;
    anyhow::ensure!(
        report.version == 1 && report.updated > 0,
        "invalid collection cache version/time"
    );
    Ok(report)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn config() -> Config {
        Config::from_toml_str("[[routers]]\nname='R1'\naddress='192.0.2.1'\nusername='u'\npassword_file='/fixture/secret'\n[routers.pppoe]\nenabled=true\n[routers.hotspot]\nenabled=true\n[routers.dhcp]\nenabled=true\n").unwrap()
    }
    fn snapshot() -> RouterSnapshot {
        serde_json::from_value(serde_json::json!({
        "ppp_secret":[{"name":"alice","password":"NEVER-PUBLISH"},{"name":"bob"},{"name":"disabled","disabled":"true"}],
        "ppp_active":[{"name":"alice","address":"192.0.2.10"},{"name":"radius","address":"192.0.2.11"}],
        "hotspot_active":[{"user":"guest","address":"192.0.2.12","mac-address":"00:11:22:33:44:55"}],
        "hotspot_user":[{"name":"guest"},{"name":"hotspot-offline","password":"NEVER-PUBLISH"}],
        "dhcp_lease":[{"server":"LAN","host-name":"bound","address":"192.0.2.13","status":"bound","mac-address":"00:11:22:33:44:66"},{"server":"LAN","host-name":"waiting","status":"waiting","mac-address":"00:11:22:33:44:77"},{"host-name":"missing-status","mac-address":"00:11:22:33:44:88"}]
    })).unwrap()
    }
    #[test]
    fn collection_uses_raw_inventory_sessions_and_lease_state_without_secrets() {
        let r = Report::collect(
            &config(),
            &[("R1".into(), RouterResult::Online(snapshot()))],
            None,
            100,
        );
        let router = &r.routers[0];
        assert_eq!(
            (router.sources["ppp"].online, router.sources["ppp"].offline),
            (Some(2), Some(1))
        );
        assert_eq!(router.sources["hotspot"].offline, Some(1));
        assert_eq!(
            router
                .subscribers
                .iter()
                .find(|s| s.name == "hotspot-offline")
                .unwrap()
                .state,
            State::Offline
        );
        assert_eq!(router.sources["dhcp"].online, Some(1));
        for (name, state) in [
            ("alice", State::Online),
            ("bob", State::Offline),
            ("disabled", State::Disabled),
            ("radius", State::Online),
            ("waiting", State::Unbound),
            ("missing-status", State::Unknown),
        ] {
            assert_eq!(
                router
                    .subscribers
                    .iter()
                    .find(|s| s.name == name)
                    .unwrap()
                    .state,
                state
            );
        }
        assert!(!serde_json::to_string(&r).unwrap().contains("NEVER-PUBLISH"));
    }
    #[test]
    fn cache_round_trip_is_public_sanitized_bounded_and_versioned() {
        use std::os::unix::fs::PermissionsExt;
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join(CACHE);
        let r = Report::collect(
            &config(),
            &[("R1".into(), RouterResult::Online(snapshot()))],
            None,
            100,
        );
        publish(dir.path(), &r).unwrap();
        assert_eq!(read(&path).unwrap().routers[0].sources["dhcp"].records, 3);
        assert_eq!(
            std::fs::metadata(&path).unwrap().permissions().mode() & 0o777,
            0o644
        );
        std::fs::write(
            &path,
            r#"{"version":9,"pid":1,"updated":100,"fresh_for":120,"routers":[]}"#,
        )
        .unwrap();
        assert!(read(&path).is_err());
        std::fs::File::create(&path)
            .unwrap()
            .set_len(MAX_BYTES + 1)
            .unwrap();
        assert!(read(&path).is_err());
    }

    #[test]
    fn failed_reads_retain_evidence_but_never_turn_subscribers_offline() {
        let cfg = config();
        let old = Report::collect(
            &cfg,
            &[("R1".into(), RouterResult::Online(snapshot()))],
            None,
            100,
        );
        let next = Report::collect(
            &cfg,
            &[("R1".into(), RouterResult::Failed)],
            Some(&old),
            110,
        );
        assert_eq!(next.routers[0].observed_at, 100);
        assert_eq!(next.routers[0].state, "failed");
        assert_eq!(
            next.routers[0].subscribers[0].state,
            old.routers[0].subscribers[0].state
        );
        let mut cfg = cfg;
        cfg.routers[0].hotspot.enabled = false;
        let disabled = Report::collect(
            &cfg,
            &[("R1".into(), RouterResult::Online(snapshot()))],
            Some(&old),
            120,
        );
        assert!(!disabled.routers[0].sources["hotspot"].enabled);
        assert!(
            !disabled.routers[0]
                .subscribers
                .iter()
                .any(|s| s.source == "hotspot")
        );
    }
}
