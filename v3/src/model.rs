use anyhow::{Result, bail, ensure};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashSet};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Settings {
    #[serde(default = "default_interval")]
    pub interval_seconds: u64,
    #[serde(default = "yes")]
    pub auto_stage: bool,
    #[serde(default)]
    pub routers: Vec<Router>,
    #[serde(default)]
    pub nodes: Vec<Node>,
}
fn default_interval() -> u64 {
    30
}
fn yes() -> bool {
    true
}
impl Default for Settings {
    fn default() -> Self {
        Self {
            interval_seconds: 30,
            auto_stage: true,
            routers: vec![],
            nodes: vec![],
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Router {
    pub id: String,
    pub name: String,
    pub host: String,
    pub port: u16,
    pub username: String,
    pub password_env: String,
    pub role: String,
    pub tls: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Node {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub parent: String,
    pub kind: String,
    #[serde(default)]
    pub router: String,
    #[serde(default)]
    pub source: String,
    pub mode: String,
    pub download: f64,
    pub upload: f64,
    pub download_factor: f64,
    pub upload_factor: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Subscriber {
    pub id: String,
    pub name: String,
    pub ipv4: String,
    pub node: String,
    pub download: f64,
    pub upload: f64,
    pub download_min: f64,
    pub upload_min: f64,
}

#[derive(Debug, Clone)]
pub struct Capacity {
    pub download: f64,
    pub upload: f64,
    pub raw_download: f64,
    pub raw_upload: f64,
    pub subscribers: usize,
}

pub struct Plan {
    pub capacities: std::collections::BTreeMap<String, Capacity>,
    pub csv: String,
    pub network: String,
}

pub fn valid_id(id: &str) -> bool {
    !id.is_empty()
        && id.len() <= 80
        && id
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
}

pub fn validate(settings: &Settings) -> Result<()> {
    ensure!(
        (1..=86400).contains(&settings.interval_seconds),
        "Sync interval must be 1–86400 seconds"
    );
    let mut router_ids = HashSet::new();
    for r in &settings.routers {
        ensure!(
            valid_id(&r.id) && router_ids.insert(&r.id),
            "Router IDs must be unique letters, digits, underscores or hyphens"
        );
        ensure!(
            !r.name.trim().is_empty() && !r.host.trim().is_empty() && r.port > 0,
            "Router name, host and API port are required"
        );
        ensure!(
            matches!(r.role.as_str(), "core" | "distro" | "both"),
            "Unknown router role"
        );
        ensure!(
            valid_id(&r.password_env) && !r.username.trim().is_empty(),
            "Username and password environment reference are required"
        );
    }
    let mut ids = HashSet::new();
    for n in &settings.nodes {
        ensure!(
            valid_id(&n.id) && ids.insert(n.id.as_str()),
            "Node IDs must be unique letters, digits, underscores or hyphens"
        );
        ensure!(!n.name.trim().is_empty(), "Node name is required");
        ensure!(
            matches!(
                n.kind.as_str(),
                "wan" | "pppoe" | "dhcp" | "hotspot" | "site"
            ),
            "Unknown node kind"
        );
        ensure!(
            matches!(n.mode.as_str(), "manual" | "computed"),
            "Unknown capacity mode"
        );
        ensure!(
            n.router.is_empty() || router_ids.contains(&n.router),
            "Node {} refers to an unknown router",
            n.id
        );
        ensure!(
            [n.download, n.upload, n.download_factor, n.upload_factor]
                .iter()
                .all(|v| v.is_finite() && *v >= 0.0),
            "Capacity values must be finite and nonnegative"
        );
        if n.parent.is_empty() {
            ensure!(
                n.kind == "wan" && n.mode == "manual",
                "Each root must be an explicit WAN"
            );
        } else {
            ensure!(n.kind != "wan", "A WAN must be a root");
        }
        if n.mode == "manual" {
            ensure!(
                n.download > 0.0 && n.upload > 0.0,
                "Explicit capacities must be positive"
            );
        }
        if n.mode == "computed" {
            ensure!(
                n.download_factor > 0.0 && n.upload_factor > 0.0,
                "Computed factors must be positive"
            );
        }
    }
    for n in &settings.nodes {
        let mut seen = HashSet::new();
        let mut id = n.id.as_str();
        while !id.is_empty() {
            ensure!(seen.insert(id), "Topology contains a cycle at {id}");
            let parent = settings
                .nodes
                .iter()
                .find(|n| n.id == id)
                .ok_or_else(|| anyhow::anyhow!("Unknown parent {id}"))?;
            id = &parent.parent;
        }
    }
    Ok(())
}

fn beneath(settings: &Settings, id: &str, ancestor: &str) -> bool {
    if id == ancestor {
        return true;
    }
    let Some(node) = settings.nodes.iter().find(|n| n.id == id) else {
        return false;
    };
    !node.parent.is_empty() && beneath(settings, &node.parent, ancestor)
}

pub fn compile(settings: &Settings, subscribers: &[Subscriber]) -> Result<Plan> {
    validate(settings)?;
    let mut subscriber_ids = HashSet::new();
    let mut ips = HashSet::new();
    for s in subscribers {
        ensure!(
            !s.id.is_empty() && subscriber_ids.insert(&s.id),
            "Subscriber IDs must be unique"
        );
        let ip = s
            .ipv4
            .strip_suffix("/32")
            .unwrap_or(&s.ipv4)
            .parse::<std::net::Ipv4Addr>()?;
        ensure!(ips.insert(ip), "Duplicate subscriber IPv4 address");
        ensure!(
            settings.nodes.iter().any(|n| n.id == s.node),
            "Subscriber {} has an unknown node",
            s.id
        );
        ensure!(
            [s.download, s.upload, s.download_min, s.upload_min]
                .iter()
                .all(|v| v.is_finite() && *v >= 0.0)
                && s.download > 0.0
                && s.upload > 0.0
                && s.download_min <= s.download
                && s.upload_min <= s.upload,
            "Subscriber {} has invalid rates",
            s.id
        );
    }
    let mut capacities: BTreeMap<String, Capacity> = BTreeMap::new();
    let mut remaining: Vec<_> = settings.nodes.iter().collect();
    while !remaining.is_empty() {
        let before = remaining.len();
        let mut next = Vec::new();
        for n in remaining {
            if !n.parent.is_empty() && !capacities.contains_key(&n.parent) {
                next.push(n);
                continue;
            }
            let members: Vec<_> = subscribers
                .iter()
                .filter(|s| beneath(settings, &s.node, &n.id))
                .collect();
            let (raw_download, raw_upload) = if n.mode == "computed" {
                (
                    members.iter().map(|s| s.download).sum::<f64>() * n.download_factor,
                    members.iter().map(|s| s.upload).sum::<f64>() * n.upload_factor,
                )
            } else {
                (n.download, n.upload)
            };
            ensure!(
                raw_download.is_finite()
                    && raw_upload.is_finite()
                    && raw_download > 0.0
                    && raw_upload > 0.0,
                "Node {} needs a complete subscriber snapshot before publication",
                n.id
            );
            let (download, upload) = if let Some(p) = capacities.get(&n.parent) {
                if n.mode == "manual" {
                    ensure!(
                        raw_download <= p.download && raw_upload <= p.upload,
                        "Explicit node {} exceeds its parent capacity",
                        n.id
                    );
                }
                (raw_download.min(p.download), raw_upload.min(p.upload))
            } else {
                (raw_download, raw_upload)
            };
            capacities.insert(
                n.id.clone(),
                Capacity {
                    download,
                    upload,
                    raw_download,
                    raw_upload,
                    subscribers: members.len(),
                },
            );
        }
        if next.len() == before {
            bail!("Topology cannot be resolved");
        }
        remaining = next;
    }
    fn tree(
        settings: &Settings,
        caps: &BTreeMap<String, Capacity>,
        parent: &str,
    ) -> serde_json::Value {
        let mut out = serde_json::Map::new();
        for n in settings.nodes.iter().filter(|n| n.parent == parent) {
            let c = &caps[&n.id];
            out.insert(n.id.clone(),serde_json::json!({"downloadBandwidthMbps":c.download,"uploadBandwidthMbps":c.upload,"type":"site","children":tree(settings,caps,&n.id)}));
        }
        serde_json::Value::Object(out)
    }
    let mut writer = csv::Writer::from_writer(Vec::new());
    writer.write_record([
        "Circuit ID",
        "Circuit Name",
        "Device ID",
        "Device Name",
        "Parent Node",
        "MAC",
        "IPv4",
        "IPv6",
        "Download Min Mbps",
        "Upload Min Mbps",
        "Download Max Mbps",
        "Upload Max Mbps",
        "Comment",
    ])?;
    let mut ordered: Vec<_> = subscribers.iter().collect();
    ordered.sort_by(|a, b| a.id.cmp(&b.id));
    for s in ordered {
        writer.write_record([
            s.id.clone(),
            s.name.clone(),
            s.id.clone(),
            s.name.clone(),
            s.node.clone(),
            String::new(),
            s.ipv4.clone(),
            String::new(),
            s.download_min.to_string(),
            s.upload_min.to_string(),
            s.download.to_string(),
            s.upload.to_string(),
            "LQoSync3".into(),
        ])?;
    }
    let csv = String::from_utf8(writer.into_inner()?)?;
    let network = serde_json::to_string_pretty(&tree(settings, &capacities, ""))?;
    Ok(Plan {
        capacities,
        csv,
        network,
    })
}
