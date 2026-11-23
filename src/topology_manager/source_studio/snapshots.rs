//! Studio collection (`StudioJob::Collect`): an explicit, bounded read of allowlisted tables.
//! Raw rows stay in this process. Only table evidence and WAN evidence leave it.
//! A failed or partial table is never an empty complete table.
use super::connection::code;
use crate::config::{Config, RouterCfg};
use crate::model::RosRow;
use crate::routeros::{Connector, RouterSession};
use crate::topology_manager::policy_draft::router_id;
use crate::topology_manager::reader::Table;
use std::collections::{BTreeMap, HashMap, VecDeque};
use std::time::Duration;
use tokio::time::{Instant, timeout_at};
use web_contract::source_studio::{
    AccessProfileView, CANDIDATE_TTL_SECS, InterfaceState, JOB_DEADLINE_SECS, MAX_SNAPSHOT_BYTES,
    MAX_TABLE_ROWS, ReadState, RouteEvidence, RouterSourceConfig, SnapshotRef, SourceKind,
    StudioCache, TableEvidence, TableId, VERSION, WanEvidence,
};

/// Snapshots kept for each router. An older one expires first.
const KEEP: usize = 2;
/// WAN evidence bounds for one router, so that the config view fits the reply budget.
const MAX_WAN: usize = 32;
const MAX_ROUTES: usize = 4;

const ALL: [Table; 11] = [
    Table::Interfaces,
    Table::Routes,
    Table::PppSecrets,
    Table::PppProfiles,
    Table::PppActive,
    Table::DhcpServers,
    Table::DhcpLeases,
    Table::HotspotUsers,
    Table::HotspotServers,
    Table::HotspotProfiles,
    Table::HotspotActive,
];

pub fn table_id(t: Table) -> TableId {
    match t {
        Table::Interfaces => TableId::Interfaces,
        Table::Routes => TableId::Routes,
        Table::PppSecrets => TableId::PppSecrets,
        Table::PppProfiles => TableId::PppProfiles,
        Table::PppActive => TableId::PppActive,
        Table::DhcpServers => TableId::DhcpServers,
        Table::DhcpLeases => TableId::DhcpLeases,
        Table::HotspotUsers => TableId::HotspotUsers,
        Table::HotspotServers => TableId::HotspotServers,
        Table::HotspotProfiles => TableId::HotspotProfiles,
        Table::HotspotActive => TableId::HotspotActive,
    }
}

/// The allowlisted read plan of each source. The browser cannot add a table or a field.
fn plan(kind: SourceKind) -> &'static [Table] {
    match kind {
        SourceKind::Ppp => &[Table::PppSecrets, Table::PppProfiles, Table::PppActive],
        SourceKind::Dhcp => &[Table::DhcpServers, Table::DhcpLeases],
        SourceKind::Hotspot => &[
            Table::HotspotUsers,
            Table::HotspotServers,
            Table::HotspotProfiles,
            Table::HotspotActive,
        ],
        SourceKind::Wan => &[Table::Interfaces, Table::Routes],
    }
}

/// One collection. `Debug` is not derived: the rows can hold account data.
pub struct Collected {
    pub reference: SnapshotRef,
    tables: BTreeMap<Table, Vec<RosRow>>,
}

impl Collected {
    /// The rows of a complete or partial table. Check the table state first.
    pub fn rows(&self, table: Table) -> Option<&[RosRow]> {
        self.tables.get(&table).map(Vec::as_slice)
    }

    /// The complete tables only: a partial table is never evidence of absence.
    pub fn tables_snapshot(&self) -> BTreeMap<Table, Vec<RosRow>> {
        self.tables
            .iter()
            .filter(|(t, _)| {
                self.reference
                    .tables
                    .iter()
                    .any(|e| e.table == table_id(**t) && e.state == ReadState::Complete)
            })
            .map(|(t, rows)| (*t, rows.clone()))
            .collect()
    }
}

