use crate::{
    model::{self, Settings, Subscriber},
    routeros::Inventory,
    speed_preview::preview_selected,
};
use anyhow::{Context, Result, bail, ensure};
use std::{
    collections::{BTreeMap, BTreeSet, HashMap, HashSet},
    net::Ipv4Addr,
    sync::Arc,
};

pub const MAX_INVENTORY_AGE_SECONDS: u64 = 300;
pub fn service(kind: &str) -> Option<&'static str> {
    match kind {
        "pppoe" => Some("ppp"),
        "dhcp" => Some("dhcp"),
        "hotspot" => Some("hotspot"),
        _ => None,
    }
}
pub fn source_table(kind: &str) -> Option<&'static str> {
    match kind {
        "pppoe" => Some("/ppp/profile/print"),
        "dhcp" => Some("/ip/dhcp-server/print"),
        "hotspot" => Some("/ip/hotspot/user/profile/print"),
        _ => None,
    }
}
pub fn has_bindings(settings: &Settings) -> bool {
    settings
        .nodes
        .iter()
        .any(|n| service(&n.kind).is_some() && !n.router.is_empty() && !n.source.is_empty())
        || settings.selections.iter().any(|s| !s.exclude)
}
pub fn selected_routers(settings: &Settings) -> BTreeSet<&str> {
    settings
        .nodes
        .iter()
        .filter(|n| service(&n.kind).is_some() && !n.router.is_empty() && !n.source.is_empty())
        .map(|n| n.router.as_str())
        .chain(
            settings
                .selections
                .iter()
                .filter(|s| !s.exclude)
                .map(|s| s.router.as_str()),
        )
        .collect()
}
pub fn is_source_node(settings: &Settings, id: &str) -> bool {
    settings.nodes.iter().any(|n| {
        n.id == id && service(&n.kind).is_some() && !n.router.is_empty() && !n.source.is_empty()
    }) || settings
        .selections
        .iter()
        .any(|s| !s.exclude && s.node == id)
}
pub fn validate_bindings(settings: &Settings) -> Result<()> {
    crate::selection::validate(settings)
}
pub fn subscriber_id(router: &str, service: &str, identity: &str) -> Result<String> {
    ensure!(
        !router.is_empty() && matches!(service, "ppp" | "dhcp" | "hotspot"),
        "Invalid subscriber router or service"
    );
    ensure!(
        !identity.is_empty() && identity.len() <= 256,
        "Invalid subscriber identity"
    );
    let canonical = if service == "dhcp" {
        canonical_mac(identity)?
    } else {
        identity.into()
    };
    let key = serde_json::to_vec(&(router, service, canonical))?;
    Ok(uuid::Uuid::new_v5(&uuid::Uuid::NAMESPACE_OID, &key).to_string())
}
pub(crate) fn canonical_mac(raw: &str) -> Result<String> {
    let value: String = raw
        .chars()
        .filter(|c| !matches!(c, ':' | '-' | '.'))
        .collect();
    ensure!(
        value.len() == 12 && value.bytes().all(|b| b.is_ascii_hexdigit()),
        "Missing or invalid subscriber MAC"
    );
    let bytes = (0..6)
        .map(|i| u8::from_str_radix(&value[i * 2..i * 2 + 2], 16))
        .collect::<std::result::Result<Vec<_>, _>>()?;
    ensure!(
        bytes[0] & 1 == 0 && bytes.iter().any(|b| *b != 0),
        "Invalid multicast or zero subscriber MAC"
    );
    Ok(value.to_ascii_lowercase())
}
fn ipv4(raw: &str) -> Result<Ipv4Addr> {
    let ip = raw.strip_suffix("/32").unwrap_or(raw).parse::<Ipv4Addr>()?;
    ensure!(
        !ip.is_unspecified()
            && !ip.is_multicast()
            && !ip.is_broadcast()
            && !ip.is_loopback()
            && ip.octets()[0] != 0,
        "Invalid subscriber IPv4"
    );
    Ok(ip)
}
struct Candidate {
    subscriber: Subscriber,
    router: String,
    service: &'static str,
    mac: Option<String>,
}

