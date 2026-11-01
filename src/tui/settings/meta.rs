//! Key metadata: the editor type, the default, and the help of each config key.

use crate::config::DEFAULTS_TOML;
use std::collections::HashMap;
use std::sync::OnceLock;
use toml_edit::{DocumentMut, Item, Table, Value};

/// The editor type of a key.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Kind {
    Bool,
    Int,
    Float,
    Text,
    Choice(&'static [&'static str]),
    List(Box<Kind>),
}

#[derive(Debug, Clone)]
pub struct KeyMeta {
    /// A TOML path; `*` stands for a list index (`routers.*.port`).
    pub path: String,
    pub kind: Kind,
    /// The default as TOML text, or `None` if the key has no default.
    pub default: Option<String>,
    pub help: String,
    /// The key can be absent.
    pub optional: bool,
    pub read_only: bool,
}

const TUI_THEME: &[&str] = &["dark", "light", "mono"];
const RUNTIME_MODE: &[&str] = &["shadow", "apply"];
const LOG_LEVEL: &[&str] = &["error", "warn", "info", "debug"];
const OFFLINE_ACTION: &[&str] = &["remove", "keep"];
const TARGET: &[&str] = &["auto", "v2_pair", "v3_pair", "v3_network"];
const NETWORK_MODE: &[&str] = &[
    "grouped_router_root",
    "flat_router_root",
    "flat_global_root",
    "no_parent",
    "root_promotion",
    "flat_promotion",
];
const RATE_LIMIT_ORDER: &[&str] = &["download_upload", "routeros_rx_tx"];
const UNITLESS_UNIT: &[&str] = &["mbps", "bps"];
const PPP_SOURCE: &[&str] = &[
    "secret_comment",
    "active_comment",
    "profile_comment",
    "profile_rate_limit",
    "profile_name",
    "default",
];
const DHCP_MODE: &[&str] = &["per_site", "per_plan"];
const DHCP_SOURCE: &[&str] = &["server_comment", "lease_comment", "settings"];

/// The keys of the 11 enums of `config.rs` and their TOML values.
pub const CHOICES: &[(&str, &[&str])] = &[
    ("tui.theme", TUI_THEME),
    ("runtime.mode", RUNTIME_MODE),
    ("runtime.log_level", LOG_LEVEL),
    ("router_access.offline_action", OFFLINE_ACTION),
    ("libreqos.target", TARGET),
    ("network.mode", NETWORK_MODE),
    ("speed.rate_limit_order", RATE_LIMIT_ORDER),
    ("speed.unitless_unit", UNITLESS_UNIT),
    ("speed.pppoe_sources", PPP_SOURCE),
    ("speed.dhcp_per_site_sources", DHCP_SOURCE),
    ("speed.dhcp_per_plan_sources", DHCP_SOURCE),
    ("router_defaults.dhcp.server_mode", DHCP_MODE),
    ("routers.*.dhcp.server_mode", DHCP_MODE),
    ("routers.*.dhcp.servers.*.mode", DHCP_MODE),
    ("routers.*.dhcp.servers.*.speed_sources", DHCP_SOURCE),
];

/// The `f64` and `Option<f64>` fields of `config.rs`. A number key not in this list is an integer.
pub const FLOATS: &[&str] = &[
    "safety.max_removed_percent",
    "network.root_download_mbps",
    "network.root_upload_mbps",
    "bandwidth_defaults.min_rate_percentage",
    "bandwidth_defaults.safe_min_mbps",
    "bandwidth_defaults.max_floor_mbps",
    "bandwidth_defaults.pppoe_aggregate_factor",
    "bandwidth_defaults.dhcp_default_per_client_mbps",
    "bandwidth_defaults.dhcp_download_factor",
    "bandwidth_defaults.dhcp_upload_factor",
    "bandwidth_defaults.hotspot_default_per_client_mbps",
    "bandwidth_defaults.hotspot_download_factor",
    "bandwidth_defaults.hotspot_upload_factor",
    "backstop_caps.pppoe.download_mbps",
    "backstop_caps.pppoe.upload_mbps",
    "backstop_caps.pppoe.multiplier",
    "backstop_caps.dhcp.download_mbps",
    "backstop_caps.dhcp.upload_mbps",
    "backstop_caps.dhcp.multiplier",
    "backstop_caps.hotspot.download_mbps",
    "backstop_caps.hotspot.upload_mbps",
    "backstop_caps.hotspot.multiplier",
    "routers.*.dhcp.clients.*.download_mbps",
    "routers.*.dhcp.clients.*.upload_mbps",
];