/// The last good WAN reads of one router, and the state of the latest read.
#[derive(Default)]
struct WanState {
    interfaces: Option<(u64, Vec<RosRow>)>,
    routes: Option<(u64, Vec<RosRow>)>,
    interfaces_read: Option<ReadState>,
    routes_read: Option<ReadState>,
}

/// Studio evidence in memory. A service restart drops it.
#[derive(Default)]
pub struct Studio {
    snapshots: HashMap<String, VecDeque<Collected>>,
    wan: HashMap<String, WanState>,
    seq: u64,
}

fn now_secs() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_secs())
}

fn evidence(
    table: Table,
    state: ReadState,
    records: u64,
    at: u64,
    issue: Option<&str>,
) -> TableEvidence {
    TableEvidence {
        table: table_id(table),
        state,
        records,
        collected_at: Some(at),
        issue_code: issue.map(str::to_owned),
    }
}

impl Studio {
    /// Read the tables of `sources` from one enabled router. A disabled router, a changed
    /// config, or a bad request is refused before any connection. The answer is the reference.
    pub async fn collect<C: Connector>(
        &mut self,
        connector: &C,
        cfg: &Config,
        loaded_digest: &str,
        id: &str,
        expected_digest: &str,
        sources: &[SourceKind],
    ) -> Result<SnapshotRef, String> {
        let router = cfg
            .routers
            .iter()
            .find(|r| router_id(&r.name) == id)
            .ok_or("This router is not in the loaded configuration.")?;
        if !router.enabled {
            return Err("The router is disabled. LQoSync makes no connection to it.".into());
        }
        if expected_digest != loaded_digest {
            return Err(
                "The loaded configuration changed. Load the router again, then collect.".into(),
            );
        }
        if sources.is_empty() || (1..sources.len()).any(|i| sources[..i].contains(&sources[i])) {
            return Err("Name each source once.".into());
        }
        let planned: Vec<Table> = ALL
            .into_iter()
            .filter(|t| sources.iter().any(|k| plan(*k).contains(t)))
            .collect();
        let start = now_secs();
        let started = Instant::now();
        let deadline = started + Duration::from_secs(JOB_DEADLINE_SECS);
        let at = || start + started.elapsed().as_secs();
        let mut states: BTreeMap<Table, TableEvidence> = BTreeMap::new();
        let mut tables: BTreeMap<Table, Vec<RosRow>> = BTreeMap::new();
        match timeout_at(deadline, connector.connect(router, &cfg.router_access)).await {
            Err(_) => {
                for &t in &planned {
                    states.insert(t, evidence(t, ReadState::Failed, 0, at(), Some("timeout")));
                }
            }
            Ok(Err(e)) => {
                for &t in &planned {
                    states.insert(t, evidence(t, ReadState::Failed, 0, at(), Some(code(&e))));
                }
            }
            Ok(Ok(mut session)) => {
                let mut bytes = 0u64;
                for &t in &planned {
                    let (state, rows, issue) = if bytes >= MAX_SNAPSHOT_BYTES {
                        (ReadState::Partial, Vec::new(), Some("byte_limit"))
                    } else {
                        read(&mut session, t, deadline, &mut bytes).await
                    };
                    states.insert(t, evidence(t, state, rows.len() as u64, at(), issue));
                    if state != ReadState::Failed {
                        tables.insert(t, rows);
                    }
                }
            }
        }
        let collected_to = at();
        self.remember_wan(id, &states, &tables, collected_to);
        self.seq += 1;
        let reference = SnapshotRef {
            id: format!("snap-{}-{}", self.seq, start),
            router_id: id.into(),
            config_digest: loaded_digest.into(),
            collected_from: start,
            collected_to,
            expires_at: collected_to + CANDIDATE_TTL_SECS,
            tables: ALL
                .into_iter()
                .map(|t| {
                    states.remove(&t).unwrap_or(TableEvidence {
                        table: table_id(t),
                        state: ReadState::NotRequested,
                        records: 0,
                        collected_at: None,
                        issue_code: None,
                    })
                })
                .collect(),
        };
        let kept = self.snapshots.entry(id.into()).or_default();
        kept.retain(|c| c.reference.expires_at > collected_to);
        while kept.len() >= KEEP {
            kept.pop_front();
        }
        kept.push_back(Collected {
            reference: reference.clone(),
            tables,
        });
        Ok(reference)
    }

