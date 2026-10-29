use crate::{
    core_plan::Binding,
    routeros::{Inventory, Row},
};
use anyhow::{Context, Result, ensure};
use std::collections::{HashMap, HashSet};
pub(crate) fn flag(row: &Row, key: &str, default: bool) -> Result<bool> {
    match row.get(key).map(String::as_str) {
        Some("true") => Ok(true),
        Some("false") => Ok(false),
        None => Ok(default),
        _ => anyhow::bail!("Unknown RouterOS flag: {key}"),
    }
}
pub(crate) fn audit(
    inventory: &Inventory,
    bindings: &[&Binding],
    interfaces: &HashMap<&str, &str>,
) -> Result<()> {
    let resource = &inventory.tables["/system/resource/print"];
    ensure!(
        resource.len() == 1
            && resource[0].get("version").is_some_and(
                |v| v.starts_with("7.") && v.as_bytes().get(2).is_some_and(u8::is_ascii_digit)
            ),
        "Core route checks require RouterOS 7"
    );
    for rule in &inventory.tables["/routing/rule/print"] {
        ensure!(
            flag(rule, "disabled", false)?,
            "Enabled routing rules need manual review"
        );
    }
    for rule in &inventory.tables["/ip/firewall/filter/print"] {
        if !flag(rule, "disabled", false)? {
            ensure!(
                rule.get("action")
                    .is_some_and(|a| a != "fasttrack-connection"),
                "FastTrack or unknown filter action needs manual review"
            );
        }
    }
    let by_list: HashMap<_, _> = bindings.iter().map(|b| (b.list.as_str(), *b)).collect();
    let mut found = HashSet::new();
    for rule in &inventory.tables["/ip/firewall/mangle/print"] {
        if flag(rule, "disabled", false)? {
            continue;
        }
        ensure!(
            !flag(rule, "invalid", false)?,
            "Invalid mangle rule needs review"
        );
        let chain = rule
            .get("chain")
            .context("Unknown mangle chain needs review")?;
        ensure!(
            rule.get("action")
                .is_some_and(|a| a != "fasttrack-connection"),
            "Mangle FastTrack needs manual review"
        );
        if chain != "prerouting" {
            continue;
        }
        ensure!(
            rule.get("action").is_some_and(|a| a == "mark-routing"),
            "Complex prerouting action needs manual review"
        );
        for (key, value) in rule {
            match key.as_str() {
                ".id" | "chain" | "action" | "src-address-list" | "new-routing-mark"
                | "passthrough" | "disabled" | "dynamic" | "invalid" | "comment" | "bytes"
                | "packets" | "log" | "log-prefix" => {}
                "dst-address-type" if value.is_empty() || value == "!local" => {}
                "src-address" | "dst-address" if value == "0.0.0.0/0" => {}
                _ => anyhow::bail!("Unsupported prerouting matcher '{key}' needs manual review"),
            }
        }
        let list = rule
            .get("src-address-list")
            .context("Missing positive source list in routing mark")?;
        let binding = by_list
            .get(list.as_str())
            .context("Unmapped prerouting source list needs manual review")?;
        ensure!(
            rule.get("new-routing-mark") == Some(&binding.table),
            "Source list targets the wrong routing table: {list}"
        );
        ensure!(
            !flag(rule, "passthrough", true)?,
            "Source list routing mark requires passthrough=false: {list}"
        );
        ensure!(
            found.insert(list.as_str()),
            "Duplicate direct routing mark for source list: {list}"
        );
    }
    for binding in bindings {
        ensure!(
            found.contains(binding.list.as_str()),
            "No direct list-to-table routing mark for {}",
            binding.list
        );
        let tables: Vec<_> = inventory.tables["/routing/table/print"]
            .iter()
            .filter(|r| r.get("name") == Some(&binding.table))
            .collect();
        ensure!(
            tables.len() == 1
                && flag(tables[0], "fib", false)?
                && !flag(tables[0], "disabled", false)?
                && !flag(tables[0], "invalid", false)?,
            "Routing table is missing, duplicated, disabled or has no FIB: {}",
            binding.table
        );
        let mut defaults = vec![];
        for route in &inventory.tables["/ip/route/print"] {
            if route.get("routing-table") == Some(&binding.table)
                && route.get("dst-address").is_some_and(|d| d == "0.0.0.0/0")
                && flag(route, "active", false)?
                && !flag(route, "disabled", false)?
            {
                defaults.push(route);
            }
        }
        ensure!(
            defaults.len() == 1,
            "Routing table needs one active default route: {}",
            binding.table
        );
        let route = defaults[0];
        ensure!(
            !flag(route, "blackhole", false)?
                && !flag(route, "unreachable", false)?
                && !flag(route, "prohibit", false)?
                && route.get("type").is_none_or(|v| v == "unicast"),
            "WAN default is a discard route"
        );
        let gateway = route
            .get("immediate-gw")
            .context("WAN default has no resolved immediate gateway")?;
        let interface = interfaces[binding.wan.as_str()];
        ensure!(
            !gateway.contains(',')
                && !flag(route, "ecmp", false)?
                && (gateway == interface
                    || gateway
                        .rsplit_once('%')
                        .is_some_and(|(_, i)| i == interface)),
            "WAN default is ambiguous or resolves through a different interface: {}",
            binding.wan
        );
    }
    Ok(())
}
