use crate::{
    model::{self, Settings, Subscriber},
    routeros::Inventory,
    speed_preview::preview_selected,
};
use anyhow::{Context, Result, bail, ensure};
use std::{
    collections::{BTreeMap, HashMap, HashSet},
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
}
pub fn validate_bindings(settings: &Settings) -> Result<()> {
    let mut bindings = HashSet::new();
    for n in &settings.nodes {
        if let Some(service) = service(&n.kind) {
            if n.router.is_empty() || n.source.is_empty() {
                continue;
            }
            ensure!(
                bindings.insert((&n.router, service, &n.source)),
                "Duplicate source binding for {} / {}",
                n.router,
                n.source
            );
            ensure!(
                settings
                    .routers
                    .iter()
                    .any(|r| r.id == n.router && r.role != "core"),
                "Subscriber source requires a distro or both-role router"
            );
        }
    }
    Ok(())
}
fn canonical_mac(raw: &str) -> Result<String> {
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
    model::validate(settings)?;
    validate_bindings(settings)?;
    let bindings: HashMap<_, _> = settings
        .nodes
        .iter()
        .filter_map(|n| {
            service(&n.kind)
                .filter(|_| !n.router.is_empty() && !n.source.is_empty())
                .map(|s| ((n.router.as_str(), s, n.source.as_str()), n))
        })
        .collect();
    ensure!(!bindings.is_empty(), "No subscriber sources are selected");
    let mut by_router: BTreeMap<&str, HashSet<&str>> = BTreeMap::new();
    for (router, service, _) in bindings.keys() {
        by_router.entry(router).or_default().insert(service);
    }
    let mut candidates = vec![];
    let mut ids = HashSet::new();
    for (router, services) in by_router {
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
        for (r, service, source) in bindings.keys() {
            if *r != router {
                continue;
            }
            let row = metadata
                .get(service)
                .and_then(|m| m.get(source))
                .and_then(|r| *r)
                .with_context(|| {
                    format!("Selected source metadata is missing or duplicated: {source}")
                })?;
            ensure!(
                !row.get("disabled").is_some_and(|v| v == "true"),
                "Selected source is disabled: {source}"
            );
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
            let Some(node) = bindings.get(&(router, entry.service, entry.group.as_str())) else {
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
            let key = serde_json::to_vec(&(router, entry.service, identity))?;
            let id = uuid::Uuid::new_v5(&uuid::Uuid::NAMESPACE_OID, &key).to_string();
            ensure!(
                ids.insert(id.clone()),
                "Multiple sessions or leases share one subscriber identity"
            );
            candidates.push(Candidate {
                subscriber: Subscriber {
                    id,
                    name: entry.name,
                    ipv4: ip.to_string(),
                    node: node.id.clone(),
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
    if subscribers.is_empty() {
        bail!("Selected scan is empty; last valid snapshot is retained");
    }
    subscribers.sort_by(|a, b| a.id.cmp(&b.id));
    Ok(subscribers)
}
