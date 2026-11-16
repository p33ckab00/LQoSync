//! Current router evidence. Row locators do not establish durable subscriber identity.

use super::catalog::{RouterRole, TopologyCatalog};
use crate::model::RosRow;
use crate::routeros::{ReadError, RouterSession};
use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::time::Duration;
use tokio::time::Instant;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Table {
    Interfaces,
    PppSecrets,
    PppProfiles,
    PppActive,
    DhcpServers,
    DhcpLeases,
    HotspotUsers,
    HotspotServers,
    HotspotProfiles,
    HotspotActive,
}

impl Table {
    pub fn command(self) -> (&'static str, &'static str) {
        match self {
            Self::Interfaces => ("/interface", ".id,name,type,disabled,running,comment"),
            Self::PppSecrets => (
                "/ppp/secret",
                ".id,name,profile,comment,disabled,caller-id,service",
            ),
            Self::PppProfiles => ("/ppp/profile", ".id,name,comment,rate-limit"),
            Self::PppActive => ("/ppp/active", ".id,name,address,caller-id,service,comment"),
            Self::DhcpServers => ("/ip/dhcp-server", ".id,name,comment,disabled"),
            Self::DhcpLeases => (
                "/ip/dhcp-server/lease",
                ".id,server,status,active-address,active-mac-address,active-client-id,host-name,comment,rate-limit,dynamic",
            ),
            Self::HotspotUsers => ("/ip/hotspot/user", ".id,name,profile,comment,disabled"),
            Self::HotspotServers => ("/ip/hotspot", ".id,name,disabled"),
            Self::HotspotProfiles => ("/ip/hotspot/user/profile", ".id,name,rate-limit"),
            Self::HotspotActive => (
                "/ip/hotspot/active",
                ".id,user,address,mac-address,server,login-by",
            ),
        }
    }
}

pub struct ReadPlan {
    router_id: String,
    inventory: BTreeSet<Table>,
    active: BTreeSet<Table>,
}

impl ReadPlan {
    pub fn for_router(catalog: &TopologyCatalog, router_id: &str) -> anyhow::Result<Self> {
        catalog.validate()?;
        let router = catalog
            .routers
            .iter()
            .find(|r| r.id == router_id)
            .ok_or_else(|| anyhow::anyhow!("unknown catalog router"))?;
        let mut plan = Self {
            router_id: router_id.into(),
            inventory: BTreeSet::new(),
            active: BTreeSet::new(),
        };
        for role in &router.roles {
            match role {
                RouterRole::Wan => {
                    plan.inventory.insert(Table::Interfaces);
                }
                RouterRole::Ppp => {
                    plan.inventory
                        .extend([Table::PppSecrets, Table::PppProfiles]);
                    plan.active.insert(Table::PppActive);
                }
                RouterRole::Dhcp => {
                    plan.inventory.insert(Table::DhcpServers);
                    plan.active.insert(Table::DhcpLeases);
                }
                RouterRole::Hotspot => {
                    plan.inventory.extend([
                        Table::HotspotUsers,
                        Table::HotspotServers,
                        Table::HotspotProfiles,
                    ]);
                    plan.active.insert(Table::HotspotActive);
                }
            }
        }
        Ok(plan)
    }
}

/// A complete read whose lifetime starts at the first command, not the last reply.
/// No serializer or Debug implementation exposes account rows by accident.
pub struct RouterRead {
    router_id: String,
    generation: u64,
    expires: Instant,
    tables: BTreeMap<Table, Vec<RosRow>>,
}

impl RouterRead {
    pub(super) fn tables(&self) -> impl Iterator<Item = Table> + '_ {
        self.tables.keys().copied()
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
    pub fn rows(&self, table: Table) -> Result<&[RosRow], ReadError> {
        if Instant::now() >= self.expires {
            return Err(timeout("router evidence expired"));
        }
        self.tables
            .get(&table)
            .map(Vec::as_slice)
            .ok_or_else(|| protocol("table was not read"))
    }
}

/// Keep one guard per router. An unsuccessful generation cannot be replayed.
pub struct ReadGuard {
    router_id: String,
    last_generation: u64,
}

