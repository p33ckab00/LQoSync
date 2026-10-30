//! Candidate session evidence. This file never represents applied shaping.
use crate::{
    assignment::{self, AssignmentState},
    model::{self, Settings, Subscriber},
    normalizer,
    routeros::{Inventory, Row},
    selection, store,
};
use anyhow::{Context, Result, ensure};
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, HashMap, HashSet},
    fs::File,
    io::Read,
    path::Path,
    sync::Arc,
};

const MAX_BYTES: u64 = 64 * 1024 * 1024;
const MAX_RECORDS: usize = 250_000;
const STATE_PATH: &str = "state/online-assignments.toml";

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Observation {
    pub assignments: AssignmentState,
    pub active: BTreeMap<String, Session>,
    pub collected: BTreeMap<String, u64>,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Session {
    pub router: String,
    pub service: String,
    pub source_node: String,
    pub id: Option<String>,
    pub uptime: Option<u64>,
    pub collected_at: u64,
}

fn handle_valid(id: &str) -> bool {
    id.strip_prefix('*')
        .is_some_and(|v| !v.is_empty() && v.len() <= 32 && v.bytes().all(|b| b.is_ascii_hexdigit()))
}
fn normalized_id(id: &str) -> bool {
    uuid::Uuid::parse_str(id).is_ok_and(|parsed| parsed.to_string() == id)
}
pub(crate) fn validate(state: &Observation) -> Result<()> {
    crate::dynamic::validate_routes(&state.assignments)?;
    ensure!(
        state.assignments.assignments.len() <= MAX_RECORDS && state.active.len() <= MAX_RECORDS,
        "Candidate observation exceeds record limit"
    );
    for (id, pin) in &state.assignments.assignments {
        ensure!(
            normalized_id(id) && model::valid_id(&pin.source_node) && model::valid_id(&pin.node),
            "Invalid candidate assignment identity or target"
        );
    }
    for router in state.collected.keys() {
        ensure!(model::valid_id(router), "Invalid observed router identity");
    }
    for (id, session) in &state.active {
        ensure!(
            normalized_id(id)
                && model::valid_id(&session.router)
                && model::valid_id(&session.source_node)
                && state
                    .assignments
                    .assignments
                    .get(id)
                    .is_some_and(|pin| pin.source_node == session.source_node)
                && state.collected.get(&session.router) == Some(&session.collected_at),
            "Candidate session has inconsistent assignment or collection evidence"
        );
        ensure!(
            match session.service.as_str() {
                "ppp" | "hotspot" => session.id.as_deref().is_some_and(handle_valid),
                "dhcp" => session.id.is_none() && session.uptime.is_none(),
                _ => false,
            },
            "Invalid candidate session evidence"
        );
    }
    Ok(())
}
fn text<'a>(row: &'a Row, key: &str) -> &'a str {
    row.get(key).map(String::as_str).unwrap_or("")
}
fn active_or_configured<'a>(row: &'a Row, active: &str, configured: &str) -> &'a str {
    let value = text(row, active);
    if value.is_empty() || value == "all" {
        text(row, configured)
    } else {
        value
    }
}
pub(crate) fn uptime(raw: &str) -> Result<u64> {
    ensure!(
        !raw.is_empty() && raw.len() <= 128,
        "Invalid session uptime"
    );
    let mut remaining = raw;
    let mut total = 0u64;
    let mut previous_unit = 9;
    while !remaining.is_empty() {
        let digits = remaining.bytes().take_while(u8::is_ascii_digit).count();
        ensure!(digits > 0, "Invalid session uptime");
        let number = remaining[..digits].parse::<u64>()?;
        let rest = &remaining[digits..];
        if rest.starts_with(':') {
            ensure!(previous_unit > 6, "Invalid session uptime unit order");
            let parts: Vec<_> = remaining.split(':').collect();
            ensure!(parts.len() == 3, "Invalid session uptime clock");
            let minutes: u64 = parts[1].parse()?;
            let seconds: u64 = parts[2].parse()?;
            ensure!(minutes < 60 && seconds < 60, "Invalid session uptime clock");
            total = total
                .checked_add(
                    number
                        .checked_mul(3600)
                        .context("Session uptime overflow")?,
                )
                .and_then(|v| v.checked_add(minutes * 60 + seconds))
                .context("Session uptime overflow")?;
            break;
        }
        let (multiplier, divisor, unit, consumed) = if rest.starts_with("ms") {
            (1, 1000, 3, 2)
        } else if rest.starts_with("us") {
            (1, 1_000_000, 2, 2)
        } else if rest.starts_with("ns") {
            (1, 1_000_000_000, 1, 2)
        } else {
            let (multiplier, unit) = match rest.as_bytes().first() {
                Some(b'w') => (604800, 8),
                Some(b'd') => (86400, 7),
                Some(b'h') => (3600, 6),
                Some(b'm') => (60, 5),
                Some(b's') => (1, 4),
                _ => anyhow::bail!("Invalid session uptime unit"),
            };
            (multiplier, 1, unit, 1)
        };
        ensure!(unit < previous_unit, "Invalid session uptime unit order");
        previous_unit = unit;
        total = total
            .checked_add(
                (number / divisor)
                    .checked_mul(multiplier)
                    .context("Session uptime overflow")?,
            )
            .context("Session uptime overflow")?;
        remaining = &rest[consumed..];
    }
    Ok(total)
}
fn required_paths(service: &str) -> &'static [&'static str] {
    match service {
        "ppp" => &[
            "/ppp/active/print",
            "/ppp/secret/print",
            "/ppp/profile/print",
        ],
        "dhcp" => &["/ip/dhcp-server/lease/print", "/ip/dhcp-server/print"],
        _ => &[
            "/ip/hotspot/active/print",
            "/ip/hotspot/user/print",
            "/ip/hotspot/user/profile/print",
        ],
    }
}

