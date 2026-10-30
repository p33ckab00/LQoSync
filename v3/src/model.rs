use anyhow::{Result, bail, ensure};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap, HashSet};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Settings {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub dynamic: Option<crate::dynamic::Config>,
    #[serde(default = "default_interval")]
    pub interval_seconds: u64,
    #[serde(default = "yes")]
    pub auto_stage: bool,
    #[serde(default)]
    pub speed_rules: crate::speeds::SpeedRules,
    #[serde(default)]
    pub assignment: crate::assignment::Policy,
    #[serde(default)]
    pub core_lists: crate::core_plan::Policy,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub selections: Vec<crate::selection::Selection>,
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
            dynamic: None,
            interval_seconds: 30,
            auto_stage: true,
            speed_rules: crate::speeds::SpeedRules::default(),
            assignment: Default::default(),
            core_lists: Default::default(),
            selections: vec![],
            routers: vec![],
            nodes: vec![],
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
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
    #[serde(default)]
    pub tls_server_name: String,
    #[serde(default)]
    pub tls_ca_file: String,
    #[serde(default = "default_router_timeout")]
    pub timeout_seconds: u64,
}
fn default_router_timeout() -> u64 {
    10
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
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

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
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
    settings.speed_rules.validate()?;
    crate::normalizer::validate_bindings(settings)?;
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
            (r.password_env.is_empty() || valid_id(&r.password_env))
                && !r.username.trim().is_empty(),
            "Username is required; password environment reference must be empty or a valid identifier"
        );
        ensure!(
            (1..=30).contains(&r.timeout_seconds),
            "Router timeout must be 1–30 seconds"
        );
    }
    for rule in &settings.speed_rules.dhcp_servers {
        ensure!(
            settings
                .routers
                .iter()
                .any(|r| r.id == rule.router && r.role != "core"),
            "DHCP speed rule refers to an unknown subscriber router"
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
                "wan" | "pppoe" | "dhcp" | "hotspot" | "site" | "group"
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
        if settings.dynamic.is_some() {
            // Exported roots are independent of routing WAN identities.
        } else if n.parent.is_empty() {
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
    let node_index: HashMap<_, _> = settings.nodes.iter().map(|n| (n.id.as_str(), n)).collect();
    for n in &settings.nodes {
        let mut seen = HashSet::new();
        let mut id = n.id.as_str();
        while !id.is_empty() {
            ensure!(seen.insert(id), "Topology contains a cycle at {id}");
            ensure!(seen.len() <= 256, "Topology depth exceeds 256 nodes");
            let parent = node_index
                .get(id)
                .ok_or_else(|| anyhow::anyhow!("Unknown parent {id}"))?;
            id = &parent.parent;
        }
    }
    crate::assignment::validate(settings)?;
    crate::core_plan::validate(settings)?;
    Ok(())
}

pub fn compile(settings: &Settings, subscribers: &[Subscriber]) -> Result<Plan> {
    let plan = compile_unallocated(settings, subscribers)?;
    crate::minimums::validate(settings, subscribers, &plan)?;
    Ok(plan)
}
pub(crate) fn compile_unallocated(settings: &Settings, subscribers: &[Subscriber]) -> Result<Plan> {
    validate(settings)?;
    ensure!(
        !subscribers.is_empty() || !settings.nodes.iter().any(|n| n.mode == "computed"),
        "Waiting for a complete nonempty subscriber snapshot"
    );
    let nodes: HashMap<_, _> = settings.nodes.iter().map(|n| (n.id.as_str(), n)).collect();
    let mut totals: HashMap<&str, (f64, f64, usize)> = HashMap::new();
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
            nodes.contains_key(s.node.as_str()),
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
    for subscriber in subscribers {
        let mut id = subscriber.node.as_str();
        while !id.is_empty() {
            let total = totals.entry(id).or_default();
            total.0 += subscriber.download;
            total.1 += subscriber.upload;
            total.2 += 1;
            id = &nodes[id].parent;
        }
    }
    let mut pruned = HashSet::new();
    let mut capacities: BTreeMap<String, Capacity> = BTreeMap::new();
    let mut remaining: Vec<_> = settings.nodes.iter().collect();
    while !remaining.is_empty() {
        let before = remaining.len();
        let mut next = Vec::new();
        for n in remaining {
            let (sum_download, sum_upload, count) =
                totals.get(n.id.as_str()).copied().unwrap_or_default();
            if pruned.contains(n.parent.as_str()) || (n.mode == "computed" && count == 0) {
                pruned.insert(n.id.as_str());
                continue;
            }
            if !n.parent.is_empty() && !capacities.contains_key(&n.parent) {
                next.push(n);
                continue;
            }
            let (raw_download, raw_upload) = if n.mode == "computed" {
                (
                    sum_download * n.download_factor,
                    sum_upload * n.upload_factor,
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
                    subscribers: count,
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
            let Some(c) = caps.get(&n.id) else {
                continue;
            };
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

pub fn compile_assigned(
    settings: &Settings,
    subscribers: &[Subscriber],
    assignments: &crate::assignment::AssignmentState,
) -> Result<Plan> {
    let plan = compile(settings, subscribers)?;
    crate::minimums::validate_assigned(settings, subscribers, assignments)?;
    Ok(plan)
}