/// Keys that are not in the defaults file as a key: (path, kind, help, optional).
const EXTRA: &[(&str, Base, &str, bool)] = &[
    (
        "routers.*.name",
        Base::Text,
        "unique router name; used in node names and in the password file name",
        false,
    ),
    (
        "routers.*.address",
        Base::Text,
        "IP address or DNS name of the router API",
        false,
    ),
    (
        "routers.*.username",
        Base::Text,
        "a read-only RouterOS user",
        false,
    ),
    (
        "routers.*.password_file",
        Base::Text,
        "the password file (mode 0600, owner root); default /etc/lqosync3/secrets/<name>.password",
        false,
    ),
    (
        "routers.*.hotspot.download_limit_mbps",
        Base::Float,
        "Mbps for each hotspot client; not set: bandwidth_defaults.hotspot_default_per_client_mbps",
        true,
    ),
    (
        "routers.*.hotspot.upload_limit_mbps",
        Base::Float,
        "Mbps for each hotspot client; not set: bandwidth_defaults.hotspot_default_per_client_mbps",
        true,
    ),
    (
        "routers.*.hotspot.download_factor",
        Base::Float,
        "not set: bandwidth_defaults.hotspot_download_factor",
        true,
    ),
    (
        "routers.*.hotspot.upload_factor",
        Base::Float,
        "not set: bandwidth_defaults.hotspot_upload_factor",
        true,
    ),
    (
        "nodes.ppp.type",
        Base::Text,
        "one type for LibreQoS 2.x and 3.x; not set: v2_type or v3_type",
        true,
    ),
    (
        "nodes.ppp_plan.type",
        Base::Text,
        "one type for LibreQoS 2.x and 3.x; not set: v2_type or v3_type",
        true,
    ),
    (
        "nodes.hotspot.type",
        Base::Text,
        "one type for LibreQoS 2.x and 3.x; not set: v2_type or v3_type",
        true,
    ),
    (
        "nodes.dhcp_site.type",
        Base::Text,
        "one type for LibreQoS 2.x and 3.x; not set: v2_type or v3_type",
        true,
    ),
    (
        "nodes.dhcp_plan.type",
        Base::Text,
        "one type for LibreQoS 2.x and 3.x; not set: v2_type or v3_type",
        true,
    ),
    (
        "routers.*.dhcp.servers.*.name",
        Base::Text,
        "the DHCP server name on the router",
        false,
    ),
    (
        "routers.*.dhcp.servers.*.mode",
        Base::Text,
        "per_site | per_plan; not set: dhcp.server_mode of the router",
        true,
    ),
    (
        "routers.*.dhcp.servers.*.speed_sources",
        Base::List,
        "speed sources of this server; not set: the speed.dhcp_* sources of its mode",
        true,
    ),
    (
        "routers.*.dhcp.servers.*.default_plan_down_mbps",
        Base::Float,
        "not set: bandwidth_defaults.dhcp_default_per_client_mbps",
        true,
    ),
    (
        "routers.*.dhcp.servers.*.default_plan_up_mbps",
        Base::Float,
        "not set: bandwidth_defaults.dhcp_default_per_client_mbps",
        true,
    ),
    (
        "routers.*.dhcp.servers.*.download_factor",
        Base::Float,
        "not set: bandwidth_defaults.dhcp_download_factor",
        true,
    ),
    (
        "routers.*.dhcp.clients.*.mac",
        Base::Text,
        "the MAC address of the client (AA:BB:CC:DD:EE:FF); the override is found by it",
        false,
    ),
    (
        "routers.*.dhcp.clients.*.download_mbps",
        Base::Float,
        "the max download rate of this client; it replaces the rate of its lease",
        false,
    ),
    (
        "routers.*.dhcp.clients.*.upload_mbps",
        Base::Float,
        "the max upload rate of this client; it replaces the rate of its lease",
        false,
    ),
    (
        "routers.*.dhcp.clients.*.name",
        Base::Text,
        "a note for the operator, shown in the TUI",
        true,
    ),
    (
        "routers.*.dhcp.servers.*.upload_factor",
        Base::Float,
        "not set: bandwidth_defaults.dhcp_upload_factor",
        true,
    ),
];

