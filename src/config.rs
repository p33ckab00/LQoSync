//! TOML configuration: types, defaults, and checks.
//!
//! All policy values live here. Each default is the value in `docs/CONFIGURATION.md`.

use crate::speed::Rate;
use anyhow::{Context, bail, ensure};
use serde::Deserialize;
use std::collections::BTreeSet;

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(deny_unknown_fields, default)]
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
#[serde(deny_unknown_fields, default)]
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
}

impl Default for Runtime {
    fn default() -> Self {
        Runtime {
            mode: RuntimeMode::Shadow,
            scan_interval_active: 25,
            scan_interval_idle: 120,
            error_retry_interval: 30,
            id_length: 8,
            remove_inactive_entries: true,
            static_comment: "static".into(),
            subscriber_grace: 0,
            log_level: LogLevel::Info,
            state_dir: "/var/lib/lqosync3".into(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OfflineAction {
    Remove,
    Keep,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields, default)]
pub struct RouterAccess {
    pub connect_timeout: u64,
    pub api_timeout: u64,
    pub read_retries: u32,
    pub retry_delay: u64,
    pub offline_after: u64,
    pub offline_action: OfflineAction,
}

impl Default for RouterAccess {
    fn default() -> Self {
        RouterAccess {
            connect_timeout: 5,
            api_timeout: 10,
            read_retries: 2,
            retry_delay: 3,
            offline_after: 300,
            offline_action: OfflineAction::Remove,
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields, default)]
pub struct Safety {
    pub enabled: bool,
    pub max_removed_percent: f64,
    pub max_removed_count: usize,
}

impl Default for Safety {
    fn default() -> Self {
        Safety {
            enabled: true,
            max_removed_percent: 20.0,
            max_removed_count: 50,
        }
    }
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
#[serde(deny_unknown_fields, default)]
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

impl Default for Libreqos {
    fn default() -> Self {
        Libreqos {
            target: Target::Auto,
            directory: "/opt/libreqos/src".into(),
            state_directory: "/opt/libreqos/state".into(),
            v3_config: "/etc/libreqos/lqos.conf".into(),
            v2_command: "/opt/libreqos/src/LibreQoS.py".into(),
            v2_args: vec!["--updateonly".into()],
            v2_timeout: 300,
            v3_status_timeout: 60,
        }
    }
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
#[serde(deny_unknown_fields, default)]
pub struct Network {
    pub mode: NetworkMode,
    pub preserve_network_config: bool,
    pub root_name: String,
    pub root_type: String,
    pub root_download_mbps: f64,
    pub root_upload_mbps: f64,
}

impl Default for Network {
    fn default() -> Self {
        Network {
            mode: NetworkMode::GroupedRouterRoot,
            preserve_network_config: false,
            root_name: "LibreQoS-Network".into(),
            root_type: "site".into(),
            root_download_mbps: 640.0,
            root_upload_mbps: 640.0,
        }
    }
}

/// Name template and optional type of a node that the tool makes.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NodeTemplate {
    pub name: String,
    #[serde(rename = "type", default)]
    pub r#type: Option<String>,
}

impl NodeTemplate {
    fn named(name: &str) -> Self {
        NodeTemplate {
            name: name.into(),
            r#type: None,
        }
    }

    /// The explicit type, else "site" on v3, else the v2 default (ADR-0006).
    pub fn type_for(&self, v2_default: &str, v3: bool) -> String {
        match &self.r#type {
            Some(t) => t.clone(),
            None if v3 => "site".into(),
            None => v2_default.into(),
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields, default)]
pub struct ClientTemplate {
    #[serde(rename = "type")]
    pub r#type: String,
}

impl Default for ClientTemplate {
    fn default() -> Self {
        ClientTemplate {
            r#type: "client".into(),
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields, default)]
pub struct Nodes {
    pub ppp: NodeTemplate,
    pub ppp_plan: NodeTemplate,
    pub hotspot: NodeTemplate,
    pub dhcp_site: NodeTemplate,
    pub dhcp_plan: NodeTemplate,
    pub client: ClientTemplate,
}

impl Default for Nodes {
    fn default() -> Self {
        Nodes {
            ppp: NodeTemplate::named("PPP-{router}"),
            ppp_plan: NodeTemplate::named("{profile}-{router}"),
            hotspot: NodeTemplate::named("HS-{router}"),
            dhcp_site: NodeTemplate::named("DHCP-{server}-{router}"),
            dhcp_plan: NodeTemplate::named("PLAN-DHCP-{plan}-{router}"),
            client: ClientTemplate::default(),
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields, default)]
pub struct CircuitNames {
    pub pppoe: String,
    pub hotspot: String,
    pub dhcp: String,
}

impl Default for CircuitNames {
    fn default() -> Self {
        CircuitNames {
            pppoe: "{user}".into(),
            hotspot: "HS-{key}".into(),
            dhcp: "DHCP-{key}".into(),
        }
    }
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
#[serde(deny_unknown_fields, default)]
pub struct Speed {
    pub rate_limit_order: RateLimitOrder,
    pub unitless_unit: UnitlessUnit,
    pub profile_name_regex: String,
    pub pppoe_sources: Vec<PppSource>,
}

impl Default for Speed {
    fn default() -> Self {
        Speed {
            rate_limit_order: RateLimitOrder::RouterosRxTx,
            unitless_unit: UnitlessUnit::Bps,
            profile_name_regex: r"(\d+(?:\.\d+)?)\s*([kKmMgG])".into(),
            pppoe_sources: vec![
                PppSource::SecretComment,
                PppSource::ActiveComment,
                PppSource::ProfileComment,
                PppSource::ProfileRateLimit,
                PppSource::ProfileName,
                PppSource::Default,
            ],
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields, default)]
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

impl Default for BandwidthDefaults {
    fn default() -> Self {
        BandwidthDefaults {
            min_rate_percentage: 0.5,
            safe_min_mbps: 0.128,
            max_floor_mbps: 0.2,
            pppoe_default_profile: "default".into(),
            pppoe_default_rate: "10M/10M".into(),
            pppoe_aggregate_factor: 0.5,
            dhcp_default_per_client_mbps: 15.0,
            dhcp_download_factor: 0.5,
            dhcp_upload_factor: 0.5,
            hotspot_default_per_client_mbps: 10.0,
            hotspot_download_factor: 1.0,
            hotspot_upload_factor: 1.0,
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields, default)]
pub struct BackstopCap {
    pub enabled: bool,
    pub download_mbps: f64,
    pub upload_mbps: f64,
    pub multiplier: f64,
}

impl BackstopCap {
    fn new(mbps: f64) -> Self {
        BackstopCap {
            enabled: false,
            download_mbps: mbps,
            upload_mbps: mbps,
            multiplier: 1.0,
        }
    }

    /// The cap rate, or `None` if the cap is off.
    pub fn limit(&self) -> Option<Rate> {
        self.enabled.then_some(Rate {
            down: self.download_mbps * self.multiplier,
            up: self.upload_mbps * self.multiplier,
        })
    }
}

impl Default for BackstopCap {
    fn default() -> Self {
        BackstopCap::new(600.0)
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields, default)]
pub struct BackstopCaps {
    pub pppoe: BackstopCap,
    pub dhcp: BackstopCap,
    pub hotspot: BackstopCap,
}

impl Default for BackstopCaps {
    fn default() -> Self {
        BackstopCaps {
            pppoe: BackstopCap::new(600.0),
            dhcp: BackstopCap::new(250.0),
            hotspot: BackstopCap::new(100.0),
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields, default)]
pub struct Telegram {
    pub enabled: bool,
    pub bot_token_file: String,
    pub bot_token_env: Option<String>,
    pub chat_id: String,
    pub chat_id_env: Option<String>,
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

impl Default for Telegram {
    fn default() -> Self {
        Telegram {
            enabled: false,
            bot_token_file: "/etc/lqosync3/secrets/telegram.token".into(),
            bot_token_env: None,
            chat_id: String::new(),
            chat_id_env: None,
            timeout: 10,
            max_message_chars: 3900,
            notify_on_changes: true,
            notify_on_error: true,
            notify_on_preflight_warning: true,
            include_added: true,
            include_updated: true,
            include_removed: true,
            include_computation: true,
            include_telemetry: true,
            max_list_items: 15,
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RouterCfg {
    pub name: String,
    pub address: String,
    #[serde(default = "default_port")]
    pub port: u16,
    #[serde(default = "yes")]
    pub tls: bool,
    pub username: String,
    pub password_file: String,
    #[serde(default)]
    pub pppoe: PppoeCfg,
    #[serde(default)]
    pub hotspot: HotspotCfg,
    #[serde(default)]
    pub dhcp: DhcpCfg,
}

fn default_port() -> u16 {
    8729
}

fn yes() -> bool {
    true
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(deny_unknown_fields, default)]
pub struct PppoeCfg {
    pub enabled: bool,
    pub per_plan_node: bool,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields, default)]
pub struct HotspotCfg {
    pub enabled: bool,
    pub include_mac: bool,
    pub download_limit_mbps: Option<f64>,
    pub upload_limit_mbps: Option<f64>,
    pub download_factor: Option<f64>,
    pub upload_factor: Option<f64>,
}

impl Default for HotspotCfg {
    fn default() -> Self {
        HotspotCfg {
            enabled: false,
            include_mac: true,
            download_limit_mbps: None,
            upload_limit_mbps: None,
            download_factor: None,
            upload_factor: None,
        }
    }
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

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(deny_unknown_fields, default)]
pub struct DhcpCfg {
    pub enabled: bool,
    pub require_bound: bool,
    pub servers: Vec<DhcpServer>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DhcpMode {
    #[default]
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
    #[serde(default)]
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
    /// The speed sources in order. The default depends on the mode.
    pub fn sources(&self) -> Vec<DhcpSource> {
        self.speed_sources
            .clone()
            .unwrap_or_else(|| match self.mode {
                DhcpMode::PerSite => vec![DhcpSource::ServerComment, DhcpSource::Settings],
                DhcpMode::PerPlan => vec![DhcpSource::LeaseComment],
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
        let cfg: Config = toml::from_str(s).context("config: TOML parse error")?;
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
                let src = s.sources();
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
        assert_eq!(c.nodes.ppp_plan.type_for("plan", false), "plan");
        assert_eq!(c.nodes.ppp_plan.type_for("plan", true), "site");
        let c = Config::from_toml_str(&format!(
            "[nodes]\nppp_plan={{name=\"{{profile}}-{{router}}\",type=\"ap\"}}\n{MIN}"
        ))
        .unwrap();
        assert_eq!(c.nodes.ppp_plan.type_for("plan", true), "ap");
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
            c.routers[0].dhcp.servers[0].sources(),
            vec![DhcpSource::LeaseComment]
        );
        let c = Config::from_toml_str(&s("per_site", "")).unwrap();
        assert_eq!(
            c.routers[0].dhcp.servers[0].sources(),
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
}
