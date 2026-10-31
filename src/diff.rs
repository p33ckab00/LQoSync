//! Semantic diff of two `ShapedDevices.csv` + `network.json` pairs (`lqosync3 diff`).

use crate::model::CsvRow;
use crate::state::read_csv;
use crate::writer::pair::field;
use serde_json::{Map, Value};
use std::collections::BTreeMap;

fn header(csv: &str) -> Vec<String> {
    csv.lines()
        .next()
        .unwrap_or_default()
        .trim_start_matches('\u{feff}')
        .split(',')
        .map(|c| c.trim().to_string())
        .collect()
}

/// Two values are the same if both are the same number ("50.0" and "50") or the same text.
fn same(a: &str, b: &str) -> bool {
    match (a.trim().parse::<f64>(), b.trim().parse::<f64>()) {
        (Ok(x), Ok(y)) => x == y,
        _ => a == b,
    }
}

fn group(rows: Vec<CsvRow>) -> BTreeMap<String, Vec<CsvRow>> {
    let mut m: BTreeMap<String, Vec<CsvRow>> = BTreeMap::new();
    for r in rows {
        m.entry(r.circuit_name.clone()).or_default().push(r);
    }
    m
}

/// name -> (parent, type, download, upload) at all levels of a network.json tree.
fn nodes(tree: &Value) -> BTreeMap<String, [String; 4]> {
    fn walk(m: &Map<String, Value>, parent: &str, out: &mut BTreeMap<String, [String; 4]>) {
        for (name, n) in m {
            let text = |k: &str| {
                n.get(k).map_or(String::new(), |v| {
                    v.as_str().map_or(v.to_string(), String::from)
                })
            };
            out.insert(
                name.clone(),
                [
                    parent.to_string(),
                    text("type"),
                    text("downloadBandwidthMbps"),
                    text("uploadBandwidthMbps"),
                ],
            );
            if let Some(Value::Object(kids)) = n.get("children") {
                walk(kids, name, out);
            }
        }
    }
    let mut out = BTreeMap::new();
    if let Value::Object(m) = tree {
        walk(m, "", &mut out);
    }
    out
}

