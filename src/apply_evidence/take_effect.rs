//! Per-identity check: did the bridge apply each LQoSync circuit, address and node?
//! Read-only. An unknown op, a duplicate identity or a non-object op is an error.
use crate::import_evidence::addresses;
use crate::model::CsvRow;
use anyhow::{Context, Result, bail, ensure};
use serde::Serialize;
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::net::{Ipv4Addr, Ipv6Addr};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
pub enum Kind {
    Circuit,
    Address,
    Node,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Waiting {
    pub kind: Kind,
    pub identity: String,
    pub fields: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct TakeEffect {
    pub applied: usize,
    pub waiting: Vec<Waiting>,
    pub waiting_removal: Vec<(Kind, String)>,
}

impl TakeEffect {
    pub fn took_effect(&self) -> bool {
        self.waiting.is_empty() && self.waiting_removal.is_empty()
    }
}

impl fmt::Display for TakeEffect {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{} applied, {} waiting, {} waiting for removal",
            self.applied,
            self.waiting.len(),
            self.waiting_removal.len()
        )?;
        // One limit of 20 identities in all, so the message stays short.
        let mut room = 20;
        for w in self.waiting.iter().take(room) {
            write!(f, "; {:?} {} [{}]", w.kind, w.identity, w.fields.join(","))?;
            room -= 1;
        }
        for (k, id) in self.waiting_removal.iter().take(room) {
            write!(f, "; remove {k:?} {id}")?;
        }
        Ok(())
    }
}

/// The identities that one network.json and CSV pair must produce.
pub(crate) struct Identities {
    /// circuit_id to parent_node_id
    circuits: BTreeMap<String, String>,
    /// "ip/prefix" to (circuit_id, device_id)
    addresses: BTreeMap<String, (String, String)>,
    /// node_id to parent_node_id
    nodes: BTreeMap<String, String>,
}

fn walk(
    level: &Value,
    parent: &str,
    names: &mut BTreeMap<String, String>,
    nodes: &mut BTreeMap<String, String>,
) -> Result<()> {
    let Some(map) = level.as_object() else {
        bail!("network.json level is not an object");
    };
    for (name, node) in map {
        let id = node["id"]
            .as_str()
            .context("network.json node without id")?;
        ensure!(
            names.insert(name.clone(), id.to_owned()).is_none(),
            "duplicate network.json node name {name}"
        );
        ensure!(
            nodes.insert(id.to_owned(), parent.to_owned()).is_none(),
            "duplicate network.json node id {id}"
        );
        if let Some(children) = node.get("children") {
            walk(children, id, names, nodes)?;
        }
    }
    Ok(())
}

pub(crate) fn identities(network: &str, rows: &[CsvRow]) -> Result<Identities> {
    let tree: Value = serde_json::from_str(network).context("invalid network.json")?;
    let (mut names, mut nodes) = (BTreeMap::new(), BTreeMap::new());
    walk(&tree, "root", &mut names, &mut nodes)?;
    let mut circuits = BTreeMap::new();
    let mut addrs = BTreeMap::new();
    for r in rows {
        let parent = names.get(&r.parent_node).with_context(|| {
            format!(
                "circuit {} has unknown parent {}",
                r.circuit_id, r.parent_node
            )
        })?;
        if let Some(old) = circuits.insert(r.circuit_id.clone(), parent.clone()) {
            ensure!(old == *parent, "circuit {} has two parents", r.circuit_id);
        }
        let v4 = addresses::<Ipv4Addr>(&r.ipv4, 32)?
            .into_iter()
            .map(|(a, p)| format!("{a}/{p}"));
        let v6 = addresses::<Ipv6Addr>(&r.ipv6, 128)?
            .into_iter()
            .map(|(a, p)| format!("{a}/{p}"));
        for key in v4.chain(v6) {
            let val = (r.circuit_id.clone(), r.device_id.clone());
            ensure!(
                addrs.insert(key.clone(), val).is_none(),
                "duplicate address {key}"
            );
        }
    }
    Ok(Identities {
        circuits,
        addresses: addrs,
        nodes,
    })
}

type Ops = BTreeMap<String, Value>;

/// Index the ops of one array by identity key. Checks the op tag.
fn index(world: &Value, array: &str, tag: &str, key: fn(&Value) -> Result<String>) -> Result<Ops> {
    let list = world[array]
        .as_array()
        .with_context(|| format!("{array} is not an array"))?;
    let mut out = Ops::new();
    for op in list {
        ensure!(op.is_object(), "{array} holds a non-object op");
        ensure!(op["op"] == tag, "{array} holds an op that is not {tag}");
        let k = key(op)?;
        ensure!(
            out.insert(k.clone(), op.clone()).is_none(),
            "duplicate {array} identity {k}"
        );
    }
    Ok(out)
}

