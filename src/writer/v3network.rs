//! The `v3_network` writer: `libreqos_network.json` schema 2 (spec "v3_network").

use crate::model::CsvRow;
use crate::topology::node_id;
use anyhow::bail;
use serde_json::{Map, Value, json};
use std::collections::{BTreeMap, HashMap};

/// v3 accepts an ID of 1 to 64 characters `[A-Za-z0-9._-]`, with no `.` or `-` at the start or end.
fn valid_id(id: &str) -> bool {
    let edge = |c: Option<char>| c.is_some_and(|c| c != '.' && c != '-');
    (1..=64).contains(&id.len())
        && id
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '-'))
        && edge(id.chars().next())
        && edge(id.chars().last())
}

/// A match prefix: an IPv4 (or IPv6) address with an optional length. No length gives /32 (/128).
fn prefix(value: &str, v6: bool) -> Option<String> {
    let (addr, len) = match value.split_once('/') {
        Some((a, l)) => (a, Some(l.parse::<u8>().ok()?)),
        None => (value, None),
    };
    let max = if v6 {
        addr.parse::<std::net::Ipv6Addr>().ok()?;
        128
    } else {
        addr.parse::<std::net::Ipv4Addr>().ok()?;
        32
    };
    let len = len.unwrap_or(max);
    (len <= max).then(|| format!("{addr}/{len}"))
}

fn list(s: &str) -> Vec<String> {
    s.split(',')
        .map(str::trim)
        .filter(|x| !x.is_empty())
        .map(str::to_string)
        .collect()
}

/// Add the nodes of `tree` to `out`, each parent before its children.
fn walk(
    tree: &Map<String, Value>,
    parent: Option<&str>,
    out: &mut Vec<Value>,
    ids: &mut HashMap<String, String>,
) -> anyhow::Result<()> {
    for (name, n) in tree {
        let id = n
            .get("id")
            .and_then(Value::as_str)
            .map_or_else(|| node_id("named", "", name), str::to_string);
        if !valid_id(&id) {
            bail!("network node \"{name}\": id \"{id}\" is not a valid LibreQoS v3 ID");
        }
        if ids.values().any(|x| *x == id) {
            bail!(
                "network node \"{name}\": the id \"{id}\" is used two times; LibreQoS v3 rejects the file"
            );
        }
        // LibreQoS 2.x treats a node without a type as a site. A node kept from the state of a v2
        // cycle can have "plan", which v3 does not have: it becomes "site" (the default v3_type).
        let kind = match n.get("type").and_then(Value::as_str) {
            None | Some("plan") => "site",
            Some(k) => k,
        };
        if !matches!(kind, "site" | "node" | "ap") {
            bail!(
                "network node \"{name}\": type \"{kind}\" does not exist in LibreQoS v3 (site, node, ap); change v3_type in [nodes]"
            );
        }
        let mut node = json!({"id": id, "type": kind, "parent": parent, "name": name});
        let rate = |k: &str| n.get(k).and_then(Value::as_f64);
        if let (Some(d), Some(u)) = (rate("downloadBandwidthMbps"), rate("uploadBandwidthMbps")) {
            // v3 requires all four values. A pair node has one rate, so commit = ceil (as the v3 legacy import).
            node["rate"] = json!({"commit_down_mbps": d, "commit_up_mbps": u, "ceil_down_mbps": d, "ceil_up_mbps": u});
        }
        out.push(node);
        ids.insert(name.clone(), id.clone());
        if let Some(Value::Object(kids)) = n.get("children") {
            walk(kids, Some(&id), out, ids)?;
        }
    }
    Ok(())
}