/// The caller must supply the complete source-selected output from `normalizer::collect`.
/// Bound DHCP leases prove lease presence, not physical client connectivity.
/// A reconnect needs observed absence, a new handle, or a lower uptime to be detected.
#[allow(clippy::too_many_arguments)]
pub fn plan(
    settings: &Settings,
    subscribers: &mut [Subscriber],
    committed: &AssignmentState,
    previous: Option<&Observation>,
    inventories: &BTreeMap<String, Arc<Inventory>>,
    errors: &BTreeMap<String, String>,
    now: u64,
) -> Result<Observation> {
    plan_inner(
        settings,
        subscribers,
        committed,
        previous,
        inventories,
        errors,
        now,
        false,
    )
}
/// Capture every selected session, including manual and planned-load policies.
pub(crate) fn all_sessions(
    settings: &Settings,
    subscribers: &mut [Subscriber],
    inventories: &BTreeMap<String, Arc<Inventory>>,
    now: u64,
) -> Result<Observation> {
    plan_inner(
        settings,
        subscribers,
        &AssignmentState::default(),
        None,
        inventories,
        &BTreeMap::new(),
        now,
        true,
    )
}
#[allow(clippy::too_many_arguments)]
fn plan_inner(
    settings: &Settings,
    subscribers: &mut [Subscriber],
    committed: &AssignmentState,
    previous: Option<&Observation>,
    inventories: &BTreeMap<String, Arc<Inventory>>,
    errors: &BTreeMap<String, String>,
    now: u64,
    capture_all: bool,
) -> Result<Observation> {
    if let Some(previous) = previous {
        validate(previous)?;
    }
    let adopted;
    let adopted_committed;
    let (committed, previous) = if settings.dynamic.is_some() {
        adopted_committed = crate::dynamic::adopt(settings, subscribers, committed)?;
        adopted = previous
            .map(|p| -> Result<Observation> {
                let mut p = p.clone();
                p.assignments = crate::dynamic::adopt(settings, subscribers, &p.assignments)?;
                for (id, session) in &mut p.active {
                    if let Some(pin) = p.assignments.assignments.get(id) {
                        session.source_node = pin.source_node.clone();
                    }
                }
                p.active
                    .retain(|id, _| p.assignments.assignments.contains_key(id));
                Ok(p)
            })
            .transpose()?;
        (&adopted_committed, adopted.as_ref())
    } else {
        (committed, previous)
    };
    let bindings = selection::bindings(settings)?;
    let mut services: BTreeMap<&str, HashSet<&str>> = BTreeMap::new();
    for binding in &bindings {
        if !binding.exclude {
            services
                .entry(binding.router)
                .or_default()
                .insert(binding.service);
        }
    }
    let mut collected = BTreeMap::new();
    for router in normalizer::selected_routers(settings) {
        ensure!(
            !errors.contains_key(router),
            "Latest selected router read failed: {router}"
        );
        let inv = inventories
            .get(router)
            .context("Selected inventory is missing")?;
        ensure!(
            inv.router_id == router
                && now >= inv.collected_at
                && now - inv.collected_at <= normalizer::MAX_INVENTORY_AGE_SECONDS,
            "Selected inventory is stale or inconsistent"
        );
        ensure!(
            !inv.errors.contains_key("collection"),
            "Selected inventory collection is incomplete"
        );
        if let Some(old) = previous.and_then(|p| p.collected.get(router)) {
            ensure!(
                inv.collected_at >= *old,
                "Selected inventory is out of order"
            );
        }
        for service in services.get(router).into_iter().flatten() {
            for path in required_paths(service) {
                ensure!(
                    !inv.errors.contains_key(*path) && inv.tables.contains_key(*path),
                    "Selected session table is missing or incomplete: {path}"
                );
            }
        }
        collected.insert(router.to_string(), inv.collected_at);
    }
    let count_sources: HashSet<&str> = if let Some(c) = &settings.dynamic {
        c.policies
            .iter()
            .filter(|p| p.balance && p.metric == "online_clients")
            .flat_map(|p| p.selections.iter().map(String::as_str))
            .collect()
    } else {
        settings
            .assignment
            .pools
            .iter()
            .filter(|pool| pool.balance && pool.metric == "online_clients")
            .map(|pool| pool.source_node.as_str())
            .collect()
    };
    let selected: HashMap<_, _> = subscribers
        .iter()
        .filter(|s| capture_all || count_sources.contains(s.node.as_str()))
        .map(|s| (s.id.as_str(), s))
        .collect();
    let allowed: HashSet<_> = bindings
        .iter()
        .filter(|b| !b.exclude)
        .map(|b| (b.router, b.service, b.node))
        .collect();
    let mut active = BTreeMap::new();
    for (router, services) in &services {
        let inv = &inventories[*router];
        for service in services {
            let path = required_paths(service)[0];
            for row in &inv.tables[path] {
                let identity = match *service {
                    "ppp" => text(row, "name"),
                    "hotspot" => text(row, "user"),
                    _ => active_or_configured(row, "active-mac-address", "mac-address"),
                };
                let Ok(id) = normalizer::subscriber_id(router, service, identity) else {
                    continue;
                };
                let Some(subscriber) = selected.get(id.as_str()) else {
                    continue;
                };
                ensure!(
                    allowed.contains(&(*router, *service, subscriber.node.as_str())),
                    "Session evidence does not match its selected source"
                );
                ensure!(
                    text(row, "disabled") != "true",
                    "Selected session is disabled"
                );
                let (handle, seconds) = if *service == "dhcp" {
                    ensure!(
                        text(row, "status") == "bound",
                        "Selected DHCP binding status is unknown or not bound"
                    );
                    let server = active_or_configured(row, "active-server", "server");
                    ensure!(
                        !server.is_empty() && server != "all",
                        "Selected DHCP binding server is unknown"
                    );
                    let address = active_or_configured(row, "active-address", "address");
                    ensure!(
                        address.strip_suffix("/32").unwrap_or(address) == subscriber.ipv4,
                        "Selected DHCP binding address changed"
                    );
                    (None, None)
                } else {
                    let handle = text(row, ".id");
                    ensure!(
                        handle_valid(handle),
                        "Selected session handle is missing or invalid"
                    );
                    let raw = text(row, "uptime");
                    (
                        Some(handle.to_string()),
                        if raw.is_empty() {
                            None
                        } else {
                            Some(uptime(raw)?)
                        },
                    )
                };
                ensure!(
                    active
                        .insert(
                            id,
                            Session {
                                router: (*router).to_string(),
                                service: (*service).to_string(),
                                source_node: subscriber.node.clone(),
                                id: handle,
                                uptime: seconds,
                                collected_at: inv.collected_at,
                            }
                        )
                        .is_none(),
                    "Multiple sessions share a selected subscriber identity"
                );
            }
        }
    }
    ensure!(
        active.len() == selected.len(),
        "Selected subscriber session evidence is missing"
    );
    // Collection timestamps prove freshness, not unique read identity. The caller
    // checks read tickets and snapshot identities before saving this candidate.
    let mut baseline = previous
        .map(|p| p.assignments.clone())
        .unwrap_or_else(|| committed.clone());
    baseline.assignments.retain(|id, pin| {
        if !count_sources.contains(pin.source_node.as_str()) {
            return true;
        }
        let Some(current) = active.get(id) else {
            return false;
        };
        let Some(previous) = previous else {
            return true;
        };
        // A retained record without session metadata is a first observation of
        // a formerly manual/planned source. Healthy count-mode absence already
        // removes its assignment, so missing metadata alone is not a reconnect.
        previous
            .active
            .get(id)
            .is_none_or(|old| continuous(old, current))
    });
    baseline
        .routes
        .retain(|id, _| baseline.assignments.contains_key(id));
    let mut planned = subscribers.to_vec();
    let assignments =
        assignment::assign(settings, &mut planned, &baseline, inventories, errors, now)?;
    let result = Observation {
        assignments,
        active,
        collected,
    };
    validate(&result)?;
    subscribers.clone_from_slice(&planned);
    Ok(result)
}

