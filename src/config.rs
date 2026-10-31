//! TOML configuration: types, defaults, and checks.
//!
//! All policy values live here. Each default is the value in `docs/CONFIGURATION.md`.

use crate::speed::Rate;
use anyhow::{Context, bail, ensure};
use serde::Deserialize;
use std::collections::BTreeSet;

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Config {
    pub runtime: Runtime,
    pub router_access: RouterAccess,
    pub safety: Safety,
    pub libreqos: Libreqos,
    pub network: Network,
    pub nodes: Nodes,
    pub circuits: CircuitNames,
    pub speed: Speed,
    pub bandwidth_defaults: BandwidthDefaults,
    pub backstop_caps: BackstopCaps,
    pub telegram: Telegram,
    pub exclude: Exclude,
    pub routers: Vec<RouterCfg>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RuntimeMode {
    Shadow,
    Apply,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LogLevel {
    Error,
    Warn,
    Info,
    Debug,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Runtime {
    pub mode: RuntimeMode,
    pub scan_interval_active: u64,
    pub scan_interval_idle: u64,
    pub error_retry_interval: u64,
    pub id_length: usize,
    pub remove_inactive_entries: bool,
    pub static_comment: String,
    pub subscriber_grace: u64,
    pub log_level: LogLevel,
    pub state_dir: String,
    /// Backup folders to keep in `<state_dir>/applied/` (apply mode).
    pub keep_backups: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OfflineAction {
    Remove,
    Keep,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RouterAccess {
    pub connect_timeout: u64,
    pub api_timeout: u64,
    pub read_retries: u32,
    pub retry_delay: u64,
    pub offline_after: u64,
    pub offline_action: OfflineAction,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Safety {
    pub enabled: bool,
    pub max_removed_percent: f64,
    pub max_removed_count: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Target {
    Auto,
    V2Pair,
    V3Pair,
    V3Network,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Libreqos {
    pub target: Target,
    pub directory: String,
    pub state_directory: String,
    pub v3_config: String,
    pub v2_command: String,
    pub v2_args: Vec<String>,
    pub v2_timeout: u64,
    pub v3_status_timeout: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NetworkMode {
    GroupedRouterRoot,
    FlatRouterRoot,
    FlatGlobalRoot,
    NoParent,
    RootPromotion,
    FlatPromotion,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Network {
    pub mode: NetworkMode,
    pub preserve_network_config: bool,
    pub root_name: String,
    pub root_type: String,
    pub root_download_mbps: f64,
    pub root_upload_mbps: f64,
}

/// Name template and optional type of a node that the tool makes.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NodeTemplate {
    pub name: String,
    /// One type for both LibreQoS versions. It overrides `v2_type` and `v3_type`.
    #[serde(rename = "type", default)]
    pub r#type: Option<String>,
    pub v2_type: String,
    pub v3_type: String,
}

impl NodeTemplate {
    /// The node type for the target (ADR-0006).
    pub fn type_for(&self, v3: bool) -> String {
        match &self.r#type {
            Some(t) => t.clone(),
            None if v3 => self.v3_type.clone(),
            None => self.v2_type.clone(),
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ClientTemplate {
    #[serde(rename = "type")]
    pub r#type: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Nodes {
    pub ppp: NodeTemplate,
    pub ppp_plan: NodeTemplate,
    pub hotspot: NodeTemplate,
    pub dhcp_site: NodeTemplate,
    pub dhcp_plan: NodeTemplate,
    pub client: ClientTemplate,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CircuitNames {
    pub pppoe: String,
    pub hotspot: String,
    pub dhcp: String,
    pub comment_pppoe: String,
    pub comment_hotspot: String,
    pub comment_dhcp: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RateLimitOrder {
    DownloadUpload,
    RouterosRxTx,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum UnitlessUnit {
    Mbps,
    Bps,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PppSource {
    SecretComment,
    ActiveComment,
    ProfileComment,
    ProfileRateLimit,
    ProfileName,
    Default,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Speed {
    pub rate_limit_order: RateLimitOrder,
    pub unitless_unit: UnitlessUnit,
    pub profile_name_regex: String,
    pub pppoe_sources: Vec<PppSource>,
    pub dhcp_per_site_sources: Vec<DhcpSource>,
    pub dhcp_per_plan_sources: Vec<DhcpSource>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BandwidthDefaults {
    pub min_rate_percentage: f64,
    pub safe_min_mbps: f64,
    pub max_floor_mbps: f64,
    pub pppoe_default_profile: String,
    pub pppoe_default_rate: String,
    pub pppoe_aggregate_factor: f64,
    pub dhcp_default_per_client_mbps: f64,
    pub dhcp_download_factor: f64,
    pub dhcp_upload_factor: f64,
    pub hotspot_default_per_client_mbps: f64,
    pub hotspot_download_factor: f64,
    pub hotspot_upload_factor: f64,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BackstopCap {
    pub enabled: bool,
    pub download_mbps: f64,
    pub upload_mbps: f64,
    pub multiplier: f64,
}

impl BackstopCap {
    /// The cap rate, or `None` if the cap is off.
    pub fn limit(&self) -> Option<Rate> {
        self.enabled.then_some(Rate {
            down: self.download_mbps * self.multiplier,
            up: self.upload_mbps * self.multiplier,
        })
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BackstopCaps {
    pub pppoe: BackstopCap,
    pub dhcp: BackstopCap,
    pub hotspot: BackstopCap,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Telegram {
    pub enabled: bool,
    pub bot_token_file: String,
    pub bot_token_env: String,
    pub chat_id: String,
    pub chat_id_env: String,
    pub timeout: u64,
    pub max_message_chars: usize,
    pub notify_on_changes: bool,
    pub notify_on_error: bool,
    pub notify_on_preflight_warning: bool,
    pub include_added: bool,
    pub include_updated: bool,
    pub include_removed: bool,
    pub include_computation: bool,
    pub include_telemetry: bool,
    pub max_list_items: usize,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RouterCfg {
    pub name: String,
    pub address: String,
    pub port: u16,
    pub tls: bool,
    /// Check the router certificate. `false` accepts any certificate (a warning at each connect).
    pub tls_verify: bool,
    /// PEM file with the CA of the router certificate. Empty uses the public CAs.
    pub tls_ca_file: String,
    pub username: String,
    pub password_file: String,
    pub pppoe: PppoeCfg,
    pub hotspot: HotspotCfg,
    pub dhcp: DhcpCfg,
    /// More exclusions for this router. The tool joins them with the global `[exclude]`.
    pub exclude: Exclude,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PppoeCfg {
    pub enabled: bool,
    pub per_plan_node: bool,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HotspotCfg {
    pub enabled: bool,
    pub include_mac: bool,
    pub download_limit_mbps: Option<f64>,
    pub upload_limit_mbps: Option<f64>,
    pub download_factor: Option<f64>,
    pub upload_factor: Option<f64>,
}

impl HotspotCfg {
    pub fn per_client(&self, bd: &BandwidthDefaults) -> Rate {
        Rate {
            down: self
                .download_limit_mbps
                .unwrap_or(bd.hotspot_default_per_client_mbps),
            up: self
                .upload_limit_mbps
                .unwrap_or(bd.hotspot_default_per_client_mbps),
        }
    }

    pub fn factors(&self, bd: &BandwidthDefaults) -> (f64, f64) {
        (
            self.download_factor.unwrap_or(bd.hotspot_download_factor),
            self.upload_factor.unwrap_or(bd.hotspot_upload_factor),
        )
    }
}

/// Subscribers that the tool does not shape. Names accept `*` wildcards (case is ignored).
/// IPs are IPv4 addresses or CIDR blocks.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Exclude {
    pub dhcp_servers: Vec<String>,
    pub ppp_profiles: Vec<String>,
    pub ips: Vec<String>,
}

/// Parse "a.b.c.d" or "a.b.c.d/n" to `(network, mask)`.
pub fn parse_ipv4_net(s: &str) -> Option<(u32, u32)> {
    let (addr, len) = match s.trim().split_once('/') {
        Some((a, l)) => (a, l.parse::<u32>().ok().filter(|l| *l <= 32)?),
        None => (s.trim(), 32),
    };
    let ip = u32::from(addr.parse::<std::net::Ipv4Addr>().ok()?);
    let mask = if len == 0 { 0 } else { u32::MAX << (32 - len) };
    Some((ip & mask, mask))
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DhcpCfg {
    pub enabled: bool,
    pub require_bound: bool,
    /// Read all DHCP servers from the router, as for PPP. `false` uses only `servers`.
    pub discover: bool,
    /// The mode of a server without its own `[[routers.dhcp.servers]]` entry.
    pub server_mode: DhcpMode,
    /// Settings for some servers. A discovered server that is not here uses the defaults.
    pub servers: Vec<DhcpServer>,
}

impl DhcpCfg {
    /// The settings of one server: its `[[routers.dhcp.servers]]` entry, or the defaults.
    pub fn server(&self, name: &str) -> DhcpServer {
        self.servers
            .iter()
            .find(|s| s.name == name)
            .cloned()
            .unwrap_or_else(|| DhcpServer {
                name: name.to_string(),
                mode: self.server_mode,
                speed_sources: None,
                default_plan_down_mbps: None,
                default_plan_up_mbps: None,
                download_factor: None,
                upload_factor: None,
            })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DhcpMode {
    PerSite,
    PerPlan,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DhcpSource {
    ServerComment,
    LeaseComment,
    Settings,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DhcpServer {
    pub name: String,
    pub mode: DhcpMode,
    #[serde(default)]
    pub speed_sources: Option<Vec<DhcpSource>>,
    #[serde(default)]
    pub default_plan_down_mbps: Option<f64>,
    #[serde(default)]
    pub default_plan_up_mbps: Option<f64>,
    #[serde(default)]
    pub download_factor: Option<f64>,
    #[serde(default)]
    pub upload_factor: Option<f64>,
}

impl DhcpServer {
    /// The speed sources in order. Without its own list, the server uses the list of its mode in `[speed]`.
    pub fn sources(&self, speed: &Speed) -> Vec<DhcpSource> {
        self.speed_sources
            .clone()
            .unwrap_or_else(|| match self.mode {
                DhcpMode::PerSite => speed.dhcp_per_site_sources.clone(),
                DhcpMode::PerPlan => speed.dhcp_per_plan_sources.clone(),
            })
    }

    pub fn settings_rate(&self, bd: &BandwidthDefaults) -> Rate {
        Rate {
            down: self
                .default_plan_down_mbps
                .unwrap_or(bd.dhcp_default_per_client_mbps),
            up: self
                .default_plan_up_mbps
                .unwrap_or(bd.dhcp_default_per_client_mbps),
        }
    }

    pub fn factors(&self, bd: &BandwidthDefaults) -> (f64, f64) {
        (
            self.download_factor.unwrap_or(bd.dhcp_download_factor),
            self.upload_factor.unwrap_or(bd.dhcp_upload_factor),
        )
    }
}

/// The embedded default settings. `config/defaults.toml` is the only source of default values.
pub const DEFAULTS_TOML: &str = include_str!("../config/defaults.toml");

fn defaults_table() -> toml::Table {
    toml::from_str(DEFAULTS_TOML).expect("config/defaults.toml is valid TOML")
}

/// The default config, with no routers. Built one time from `config/defaults.toml`.
pub fn defaults() -> &'static Config {
    static CFG: std::sync::OnceLock<Config> = std::sync::OnceLock::new();
    CFG.get_or_init(|| {
        Config::merge(defaults_table(), toml::Table::new())
            .expect("config/defaults.toml matches the config types")
    })
}

/// Put `over` on top of `base`: tables merge key by key, all other values replace.
fn deep_merge(base: &mut toml::Table, over: toml::Table) {
    for (k, v) in over {
        match (base.get_mut(&k), v) {
            (Some(toml::Value::Table(b)), toml::Value::Table(o)) => deep_merge(b, o),
            (_, v) => {
                base.insert(k, v);
            }
        }
    }
}

macro_rules! default_from_file {
    ($($ty:ident => $field:ident),* $(,)?) => {
        $(impl Default for $ty {
            fn default() -> Self {
                defaults().$field.clone()
            }
        })*
    };
}

default_from_file!(
    Runtime => runtime, RouterAccess => router_access, Safety => safety, Libreqos => libreqos,
    Network => network, Nodes => nodes, CircuitNames => circuits, Speed => speed,
    BandwidthDefaults => bandwidth_defaults, BackstopCaps => backstop_caps, Telegram => telegram,
    Exclude => exclude,
);

impl Default for Config {
    fn default() -> Self {
        defaults().clone()
    }
}

impl Config {
    /// Merge a user table over the defaults, apply `[router_defaults]` to each router, and read the types.
    fn merge(mut base: toml::Table, mut user: toml::Table) -> anyhow::Result<Config> {
        let mut router_defaults = match base.remove("router_defaults") {
            Some(toml::Value::Table(t)) => t,
            _ => toml::Table::new(),
        };
        match user.remove("router_defaults") {
            Some(toml::Value::Table(t)) => deep_merge(&mut router_defaults, t),
            Some(_) => bail!("config: [router_defaults] must be a table"),
            None => {}
        }
        let routers = match user.remove("routers") {
            Some(toml::Value::Array(a)) => a,
            Some(_) => bail!("config: routers must be a list of [[routers]] tables"),
            None => Vec::new(),
        };
        deep_merge(&mut base, user);
        let mut merged = Vec::new();
        for r in routers {
            let toml::Value::Table(r) = r else {
                bail!("config: each [[routers]] entry must be a table");
            };
            let mut m = router_defaults.clone();
            deep_merge(&mut m, r);
            // A listed DHCP server without a mode uses the router server_mode.
            if let Some(toml::Value::Table(dhcp)) = m.get_mut("dhcp") {
                let mode = dhcp.get("server_mode").cloned();
                if let (Some(mode), Some(toml::Value::Array(servers))) =
                    (mode, dhcp.get_mut("servers"))
                {
                    for s in servers.iter_mut() {
                        if let toml::Value::Table(s) = s {
                            s.entry("mode").or_insert(mode.clone());
                        }
                    }
                }
            }
            merged.push(toml::Value::Table(m));
        }
        base.insert("routers".into(), toml::Value::Array(merged));
        // The path tells the operator where the bad key is, for example routers[1].pppoe.
        serde_path_to_error::deserialize::<_, Config>(toml::Value::Table(base))
            .map_err(|e| anyhow::anyhow!("config: {}: {}", e.path(), e.inner()))
    }
}

impl Config {
    /// TOML accepts `inf` and `nan`. Reject them, so that no rate becomes infinite or `null`.
    fn check_finite(&self) -> anyhow::Result<()> {
        let bd = &self.bandwidth_defaults;
        let n = &self.network;
        let c = &self.backstop_caps;
        let mut values: Vec<(String, f64)> = vec![
            (
                "bandwidth_defaults.min_rate_percentage".into(),
                bd.min_rate_percentage,
            ),
            ("bandwidth_defaults.safe_min_mbps".into(), bd.safe_min_mbps),
            (
                "bandwidth_defaults.max_floor_mbps".into(),
                bd.max_floor_mbps,
            ),
            (
                "bandwidth_defaults.pppoe_aggregate_factor".into(),
                bd.pppoe_aggregate_factor,
            ),
            (
                "bandwidth_defaults.dhcp_default_per_client_mbps".into(),
                bd.dhcp_default_per_client_mbps,
            ),
            (
                "bandwidth_defaults.dhcp_download_factor".into(),
                bd.dhcp_download_factor,
            ),
            (
                "bandwidth_defaults.dhcp_upload_factor".into(),
                bd.dhcp_upload_factor,
            ),
            (
                "bandwidth_defaults.hotspot_default_per_client_mbps".into(),
                bd.hotspot_default_per_client_mbps,
            ),
            (
                "bandwidth_defaults.hotspot_download_factor".into(),
                bd.hotspot_download_factor,
            ),
            (
                "bandwidth_defaults.hotspot_upload_factor".into(),
                bd.hotspot_upload_factor,
            ),
            ("network.root_download_mbps".into(), n.root_download_mbps),
            ("network.root_upload_mbps".into(), n.root_upload_mbps),
            (
                "safety.max_removed_percent".into(),
                self.safety.max_removed_percent,
            ),
        ];
        for (k, cap) in [
            ("pppoe", &c.pppoe),
            ("dhcp", &c.dhcp),
            ("hotspot", &c.hotspot),
        ] {
            values.push((
                format!("backstop_caps.{k}.download_mbps"),
                cap.download_mbps,
            ));
            values.push((format!("backstop_caps.{k}.upload_mbps"), cap.upload_mbps));
            values.push((format!("backstop_caps.{k}.multiplier"), cap.multiplier));
        }
        for r in &self.routers {
            let h = &r.hotspot;
            for (k, v) in [
                ("hotspot.download_limit_mbps", h.download_limit_mbps),
                ("hotspot.upload_limit_mbps", h.upload_limit_mbps),
                ("hotspot.download_factor", h.download_factor),
                ("hotspot.upload_factor", h.upload_factor),
            ] {
                values.extend(v.map(|v| (format!("router \"{}\": {k}", r.name), v)));
            }
            for s in &r.dhcp.servers {
                for (k, v) in [
                    ("default_plan_down_mbps", s.default_plan_down_mbps),
                    ("default_plan_up_mbps", s.default_plan_up_mbps),
                    ("download_factor", s.download_factor),
                    ("upload_factor", s.upload_factor),
                ] {
                    values.extend(v.map(|v| {
                        (
                            format!("router \"{}\" server \"{}\": {k}", r.name, s.name),
                            v,
                        )
                    }));
                }
            }
        }
        for (k, v) in values {
            ensure!(
                v.is_finite(),
                "config: {k} must be a finite number, not {v}"
            );
        }
        Ok(())
    }

    /// Parse a TOML text and check all values.
    pub fn from_toml_str(s: &str) -> anyhow::Result<Config> {
        let user: toml::Table = toml::from_str(s).context("config: TOML parse error")?;
        let cfg = Config::merge(defaults_table(), user)?;
        cfg.validate()?;
        Ok(cfg)
    }

    /// `Some(true)` for a fixed v3 target, `Some(false)` for `v2_pair`, `None` for `auto`.
    pub fn targets_v3(&self) -> Option<bool> {
        match self.libreqos.target {
            Target::Auto => None,
            Target::V2Pair => Some(false),
            Target::V3Pair | Target::V3Network => Some(true),
        }
    }

    fn validate(&self) -> anyhow::Result<()> {
        self.check_finite()?;
        ensure!(
            !self.routers.is_empty(),
            "config: [[routers]] must have one router or more"
        );
        let check_exclude = |at: &str, x: &Exclude| -> anyhow::Result<()> {
            for (k, list) in [
                ("dhcp_servers", &x.dhcp_servers),
                ("ppp_profiles", &x.ppp_profiles),
            ] {
                ensure!(
                    list.iter().all(|n| !n.trim().is_empty()),
                    "config: {at}.{k} must not have an empty name"
                );
            }
            for ip in &x.ips {
                ensure!(
                    parse_ipv4_net(ip).is_some(),
                    "config: {at}.ips: \"{ip}\" is not an IPv4 address or CIDR block"
                );
            }
            Ok(())
        };
        check_exclude("exclude", &self.exclude)?;
        for r in &self.routers {
            check_exclude(&format!("router \"{}\": exclude", r.name), &r.exclude)?;
        }
        let mut names = BTreeSet::new();
        for r in &self.routers {
            ensure!(
                !r.name.trim().is_empty(),
                "config: routers[].name must not be empty"
            );
            ensure!(
                names.insert(r.name.as_str()),
                "config: routers[].name \"{}\" is used two times",
                r.name
            );
            let mut servers = BTreeSet::new();
            for s in &r.dhcp.servers {
                ensure!(
                    servers.insert(s.name.as_str()),
                    "config: router \"{}\": routers.dhcp.servers[].name \"{}\" is used two times",
                    r.name,
                    s.name
                );
                let src = s.sources(&self.speed);
                ensure!(
                    !src.is_empty(),
                    "config: router \"{}\" server \"{}\": speed_sources is empty",
                    r.name,
                    s.name
                );
                ensure!(
                    !(s.mode == DhcpMode::PerSite && src.contains(&DhcpSource::LeaseComment)),
                    "config: router \"{}\" server \"{}\": speed_sources must not contain lease_comment in per_site mode",
                    r.name,
                    s.name
                );
                for (k, v) in [
                    ("default_plan_down_mbps", s.default_plan_down_mbps),
                    ("default_plan_up_mbps", s.default_plan_up_mbps),
                ] {
                    if let Some(v) = v {
                        ensure!(
                            v > 0.0,
                            "config: router \"{}\" server \"{}\": {k} must be > 0",
                            r.name,
                            s.name
                        );
                    }
                }
                for (k, v) in [
                    ("download_factor", s.download_factor),
                    ("upload_factor", s.upload_factor),
                ] {
                    if let Some(v) = v {
                        ensure!(
                            v >= 0.0,
                            "config: router \"{}\" server \"{}\": {k} must be >= 0",
                            r.name,
                            s.name
                        );
                    }
                }
            }
            let h = &r.hotspot;
            for (k, v) in [
                ("download_limit_mbps", h.download_limit_mbps),
                ("upload_limit_mbps", h.upload_limit_mbps),
            ] {
                if let Some(v) = v {
                    ensure!(
                        v > 0.0,
                        "config: router \"{}\": hotspot.{k} must be > 0",
                        r.name
                    );
                }
            }
            for (k, v) in [
                ("download_factor", h.download_factor),
                ("upload_factor", h.upload_factor),
            ] {
                if let Some(v) = v {
                    ensure!(
                        v >= 0.0,
                        "config: router \"{}\": hotspot.{k} must be >= 0",
                        r.name
                    );
                }
            }
        }

        let bd = &self.bandwidth_defaults;
        ensure!(
            bd.min_rate_percentage > 0.0 && bd.min_rate_percentage <= 1.0,
            "config: bandwidth_defaults.min_rate_percentage must be > 0 and <= 1"
        );
        ensure!(
            bd.safe_min_mbps >= 0.1,
            "config: bandwidth_defaults.safe_min_mbps must be >= 0.1 (LibreQoS rule)"
        );
        ensure!(
            bd.max_floor_mbps >= 0.2,
            "config: bandwidth_defaults.max_floor_mbps must be >= 0.2 (LibreQoS rule)"
        );
        for (k, v) in [
            ("pppoe_aggregate_factor", bd.pppoe_aggregate_factor),
            ("dhcp_download_factor", bd.dhcp_download_factor),
            ("dhcp_upload_factor", bd.dhcp_upload_factor),
            ("hotspot_download_factor", bd.hotspot_download_factor),
            ("hotspot_upload_factor", bd.hotspot_upload_factor),
        ] {
            ensure!(v >= 0.0, "config: bandwidth_defaults.{k} must be >= 0");
        }
        for (k, v) in [
            (
                "dhcp_default_per_client_mbps",
                bd.dhcp_default_per_client_mbps,
            ),
            (
                "hotspot_default_per_client_mbps",
                bd.hotspot_default_per_client_mbps,
            ),
        ] {
            ensure!(v > 0.0, "config: bandwidth_defaults.{k} must be > 0");
        }
        ensure!(
            crate::speed::parse_default_rate(&bd.pppoe_default_rate).is_some(),
            "config: bandwidth_defaults.pppoe_default_rate must be \"<down>/<up>\", for example \"10M/10M\""
        );

        let n = &self.network;
        ensure!(
            n.root_download_mbps > 0.0,
            "config: network.root_download_mbps must be > 0"
        );
        ensure!(
            n.root_upload_mbps > 0.0,
            "config: network.root_upload_mbps must be > 0"
        );
        for (k, c) in [
            ("pppoe", &self.backstop_caps.pppoe),
            ("dhcp", &self.backstop_caps.dhcp),
            ("hotspot", &self.backstop_caps.hotspot),
        ] {
            ensure!(
                c.download_mbps > 0.0 && c.upload_mbps > 0.0,
                "config: backstop_caps.{k} rates must be > 0"
            );
            ensure!(
                c.multiplier >= 0.0,
                "config: backstop_caps.{k}.multiplier must be >= 0"
            );
        }

        let rt = &self.runtime;
        let ra = &self.router_access;
        for (k, v) in [
            ("runtime.scan_interval_active", rt.scan_interval_active),
            ("runtime.scan_interval_idle", rt.scan_interval_idle),
            ("runtime.error_retry_interval", rt.error_retry_interval),
            ("router_access.connect_timeout", ra.connect_timeout),
            ("router_access.api_timeout", ra.api_timeout),
            ("libreqos.v2_timeout", self.libreqos.v2_timeout),
            (
                "libreqos.v3_status_timeout",
                self.libreqos.v3_status_timeout,
            ),
            ("telegram.timeout", self.telegram.timeout),
        ] {
            ensure!(v > 0, "config: {k} must be > 0");
        }
        ensure!(
            rt.keep_backups >= 1,
            "config: runtime.keep_backups must be 1 or more"
        );
        ensure!(
            (4..=32).contains(&rt.id_length),
            "config: runtime.id_length must be 4 to 32"
        );
        ensure!(
            !rt.static_comment.trim().is_empty(),
            "config: runtime.static_comment must not be empty"
        );

        let sf = &self.safety;
        ensure!(
            sf.max_removed_percent > 0.0 && sf.max_removed_percent <= 100.0,
            "config: safety.max_removed_percent must be > 0 and <= 100"
        );

        let sp = &self.speed;
        ensure!(
            !sp.pppoe_sources.is_empty(),
            "config: speed.pppoe_sources must not be empty"
        );
        let re = regex::Regex::new(&sp.profile_name_regex)
            .with_context(|| "config: speed.profile_name_regex does not compile".to_string())?;
        let groups = re.captures_len() - 1;
        ensure!(
            (1..=2).contains(&groups),
            "config: speed.profile_name_regex must have 1 or 2 capture groups"
        );

        let nd = &self.nodes;
        for (k, t, need) in [
            ("nodes.ppp.name", &nd.ppp.name, &["{router}"][..]),
            ("nodes.hotspot.name", &nd.hotspot.name, &["{router}"]),
            (
                "nodes.ppp_plan.name",
                &nd.ppp_plan.name,
                &["{profile}", "{router}"],
            ),
            (
                "nodes.dhcp_site.name",
                &nd.dhcp_site.name,
                &["{server}", "{router}"],
            ),
            (
                "nodes.dhcp_plan.name",
                &nd.dhcp_plan.name,
                &["{plan}", "{router}"],
            ),
            ("circuits.pppoe", &self.circuits.pppoe, &["{user}"]),
            ("circuits.hotspot", &self.circuits.hotspot, &["{key}"]),
            ("circuits.dhcp", &self.circuits.dhcp, &["{key}"]),
        ] {
            for p in need {
                ensure!(t.contains(p), "config: {k} must contain {p}");
            }
        }
        // Only the known fields: a typo such as {usr} would stay as text in each name.
        let fields = regex::Regex::new(r"\{[^{}]*\}").expect("valid regex");
        for (k, t, allowed) in [
            ("nodes.ppp.name", &nd.ppp.name, &["{router}"][..]),
            ("nodes.hotspot.name", &nd.hotspot.name, &["{router}"]),
            (
                "nodes.ppp_plan.name",
                &nd.ppp_plan.name,
                &["{profile}", "{router}"],
            ),
            (
                "nodes.dhcp_site.name",
                &nd.dhcp_site.name,
                &["{server}", "{router}"],
            ),
            (
                "nodes.dhcp_plan.name",
                &nd.dhcp_plan.name,
                &["{plan}", "{router}"],
            ),
            (
                "circuits.pppoe",
                &self.circuits.pppoe,
                &["{user}", "{router}"],
            ),
            (
                "circuits.hotspot",
                &self.circuits.hotspot,
                &["{key}", "{router}"],
            ),
            ("circuits.dhcp", &self.circuits.dhcp, &["{key}", "{router}"]),
        ] {
            for m in fields.find_iter(t) {
                ensure!(
                    allowed.contains(&m.as_str()),
                    "config: {k}: {} is not a known field; use {}",
                    m.as_str(),
                    allowed.join(", ")
                );
            }
        }

        ensure!(
            !sp.dhcp_per_site_sources.is_empty(),
            "config: speed.dhcp_per_site_sources must not be empty"
        );
        ensure!(
            !sp.dhcp_per_plan_sources.is_empty(),
            "config: speed.dhcp_per_plan_sources must not be empty"
        );
        ensure!(
            !sp.dhcp_per_site_sources.contains(&DhcpSource::LeaseComment),
            "config: speed.dhcp_per_site_sources must not contain lease_comment"
        );
        // A tool row with the static comment comes back as an operator row and is never removed.
        let stat = rt.static_comment.trim();
        for (k, v) in [
            ("comment_pppoe", &self.circuits.comment_pppoe),
            ("comment_hotspot", &self.circuits.comment_hotspot),
            ("comment_dhcp", &self.circuits.comment_dhcp),
        ] {
            ensure!(
                !v.trim().eq_ignore_ascii_case(stat),
                "config: circuits.{k} must not be the same as runtime.static_comment \"{stat}\""
            );
        }

        if self.targets_v3() == Some(true) && n.mode == NetworkMode::NoParent {
            bail!(
                "config: network.mode \"no_parent\" does not work with a LibreQoS v3 target; use \"root_promotion\""
            );
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const MIN: &str =
        "[[routers]]\nname=\"R1\"\naddress=\"192.0.2.1\"\nusername=\"u\"\npassword_file=\"/x\"\n";

    #[test]
    fn minimal_config_gets_documented_defaults() {
        let c = Config::from_toml_str(MIN).unwrap();
        assert_eq!(c.runtime.scan_interval_active, 25);
        assert_eq!(c.runtime.scan_interval_idle, 120);
        assert_eq!(c.router_access.offline_after, 300);
        assert_eq!(c.router_access.offline_action, OfflineAction::Remove);
        assert_eq!(c.runtime.static_comment, "static");
        assert_eq!(c.network.root_name, "LibreQoS-Network");
        assert_eq!(c.nodes.dhcp_site.name, "DHCP-{server}-{router}");
        assert_eq!(c.speed.pppoe_sources.len(), 6);
        assert_eq!(c.routers[0].port, 8729);
        assert_eq!(c.bandwidth_defaults.max_floor_mbps, 0.2);
        assert_eq!(
            (
                c.safety.enabled,
                c.safety.max_removed_percent,
                c.safety.max_removed_count
            ),
            (true, 20.0, 50)
        );
        assert_eq!(c.speed.rate_limit_order, RateLimitOrder::RouterosRxTx);
    }

    #[test]
    fn unknown_key_is_rejected_with_its_name() {
        let e = Config::from_toml_str(&format!("[runtime]\nscan_intervall=5\n{MIN}")).unwrap_err();
        assert!(format!("{e:#}").contains("scan_intervall"));
    }

    #[test]
    fn out_of_range_values_are_rejected() {
        for bad in [
            "[bandwidth_defaults]\nmin_rate_percentage=1.5\n",
            "[bandwidth_defaults]\nsafe_min_mbps=0.05\n",
            "[bandwidth_defaults]\nmax_floor_mbps=0.1\n",
            "[router_access]\napi_timeout=0\n",
            "[speed]\nprofile_name_regex='('\n",
            "[speed]\npppoe_sources=[]\n",
            "[nodes]\nppp={name=\"PPP\",type=\"plan\"}\n",
            "[circuits]\nhotspot=\"HS\"\n",
        ] {
            assert!(
                Config::from_toml_str(&format!("{bad}{MIN}")).is_err(),
                "{bad}"
            );
        }
    }

    #[test]
    fn router_list_problems_are_rejected() {
        assert!(Config::from_toml_str("").is_err());
        assert!(
            format!(
                "{:#}",
                Config::from_toml_str(&format!("{MIN}{MIN}")).unwrap_err()
            )
            .contains("R1")
        );
    }

    #[test]
    fn v3_target_rejects_no_parent() {
        let e = Config::from_toml_str(&format!(
            "[libreqos]\ntarget=\"v3_pair\"\n[network]\nmode=\"no_parent\"\n{MIN}"
        ))
        .unwrap_err();
        assert!(format!("{e:#}").contains("root_promotion"));
        // With target auto, Plan 3 does this check.
        assert!(Config::from_toml_str(&format!("[network]\nmode=\"no_parent\"\n{MIN}")).is_ok());
    }

    #[test]
    fn node_type_defaults_follow_target() {
        let c = Config::from_toml_str(MIN).unwrap();
        assert_eq!(c.nodes.ppp_plan.type_for(false), "plan");
        assert_eq!(c.nodes.ppp_plan.type_for(true), "site");
        let c = Config::from_toml_str(&format!(
            "[nodes]\nppp_plan={{name=\"{{profile}}-{{router}}\",type=\"ap\"}}\n{MIN}"
        ))
        .unwrap();
        assert_eq!(c.nodes.ppp_plan.type_for(true), "ap");
    }

    #[test]
    fn per_site_rejects_lease_comment_and_defaults_follow_mode() {
        let s = |mode: &str, src: &str| {
            format!(
                "{MIN}[routers.dhcp]\nenabled=true\n[[routers.dhcp.servers]]\nname=\"LAN\"\nmode=\"{mode}\"\n{src}"
            )
        };
        assert!(
            Config::from_toml_str(&s("per_site", "speed_sources=[\"lease_comment\"]\n")).is_err()
        );
        let c = Config::from_toml_str(&s("per_plan", "")).unwrap();
        assert_eq!(
            c.routers[0].dhcp.servers[0].sources(&c.speed),
            vec![DhcpSource::LeaseComment]
        );
        let c = Config::from_toml_str(&s("per_site", "")).unwrap();
        assert_eq!(
            c.routers[0].dhcp.servers[0].sources(&c.speed),
            vec![DhcpSource::ServerComment, DhcpSource::Settings]
        );
    }

    #[test]
    fn non_finite_values_are_rejected() {
        for bad in [
            "[network]\nroot_download_mbps=inf\n",
            "[bandwidth_defaults]\npppoe_aggregate_factor=nan\n",
            "[backstop_caps.pppoe]\nmultiplier=inf\n",
        ] {
            assert!(
                Config::from_toml_str(&format!("{bad}{MIN}")).is_err(),
                "{bad}"
            );
        }
    }

    #[test]
    fn exclusions_parse_and_bad_ip_is_rejected() {
        let c = Config::from_toml_str(&format!(
            "[exclude]\nips=[\"100.64.9.0/24\",\"100.64.0.7\"]\nppp_profiles=[\"default-*\"]\n{MIN}[routers.exclude]\ndhcp_servers=[\"Mgt*\"]\n"
        ))
        .unwrap();
        assert_eq!(c.exclude.ips.len(), 2);
        assert_eq!(c.routers[0].exclude.dhcp_servers, vec!["Mgt*".to_string()]);
        for bad in [
            "[exclude]\nips=[\"100.64.9.0/33\"]\n",
            "[exclude]\nips=[\"not-an-ip\"]\n",
            "[exclude]\nppp_profiles=[\"\"]\n",
        ] {
            assert!(
                Config::from_toml_str(&format!("{bad}{MIN}")).is_err(),
                "{bad}"
            );
        }
    }

    #[test]
    fn dhcp_server_gives_override_or_default() {
        let c = Config::from_toml_str(&format!(
            "{MIN}[routers.dhcp]\nenabled=true\n[[routers.dhcp.servers]]\nname=\"LAN\"\ndefault_plan_down_mbps=50\n"
        ))
        .unwrap();
        let d = &c.routers[0].dhcp;
        assert!(d.discover);
        assert_eq!(d.server("LAN").default_plan_down_mbps, Some(50.0));
        let other = d.server("GUEST");
        assert_eq!(
            (
                other.name.as_str(),
                other.mode,
                other.default_plan_down_mbps
            ),
            ("GUEST", DhcpMode::PerSite, None)
        );
    }

    #[test]
    fn router_defaults_apply_to_each_router_and_a_router_overrides_them() {
        let two = format!("{MIN}{}", MIN.replace("R1", "R2") + "port=8000\n");
        let c = Config::from_toml_str(&format!(
            "[router_defaults]\nport=8728\n[router_defaults.pppoe]\nenabled=true\n{two}"
        ))
        .unwrap();
        assert_eq!(
            (c.routers[0].port, c.routers[0].pppoe.enabled),
            (8728, true)
        );
        assert_eq!(c.routers[1].port, 8000);
    }

    #[test]
    fn every_default_comes_from_the_defaults_file() {
        let file: toml::Table = toml::from_str(DEFAULTS_TOML).unwrap();
        assert_eq!(
            file["runtime"]["scan_interval_active"].as_integer(),
            Some(defaults().runtime.scan_interval_active as i64)
        );
        assert_eq!(file["router_defaults"]["port"].as_integer(), Some(8729));
        assert!(defaults().routers.is_empty());
    }

    #[test]
    fn unknown_template_fields_are_rejected() {
        for bad in [
            "[circuits]\npppoe=\"{user}-{usr}\"\n",
            "[nodes.ppp]\nname=\"PPP-{router}-{site}\"\n",
            "[circuits]\nhotspot=\"HS-{key}-{profile}\"\n",
        ] {
            let e = Config::from_toml_str(&format!("{bad}{MIN}")).unwrap_err();
            assert!(format!("{e:#}").contains('{'), "{bad}: {e:#}");
        }
        Config::from_toml_str(&format!("[circuits]\npppoe=\"{{user}}@{{router}}\"\n{MIN}"))
            .unwrap();
    }

    #[test]
    fn config_errors_name_the_key_path() {
        let e = format!(
            "{:#}",
            Config::from_toml_str(&format!("[runtime]\nscan_interval_active=\"x\"\n{MIN}"))
                .unwrap_err()
        );
        assert!(e.contains("runtime.scan_interval_active"), "{e}");
        let e = format!(
            "{:#}",
            Config::from_toml_str(&format!("{MIN}[routers.pppoe]\nenabeld=true\n")).unwrap_err()
        );
        assert!(
            e.contains("routers[0].pppoe") && e.contains("enabeld"),
            "{e}"
        );
    }

    #[test]
    fn global_dhcp_source_lists_are_checked() {
        for bad in [
            "[speed]\ndhcp_per_site_sources=[]\n",
            "[speed]\ndhcp_per_plan_sources=[]\n",
            "[speed]\ndhcp_per_site_sources=[\"lease_comment\"]\n",
        ] {
            assert!(
                Config::from_toml_str(&format!("{bad}{MIN}")).is_err(),
                "{bad}"
            );
        }
    }

    #[test]
    fn a_circuit_comment_must_not_be_the_static_comment() {
        let e = Config::from_toml_str(&format!("[circuits]\ncomment_pppoe=\" Static \"\n{MIN}"))
            .unwrap_err();
        assert!(format!("{e:#}").contains("static_comment"), "{e:#}");
    }

    #[test]
    fn router_example_in_defaults_file_loads_two_routers() {
        let start = DEFAULTS_TOML.find("# EXAMPLE-START\n").unwrap();
        let end = DEFAULTS_TOML.find("# EXAMPLE-END").unwrap();
        let example: String = DEFAULTS_TOML[start..end]
            .lines()
            .skip(1)
            .map(|l| {
                l.strip_prefix("# ")
                    .or_else(|| l.strip_prefix('#'))
                    .unwrap_or(l)
            })
            .collect::<Vec<_>>()
            .join("\n");
        let c = Config::from_toml_str(&example).unwrap();
        assert_eq!(c.routers.len(), 2);
        let (hub, tower) = (&c.routers[0], &c.routers[1]);
        assert_eq!(
            (hub.port, hub.tls, hub.pppoe.enabled, hub.dhcp.discover),
            (8729, true, true, true)
        );
        assert_eq!(
            (
                tower.port,
                tower.tls,
                tower.pppoe.enabled,
                tower.dhcp.discover
            ),
            (8728, false, false, false)
        );
        assert_eq!(tower.dhcp.servers[0].mode, DhcpMode::PerPlan);
    }

    #[test]
    fn tls_keys_have_defaults_and_can_change() {
        let c = Config::from_toml_str(&format!(
            "{MIN}tls_verify=false\ntls_ca_file=\"/etc/ca.pem\"\n"
        ))
        .unwrap();
        assert_eq!(
            (c.routers[0].tls_verify, c.routers[0].tls_ca_file.as_str()),
            (false, "/etc/ca.pem")
        );
        let c = Config::from_toml_str(MIN).unwrap();
        assert!(c.routers[0].tls_verify && c.routers[0].tls_ca_file.is_empty());
    }
}