fn text(op: &Value, field: &str) -> Result<String> {
    Ok(op[field]
        .as_str()
        .with_context(|| format!("op without {field}"))?
        .to_owned())
}

fn address_key(op: &Value) -> Result<String> {
    let prefix = op["prefix_len"].as_u64().context("op without prefix_len")?;
    let ip: std::net::IpAddr = text(op, "ip")?.parse().context("invalid op ip")?;
    Ok(format!("{ip}/{prefix}"))
}

/// The three op maps of one world.
fn world(v: &Value) -> Result<[Ops; 3]> {
    Ok([
        index(v, "leaf_ops", "upsert", |o| text(o, "circuit_id"))?,
        index(v, "mapping_ops", "map", address_key)?,
        index(v, "inner_ops", "upsert", |o| text(o, "node_id"))?,
    ])
}

/// Fail unless the generated transaction holds exactly the identities of the pair.
pub(crate) fn coverage(ids: &Identities, generated: &Value) -> Result<()> {
    let [leaf, map, inner] = world(generated)?;
    ensure!(
        leaf.len() == ids.circuits.len()
            && ids.circuits.iter().all(|(c, p)| leaf
                .get(c)
                .is_some_and(|o| o["parent_node_id"].as_str() == Some(p))),
        "generated circuits differ from the pair: {}",
        offenders(ids.circuits.keys(), leaf.keys())
    );
    ensure!(
        map.len() == ids.addresses.len()
            && ids.addresses.iter().all(|(a, (c, d))| map
                .get(a)
                .is_some_and(
                    |o| o["circuit_id"].as_str() == Some(c) && o["device_id"].as_str() == Some(d)
                )),
        "generated addresses differ from the pair: {}",
        offenders(ids.addresses.keys(), map.keys())
    );
    ensure!(
        inner.len() == ids.nodes.len()
            && ids.nodes.iter().all(|(n, p)| inner
                .get(n)
                .is_some_and(|o| o["parent_node_id"].as_str() == Some(p))),
        "generated nodes differ from the pair: {}",
        offenders(ids.nodes.keys(), inner.keys())
    );
    Ok(())
}

/// Name identities on one side only, or say that a field differs.
fn offenders<'a>(
    want: impl Iterator<Item = &'a String>,
    got: impl Iterator<Item = &'a String>,
) -> String {
    let want: BTreeSet<_> = want.collect();
    let got: BTreeSet<_> = got.collect();
    let odd: Vec<_> = want
        .symmetric_difference(&got)
        .take(20)
        .map(|s| s.as_str())
        .collect();
    if odd.is_empty() {
        "parent or device differs".to_owned()
    } else {
        odd.join(",")
    }
}

/// Top-level keys whose values differ, sorted.
fn differing(a: &Value, b: &Value) -> Vec<String> {
    let (a, b) = (a.as_object().unwrap(), b.as_object().unwrap());
    let keys: BTreeSet<_> = a.keys().chain(b.keys()).collect();
    keys.into_iter()
        .filter(|k| a.get(*k) != b.get(*k))
        .cloned()
        .collect()
}

/// The top-level keys of one world, as seen in production on 2026-11-20.
const WORLD_KEYS: [&str; 8] = [
    "apply_mode",
    "generation",
    "inner_ops",
    "leaf_ops",
    "lpm_delta",
    "mapping_ops",
    "schema_version",
    "steering_vlans",
];

/// Fail closed on the schema of one world. The values that depend on the generation are not compared.
fn schema(world: &Value, side: &str, generated: bool) -> Result<()> {
    let object = world
        .as_object()
        .with_context(|| format!("{side} world is not an object"))?;
    ensure!(
        object.get("schema_version").and_then(Value::as_u64) == Some(6),
        "{side} world has an unsupported schema_version"
    );
    if let Some(key) = object.keys().find(|k| {
        !WORLD_KEYS.contains(&k.as_str()) && !(generated && k.as_str() == "circuit_capacity")
    }) {
        bail!("{side} world has an unknown key: {key}");
    }
    for key in WORLD_KEYS
        .into_iter()
        .chain(generated.then_some("circuit_capacity"))
    {
        ensure!(
            object.contains_key(key),
            "{side} world lacks the key: {key}"
        );
    }
    ensure!(
        !generated || object["circuit_capacity"].as_u64().is_some_and(|c| c > 0),
        "generated circuit_capacity is not positive"
    );
    Ok(())
}

