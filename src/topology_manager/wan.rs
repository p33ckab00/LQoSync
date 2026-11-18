//! WAN discovery. Interfaces are candidates for the operator to confirm, not proof of an Internet WAN.

use super::catalog::{SourceOrigin, TopologyCatalog};
use crate::model::RosRow;
use serde_json::{Value, json};
use std::collections::{BTreeMap, BTreeSet, HashMap};

/// The `/interface` read of one configured router.
pub struct RouterInterfaces {
    pub router_id: String,
    pub name: String,
    pub read: Result<Vec<RosRow>, String>,
}

/// A catalog WAN source with the configured router name of its catalog router.
struct WanSource<'a> {
    id: &'a str,
    router_id: &'a str,
    router_name: Option<&'a str>,
    endpoint: &'a str,
}

/// The `wan.discover` answer. Catalog routers match configured routers by name.
pub fn answer(
    reads: &[RouterInterfaces],
    catalog: Option<&TopologyCatalog>,
    collected_at: u64,
) -> Value {
    let mut wans = Vec::new();
    let mut used_by: HashMap<&str, BTreeSet<&str>> = HashMap::new();
    if let Some(catalog) = catalog {
        let router_names: HashMap<&str, &str> = catalog
            .routers
            .iter()
            .map(|r| (r.id.as_str(), r.name.as_str()))
            .collect();
        for source in &catalog.sources {
            if let SourceOrigin::Wan {
                router_id,
                endpoint_ref,
            } = &source.origin
            {
                wans.push(WanSource {
                    id: &source.id,
                    router_id,
                    router_name: router_names.get(router_id.as_str()).copied(),
                    endpoint: endpoint_ref,
                });
            }
        }
        for node in &catalog.nodes {
            if let Some(source) = node.source_id.as_deref() {
                used_by.entry(source).or_default().insert(&node.id);
            }
        }
    }
    let routers: Vec<Value> = reads
        .iter()
        .map(|r| {
            let rows = match &r.read {
                Ok(rows) => rows,
                Err(e) => {
                    return json!({
                        "router_id": r.router_id, "name": r.name, "state": "failed",
                        "error": e, "candidates": [], "missing": [],
                    });
                }
            };
            let mine: Vec<_> = wans
                .iter()
                .filter(|w| w.router_name == Some(r.name.as_str()))
                .collect();
            // Server-side dynamic tunnels (`pppoe-in` and others) are subscriber sessions.
            let mut found: BTreeMap<(bool, &str), Value> = BTreeMap::new();
            for row in rows {
                let field = |k: &str| row.get(k).map_or("", String::as_str);
                let (name, kind) = (field("name"), field("type"));
                if name.is_empty() || kind.ends_with("-in") {
                    continue;
                }
                let suggested = kind == "pppoe-out";
                let source = mine.iter().find(|w| w.endpoint == name).map(|w| w.id);
                let users: Vec<&str> = source
                    .and_then(|s| used_by.get(s))
                    .map_or_else(Vec::new, |u| u.iter().copied().collect());
                found.insert(
                    (!suggested, name),
                    json!({
                        "name": name, "type": kind,
                        "running": field("running") == "true",
                        "disabled": field("disabled") == "true",
                        "comment": row.get("comment"),
                        "suggested": suggested, "source_id": source, "used_by": users,
                    }),
                );
            }
            let present: BTreeSet<&str> = found.keys().map(|(_, n)| *n).collect();
            let missing: Vec<Value> = mine
                .iter()
                .filter(|w| !present.contains(w.endpoint))
                .map(|w| json!({"source_id": w.id, "endpoint_ref": w.endpoint}))
                .collect();
            json!({
                "router_id": r.router_id, "name": r.name, "state": "ok", "error": null,
                "candidates": found.into_values().collect::<Vec<_>>(), "missing": missing,
            })
        })
        .collect();
    let configured: BTreeSet<&str> = reads.iter().map(|r| r.name.as_str()).collect();
    let unconfigured: Vec<Value> = wans
        .iter()
        .filter(|w| w.router_name.is_none_or(|n| !configured.contains(n)))
        .map(|w| json!({"source_id": w.id, "router_id": w.router_id, "endpoint_ref": w.endpoint}))
        .collect();
    json!({"collected_at": collected_at, "routers": routers, "unconfigured": unconfigured})
}

#[cfg(test)]
#[path = "wan_tests.rs"]
mod tests;