#[derive(Debug, Clone, Copy)]
enum Base {
    Text,
    Float,
    List,
}

fn kind_for(path: &str, base: Kind) -> Kind {
    let choice = CHOICES.iter().find(|(p, _)| *p == path).map(|(_, c)| *c);
    match (base, choice) {
        (Kind::List(_), Some(c)) => Kind::List(Box::new(Kind::Choice(c))),
        (Kind::Text, Some(c)) => Kind::Choice(c),
        (Kind::Int, _) if FLOATS.contains(&path) => Kind::Float,
        (k, _) => k,
    }
}

/// The text of comment lines and of a trailing comment, without `#`.
fn comment_text(raw: Option<&str>) -> Vec<String> {
    raw.unwrap_or("")
        .lines()
        .filter_map(|l| l.trim().strip_prefix('#'))
        .map(|l| l.trim().to_string())
        .filter(|l| !l.is_empty())
        .collect()
}

fn walk(t: &Table, prefix: &str, out: &mut Vec<KeyMeta>) {
    for (k, item) in t.iter() {
        let path = if prefix.is_empty() {
            k.to_string()
        } else {
            format!("{prefix}.{k}")
        };
        match item {
            Item::Table(sub) => walk(sub, &path, out),
            // The DHCP servers and the client overrides are groups of the router, not keys.
            Item::Value(_)
                if path == "router_defaults.dhcp.servers"
                    || path == "router_defaults.dhcp.clients" => {}
            Item::Value(v) => {
                let base = match v {
                    Value::Boolean(_) => Kind::Bool,
                    Value::Integer(_) => Kind::Int,
                    Value::Float(_) => Kind::Float,
                    Value::Array(_) => Kind::List(Box::new(Kind::Text)),
                    _ => Kind::Text,
                };
                let mut help = comment_text(
                    t.key(k)
                        .and_then(|key| key.leaf_decor().prefix())
                        .and_then(|r| r.as_str()),
                );
                help.extend(comment_text(v.decor().suffix().and_then(|r| r.as_str())));
                out.push(KeyMeta {
                    kind: kind_for(&path, base),
                    default: Some(v.clone().decorated("", "").to_string()),
                    help: help.join("\n"),
                    optional: false,
                    read_only: path == "runtime.state_dir",
                    path,
                });
            }
            _ => {}
        }
    }
}

fn build() -> Vec<KeyMeta> {
    let doc: DocumentMut = DEFAULTS_TOML
        .parse()
        .expect("config/defaults.toml is valid TOML");
    let mut out = Vec::new();
    walk(doc.as_table(), "", &mut out);
    // A router key has the default of router_defaults.
    let router: Vec<KeyMeta> = out
        .iter()
        .filter_map(|m| {
            let rest = m.path.strip_prefix("router_defaults.")?;
            let path = format!("routers.*.{rest}");
            Some(KeyMeta {
                kind: kind_for(&path, m.kind.clone()),
                path,
                ..m.clone()
            })
        })
        .collect();
    out.extend(router);
    for (path, base, help, optional) in EXTRA {
        let base = match base {
            Base::Text => Kind::Text,
            Base::Float => Kind::Float,
            Base::List => Kind::List(Box::new(Kind::Text)),
        };
        out.push(KeyMeta {
            path: path.to_string(),
            kind: kind_for(path, base),
            default: None,
            help: help.to_string(),
            optional: *optional,
            read_only: false,
        });
    }
    for m in out.iter_mut().filter(|m| m.path == "runtime.state_dir") {
        m.help.push_str("\nread-only here: change it by hand; copy the folder first, or the service loses its state");
    }
    for m in out.iter_mut().filter(|m| m.path == "tui.theme") {
        m.help
            .push_str("\nApply the change, then restart the TUI to use the theme.");
    }
    out
}

