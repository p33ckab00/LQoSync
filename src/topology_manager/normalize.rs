//! Current account evidence. This module does not authorize permanent subscriber identity.

use super::model::SourceKind;
use super::observation::Endpoint;
use super::reader::{RouterRead, Table};
use crate::model::RosRow;
use crate::routeros::ReadError;
use crate::sources::Excluder;
use std::collections::{BTreeSet, HashMap, HashSet};
use std::net::IpAddr;
use tokio::time::Instant;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Issue {
    MissingField,
    InvalidField,
    DuplicateName,
    MissingAccount,
    DisabledAccount,
    MissingProfile,
    MissingServer,
    DisabledServer,
    UnsupportedAuthentication,
    ConflictingIp,
}

#[derive(Debug, PartialEq, Eq)]
pub enum NormalizeError {
    Evidence(ReadError),
    Row {
        table: Table,
        index: usize,
        issue: Issue,
    },
}

impl std::fmt::Display for NormalizeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Evidence(error) => write!(f, "{error}"),
            Self::Row {
                table,
                index,
                issue,
            } => write!(f, "{table:?} row {index}: {issue:?}"),
        }
    }
}
impl std::error::Error for NormalizeError {}

#[derive(Debug, PartialEq, Eq)]
pub enum AuthorityEvidence {
    LocalAccount {
        name: String,
        comment: Option<String>,
    },
    EndpointOnly {
        name: Option<String>,
        client_id: Option<String>,
        comment: Option<String>,
    },
}

#[derive(PartialEq, Eq)]
pub struct CurrentAccount {
    pub kind: SourceKind,
    pub row_ref: String,
    pub authority: AuthorityEvidence,
    pub profile_ref: Option<String>,
    pub direct_rate_limit: Option<String>,
    pub service_refs: Vec<String>,
    pub endpoints: Vec<Endpoint>,
    pub active_comments: Vec<Option<String>>,
}

#[derive(Debug, PartialEq, Eq)]
pub struct CurrentProfile {
    pub kind: SourceKind,
    pub row_ref: String,
    pub name: String,
    pub rate_limit: Option<String>,
    pub comment: Option<String>,
    /// An active session of this read uses this profile.
    pub active: bool,
}

#[derive(Debug, PartialEq, Eq)]
pub struct CurrentService {
    pub kind: SourceKind,
    pub row_ref: String,
    pub name: String,
    pub disabled: bool,
    pub comment: Option<String>,
    /// An account of this read (a lease or a session) names this service.
    pub active: bool,
}

pub struct CurrentRouter {
    router_id: String,
    generation: u64,
    expires: Instant,
    accounts: Vec<CurrentAccount>,
    profiles: Vec<CurrentProfile>,
    completed: BTreeSet<SourceKind>,
    services: Vec<CurrentService>,
}

impl CurrentRouter {
    pub(super) fn was_read(&self, kind: SourceKind) -> bool {
        self.completed.contains(&kind)
    }
    pub(super) fn expires(&self) -> Instant {
        self.expires
    }
    pub fn router_id(&self) -> &str {
        &self.router_id
    }
    pub fn generation(&self) -> u64 {
        self.generation
    }
    fn fresh(&self) -> Result<(), NormalizeError> {
        if Instant::now() >= self.expires {
            return Err(NormalizeError::Evidence(ReadError::Timeout(
                "normalized evidence expired".into(),
            )));
        }
        Ok(())
    }
    pub fn accounts(&self) -> Result<&[CurrentAccount], NormalizeError> {
        self.fresh()?;
        Ok(&self.accounts)
    }
    pub fn profiles(&self) -> Result<&[CurrentProfile], NormalizeError> {
        self.fresh()?;
        Ok(&self.profiles)
    }
    pub fn services(&self) -> Result<&[CurrentService], NormalizeError> {
        self.fresh()?;
        Ok(&self.services)
    }
}

pub(super) fn normalize_configured(
    read: &RouterRead,
    global: &crate::config::Exclude,
    local: &crate::config::Exclude,
) -> Result<CurrentRouter, NormalizeError> {
    normalize_inner(read, Some(&Excluder::from_lists(global, local)))
}

pub fn normalize(read: &RouterRead) -> Result<CurrentRouter, NormalizeError> {
    normalize_inner(read, None)
}

fn excluded_ip(exclusions: &Excluder<'_>, endpoint: &Endpoint) -> bool {
    endpoint
        .ips
        .iter()
        .any(|ip| matches!(ip, IpAddr::V4(address) if exclusions.ipv4(*address)))
}

