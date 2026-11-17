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
pub fn compare_legacy(
    pair: &PreparedPair,
    rows: &[CsvRow],
    network: &Value,
) -> anyhow::Result<Vec<ParityIssue>> {
    compare_snapshots(pair.rows(), pair.network(), rows, network)
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
