use crate::{
    core_diagnostics::{Finding, Outcome},
    core_plan::Binding,
    core_routes::flag,
    routeros::{Inventory, Row},
};
use std::collections::HashMap;
pub(crate) struct Findings(pub Vec<Finding>);
impl Findings {
    pub fn add(
        &mut self,
        status: Outcome,
        code: &str,
        wan: &str,
        table: &str,
        row: &Row,
        message: impl Into<String>,
    ) {
        if self.0.len() < 256 {
            self.0.push(Finding {
                status,
                code: code.into(),
                wan: wan.into(),
                table: table.into(),
                row: row.get(".id").cloned().unwrap_or_default(),
                message: message.into().chars().take(512).collect(),
            });
        } else if self.0.len() == 256 {
            self.0.push(Finding {
                status: Outcome::Unsupported,
                code: "finding-limit".into(),
                wan: String::new(),
                table: String::new(),
                row: String::new(),
                message: "Additional policy findings omitted; full review is required".into(),
            });
        }
    }
}
pub(crate) fn analyze(
    inventory: &Inventory,
    bindings: &[&Binding],
    interfaces: &HashMap<&str, &str>,
) -> Vec<Finding> {
    let mut f = Findings(vec![]);
    let empty = Row::new();
    let rows = |path: &str| inventory.tables.get(path).map(Vec::as_slice).unwrap_or(&[]);
    let resource = rows("/system/resource/print");
    if resource.len() != 1
        || !resource[0].get("version").is_some_and(|v| {
            v.starts_with("7.") && v.as_bytes().get(2).is_some_and(u8::is_ascii_digit)
        })
    {
        f.add(
            Outcome::Unsupported,
            "routeros-version",
            "",
            "/system/resource/print",
            &empty,
            "Core route checks require RouterOS 7",
        );
    }
    let enabled = |r: &Row| flag(r, "disabled", false).map(|v| !v);
    for rule in rows("/routing/rule/print") {
        match enabled(rule) {
            Ok(false) => {}
            Ok(true) => f.add(
                Outcome::Unsupported,
                "routing-rule",
                "",
                "/routing/rule/print",
                rule,
                "Enabled routing rules need manual review",
            ),
            Err(_) => f.add(
                Outcome::Unavailable,
                "unknown-status",
                "",
                "/routing/rule/print",
                rule,
                "Unknown RouterOS flag: disabled",
            ),
        }
    }
    for rule in rows("/ip/firewall/filter/print") {
        match enabled(rule) {
            Ok(false) => {}
            Err(_) => f.add(
                Outcome::Unavailable,
                "unknown-status",
                "",
                "/ip/firewall/filter/print",
                rule,
                "Unknown RouterOS flag: disabled",
            ),
            Ok(true) => {
                if rule
                    .get("action")
                    .is_none_or(|a| a == "fasttrack-connection")
                {
                    f.add(
                        Outcome::Unsupported,
                        "fasttrack",
                        "",
                        "/ip/firewall/filter/print",
                        rule,
                        "FastTrack or unknown filter action needs manual review",
                    );
                } else if !rule.get("action").is_some_and(|a| {
                    [
                        "accept",
                        "drop",
                        "reject",
                        "tarpit",
                        "log",
                        "passthrough",
                        "jump",
                        "return",
                        "add-src-to-address-list",
                        "add-dst-to-address-list",
                    ]
                    .contains(&a.as_str())
                }) {
                    f.add(
                        Outcome::Unsupported,
                        "unknown-filter-action",
                        "",
                        "/ip/firewall/filter/print",
                        rule,
                        "Unknown filter action needs manual review",
                    );
                }
            }
        }
    }
    let by_list: HashMap<_, _> = bindings.iter().map(|b| (b.list.as_str(), *b)).collect();
    let by_table: HashMap<_, _> = bindings.iter().map(|b| (b.table.as_str(), *b)).collect();
    let mut found = HashMap::<&str, usize>::new();
    for rule in rows("/ip/firewall/mangle/print") {
        let path = "/ip/firewall/mangle/print";
        match enabled(rule) {
            Ok(false) => continue,
            Err(_) => {
                f.add(
                    Outcome::Unavailable,
                    "unknown-status",
                    "",
                    path,
                    rule,
                    "Unknown RouterOS flag: disabled",
                );
                continue;
            }
            _ => {}
        }
        if !matches!(flag(rule, "invalid", false), Ok(false)) {
            f.add(
                Outcome::Conflict,
                "invalid-rule",
                "",
                path,
                rule,
                "Invalid mangle rule needs review",
            );
        }
        if rule
            .get("action")
            .is_none_or(|a| a == "fasttrack-connection")
        {
            f.add(
                Outcome::Unsupported,
                "fasttrack",
                "",
                path,
                rule,
                "Mangle FastTrack or unknown action needs manual review",
            );
        }
        let Some(chain) = rule.get("chain") else {
            f.add(
                Outcome::Unsupported,
                "unknown-chain",
                "",
                path,
                rule,
                "Unknown mangle chain needs review",
            );
            continue;
        };
        if chain != "prerouting" {
            continue;
        }
        if !rule.get("action").is_some_and(|a| a == "mark-routing") {
            let code = if rule.contains_key("per-connection-classifier") {
                "pcc-policy"
            } else {
                "complex-prerouting"
            };
            f.add(
                Outcome::Unsupported,
                code,
                "",
                path,
                rule,
                "Complex prerouting action needs manual review",
            );
            continue;
        }
        let list = rule
            .get("src-address-list")
            .map(String::as_str)
            .unwrap_or("");
        let binding = by_list.get(list).copied();
        let wan = binding.map(|b| b.wan.as_str()).unwrap_or("");
        for (key, value) in rule {
            match key.as_str() {
                ".id" | "chain" | "action" | "src-address-list" | "new-routing-mark"
                | "passthrough" | "disabled" | "dynamic" | "invalid" | "comment" | "bytes"
                | "packets" | "log" | "log-prefix" => {}
                "dst-address-type" if value.is_empty() || value == "!local" => {}
                "src-address" | "dst-address" if value == "0.0.0.0/0" => {}
                _ => f.add(
                    Outcome::Unsupported,
                    if key == "per-connection-classifier" {
                        "pcc-policy"
                    } else {
                        "conditional-matcher"
                    },
                    wan,
                    path,
                    rule,
                    format!("Unsupported prerouting matcher '{key}' needs manual review"),
                ),
            }
        }
        let Some(binding) = binding else {
            f.add(
                Outcome::Unsupported,
                "unmapped-source-list",
                "",
                path,
                rule,
                "Unmapped or missing positive prerouting source list needs manual review",
            );
            continue;
        };
        *found.entry(list).or_default() += 1;
        if rule.get("new-routing-mark") != Some(&binding.table) {
            f.add(
                Outcome::Conflict,
                "wrong-routing-table",
                wan,
                path,
                rule,
                format!("Source list targets the wrong routing table: {list}"),
            );
        }
        if !matches!(flag(rule, "passthrough", true), Ok(false)) {
            f.add(
                Outcome::Conflict,
                "passthrough",
                wan,
                path,
                rule,
                format!("Source list routing mark requires passthrough=false: {list}"),
            );
        }
    }
    let mut tables = HashMap::<&str, Vec<&Row>>::new();
    for row in rows("/routing/table/print") {
        if let Some(name) = row.get("name") {
            tables.entry(name).or_default().push(row);
        }
    }
    let mut defaults = HashMap::<&str, Vec<&Row>>::new();
    for route in rows("/ip/route/print") {
        if !route.get("dst-address").is_some_and(|d| d == "0.0.0.0/0") {
            continue;
        }
        let Some(table) = route.get("routing-table") else {
            continue;
        };
        let Some(binding) = by_table.get(table.as_str()) else {
            continue;
        };
        match (flag(route, "active", false), flag(route, "disabled", false)) {
            (Ok(true), Ok(false)) => defaults.entry(table).or_default().push(route),
            (Err(_), _) | (_, Err(_)) => f.add(
                Outcome::Unavailable,
                "unknown-status",
                &binding.wan,
                "/ip/route/print",
                route,
                "Unknown RouterOS route active/disabled status",
            ),
            _ => {}
        }
    }
    for b in bindings {
        match found.get(b.list.as_str()).copied().unwrap_or(0) {
            0 => f.add(
                Outcome::Conflict,
                "missing-routing-mark",
                &b.wan,
                "/ip/firewall/mangle/print",
                &empty,
                format!("No direct list-to-table routing mark for {}", b.list),
            ),
            1 => {}
            _ => f.add(
                Outcome::Conflict,
                "duplicate-routing-mark",
                &b.wan,
                "/ip/firewall/mangle/print",
                &empty,
                format!("Duplicate direct routing mark for source list: {}", b.list),
            ),
        }
        let table = tables
            .get(b.table.as_str())
            .map(Vec::as_slice)
            .unwrap_or(&[]);
        if table.len() != 1
            || !matches!(crate::core_routes::fib(table[0]), Ok(true))
            || !matches!(flag(table[0], "disabled", false), Ok(false))
            || !matches!(flag(table[0], "invalid", false), Ok(false))
        {
            f.add(
                Outcome::Conflict,
                "table-fib",
                &b.wan,
                "/routing/table/print",
                &empty,
                format!(
                    "Routing table is missing, duplicated, disabled or has no FIB: {}",
                    b.table
                ),
            );
        }
        let routes = defaults
            .get(b.table.as_str())
            .map(Vec::as_slice)
            .unwrap_or(&[]);
        if routes.len() != 1 {
            f.add(
                Outcome::Conflict,
                "active-default",
                &b.wan,
                "/ip/route/print",
                &empty,
                format!("Routing table needs one active default route: {}", b.table),
            );
            continue;
        }
        let route = routes[0];
        if !matches!(flag(route, "blackhole", false), Ok(false))
            || !matches!(flag(route, "unreachable", false), Ok(false))
            || !matches!(flag(route, "prohibit", false), Ok(false))
            || route.get("type").is_some_and(|v| v != "unicast")
        {
            f.add(
                Outcome::Conflict,
                "discard-default",
                &b.wan,
                "/ip/route/print",
                route,
                "WAN default is a discard route",
            );
        }
        let gateway = route.get("immediate-gw").map(String::as_str).unwrap_or("");
        let interface = interfaces.get(b.wan.as_str()).copied().unwrap_or("");
        if interface.is_empty()
            || gateway.is_empty()
            || gateway.contains(',')
            || !matches!(flag(route, "ecmp", false), Ok(false))
            || !(gateway == interface
                || gateway
                    .rsplit_once('%')
                    .is_some_and(|(_, i)| i == interface))
        {
            f.add(
                Outcome::Conflict,
                "wrong-wan-default",
                &b.wan,
                "/ip/route/print",
                route,
                format!(
                    "WAN default is ambiguous or resolves through a different interface: {}",
                    b.wan
                ),
            );
        }
    }
    f.0
}
