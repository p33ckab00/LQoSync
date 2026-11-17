//! Explicit legacy static extraction. Proposals do not write policy or runtime files.
use super::{
    migration::{self, MigrationManifest},
    model::RegistryDocument,
    ownership::{self, StaticAttachment, StaticInput},
    publication::PreparedPair,
};
use crate::model::CsvRow;
use anyhow::{Context, ensure};
use serde_json::{Map, Value};
use std::collections::{HashMap, HashSet};

pub struct StaticAdoption {
    pub registry: RegistryDocument,
    rows: Vec<CsvRow>,
    network: Value,
    seal: String,
    expires: tokio::time::Instant,
}
impl StaticAdoption {
    pub(super) fn check_seal(&self) -> anyhow::Result<()> {
        ensure!(
            tokio::time::Instant::now() < self.expires,
            "shared adoption evidence expired"
        );
        ensure!(
            self.seal == crate::write_receipt::sha256(&serde_json::to_vec(&self.registry)?),
            "shared adoption registry changed after proof"
        );
        Ok(())
    }
    pub fn static_input(&self) -> StaticInput<'_> {
        StaticInput {
            rows: &self.rows,
            network: &self.network,
        }
    }
    /// Return a source draft. Installation requires the normal private-file boundary.
    pub fn source_document(&self, revision: u64) -> anyhow::Result<Vec<u8>> {
        ensure!(revision > 0, "static source revision must be positive");
        ensure!(
            self.rows.len() <= super::static_input::MAX_STATIC_ROWS,
            "static source row budget exceeded"
        );
        #[derive(serde::Serialize)]
        struct Draft<'a> {
            schema_version: u32,
            revision: u64,
            rows: &'a [CsvRow],
            network: &'a Value,
        }
        let bytes = serde_json::to_vec(&Draft {
            schema_version: 1,
            revision,
            rows: &self.rows,
            network: &self.network,
        })?;
        ensure!(
            bytes.len() <= super::admission::MAX_PAIR_BYTES,
            "static source byte budget exceeded"
        );
        Ok(bytes)
    }
}

fn extract(
    tree: &Value,
    selected: &HashSet<&str>,
    parent: Option<&Value>,
    source: &mut Map<String, Value>,
    parents: &mut HashMap<String, Option<String>>,
    depth: usize,
) -> anyhow::Result<Value> {
    ensure!(
        depth < 63 || tree.as_object().is_some_and(Map::is_empty),
        "legacy extraction exceeds the JSON import depth limit"
    );
    let mut managed = Map::new();
    for (label, node) in tree
        .as_object()
        .context("legacy forest must be an object")?
    {
        let id = node.get("id").and_then(Value::as_str);
        if let Some(id) = id.filter(|id| selected.contains(id)) {
            let parent = parent
                .map(|n| {
                    n.get("id")
                        .and_then(Value::as_str)
                        .filter(|id| !id.trim().is_empty())
                        .context("selected branch parent requires an explicit ID")
                        .map(str::to_owned)
                })
                .transpose()?;
            ensure!(
                parents.insert(id.to_owned(), parent).is_none(),
                "duplicate selected static root"
            );
            source.insert(label.clone(), node.clone());
        } else {
            let mut retained = Map::new();
            for (key, value) in node.as_object().context("legacy node must be an object")? {
                retained.insert(
                    key.clone(),
                    if key == "children" {
                        // A selected subtree is copied once; retained ancestors do not clone it again.
                        extract(value, selected, Some(node), source, parents, depth + 1)?
                    } else {
                        value.clone()
                    },
                );
            }
            managed.insert(label.clone(), Value::Object(retained));
        }
    }
    Ok(Value::Object(managed))
}

