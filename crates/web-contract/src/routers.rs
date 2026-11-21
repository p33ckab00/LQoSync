//! Router inventory for the web. The service writes it as `routers.json` at the end of each cycle.
//! It holds no secret value and no file path: only "configured" flags for the credential and the CA.
use serde::{Deserialize, Serialize};

pub const CACHE: &str = "routers.json";
pub const VERSION: u32 = 1;

fn enabled() -> bool {
    true
}

#[derive(Clone, Default, Debug, PartialEq, Serialize, Deserialize)]
pub struct RouterInventory {
    pub version: u32,
    /// Unix seconds when the service wrote the inventory.
    pub updated: u64,
    /// Seconds after `updated` when the inventory is stale.
    pub fresh_for: u64,
    pub routers: Vec<RouterSummary>,
    /// The gateway dropped routers to keep the reply bounded.
    #[serde(default)]
    pub truncated: bool,
}

#[derive(Clone, Default, Debug, PartialEq, Serialize, Deserialize)]
pub struct RouterSummary {
    /// Stable ID, the same as the topology registry router ID.
    pub id: String,
    pub name: String,
    /// `false`: LQoSync makes no connection to this router. Absent in older caches means enabled.
    #[serde(default = "enabled")]
    pub enabled: bool,
    pub address: String,
    pub port: u16,
    pub tls: bool,
    pub tls_verify: bool,
    /// A CA file is configured for the certificate check.
    pub tls_ca: bool,
    pub username: String,
    /// The credential file exists and is not empty.
    pub credential: bool,
    /// `wan`, `ppp`, `dhcp`, `hotspot`. WAN comes from the topology catalog, the others from the config.
    pub roles: Vec<String>,
    /// Catalog WAN endpoints of this router.
    pub wans: Vec<String>,
    /// The latest subscriber read. `None` when the cycle does not read this router (no subscriber role).
    pub collection: Option<RouterCollection>,
    pub sources: Vec<RouterSource>,
    /// Latitude and longitude for the LibreQoS Site Map.
    pub location: Option<[f64; 2]>,
    pub options: RouterOptions,
}

#[derive(Clone, Default, Debug, PartialEq, Serialize, Deserialize)]
pub struct RouterCollection {
    /// `online`, `failed`, `unreachable`, or `offline`.
    pub state: String,
    /// Unix seconds of the last successful read. 0 when no read succeeded.
    pub observed_at: u64,
    pub subscribers: u64,
}

#[derive(Clone, Default, Debug, PartialEq, Serialize, Deserialize)]
pub struct RouterSource {
    /// `ppp`, `dhcp`, or `hotspot`.
    pub kind: String,
    pub enabled: bool,
    pub records: Option<u64>,
    pub online: Option<u64>,
    pub offline: Option<u64>,
}

#[derive(Clone, Default, Debug, PartialEq, Serialize, Deserialize)]
pub struct RouterOptions {
    pub ppp_per_plan_node: bool,
    pub hotspot_include_mac: bool,
    pub dhcp_require_bound: bool,
    pub dhcp_discover: bool,
    pub dhcp_shape_unassigned: bool,
    pub dhcp_servers: u64,
    pub dhcp_clients: u64,
    /// Router-scoped exclusions: DHCP servers, PPP profiles, and IPs together.
    pub exclusions: u64,
}