fn normalize_inner(
    read: &RouterRead,
    exclusions: Option<&Excluder<'_>>,
) -> Result<CurrentRouter, NormalizeError> {
    let mut current = CurrentRouter {
        router_id: read.router_id().into(),
        generation: read.generation(),
        expires: read.expires(),
        accounts: Vec::new(),
        profiles: Vec::new(),
        completed: BTreeSet::new(),
        services: Vec::new(),
    };
    current.fresh()?;
    let tables: BTreeSet<_> = read.tables().collect();
    let mut ips = HashSet::new();
    for (kind, inventory, profiles, active) in [
        (
            SourceKind::Ppp,
            Table::PppSecrets,
            Table::PppProfiles,
            Table::PppActive,
        ),
        (
            SourceKind::Hotspot,
            Table::HotspotUsers,
            Table::HotspotProfiles,
            Table::HotspotActive,
        ),
    ] {
        if tables.contains(&active) {
            current.completed.insert(kind);
            local_accounts(
                read,
                &mut current,
                kind,
                (inventory, profiles, active),
                &mut ips,
                exclusions,
            )?;
        }
    }
    if tables.contains(&Table::DhcpLeases) {
        current.completed.insert(SourceKind::Dhcp);
        dhcp_accounts(read, &mut current, &mut ips, exclusions)?;
    }
    for account in &mut current.accounts {
        account
            .endpoints
            .sort_unstable_by(|a, b| a.session_id.cmp(&b.session_id));
        account.service_refs.sort_unstable();
        account.service_refs.dedup();
    }
    // A profile or a service is active when an account of this read uses it. RouterOS `.id`
    // values are per table, so profiles and services of each kind are matched apart.
    let profiles: HashSet<(SourceKind, &str)> = current
        .accounts
        .iter()
        .filter_map(|a| Some((a.kind, a.profile_ref.as_deref()?)))
        .collect();
    let services: HashSet<(SourceKind, &str)> = current
        .accounts
        .iter()
        .flat_map(|a| a.service_refs.iter().map(move |s| (a.kind, s.as_str())))
        .collect();
    for profile in &mut current.profiles {
        profile.active = profiles.contains(&(profile.kind, profile.row_ref.as_str()));
    }
    for service in &mut current.services {
        service.active = services.contains(&(service.kind, service.row_ref.as_str()));
    }
    current
        .accounts
        .sort_unstable_by(|a, b| (a.kind, &a.row_ref).cmp(&(b.kind, &b.row_ref)));
    current
        .profiles
        .sort_unstable_by(|a, b| (a.kind, &a.row_ref).cmp(&(b.kind, &b.row_ref)));
    current
        .services
        .sort_unstable_by(|a, b| (a.kind, &a.row_ref).cmp(&(b.kind, &b.row_ref)));
    current.fresh()?;
    Ok(current)
}

fn claim(
    endpoint: &Endpoint,
    ips: &mut HashSet<IpAddr>,
    table: Table,
    index: usize,
) -> Result<(), NormalizeError> {
    for ip in &endpoint.ips {
        if !ips.insert(*ip) {
            return Err(error(table, index, Issue::ConflictingIp));
        }
    }
    Ok(())
}

fn error(table: Table, index: usize, issue: Issue) -> NormalizeError {
    NormalizeError::Row {
        table,
        index,
        issue,
    }
}
fn optional(row: &RosRow, key: &str) -> Option<String> {
    row.get(key).filter(|v| !v.trim().is_empty()).cloned()
}
fn required<'a>(
    row: &'a RosRow,
    table: Table,
    index: usize,
    key: &str,
) -> Result<&'a str, NormalizeError> {
    row.get(key)
        .map(String::as_str)
        .filter(|v| !v.trim().is_empty())
        .ok_or_else(|| error(table, index, Issue::MissingField))
}
fn disabled(row: &RosRow, table: Table, index: usize) -> Result<bool, NormalizeError> {
    match required(row, table, index, "disabled")? {
        "true" | "yes" => Ok(true),
        "false" | "no" => Ok(false),
        _ => Err(error(table, index, Issue::InvalidField)),
    }
}
fn named(rows: &[RosRow], table: Table) -> Result<HashMap<&str, (usize, &RosRow)>, NormalizeError> {
    let mut map = HashMap::with_capacity(rows.len());
    for (index, row) in rows.iter().enumerate() {
        let name = required(row, table, index, "name")?;
        if map.insert(name, (index, row)).is_some() {
            return Err(error(table, index, Issue::DuplicateName));
        }
    }
    Ok(map)
}
fn rows(read: &RouterRead, table: Table) -> Result<&[RosRow], NormalizeError> {
    read.rows(table).map_err(NormalizeError::Evidence)
}
fn mac(value: &str, table: Table, index: usize) -> Result<String, NormalizeError> {
    let normalized = crate::config::normalize_mac(value)
        .ok_or_else(|| error(table, index, Issue::InvalidField))?;
    let first = u8::from_str_radix(&normalized[..2], 16)
        .map_err(|_| error(table, index, Issue::InvalidField))?;
    if normalized == "00:00:00:00:00:00" || first & 1 != 0 {
        return Err(error(table, index, Issue::InvalidField));
    }
    Ok(normalized)
}
fn endpoint(row: &RosRow, table: Table, index: usize) -> Result<Endpoint, NormalizeError> {
    let ip_key = if table == Table::DhcpLeases {
        "active-address"
    } else {
        "address"
    };
    let ip: IpAddr = required(row, table, index, ip_key)?
        .parse()
        .map_err(|_| error(table, index, Issue::InvalidField))?;
    if ip.is_unspecified() || ip.is_multicast() || matches!(ip,IpAddr::V4(v4) if v4.is_broadcast())
    {
        return Err(error(table, index, Issue::InvalidField));
    }
    let mac = if table == Table::PppActive {
        // PPP caller IDs can be phone numbers or other transport identifiers.
        row.get("caller-id")
            .and_then(|v| crate::config::normalize_mac(v))
            .map(|v| mac(&v, table, index))
            .transpose()?
    } else if table == Table::DhcpLeases {
        // A lease without a valid client MAC fails only its own identity check, not the router read.
        row.get("active-mac-address")
            .and_then(|v| mac(v, table, index).ok())
    } else {
        Some(mac(
            required(row, table, index, "mac-address")?,
            table,
            index,
        )?)
    };
    Ok(Endpoint {
        session_id: required(row, table, index, ".id")?.into(),
        ips: vec![ip],
        mac,
    })
}