/// Prove exact snapshot preservation, then return aliases and an explicit static source draft.
pub fn adopt(
    pair: &PreparedPair,
    rows: &[CsvRow],
    network: &Value,
    manifest: &MigrationManifest,
    protected_roots: &[String],
    attachments: &[StaticAttachment],
) -> anyhow::Result<StaticAdoption> {
    pair.check_fresh()?;
    ensure!(
        manifest.expected_revision == pair.expected_revision()
            && manifest.generation == pair.generation()
            && manifest.baseline_digest == migration::fingerprint(rows, network)
            && manifest.candidate_digest == migration::fingerprint(pair.rows(), pair.network()),
        "stale shared adoption evidence"
    );
    super::admission::check_pair_size(
        crate::writer::pair::render_csv(rows, crate::model::Columns::V3).len(),
        crate::writer::pair::render_network_json(network).len(),
    )?;
    migration::nodes(network)?;
    let selected: HashSet<_> = protected_roots.iter().map(String::as_str).collect();
    ensure!(
        selected.len() == protected_roots.len(),
        "duplicate protected root selection"
    );
    let targets: HashMap<_, _> = attachments
        .iter()
        .map(|a| (a.root_id.as_str(), a.parent_id.as_str()))
        .collect();
    ensure!(
        targets.len() == attachments.len() && targets.keys().all(|id| selected.contains(id)),
        "static attachments require unique selected roots"
    );
    let mappings: HashMap<_, _> = manifest
        .nodes
        .iter()
        .map(|m| (m.legacy_id.as_str(), m.current_id.as_str()))
        .collect();
    let mut static_tree = Map::new();
    let mut parents = HashMap::new();
    let managed_tree = extract(network, &selected, None, &mut static_tree, &mut parents, 0)?;
    ensure!(
        parents.len() == selected.len(),
        "selected static roots are missing or overlap"
    );
    for (root, parent) in &parents {
        if let Some(parent) = parent {
            let current = mappings.get(parent.as_str()).copied().unwrap_or(parent);
            ensure!(
                targets.get(root.as_str()).copied() == Some(current),
                "selected static branch differs from its explicit managed parent"
            );
        } else {
            ensure!(
                !targets.contains_key(root.as_str()),
                "independent static root cannot acquire a parent during adoption"
            );
        }
    }
    let static_tree = Value::Object(static_tree);
    let static_ids = migration::nodes(&static_tree)?;
    ensure!(
        manifest
            .nodes
            .iter()
            .all(|m| !static_ids.contains_key(m.legacy_id.as_str())),
        "selected static subtree contains a mapped managed node"
    );
    let mut detached = managed_tree;
    detached
        .as_object_mut()
        .context("managed forest is missing")?
        .extend(
            static_tree
                .as_object()
                .context("static forest is missing")?
                .clone(),
        );
    let mut detached_manifest = manifest.clone();
    detached_manifest.baseline_digest = migration::fingerprint(rows, &detached);
    let mut registry =
        ownership::adopt_legacy(pair, rows, &detached, &detached_manifest, protected_roots)?;
    let policy = registry
        .export_ownership
        .as_mut()
        .context("adopted ownership is missing")?;
    policy.shared_attachments = attachments.to_vec();
    policy.canonicalize();
    registry.validate()?;
    let policy = registry
        .export_ownership
        .as_ref()
        .context("adopted ownership is missing")?;
    let node_aliases: HashMap<_, _> = policy
        .nodes
        .iter()
        .map(|a| (a.node_id.as_str(), (a.id.as_str(), a.name.as_str())))
        .collect();
    let exported: HashSet<_> = registry
        .catalog
        .iter()
        .flat_map(|c| &c.nodes)
        .map(|n| {
            node_aliases
                .get(n.id.as_str())
                .map_or(n.id.as_str(), |a| a.0)
        })
        .collect();
    ensure!(
        static_ids.keys().all(|id| !exported.contains(id)),
        "static source collides with a managed export ID"
    );
    let protected: HashSet<_> = manifest
        .protected_rows
        .iter()
        .map(|r| (r.circuit_id.as_str(), r.device_id.as_str()))
        .collect();
    let static_rows: Vec<_> = rows
        .iter()
        .filter(|r| protected.contains(&(r.circuit_id.as_str(), r.device_id.as_str())))
        .cloned()
        .collect();
    let row_mappings: HashMap<_, _> = manifest
        .rows
        .iter()
        .map(|m| {
            (
                (m.current.circuit_id.as_str(), m.current.device_id.as_str()),
                m,
            )
        })
        .collect();
    let mut aliased_rows = pair.rows().to_vec();
    for row in &mut aliased_rows {
        if let Some((circuit, device, circuit_name, device_name)) = row_mappings
            .get(&(row.circuit_id.as_str(), row.device_id.as_str()))
            .map(|m| {
                (
                    m.legacy.circuit_id.clone(),
                    m.legacy.device_id.clone(),
                    m.legacy_circuit_name.clone(),
                    m.legacy_device_name.clone(),
                )
            })
        {
            row.circuit_id = circuit;
            row.device_id = device;
            row.circuit_name = circuit_name;
            row.device_name = device_name;
        }
        if let Some(&(id, name)) = node_aliases.get(row.parent_node_id.as_str()) {
            row.parent_node_id = id.into();
            row.parent_node = name.into();
        }
        if let Some(&(id, _)) = node_aliases.get(row.anchor_node_id.as_str()) {
            row.anchor_node_id = id.into();
        }
    }
    let mut aliased_network = migration::remap_tree(pair.network().clone(), &node_aliases)?;
    ownership::merge_static(
        Some(policy),
        Some(StaticInput {
            rows: &static_rows,
            network: &static_tree,
        }),
        &mut aliased_rows,
        &mut aliased_network,
    )?;
    ensure!(
        super::parity::compare_snapshots(&aliased_rows, &aliased_network, rows, network)?
            .is_empty(),
        "shared adoption changes the complete legacy snapshot"
    );
    crate::validate::check(
        &aliased_rows,
        &aliased_network,
        crate::config::NetworkMode::RootPromotion,
    )?;
    let csv = crate::writer::pair::render_csv(&aliased_rows, crate::model::Columns::V3);
    let network_text = crate::writer::pair::render_network_json(&aliased_network);
    super::admission::check_pair_size(csv.len(), network_text.len())?;
    crate::import_evidence::ExpectedImport::new(&network_text, &aliased_rows)?;
    let proposal = StaticAdoption {
        seal: crate::write_receipt::sha256(&serde_json::to_vec(&registry)?),
        expires: pair.deadline(),
        registry,
        rows: static_rows,
        network: static_tree,
    };
    proposal.source_document(1)?;
    pair.check_fresh()?;
    Ok(proposal)
}

pub fn adopt_configured(
    cfg: &crate::config::Config,
    pair: &PreparedPair,
    rows: &[CsvRow],
    network: &Value,
    manifest: &MigrationManifest,
    protected_roots: &[String],
    attachments: &[StaticAttachment],
) -> anyhow::Result<StaticAdoption> {
    pair.check_fresh()?;
    pair.check_source_filters(cfg)?;
    let issues = super::preflight::legacy_issues(cfg, &pair.candidate)?;
    ensure!(
        issues.is_empty(),
        "legacy configuration parity conflicts remain: {issues:?}"
    );
    adopt(pair, rows, network, manifest, protected_roots, attachments)
}

#[cfg(test)]
#[path = "legacy_static_tests.rs"]
pub(crate) mod tests;
