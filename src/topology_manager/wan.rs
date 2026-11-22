//! WAN discovery. Interfaces are candidates for the operator to confirm, not proof of an Internet WAN.

use super::catalog::{SourceOrigin, TopologyCatalog};
use super::policy_draft::router_id;
use super::reader::Table;
use crate::config::Config;
use crate::model::RosRow;
use crate::routeros::{Connector, RouterSession};
use serde_json::{Value, json};
use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::sync::Arc;
use std::time::Duration;
use tokio::task::JoinSet;
use tokio::time::{Instant, timeout_at};

/// The `/interface` read of one configured router.
pub struct RouterInterfaces {
    pub router_id: String,
    pub name: String,
    pub read: Result<Vec<RosRow>, String>,
    /// The `/ip/route` rows. A failed route read gives no rows: no interface is a default route.
    pub routes: Vec<RosRow>,
}

/// The interfaces of the active `0.0.0.0/0` routes, in any routing table. The gateway is an
/// interface name, or `address%interface` in `immediate-gw`.
pub fn default_route_interfaces(routes: &[RosRow]) -> BTreeSet<String> {
    let field = |row: &RosRow, k: &str| row.get(k).map_or("", String::as_str).to_string();
    routes
        .iter()
        .filter(|r| field(r, "dst-address") == "0.0.0.0/0" && field(r, "active") == "true")
        .flat_map(|r| [field(r, "gateway"), field(r, "immediate-gw")])
        .filter_map(|gw| match gw.split_once('%') {
            Some((_, iface)) => Some(iface.to_string()),
            None => (!gw.is_empty() && gw.parse::<std::net::IpAddr>().is_err()).then_some(gw),
        })
        .collect()
}

/// Read `/interface` once from every configured router at the same time, with one deadline.
/// A router that is not done at the deadline gets "read expired".
pub async fn read_all<C: Connector + 'static>(
    connector: Arc<C>,
    cfg: &Config,
    limit: Duration,
    only: &[String],
) -> Vec<RouterInterfaces>
where
    C::Session: 'static,
{
    let now = Instant::now();
    let deadline = now.checked_add(limit).unwrap_or(now);
    let (path, fields) = Table::Interfaces.command();
    let mut tasks = JoinSet::new();
    // A disabled router gets no discovery read.
    for (i, router) in cfg
        .routers
        .iter()
        .cloned()
        .enumerate()
        .filter(|(_, r)| r.enabled && (only.is_empty() || only.contains(&router_id(&r.name))))
    {
        let (connector, access) = (connector.clone(), cfg.router_access.clone());
        tasks.spawn(async move {
            let read = async {
                let mut session = connector
                    .connect(&router, &access)
                    .await
                    .map_err(|e| e.to_string())?;
                let interfaces = session
                    .print(path, fields)
                    .await
                    .map_err(|e| e.to_string())?;
                let (path, fields) = Table::Routes.command();
                let routes = session.print(path, fields).await.unwrap_or_default();
                Ok::<_, String>((interfaces, routes))
            };
            let out = timeout_at(deadline, read)
                .await
                .unwrap_or_else(|_| Err("read expired".into()));
            (i, out)
        });
    }
    let mut out: Vec<_> = cfg
        .routers
        .iter()
        .map(|r| RouterInterfaces {
            router_id: router_id(&r.name),
            name: r.name.clone(),
            routes: Vec::new(),
            read: Err(if r.enabled {
                "the read task stopped".into()
            } else {
                "the router is disabled; no discovery read".into()
            }),
        })
        .collect();
    while let Some(done) = tasks.join_next().await {
        if let Ok((i, read)) = done {
            match read {
                Ok((interfaces, routes)) => {
                    out[i].read = Ok(interfaces);
                    out[i].routes = routes;
                }
                Err(e) => out[i].read = Err(e),
            }
        }
    }
    // An unselected router is neither read nor listed.
    out.retain(|r| only.is_empty() || only.contains(&r.router_id));
    out
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
            let defaults = default_route_interfaces(&r.routes);
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
                        "default_route": defaults.contains(name),
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
