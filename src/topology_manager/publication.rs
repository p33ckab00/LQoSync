//! Prepare the managed pair from fresh authorized observations.
use super::identity::AuthorizedReconciliation;
use super::model::RegistryDocument;
use super::{aggregate, resolver};
use crate::model::Columns;
use crate::model::CsvRow;
use anyhow::{Context, ensure};
use serde_json::Value;
use std::collections::{HashMap, HashSet};
use tokio::time::Instant;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NodeExportAdjustment {
    pub node_id: String,
    pub calculated_ceiling: super::rates::RateBps,
    pub exported_ceiling: super::rates::RateBps,
}
pub struct PreparedPair {
    candidate: RegistryDocument,
    generation: u64,
    expires: Instant,
    rows: Vec<CsvRow>,
    network: Value,
    csv: String,
    network_text: String,
    node_adjustments: Vec<NodeExportAdjustment>,
}
pub struct PublishRequest<'a> {
    pub directory: &'a std::path::Path,
    pub meta: crate::write_receipt::Meta,
    pub now: u64,
    pub hub: &'a crate::system_health::Hub,
}
impl PreparedPair {
    pub(super) fn publish_admitted(
        self,
        store: &mut super::store::RegistryStore,
        tracker: &mut crate::write_receipt::Tracker,
        mut request: PublishRequest<'_>,
    ) -> anyhow::Result<crate::write_receipt::WriteResult> {
        ensure!(
            Instant::now() < self.expires,
            "source evidence expired before publication"
        );
        tracker.reconcile(request.now, request.hub)?;
        ensure!(
            Instant::now() < self.expires,
            "source evidence expired during reconciliation"
        );
        let expected = self.candidate.revision;
        let committed = store.commit(expected, self.candidate)?;
        ensure!(
            Instant::now() < self.expires,
            "source evidence expired after registry commit"
        );
        request.meta.topology = Some(crate::write_receipt::TopologyContext {
            policy_revision: committed.revision,
            observation_generation: self.generation,
            registry_digest: crate::write_receipt::sha256(&serde_json::to_vec(store.document()?)?),
        });
        let outputs = vec![
            (request.directory.join("network.json"), self.network_text),
            (request.directory.join("ShapedDevices.csv"), self.csv),
        ];
        ensure!(
            Instant::now() < self.expires,
            "source evidence expired before file intent"
        );
        Ok(tracker.write(&outputs, request.meta, request.now, request.hub))
    }
    pub fn node_export_adjustments(&self) -> &[NodeExportAdjustment] {
        &self.node_adjustments
    }
    pub fn expected_revision(&self) -> u64 {
        self.candidate.revision
    }
    pub fn generation(&self) -> u64 {
        self.generation
    }
    pub(super) fn check_fresh(&self) -> anyhow::Result<()> {
        ensure!(
            Instant::now() < self.expires,
            "source evidence expired before admission"
        );
        Ok(())
    }
    pub fn rows(&self) -> &[CsvRow] {
        &self.rows
    }
    pub fn network(&self) -> &Value {
        &self.network
    }
    pub fn csv(&self) -> &str {
        &self.csv
    }
    pub fn network_text(&self) -> &str {
        &self.network_text
    }
}
pub fn prepare(input: AuthorizedReconciliation) -> anyhow::Result<PreparedPair> {
    ensure!(
        Instant::now() < input.expires,
        "source evidence expired before pair preparation"
    );
    let evaluation = aggregate::evaluate(resolver::resolve(input.reconciliation)?)?;
    ensure!(
        evaluation.conflicts.is_empty(),
        "critical aggregate conflicts block publication"
    );
    let candidate = evaluation
        .candidate
        .context("publication requires a valid candidate")?;
    let catalog = candidate
        .catalog
        .as_ref()
        .context("publication requires a catalog")?;
    let definitions: HashMap<_, _> = catalog.nodes.iter().map(|n| (n.id.as_str(), n)).collect();
    let indices: HashMap<_, _> = evaluation
        .nodes
        .iter()
        .enumerate()
        .map(|(i, n)| (n.node_id.as_str(), i))
        .collect();
    let mut labels = HashSet::new();
    let mut keys = Vec::with_capacity(evaluation.nodes.len());
    let mut children = vec![Vec::new(); evaluation.nodes.len()];
    let mut order = Vec::new();
    for (i, node) in evaluation.nodes.iter().enumerate() {
        let definition = definitions
            .get(node.node_id.as_str())
            .context("export node definition missing")?;
        let label = format!("{} [{}]", definition.name, definition.id);
        ensure!(
            labels.insert(label.clone()),
            "export node labels are ambiguous"
        );
        keys.push(label);
        if let Some(parent) = &node.parent_id {
            let &parent = indices
                .get(parent.as_str())
                .context("export ancestor is omitted")?;
            children[parent].push(i);
        } else {
            order.push((i, 1usize));
        }
    }
    let mut cursor = 0;
    while cursor < order.len() {
        let (i, depth) = order[cursor];
        ensure!(depth <= 63, "topology exceeds the JSON import depth limit");
        order.extend(children[i].iter().map(|&child| (child, depth + 1)));
        cursor += 1;
    }
    ensure!(
        order.len() == evaluation.nodes.len(),
        "export hierarchy is incomplete"
    );
    let sources: HashMap<_, _> = catalog
        .sources
        .iter()
        .map(|source| (source.id.as_str(), source))
        .collect();
    let mut node_adjustments = Vec::new();
    let mut built: Vec<Option<Value>> = vec![None; evaluation.nodes.len()];
    for &(i, _) in order.iter().rev() {
        let node = &evaluation.nodes[i];
        let exported = super::rates::RateBps {
            down: node.rates.ceiling.down / 1_000_000 * 1_000_000,
            up: node.rates.ceiling.up / 1_000_000 * 1_000_000,
        };
        exported
            .validate()
            .context("node ceiling is below the supported whole-Mbps export range")?;
        ensure!(
            node.minimum_demand.down <= exported.down && node.minimum_demand.up <= exported.up,
            "node export quantization violates subscriber minimum commitments"
        );
        let definition = definitions[node.node_id.as_str()];
        if matches!(
            definition.capacity,
            super::catalog::CapacityPolicy::Automatic { .. }
        ) && let Some(super::catalog::SourceOrigin::Subscribers { group, .. }) = definition
            .source_id
            .as_deref()
            .and_then(|id| sources.get(id))
            .map(|source| &source.origin)
            && let Some(floor) = group.floor
        {
            ensure!(
                floor.down <= exported.down && floor.up <= exported.up,
                "node export quantization violates the configured group floor"
            );
        }
        if exported != node.rates.ceiling {
            node_adjustments.push(NodeExportAdjustment {
                node_id: node.node_id.clone(),
                calculated_ceiling: node.rates.ceiling,
                exported_ceiling: exported,
            });
        }
        let rate = crate::speed::Rate::try_from(exported)?;
        let mut child_map = serde_json::Map::new();
        for &child in &children[i] {
            child_map.insert(
                keys[child].clone(),
                built[child].take().context("export child missing")?,
            );
        }
        built[i] = Some(
            serde_json::json!({ "id": node.node_id, "type": "site", "downloadBandwidthMbps": rate.down, "uploadBandwidthMbps": rate.up, "children": child_map }),
        );
    }
    let mut forest = serde_json::Map::new();
    for &(i, depth) in &order {
        if depth == 1 {
            forest.insert(
                keys[i].clone(),
                built[i].take().context("export root missing")?,
            );
        }
    }
    let network = Value::Object(forest);
    let placements: HashMap<_, _> = evaluation
        .placements
        .iter()
        .map(|p| (p.subscriber_id.as_str(), p))
        .collect();
    let mut rows = Vec::new();
    let mut device_ids = HashSet::new();
    for subscriber in &evaluation.active {
        let placement = placements
            .get(subscriber.subscriber_id.as_str())
            .context("export placement missing")?;
        let &parent = indices
            .get(placement.target_id.as_str())
            .context("export target missing")?;
        let down_min = rate_text(placement.rates.min.down)?;
        let up_min = rate_text(placement.rates.min.up)?;
        let down_max = rate_text(placement.rates.max.down)?;
        let up_max = rate_text(placement.rates.max.up)?;
        for endpoint in &subscriber.endpoints {
            let identity = serde_json::to_vec(&(
                &subscriber.subscriber_id,
                &endpoint.identity,
                &endpoint.endpoint.session_id,
            ))?;
            let device_id = format!("tm-device-{}", crate::write_receipt::sha256(&identity));
            ensure!(
                device_ids.insert(device_id.clone()),
                "duplicate current device identity"
            );
            let mut ipv4 = Vec::new();
            let mut ipv6 = Vec::new();
            for ip in &endpoint.endpoint.ips {
                if ip.is_ipv4() {
                    ipv4.push(ip.to_string());
                } else {
                    ipv6.push(ip.to_string());
                }
            }
            ipv4.sort_unstable();
            ipv6.sort_unstable();
            ensure!(
                !ipv4.is_empty() || !ipv6.is_empty(),
                "current device has no endpoint"
            );
            rows.push(CsvRow {
                circuit_id: subscriber.subscriber_id.clone(),
                circuit_name: subscriber.subscriber_id.clone(),
                device_name: device_id.clone(),
                device_id,
                parent_node: keys[parent].clone(),
                parent_node_id: placement.target_id.clone(),
                ipv4: ipv4.join(","),
                ipv6: ipv6.join(","),
                mac: endpoint.endpoint.mac.clone().unwrap_or_default(),
                download_min: down_min.clone(),
                upload_min: up_min.clone(),
                download_max: down_max.clone(),
                upload_max: up_max.clone(),
                ..Default::default()
            });
        }
    }
    rows.sort_unstable_by(|a, b| {
        (
            &a.parent_node,
            &a.circuit_name,
            &a.device_name,
            &a.ipv4,
            &a.mac,
        )
            .cmp(&(
                &b.parent_node,
                &b.circuit_name,
                &b.device_name,
                &b.ipv4,
                &b.mac,
            ))
    });
    crate::validate::check(&rows, &network, crate::config::NetworkMode::RootPromotion)?;
    let csv = crate::writer::pair::render_csv(&rows, Columns::V3);
    let network_text = crate::writer::pair::render_network_json(&network);
    super::admission::check_pair_size(csv.len(), network_text.len())?;
    crate::import_evidence::ExpectedImport::new(&network_text, &rows)?;
    ensure!(
        Instant::now() < input.expires,
        "source evidence expired during pair preparation"
    );
    node_adjustments.sort_unstable_by(|a, b| a.node_id.cmp(&b.node_id));
    Ok(PreparedPair {
        node_adjustments,
        candidate,
        generation: evaluation.generation,
        expires: input.expires,
        rows,
        network,
        csv,
        network_text,
    })
}

fn rate_text(bps: u64) -> anyhow::Result<String> {
    ensure!(
        bps > 0 && bps < (1u64 << 53),
        "export rate exceeds exact conversion bounds"
    );
    let text = if bps.is_multiple_of(1_000_000) {
        (bps / 1_000_000).to_string()
    } else {
        format!("{}.{:06}", bps / 1_000_000, bps % 1_000_000)
            .trim_end_matches('0')
            .to_owned()
    };
    let imported = text.parse::<f32>()?;
    ensure!(
        imported.is_finite() && ((imported as f64 * 1_000_000.0).round() as u64) == bps,
        "source rate loses integer-bps precision in the imported pair"
    );
    Ok(text)
}

#[cfg(test)]
#[path = "publication_tests.rs"]
pub(super) mod tests;
