use crate::{model::Settings, routeros::Inventory};
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, HashMap},
    path::Path,
    sync::Arc,
};
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Outcome {
    Pass,
    Unsupported,
    Conflict,
    Unavailable,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Origin {
    Live,
    Saved,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Health {
    Fresh,
    Historical,
    Stale,
    Missing,
    Failed,
    Partial,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Finding {
    pub status: Outcome,
    pub code: String,
    pub wan: String,
    pub table: String,
    pub row: String,
    pub message: String,
}
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Counts {
    pub address_members: usize,
    pub mangle_rules: usize,
    pub pcc_rules: usize,
    pub fasttrack_rules: usize,
    pub routes: usize,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WanReview {
    pub wan: String,
    pub table: String,
    pub list: String,
    pub interface: String,
    pub mapped: bool,
    pub status: Outcome,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CoreReview {
    pub router: String,
    pub collected_at: Option<u64>,
    pub health: Health,
    pub counts: Counts,
    pub findings: Vec<Finding>,
    pub wans: Vec<WanReview>,
    pub observed_paths_omitted: usize,
    pub connection_paths: Vec<crate::core_connections::ConnectionPath>,
    pub forward_scopes: Vec<crate::core_scopes::ForwardScope>,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Report {
    pub origin: Origin,
    pub cores: Vec<CoreReview>,
}
pub fn outcome(findings: &[Finding], wan: &str) -> Outcome {
    findings
        .iter()
        .filter(|f| f.wan.is_empty() || f.wan == wan)
        .map(|f| f.status)
        .max()
        .unwrap_or(Outcome::Pass)
}
fn observed(
    inv: &Inventory,
    router: &str,
) -> (
    Vec<crate::core_plan::Binding>,
    HashMap<String, String>,
    usize,
) {
    let mut paths = BTreeMap::<String, String>::new();
    for route in inv.tables.get("/ip/route/print").into_iter().flatten() {
        if route.get("dst-address").is_some_and(|v| v == "0.0.0.0/0")
            && route.get("active").is_some_and(|v| v == "true")
            && let Some(table) = route.get("routing-table").filter(|v| v.as_str() != "main")
        {
            let gateway = route.get("immediate-gw").map(String::as_str).unwrap_or("");
            let interface = if gateway.contains(',') {
                ""
            } else {
                gateway.rsplit_once('%').map(|(_, i)| i).unwrap_or(gateway)
            };
            paths.entry(table.clone()).or_insert(interface.into());
        }
    }
    let mut lists = HashMap::<&str, Vec<&str>>::new();
    for rule in inv
        .tables
        .get("/ip/firewall/mangle/print")
        .into_iter()
        .flatten()
    {
        if rule.get("disabled").is_some_and(|v| v == "true") {
            continue;
        }
        if rule.get("action").is_some_and(|v| v == "mark-routing")
            && let (Some(table), Some(list)) =
                (rule.get("new-routing-mark"), rule.get("src-address-list"))
        {
            lists.entry(table).or_default().push(list);
        }
    }
    let mut interfaces = HashMap::new();
    let mut bindings = vec![];
    let omitted = paths.len().saturating_sub(512);
    for (n, (table, interface)) in paths.into_iter().take(512).enumerate() {
        let wan = format!("observed-{}", n + 1);
        interfaces.insert(wan.clone(), interface);
        let list = lists
            .get(table.as_str())
            .filter(|ls| ls.len() == 1)
            .map(|ls| ls[0])
            .unwrap_or("");
        bindings.push(crate::core_plan::Binding {
            wan,
            router: router.into(),
            list: list.into(),
            table,
        });
    }
    (bindings, interfaces, omitted)
}
pub fn review(
    settings: &Settings,
    inventories: &BTreeMap<String, Arc<Inventory>>,
    errors: &BTreeMap<String, String>,
    now: u64,
    origin: Origin,
) -> Report {
    let nodes: HashMap<_, _> = settings.nodes.iter().map(|n| (n.id.as_str(), n)).collect();
    let mut cores = vec![];
    for router in settings
        .routers
        .iter()
        .filter(|r| r.role == "core" || r.role == "both")
    {
        let inventory = inventories.get(&router.id);
        let mapped: Vec<_> = settings
            .core_lists
            .bindings
            .iter()
            .filter(|b| b.router == router.id)
            .cloned()
            .collect();
        let inferred = mapped.is_empty();
        let (bindings, interfaces, observed_paths_omitted) = if inferred {
            inventory
                .map(|i| observed(i, &router.id))
                .unwrap_or_default()
        } else {
            let interfaces = mapped
                .iter()
                .map(|b| {
                    (
                        b.wan.clone(),
                        nodes
                            .get(b.wan.as_str())
                            .map(|n| n.source.clone())
                            .unwrap_or_default(),
                    )
                })
                .collect();
            (mapped, interfaces, 0)
        };
        let mut findings = crate::core_analysis::Findings(vec![]);
        let empty = crate::routeros::Row::new();
        if observed_paths_omitted > 0 {
            findings.add(Outcome::Unsupported, "observed-path-limit", "", "", &empty,
                format!("Only the first 512 observed paths were analyzed; {observed_paths_omitted} additional paths were omitted"));
        }
        let mut counts = Counts::default();
        let mut connection_paths = vec![];
        let mut forward_scopes = vec![];
        let health = match inventory {
            None if errors.contains_key(&router.id) => Health::Failed,
            None => Health::Missing,
            Some(i)
                if errors.contains_key(&router.id)
                    || i.router_id != router.id
                    || i.errors.contains_key("collection") =>
            {
                Health::Failed
            }
            Some(i)
                if crate::core_plan::CORE_TABLES
                    .iter()
                    .any(|path| !i.tables.contains_key(*path) || i.errors.contains_key(*path)) =>
            {
                Health::Partial
            }
            Some(i)
                if now < i.collected_at
                    || now - i.collected_at > crate::normalizer::MAX_INVENTORY_AGE_SECONDS =>
            {
                Health::Stale
            }
            Some(_) if origin == Origin::Saved => Health::Historical,
            Some(_) => Health::Fresh,
        };
        if matches!(health, Health::Missing | Health::Failed | Health::Partial) {
            findings.add(
                Outcome::Unavailable,
                "inventory-unavailable",
                "",
                "",
                &empty,
                "Core inventory is missing, failed or incomplete; collect all required tables",
            );
        }
        if let Some(i) = inventory {
            let rows = |path: &str| i.tables.get(path).map(Vec::as_slice).unwrap_or(&[]);
            counts.address_members = rows("/ip/firewall/address-list/print").len();
            counts.mangle_rules = rows("/ip/firewall/mangle/print").len();
            counts.routes = rows("/ip/route/print").len();
            counts.pcc_rules = rows("/ip/firewall/mangle/print")
                .iter()
                .filter(|r| {
                    r.get("disabled").is_none_or(|v| v != "true")
                        && r.contains_key("per-connection-classifier")
                })
                .count();
            counts.fasttrack_rules = rows("/ip/firewall/filter/print")
                .iter()
                .chain(rows("/ip/firewall/mangle/print"))
                .filter(|r| {
                    r.get("disabled").is_none_or(|v| v != "true")
                        && r.get("action").is_some_and(|v| v == "fasttrack-connection")
                })
                .count();
            let refs = bindings.iter().collect::<Vec<_>>();
            let interface_refs = interfaces
                .iter()
                .map(|(k, v)| (k.as_str(), v.as_str()))
                .collect();
            let analysis = crate::core_analysis::analyze(i, &refs, &interface_refs);
            connection_paths = analysis.connection_paths;
            forward_scopes = analysis.forward_scopes;
            for f in analysis.findings {
                findings.add(
                    f.status,
                    &f.code,
                    &f.wan,
                    &f.table,
                    &[(".id".into(), f.row)].into_iter().collect(),
                    f.message,
                );
            }
            let mut interface_index = HashMap::<&str, Option<&crate::routeros::Row>>::new();
            for row in rows("/interface/print") {
                if let Some(name) = row.get("name") {
                    interface_index
                        .entry(name.as_str())
                        .and_modify(|v| *v = None)
                        .or_insert(Some(row));
                }
            }
            for b in &bindings {
                if !inferred {
                    let matching = interfaces
                        .get(&b.wan)
                        .and_then(|name| interface_index.get(name.as_str()))
                        .and_then(|r| *r);
                    if !matching.is_some_and(|r| {
                        r.get("running").is_some_and(|v| v == "true")
                            && r.get("disabled").is_some_and(|v| v == "false")
                    }) {
                        findings.add(Outcome::Conflict,"wan-interface",&b.wan,"/interface/print",&empty,"Mapped WAN interface is missing, duplicated, down or has unknown status");
                    }
                }
            }
        }
        if inferred {
            findings.add(Outcome::Unsupported,"mapping-required","","",&empty,"Observed paths are not configured WAN mappings; select actual WAN/list/table bindings before planning");
        }
        let wans = bindings
            .iter()
            .map(|b| WanReview {
                wan: b.wan.clone(),
                table: b.table.clone(),
                list: b.list.clone(),
                interface: interfaces.get(&b.wan).cloned().unwrap_or_default(),
                mapped: !inferred,
                status: outcome(&findings.0, &b.wan),
            })
            .collect();
        cores.push(CoreReview {
            router: router.id.clone(),
            collected_at: inventory.map(|i| i.collected_at),
            health,
            counts,
            findings: findings.0,
            wans,
            observed_paths_omitted,
            connection_paths,
            forward_scopes,
        });
    }
    Report { origin, cores }
}
pub fn saved(
    settings: &Settings,
    root: &Path,
) -> (BTreeMap<String, Arc<Inventory>>, BTreeMap<String, String>) {
    saved_routers(
        settings
            .routers
            .iter()
            .filter(|r| r.role == "core" || r.role == "both"),
        root,
    )
}
pub fn saved_routers<'a>(
    routers: impl IntoIterator<Item = &'a crate::model::Router>,
    root: &Path,
) -> (BTreeMap<String, Arc<Inventory>>, BTreeMap<String, String>) {
    let mut inventories = BTreeMap::new();
    let mut errors = BTreeMap::new();
    for router in routers {
        let result = (|| -> anyhow::Result<Inventory> {
            anyhow::ensure!(
                crate::model::valid_id(&router.id),
                "Invalid saved inventory router ID"
            );
            let path = root
                .join("state/router-inventory")
                .join(format!("{}.toml", router.id));
            anyhow::ensure!(
                std::fs::metadata(&path)?.len() <= 64 * 1024 * 1024,
                "Saved inventory is too large"
            );
            Ok(toml::from_str(&std::fs::read_to_string(path)?)?)
        })();
        match result {
            Ok(i) => {
                inventories.insert(router.id.clone(), Arc::new(i));
            }
            Err(_) => {
                errors.insert(
                    router.id.clone(),
                    "Saved core inventory is missing, invalid or too large".into(),
                );
            }
        }
    }
    (inventories, errors)
}
