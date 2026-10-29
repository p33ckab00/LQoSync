use crate::{
    core_diagnostics::Report,
    model::{Router, Settings},
    routeros::Inventory,
};
use std::collections::BTreeMap;
#[derive(Default)]
struct Aliases(BTreeMap<(String, String), String>);
impl Aliases {
    fn name(&mut self, kind: &str, value: &str) -> String {
        if value.is_empty() {
            return String::new();
        }
        if kind == "mark" && value == "no-mark" {
            return "no-mark".into();
        }
        if kind == "table" && value == "main" {
            return "main".into();
        }
        let (negated, value) = if kind == "list" {
            value
                .strip_prefix('!')
                .map(|s| ("!", s))
                .unwrap_or(("", value))
        } else {
            ("", value)
        };
        let key = (kind.to_string(), value.to_string());
        let n = self.0.len() + 1;
        let result = self
            .0
            .entry(key)
            .or_insert_with(|| format!("{kind}-{n:04}"));
        format!("{negated}{result}")
    }
    fn gateway(&mut self, value: &str) -> String {
        value
            .split(',')
            .map(|part| {
                if let Some((_, interface)) = part.rsplit_once('%') {
                    format!("203.0.113.1%{}", self.name("interface", interface))
                } else if part.parse::<std::net::Ipv4Addr>().is_ok() {
                    "203.0.113.1".into()
                } else {
                    self.name("interface", part)
                }
            })
            .collect::<Vec<_>>()
            .join(",")
    }
}
pub fn project(inventory: &Inventory) -> Inventory {
    let mut aliases = Aliases::default();
    let mut tables = BTreeMap::new();
    for path in crate::core_plan::CORE_TABLES
        .iter()
        .chain(["/interface/list/print", "/interface/list/member/print"].iter())
    {
        let Some(rows) = inventory.tables.get(*path) else {
            continue;
        };
        let mut projected = vec![];
        for (n, row) in rows.iter().enumerate() {
            let mut out = crate::routeros::Row::new();
            for (key, value) in row {
                let mut key = key.clone();
                let value = match key.as_str() {
                    "comment" | "log-prefix" | "bytes" | "packets" | "mac-address" => continue,
                    ".id" => format!("*{:X}", n + 1),
                    "version" => {
                        let parts: Vec<_> = value.split('.').take(2).collect();
                        if parts.len() == 2
                            && parts
                                .iter()
                                .all(|p| p.len() < 8 && p.bytes().all(|b| b.is_ascii_digit()))
                        {
                            parts.join(".")
                        } else {
                            "unknown".into()
                        }
                    }
                    "name" if *path == "/interface/print" => aliases.name("interface", value),
                    "name" if *path == "/routing/table/print" => aliases.name("table", value),
                    "name" if *path == "/interface/list/print" => aliases.name("list", value),
                    "include" | "exclude" => aliases.name("list", value),
                    "name" => continue,
                    "fib" if value.is_empty() => String::new(),
                    "disabled" | "invalid" | "dynamic" | "running" | "active" | "fib" | "ecmp"
                    | "blackhole" | "unreachable" | "prohibit" | "passthrough" | "log"
                    | "builtin" => {
                        if value == "true" || value == "false" {
                            value.clone()
                        } else {
                            "unknown".into()
                        }
                    }
                    "action" => {
                        if [
                            "accept",
                            "drop",
                            "reject",
                            "tarpit",
                            "log",
                            "passthrough",
                            "jump",
                            "return",
                            "mark-routing",
                            "mark-connection",
                            "mark-packet",
                            "change-mss",
                            "change-dscp",
                            "change-ttl",
                            "set-priority",
                            "add-src-to-address-list",
                            "add-dst-to-address-list",
                            "fasttrack-connection",
                            "route",
                        ]
                        .contains(&value.as_str())
                        {
                            value.clone()
                        } else {
                            "unknown".into()
                        }
                    }
                    "chain" => {
                        if ["prerouting", "postrouting", "input", "output", "forward"]
                            .contains(&value.as_str())
                        {
                            value.clone()
                        } else {
                            aliases.name("chain", value)
                        }
                    }
                    "new-routing-mark" | "routing-table" | "table" => aliases.name("table", value),
                    "list" | "src-address-list" | "dst-address-list" | "address-list"
                    | "in-interface-list" | "out-interface-list" => aliases.name("list", value),
                    "in-interface" | "out-interface" | "interface" => {
                        aliases.name("interface", value)
                    }
                    "immediate-gw" | "gateway" => aliases.gateway(value),
                    "dst-address-type" => {
                        if ["", "!local", "local", "unicast", "!unicast"].contains(&value.as_str())
                        {
                            value.clone()
                        } else {
                            "redacted".into()
                        }
                    }
                    "address" | "src-address" | "dst-address" => {
                        if value == "0.0.0.0/0" {
                            value.clone()
                        } else if value.contains('-') {
                            "198.18.0.1-198.18.0.10".into()
                        } else if let Some((_, prefix)) = value.split_once('/') {
                            let prefix =
                                prefix.parse::<u8>().ok().filter(|p| *p <= 32).unwrap_or(32);
                            format!("198.18.0.{}/{prefix}", n % 250 + 1)
                        } else if value.parse::<std::net::Ipv4Addr>().is_ok() {
                            format!("198.18.0.{}", n % 250 + 1)
                        } else {
                            "redacted.invalid".into()
                        }
                    }
                    "connection-mark"
                    | "new-connection-mark"
                    | "new-packet-mark"
                    | "packet-mark" => aliases.name("mark", value),
                    "type" => {
                        if ["unicast", "blackhole", "unreachable", "prohibit"]
                            .contains(&value.as_str())
                        {
                            value.clone()
                        } else {
                            "unknown".into()
                        }
                    }
                    "connection-state" => {
                        if value.split(',').all(|s| {
                            ["new", "established", "related", "invalid", "untracked"].contains(&s)
                        }) {
                            value.clone()
                        } else {
                            "redacted".into()
                        }
                    }
                    "protocol" if ["ospf", "89", "udp", "17"].contains(&value.as_str()) => {
                        value.clone()
                    }
                    "dst-port" if ["3784,3785", "3785,3784"].contains(&value.as_str()) => {
                        value.clone()
                    }
                    "timeout" if value.is_empty() || value == "0s" => value.clone(),
                    "protocol"
                    | "src-port"
                    | "dst-port"
                    | "tcp-flags"
                    | "per-connection-classifier"
                    | "new-mss"
                    | "jump-target"
                    | "limit"
                    | "timeout"
                    | "distance" => "redacted".into(),
                    _ => {
                        if *path == "/system/resource/print" || *path == "/interface/print" {
                            continue;
                        }
                        key = aliases.name("unknown-field", &key);
                        "redacted".into()
                    }
                };
                out.insert(key, value);
            }
            projected.push(out);
        }
        tables.insert((*path).into(), projected);
    }
    Inventory {
        router_id: "fixture-core".into(),
        collected_at: 100,
        tables,
        errors: inventory
            .errors
            .keys()
            .filter(|k| {
                k.as_str() == "collection"
                    || crate::core_plan::CORE_TABLES.contains(&k.as_str())
                    || ["/interface/list/print", "/interface/list/member/print"]
                        .contains(&k.as_str())
            })
            .map(|k| (k.clone(), "redacted table error".into()))
            .collect(),
    }
}
pub fn settings() -> Settings {
    Settings {
        routers: vec![Router {
            id: "fixture-core".into(),
            name: "Fixture core".into(),
            host: "127.0.0.1".into(),
            port: 1,
            username: "fixture".into(),
            password_env: "NOT_SET".into(),
            role: "core".into(),
            tls: false,
            tls_server_name: String::new(),
            tls_ca_file: String::new(),
            timeout_seconds: 1,
        }],
        ..Default::default()
    }
}
pub fn signature(report: &Report) -> BTreeMap<String, usize> {
    let mut codes = BTreeMap::new();
    for c in &report.cores {
        for f in &c.findings {
            *codes.entry(f.code.clone()).or_default() += 1;
        }
    }
    codes
}
#[derive(serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Fixture {
    pub scope: String,
    pub expected_codes: BTreeMap<String, usize>,
    #[serde(default)]
    pub expected_paths: Vec<(
        crate::core_connections::Direction,
        crate::core_diagnostics::Outcome,
    )>,
    #[serde(default)]
    pub expected_scopes: Vec<(usize, Vec<String>)>,
    pub inventory: Inventory,
}
pub fn fixture(inventory: &Inventory) -> anyhow::Result<Fixture> {
    let settings = settings();
    let mut baseline = inventory.clone();
    baseline.router_id = "fixture-core".into();
    let before = crate::core_diagnostics::review(
        &settings,
        &[(baseline.router_id.clone(), std::sync::Arc::new(baseline))]
            .into_iter()
            .collect(),
        &BTreeMap::new(),
        inventory.collected_at,
        crate::core_diagnostics::Origin::Saved,
    );
    let projected = project(inventory);
    let after = crate::core_diagnostics::review(
        &settings,
        &[(
            projected.router_id.clone(),
            std::sync::Arc::new(projected.clone()),
        )]
        .into_iter()
        .collect(),
        &BTreeMap::new(),
        101,
        crate::core_diagnostics::Origin::Saved,
    );
    let paths = |r: &Report| {
        r.cores
            .iter()
            .flat_map(|c| c.connection_paths.iter().map(|p| (p.direction, p.status)))
            .collect::<Vec<_>>()
    };
    let expected_paths = paths(&before);
    let scopes = |r: &Report| {
        r.cores
            .iter()
            .flat_map(|c| {
                c.forward_scopes
                    .iter()
                    .map(|s| (s.ingress_interfaces.len(), s.excluded_protocols.clone()))
            })
            .collect::<Vec<_>>()
    };
    let expected_scopes = scopes(&before);
    anyhow::ensure!(
        expected_scopes == scopes(&after),
        "Sanitized routing projection changed forwarding scopes; do not export this fixture"
    );
    anyhow::ensure!(
        expected_paths == paths(&after),
        "Sanitized routing projection changed connection-path results; do not export this fixture"
    );
    let expected_codes = signature(&before);
    anyhow::ensure!(
        expected_codes == signature(&after),
        "Sanitized routing projection changed diagnostic codes; do not export this fixture"
    );
    Ok(Fixture{scope:"Actual-policy routing-field projection. Address containment and packet behavior are not replayed.".into(),expected_codes,expected_paths,expected_scopes,inventory:projected})
}