/// The diff text: one line for each change, then a count line.
pub fn diff_pair(
    old_csv: &str,
    new_csv: &str,
    old_net: &Value,
    new_net: &Value,
) -> anyhow::Result<String> {
    let new_head = header(new_csv);
    let cols: Vec<String> = header(old_csv)
        .into_iter()
        .filter(|c| new_head.contains(c))
        .collect();
    let (old, new) = (group(read_csv(old_csv)?), group(read_csv(new_csv)?));
    let mut lines = Vec::new();
    let (mut ca, mut cr, mut cc) = (0, 0, 0);
    for name in new.keys().filter(|n| !old.contains_key(*n)) {
        lines.push(format!("+ circuit {name}"));
        ca += 1;
    }
    for name in old.keys().filter(|n| !new.contains_key(*n)) {
        lines.push(format!("- circuit {name}"));
        cr += 1;
    }
    for (name, o) in old.iter().filter(|(n, _)| new.contains_key(*n)) {
        let n = &new[name];
        let changes: Vec<String> = if o.len() == 1 && n.len() == 1 {
            cols.iter()
                .filter(|c| !same(field(&o[0], c), field(&n[0], c)))
                .map(|c| format!("{c} {} -> {}", field(&o[0], c), field(&n[0], c)))
                .collect()
        } else {
            let key = |rows: &Vec<CsvRow>| {
                let mut v: Vec<Vec<String>> = rows
                    .iter()
                    .map(|r| cols.iter().map(|c| field(r, c).to_string()).collect())
                    .collect();
                v.sort();
                v
            };
            if key(o) == key(n) {
                vec![]
            } else {
                vec![format!("devices {} -> {}", o.len(), n.len())]
            }
        };
        if !changes.is_empty() {
            lines.push(format!("~ circuit {name}: {}", changes.join(", ")));
            cc += 1;
        }
    }
    let (on, nn) = (nodes(old_net), nodes(new_net));
    let (mut na, mut nr, mut nc) = (0, 0, 0);
    for name in nn.keys().filter(|n| !on.contains_key(*n)) {
        lines.push(format!("+ node {name}"));
        na += 1;
    }
    for name in on.keys().filter(|n| !nn.contains_key(*n)) {
        lines.push(format!("- node {name}"));
        nr += 1;
    }
    let keys = [
        "parent",
        "type",
        "downloadBandwidthMbps",
        "uploadBandwidthMbps",
    ];
    for (name, o) in on.iter().filter(|(n, _)| nn.contains_key(*n)) {
        let n = &nn[name];
        let changes: Vec<String> = (0..4)
            .filter(|&i| !same(&o[i], &n[i]))
            .map(|i| format!("{} {} -> {}", keys[i], o[i], n[i]))
            .collect();
        if !changes.is_empty() {
            lines.push(format!("~ node {name}: {}", changes.join(", ")));
            nc += 1;
        }
    }
    lines.push(format!(
        "circuits: +{ca} -{cr} ~{cc}; nodes: +{na} -{nr} ~{nc}"
    ));
    Ok(lines.join("\n") + "\n")
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    const OLD13: &str = "Circuit ID,Circuit Name,Device ID,Device Name,Parent Node,MAC,IPv4,IPv6,Download Min Mbps,Upload Min Mbps,Download Max Mbps,Upload Max Mbps,Comment\n\
        C1,alice,D1,alice,PPP-R1,,10.0.0.1,,5.0,5.0,50.0,50.0,PPP\n\
        C2,bob,D2,bob,PPP-R1,,10.0.0.2,,5.0,5.0,50.0,50.0,PPP\n\
        C3,gone,D3,gone,PPP-R1,,10.0.0.3,,5.0,5.0,50.0,50.0,PPP\n";
    const NEW14: &str = "Circuit ID,Circuit Name,Device ID,Device Name,Parent Node,Parent Node ID,MAC,IPv4,IPv6,Download Min Mbps,Upload Min Mbps,Download Max Mbps,Upload Max Mbps,Comment\n\
        C4,new,D4,new,PPP-R1,lq3-x,,10.0.0.4,,5,5,50,50,PPP\n\
        C2,bob,D2,bob,PPP-R1,lq3-x,,10.0.0.2,,5,5,40,50,PPP\n\
        C1,alice,D1,alice,PPP-R1,lq3-x,,10.0.0.1,,5,5,50,50,PPP\n";

    #[test]
    fn diff_reports_added_removed_changed() {
        let old = json!({"PPP-R1": {"children": {}, "downloadBandwidthMbps": 35.0, "type": "plan", "uploadBandwidthMbps": 35.0},
                         "Old": {"children": {}, "downloadBandwidthMbps": 1.0, "type": "site", "uploadBandwidthMbps": 1.0}});
        let new = json!({"PPP-R1": {"children": {}, "downloadBandwidthMbps": 52.5, "id": "lq3-x", "type": "plan", "uploadBandwidthMbps": 35.0},
                         "New": {"children": {}, "downloadBandwidthMbps": 2.0, "id": "lq3-y", "type": "site", "uploadBandwidthMbps": 2.0}});
        let out = diff_pair(OLD13, NEW14, &old, &new).unwrap();
        assert!(out.contains("+ circuit new"), "{out}");
        assert!(out.contains("- circuit gone"), "{out}");
        assert!(
            out.contains("~ circuit bob: Download Max Mbps 50.0 -> 40"),
            "{out}"
        );
        assert!(
            out.contains("+ node New") && out.contains("- node Old"),
            "{out}"
        );
        assert!(
            out.contains("~ node PPP-R1: downloadBandwidthMbps 35.0 -> 52.5"),
            "{out}"
        );
        assert!(
            out.lines().last().unwrap().contains("circuits: +1 -1 ~1"),
            "{out}"
        );
    }

    #[test]
    fn diff_ignores_column_order_row_order_and_number_text() {
        let net = json!({});
        let out = diff_pair(
            OLD13,
            OLD13.replace("50.0,50.0", "50,50").as_str(),
            &net,
            &net,
        )
        .unwrap();
        assert!(!out.contains("~ circuit"), "{out}");
        // alice exists in both files with the same values: a new Parent Node ID column is not a change.
        let out = diff_pair(OLD13, NEW14, &net, &net).unwrap();
        assert!(!out.contains("~ circuit alice"), "{out}");
    }
}