static META: OnceLock<(Vec<KeyMeta>, HashMap<String, usize>)> = OnceLock::new();

fn table() -> &'static (Vec<KeyMeta>, HashMap<String, usize>) {
    META.get_or_init(|| {
        let all = build();
        let index = all
            .iter()
            .enumerate()
            .map(|(i, m)| (m.path.clone(), i))
            .collect();
        (all, index)
    })
}

/// All key metadata.
pub fn all() -> &'static [KeyMeta] {
    &table().0
}

/// The metadata of a path. A number segment (a list index) matches `*`.
pub fn find(path: &str) -> Option<&'static KeyMeta> {
    let norm: Vec<&str> = path
        .split('.')
        .map(|s| {
            if s.chars().all(|c| c.is_ascii_digit()) && !s.is_empty() {
                "*"
            } else {
                s
            }
        })
        .collect();
    let (all, index) = table();
    index.get(&norm.join(".")).map(|&i| &all[i])
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{Config, DEFAULTS_TOML};

    const ROUTER: &str = "[[routers]]\nname = \"R1\"\naddress = \"192.0.2.1\"\nusername = \"u\"\npassword_file = \"/x\"\n";

    /// A minimal config file that sets `path` to the TOML value `v`.
    fn file(path: &str, v: &str) -> String {
        let parts: Vec<&str> = path.split('.').collect();
        match parts.as_slice() {
            ["routers", "*", "dhcp", "servers", "*", k] => {
                let mode = if *k == "speed_sources" && v.contains("lease_comment") {
                    "mode = \"per_plan\"\n"
                } else {
                    ""
                };
                let line = if *k == "name" {
                    format!("name = {v}\n")
                } else {
                    format!("name = \"S\"\n{k} = {v}\n")
                };
                format!("{ROUTER}[[routers.dhcp.servers]]\n{line}{mode}")
            }
            ["routers", "*", "dhcp", "clients", "*", k] => {
                let mut fields = vec![
                    ("mac", "\"aa:bb:cc:dd:ee:ff\"".to_string()),
                    ("download_mbps", "5".into()),
                    ("upload_mbps", "5".into()),
                ];
                match fields.iter_mut().find(|(n, _)| n == k) {
                    Some(f) => f.1 = v.to_string(),
                    None => fields.push((k, v.to_string())),
                }
                let body: String = fields.iter().map(|(n, x)| format!("{n} = {x}\n")).collect();
                format!("{ROUTER}[[routers.dhcp.clients]]\n{body}")
            }
            ["routers", "*", k] => {
                let mut lines: Vec<String> = ROUTER.lines().map(str::to_string).collect();
                match lines.iter().position(|l| l.starts_with(&format!("{k} = "))) {
                    Some(i) => lines[i] = format!("{k} = {v}"),
                    None => lines.push(format!("{k} = {v}")),
                }
                lines.join("\n") + "\n"
            }
            ["routers", "*", t, k] => format!("{ROUTER}[routers.{t}]\n{k} = {v}\n"),
            _ => {
                let (t, k) = path.rsplit_once('.').unwrap();
                format!("[{t}]\n{k} = {v}\n{ROUTER}")
            }
        }
    }

    fn leaves(t: &toml::Table, prefix: &str, out: &mut Vec<String>) {
        for (k, v) in t {
            let p = if prefix.is_empty() {
                k.clone()
            } else {
                format!("{prefix}.{k}")
            };
            match v {
                toml::Value::Table(sub) => leaves(sub, &p, out),
                _ => out.push(p),
            }
        }
    }

    #[test]
    fn each_default_key_has_meta() {
        let t: toml::Table = toml::from_str(DEFAULTS_TOML).unwrap();
        let mut keys = Vec::new();
        leaves(&t, "", &mut keys);
        for k in keys.iter().filter(|k| {
            !matches!(
                k.as_str(),
                "router_defaults.dhcp.servers" | "router_defaults.dhcp.clients"
            )
        }) {
            assert!(find(k).is_some(), "{k}");
        }
        for k in [
            "routers.0.name",
            "routers.3.pppoe.enabled",
            "routers.1.dhcp.servers.2.mode",
            "routers.0.hotspot.download_limit_mbps",
            "nodes.ppp.type",
        ] {
            assert!(find(k).is_some(), "{k}");
        }
        assert!(find("nodes.client.type").is_some_and(|m| !m.optional));
        assert!(find("routers.*.dhcp.servers").is_none());
        assert!(
            find("routers.*.dhcp.clients").is_none()
                && find("router_defaults.dhcp.clients").is_none()
        );
        for k in [
            "routers.0.dhcp.clients.1.mac",
            "routers.0.dhcp.clients.0.download_mbps",
            "routers.0.dhcp.clients.0.name",
        ] {
            assert!(find(k).is_some(), "{k}");
        }
    }

    #[test]
    fn each_enum_choice_loads_and_a_wrong_value_fails() {
        let mut types = std::collections::HashSet::new();
        for (path, choices) in CHOICES {
            types.insert(choices.as_ptr() as usize);
            let m = find(path).unwrap_or_else(|| panic!("{path}"));
            let list = matches!(m.kind, Kind::List(_));
            for c in *choices {
                let v = if list {
                    format!("[\"{c}\"]")
                } else {
                    format!("\"{c}\"")
                };
                let r = Config::from_toml_str(&file(path, &v));
                if *c == "lease_comment" && *path == "speed.dhcp_per_site_sources" {
                    assert!(r.is_err(), "{path} {c}");
                } else {
                    assert!(r.is_ok(), "{path} = {v}: {:#}", r.unwrap_err());
                }
            }
            let v = if list { "[\"wrong\"]" } else { "\"wrong\"" };
            let e = format!("{:#}", Config::from_toml_str(&file(path, v)).unwrap_err());
            let last = path.rsplit('.').next().unwrap();
            assert!(e.contains(last), "{path}: {e}");
        }
        assert_eq!(types.len(), 11);
    }

    #[test]
    fn number_kinds_match_the_config_types() {
        for m in all()
            .iter()
            .filter(|m| matches!(m.kind, Kind::Int | Kind::Float))
        {
            let r = Config::from_toml_str(&file(&m.path, "1.5"));
            let type_error = r
                .as_ref()
                .err()
                .is_some_and(|e| format!("{e:#}").contains("invalid type"));
            assert_eq!(
                type_error,
                matches!(m.kind, Kind::Int),
                "{} {:?}",
                m.path,
                r.err().map(|e| format!("{e:#}"))
            );
        }
    }

    #[test]
    fn help_comes_from_the_comment() {
        assert!(
            find("runtime.mode")
                .unwrap()
                .help
                .contains("shadow | apply")
        );
        assert!(find("runtime.id_length").unwrap().help.contains("4 to 32"));
        assert!(!find("runtime.mode").unwrap().help.contains('#'));
        assert!(!find("routers.*.name").unwrap().help.is_empty());
    }

    #[test]
    fn state_dir_is_read_only() {
        let m = find("runtime.state_dir").unwrap();
        assert!(m.read_only && m.help.contains("copy the folder first"));
        assert!(!find("runtime.mode").unwrap().read_only);
    }

    #[test]
    fn router_keys_use_router_defaults() {
        assert_eq!(
            find("routers.*.port").unwrap().default.as_deref(),
            Some("8729")
        );
        assert_eq!(
            find("routers.2.port").unwrap().default.as_deref(),
            Some("8729")
        );
        assert!(find("router_defaults.port").is_some());
        assert_eq!(
            find("runtime.mode").unwrap().default.as_deref(),
            Some("\"shadow\"")
        );
        assert!(matches!(
            find("runtime.mode").unwrap().kind,
            Kind::Choice(_)
        ));
        assert!(matches!(
            find("safety.max_removed_percent").unwrap().kind,
            Kind::Float
        ));
        assert!(matches!(find("exclude.ips").unwrap().kind, Kind::List(_)));
    }
}