/// Make `libreqos_network.json` from the cycle rows and the node tree.
/// Returns the text and one alert line for each skipped circuit.
pub fn render_libreqos_network(
    rows: &[CsvRow],
    network: &Value,
) -> anyhow::Result<(String, Vec<String>)> {
    let mut topology = Vec::new();
    let mut ids = HashMap::new();
    if let Value::Object(tree) = network {
        walk(tree, None, &mut topology, &mut ids)?;
    }
    let node_ids: std::collections::HashSet<&String> = ids.values().collect();

    // One circuit for each Circuit ID, in Circuit Name order.
    let mut sorted: Vec<&CsvRow> = rows.iter().collect();
    sorted.sort_by(|a, b| {
        (&a.circuit_name, &a.device_name, &a.ipv4).cmp(&(&b.circuit_name, &b.device_name, &b.ipv4))
    });
    let mut groups: Vec<(&str, Vec<&CsvRow>)> = Vec::new();
    let mut index: BTreeMap<&str, usize> = BTreeMap::new();
    for r in sorted {
        match index.get(r.circuit_id.as_str()) {
            Some(&i) => groups[i].1.push(r),
            None => {
                index.insert(&r.circuit_id, groups.len());
                groups.push((&r.circuit_id, vec![r]));
            }
        }
    }

    let mut alerts = Vec::new();
    let mut circuits = Vec::new();
    for (cid, devs) in groups {
        let first = devs[0];
        let name = &first.circuit_name;
        let skip = |why: String| format!("v3_network: circuit {name} is not written: {why}");
        let parent = if !first.parent_node_id.is_empty() {
            Some(first.parent_node_id.clone())
        } else {
            ids.get(&first.parent_node).cloned()
        };
        let Some(parent) = parent.filter(|p| node_ids.contains(p)) else {
            let why = if first.parent_node.is_empty() {
                "it has no parent node, and LibreQoS v3 requires one".to_string()
            } else {
                format!(
                    "its parent node \"{}\" is not in the node tree",
                    first.parent_node
                )
            };
            alerts.push(skip(why));
            continue;
        };
        if let Some(bad) = std::iter::once(cid)
            .chain(devs.iter().map(|d| d.device_id.as_str()))
            .find(|id| !valid_id(id))
        {
            alerts.push(skip(format!(
                "the ID \"{bad}\" is not a valid LibreQoS v3 ID"
            )));
            continue;
        }
        let rate = |s: &str| s.trim().parse::<f64>().ok();
        let (Some(dmin), Some(umin), Some(dmax), Some(umax)) = (
            rate(&first.download_min),
            rate(&first.upload_min),
            rate(&first.download_max),
            rate(&first.upload_max),
        ) else {
            alerts.push(skip("a rate is not a number".into()));
            continue;
        };
        let mut prefixes = Vec::new();
        let mut bad_ip = None;
        let devices: Vec<Value> = devs
            .iter()
            .map(|d| {
                let (v4, v6) = (list(&d.ipv4), list(&d.ipv6));
                for (ip, v6) in v4
                    .iter()
                    .map(|ip| (ip, false))
                    .chain(v6.iter().map(|ip| (ip, true)))
                {
                    match prefix(ip, v6) {
                        Some(p) => prefixes.push(p),
                        None => bad_ip = Some(ip.clone()),
                    }
                }
                json!({"id": d.device_id, "name": d.device_name, "mac": d.mac,
                       "ipv4": v4, "ipv6": v6, "comment": d.comment})
            })
            .collect();
        if let Some(ip) = bad_ip {
            alerts.push(skip(format!(
                "\"{ip}\" is not a valid IP address or prefix"
            )));
            continue;
        }
        if prefixes.is_empty() {
            alerts.push(skip(
                "it has no IP address, and the ip match mode requires one".into(),
            ));
            continue;
        }
        circuits.push(json!({
            "id": cid,
            "parent": parent,
            "display_name": name,
            "plan": {"commit_down_mbps": dmin, "commit_up_mbps": umin,
                     "ceil_down_mbps": dmax, "ceil_up_mbps": umax},
            "devices": devices,
            "match": {"prefixes": prefixes},
        }));
    }
    let doc = json!({
        "schema_version": 2,
        "match": {"mode": "ip"},
        "topology": topology,
        "circuits": circuits,
    });
    let mut text = serde_json::to_string_pretty(&doc).expect("a JSON value serializes");
    text.push('\n');
    Ok((text, alerts))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::CsvRow;
    use serde_json::{Value, json};

    fn row(c: &str, name: &str, d: &str, parent: &str, pid: &str, ip: &str) -> CsvRow {
        CsvRow {
            circuit_id: c.into(),
            circuit_name: name.into(),
            device_id: d.into(),
            device_name: name.into(),
            parent_node: parent.into(),
            parent_node_id: pid.into(),
            mac: "AA:BB:CC:00:00:01".into(),
            ipv4: ip.into(),
            download_min: "5".into(),
            upload_min: "2.5".into(),
            download_max: "50".into(),
            upload_max: "25".into(),
            comment: "PPP".into(),
            ..CsvRow::default()
        }
    }

    fn tree() -> Value {
        json!({
            "R1": {"id": "lq3-r1", "type": "site", "downloadBandwidthMbps": 200.0, "uploadBandwidthMbps": 100.0,
                   "children": {"PLAN-A": {"id": "lq3-a", "type": "node", "downloadBandwidthMbps": 50.0,
                                           "uploadBandwidthMbps": 25.0, "children": {}}}},
            "Manual": {"downloadBandwidthMbps": 10.0, "uploadBandwidthMbps": 10.0, "children": {}}
        })
    }

    fn render(rows: &[CsvRow], t: &Value) -> (Value, Vec<String>) {
        let (text, alerts) = render_libreqos_network(rows, t).unwrap();
        (serde_json::from_str(&text).unwrap(), alerts)
    }

    #[test]
    fn writes_schema_2_with_nodes_and_circuits() {
        let (v, alerts) = render(
            &[row("C1", "alice", "D1", "PLAN-A", "lq3-a", "100.64.0.2")],
            &tree(),
        );
        assert!(alerts.is_empty(), "{alerts:?}");
        assert_eq!(v["schema_version"], 2);
        assert_eq!(v["match"], json!({"mode": "ip"}));
        let ids: Vec<&str> = v["topology"]
            .as_array()
            .unwrap()
            .iter()
            .map(|n| n["id"].as_str().unwrap())
            .collect();
        let pos = |id: &str| ids.iter().position(|x| *x == id).unwrap();
        assert!(pos("lq3-r1") < pos("lq3-a"), "parent first: {ids:?}");
        let a = v["topology"]
            .as_array()
            .unwrap()
            .iter()
            .find(|n| n["id"] == "lq3-a")
            .unwrap();
        assert_eq!(a["parent"], "lq3-r1");
        assert_eq!(a["type"], "node");
        assert_eq!(a["name"], "PLAN-A");
        assert_eq!(
            a["rate"],
            json!({"commit_down_mbps": 50.0, "commit_up_mbps": 25.0, "ceil_down_mbps": 50.0, "ceil_up_mbps": 25.0})
        );
        let r1 = v["topology"]
            .as_array()
            .unwrap()
            .iter()
            .find(|n| n["id"] == "lq3-r1")
            .unwrap();
        assert_eq!(r1["parent"], Value::Null);
        let c = &v["circuits"][0];
        assert_eq!(c["id"], "C1");
        assert_eq!(c["parent"], "lq3-a");
        assert_eq!(c["display_name"], "alice");
        assert_eq!(
            c["plan"],
            json!({"commit_down_mbps": 5.0, "commit_up_mbps": 2.5, "ceil_down_mbps": 50.0, "ceil_up_mbps": 25.0})
        );
        assert_eq!(
            c["devices"][0],
            json!({"id": "D1", "name": "alice", "mac": "AA:BB:CC:00:00:01", "ipv4": ["100.64.0.2"], "ipv6": [], "comment": "PPP"})
        );
        assert_eq!(c["match"], json!({"prefixes": ["100.64.0.2/32"]}));
    }

    #[test]
    fn a_node_without_id_or_type_gets_a_stable_id_and_site() {
        let (v, _) = render(
            &[row("C2", "bob", "D2", "Manual", "", "100.64.0.3")],
            &tree(),
        );
        let m = v["topology"]
            .as_array()
            .unwrap()
            .iter()
            .find(|n| n["name"] == "Manual")
            .unwrap();
        let id = m["id"].as_str().unwrap();
        assert!(id.starts_with("lq3-"), "{id}");
        assert_eq!(m["type"], "site");
        assert_eq!(v["circuits"][0]["parent"], id, "parent found by name");
        let (again, _) = render(
            &[row("C2", "bob", "D2", "Manual", "", "100.64.0.3")],
            &tree(),
        );
        assert_eq!(again, v);
    }

    #[test]
    fn rows_of_one_circuit_id_are_one_circuit_with_devices() {
        let mut r2 = row(
            "C1",
            "alice",
            "D9",
            "PLAN-A",
            "lq3-a",
            "100.64.0.9, 100.64.0.10",
        );
        r2.ipv6 = "2001:db8::/64".into();
        let (v, _) = render(
            &[
                row("C1", "alice", "D1", "PLAN-A", "lq3-a", "100.64.0.2"),
                r2,
            ],
            &tree(),
        );
        assert_eq!(v["circuits"].as_array().unwrap().len(), 1);
        let c = &v["circuits"][0];
        assert_eq!(c["devices"].as_array().unwrap().len(), 2);
        assert_eq!(
            c["devices"][1]["ipv4"],
            json!(["100.64.0.9", "100.64.0.10"])
        );
        assert_eq!(
            c["match"]["prefixes"],
            json!([
                "100.64.0.2/32",
                "100.64.0.9/32",
                "100.64.0.10/32",
                "2001:db8::/64"
            ])
        );
    }

    #[test]
    fn bad_circuits_are_skipped_with_an_alert() {
        let rows = [
            row("C1", "orphan", "D1", "", "", "100.64.0.2"),
            row("C2", "noip", "D2", "PLAN-A", "lq3-a", ""),
            row("C 3", "space", "D3", "PLAN-A", "lq3-a", "100.64.0.4"),
            row("C4", "lost", "D4", "Nowhere", "", "100.64.0.5"),
            row("C5", "good", "D5", "PLAN-A", "lq3-a", "100.64.0.6"),
        ];
        let (v, alerts) = render(&rows, &tree());
        let ids: Vec<&str> = v["circuits"]
            .as_array()
            .unwrap()
            .iter()
            .map(|c| c["id"].as_str().unwrap())
            .collect();
        assert_eq!(ids, ["C5"]);
        assert_eq!(alerts.len(), 4, "{alerts:?}");
        for name in ["orphan", "noip", "space", "lost"] {
            assert!(
                alerts.iter().any(|a| a.contains(name)),
                "{name}: {alerts:?}"
            );
        }
    }

    #[test]
    fn a_node_type_that_v3_does_not_know_is_an_error() {
        let t = json!({"X": {"id": "lq3-x", "type": "client", "children": {}}});
        let e = render_libreqos_network(&[], &t).unwrap_err();
        assert!(format!("{e:#}").contains("client"), "{e:#}");
    }

    #[test]
    fn a_v2_plan_node_from_the_state_is_written_as_site() {
        let t = json!({"P": {"id": "lq3-p", "type": "plan", "children": {}}});
        let (v, _) = render(&[row("C1", "a", "D1", "P", "lq3-p", "100.64.0.2")], &t);
        assert_eq!(v["topology"][0]["type"], "site");
    }

    #[test]
    fn a_bad_ip_value_skips_the_circuit() {
        let rows = [
            row("C1", "typo", "D1", "PLAN-A", "lq3-a", "10.0.0.300"),
            row("C2", "badmask", "D2", "PLAN-A", "lq3-a", "10.0.0.0/33"),
            row("C3", "cidr", "D3", "PLAN-A", "lq3-a", "10.0.1.0/24"),
        ];
        let mut v6 = row("C4", "v6", "D4", "PLAN-A", "lq3-a", "");
        v6.ipv6 = "2001:db8::1".into();
        let (v, alerts) = render(
            &[rows[0].clone(), rows[1].clone(), rows[2].clone(), v6],
            &tree(),
        );
        let ids: Vec<&str> = v["circuits"]
            .as_array()
            .unwrap()
            .iter()
            .map(|c| c["id"].as_str().unwrap())
            .collect();
        assert_eq!(ids, ["C3", "C4"], "{alerts:?}");
        assert_eq!(
            v["circuits"][0]["match"]["prefixes"],
            json!(["10.0.1.0/24"])
        );
        assert_eq!(
            v["circuits"][1]["match"]["prefixes"],
            json!(["2001:db8::1/128"])
        );
        assert!(
            alerts
                .iter()
                .any(|a| a.contains("typo") && a.contains("10.0.0.300")),
            "{alerts:?}"
        );
        assert!(alerts.iter().any(|a| a.contains("badmask")), "{alerts:?}");
    }

    #[test]
    fn a_duplicate_node_id_is_an_error() {
        let t = json!({
            "A": {"id": "same", "children": {}},
            "B": {"id": "same", "children": {}}
        });
        let e = render_libreqos_network(&[], &t).unwrap_err();
        assert!(format!("{e:#}").contains("same"), "{e:#}");
    }
}
