//! Data types of the circuit model and the state file.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// One row of `ShapedDevices.csv`. All values stay as text.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct CsvRow {
    #[serde(rename = "Circuit ID", default)]
    pub circuit_id: String,
    #[serde(rename = "Circuit Name", default)]
    pub circuit_name: String,
    #[serde(rename = "Device ID", default)]
    pub device_id: String,
    #[serde(rename = "Device Name", default)]
    pub device_name: String,
    #[serde(rename = "Parent Node", default)]
    pub parent_node: String,
    #[serde(rename = "Parent Node ID", default)]
    pub parent_node_id: String,
    #[serde(rename = "Anchor Node ID", default)]
    pub anchor_node_id: String,
    #[serde(rename = "MAC", default)]
    pub mac: String,
    #[serde(rename = "IPv4", default)]
    pub ipv4: String,
    #[serde(rename = "IPv6", default)]
    pub ipv6: String,
    #[serde(rename = "Download Min Mbps", default)]
    pub download_min: String,
    #[serde(rename = "Upload Min Mbps", default)]
    pub upload_min: String,
    #[serde(rename = "Download Max Mbps", default)]
    pub download_max: String,
    #[serde(rename = "Upload Max Mbps", default)]
    pub upload_max: String,
    #[serde(rename = "Comment", default)]
    pub comment: String,
}

/// The column set of the CSV file that a target writes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Columns {
    /// LibreQoS 2.x: 14 columns, with `Parent Node ID`.
    V2,
    /// LibreQoS 3.x: 15 columns, with `Parent Node ID` and `Anchor Node ID`.
    V3,
}

const V3_COLUMNS: [&str; 15] = [
    "Circuit ID",
    "Circuit Name",
    "Device ID",
    "Device Name",
    "Parent Node",
    "Parent Node ID",
    "Anchor Node ID",
    "MAC",
    "IPv4",
    "IPv6",
    "Download Min Mbps",
    "Upload Min Mbps",
    "Download Max Mbps",
    "Upload Max Mbps",
    "Comment",
];

const V2_COLUMNS: [&str; 14] = [
    "Circuit ID",
    "Circuit Name",
    "Device ID",
    "Device Name",
    "Parent Node",
    "Parent Node ID",
    "MAC",
    "IPv4",
    "IPv6",
    "Download Min Mbps",
    "Upload Min Mbps",
    "Download Max Mbps",
    "Upload Max Mbps",
    "Comment",
];

impl Columns {
    pub fn names(self) -> &'static [&'static str] {
        match self {
            Columns::V2 => &V2_COLUMNS,
            Columns::V3 => &V3_COLUMNS,
        }
    }
}

/// Where a circuit comes from.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Source {
    Ppp,
    Hotspot,
    Dhcp,
    Unknown,
}

/// One RouterOS reply row: field name to value.
pub type RosRow = BTreeMap<String, String>;

/// The raw RouterOS rows of one router, in reply order.
#[derive(Debug, Clone, Default, Deserialize)]
pub struct RouterSnapshot {
    #[serde(default)]
    pub ppp_secret: Vec<RosRow>,
    #[serde(default)]
    pub ppp_active: Vec<RosRow>,
    #[serde(default)]
    pub ppp_profile: Vec<RosRow>,
    #[serde(default)]
    pub hotspot_active: Vec<RosRow>,
    #[serde(default)]
    pub dhcp_server: Vec<RosRow>,
    #[serde(default)]
    pub dhcp_lease: Vec<RosRow>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StoredCircuit {
    pub router: Option<String>,
    pub source: Source,
    pub last_seen: u64,
    pub row: CsvRow,
}

/// A node that the tool made.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StoredNode {
    pub id: String,
    pub router: Option<String>,
    pub parent: Option<String>,
    #[serde(rename = "type")]
    pub r#type: String,
    pub down: f64,
    pub up: f64,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct RouterMemo {
    pub unreachable_since: Option<u64>,
}

/// A removal that the guard holds until the next cycle.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct GuardHold {
    /// FNV-1a 64 of the removed names in name order, joined with a newline.
    pub fingerprint: u64,
    pub since: u64,
    pub count: usize,
}

/// The contents of `state.json`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct State {
    pub version: u32,
    pub circuits: BTreeMap<String, StoredCircuit>,
    pub nodes: BTreeMap<String, StoredNode>,
    pub routers: BTreeMap<String, RouterMemo>,
    #[serde(default)]
    pub guard: Option<GuardHold>,
}

impl Default for State {
    fn default() -> Self {
        State {
            version: 1,
            circuits: BTreeMap::new(),
            nodes: BTreeMap::new(),
            routers: BTreeMap::new(),
            guard: None,
        }
    }
}