    /// The newest kept snapshot of one router that has not expired.
    pub fn latest(&self, router_id: &str, now: u64) -> Option<&Collected> {
        self.snapshots
            .get(router_id)?
            .iter()
            .rev()
            .find(|c| c.reference.expires_at > now)
    }

    /// One kept snapshot that has not expired.
    pub fn snapshot(&self, id: &str, now: u64) -> Option<&Collected> {
        self.snapshots
            .values()
            .flatten()
            .find(|c| c.reference.id == id && c.reference.expires_at > now)
    }

    /// A complete read replaces the last good rows. Any other read keeps them.
    fn remember_wan(
        &mut self,
        id: &str,
        states: &BTreeMap<Table, TableEvidence>,
        tables: &BTreeMap<Table, Vec<RosRow>>,
        at: u64,
    ) {
        if !states.contains_key(&Table::Interfaces) && !states.contains_key(&Table::Routes) {
            return;
        }
        let wan = self.wan.entry(id.into()).or_default();
        for (table, last, read) in [
            (
                Table::Interfaces,
                &mut wan.interfaces,
                &mut wan.interfaces_read,
            ),
            (Table::Routes, &mut wan.routes, &mut wan.routes_read),
        ] {
            let Some(e) = states.get(&table) else {
                continue;
            };
            *read = Some(e.state);
            if e.state == ReadState::Complete {
                *last = Some((at, tables.get(&table).cloned().unwrap_or_default()));
            }
        }
    }

    fn wan_evidence(&self, id: &str) -> Vec<WanEvidence> {
        let Some(wan) = self.wan.get(id) else {
            return Vec::new();
        };
        let Some((seen, interfaces)) = &wan.interfaces else {
            return Vec::new();
        };
        let read_state = match (wan.interfaces_read, wan.routes_read) {
            (Some(s), _) if s != ReadState::Complete => s,
            (_, Some(s)) => s,
            _ => ReadState::NotRequested,
        };
        let field = |row: &RosRow, k: &str| row.get(k).map_or("", String::as_str).to_owned();
        interfaces
            .iter()
            .filter(|row| {
                let (name, kind) = (field(row, "name"), field(row, "type"));
                // Server-side dynamic tunnels (`pppoe-in` and others) are subscriber sessions.
                !name.is_empty() && !kind.ends_with("-in")
            })
            .take(MAX_WAN)
            .map(|row| {
                let name = field(row, "name");
                let routes = wan
                    .routes
                    .iter()
                    .flat_map(|(at, rows)| rows.iter().map(move |r| (*at, r)))
                    .filter(|(_, r)| field(r, "dst-address") == "0.0.0.0/0")
                    .filter_map(|(at, r)| {
                        let resolved = route_interface(r)?;
                        (resolved == name).then(|| RouteEvidence {
                            routing_table: Some(field(r, "routing-table"))
                                .filter(|t| !t.is_empty())
                                .unwrap_or_else(|| "main".into()),
                            active: field(r, "active") == "true",
                            gateway: Some(field(r, "gateway")).filter(|g| !g.is_empty()),
                            resolved_interface: Some(resolved),
                            observed_at: Some(at),
                        })
                    })
                    .take(MAX_ROUTES)
                    .collect();
                WanEvidence {
                    interface_state: if field(row, "disabled") == "true" {
                        InterfaceState::Disabled
                    } else if field(row, "running") == "true" {
                        InterfaceState::Running
                    } else {
                        InterfaceState::Stopped
                    },
                    interface: name,
                    routes,
                    read_state,
                    last_observed_at: Some(*seen),
                    // Saved decisions arrive with plan task B4.
                    decision: None,
                }
            })
            .collect()
    }