pub fn normalize(
    settings: &Settings,
    inventories: &BTreeMap<String, Arc<Inventory>>,
    errors: &BTreeMap<String, String>,
    now: u64,
) -> Result<Vec<Subscriber>> {
    let subscribers = collect(settings, inventories, errors, now)?;
    if subscribers.is_empty() {
        bail!("Selected scan is empty; last valid snapshot is retained");
    }
    Ok(subscribers)
}
pub fn collect(
    settings: &Settings,
    inventories: &BTreeMap<String, Arc<Inventory>>,
    errors: &BTreeMap<String, String>,
    now: u64,
) -> Result<Vec<Subscriber>> {
    Ok(collect_with_sources(settings, inventories, errors, now)?.subscribers)
}
#[derive(Debug, Clone)]
pub struct Collection {
    pub subscribers: Vec<Subscriber>,
    pub sources: BTreeMap<String, String>,
}
pub fn collect_with_sources(
    settings: &Settings,
    inventories: &BTreeMap<String, Arc<Inventory>>,
    errors: &BTreeMap<String, String>,
    now: u64,
) -> Result<Collection> {
    model::validate(settings)?;
    let bindings = crate::selection::bindings(settings)?;
    ensure!(has_bindings(settings), "No subscriber sources are selected");
    let active_services: HashSet<_> = bindings
        .iter()
        .filter(|b| !b.exclude)
        .map(|b| (b.router, b.service))
        .collect();
    let mut by_router: BTreeMap<&str, Vec<_>> = BTreeMap::new();
    for binding in &bindings {
        if active_services.contains(&(binding.router, binding.service)) {
            by_router.entry(binding.router).or_default().push(binding);
        }
    }
    let mut candidates = vec![];
    let mut ids = HashSet::new();
    for (router, selected) in by_router {
        let services: HashSet<_> = selected.iter().map(|b| b.service).collect();
        let groups: HashMap<_, _> = selected
            .iter()
            .filter(|b| b.selector == "group")
            .map(|b| ((b.service, b.value.as_str()), *b))
            .collect();
        let individuals: HashMap<_, _> = selected
            .iter()
            .filter(|b| b.selector == "subscriber")
            .map(|b| ((b.service, b.value.as_str()), *b))
            .collect();
        ensure!(
            !errors.contains_key(router),
            "Latest inventory read failed for {router}"
        );
        let inv = inventories
            .get(router)
            .with_context(|| format!("Read inventory first: {router}"))?;
        ensure!(
            inv.router_id == router,
            "Inventory router identity does not match"
        );
        ensure!(
            now >= inv.collected_at && now - inv.collected_at <= MAX_INVENTORY_AGE_SECONDS,
            "Inventory is stale or has a future timestamp: {router}"
        );
        let mut metadata: HashMap<&str, HashMap<&str, Option<&crate::routeros::Row>>> =
            HashMap::new();
        for service in &services {
            let kind = if *service == "ppp" { "pppoe" } else { service };
            let path = source_table(kind).expect("selected service validated");
            ensure!(
                !inv.errors.contains_key(path),
                "Selected source metadata read failed: {path}"
            );
            let rows = inv
                .tables
                .get(path)
                .context("Selected source metadata is missing")?;
            let mut index = HashMap::with_capacity(rows.len());
            for row in rows {
                let name = row
                    .get("name")
                    .filter(|n| !n.is_empty())
                    .context("Source metadata has an unnamed row")?;
                index
                    .entry(name.as_str())
                    .and_modify(|v| *v = None)
                    .or_insert(Some(row));
            }
            metadata.insert(service, index);
        }
        for binding in &selected {
            if binding.exclude || binding.selector != "group" {
                continue;
            }
            let source = &binding.value;
            let row = metadata
                .get(binding.service)
                .and_then(|m| m.get(source.as_str()))
                .and_then(|r| *r)
                .with_context(|| {
                    format!("Selected source metadata is missing or duplicated: {source}")
                })?;
            ensure!(
                !row.get("disabled").is_some_and(|v| v == "true"),
                "Selected source is disabled: {source}"
            );
        }
        // Configured accounts must exist; dynamic leases can disappear after disconnect.
        for service in &services {
            let requested: HashSet<_> = selected
                .iter()
                .filter(|b| !b.exclude && b.service == *service && b.selector == "subscriber")
                .map(|b| b.value.as_str())
                .collect();
            if requested.is_empty() {
                continue;
            }
            let path = match *service {
                "ppp" => "/ppp/secret/print",
                "dhcp" => "/ip/dhcp-server/lease/print",
                _ => "/ip/hotspot/user/print",
            };
            ensure!(
                !inv.errors.contains_key(path),
                "Selected subscriber metadata read failed: {path}"
            );
            let rows = inv
                .tables
                .get(path)
                .context("Selected subscriber metadata is missing")?;
            let mut found: HashMap<String, bool> = HashMap::new();
            for row in rows {
                let key = if *service == "dhcp" {
                    let raw = row
                        .get("active-mac-address")
                        .filter(|v| !v.is_empty())
                        .or_else(|| row.get("mac-address"));
                    let Some(raw) = raw else {
                        continue;
                    };
                    let Ok(mac) = canonical_mac(raw) else {
                        continue;
                    };
                    mac
                } else {
                    row.get("name").cloned().unwrap_or_default()
                };
                if requested.contains(key.as_str()) {
                    let group = if *service == "dhcp" {
                        row.get("active-server")
                            .filter(|v| !v.is_empty() && *v != "all")
                            .or_else(|| row.get("server").filter(|v| !v.is_empty() && *v != "all"))
                    } else {
                        row.get("profile")
                    };
                    let group_row = group.and_then(|g| {
                        metadata
                            .get(service)
                            .and_then(|m| m.get(g.as_str()))
                            .and_then(|r| *r)
                    });
                    ensure!(
                        group_row.is_some_and(|r| !r.get("disabled").is_some_and(|v| v == "true")),
                        "Selected subscriber profile/server mapping is missing, disabled or duplicated"
                    );
                    let enabled = !row.get("disabled").is_some_and(|v| v == "true");
                    found
                        .entry(key)
                        .and_modify(|v| *v = false)
                        .or_insert(enabled);
                }
            }
            for identity in requested {
                ensure!(
                    found.get(identity).copied().unwrap_or(*service == "dhcp"),
                    "Selected subscriber metadata is missing, disabled or duplicated: {identity}"
                );
            }
        }
        let report = preview_selected(inv, &settings.speed_rules, &services)?;
        ensure!(
            report.errors.is_empty(),
            "Selected source tables are incomplete or ambiguous: {}",
            report
                .errors
                .first()
                .map(String::as_str)
                .unwrap_or("unknown")
        );
        for entry in report.entries {
            let canonical_identity = if entry.service == "dhcp" {
                canonical_mac(&entry.mac).ok()
            } else {
                Some(entry.identity.clone())
            };
            let binding = canonical_identity
                .as_deref()
                .and_then(|identity| individuals.get(&(entry.service, identity)))
                .or_else(|| groups.get(&(entry.service, entry.group.as_str())));
            if binding.is_some_and(|b| b.exclude) {
                continue;
            }
            if entry.service == "dhcp" {
                ensure!(
                    canonical_identity.is_some(),
                    "Missing or invalid subscriber MAC"
                );
            }
            ensure!(
                !entry.group.is_empty(),
                "Subscriber profile/server mapping is unresolved: {}",
                entry.identity
            );
            ensure!(
                metadata
                    .get(entry.service)
                    .and_then(|m| m.get(entry.group.as_str()))
                    .is_some_and(|r| r.is_some()),
                "Subscriber source mapping is missing or duplicated: {}",
                entry.group
            );
            let Some(binding) = binding else {
                continue;
            };
            let speed = entry.speed.with_context(|| {
                format!(
                    "Selected subscriber {} is unresolved: {}",
                    entry.identity,
                    entry.error.unwrap_or_default()
                )
            })?;
            ensure!(
                !entry.identity.is_empty() && entry.identity.len() <= 256,
                "Invalid subscriber identity"
            );
            let ip = ipv4(&entry.ipv4)
                .with_context(|| format!("Invalid IPv4 for {}", entry.identity))?;
            let mac = if entry.service == "dhcp" || !entry.mac.is_empty() {
                Some(canonical_mac(&entry.mac)?)
            } else {
                None
            };
            let identity = if entry.service == "dhcp" {
                mac.as_deref().expect("DHCP MAC validated")
            } else {
                &entry.identity
            };
            let id = subscriber_id(router, entry.service, identity)?;
            ensure!(
                ids.insert(id.clone()),
                "Multiple sessions or leases share one subscriber identity"
            );
            candidates.push(Candidate {
                subscriber: Subscriber {
                    id,
                    name: entry.name,
                    ipv4: ip.to_string(),
                    node: binding.node.into(),
                    download: speed.rates.download,
                    upload: speed.rates.upload,
                    download_min: speed.rates.download_min,
                    upload_min: speed.rates.upload_min,
                },
                router: router.into(),
                service: entry.service,
                mac,
            });
        }
    }
    // Only an exact same-router DHCP/Hotspot device overlap has automatic ownership.
    let mut ips: HashMap<String, usize> = HashMap::new();
    let mut suppressed = HashSet::new();
    for (i, c) in candidates.iter().enumerate() {
        if let Some(&j) = ips.get(&c.subscriber.ipv4) {
            let old = &candidates[j];
            let pair = (old.service, c.service);
            ensure!(
                matches!(pair, ("dhcp", "hotspot") | ("hotspot", "dhcp"))
                    && old.router == c.router
                    && old.mac.is_some()
                    && old.mac == c.mac,
                "Conflicting subscriber IPv4: {}",
                c.subscriber.ipv4
            );
            if c.service == "hotspot" {
                suppressed.insert(j);
                ips.insert(c.subscriber.ipv4.clone(), i);
            } else {
                suppressed.insert(i);
            }
        } else {
            ips.insert(c.subscriber.ipv4.clone(), i);
        }
    }
    let mut subscribers: Vec<_> = candidates
        .into_iter()
        .enumerate()
        .filter(|(i, _)| !suppressed.contains(i))
        .map(|(_, c)| c.subscriber)
        .collect();
    subscribers.sort_by(|a, b| a.id.cmp(&b.id));
    let sources = subscribers
        .iter()
        .map(|s| (s.id.clone(), s.node.clone()))
        .collect();
    Ok(Collection {
        subscribers,
        sources,
    })
}
