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
    /// The digest of the config that the service has loaded. An edit sends it back as its expected digest.
    #[serde(default)]
    pub config_digest: String,
    /// The removal rules that decide what a disable does to the shaping.
    #[serde(default)]
    pub shaping: ShapingPolicy,
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
    /// Circuits of this router in the LQoSync shaping list at the start of the last cycle.
    #[serde(default)]
    pub circuits: Option<u64>,
    /// The router lists. Only the one-router detail carries them; the inventory list leaves them out.
    #[serde(default)]
    pub lists: Option<RouterLists>,
}

/// The longest list that one edit can set. The core checks it again.
pub const MAX_LIST: usize = 200;
pub const MAX_CLIENTS: usize = 500;

#[derive(Clone, Default, Debug, PartialEq, Serialize, Deserialize)]
pub struct RouterLists {
    /// Router-scoped exclusions; the service joins them with the global `[exclude]`.
    pub exclude_dhcp_servers: Vec<String>,
    pub exclude_ppp_profiles: Vec<String>,
    /// IPv4 addresses or networks (`a.b.c.d` or `a.b.c.d/n`).
    pub exclude_ips: Vec<String>,
    pub dhcp_clients: Vec<DhcpClientOverride>,
    /// The mode of a DHCP server without its own settings: `per_site` or `per_plan`.
    #[serde(default)]
    pub dhcp_server_mode: String,
    #[serde(default)]
    pub dhcp_servers: Vec<DhcpServerSetting>,
    /// The gateway dropped client overrides to keep the reply bounded. The page then refuses to
    /// edit the client list, because a save would remove the entries that it did not get.
    #[serde(default)]
    pub truncated: bool,
}

pub const MAX_SERVERS: usize = 100;

/// The settings of one DHCP server of a router. An absent value uses the default.
#[derive(Clone, Default, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DhcpServerSetting {
    pub name: String,
    /// `per_site` or `per_plan`.
    pub mode: String,
    /// In order: `server_comment`, `lease_comment`, `settings`. Absent: the list of the mode in `[speed]`.
    pub speed_sources: Option<Vec<String>>,
    pub default_plan_down_mbps: Option<f64>,
    pub default_plan_up_mbps: Option<f64>,
    pub download_factor: Option<f64>,
    pub upload_factor: Option<f64>,
}

/// A speed override for one DHCP client, found by its MAC address.
#[derive(Clone, Default, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DhcpClientOverride {
    pub mac: String,
    pub download_mbps: f64,
    pub upload_mbps: f64,
    pub name: Option<String>,
}

