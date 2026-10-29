use crate::{
    routeros::{Inventory, Row},
    speeds::{self, ResolvedSpeed, SpeedRules},
};
use anyhow::{Context, Result, ensure};
use serde::Serialize;
use std::collections::{HashMap, HashSet};
#[derive(Debug, Serialize)]
pub struct SpeedEntry {
    pub service: &'static str,
    pub identity: String,
    pub group: String,
    pub ipv4: String,
    pub speed: Option<ResolvedSpeed>,
    pub error: Option<String>,
}
#[derive(Debug, Serialize)]
pub struct SpeedPreview {
    pub entries: Vec<SpeedEntry>,
    pub errors: Vec<String>,
}
impl SpeedPreview {
    pub fn resolved(&self) -> usize {
        self.entries.iter().filter(|e| e.speed.is_some()).count()
    }
    pub fn unresolved(&self) -> usize {
        self.entries.len() - self.resolved()
    }
}
fn text<'a>(row: &'a Row, key: &str) -> &'a str {
    row.get(key).map(|s| s.trim()).unwrap_or("")
}
fn table<'a>(inv: &'a Inventory, path: &str, errors: &mut Vec<String>) -> &'a [Row] {
    if inv.errors.contains_key(path) {
        errors.push(format!("Required table read failed: {path}"));
        return &[];
    }
    match inv.tables.get(path) {
        Some(rows) => rows,
        None => {
            errors.push(format!("Required table is missing: {path}"));
            &[]
        }
    }
}
fn index<'a>(
    rows: &'a [Row],
    label: &str,
    errors: &mut Vec<String>,
) -> HashMap<&'a str, Option<&'a Row>> {
    let mut result = HashMap::with_capacity(rows.len());
    for row in rows {
        let key = text(row, "name");
        if key.is_empty() {
            errors.push(format!("Missing name in {label}"));
            continue;
        }
        result
            .entry(key)
            .and_modify(|v| {
                *v = None;
                errors.push(format!("Duplicate name in {label}: {key}"));
            })
            .or_insert(Some(row));
    }
    result
}
fn lookup<'a>(index: &HashMap<&str, Option<&'a Row>>, key: &str) -> Result<&'a Row> {
    index
        .get(key)
        .with_context(|| {
            format!("Local user mapping is missing: {key}; external/RADIUS policies are unresolved")
        })?
        .with_context(|| format!("Duplicate local mapping: {key}"))
}
type Cache<'a> = HashMap<&'a str, std::result::Result<ResolvedSpeed, String>>;
fn cached(cache: &Cache<'_>, name: &str) -> Result<ResolvedSpeed> {
    cache
        .get(name)
        .with_context(|| format!("Profile is missing: {name}"))?
        .clone()
        .map_err(anyhow::Error::msg)
}
fn push(
    report: &mut SpeedPreview,
    service: &'static str,
    identity: &str,
    group: &str,
    ipv4: &str,
    result: Result<ResolvedSpeed>,
) {
    let (speed, error) = match result {
        Ok(value) => (Some(value), None),
        Err(e) => (None, Some(e.to_string())),
    };
    report.entries.push(SpeedEntry {
        service,
        identity: identity.into(),
        group: group.into(),
        ipv4: ipv4.into(),
        speed,
        error,
    });
}

fn duplicate_keys<'a, K: Eq + std::hash::Hash>(
    rows: impl IntoIterator<Item = &'a Row>,
    key: impl Fn(&'a Row) -> K,
) -> HashSet<K> {
    let mut seen = HashSet::new();
    let mut duplicates = HashSet::new();
    for row in rows {
        let k = key(row);
        if !seen.insert(key(row)) {
            duplicates.insert(k);
        }
    }
    duplicates
}
fn lease_address(row: &Row) -> &str {
    if text(row, "active-address").is_empty() {
        text(row, "address")
    } else {
        text(row, "active-address")
    }
}
fn eligible_lease(row: &Row) -> bool {
    text(row, "disabled") != "true"
        && (text(row, "status").is_empty() || text(row, "status") == "bound")
}

