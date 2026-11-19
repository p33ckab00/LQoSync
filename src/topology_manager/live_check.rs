//! Model of how LibreQoS turns the input pair into shaping, and a reader for its state files.
//! This module is pure. Only `read_observed` reads files.

use crate::model::CsvRow;
use anyhow::{Context, bail};
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

/// Parent id of a top-level node, as LibreQoS writes it.
const ROOT: &str = "root";

/// Rates of one direction, in kbps.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Rates {
    pub ceil_kbps: u64,
    pub commit_kbps: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExpectedCircuit {
    pub parent: String,
    pub download: Rates,
    pub upload: Rates,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExpectedNode {
    /// Display name. Observed nodes carry their id here. `compare` ignores it.
    pub name: String,
    pub parent: String,
    pub download: Rates,
    pub upload: Rates,
}

#[derive(Debug, Clone, Default)]
pub struct Expected {
    pub circuits: BTreeMap<String, ExpectedCircuit>,
    pub nodes: BTreeMap<String, ExpectedNode>,
    /// Clamps that LibreQoS applies. They are not differences.
    pub warnings: Vec<String>,
}

/// Round a Mbps value up to whole Mbps, as kbps.
fn up_kbps(mbps: f64) -> u64 {
    (mbps.max(0.0).ceil() as u64) * 1000
}

fn text_mbps(text: &str) -> anyhow::Result<f64> {
    let t = text.trim();
    if t.is_empty() {
        return Ok(0.0);
    }
    t.parse().with_context(|| format!("invalid rate {t:?}"))
}

struct RawNode {
    name: String,
    parent: String,
    ceil: [u64; 2],
    explicit: [Option<u64>; 2],
}

fn collect(
    name: &str,
    node: &Value,
    parent: &str,
    out: &mut BTreeMap<String, RawNode>,
) -> anyhow::Result<()> {
    let id = node["id"].as_str().unwrap_or(name).to_string();
    let rate = |key: &str| node[key].as_f64();
    let down = rate("downloadBandwidthMbps")
        .with_context(|| format!("node {name}: no download ceiling"))?;
    let up = rate("uploadBandwidthMbps").unwrap_or(down);
    let raw = RawNode {
        name: name.to_string(),
        parent: parent.to_string(),
        ceil: [up_kbps(down), up_kbps(up)],
        explicit: [
            rate("downloadBandwidthMbpsMin").map(up_kbps),
            rate("uploadBandwidthMbpsMin").map(up_kbps),
        ],
    };
    if out.insert(id.clone(), raw).is_some() {
        bail!("duplicate node id {id}");
    }
    if let Some(children) = node["children"].as_object() {
        for (n, c) in children {
            collect(n, c, &id, out)?;
        }
    }
    Ok(())
}

/// Commitment of a node in one direction: explicit, else the floored sum of its children.
fn commit(
    id: &str,
    dir: usize,
    nodes: &BTreeMap<String, RawNode>,
    circuits: &[(String, String, [u64; 2], [u64; 2])],
) -> u64 {
    if let Some(v) = nodes[id].explicit[dir] {
        return v;
    }
    let children = nodes.iter().filter(|(_, n)| n.parent == id);
    let sum: u64 = children
        .map(|(c, _)| commit(c, dir, nodes, circuits))
        .sum::<u64>()
        + circuits
            .iter()
            .filter(|c| c.1 == id)
            .map(|c| c.2[dir])
            .sum::<u64>();
    sum / 1000 * 1000
}

/// Build the rates that LibreQoS must plan from the CSV rows and `network.json`.
pub fn expected(rows: &[CsvRow], network: &Value) -> anyhow::Result<Expected> {
    let top = network
        .as_object()
        .context("network.json must be an object")?;
    let mut raw = BTreeMap::new();
    for (n, v) in top {
        collect(n, v, ROOT, &mut raw)?;
    }
    let by_name: BTreeMap<&str, &str> = raw
        .iter()
        .map(|(i, n)| (n.name.as_str(), i.as_str()))
        .collect();
    // (circuit id, parent id, minimum kbps, maximum kbps), one entry per circuit.
    let mut circuits: Vec<(String, String, [u64; 2], [u64; 2])> = Vec::new();
    for r in rows {
        let id = r.circuit_id.trim();
        if circuits.iter().any(|c| c.0 == id) {
            continue;
        }
        let pid = r.parent_node_id.trim();
        let parent = if raw.contains_key(pid) {
            pid.to_string()
        } else {
            by_name
                .get(r.parent_node.trim())
                .map_or(ROOT, |i| *i)
                .to_string()
        };
        let m = |s: &str| text_mbps(s).map(up_kbps);
        let min = [m(&r.download_min)?, m(&r.upload_min)?];
        let max = [m(&r.download_max)?, m(&r.upload_max)?];
        circuits.push((id.to_string(), parent, min, max));
    }
    let mut out = Expected::default();
    for (id, n) in &raw {
        let c = |d| Rates {
            ceil_kbps: n.ceil[d],
            commit_kbps: commit(id, d, &raw, &circuits),
        };
        out.nodes.insert(
            id.clone(),
            ExpectedNode {
                name: n.name.clone(),
                parent: n.parent.clone(),
                download: c(0),
                upload: c(1),
            },
        );
    }
    for (id, parent, min, max) in &circuits {
        let mut ceil = *max;
        if let Some(p) = raw.get(parent) {
            for (d, label) in ["download", "upload"].iter().enumerate() {
                if ceil[d] > p.ceil[d] {
                    out.warnings.push(format!(
                        "circuit {id}: {label} ceiling {} kbps clamped to {} kbps of {}",
                        ceil[d], p.ceil[d], p.name
                    ));
                    ceil[d] = p.ceil[d];
                }
            }
        }
        let r = |d| Rates {
            ceil_kbps: ceil[d],
            commit_kbps: min[d],
        };
        out.circuits.insert(
            id.clone(),
            ExpectedCircuit {
                parent: parent.clone(),
                download: r(0),
                upload: r(1),
            },
        );
    }
    Ok(out)
}

/// Name each node whose child commitments exceed its ceiling.
pub fn admission(expected: &Expected) -> Vec<String> {
    let mut found = Vec::new();
    for (id, node) in &expected.nodes {
        for pick in [true, false] {
            let rate = |r: &Rates| r.commit_kbps;
            let side = |d: &Rates, u: &Rates| rate(if pick { d } else { u });
            let sum: u64 = expected
                .nodes
                .values()
                .filter(|n| n.parent == *id)
                .map(|n| side(&n.download, &n.upload))
                .chain(
                    expected
                        .circuits
                        .values()
                        .filter(|c| c.parent == *id)
                        .map(|c| side(&c.download, &c.upload)),
                )
                .sum();
            let ceil = if pick {
                node.download.ceil_kbps
            } else {
                node.upload.ceil_kbps
            };
            if sum > ceil {
                found.push(format!(
                    "{}: commitments {sum} kbps exceed ceiling {ceil} kbps",
                    node.name
                ));
            }
        }
    }
    found.sort();
    found.dedup();
    found
}

/// What LibreQoS wrote after it planned the pair.
#[derive(Debug, Clone, Default)]
pub struct Observed {
    pub ready: bool,
    pub generation: String,
    pub circuits: BTreeMap<String, ExpectedCircuit>,
    pub nodes: BTreeMap<String, ExpectedNode>,
    imported_nodes: BTreeSet<String>,
    compiled_circuits: BTreeSet<String>,
    input_circuits: BTreeSet<String>,
    input_generation: String,
}

fn read_json(state: &Path, rel: &str) -> anyhow::Result<Value> {
    let path = state.join(rel);
    let text = std::fs::read_to_string(&path)
        .with_context(|| format!("cannot read {}", path.display()))?;
    serde_json::from_str(&text).with_context(|| format!("invalid JSON in {}", path.display()))
}

fn str_of(v: &Value, key: &str) -> anyhow::Result<String> {
    v[key]
        .as_str()
        .map(str::to_string)
        .with_context(|| format!("missing field {key}"))
}

fn rates_of(v: &Value) -> anyhow::Result<Rates> {
    let n = |k: &str| v[k].as_u64().with_context(|| format!("missing field {k}"));
    Ok(Rates {
        ceil_kbps: n("ceil_kbps")?,
        commit_kbps: n("commit_kbps")?,
    })
}

fn ids(v: &Value, key: &str) -> anyhow::Result<BTreeSet<String>> {
    let list = v.as_array().context("expected a list")?;
    list.iter().map(|d| str_of(d, key)).collect()
}

/// Read the five fresh state files. Other files in the folders are stale and stay unread.
pub fn read_observed(state: &Path) -> anyhow::Result<Observed> {
    let status = read_json(state, "topology/topology_runtime_status.json")?;
    let import = read_json(state, "topology/topology_import.json")?;
    let compiled = read_json(state, "shaping/topology_compiled_shaping.json")?;
    let inputs = read_json(state, "shaping/shaping_inputs.json")?;
    let plan = read_json(state, "shaping/dataplane_transaction.json")?;
    let mut o = Observed {
        ready: status["ready"].as_bool().unwrap_or(false),
        generation: str_of(&plan, "generation")?,
        input_generation: str_of(&inputs, "shaping_generation")?,
        compiled_circuits: ids(&compiled["shaped_devices"], "circuit_id")?,
        input_circuits: ids(&inputs["circuits"], "circuit_id")?,
        ..Observed::default()
    };
    let net = import["imported"]["compatibility_network_json"]
        .as_object()
        .context("import has no network")?;
    let mut raw = BTreeMap::new();
    for (n, v) in net {
        collect(n, v, ROOT, &mut raw)?;
    }
    o.imported_nodes = raw.into_keys().collect();
    for op in plan["inner_ops"].as_array().context("no inner_ops")? {
        let id = str_of(op, "node_id")?;
        let node = ExpectedNode {
            name: id.clone(),
            parent: str_of(op, "parent_node_id")?,
            download: rates_of(&op["download"])?,
            upload: rates_of(&op["upload"])?,
        };
        o.nodes.insert(id, node);
    }
    for op in plan["leaf_ops"].as_array().context("no leaf_ops")? {
        let c = ExpectedCircuit {
            parent: str_of(op, "parent_node_id")?,
            download: rates_of(&op["download"])?,
            upload: rates_of(&op["upload"])?,
        };
        o.circuits.insert(str_of(op, "circuit_id")?, c);
    }
    Ok(o)
}

#[derive(Debug, Clone, Default, serde::Serialize)]
pub struct Diff {
    pub differences: Vec<String>,
    pub warnings: Vec<String>,
}

fn same_keys<T>(
    what: &str,
    want: &BTreeMap<String, T>,
    got: &BTreeMap<String, T>,
    changed: impl Fn(&T, &T) -> bool,
    out: &mut Vec<String>,
) {
    for (k, w) in want {
        match got.get(k) {
            None => out.push(format!("missing {what} {k}")),
            Some(g) if changed(w, g) => out.push(format!("{what} {k} changed")),
            Some(_) => {}
        }
    }
    for k in got.keys().filter(|k| !want.contains_key(*k)) {
        out.push(format!("unexpected {what} {k}"));
    }
}

/// Compare the model with the observed state. Clamps are warnings, not differences.
pub fn compare(expected: &Expected, observed: &Observed) -> Diff {
    let mut d = Diff {
        warnings: expected.warnings.clone(),
        ..Diff::default()
    };
    let out = &mut d.differences;
    if !observed.ready {
        out.push("runtime status is not ready".into());
    }
    if observed.generation != observed.input_generation {
        out.push("dataplane generation differs from shaping input generation".into());
    }
    same_keys(
        "circuit",
        &expected.circuits,
        &observed.circuits,
        |a, b| a != b,
        out,
    );
    same_keys(
        "node",
        &expected.nodes,
        &observed.nodes,
        |a, b| (&a.parent, a.download, a.upload) != (&b.parent, b.download, b.upload),
        out,
    );
    let want: BTreeSet<_> = expected.circuits.keys().cloned().collect();
    if want != observed.compiled_circuits || want != observed.input_circuits {
        out.push(
            "circuit set differs between compiled shaping, shaping inputs and the model".into(),
        );
    }
    let nodes: BTreeSet<_> = expected.nodes.keys().cloned().collect();
    if nodes != observed.imported_nodes {
        out.push("node set differs between the import and the model".into());
    }
    d
}

#[cfg(test)]
#[path = "live_check_tests.rs"]
mod tests;
