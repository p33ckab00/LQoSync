//! Explicit snapshot mappings. Endpoints never authorize identity continuity.
use super::parity::ParityIssue;
use super::publication::PreparedPair;
use crate::model::CsvRow;
use anyhow::{Context, ensure};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::{HashMap, HashSet};

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RowIdentity {
    pub circuit_id: String,
    pub device_id: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RowMapping {
    pub legacy: RowIdentity,
    pub current: RowIdentity,
    pub legacy_circuit_name: String,
    pub legacy_device_name: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NodeMapping {
    pub legacy_id: String,
    pub legacy_name: String,
    pub current_id: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MigrationManifest {
    pub expected_revision: u64,
    pub generation: u64,
    pub baseline_digest: String,
    pub candidate_digest: String,
    pub rows: Vec<RowMapping>,
    pub nodes: Vec<NodeMapping>,
    pub protected_rows: Vec<RowIdentity>,
    /// Legacy rows that the operator explicitly leaves out of the adopted snapshot.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub dropped_rows: Vec<RowIdentity>,
}
pub fn fingerprint(rows: &[CsvRow], network: &Value) -> String {
    let csv = crate::writer::pair::render_csv(rows, crate::model::Columns::V3);
    let network = crate::writer::pair::render_network_json(network);
    crate::write_receipt::sha256(
        &serde_json::to_vec(&(
            crate::write_receipt::sha256(csv.as_bytes()),
            crate::write_receipt::sha256(network.as_bytes()),
        ))
        .expect("digest strings serialize"),
    )
}
/// Remove explicit dropped rows from the legacy baseline. The returned manifest binds the kept rows.
/// A dropped row must exist once and must not also be mapped or protected.
pub fn without_dropped(
    rows: &[CsvRow],
    network: &Value,
    manifest: &MigrationManifest,
) -> anyhow::Result<(Vec<CsvRow>, MigrationManifest)> {
    ensure!(
        manifest.baseline_digest == fingerprint(rows, network),
        "migration content digest conflict"
    );
    let mut dropped = HashSet::new();
    for row in &manifest.dropped_rows {
        let key = (row.circuit_id.as_str(), row.device_id.as_str());
        ensure!(
            dropped.insert(key)
                && rows
                    .iter()
                    .any(|r| (r.circuit_id.as_str(), r.device_id.as_str()) == key)
                && !manifest.protected_rows.contains(row)
                && !manifest.rows.iter().any(|m| &m.legacy == row),
            "invalid dropped legacy row"
        );
    }
    let kept: Vec<_> = rows
        .iter()
        .filter(|r| !dropped.contains(&(r.circuit_id.as_str(), r.device_id.as_str())))
        .cloned()
        .collect();
    let mut kept_manifest = manifest.clone();
    kept_manifest.dropped_rows.clear();
    kept_manifest.baseline_digest = fingerprint(&kept, network);
    Ok((kept, kept_manifest))
}

pub fn compare_mapped(
    pair: &PreparedPair,
    rows: &[CsvRow],
    network: &Value,
    manifest: &MigrationManifest,
) -> anyhow::Result<Vec<ParityIssue>> {
    pair.check_fresh()?;
    ensure!(
        manifest.expected_revision == pair.expected_revision()
            && manifest.generation == pair.generation(),
        "migration revision or observation generation conflict"
    );
    let old_nodes = nodes(network)?;
    let current_nodes = nodes(pair.network())?;
    ensure!(
        manifest.baseline_digest == fingerprint(rows, network)
            && manifest.candidate_digest == fingerprint(pair.rows(), pair.network()),
        "migration content digest conflict"
    );
    let mut baseline = HashMap::new();
    let mut device_ids = HashSet::new();
    for row in rows {
        ensure!(
            baseline
                .insert((row.circuit_id.as_str(), row.device_id.as_str()), row)
                .is_none()
                && device_ids.insert(row.device_id.as_str()),
            "duplicate baseline row identity"
        );
    }
    let current: HashMap<_, _> = pair
        .rows()
        .iter()
        .map(|r| ((r.circuit_id.as_str(), r.device_id.as_str()), r))
        .collect();
    let mut old_rows = HashMap::new();
    let mut new_rows = HashSet::new();
    let mut old_circuits = HashMap::new();
    let mut new_circuits = HashMap::new();
    for mapping in &manifest.rows {
        let old = (&*mapping.legacy.circuit_id, &*mapping.legacy.device_id);
        let new = (&*mapping.current.circuit_id, &*mapping.current.device_id);
        ensure!(
            ![old.0, old.1, new.0, new.1]
                .iter()
                .any(|id| id.trim().is_empty()),
            "migration row identity must not be blank"
        );
        let before = baseline.get(&old).context("mapped legacy row is missing")?;
        let after = current.get(&new).context("mapped current row is missing")?;
        ensure!(
            before.circuit_name == mapping.legacy_circuit_name
                && before.device_name == mapping.legacy_device_name,
            "mapped legacy labels changed"
        );
        ensure!(
            old_rows.insert(old, *after).is_none() && new_rows.insert(new),
            "duplicate migration row mapping"
        );
        ensure!(
            old_circuits.insert(old.0, new.0).is_none_or(|v| v == new.0)
                && new_circuits.insert(new.0, old.0).is_none_or(|v| v == old.0),
            "migration circuit mapping is not bijective"
        );
    }
    let mut protected = HashSet::new();
    for row in &manifest.protected_rows {
        let key = (row.circuit_id.as_str(), row.device_id.as_str());
        ensure!(
            baseline.contains_key(&key) && !old_rows.contains_key(&key) && protected.insert(key),
            "invalid protected row ownership"
        );
    }
    let mut node_ids = HashMap::new();
    let mut names = HashMap::new();
    let mut destinations = HashSet::new();
    for mapping in &manifest.nodes {
        ensure!(
            !mapping.legacy_id.trim().is_empty() && !mapping.current_id.trim().is_empty(),
            "migration node identity must not be blank"
        );
        let old = old_nodes
            .get(mapping.legacy_id.as_str())
            .context("mapped legacy node is missing")?;
        let new = current_nodes
            .get(mapping.current_id.as_str())
            .context("mapped current node is missing")?;
        ensure!(
            old.0 == mapping.legacy_name,
            "mapped legacy node label changed"
        );
        ensure!(
            node_ids
                .insert(
                    mapping.legacy_id.as_str(),
                    (mapping.current_id.as_str(), new.0)
                )
                .is_none()
                && destinations.insert(mapping.current_id.as_str())
                && names
                    .insert(
                        old.0,
                        (
                            mapping.legacy_id.as_str(),
                            mapping.current_id.as_str(),
                            new.0
                        )
                    )
                    .is_none(),
            "duplicate migration node mapping"
        );
    }
    let mut normalized = rows.to_vec();
    for row in &mut normalized {
        if let Some((circuit, device, circuit_name, device_name)) = old_rows
            .get(&(row.circuit_id.as_str(), row.device_id.as_str()))
            .map(|desired| {
                (
                    desired.circuit_id.clone(),
                    desired.device_id.clone(),
                    desired.circuit_name.clone(),
                    desired.device_name.clone(),
                )
            })
        {
            row.circuit_id = circuit;
            row.device_id = device;
            row.circuit_name = circuit_name;
            row.device_name = device_name;
        }
        if let Some(&(old_id, new_id, new_name)) = names.get(row.parent_node.as_str()) {
            ensure!(
                row.parent_node_id.is_empty() || row.parent_node_id == old_id,
                "legacy row parent identity conflicts with its label"
            );
            row.parent_node = new_name.into();
            row.parent_node_id = new_id.into();
        }
        if let Some(&(new_id, _)) = node_ids.get(row.anchor_node_id.as_str()) {
            row.anchor_node_id = new_id.into();
        }
    }
    let mapped_network = remap_tree(network.clone(), &node_ids)?;
    let mut result = super::parity::compare_legacy(pair, &normalized, &mapped_network)?;
    let mut ownership: Vec<_> = baseline.keys().copied().collect();
    ownership.sort_unstable();
    for key in ownership {
        if !old_rows.contains_key(&key) {
            let (circuit_id, device_id) = (key.0.to_string(), key.1.to_string());
            result.push(if protected.contains(&key) {
                ParityIssue::ProtectedRow {
                    circuit_id,
                    device_id,
                }
            } else {
                ParityIssue::UnclassifiedRow {
                    circuit_id,
                    device_id,
                }
            });
        }
    }
    pair.check_fresh()?;
    Ok(result)
}

pub(super) fn nodes(network: &Value) -> anyhow::Result<HashMap<&str, (&str, &Value)>> {
    let mut result = HashMap::new();
    let mut names = HashSet::new();
    let mut work = vec![(network, 0usize)];
    while let Some((tree, depth)) = work.pop() {
        ensure!(
            depth < 63,
            "legacy hierarchy exceeds the JSON import depth limit"
        );
        for (name, node) in tree
            .as_object()
            .context("legacy network must be an object")?
        {
            ensure!(
                node.is_object() && names.insert(name.as_str()),
                "legacy node label is ambiguous"
            );
            if let Some(id) = node
                .get("id")
                .and_then(Value::as_str)
                .filter(|id| !id.is_empty())
            {
                ensure!(
                    result.insert(id, (name.as_str(), node)).is_none(),
                    "legacy node ID is ambiguous"
                );
            }
            if let Some(children) = node.get("children")
                && children.as_object().is_none_or(|c| !c.is_empty())
            {
                work.push((children, depth + 1));
            }
        }
    }
    Ok(result)
}

#[cfg(test)]
#[path = "migration_tests.rs"]
mod tests;

pub(super) fn remap_tree(tree: Value, ids: &HashMap<&str, (&str, &str)>) -> anyhow::Result<Value> {
    let Value::Object(tree) = tree else {
        anyhow::bail!("legacy network children must be objects");
    };
    let mut mapped = serde_json::Map::new();
    for (name, mut node) in tree {
        let new_name = if let Some(&(id, label)) = node
            .get("id")
            .and_then(Value::as_str)
            .and_then(|id| ids.get(id))
        {
            node["id"] = id.into();
            label.to_owned()
        } else {
            name
        };
        if let Some(children) = node.as_object_mut().and_then(|n| n.remove("children")) {
            node["children"] = remap_tree(children, ids)?;
        }
        ensure!(
            mapped.insert(new_name, node).is_none(),
            "migration node labels collide"
        );
    }
    Ok(Value::Object(mapped))
}