pub(crate) fn compare(generated: &Value, applied: &Value) -> Result<TakeEffect> {
    schema(generated, "generated", true)?;
    schema(applied, "applied", false)?;
    let (want, got) = (world(generated)?, world(applied)?);
    let mut out = TakeEffect {
        applied: 0,
        waiting: vec![],
        waiting_removal: vec![],
    };
    for ((kind, w), g) in [Kind::Circuit, Kind::Address, Kind::Node]
        .into_iter()
        .zip(want)
        .zip(got)
    {
        for (id, op) in &w {
            match g.get(id) {
                None => out.waiting.push(Waiting {
                    kind,
                    identity: id.clone(),
                    fields: vec!["missing".to_owned()],
                }),
                Some(other) => {
                    let fields = differing(op, other);
                    if fields.is_empty() {
                        out.applied += 1;
                    } else {
                        out.waiting.push(Waiting {
                            kind,
                            identity: id.clone(),
                            fields,
                        });
                    }
                }
            }
        }
        out.waiting_removal.extend(
            g.keys()
                .filter(|k| !w.contains_key(*k))
                .map(|k| (kind, k.clone())),
        );
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    const NETWORK: &str = r#"{"A":{"id":"n-a","downloadBandwidthMbps":100,"children":{"B":{"id":"n-b","downloadBandwidthMbps":50}}}}"#;

    fn row(circuit: &str, device: &str, parent: &str, v4: &str, v6: &str) -> CsvRow {
        CsvRow {
            circuit_id: circuit.into(),
            device_id: device.into(),
            parent_node: parent.into(),
            ipv4: v4.into(),
            ipv6: v6.into(),
            ..Default::default()
        }
    }

    fn rows() -> Vec<CsvRow> {
        vec![
            row("C1", "D1", "B", "10.0.0.1", ""),
            row("C1", "D2", "B", "", "2001:db8::/64"),
            row("C2", "D3", "A", "", ""),
        ]
    }

    fn rate(n: u64) -> Value {
        json!({"ceil_kbps": n, "commit_kbps": 1})
    }

    fn leaf(c: &str, parent: &str) -> Value {
        json!({"op":"upsert","circuit_id":c,"parent_node_id":parent,"admin_up":true,
            "download":rate(10),"upload":rate(10)})
    }

    fn generated() -> Value {
        json!({
            "leaf_ops": [leaf("C1","n-b"), leaf("C2","n-a")],
            "mapping_ops": [
                {"op":"map","circuit_id":"C1","device_id":"D1","ip":"10.0.0.1","prefix_len":32},
                {"op":"map","circuit_id":"C1","device_id":"D2","ip":"2001:db8::","prefix_len":64}],
            "inner_ops": [
                {"op":"upsert","node_id":"n-a","parent_node_id":"root","download":rate(100),"upload":rate(100)},
                {"op":"upsert","node_id":"n-b","parent_node_id":"n-a","download":rate(50),"upload":rate(50)}],
            "schema_version": 6, "generation": "g1", "apply_mode": "incremental",
            "circuit_capacity": 1000, "lpm_delta": [], "steering_vlans": [],
        })
    }

    /// The bridge side of `generated()`: the same world without `circuit_capacity`.
    fn applied() -> Value {
        let mut a = generated();
        a.as_object_mut().unwrap().remove("circuit_capacity");
        a
    }

    fn waiting(kind: Kind, id: &str, fields: &[&str]) -> Waiting {
        Waiting {
            kind,
            identity: id.into(),
            fields: fields.iter().map(|s| s.to_string()).collect(),
        }
    }

    #[test]
    fn identities_cover_circuits_addresses_and_nodes() -> Result<()> {
        coverage(&identities(NETWORK, &rows())?, &generated())
    }

    #[test]
    fn a_generated_world_of_another_pair_fails_coverage() -> Result<()> {
        let ids = identities(NETWORK, &rows())?;
        let mut g = generated();
        g["leaf_ops"]
            .as_array_mut()
            .unwrap()
            .push(leaf("C3", "n-a"));
        let err = coverage(&ids, &g).unwrap_err().to_string();
        assert!(err.contains("C3"), "{err}");
        Ok(())
    }

    #[test]
    fn equal_worlds_took_effect() -> Result<()> {
        let t = compare(&generated(), &applied())?;
        assert!(t.took_effect());
        assert_eq!(t.applied, 6);
        Ok(())
    }

    #[test]
    fn a_changed_leaf_rate_waits() -> Result<()> {
        let mut a = applied();
        a["leaf_ops"][0]["download"]["ceil_kbps"] = json!(11);
        let t = compare(&generated(), &a)?;
        assert_eq!(t.waiting, [waiting(Kind::Circuit, "C1", &["download"])]);
        assert!(!t.took_effect());
        Ok(())
    }

    #[test]
    fn a_missing_address_waits() -> Result<()> {
        let mut a = applied();
        a["mapping_ops"].as_array_mut().unwrap().remove(0);
        let t = compare(&generated(), &a)?;
        assert_eq!(
            t.waiting,
            [waiting(Kind::Address, "10.0.0.1/32", &["missing"])]
        );
        Ok(())
    }

    #[test]
    fn an_old_circuit_in_the_bridge_waits_for_removal() -> Result<()> {
        let mut a = applied();
        a["leaf_ops"]
            .as_array_mut()
            .unwrap()
            .push(leaf("C9", "n-a"));
        let t = compare(&generated(), &a)?;
        assert_eq!(t.waiting_removal, [(Kind::Circuit, "C9".to_owned())]);
        assert!(t.waiting.is_empty() && !t.took_effect());
        Ok(())
    }

    #[test]
    fn a_changed_node_rate_waits() -> Result<()> {
        let mut a = applied();
        a["inner_ops"][0]["upload"] = rate(7);
        let t = compare(&generated(), &a)?;
        assert_eq!(t.waiting, [waiting(Kind::Node, "n-a", &["upload"])]);
        Ok(())
    }

    #[test]
    fn an_unknown_bridge_field_waits() -> Result<()> {
        let mut a = applied();
        a["leaf_ops"][1]["shard"] = json!(3);
        let t = compare(&generated(), &a)?;
        assert_eq!(t.waiting, [waiting(Kind::Circuit, "C2", &["shard"])]);
        Ok(())
    }

    #[test]
    fn an_unknown_op_tag_or_duplicate_identity_is_an_error() {
        let mut a = applied();
        a["leaf_ops"][0]["op"] = json!("remove");
        assert!(compare(&generated(), &a).is_err());
        let mut d = applied();
        d["leaf_ops"]
            .as_array_mut()
            .unwrap()
            .push(leaf("C1", "n-b"));
        assert!(compare(&generated(), &d).is_err());
        let mut n = applied();
        n["leaf_ops"][0] = json!(5);
        assert!(compare(&generated(), &n).is_err());
    }

    #[test]
    fn an_unknown_top_level_key_or_a_missing_one_is_an_error() {
        let mut a = applied();
        a["extra"] = json!(1);
        assert!(
            compare(&generated(), &a)
                .unwrap_err()
                .to_string()
                .contains("extra")
        );
        let mut g = generated();
        g["extra"] = json!(1);
        assert!(compare(&g, &applied()).is_err());
        let mut m = applied();
        m.as_object_mut().unwrap().remove("lpm_delta");
        assert!(
            compare(&generated(), &m)
                .unwrap_err()
                .to_string()
                .contains("lpm_delta")
        );
        let mut c = applied();
        c["circuit_capacity"] = json!(1000);
        assert!(compare(&generated(), &c).is_err());
    }

    #[test]
    fn another_schema_version_is_an_error_on_either_side() {
        let mut a = applied();
        a["schema_version"] = json!(7);
        assert!(compare(&generated(), &a).is_err());
        let mut g = generated();
        g["schema_version"] = json!(7);
        assert!(compare(&g, &applied()).is_err());
    }

    #[test]
    fn a_zero_circuit_capacity_is_an_error_even_when_waiting() {
        let mut g = generated();
        g["circuit_capacity"] = json!(0);
        assert!(compare(&g, &applied()).is_err());
        let mut a = applied();
        a["leaf_ops"] = json!([]);
        assert!(compare(&g, &a).is_err());
    }

    #[test]
    fn the_display_lists_at_most_20_identities_in_all() {
        let t = TakeEffect {
            applied: 0,
            waiting: (0..15)
                .map(|i| waiting(Kind::Circuit, &format!("w{i}"), &["x"]))
                .collect(),
            waiting_removal: (0..15).map(|i| (Kind::Node, format!("r{i}"))).collect(),
        };
        let text = t.to_string();
        assert!(text.starts_with("0 applied, 15 waiting, 15 waiting for removal"));
        assert_eq!(text.matches("; ").count(), 20);
    }
}