impl ReadGuard {
    pub fn new(router_id: String) -> anyhow::Result<Self> {
        anyhow::ensure!(!router_id.trim().is_empty(), "router ID is required");
        Ok(Self {
            router_id,
            last_generation: 0,
        })
    }

    /// Return the session only after success. Failure or cancellation drops it.
    pub async fn read<S: RouterSession>(
        &mut self,
        session_slot: &mut Option<S>,
        plan: &ReadPlan,
        generation: u64,
        api_timeout: Duration,
        max_age: Duration,
    ) -> Result<RouterRead, ReadError> {
        let mut session = session_slot
            .take()
            .ok_or_else(|| protocol("router session is not connected"))?;
        let result = self
            .read_current(&mut session, plan, generation, api_timeout, max_age)
            .await;
        if result.is_ok() {
            *session_slot = Some(session);
        }
        result
    }

    async fn read_current<S: RouterSession>(
        &mut self,
        session: &mut S,
        plan: &ReadPlan,
        generation: u64,
        api_timeout: Duration,
        max_age: Duration,
    ) -> Result<RouterRead, ReadError> {
        if generation == 0 || generation <= self.last_generation {
            return Err(protocol("router read generation did not advance"));
        }
        self.last_generation = generation;
        if plan.router_id != self.router_id {
            return Err(protocol("read plan belongs to another router"));
        }
        if api_timeout.is_zero() || max_age.is_zero() {
            return Err(protocol("read time limits must be positive"));
        }
        let expires = Instant::now()
            .checked_add(max_age)
            .ok_or_else(|| protocol("router read age exceeds clock range"))?;
        let mut tables = BTreeMap::new();
        for &table in plan.inventory.iter().chain(&plan.active) {
            tables.insert(
                table,
                read_table(session, table, api_timeout, expires).await?,
            );
        }
        for &table in &plan.inventory {
            let after = read_table(session, table, api_timeout, expires).await?;
            // Compare requested configuration by current row locator. Reply order has no meaning.
            let before = index_rows(&tables[&table])?;
            let after = index_rows(&after)?;
            if before != after {
                return Err(protocol("router inventory changed during read"));
            }
        }
        if Instant::now() >= expires {
            return Err(timeout("router evidence expired during read"));
        }
        Ok(RouterRead {
            router_id: self.router_id.clone(),
            generation,
            expires,
            tables,
        })
    }
}

async fn read_table<S: RouterSession>(
    session: &mut S,
    table: Table,
    api_timeout: Duration,
    expires: Instant,
) -> Result<Vec<RosRow>, ReadError> {
    let now = Instant::now();
    if now >= expires {
        return Err(timeout("router read age exceeded"));
    }
    let (path, fields) = table.command();
    let limit = (now + api_timeout.min(expires - now)).min(expires);
    let rows = tokio::time::timeout_at(limit, session.print(path, fields))
        .await
        .map_err(|_| timeout("router command exceeded read time limit"))??;
    let allowed: BTreeSet<_> = fields.split(',').collect();
    if rows
        .iter()
        .any(|row| row.keys().any(|key| !allowed.contains(key.as_str())))
    {
        return Err(protocol("router returned unrequested fields"));
    }
    index_rows(&rows)?;
    Ok(rows)
}

fn index_rows(rows: &[RosRow]) -> Result<HashMap<&str, &RosRow>, ReadError> {
    let mut index = HashMap::with_capacity(rows.len());
    for row in rows {
        let id = row
            .get(".id")
            .map(String::as_str)
            .filter(|id| !id.trim().is_empty())
            .ok_or_else(|| protocol("router row has no current locator"))?;
        if index.insert(id, row).is_some() {
            return Err(protocol("router returned duplicate row locators"));
        }
    }
    Ok(index)
}

fn protocol(message: &str) -> ReadError {
    ReadError::Protocol(message.into())
}
fn timeout(message: &str) -> ReadError {
    ReadError::Timeout(message.into())
}

#[cfg(test)]
#[path = "reader_tests.rs"]
mod tests;