fn read(root: &Path) -> Result<Option<String>> {
    let path = root.join(STATE_PATH);
    let file = match File::open(&path) {
        Ok(file) => file,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(error.into()),
    };
    ensure!(
        file.metadata()?.len() <= MAX_BYTES,
        "Candidate observation exceeds size limit"
    );
    let mut text = String::new();
    file.take(MAX_BYTES + 1).read_to_string(&mut text)?;
    ensure!(
        text.len() as u64 <= MAX_BYTES,
        "Candidate observation exceeds size limit"
    );
    Ok(Some(text))
}
pub(crate) struct Revision(Option<String>);

pub(crate) fn load_for_update(root: &Path) -> Result<(Option<Observation>, Revision)> {
    let text = read(root)?;
    let state = text
        .as_deref()
        .map(|text| -> Result<Observation> {
            let state: Observation =
                toml::from_str(text).context("Candidate session observation is corrupt")?;
            validate(&state)?;
            Ok(state)
        })
        .transpose()?;
    Ok((state, Revision(text)))
}

pub fn load(root: &Path) -> Result<Option<Observation>> {
    Ok(load_for_update(root)?.0)
}

impl Revision {
    pub(crate) fn save(self, root: &Path, state: &Observation) -> Result<()> {
        validate(state)?;
        let existing = read(root)?;
        ensure!(
            existing == self.0,
            "Candidate session state changed; retry collection"
        );
        let text = toml::to_string(state)?;
        ensure!(
            text.len() as u64 <= MAX_BYTES,
            "Candidate observation exceeds size limit"
        );
        if existing.as_deref() != Some(text.as_str()) {
            store::atomic_write(&root.join(STATE_PATH), &text)?;
        }
        Ok(())
    }
}

pub fn save(root: &Path, state: &Observation) -> Result<()> {
    load_for_update(root)?.1.save(root, state)
}

pub(crate) fn continuous(old: &Session, current: &Session) -> bool {
    old.source_node == current.source_node
        && old.router == current.router
        && old.service == current.service
        && old.id == current.id
        && match (old.uptime, current.uptime) {
            (Some(a), Some(b)) => b >= a,
            _ => true,
        }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn candidate_revision_rejects_an_intervening_write_without_overwriting_it() {
        let dir = tempfile::tempdir().unwrap();
        let (_, revision) = load_for_update(dir.path()).unwrap();
        let state = Observation::default();
        save(dir.path(), &state).unwrap();
        let before = std::fs::read(dir.path().join(STATE_PATH)).unwrap();
        assert!(revision.save(dir.path(), &state).is_err());
        assert_eq!(std::fs::read(dir.path().join(STATE_PATH)).unwrap(), before);
    }
}