pub fn preview(inv: &Inventory, rules: &SpeedRules) -> Result<SpeedPreview> {
    rules.validate()?;
    let mut report = SpeedPreview {
        entries: vec![],
        errors: vec![],
    };
    if inv.errors.contains_key("collection") {
        report
            .errors
            .push("The inventory collection deadline was exceeded".into());
    }
    let active = table(inv, "/ppp/active/print", &mut report.errors);
    let secrets = index(
        table(inv, "/ppp/secret/print", &mut report.errors),
        "PPP secrets",
        &mut report.errors,
    );
    let profiles = index(
        table(inv, "/ppp/profile/print", &mut report.errors),
        "PPP profiles",
        &mut report.errors,
    );
    let empty = Row::new();
    // Resolve every shared profile once, regardless of subscriber count.
    let profile_cache: Cache<'_> = profiles
        .iter()
        .map(|(&name, row)| {
            let result = row
                .map(|r| {
                    speeds::resolve_ppp(&empty, &empty, Some(r), rules).map_err(|e| e.to_string())
                })
                .unwrap_or_else(|| Err(format!("Duplicate PPP profile: {name}")));
            (name, result)
        })
        .collect();
    let duplicates = duplicate_keys(active, |r| (text(r, "name"), text(r, "address")));
    for row in active {
        let name = text(row, "name");
        let address = text(row, "address");
        let secret = lookup(&secrets, name);
        let profile = secret.as_ref().map(|r| text(r, "profile")).unwrap_or("");
        let result = (|| {
            ensure!(!name.is_empty(), "PPP active identity is missing");
            ensure!(
                !duplicates.contains(&(name, address)),
                "Duplicate PPP active session"
            );
            let secret = secret?;
            ensure!(
                text(secret, "disabled") != "true",
                "PPP local user is disabled"
            );
            if let Some(speed) =
                speeds::comment(text(secret, "comment"), "ppp_secret_comment", rules)?
            {
                return Ok(speed);
            }
            if let Some(speed) = speeds::comment(text(row, "comment"), "ppp_active_comment", rules)?
            {
                return Ok(speed);
            }
            cached(&profile_cache, profile)
        })();
        push(&mut report, "ppp", name, profile, address, result);
    }
    let leases = table(inv, "/ip/dhcp-server/lease/print", &mut report.errors);
    let duplicates = duplicate_keys(leases.iter().filter(|r| eligible_lease(r)), |r| {
        (text(r, "server"), text(r, "mac-address"), lease_address(r))
    });
    for row in leases {
        if text(row, "disabled") == "true" {
            continue;
        }
        let status = text(row, "status");
        if !status.is_empty() && status != "bound" {
            continue;
        }
        let name = text(row, "mac-address");
        let server = text(row, "server");
        let address = if text(row, "active-address").is_empty() {
            text(row, "address")
        } else {
            text(row, "active-address")
        };
        let result = (|| {
            ensure!(status == "bound", "DHCP binding status is missing");
            ensure!(
                !name.is_empty() && !server.is_empty(),
                "DHCP MAC or server identity is missing"
            );
            ensure!(
                !duplicates.contains(&(server, name, address)),
                "Duplicate DHCP lease"
            );
            speeds::resolve_dhcp(row, rules)
        })();
        push(&mut report, "dhcp", name, server, address, result);
    }
    let active = table(inv, "/ip/hotspot/active/print", &mut report.errors);
    let users = index(
        table(inv, "/ip/hotspot/user/print", &mut report.errors),
        "Hotspot users",
        &mut report.errors,
    );
    let profiles = index(
        table(inv, "/ip/hotspot/user/profile/print", &mut report.errors),
        "Hotspot profiles",
        &mut report.errors,
    );
    let profile_cache: Cache<'_> = profiles
        .iter()
        .map(|(&name, row)| {
            let result = row
                .map(|r| speeds::resolve_hotspot(&empty, Some(r), rules).map_err(|e| e.to_string()))
                .unwrap_or_else(|| Err(format!("Duplicate Hotspot profile: {name}")));
            (name, result)
        })
        .collect();
    let duplicates = duplicate_keys(active, |r| (text(r, "user"), text(r, "address")));
    for row in active {
        let name = text(row, "user");
        let address = text(row, "address");
        let user = lookup(&users, name);
        let profile = user.as_ref().map(|r| text(r, "profile")).unwrap_or("");
        let result = (|| {
            ensure!(!name.is_empty(), "Hotspot active identity is missing");
            ensure!(
                !duplicates.contains(&(name, address)),
                "Duplicate Hotspot active session"
            );
            let user = user?;
            ensure!(
                text(user, "disabled") != "true",
                "Hotspot local user is disabled"
            );
            if let Some(speed) =
                speeds::comment(text(user, "comment"), "hotspot_user_comment", rules)?
            {
                return Ok(speed);
            }
            cached(&profile_cache, profile)
        })();
        push(&mut report, "hotspot", name, profile, address, result);
    }
    report.entries.sort_by(|a, b| {
        (a.service, &a.identity, &a.group, &a.ipv4).cmp(&(
            b.service,
            &b.identity,
            &b.group,
            &b.ipv4,
        ))
    });
    report.errors.sort();
    report.errors.dedup();
    Ok(report)
}