    /// The `source-config` view of one router. It reads no router and no secret value.
    pub fn router_config(
        &self,
        r: &RouterCfg,
        file_digest: &str,
        loaded_digest: &str,
        revision: Option<u64>,
        now: u64,
    ) -> RouterSourceConfig {
        let id = router_id(&r.name);
        let mut access_profiles = vec![AccessProfileView {
            id: crate::config::DEFAULT_PROFILE.into(),
            label: "Router credential".into(),
            credential_set: crate::router_view::configured(&r.password_file),
            supported_writes: Vec::new(),
        }];
        access_profiles.extend(r.access_profiles.iter().map(|p| AccessProfileView {
            id: p.id.clone(),
            label: p.label.clone(),
            credential_set: crate::router_view::configured(&p.password_file),
            supported_writes: p.supported_writes.clone(),
        }));
        RouterSourceConfig {
            version: VERSION,
            router_id: id.clone(),
            config_digest: file_digest.into(),
            loaded_config_digest: loaded_digest.into(),
            policy_revision: revision.map(|v| v.to_string()),
            enabled: r.enabled,
            access_profiles,
            // Saved source policies arrive with plan task B4.
            policies: Vec::new(),
            snapshots: self
                .snapshots
                .get(&id)
                .into_iter()
                .flatten()
                .filter(|c| c.reference.expires_at > now)
                .map(|c| c.reference.clone())
                .collect(),
            wan_candidates: self.wan_evidence(&id),
        }
    }

    /// The `source-studio.json` cache: every configured router.
    pub fn cache(
        &self,
        cfg: &Config,
        file_digest: &str,
        loaded_digest: &str,
        revision: Option<u64>,
        now: u64,
    ) -> StudioCache {
        StudioCache {
            version: VERSION,
            updated: now,
            routers: cfg
                .routers
                .iter()
                .map(|r| self.router_config(r, file_digest, loaded_digest, revision, now))
                .collect(),
        }
    }
}

/// The interface of a route: `address%interface` in `immediate-gw`, or an interface gateway.
fn route_interface(r: &RosRow) -> Option<String> {
    let field = |k: &str| r.get(k).map_or("", String::as_str);
    if let Some((_, iface)) = field("immediate-gw").split_once('%') {
        return Some(iface.into());
    }
    let gw = field("gateway");
    (!gw.is_empty() && gw.parse::<std::net::IpAddr>().is_err()).then(|| gw.into())
}

/// One table, with the row and byte limits. A limit gives `partial`, never a short complete table.
async fn read<S: RouterSession>(
    session: &mut S,
    table: Table,
    deadline: Instant,
    bytes: &mut u64,
) -> (ReadState, Vec<RosRow>, Option<&'static str>) {
    let (path, fields) = table.command();
    // shortcut: the print loads the whole reply before the row limit applies; a streamed
    // print could stop at the limit.
    let mut rows = match timeout_at(deadline, session.print(path, fields)).await {
        Err(_) => return (ReadState::Failed, Vec::new(), Some("timeout")),
        Ok(Err(e)) => return (ReadState::Failed, Vec::new(), Some(code(&e))),
        Ok(Ok(rows)) => rows,
    };
    let allowed: std::collections::BTreeSet<&str> = fields.split(',').collect();
    if rows
        .iter()
        .any(|row| row.keys().any(|k| !allowed.contains(k.as_str())))
    {
        return (ReadState::Failed, Vec::new(), Some("protocol"));
    }
    let mut issue = None;
    if rows.len() as u64 > MAX_TABLE_ROWS {
        rows.truncate(MAX_TABLE_ROWS as usize);
        issue = Some("row_limit");
    }
    let mut kept = 0;
    for row in &rows {
        let size: u64 = row.iter().map(|(k, v)| (k.len() + v.len()) as u64).sum();
        if *bytes + size > MAX_SNAPSHOT_BYTES {
            issue = Some("byte_limit");
            break;
        }
        *bytes += size;
        kept += 1;
    }
    rows.truncate(kept);
    match issue {
        Some(_) => (ReadState::Partial, rows, issue),
        None => (ReadState::Complete, rows, None),
    }
}

#[cfg(test)]
#[path = "snapshots_tests.rs"]
mod tests;
