//! Diagnostic comparison only. A matching endpoint does not establish identity.
use super::publication::PreparedPair;
use crate::model::CsvRow;
use serde_json::Value;
use std::collections::{BTreeMap, HashSet};

#[derive(Debug, PartialEq, Eq)]
pub enum ParityIssue {
    DuplicateRow,
    UnclassifiedRow {
        circuit_id: String,
        device_id: String,
    },
    ProtectedRow {
        circuit_id: String,
        device_id: String,
    },
    AddedRow {
        circuit_id: String,
        device_id: String,
    },
    MissingRow {
        circuit_id: String,
        device_id: String,
    },
    ChangedRow {
        circuit_id: String,
        device_id: String,
    },
    ChangedNetwork,
}
/// A difference that an operator can review and accept with `--accept-changes`. A missing,
/// protected, unclassified, or duplicate row always refuses.
pub fn needs_review(issue: &ParityIssue) -> bool {
    matches!(
        issue,
        ParityIssue::AddedRow { .. } | ParityIssue::ChangedRow { .. } | ParityIssue::ChangedNetwork
    )
}

pub fn compare_legacy(
    pair: &PreparedPair,
    rows: &[CsvRow],
    network: &Value,
) -> anyhow::Result<Vec<ParityIssue>> {
    let network = quantized_network(network, pair.node_export_adjustments(), str::to_string)?;
    compare_snapshots(pair.rows(), pair.network(), rows, &network)
}

pub(super) fn compare_snapshots(
    desired_rows: &[CsvRow],
    desired_network: &Value,
    rows: &[CsvRow],
    network: &Value,
) -> anyhow::Result<Vec<ParityIssue>> {
    let mut issues = Vec::new();
    let desired = indexed(desired_rows, &mut issues)?;
    let baseline = indexed(rows, &mut issues)?;
    for ((circuit, device), row) in &desired {
        let identity = || (circuit.to_string(), device.to_string());
        match baseline.get(&(*circuit, *device)) {
            None => {
                let (circuit_id, device_id) = identity();
                issues.push(ParityIssue::AddedRow {
                    circuit_id,
                    device_id,
                });
            }
            Some(previous) if previous != row => {
                let (circuit_id, device_id) = identity();
                issues.push(ParityIssue::ChangedRow {
                    circuit_id,
                    device_id,
                });
            }
            _ => {}
        }
    }
    for &(circuit, device) in baseline.keys() {
        if !desired.contains_key(&(circuit, device)) {
            issues.push(ParityIssue::MissingRow {
                circuit_id: circuit.into(),
                device_id: device.into(),
            });
        }
    }
    if numeric_network(network.clone()) != numeric_network(desired_network.clone()) {
        issues.push(ParityIssue::ChangedNetwork);
    }
    Ok(issues)
}

fn indexed<'a>(
    rows: &'a [CsvRow],
    issues: &mut Vec<ParityIssue>,
) -> anyhow::Result<BTreeMap<(&'a str, &'a str), CsvRow>> {
    let mut result = BTreeMap::new();
    let mut devices = HashSet::new();
    for row in rows {
        let mut normalized = row.clone();
        // LibreQoS does not use the Comment column for shaping. Legacy and manager texts differ.
        normalized.comment.clear();
        for (down, up) in [
            (&mut normalized.download_min, &mut normalized.upload_min),
            (&mut normalized.download_max, &mut normalized.upload_max),
        ] {
            let exact = super::rates::RateBps::try_from(crate::speed::Rate {
                down: down.parse()?,
                up: up.parse()?,
            })?;
            *down = exact.down.to_string();
            *up = exact.up.to_string();
        }
        if result
            .insert(
                (row.circuit_id.as_str(), row.device_id.as_str()),
                normalized,
            )
            .is_some()
            || !devices.insert(row.device_id.as_str())
        {
            issues.push(ParityIssue::DuplicateRow);
        }
    }
    Ok(result)
}

fn numeric_network(mut value: Value) -> Value {
    match &mut value {
        Value::Number(number) => {
            if let Some(normalized) = number.as_f64().and_then(serde_json::Number::from_f64) {
                *number = normalized;
            }
        }
        Value::Object(map) => {
            for item in map.values_mut() {
                *item = numeric_network(std::mem::take(item));
            }
        }
        Value::Array(items) => {
            for item in items {
                *item = numeric_network(std::mem::take(item));
            }
        }
        _ => {}
    }
    value
}

#[cfg(test)]
#[path = "parity_tests.rs"]
mod tests;

/// Replace each legacy node capacity that equals a calculated manager ceiling with its whole-Mbps export.
/// Any other legacy value stays unchanged, so a real policy difference still fails parity.
/// `id_of` maps an adjustment node ID to the node ID used in `network`.
pub fn quantized_network(
    network: &Value,
    adjustments: &[super::publication::NodeExportAdjustment],
    id_of: impl Fn(&str) -> String,
) -> anyhow::Result<Value> {
    let by_id: std::collections::HashMap<_, _> =
        adjustments.iter().map(|a| (id_of(&a.node_id), a)).collect();
    let mut out = network.clone();
    let mut work = vec![&mut out];
    while let Some(tree) = work.pop() {
        let Some(map) = tree.as_object_mut() else {
            continue;
        };
        for node in map.values_mut() {
            if let Some(adjustment) = node
                .get("id")
                .and_then(Value::as_str)
                .and_then(|id| by_id.get(id).copied())
            {
                let legacy = super::rates::RateBps::try_from(crate::speed::Rate {
                    down: node["downloadBandwidthMbps"].as_f64().unwrap_or(f64::NAN),
                    up: node["uploadBandwidthMbps"].as_f64().unwrap_or(f64::NAN),
                });
                if legacy.is_ok_and(|l| l == adjustment.calculated_ceiling) {
                    let rate = crate::speed::Rate::try_from(adjustment.exported_ceiling)?;
                    node["downloadBandwidthMbps"] = rate.down.into();
                    node["uploadBandwidthMbps"] = rate.up.into();
                }
            }
            if let Some(children) = node.get_mut("children") {
                work.push(children);
            }
        }
    }
    Ok(out)
}
