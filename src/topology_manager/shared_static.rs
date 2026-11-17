//! Explicit static attachments and shared budgets. No rates or endpoints are changed.
use super::{ownership::ExportOwnership, rates::RateBps};
use crate::model::CsvRow;
use anyhow::{Context, ensure};
use serde_json::{Map, Value};
use std::collections::{HashMap, HashSet};

pub(super) fn merge(
    policy: Option<&ExportOwnership>,
    roots: &Map<String, Value>,
    network: &mut Value,
) -> anyhow::Result<()> {
    let aliases: HashMap<_, _> = policy
        .into_iter()
        .flat_map(|p| &p.nodes)
        .map(|n| (n.node_id.as_str(), n.id.as_str()))
        .collect();
    let attachments: HashMap<_, _> = policy
        .into_iter()
        .flat_map(|p| &p.shared_attachments)
        .map(|a| {
            (
                a.root_id.as_str(),
                aliases
                    .get(a.parent_id.as_str())
                    .copied()
                    .unwrap_or(&a.parent_id),
            )
        })
        .collect();
    let forest = network
        .as_object_mut()
        .context("managed network must be an object")?;
    let mut pending: HashMap<&str, Vec<(String, Value)>> = HashMap::new();
    for (name, node) in roots {
        let id = node["id"].as_str().context("static root requires an ID")?;
        if let Some(&parent) = attachments.get(id) {
            pending
                .entry(parent)
                .or_default()
                .push((name.clone(), node.clone()));
        } else {
            ensure!(
                !forest.contains_key(name),
                "static root label collides with managed topology"
            );
            forest.insert(name.clone(), node.clone());
        }
    }
    fn insert(
        tree: &mut Map<String, Value>,
        pending: &mut HashMap<&str, Vec<(String, Value)>>,
        depth: usize,
    ) -> anyhow::Result<()> {
        ensure!(
            depth < 63,
            "shared hierarchy exceeds the JSON import depth limit"
        );
        for node in tree.values_mut() {
            let additions = node
                .get("id")
                .and_then(Value::as_str)
                .and_then(|id| pending.remove(id));
            if let Some(additions) = additions {
                let children = node
                    .get_mut("children")
                    .and_then(Value::as_object_mut)
                    .context("static attachment parent requires managed children")?;
                for (name, child) in additions {
                    ensure!(
                        !children.contains_key(&name),
                        "static attachment label collides with a child"
                    );
                    children.insert(name, child);
                }
            }
            if let Some(children) = node.get_mut("children").and_then(Value::as_object_mut)
                && !children.is_empty()
            {
                insert(children, pending, depth + 1)?;
            }
        }
        Ok(())
    }
    if !pending.is_empty() {
        insert(forest, &mut pending, 0)?;
        ensure!(
            pending.is_empty(),
            "static attachment parent is absent from exported topology"
        );
    }
    Ok(())
}

const ZERO: RateBps = RateBps { down: 0, up: 0 };
fn add(a: RateBps, b: RateBps) -> anyhow::Result<RateBps> {
    Ok(RateBps {
        down: a
            .down
            .checked_add(b.down)
            .context("shared download commitments overflow")?,
        up: a
            .up
            .checked_add(b.up)
            .context("shared upload commitments overflow")?,
    })
}
fn fits(required: RateBps, ceiling: RateBps) -> bool {
    required.down <= ceiling.down && required.up <= ceiling.up
}
fn rates(down: f64, up: f64) -> anyhow::Result<RateBps> {
    RateBps::try_from(crate::speed::Rate { down, up })
}
fn ceiling(node: &Value) -> anyhow::Result<RateBps> {
    let result = rates(
        node["downloadBandwidthMbps"]
            .as_f64()
            .context("shared node download ceiling is missing")?,
        node["uploadBandwidthMbps"]
            .as_f64()
            .context("shared node upload ceiling is missing")?,
    )?;
    ensure!(
        result.down.is_multiple_of(1_000_000) && result.up.is_multiple_of(1_000_000),
        "shared node ceilings require positive whole Mbps"
    );
    Ok(result)
}