fn local_accounts(
    read: &RouterRead,
    current: &mut CurrentRouter,
    kind: SourceKind,
    tables: (Table, Table, Table),
    ips: &mut HashSet<IpAddr>,
    exclusions: Option<&Excluder<'_>>,
) -> Result<(), NormalizeError> {
    let (inventory, profile_table, active) = tables;
    let servers = if kind == SourceKind::Hotspot {
        Some(service_inventory(
            read,
            current,
            Table::HotspotServers,
            kind,
        )?)
    } else {
        None
    };
    let accounts = named(rows(read, inventory)?, inventory)?;
    let profile_rows = rows(read, profile_table)?;
    let profiles = named(profile_rows, profile_table)?;
    let mut excluded_profiles = HashSet::new();
    for (index, row) in profile_rows.iter().enumerate() {
        let name = required(row, profile_table, index, "name")?;
        let reference = required(row, profile_table, index, ".id")?;
        if kind == SourceKind::Ppp && exclusions.is_some_and(|ex| ex.profile(name)) {
            excluded_profiles.insert(reference);
        }
        current.profiles.push(CurrentProfile {
            kind,
            row_ref: reference.into(),
            name: name.into(),
            rate_limit: optional(row, "rate-limit"),
            comment: optional(row, "comment"),
            active: false,
        });
    }
    let mut grouped: HashMap<String, usize> = HashMap::new();
    for (index, row) in rows(read, active)?.iter().enumerate() {
        let name = required(
            row,
            active,
            index,
            if kind == SourceKind::Ppp {
                "name"
            } else {
                "user"
            },
        )?;
        let mut endpoint_only = false;
        let mut trial = false;
        if kind == SourceKind::Hotspot {
            for method in required(row, active, index, "login-by")?
                .split(',')
                .map(str::trim)
            {
                match method {
                    "http-chap" | "http-pap" | "https" | "cookie" | "mac-cookie" => {}
                    "mac" => endpoint_only = true,
                    "trial" => {
                        endpoint_only = true;
                        trial = true;
                    }
                    _ => return Err(error(active, index, Issue::UnsupportedAuthentication)),
                }
            }
        }
        // Trial presence does not authenticate a local user, even when names match.
        let account = if trial { None } else { accounts.get(name) };
        if account.is_none() && !trial {
            return Err(error(active, index, Issue::MissingAccount));
        }
        let (row_ref, profile_ref, comment) = if let Some(&(account_index, account)) = account {
            if disabled(account, inventory, account_index)? {
                return Err(error(active, index, Issue::DisabledAccount));
            }
            let profile_name = required(account, inventory, account_index, "profile")?;
            let &(profile_index, profile) = profiles
                .get(profile_name)
                .ok_or_else(|| error(active, index, Issue::MissingProfile))?;
            (
                required(account, inventory, account_index, ".id")?.to_owned(),
                Some(required(profile, profile_table, profile_index, ".id")?.to_owned()),
                optional(account, "comment"),
            )
        } else {
            // Trial sessions have no local account authority and cannot share an account key.
            (
                format!("session:{}", required(row, active, index, ".id")?),
                None,
                None,
            )
        };
        let authority = if endpoint_only {
            AuthorityEvidence::EndpointOnly {
                name: Some(name.into()),
                client_id: None,
                comment,
            }
        } else {
            AuthorityEvidence::LocalAccount {
                name: name.into(),
                comment,
            }
        };
        let endpoint = endpoint(row, active, index)?;
        claim(&endpoint, ips, active, index)?;
        let service = required(
            row,
            active,
            index,
            if kind == SourceKind::Ppp {
                "service"
            } else {
                "server"
            },
        )?;
        let service = if let Some(servers) = &servers {
            let &(server_index, server) = servers
                .get(service)
                .ok_or_else(|| error(active, index, Issue::MissingServer))?;
            if disabled(server, Table::HotspotServers, server_index)? {
                return Err(error(active, index, Issue::DisabledServer));
            }
            required(server, Table::HotspotServers, server_index, ".id")?.to_owned()
        } else {
            service.to_owned()
        };
        // Validate identity joins, endpoints, and service evidence before an exclusion can hide a row.
        if profile_ref
            .as_deref()
            .is_some_and(|id| excluded_profiles.contains(id))
            || exclusions.is_some_and(|ex| excluded_ip(ex, &endpoint))
        {
            continue;
        }
        if let Some(&position) = grouped.get(&row_ref) {
            let account = &mut current.accounts[position];
            if endpoint_only {
                account.authority = authority;
            }
            account.active_comments.push(optional(row, "comment"));
            account.endpoints.push(endpoint);
            account.service_refs.push(service);
        } else {
            grouped.insert(row_ref.clone(), current.accounts.len());
            current.accounts.push(CurrentAccount {
                kind,
                row_ref,
                authority,
                profile_ref,
                direct_rate_limit: None,
                service_refs: vec![service],
                endpoints: vec![endpoint],
                active_comments: vec![optional(row, "comment")],
            });
        }
    }
    Ok(())
}

