//! Operator-confirmed mappings from the live legacy pair to a fresh candidate.
//! Suggestions use the PPP secret name or the DHCP lease MAC. An IP address never suggests a mapping.

use super::migration::{MigrationManifest, NodeMapping, RowIdentity, RowMapping, fingerprint};
use super::ownership::StaticAttachment;
use super::publication::PreparedPair;
use crate::model::CsvRow;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet, HashMap};
type ById<K, V> = BTreeMap<K, V>;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SuggestRule {
    CircuitNameEqualsPppName,
    MacEqualsLeaseMac,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum RowDecision {
    Undecided,
    Map {
        account_ref: String,
        rule: Option<SuggestRule>,
        confirmed: bool,
    },
    Protect,
    Drop,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RowEntry {
    pub legacy: RowIdentity,
    pub legacy_circuit_name: String,
    pub legacy_device_name: String,
    pub decision: RowDecision,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NodeEntry {
    pub legacy_id: String,
    pub legacy_name: String,
    pub current_id: Option<String>,
    pub confirmed: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MappingFile {
    pub rows: Vec<RowEntry>,
    pub nodes: Vec<NodeEntry>,
    pub static_roots: Vec<String>,
    pub attachments: Vec<StaticAttachment>,
    /// Current accounts with no legacy row. They enter after activation. Information only.
    pub candidate_only: Vec<String>,
}

#[derive(Debug)]
pub struct Drift {
    pub issues: Vec<String>,
}
impl std::fmt::Display for Drift {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "enrollment mapping does not match the fresh data:")?;
        for issue in &self.issues {
            write!(f, "\n- {issue}")?;
        }
        Ok(())
    }
}

/// Current candidate rows by identity `account_ref`.
fn current_by_account(pair: &PreparedPair) -> BTreeMap<String, Vec<&CsvRow>> {
    let mut out: BTreeMap<String, Vec<&CsvRow>> = BTreeMap::new();
    for row in pair.rows() {
        if let Some(identity) = pair.device_identities.get(&row.device_id) {
            out.entry(identity.account_ref.clone())
                .or_default()
                .push(row);
        }
    }
    out
}

fn suggest(row: &CsvRow, accounts: &BTreeMap<String, Vec<&CsvRow>>) -> RowDecision {
    let by_name = format!(
        "ppp-name/v1/{}",
        super::identity::encode_name(&row.circuit_name)
    );
    if accounts.contains_key(&by_name) {
        return RowDecision::Map {
            account_ref: by_name,
            rule: Some(SuggestRule::CircuitNameEqualsPppName),
            confirmed: false,
        };
    }
    if let Some(mac) = crate::config::normalize_mac(&row.mac) {
        let by_mac = format!("dhcp-mac/v1/{mac}");
        if accounts.contains_key(&by_mac) {
            return RowDecision::Map {
                account_ref: by_mac,
                rule: Some(SuggestRule::MacEqualsLeaseMac),
                confirmed: false,
            };
        }
    }
    RowDecision::Undecided
}

fn identity(row: &CsvRow) -> RowIdentity {
    RowIdentity {
        circuit_id: row.circuit_id.clone(),
        device_id: row.device_id.clone(),
    }
}

/// Propose a mapping file. A decision in `previous` for the same legacy row is kept.
pub fn propose(
    pair: &PreparedPair,
    legacy_rows: &[CsvRow],
    legacy_network: &Value,
    previous: Option<&MappingFile>,
) -> anyhow::Result<MappingFile> {
    let accounts = current_by_account(pair);
    let kept: ById<_, _> = previous
        .map(|p| p.rows.iter().map(|r| (&r.legacy, &r.decision)).collect())
        .unwrap_or_default();
    let rows: Vec<_> = legacy_rows
        .iter()
        .map(|row| {
            let legacy = identity(row);
            let decision = kept
                .get(&legacy)
                .map_or_else(|| suggest(row, &accounts), |d| (*d).clone());
            RowEntry {
                legacy,
                legacy_circuit_name: row.circuit_name.clone(),
                legacy_device_name: row.device_name.clone(),
                decision,
            }
        })
        .collect();
    let mapped: BTreeSet<_> = rows
        .iter()
        .filter_map(|r| match &r.decision {
            RowDecision::Map { account_ref, .. } => Some(account_ref.as_str()),
            _ => None,
        })
        .collect();
    let candidate_only = accounts
        .keys()
        .filter(|a| !mapped.contains(a.as_str()))
        .cloned()
        .collect();
    let current_nodes = super::migration::nodes(pair.network())?;
    let old_nodes: HashMap<_, _> = previous
        .map(|p| p.nodes.iter().map(|n| (n.legacy_id.as_str(), n)).collect())
        .unwrap_or_default();
    let mut nodes: Vec<_> = super::migration::nodes(legacy_network)?
        .into_iter()
        .map(|(id, (name, _))| {
            old_nodes.get(id).map_or_else(
                || NodeEntry {
                    legacy_id: id.into(),
                    legacy_name: name.into(),
                    current_id: current_nodes.contains_key(id).then(|| id.to_string()),
                    confirmed: false,
                },
                |n| (*n).clone(),
            )
        })
        .collect();
    nodes.sort_by(|a, b| a.legacy_id.cmp(&b.legacy_id));
    Ok(MappingFile {
        rows,
        nodes,
        static_roots: previous.map(|p| p.static_roots.clone()).unwrap_or_default(),
        attachments: previous.map(|p| p.attachments.clone()).unwrap_or_default(),
        candidate_only,
    })
}

/// Build the manifest from the fresh pair and a fully confirmed file. Any mismatch is drift.
pub fn build_manifest(
    pair: &PreparedPair,
    legacy_rows: &[CsvRow],
    legacy_network: &Value,
    file: &MappingFile,
) -> Result<(MigrationManifest, Vec<String>, Vec<StaticAttachment>), Drift> {
    let mut issues = Vec::new();
    let accounts = current_by_account(pair);
    let live: ById<_, _> = legacy_rows.iter().map(|r| (identity(r), r)).collect();
    let decided: BTreeSet<_> = file.rows.iter().map(|r| r.legacy.clone()).collect();
    for row in legacy_rows {
        if !decided.contains(&identity(row)) {
            issues.push(format!(
                "legacy row {} / {} is not in the mapping file",
                row.circuit_id, row.device_id
            ));
        }
    }
    let (mut rows, mut protected, mut dropped) = (vec![], vec![], vec![]);
    let mut used = BTreeSet::new();
    for entry in &file.rows {
        let label = format!(
            "legacy row {} ({})",
            entry.legacy.circuit_id, entry.legacy_circuit_name
        );
        if !live.contains_key(&entry.legacy) {
            issues.push(format!("{label} is no longer in the live legacy files"));
            continue;
        }
        match &entry.decision {
            RowDecision::Undecided => issues.push(format!("{label} has no decision")),
            RowDecision::Map {
                confirmed: false, ..
            } => {
                issues.push(format!("{label} has an unconfirmed mapping"));
            }
            RowDecision::Map { account_ref, .. } => {
                match accounts.get(account_ref).map(Vec::as_slice) {
                    Some([current]) => {
                        used.insert(account_ref.as_str());
                        rows.push(RowMapping {
                            legacy: entry.legacy.clone(),
                            current: identity(current),
                            legacy_circuit_name: entry.legacy_circuit_name.clone(),
                            legacy_device_name: entry.legacy_device_name.clone(),
                        });
                    }
                    Some(_) => issues.push(format!(
                        "{label}: {account_ref} has several current sessions"
                    )),
                    None => issues.push(format!("{label}: {account_ref} is not current")),
                }
            }
            RowDecision::Protect => protected.push(entry.legacy.clone()),
            RowDecision::Drop => dropped.push(entry.legacy.clone()),
        }
    }
    for account in accounts.keys().filter(|a| !used.contains(a.as_str())) {
        issues.push(format!(
            "current account {account} has no legacy row; it enters after activation"
        ));
    }
    let mut nodes = vec![];
    for node in &file.nodes {
        match (&node.current_id, node.confirmed) {
            (Some(current), true) => nodes.push(NodeMapping {
                legacy_id: node.legacy_id.clone(),
                legacy_name: node.legacy_name.clone(),
                current_id: current.clone(),
            }),
            _ => issues.push(format!(
                "legacy node {} has no confirmed mapping",
                node.legacy_name
            )),
        }
    }
    if !issues.is_empty() {
        return Err(Drift { issues });
    }
    let manifest = MigrationManifest {
        expected_revision: pair.expected_revision(),
        generation: pair.generation(),
        baseline_digest: fingerprint(legacy_rows, legacy_network),
        candidate_digest: fingerprint(pair.rows(), pair.network()),
        rows,
        nodes,
        protected_rows: protected,
        dropped_rows: dropped,
    };
    if let Err(e) = super::legacy_static::adopt(
        pair,
        legacy_rows,
        legacy_network,
        &manifest,
        &file.static_roots,
        &file.attachments,
    ) {
        let mut issues = vec![format!("{e:#}")];
        // Name the managed rows and nodes that differ. Protected rows are checked by adoption itself.
        let mut managed = manifest.clone();
        managed.protected_rows.clear();
        let keep: BTreeSet<_> = manifest.protected_rows.iter().collect();
        let rows: Vec<_> = legacy_rows
            .iter()
            .filter(|r| !keep.contains(&identity(r)))
            .cloned()
            .collect();
        managed.baseline_digest = fingerprint(&rows, legacy_network);
        if let Ok((kept, kept_manifest)) =
            super::migration::without_dropped(&rows, legacy_network, &managed)
            && let Ok(found) =
                super::migration::compare_mapped(pair, &kept, legacy_network, &kept_manifest)
        {
            issues.extend(found.iter().map(|i| format!("{i:?}")));
        }
        return Err(Drift { issues });
    }
    Ok((
        manifest,
        file.static_roots.clone(),
        file.attachments.clone(),
    ))
}

#[cfg(test)]
#[path = "enroll_mapping_tests.rs"]
mod tests;
