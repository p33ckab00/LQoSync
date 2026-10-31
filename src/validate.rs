//! Checks that stop the cycle before a file write.

use crate::config::NetworkMode;
use crate::model::CsvRow;
use anyhow::{Context, bail, ensure};
use serde_json::{Map, Value};
use std::collections::{HashMap, HashSet};

/// Collect `name -> id` of all nodes at all levels. Stop on a duplicate name.
fn collect_nodes(
    tree: &Map<String, Value>,
    out: &mut HashMap<String, String>,
) -> anyhow::Result<()> {
    for (name, node) in tree {
        let id = node
            .get("id")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string();
        if out.insert(name.clone(), id).is_some() {
            bail!(
                "network.json: node name \"{name}\" is used two times; LibreQoS needs unique node names"
            );
        }
        if let Some(Value::Object(kids)) = node.get("children") {
            collect_nodes(kids, out)?;
        }
    }
    Ok(())
}

fn rate(row: &CsvRow, column: &str, value: &str) -> anyhow::Result<f64> {
    value
        .trim()
        .parse::<f64>()
        .ok()
        .filter(|v| v.is_finite())
        .with_context(|| {
            format!(
                "circuit \"{}\": {column} \"{value}\" is not a number",
                row.circuit_name
            )
        })
}

/// Stop the cycle if LibreQoS would reject the files.
pub fn check(rows: &[CsvRow], network: &Value, mode: NetworkMode) -> anyhow::Result<()> {
    let mut nodes = HashMap::new();
    if let Value::Object(tree) = network {
        collect_nodes(tree, &mut nodes)?;
    }
    let mut ids = HashSet::new();
    for row in rows {
        let name = &row.circuit_name;
        ensure!(
            ids.insert(row.circuit_id.as_str()),
            "circuit \"{name}\": Circuit ID \"{}\" is used two times",
            row.circuit_id
        );
        if mode != NetworkMode::NoParent && !row.parent_node.is_empty() {
            let Some(id) = nodes.get(&row.parent_node) else {
                bail!(
                    "circuit \"{name}\": Parent Node \"{}\" is not a node in network.json",
                    row.parent_node
                );
            };
            ensure!(
                row.parent_node_id.is_empty() || &row.parent_node_id == id,
                "circuit \"{name}\": Parent Node ID \"{}\" is not the id of node \"{}\"",
                row.parent_node_id,
                row.parent_node
            );
        }
        let dmin = rate(row, "Download Min Mbps", &row.download_min)?;
        let umin = rate(row, "Upload Min Mbps", &row.upload_min)?;
        let dmax = rate(row, "Download Max Mbps", &row.download_max)?;
        let umax = rate(row, "Upload Max Mbps", &row.upload_max)?;
        ensure!(
            dmin <= dmax && umin <= umax,
            "circuit \"{name}\": a min rate is above the max rate"
        );
        ensure!(
            dmax >= 0.2 && umax >= 0.2,
            "circuit \"{name}\": a max rate is below 0.2 Mbps (LibreQoS rule)"
        );
        ensure!(
            dmin >= 0.1 && umin >= 0.1,
            "circuit \"{name}\": a min rate is below 0.1 Mbps (LibreQoS rule)"
        );
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::NetworkMode;
    use crate::model::CsvRow;
    use serde_json::json;

    fn row(name: &str, id: &str, parent: &str, pid: &str) -> CsvRow {
        CsvRow {
            circuit_id: id.into(),
            circuit_name: name.into(),
            device_id: format!("D{id}"),
            parent_node: parent.into(),
            parent_node_id: pid.into(),
            download_min: "5".into(),
            upload_min: "5".into(),
            download_max: "10".into(),
            upload_max: "10".into(),
            ..Default::default()
        }
    }
    fn net() -> serde_json::Value {
        json!({"P": {"children": {"Q": {"children": {}, "id": "lq3-q", "type": "site"}}, "id": "lq3-p", "type": "site"}})
    }
    const M: NetworkMode = NetworkMode::RootPromotion;

    #[test]
    fn valid_rows_pass() {
        check(
            &[
                row("a", "1", "P", "lq3-p"),
                row("b", "2", "Q", "lq3-q"),
                row("c", "3", "", ""),
            ],
            &net(),
            M,
        )
        .unwrap();
    }

    #[test]
    fn duplicate_circuit_id_stops() {
        let e = check(
            &[row("a", "1", "P", "lq3-p"), row("b", "1", "P", "lq3-p")],
            &net(),
            M,
        )
        .unwrap_err();
        assert!(format!("{e:#}").contains("b"));
    }

    #[test]
    fn missing_parent_stops_except_in_no_parent() {
        let e = check(&[row("a", "1", "Nope", "")], &net(), M).unwrap_err();
        assert!(format!("{e:#}").contains("Nope"));
        check(
            &[row("a", "1", "Nope", "")],
            &json!({}),
            NetworkMode::NoParent,
        )
        .unwrap();
    }

    #[test]
    fn parent_node_id_must_match_the_parent_node() {
        let e = check(&[row("a", "1", "P", "lq3-q")], &net(), M).unwrap_err();
        assert!(format!("{e:#}").contains("Parent Node ID"));
    }

    #[test]
    fn duplicate_node_name_at_any_level_stops() {
        let n = json!({"P": {"children": {"P": {"children": {}}}}});
        assert!(check(&[], &n, M).is_err());
    }

    #[test]
    fn min_above_max_or_bad_number_stops() {
        let mut r = row("a", "1", "P", "lq3-p");
        r.download_min = "20".into();
        assert!(check(&[r], &net(), M).is_err());
        let mut r = row("a", "1", "P", "lq3-p");
        r.upload_max = "fast".into();
        let e = check(&[r], &net(), M).unwrap_err();
        assert!(format!("{e:#}").contains("a"));
    }

    #[test]
    fn rates_below_libreqos_floor_stop() {
        let mut r = row("a", "1", "P", "lq3-p");
        r.download_max = "0.15".into();
        r.download_min = "0.1".into();
        assert!(check(&[r], &net(), M).is_err());
        let mut r = row("a", "1", "P", "lq3-p");
        r.upload_min = "0.05".into();
        assert!(check(&[r], &net(), M).is_err());
    }
}