fn dhcp_accounts(
    read: &RouterRead,
    current: &mut CurrentRouter,
    ips: &mut HashSet<IpAddr>,
    exclusions: Option<&Excluder<'_>>,
) -> Result<(), NormalizeError> {
    let table = Table::DhcpLeases;
    let servers = service_inventory(read, current, Table::DhcpServers, SourceKind::Dhcp)?;
    let excluded_servers: HashSet<_> = servers
        .keys()
        .copied()
        .filter(|name| exclusions.is_some_and(|ex| ex.server(name)))
        .collect();
    for (index, row) in rows(read, table)?.iter().enumerate() {
        match required(row, table, index, "status")? {
            "waiting" | "testing" | "declined" | "offered" | "authorizing" | "conflict" => continue,
            "bound" => {}
            _ => return Err(error(table, index, Issue::InvalidField)),
        }
        let server_name = required(row, table, index, "server")?;
        let &(server_index, server) = servers
            .get(server_name)
            .ok_or_else(|| error(table, index, Issue::MissingServer))?;
        if disabled(server, Table::DhcpServers, server_index)? {
            return Err(error(table, index, Issue::DisabledServer));
        }
        let endpoint = endpoint(row, table, index)?;
        claim(&endpoint, ips, table, index)?;
        if excluded_servers.contains(server_name)
            || exclusions.is_some_and(|ex| excluded_ip(ex, &endpoint))
        {
            continue;
        }
        current.accounts.push(CurrentAccount {
            kind: SourceKind::Dhcp,
            row_ref: required(row, table, index, ".id")?.into(),
            authority: AuthorityEvidence::EndpointOnly {
                name: optional(row, "host-name"),
                client_id: optional(row, "active-client-id"),
                comment: optional(row, "comment"),
            },
            profile_ref: None,
            direct_rate_limit: optional(row, "rate-limit"),
            service_refs: vec![required(server, Table::DhcpServers, server_index, ".id")?.into()],
            endpoints: vec![endpoint],
            active_comments: vec![],
        });
    }
    Ok(())
}

fn service_inventory<'a>(
    read: &'a RouterRead,
    current: &mut CurrentRouter,
    table: Table,
    kind: SourceKind,
) -> Result<HashMap<&'a str, (usize, &'a RosRow)>, NormalizeError> {
    let inventory = rows(read, table)?;
    let index = named(inventory, table)?;
    for (position, row) in inventory.iter().enumerate() {
        current.services.push(CurrentService {
            kind,
            row_ref: required(row, table, position, ".id")?.into(),
            name: required(row, table, position, "name")?.into(),
            disabled: disabled(row, table, position)?,
            comment: optional(row, "comment"),
            active: false,
        });
    }
    Ok(index)
}

#[cfg(test)]
#[path = "normalize_tests.rs"]
mod tests;
