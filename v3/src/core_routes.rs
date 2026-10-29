use crate::{
    core_plan::Binding,
    routeros::{Inventory, Row},
};
use anyhow::Result;
use std::collections::HashMap;
pub(crate) fn flag(row: &Row, key: &str, default: bool) -> Result<bool> {
    match row.get(key).map(String::as_str) {
        Some("true") => Ok(true),
        Some("false") => Ok(false),
        None => Ok(default),
        _ => anyhow::bail!("Unknown RouterOS flag: {key}"),
    }
}
pub(crate) fn fib(row: &Row) -> Result<bool> {
    // RouterOS exposes FIB as a presence-only switch, unlike boolean status flags.
    if row.get("fib").is_some_and(String::is_empty) {
        Ok(true)
    } else {
        flag(row, "fib", false)
    }
}
pub(crate) fn audit(
    inventory: &Inventory,
    bindings: &[&Binding],
    interfaces: &HashMap<&str, &str>,
) -> Result<()> {
    let findings = crate::core_analysis::analyze(inventory, bindings, interfaces);
    if let Some(finding) = findings.findings.first() {
        anyhow::bail!("{}", finding.message);
    }
    Ok(())
}
