use crate::{
    assignment::AssignmentState,
    model::{Settings, Subscriber},
    routeros::Inventory,
};
use anyhow::{Context, Result, ensure};
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, HashMap, HashSet},
    net::Ipv4Addr,
    sync::Arc,
};
pub const CORE_TABLES: &[&str] = &[
    "/system/resource/print",
    "/interface/print",
    "/ip/route/print",
    "/routing/table/print",
    "/routing/rule/print",
    "/ip/firewall/address-list/print",
    "/ip/firewall/mangle/print",
    "/ip/firewall/filter/print",
];
pub const EXTRA_TABLES: &[&str] = &[
    "/interface/list/print",
    "/routing/table/print",
    "/routing/rule/print",
    "/ip/firewall/address-list/print",
    "/ip/firewall/mangle/print",
    "/ip/firewall/filter/print",
];
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Policy {
    pub owner: String,
    pub bindings: Vec<Binding>,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Binding {
    pub wan: String,
    pub router: String,
    pub list: String,
    pub table: String,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Action {
    Add,
    Keep,
    Remove,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Entry {
    pub action: Action,
    pub router: String,
    pub wan: String,
    pub list: String,
    pub address: String,
    pub subscriber: String,
    pub node: String,
    pub comment: String,
    pub entry_id: String,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Plan {
    pub owner: String,
    pub bindings: Vec<Binding>,
    pub source_routers: Vec<String>,
    pub inventory_at: BTreeMap<String, u64>,
    pub entries: Vec<Entry>,
    pub protected: usize,
}
pub fn marker(owner: &str, subscriber: &str, wan: &str) -> String {
    format!("lqosync3:{owner}:{subscriber}:{wan}")
}
pub fn valid_name(name: &str) -> bool {
    !name.is_empty()
        && name.len() <= 64
        && name.trim() == name
        && !name.starts_with('!')
        && !name.chars().any(char::is_control)
}
pub fn validate(settings: &Settings) -> Result<()> {
    let policy = &settings.core_lists;
    if !policy.owner.is_empty() {
        ensure!(
            !uuid::Uuid::parse_str(&policy.owner)?.is_nil(),
            "Core ownership UUID cannot be nil"
        );
    }
    if policy.bindings.is_empty() {
        return Ok(());
    }
    ensure!(
        !policy.owner.is_empty(),
        "Core lists require a persistent ownership UUID"
    );
    let nodes: HashMap<_, _> = settings.nodes.iter().map(|n| (n.id.as_str(), n)).collect();
    let routers: HashMap<_, _> = settings
        .routers
        .iter()
        .map(|r| (r.id.as_str(), r))
        .collect();
    let mut wans = HashSet::new();
    let mut lists = HashSet::new();
    let mut tables = HashSet::new();
    for b in &policy.bindings {
        let wan = nodes
            .get(b.wan.as_str())
            .context("Core binding refers to an unknown WAN")?;
        let router = routers
            .get(b.router.as_str())
            .context("Core binding refers to an unknown router")?;
        ensure!(
            wan.kind == "wan"
                && wan.parent.is_empty()
                && wan.router == b.router
                && !wan.source.is_empty(),
            "Core router must match the explicitly mapped WAN router"
        );
        ensure!(
            router.role == "core" || router.role == "both",
            "Core list router needs the core or both role"
        );
        ensure!(
            valid_name(&b.list) && valid_name(&b.table) && b.table != "main",
            "Core list and custom routing table names are invalid"
        );
        ensure!(wans.insert(&b.wan), "Duplicate core WAN binding");
        ensure!(
            lists.insert((&b.router, &b.list)),
            "Duplicate core address list"
        );
        ensure!(
            tables.insert((&b.router, &b.table)),
            "Duplicate core routing table"
        );
    }
    Ok(())
}
pub fn host(address: &str) -> Result<u32> {
    let bare = address.strip_suffix("/32").unwrap_or(address);
    Ok(u32::from(bare.parse::<Ipv4Addr>().context(
        "Core list addresses must be individual IPv4 /32 hosts",
    )?))
}
pub(crate) fn span(address: &str) -> Result<(u32, u32)> {
    if let Some((first, last)) = address.split_once('-') {
        let first = host(first)?;
        let last = host(last)?;
        ensure!(first <= last, "Invalid address range");
        return Ok((first, last));
    }
    if let Some((ip, prefix)) = address.split_once('/') {
        let ip = host(ip)?;
        let prefix: u32 = prefix.parse()?;
        ensure!(prefix <= 32, "Invalid address prefix");
        let mask = u32::MAX.checked_shl(32 - prefix).unwrap_or(0);
        return Ok((ip & mask, ip | !mask));
    }
    let ip = host(address)?;
    Ok((ip, ip))
}
fn owned<'a>(comment: &'a str, owner: &str) -> Result<Option<(&'a str, &'a str)>> {
    let prefix = format!("lqosync3:{owner}:");
    let Some(rest) = comment.strip_prefix(&prefix) else {
        return Ok(None);
    };
    let (subscriber, wan) = rest
        .split_once(':')
        .context("Malformed owned address-list marker")?;
    ensure!(
        uuid::Uuid::parse_str(subscriber).is_ok_and(|id| id.to_string() == subscriber)
            && crate::model::valid_id(wan),
        "Malformed owned address-list marker"
    );
    Ok(Some((subscriber, wan)))
}
pub fn validate_fresh(plan: &Plan, now: u64) -> Result<()> {
    for timestamp in plan.inventory_at.values() {
        ensure!(
            now >= *timestamp && now - *timestamp <= crate::normalizer::MAX_INVENTORY_AGE_SECONDS,
            "Core inventory is stale or has a future timestamp"
        );
    }
    Ok(())
}
pub fn validate_snapshot(plan: &Plan, subscribers: &[Subscriber]) -> Result<()> {
    ensure!(
        !uuid::Uuid::parse_str(&plan.owner)?.is_nil() && !plan.bindings.is_empty(),
        "Invalid committed core ownership or bindings"
    );
    let mut bindings = HashMap::new();
    let mut lists = HashSet::new();
    let mut tables = HashSet::new();
    for b in &plan.bindings {
        ensure!(
            crate::model::valid_id(&b.wan)
                && crate::model::valid_id(&b.router)
                && valid_name(&b.list)
                && valid_name(&b.table)
                && b.table != "main"
                && bindings.insert((&b.router, &b.wan), b).is_none()
                && lists.insert((&b.router, &b.list))
                && tables.insert((&b.router, &b.table)),
            "Invalid or duplicate committed core binding"
        );
    }
    let mut routers: HashSet<_> = plan.bindings.iter().map(|b| &b.router).collect();
    let mut sources = HashSet::new();
    for router in &plan.source_routers {
        ensure!(
            crate::model::valid_id(router) && sources.insert(router),
            "Invalid committed source router provenance"
        );
        routers.insert(router);
    }
    ensure!(
        !sources.is_empty(),
        "Core plan source provenance is missing"
    );
    ensure!(
        routers.len() == plan.inventory_at.len()
            && routers.iter().all(|r| plan.inventory_at.contains_key(*r)),
        "Committed core inventory provenance is incomplete"
    );
    let subs: HashMap<_, _> = subscribers.iter().map(|s| (s.id.as_str(), s)).collect();
    let mut active = HashSet::new();
    let mut row_ids = HashSet::new();
    ensure!(
        subs.len() == subscribers.len(),
        "Duplicate committed subscriber identity"
    );
    for e in &plan.entries {
        let binding = bindings
            .get(&(&e.router, &e.wan))
            .context("Committed core entry has no binding")?;
        ensure!(
            binding.list == e.list
                && uuid::Uuid::parse_str(&e.subscriber).is_ok()
                && e.comment == marker(&plan.owner, &e.subscriber, &e.wan)
                && e.address == format!("{}/32", Ipv4Addr::from(host(&e.address)?)),
            "Invalid committed core entry scope or ownership"
        );
        if e.action == Action::Add {
            ensure!(
                e.entry_id.is_empty(),
                "Planned add unexpectedly has a router row ID"
            );
        } else {
            ensure!(
                e.entry_id.starts_with('*')
                    && e.entry_id.len() > 1
                    && e.entry_id[1..].bytes().all(|c| c.is_ascii_alphanumeric())
                    && row_ids.insert((&e.router, &e.entry_id)),
                "Invalid or duplicate planned router row ID"
            );
        }
        if e.action != Action::Remove {
            let sub = subs
                .get(e.subscriber.as_str())
                .context("Core plan subscriber is absent from committed snapshot")?;
            ensure!(
                active.insert(&e.subscriber)
                    && e.node == sub.node
                    && host(&e.address)? == host(&sub.ipv4)?,
                "Core plan does not match committed subscriber parent or IP"
            );
        } else {
            ensure!(
                e.node.is_empty(),
                "Removal must not claim an active subscriber parent"
            );
        }
    }
    ensure!(
        active.len() == subscribers.len(),
        "Core plan does not cover every committed subscriber"
    );
    Ok(())
}
pub fn build(
    settings: &Settings,
    subscribers: &[Subscriber],
    assignments: &AssignmentState,
    inventories: &BTreeMap<String, Arc<Inventory>>,
    errors: &BTreeMap<String, String>,
    now: u64,
) -> Result<Plan> {
    crate::model::validate(settings)?;
    ensure!(
        !settings.core_lists.bindings.is_empty() && !subscribers.is_empty(),
        "Core plans require bindings and a nonempty fresh subscriber snapshot"
    );
    crate::assignment::validate_wans(settings, inventories, errors, now)?;
    let topology = crate::assignment::Topology::new(settings)?;
    let mut bindings = settings.core_lists.bindings.clone();
    bindings.sort_by(|a, b| a.wan.cmp(&b.wan));
    let by_wan: HashMap<_, _> = bindings.iter().map(|b| (b.wan.as_str(), b)).collect();
    let mut by_router: BTreeMap<&str, Vec<&Binding>> = BTreeMap::new();
    for b in &bindings {
        by_router.entry(&b.router).or_default().push(b);
    }
    let interfaces: HashMap<_, _> = bindings
        .iter()
        .map(|b| {
            (
                b.wan.as_str(),
                topology.nodes[b.wan.as_str()].source.as_str(),
            )
        })
        .collect();
    let mut inventory_at = BTreeMap::new();
    for (router, bs) in &by_router {
        ensure!(
            !errors.contains_key(*router),
            "Latest core router read failed: {router}"
        );
        let inv = inventories
            .get(*router)
            .context("Read core inventory first")?;
        ensure!(
            inv.router_id == *router
                && now >= inv.collected_at
                && now - inv.collected_at <= crate::normalizer::MAX_INVENTORY_AGE_SECONDS,
            "Core inventory is stale or inconsistent"
        );
        ensure!(
            !inv.errors.contains_key("collection"),
            "Core collection is incomplete"
        );
        for path in CORE_TABLES {
            ensure!(
                !inv.errors.contains_key(*path) && inv.tables.contains_key(*path),
                "Required core table is missing or incomplete: {path}"
            );
        }
        crate::core_routes::audit(inv, bs, &interfaces)
            .map_err(|e| anyhow::anyhow!("Core {router} route validation: {e:#}"))?;
        inventory_at.insert((*router).to_string(), inv.collected_at);
    }
    let mut desired = BTreeMap::new();
    let mut hosts: HashMap<&str, BTreeMap<u32, &str>> = HashMap::new();
    let mut ids = HashSet::new();
    for sub in subscribers {
        ensure!(
            ids.insert(&sub.id) && uuid::Uuid::parse_str(&sub.id).is_ok(),
            "Duplicate or invalid core subscriber identity"
        );
        let record = assignments
            .assignments
            .get(&sub.id)
            .context("Subscriber assignment ownership is missing")?;
        ensure!(
            record.node == sub.node,
            "Subscriber assignment parent is inconsistent"
        );
        let root = topology.root(&sub.node)?;
        let binding = by_wan
            .get(root.id.as_str())
            .context("Every active subscriber WAN needs a core list binding")?;
        let ip = host(&sub.ipv4)?;
        ensure!(
            hosts
                .entry(&binding.router)
                .or_default()
                .insert(ip, &sub.id)
                .is_none(),
            "Duplicate core subscriber IPv4 host"
        );
        desired.insert(
            (binding.router.clone(), binding.list.clone(), ip),
            Entry {
                action: Action::Add,
                router: binding.router.clone(),
                wan: binding.wan.clone(),
                list: binding.list.clone(),
                address: format!("{}/32", Ipv4Addr::from(ip)),
                subscriber: sub.id.clone(),
                node: sub.node.clone(),
                comment: marker(&settings.core_lists.owner, &sub.id, &binding.wan),
                entry_id: String::new(),
            },
        );
    }
    let mut source_routers: Vec<_> = settings
        .nodes
        .iter()
        .filter(|n| {
            crate::normalizer::service(&n.kind).is_some()
                && !n.router.is_empty()
                && !n.source.is_empty()
        })
        .map(|n| n.router.clone())
        .collect();
    source_routers.sort();
    source_routers.dedup();
    for router in &source_routers {
        let inv = inventories
            .get(router)
            .context("Source inventory is missing from core plan inputs")?;
        ensure!(
            !errors.contains_key(router)
                && !inv.errors.contains_key("collection")
                && inv.router_id == *router
                && now >= inv.collected_at
                && now - inv.collected_at <= crate::normalizer::MAX_INVENTORY_AGE_SECONDS,
            "Core plan source inventory is stale, failed or inconsistent"
        );
        inventory_at.insert(router.clone(), inv.collected_at);
    }
    let mut entries = vec![];
    let mut protected = 0;
    for (router, bs) in &by_router {
        let lists: HashMap<_, _> = bs.iter().map(|b| (b.list.as_str(), *b)).collect();
        let inv = &inventories[*router];
        let mut row_ids = HashSet::new();
        let mut owned_ids = HashSet::new();
        let mut owned_ips = HashSet::new();
        for row in &inv.tables["/ip/firewall/address-list/print"] {
            let list = row
                .get("list")
                .context("Core address-list row has no list name")?;
            let Some(binding) = lists.get(list.as_str()) else {
                continue;
            };
            let row_id = row.get(".id").context("Core address-list row has no ID")?;
            ensure!(
                row_id.starts_with('*')
                    && row_id.len() > 1
                    && row_id[1..].bytes().all(|c| c.is_ascii_alphanumeric())
                    && row_ids.insert(row_id),
                "Duplicate or invalid core address-list row ID"
            );
            let address = row.get("address").context("Core list row has no address")?;
            let comment = row.get("comment").map(String::as_str).unwrap_or("");
            if let Some((subscriber, wan)) = owned(comment, &settings.core_lists.owner)? {
                ensure!(
                    wan == binding.wan,
                    "Owned marker conflicts with its configured list/WAN scope"
                );
                ensure!(
                    !crate::core_routes::flag(row, "dynamic", false)?
                        && !crate::core_routes::flag(row, "disabled", false)?
                        && row.get("timeout").is_none_or(|v| v == "0s" || v.is_empty()),
                    "Dynamic, disabled or expiring owned entry needs review"
                );
                let ip = host(address).context("Broad or unresolved owned address needs review")?;
                ensure!(
                    owned_ids.insert(subscriber) && owned_ips.insert((list, ip)),
                    "Duplicate owned subscriber or IPv4 address"
                );
                let key = ((*router).to_string(), list.clone(), ip);
                if desired.get(&key).is_some_and(|e| e.comment == comment) {
                    let mut entry = desired.remove(&key).expect("checked desired entry");
                    entry.action = Action::Keep;
                    entry.entry_id = row_id.clone();
                    entries.push(entry);
                } else {
                    entries.push(Entry {
                        action: Action::Remove,
                        router: (*router).to_string(),
                        wan: wan.into(),
                        list: list.clone(),
                        address: format!("{}/32", Ipv4Addr::from(ip)),
                        subscriber: subscriber.into(),
                        node: String::new(),
                        comment: comment.into(),
                        entry_id: row_id.clone(),
                    });
                }
            } else {
                protected += 1;
                let (first, last) = span(address).context(
                    "Protected core list address is unresolved; review DNS or range membership",
                )?;
                ensure!(
                    hosts
                        .get(*router)
                        .is_none_or(|h| h.range(first..=last).next().is_none()),
                    "Subscriber IPv4 overlaps a protected manual/other-owner list member: {list} {address}"
                );
            }
        }
    }
    entries.extend(desired.into_values());
    entries.sort_by(|a, b| {
        (
            &a.router,
            &a.list,
            &a.address,
            a.action,
            &a.subscriber,
            &a.entry_id,
        )
            .cmp(&(
                &b.router,
                &b.list,
                &b.address,
                b.action,
                &b.subscriber,
                &b.entry_id,
            ))
    });
    Ok(Plan {
        owner: settings.core_lists.owner.clone(),
        bindings,
        source_routers,
        inventory_at,
        entries,
        protected,
    })
}
