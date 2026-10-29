use crate::{
    core_analysis::Findings,
    core_diagnostics::Outcome,
    core_plan::Binding,
    core_routes::flag,
    routeros::{Inventory, Row},
};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ForwardScope {
    pub wan: String,
    pub source_list: String,
    pub ingress_list: String,
    pub ingress_interfaces: Vec<String>,
    pub excluded_destination_list: String,
    pub excluded_protocols: Vec<String>,
}
pub(crate) struct Analysis {
    pub scopes: Vec<ForwardScope>,
    pub routes: HashSet<usize>,
    pub bypasses: HashSet<usize>,
    pub findings: Vec<crate::core_diagnostics::Finding>,
}
fn healthy(row: &Row) -> bool {
    ["disabled", "dynamic", "invalid"]
        .iter()
        .all(|k| matches!(flag(row, k, false), Ok(false)))
}
fn fields(row: &Row, specific: &[&str]) -> bool {
    healthy(row)
        && row.keys().all(|k| {
            [
                ".id",
                "chain",
                "action",
                "disabled",
                "dynamic",
                "invalid",
                "comment",
                "bytes",
                "packets",
                "log",
                "log-prefix",
            ]
            .contains(&k.as_str())
                || specific.contains(&k.as_str())
        })
}
fn positive(value: &str) -> bool {
    !value.is_empty()
        && !value.starts_with('!')
        && value.trim() == value
        && !value.chars().any(char::is_control)
}
fn rows<'a>(inv: &'a Inventory, path: &str) -> &'a [Row] {
    inv.tables.get(path).map(Vec::as_slice).unwrap_or(&[])
}
fn table<'a>(inv: &'a Inventory, path: &str) -> Result<&'a [Row], String> {
    if inv.errors.contains_key("collection")
        || inv.errors.contains_key(path)
        || !inv.tables.contains_key(path)
    {
        return Err(format!("Scope evidence is missing or incomplete: {path}"));
    }
    Ok(rows(inv, path))
}
type RowIndex<'a> = HashMap<&'a str, Vec<&'a Row>>;
fn index<'a>(inv: &'a Inventory, path: &str, key: &str) -> RowIndex<'a> {
    let mut out = HashMap::new();
    for row in rows(inv, path) {
        if let Some(name) = row.get(key) {
            out.entry(name.as_str()).or_insert_with(Vec::new).push(row);
        }
    }
    out
}
struct Evidence<'a> {
    definitions: RowIndex<'a>,
    members: RowIndex<'a>,
    interfaces: RowIndex<'a>,
    addresses: RowIndex<'a>,
    wan_interfaces: HashSet<&'a str>,
    writers: HashSet<&'a str>,
    unknown_writer: bool,
}
impl<'a> Evidence<'a> {
    fn new(inv: &'a Inventory, wans: &HashMap<&str, &'a str>) -> Self {
        let mut result = Self {
            definitions: index(inv, "/interface/list/print", "name"),
            members: index(inv, "/interface/list/member/print", "list"),
            interfaces: index(inv, "/interface/print", "name"),
            addresses: index(inv, "/ip/firewall/address-list/print", "list"),
            wan_interfaces: wans.values().copied().collect(),
            writers: HashSet::new(),
            unknown_writer: false,
        };
        for row in rows(inv, "/ip/firewall/filter/print")
            .iter()
            .chain(rows(inv, "/ip/firewall/mangle/print"))
        {
            if matches!(flag(row, "disabled", false), Ok(true)) {
                continue;
            }
            if row.get("action").is_some_and(|v| {
                ["add-src-to-address-list", "add-dst-to-address-list"].contains(&v.as_str())
            }) {
                match row.get("address-list").filter(|v| positive(v)) {
                    Some(name) => {
                        result.writers.insert(name);
                    }
                    None => result.unknown_writer = true,
                }
            }
        }
        result
    }
}
fn ingress(inv: &Inventory, name: &str, evidence: &Evidence<'_>) -> Result<Vec<String>, String> {
    if !positive(name) || ["all", "none", "dynamic", "static"].contains(&name) {
        return Err("Ingress needs an explicit static interface list".into());
    }
    table(inv, "/interface/list/print")?;
    let definitions = evidence
        .definitions
        .get(name)
        .map(Vec::as_slice)
        .unwrap_or(&[]);
    if definitions.len() != 1
        || !fields(definitions[0], &["name", "include", "exclude", "builtin"])
        || !matches!(flag(definitions[0], "builtin", false), Ok(false))
        || ["include", "exclude"]
            .iter()
            .any(|k| definitions[0].get(*k).is_some_and(|v| !v.is_empty()))
    {
        return Err("Ingress list is missing, ambiguous, builtin, dynamic, or composed".into());
    }
    table(inv, "/interface/print")?;
    table(inv, "/interface/list/member/print")?;
    let mut result = vec![];
    let mut seen = HashSet::new();
    for member in evidence.members.get(name).into_iter().flatten() {
        if matches!(flag(member, "disabled", false), Ok(true)) {
            continue;
        }
        let interface = member
            .get("interface")
            .filter(|v| positive(v))
            .ok_or("Ingress member has no exact interface")?;
        if !fields(member, &["list", "interface"])
            || !seen.insert(interface)
            || evidence.wan_interfaces.contains(interface.as_str())
        {
            return Err(
                "Ingress membership is dynamic, ambiguous, or includes a mapped WAN".into(),
            );
        }
        let matches = evidence
            .interfaces
            .get(interface.as_str())
            .map(Vec::as_slice)
            .unwrap_or(&[]);
        if matches.len() != 1
            || !healthy(matches[0])
            || !matches!(flag(matches[0], "running", false), Ok(true))
        {
            return Err("Ingress interface is missing, duplicated, disabled, or down".into());
        }
        result.push(interface.clone());
    }
    if result.is_empty() {
        return Err("Ingress list has no enabled static members".into());
    }
    result.sort();
    Ok(result)
}
fn destinations(inv: &Inventory, name: &str, evidence: &Evidence<'_>) -> Result<(), String> {
    if !positive(name) {
        return Err("Destination exclusion needs an exact positive list name".into());
    }
    let mut count = 0;
    let mut spans = vec![];
    table(inv, "/ip/firewall/address-list/print")?;
    table(inv, "/ip/firewall/filter/print")?;
    table(inv, "/ip/firewall/mangle/print")?;
    if evidence.unknown_writer || evidence.writers.contains(name) {
        return Err(
            "Active or unknown-status firewall writers can mutate the destination exclusion list"
                .into(),
        );
    }
    for r in evidence.addresses.get(name).into_iter().flatten() {
        if matches!(flag(r, "disabled", false), Ok(true)) {
            continue;
        }
        if !fields(r, &["list", "address", "timeout"])
            || r.get("timeout").is_some_and(|v| !v.is_empty() && v != "0s")
        {
            return Err("Destination exclusions must be static and have no timeout".into());
        }
        let range = r
            .get("address")
            .and_then(|v| crate::core_plan::span(v).ok())
            .ok_or("Destination exclusions need explicit IPv4 hosts, CIDRs, or ranges")?;
        if range == (0, u32::MAX) {
            return Err("Destination exclusion covers every IPv4 destination".into());
        }
        spans.push(range);
        count += 1;
    }
    if count == 0 {
        return Err("Destination exclusion list has no enabled entries".into());
    }
    spans.sort_unstable();
    let mut end = spans[0].1;
    if spans[0].0 == 0 {
        for &(next, last) in &spans[1..] {
            if next > end.saturating_add(1) {
                break;
            }
            end = end.max(last);
        }
        if end == u32::MAX {
            return Err("Destination exclusions jointly cover every IPv4 destination".into());
        }
    }
    Ok(())
}
pub(crate) fn analyze(
    inv: &Inventory,
    bindings: &[&Binding],
    wans: &HashMap<&str, &str>,
) -> Analysis {
    let rules = rows(inv, "/ip/firewall/mangle/print");
    let mut out = Analysis {
        scopes: vec![],
        routes: HashSet::new(),
        bypasses: HashSet::new(),
        findings: vec![],
    };
    let mut findings = Findings(vec![]);
    let by_list: HashMap<_, _> = bindings.iter().map(|b| (b.list.as_str(), *b)).collect();
    let direct: Vec<_> = rules
        .iter()
        .enumerate()
        .filter(|(_, r)| {
            r.get("chain").is_some_and(|v| v == "prerouting")
                && r.get("action").is_some_and(|v| v == "mark-routing")
                && !matches!(flag(r, "disabled", false), Ok(true))
        })
        .collect();
    let candidate_count = direct
        .iter()
        .filter(|(_, r)| r.contains_key("in-interface-list") && r.contains_key("dst-address-list"))
        .take(513)
        .count();
    if candidate_count == 0 {
        return out;
    }
    if candidate_count > 512 {
        findings.add(
            Outcome::Unsupported,
            "scope-limit",
            "",
            "/ip/firewall/mangle/print",
            &Row::new(),
            "Forwarding scope candidate limit reached; review is incomplete",
        );
    }
    let evidence = Evidence::new(inv, wans);
    let mut bypasses = HashMap::<&str, Vec<usize>>::new();
    for (n, r) in rules.iter().enumerate() {
        if r.get("chain").is_some_and(|v| v == "prerouting")
            && r.get("action").is_some_and(|v| v == "accept")
            && fields(r, &["dst-address-list"])
            && let Some(name) = r.get("dst-address-list").filter(|v| positive(v))
        {
            bypasses.entry(name).or_default().push(n);
        }
    }
    let mut ingress_cache = HashMap::new();
    let mut destination_cache = HashMap::new();
    let mut processed = 0;
    for &(n, r) in &direct {
        let (Some(input), Some(dst)) = (r.get("in-interface-list"), r.get("dst-address-list"))
        else {
            continue;
        };
        if processed >= 512 {
            break;
        }
        processed += 1;
        let binding = r
            .get("src-address-list")
            .and_then(|v| by_list.get(v.as_str()))
            .copied();
        let result = (|| -> Result<ForwardScope, String> {
            let binding =
                binding.ok_or("Conditional routing source list is not explicitly bound")?;
            let excluded = dst
                .strip_prefix('!')
                .filter(|v| positive(v))
                .ok_or("Routing scope must exclude a positive destination list")?;
            if !healthy(r) || r.get("dst-address-type").is_none_or(|v| v != "!local") {
                return Err(
                    "Conditional routing needs known flags and dst-address-type=!local".into(),
                );
            }
            let interfaces = ingress_cache
                .entry(input.clone())
                .or_insert_with(|| ingress(inv, input, &evidence))
                .clone()?;
            destination_cache
                .entry(excluded.to_string())
                .or_insert_with(|| destinations(inv, excluded, &evidence))
                .clone()?;
            if !bypasses
                .get(excluded)
                .is_some_and(|rows| rows.len() == 1 && rows[0] < n)
            {
                return Err(
                    "Routing exclusion needs one matching earlier destination-list bypass".into(),
                );
            }
            Ok(ForwardScope {
                wan: binding.wan.clone(),
                source_list: binding.list.clone(),
                ingress_list: input.clone(),
                ingress_interfaces: interfaces,
                excluded_destination_list: excluded.into(),
                excluded_protocols: vec![],
            })
        })();
        match result {
            Ok(scope) => {
                out.routes.insert(n);
                out.scopes.push(scope);
            }
            Err(reason) => findings.add(
                Outcome::Unsupported,
                "forward-scope",
                binding.map(|b| b.wan.as_str()).unwrap_or(""),
                "/ip/firewall/mangle/print",
                r,
                reason,
            ),
        }
    }
    // Bypasses affect the whole prerouting chain. Admit them only with a shared, fully checked forwarding scope.
    if !direct.is_empty()
        && direct.iter().all(|(n, _)| out.routes.contains(n))
        && let Some(first) = out.scopes.first()
        && out
            .scopes
            .iter()
            .all(|s| s.excluded_destination_list == first.excluded_destination_list)
    {
        let first_route = direct[0].0;
        let mut excluded = vec![];
        for (n, r) in rules.iter().enumerate().take(first_route) {
            if r.get("chain").is_none_or(|v| v != "prerouting")
                || r.get("action").is_none_or(|v| v != "accept")
            {
                continue;
            }
            if fields(r, &["dst-address-list"])
                && r.get("dst-address-list")
                    .is_some_and(|v| v == &first.excluded_destination_list)
            {
                out.bypasses.insert(n);
            } else if fields(r, &["protocol"])
                && r.get("protocol").is_some_and(|v| v == "ospf" || v == "89")
            {
                out.bypasses.insert(n);
                excluded.push("OSPF (IP protocol 89)".into());
            } else if fields(r, &["protocol", "dst-port"])
                && r.get("protocol").is_some_and(|v| v == "udp" || v == "17")
                && r.get("dst-port")
                    .is_some_and(|v| v == "3784,3785" || v == "3785,3784")
            {
                out.bypasses.insert(n);
                excluded.push("UDP destination ports 3784,3785".into());
            }
        }
        for scope in &mut out.scopes {
            scope.excluded_protocols = excluded.clone();
        }
    }
    out.findings = findings.0;
    out
}