/// The removal rules of the legacy writer, from the config and the last cycle.
#[derive(Clone, Default, Debug, PartialEq, Serialize, Deserialize)]
pub struct ShapingPolicy {
    /// Circuits in the LQoSync shaping list at the start of the last cycle.
    pub total_circuits: u64,
    /// `runtime.subscriber_grace`: a circuit that is not read leaves after this time.
    pub grace_secs: u64,
    /// `runtime.remove_inactive_entries`. False: the circuits of a router that is not read stay.
    pub remove_inactive: bool,
    pub guard_enabled: bool,
    pub guard_max_count: u64,
    pub guard_max_percent: f64,
    /// The circuit count of a removal that the guard holds now.
    pub guard_held: Option<u64>,
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

/// Limits of a router edit. The core checks them again.
pub const MAX_PASSWORD: usize = 256;
pub const MAX_CA_PEM: usize = 16 * 1024;

/// A local router change from the web. Only the fields that are present change. It makes no router I/O.
/// A new router is saved disabled. The name is the router identity: an update cannot change it.
#[derive(Clone, Default, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RouterEdit {
    /// `RouterInventory::config_digest` when the form loaded. Another change since then is a conflict.
    pub expected_digest: String,
    /// Only for a new router.
    pub name: Option<String>,
    pub address: Option<String>,
    pub port: Option<u16>,
    pub tls: Option<bool>,
    pub tls_verify: Option<bool>,
    pub username: Option<String>,
    /// A new credential. Absent keeps the current one. It is never sent back.
    pub password: Option<String>,
    /// A new CA certificate in PEM text, for the certificate check.
    pub ca_pem: Option<String>,
    /// Remove the CA setting: the check then uses the public CAs.
    #[serde(default)]
    pub clear_ca: bool,
    pub pppoe: Option<bool>,
    pub dhcp: Option<bool>,
    pub hotspot: Option<bool>,
    pub ppp_per_plan_node: Option<bool>,
    pub hotspot_include_mac: Option<bool>,
    pub dhcp_require_bound: Option<bool>,
    pub dhcp_discover: Option<bool>,
    pub dhcp_shape_unassigned: Option<bool>,
    /// Latitude and longitude for the LibreQoS Site Map.
    pub location: Option<[f64; 2]>,
    #[serde(default)]
    pub clear_location: bool,
    /// Disable (`false`) or enable (`true`) an existing router. A disable needs the operator password.
    pub enabled: Option<bool>,
    /// A list that is present replaces the whole list. An empty list removes it.
    pub exclude_dhcp_servers: Option<Vec<String>>,
    pub exclude_ppp_profiles: Option<Vec<String>>,
    pub exclude_ips: Option<Vec<String>>,
    pub dhcp_clients: Option<Vec<DhcpClientOverride>>,
    pub dhcp_server_mode: Option<String>,
    pub dhcp_servers: Option<Vec<DhcpServerSetting>>,
    /// A present list replaces all access profiles, at most 8. An absent password keeps the
    /// saved secret. Older forms omit the field.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub access_profiles: Option<Vec<crate::source_studio::AccessProfileEdit>>,
}

/// What the browser sends: the edit, and the operator password when the edit replaces a credential.
#[derive(Clone, Default, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RouterEditForm {
    pub edit: RouterEdit,
    pub current_password: Option<String>,
}

/// The broker gives the core the signed-in account as the actor. `router_id` is absent for a new router.
#[derive(Clone, Default, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RouterEditRequest {
    pub actor: String,
    pub router_id: Option<String>,
    pub edit: RouterEdit,
}

#[derive(Clone, Default, Debug, PartialEq, Serialize, Deserialize)]
pub struct RouterEditReply {
    pub router_id: String,
    /// The digest of the saved config file. The service loaded it when the inventory shows the same digest.
    pub digest: String,
    /// `sent`, or why the reload signal did not go to the service.
    pub reload: String,
    pub warnings: Vec<String>,
}

/// The core answer: `code` is `ok`, `conflict`, `invalid`, `not_found`, or `internal`.
#[derive(Clone, Default, Debug, PartialEq, Serialize, Deserialize)]
pub struct RouterEditOutcome {
    pub code: String,
    pub message: String,
    pub reply: Option<RouterEditReply>,
}

/// One router action in the audit log. It holds field names, never values of secrets.
#[derive(Clone, Default, Debug, PartialEq, Serialize, Deserialize)]
pub struct RouterEvent {
    pub at: u64,
    pub actor: String,
    pub router_id: String,
    pub router: String,
    /// `create` or `update`.
    pub action: String,
    pub changes: Vec<String>,
    pub digest: String,
}

#[derive(Clone, Default, Debug, PartialEq, Serialize, Deserialize)]
pub struct RouterActivity {
    pub events: Vec<RouterEvent>,
}

/// One connection test of one router: TCP, TLS, sign-in, and an identity read. The service runs it.
/// `class` is `ok`, `disabled`, `not_found`, `unreachable`, `tls`, `login`, `timeout`, `trap`,
/// `protocol`, `busy`, or `service` (the service did not answer). It never carries error text.
#[derive(Clone, Default, Debug, PartialEq, Serialize, Deserialize)]
pub struct RouterTestResult {
    pub router_id: String,
    pub class: String,
    /// The RouterOS identity name after a successful read.
    pub identity: Option<String>,
    pub elapsed_ms: u64,
    pub at: u64,
}