pub(super) fn check(
    rows: &[CsvRow],
    network: &Value,
    managed: &HashSet<String>,
) -> anyhow::Result<()> {
    let mut work: Vec<(&Value, Option<usize>)> = network
        .as_object()
        .context("shared network must be an object")?
        .values()
        .filter(|node| node["id"].as_str().is_some_and(|id| managed.contains(id)))
        .map(|node| (node, None))
        .collect();
    let mut indices = HashMap::new();
    let mut ceilings = Vec::new();
    let mut cursor = 0;
    while cursor < work.len() {
        let (node, parent) = work[cursor];
        let id = node["id"]
            .as_str()
            .filter(|id| !id.trim().is_empty())
            .context("shared node requires an explicit ID")?;
        ensure!(
            indices.insert(id, cursor).is_none(),
            "shared node ID is ambiguous"
        );
        let cap = ceiling(node).with_context(|| format!("shared node {id}"))?;
        if let Some(parent) = parent {
            ensure!(
                fits(cap, ceilings[parent]),
                "shared child {id} ceiling exceeds its parent"
            );
        }
        ceilings.push(cap);
        if let Some(children) = node.get("children") {
            work.extend(
                children
                    .as_object()
                    .context("shared node children must be an object")?
                    .values()
                    .map(|child| (child, Some(cursor))),
            );
        }
        cursor += 1;
    }
    let mut required = vec![ZERO; work.len()];
    let mut direct = vec![false; work.len()];
    let mut circuits = HashMap::new();
    for row in rows {
        let Some(&parent) = indices.get(row.parent_node_id.as_str()) else {
            continue;
        };
        let minimum = rates(row.download_min.parse()?, row.upload_min.parse()?)?;
        let maximum = rates(row.download_max.parse()?, row.upload_max.parse()?)?;
        let identity = (row.parent_node_id.as_str(), minimum, maximum);
        if let Some(previous) = circuits.insert(row.circuit_id.as_str(), identity) {
            ensure!(
                previous == identity,
                "shared circuit devices disagree on parent or rates"
            );
        } else {
            required[parent] = add(required[parent], minimum)?;
            direct[parent] = true;
        }
    }
    ensure!(
        rows.iter().all(|row| circuits
            .get(row.circuit_id.as_str())
            .is_none_or(|(parent, _, _)| *parent == row.parent_node_id)),
        "shared circuit spans different parent nodes"
    );
    for i in (0..work.len()).rev() {
        let (node, parent) = work[i];
        let id = node["id"].as_str().context("shared node ID is missing")?;
        ensure!(
            !direct[i]
                || node
                    .get("children")
                    .and_then(Value::as_object)
                    .is_none_or(Map::is_empty),
            "shared node {id} cannot mix direct circuits and child groups"
        );
        let declared = |key: &str, cap: u64| -> anyhow::Result<Option<u64>> {
            match node.get(key) {
                Some(value) => Ok(Some(
                    value
                        .as_u64()
                        .filter(|n| *n > 0)
                        .and_then(|n| n.checked_mul(1_000_000))
                        .context("invalid shared node minimum")?,
                )),
                // Reserve the full static ceiling when its source has no explicit minimum.
                None => Ok((!managed.contains(id)).then_some(cap)),
            }
        };
        let down = declared("downloadBandwidthMbpsMin", ceilings[i].down)?;
        let up = declared("uploadBandwidthMbpsMin", ceilings[i].up)?;
        ensure!(
            down.is_none_or(|n| n >= required[i].down) && up.is_none_or(|n| n >= required[i].up),
            "shared node {id} minimum is below its descendant commitments"
        );
        required[i] = RateBps {
            down: down.unwrap_or(0).max(required[i].down),
            up: up.unwrap_or(0).max(required[i].up),
        };
        ensure!(
            fits(required[i], ceilings[i]),
            "shared node {id} commitments exceed its ceiling"
        );
        if let Some(parent) = parent {
            required[parent] = add(required[parent], required[i])?;
        }
    }
    Ok(())
}
