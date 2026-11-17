//! Export names and explicit static ownership. This policy contains no endpoints.
use super::migration::RowIdentity;
use super::model::{IdentityRef, RegistryDocument};
use anyhow::{Context, ensure};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CircuitAlias {
    pub subscriber_id: String,
    pub id: String,
    pub name: String,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DeviceAlias {
    pub identity: IdentityRef,
    pub id: String,
    pub name: String,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NodeAlias {
    pub node_id: String,
    pub id: String,
    pub name: String,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StaticAttachment {
    pub root_id: String,
    pub parent_id: String,
}
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ExportOwnership {
    pub circuits: Vec<CircuitAlias>,
    pub devices: Vec<DeviceAlias>,
    pub nodes: Vec<NodeAlias>,
    pub protected_rows: Vec<RowIdentity>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub shared_attachments: Vec<StaticAttachment>,
    /// Root IDs supplied by the current static configuration.
    pub protected_roots: Vec<String>,
}
impl ExportOwnership {
    pub fn validate(&self, registry: &RegistryDocument) -> anyhow::Result<()> {
        let subscribers: HashSet<_> = registry.subscribers.iter().map(|s| s.id.as_str()).collect();
        let identities: HashSet<_> = registry
            .subscribers
            .iter()
            .flat_map(|s| &s.identities)
            .collect();
        let node_ids: HashSet<_> = registry
            .catalog
            .iter()
            .flat_map(|c| &c.nodes)
            .map(|n| n.id.as_str())
            .collect();
        let mut targets = HashSet::new();
        let mut circuit_ids = HashSet::new();
        for alias in &self.circuits {
            ensure!(
                subscribers.contains(alias.subscriber_id.as_str())
                    && targets.insert(alias.subscriber_id.as_str()),
                "invalid or duplicate circuit alias owner"
            );
            valid(&alias.id, &alias.name)?;
            ensure!(
                circuit_ids.insert(alias.id.as_str()),
                "duplicate circuit export ID"
            );
        }
        for subscriber in &registry.subscribers {
            if !targets.contains(subscriber.id.as_str()) {
                ensure!(
                    circuit_ids.insert(subscriber.id.as_str()),
                    "circuit alias collides with a managed ID"
                );
            }
        }
        let mut device_ids = HashSet::new();
        let mut refs = HashSet::new();
        for alias in &self.devices {
            valid(&alias.id, &alias.name)?;
            ensure!(
                !alias.id.starts_with("tm-device-")
                    && identities.contains(&alias.identity)
                    && refs.insert(&alias.identity)
                    && device_ids.insert(alias.id.as_str()),
                "invalid or conflicting device alias"
            );
        }
        let mut nodes = HashSet::new();
        let mut exported_nodes = HashSet::new();
        let mut labels = HashSet::new();
        for alias in &self.nodes {
            valid(&alias.id, &alias.name)?;
            ensure!(
                node_ids.contains(alias.node_id.as_str())
                    && nodes.insert(alias.node_id.as_str())
                    && exported_nodes.insert(alias.id.as_str())
                    && labels.insert(alias.name.as_str()),
                "invalid or conflicting node alias"
            );
        }
        for node in node_ids {
            if !nodes.contains(node) {
                ensure!(
                    exported_nodes.insert(node),
                    "node alias collides with a managed ID"
                );
            }
        }
        let mut protected_devices = HashSet::new();
        for row in &self.protected_rows {
            valid(&row.circuit_id, &row.device_id)?;
            ensure!(
                !row.device_id.starts_with("tm-device-")
                    && !circuit_ids.contains(row.circuit_id.as_str())
                    && !device_ids.contains(row.device_id.as_str())
                    && protected_devices.insert(row.device_id.as_str()),
                "conflicting protected row ownership"
            );
        }
        let mut roots = HashSet::new();
        for root in &self.protected_roots {
            ensure!(
                !root.trim().is_empty()
                    && !exported_nodes.contains(root.as_str())
                    && roots.insert(root),
                "conflicting protected root ownership"
            );
        }
        if !self.shared_attachments.is_empty() {
            let definitions: HashMap<_, _> = registry
                .catalog
                .iter()
                .flat_map(|c| &c.nodes)
                .map(|n| (n.id.as_str(), n))
                .collect();
            let mut attached = HashSet::new();
            for attachment in &self.shared_attachments {
                ensure!(
                    roots.contains(&attachment.root_id) && attached.insert(&attachment.root_id),
                    "attachment requires a unique protected static root"
                );
                let parent = definitions
                    .get(attachment.parent_id.as_str())
                    .context("static attachment parent is absent from the catalog")?;
                ensure!(
                    matches!(
                        parent.kind,
                        super::layout::NodeKind::Wan | super::layout::NodeKind::Intermediate
                    ),
                    "static attachment requires a managed WAN or intermediate parent"
                );
            }
        }
        ensure!(
            self.protected_rows.is_empty() || !roots.is_empty(),
            "protected rows require a static source root"
        );
        Ok(())
    }
    pub(super) fn canonicalize(&mut self) {
        self.circuits
            .sort_unstable_by(|a, b| a.subscriber_id.cmp(&b.subscriber_id));
        self.devices
            .sort_unstable_by(|a, b| a.identity.cmp(&b.identity));
        self.nodes
            .sort_unstable_by(|a, b| a.node_id.cmp(&b.node_id));
        self.protected_rows.sort_unstable();
        self.protected_roots.sort_unstable();
        self.shared_attachments
            .sort_unstable_by(|a, b| a.root_id.cmp(&b.root_id));
    }
    pub(super) fn preserves(&self, old: &Self) -> anyhow::Result<()> {
        // Export IDs remain reserved, including when their subscriber is offline.
        let circuits: HashSet<_> = self
            .circuits
            .iter()
            .map(|a| (&a.subscriber_id, &a.id))
            .collect();
        let devices: HashSet<_> = self.devices.iter().map(|a| (&a.identity, &a.id)).collect();
        let nodes: HashSet<_> = self.nodes.iter().map(|a| (&a.node_id, &a.id)).collect();
        let rows: HashSet<_> = self
            .protected_rows
            .iter()
            .map(|r| (&r.circuit_id, &r.device_id))
            .collect();
        let roots: HashSet<_> = self.protected_roots.iter().collect();
        ensure!(
            old.circuits
                .iter()
                .all(|a| circuits.contains(&(&a.subscriber_id, &a.id))),
            "ordinary edit cannot replace circuit ownership"
        );
        ensure!(
            old.devices
                .iter()
                .all(|a| devices.contains(&(&a.identity, &a.id))),
            "ordinary edit cannot replace device ownership"
        );
        ensure!(
            old.nodes
                .iter()
                .all(|a| nodes.contains(&(&a.node_id, &a.id))),
            "ordinary edit cannot replace node ownership"
        );
        ensure!(
            old.protected_rows
                .iter()
                .all(|r| rows.contains(&(&r.circuit_id, &r.device_id))),
            "ordinary edit cannot remove static row ownership"
        );
        ensure!(
            old.protected_roots.iter().all(|r| roots.contains(r)),
            "ordinary edit cannot remove static root ownership"
        );
        Ok(())
    }
}
fn valid(id: &str, name: &str) -> anyhow::Result<()> {
    ensure!(
        !id.trim().is_empty()
            && !name.trim().is_empty()
            && !id.chars().any(char::is_control)
            && !name.chars().any(char::is_control),
        "invalid export ID or label"
    );
    Ok(())
}

pub(super) struct AliasIndex<'a> {
    pub circuits: HashMap<&'a str, &'a CircuitAlias>,
    pub devices: HashMap<&'a IdentityRef, &'a DeviceAlias>,
    pub nodes: HashMap<&'a str, &'a NodeAlias>,
}
impl<'a> AliasIndex<'a> {
    pub fn new(registry: &'a RegistryDocument) -> anyhow::Result<Self> {
        let policy = registry.export_ownership.as_ref();
        if let Some(policy) = policy {
            policy.validate(registry)?;
        }
        Ok(Self {
            circuits: policy
                .into_iter()
                .flat_map(|p| &p.circuits)
                .map(|a| (a.subscriber_id.as_str(), a))
                .collect(),
            devices: policy
                .into_iter()
                .flat_map(|p| &p.devices)
                .map(|a| (&a.identity, a))
                .collect(),
            nodes: policy
                .into_iter()
                .flat_map(|p| &p.nodes)
                .map(|a| (a.node_id.as_str(), a))
                .collect(),
        })
    }
}

/// Current operator-owned static configuration, never a previous managed output pair.
/// Configured runtime collection binds these inputs through `static_input::StaticBinding`.
pub struct StaticInput<'a> {
    pub rows: &'a [crate::model::CsvRow],
    pub network: &'a serde_json::Value,
}

pub(super) fn merge_static(
    policy: Option<&ExportOwnership>,
    input: Option<StaticInput<'_>>,
    rows: &mut Vec<crate::model::CsvRow>,
    network: &mut serde_json::Value,
) -> anyhow::Result<()> {
    let expected_rows: HashSet<_> = policy
        .into_iter()
        .flat_map(|p| &p.protected_rows)
        .map(|r| (r.circuit_id.as_str(), r.device_id.as_str()))
        .collect();
    let expected_roots: HashSet<_> = policy
        .into_iter()
        .flat_map(|p| &p.protected_roots)
        .map(String::as_str)
        .collect();
    let Some(input) = input else {
        ensure!(
            expected_rows.is_empty() && expected_roots.is_empty(),
            "current authoritative static input is required"
        );
        return Ok(());
    };
    super::migration::nodes(input.network)?;
    super::admission::check_pair_size(
        crate::writer::pair::render_csv(input.rows, crate::model::Columns::V3).len(),
        crate::writer::pair::render_network_json(input.network).len(),
    )?;
    let mut seen = HashSet::new();
    for row in input.rows {
        ensure!(
            !row.parent_node.trim().is_empty() && !row.parent_node_id.trim().is_empty(),
            "protected static row requires its explicit parent"
        );
        ensure!(
            expected_rows.contains(&(row.circuit_id.as_str(), row.device_id.as_str()))
                && seen.insert(row.device_id.as_str()),
            "unclassified or duplicate static input row"
        );
    }
    ensure!(
        seen.len() == expected_rows.len(),
        "protected static input row is missing"
    );
    let roots = input
        .network
        .as_object()
        .context("static network must be an object")?;
    let actual: HashSet<_> = roots
        .values()
        .map(|n| {
            n.get("id")
                .and_then(serde_json::Value::as_str)
                .context("static root requires an ID")
        })
        .collect::<anyhow::Result<_>>()?;
    ensure!(
        actual == expected_roots && actual.len() == roots.len(),
        "static root ownership mismatch"
    );
    // Independent validation prevents static rows from borrowing managed parents.
    crate::validate::check(
        input.rows,
        input.network,
        crate::config::NetworkMode::RootPromotion,
    )?;
    super::migration::nodes(input.network)?;
    let managed_ids = super::migration::nodes(network)?;
    let static_ids = super::migration::nodes(input.network)?;
    ensure!(
        static_ids.keys().all(|id| !managed_ids.contains_key(id)),
        "static node ID collides with managed topology"
    );
    let shared = policy.is_some_and(|p| !p.shared_attachments.is_empty());
    let managed_ids: HashSet<String> = if shared {
        managed_ids.keys().map(|id| (*id).to_owned()).collect()
    } else {
        HashSet::new()
    };
    super::shared_static::merge(policy, roots, network)?;
    super::migration::nodes(network)?;
    rows.extend_from_slice(input.rows);
    if shared {
        super::shared_static::check(rows, network, &managed_ids)?;
    }
    Ok(())
}

#[cfg(test)]
#[path = "ownership_tests.rs"]
mod tests;

/// Build durable aliases only after explicit snapshot mappings preserve all managed policy.
pub fn adopt_legacy(
    pair: &super::publication::PreparedPair,
    rows: &[crate::model::CsvRow],
    network: &serde_json::Value,
    manifest: &super::migration::MigrationManifest,
    protected_roots: &[String],
) -> anyhow::Result<RegistryDocument> {
    pair.check_fresh()?;
    ensure!(
        pair.candidate.export_ownership.is_none(),
        "legacy ownership is already configured"
    );
    ensure!(
        manifest.expected_revision == pair.expected_revision()
            && manifest.generation == pair.generation()
            && manifest.baseline_digest == super::migration::fingerprint(rows, network)
            && manifest.candidate_digest
                == super::migration::fingerprint(pair.rows(), pair.network()),
        "stale adoption evidence"
    );
    super::migration::nodes(network)?;
    let protected: HashSet<_> = manifest
        .protected_rows
        .iter()
        .map(|r| (r.circuit_id.as_str(), r.device_id.as_str()))
        .collect();
    ensure!(
        protected.len() == manifest.protected_rows.len(),
        "duplicate protected row"
    );
    let (static_rows, managed_rows): (Vec<_>, Vec<_>) = rows
        .iter()
        .cloned()
        .partition(|r| protected.contains(&(r.circuit_id.as_str(), r.device_id.as_str())));
    ensure!(
        static_rows.len() == protected.len(),
        "protected legacy row is missing"
    );
    let roots: HashSet<_> = protected_roots.iter().map(String::as_str).collect();
    ensure!(
        roots.len() == protected_roots.len(),
        "duplicate protected root"
    );
    let (static_tree, managed_tree): (serde_json::Map<_, _>, serde_json::Map<_, _>) = network
        .as_object()
        .context("legacy network must be an object")?
        .iter()
        .map(|(k, v)| (k.clone(), v.clone()))
        .partition(|(_, v)| {
            v.get("id")
                .and_then(serde_json::Value::as_str)
                .is_some_and(|id| roots.contains(id))
        });
    let managed_tree = serde_json::Value::Object(managed_tree);
    let static_tree = serde_json::Value::Object(static_tree);
    let mut managed_manifest = manifest.clone();
    managed_manifest.protected_rows.clear();
    managed_manifest.baseline_digest = super::migration::fingerprint(&managed_rows, &managed_tree);
    ensure!(
        super::migration::compare_mapped(pair, &managed_rows, &managed_tree, &managed_manifest)?
            .is_empty(),
        "legacy managed policy parity conflict"
    );
    let mut candidate = pair.candidate.clone();
    let mut ownership = ExportOwnership {
        protected_rows: manifest.protected_rows.clone(),
        protected_roots: protected_roots.to_vec(),
        ..Default::default()
    };
    let mut circuits: HashMap<&str, (&str, &str)> = HashMap::new();
    for mapping in &manifest.rows {
        let value = (
            mapping.legacy.circuit_id.as_str(),
            mapping.legacy_circuit_name.as_str(),
        );
        ensure!(
            circuits
                .insert(&mapping.current.circuit_id, value)
                .is_none_or(|old| old == value),
            "legacy circuit labels disagree"
        );
        ownership.devices.push(DeviceAlias {
            identity: pair
                .device_identities
                .get(&mapping.current.device_id)
                .context("legacy device lacks a current authoritative identity")?
                .clone(),
            id: mapping.legacy.device_id.clone(),
            name: mapping.legacy_device_name.clone(),
        });
    }
    ownership.circuits = circuits
        .into_iter()
        .map(|(id, (legacy, name))| CircuitAlias {
            subscriber_id: id.into(),
            id: legacy.into(),
            name: name.into(),
        })
        .collect();
    ownership.nodes = manifest
        .nodes
        .iter()
        .map(|n| NodeAlias {
            node_id: n.current_id.clone(),
            id: n.legacy_id.clone(),
            name: n.legacy_name.clone(),
        })
        .collect();
    ownership.canonicalize();
    ownership.validate(&candidate)?;
    let mut check_rows = Vec::new();
    let mut check_network = pair.network().clone();
    merge_static(
        Some(&ownership),
        Some(StaticInput {
            rows: &static_rows,
            network: &static_tree,
        }),
        &mut check_rows,
        &mut check_network,
    )?;
    candidate.export_ownership = Some(ownership);
    candidate.validate()?;
    pair.check_fresh()?;
    Ok(candidate)
}

/// Check configured legacy policies before exact snapshot ownership adoption.
pub fn adopt_configured_legacy(
    cfg: &crate::config::Config,
    pair: &super::publication::PreparedPair,
    rows: &[crate::model::CsvRow],
    network: &serde_json::Value,
    manifest: &super::migration::MigrationManifest,
    protected_roots: &[String],
) -> anyhow::Result<RegistryDocument> {
    pair.check_fresh()?;
    pair.check_source_filters(cfg)?;
    let issues = super::preflight::legacy_issues(cfg, &pair.candidate)?;
    ensure!(
        issues.is_empty(),
        "legacy configuration parity conflicts remain: {issues:?}"
    );
    adopt_legacy(pair, rows, network, manifest, protected_roots)
}

#[cfg(test)]
#[path = "shared_static_tests.rs"]
mod shared_tests;
