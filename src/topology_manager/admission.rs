//! Compiled-candidate preservation checks. This is not a live dataplane ACK.
use super::publication::PreparedPair;
use anyhow::{Context, ensure};
use serde_json::Value;
use std::collections::{HashMap, HashSet};

pub const MAX_PAIR_BYTES: usize = 16 * 1024 * 1024;
pub const MAX_ARTIFACT_BYTES: usize = 64 * 1024 * 1024;
pub struct AdmittedPair {
    pair: PreparedPair,
}
impl AdmittedPair {
    pub(super) fn generation(&self) -> u64 {
        self.pair.generation()
    }
    pub(super) fn expected_revision(&self) -> u64 {
        self.pair.expected_revision()
    }
    pub(super) fn check_fresh(&self) -> anyhow::Result<()> {
        self.pair.check_fresh()
    }
    pub(super) fn bind_compiler(&mut self, binding: super::compiler::CompilerBinding) {
        self.pair.compiler = Some(binding);
    }
    pub fn publish(
        self,
        store: &mut super::store::RegistryStore,
        tracker: &mut crate::write_receipt::Tracker,
        request: super::publication::PublishRequest<'_>,
    ) -> anyhow::Result<crate::write_receipt::WriteResult> {
        self.pair.publish_admitted(store, tracker, request)
    }
}

pub(super) fn check_pair_size(csv: usize, network: usize) -> anyhow::Result<()> {
    ensure!(
        csv.checked_add(network)
            .is_some_and(|total| total <= MAX_PAIR_BYTES),
        "pair exceeds compilation admission byte budget"
    );
    Ok(())
}
struct Budget(usize);
impl std::io::Write for Budget {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        if bytes.len() > self.0 {
            return Err(std::io::Error::other(
                "compilation artifacts exceed byte budget",
            ));
        }
        self.0 -= bytes.len();
        Ok(bytes.len())
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

/// Check compiler artifacts, not a compiler success flag. The runtime must supply artifacts from its isolated compiler owner.
pub fn check_compiled(
    pair: PreparedPair,
    canonical: &Value,
    shaping: &Value,
) -> anyhow::Result<AdmittedPair> {
    pair.check_fresh()?;
    check_pair_size(pair.csv().len(), pair.network_text().len())?;
    check_content(
        pair.network(),
        pair.network_text(),
        pair.rows(),
        canonical,
        shaping,
    )?;
    pair.check_fresh()?;
    Ok(AdmittedPair { pair })
}

/// Offline diagnostics only. Serialized files do not authorize fresh subscribers or publication.
pub fn check_artifacts(
    network_text: &str,
    csv: &str,
    canonical: &Value,
    shaping: &Value,
) -> anyhow::Result<()> {
    check_pair_size(csv.len(), network_text.len())?;
    let network: Value = serde_json::from_str(network_text)?;
    let rows = crate::state::read_csv(csv)?;
    crate::validate::check(&rows, &network, crate::config::NetworkMode::RootPromotion)?;
    check_content(&network, network_text, &rows, canonical, shaping)
}

fn check_content(
    network: &Value,
    network_text: &str,
    rows: &[crate::model::CsvRow],
    canonical: &Value,
    shaping: &Value,
) -> anyhow::Result<()> {
    for row in rows {
        for (down, up) in [
            (&row.download_min, &row.upload_min),
            (&row.download_max, &row.upload_max),
        ] {
            let exact = super::rates::RateBps::try_from(crate::speed::Rate {
                down: down.parse()?,
                up: up.parse()?,
            })?;
            for (text, bps) in [(down, exact.down), (up, exact.up)] {
                ensure!(
                    (text.parse::<f32>()? as f64 * 1_000_000.0).round() == bps as f64,
                    "CSV rate loses integer-bps precision in compiled import"
                );
            }
        }
    }
    let mut budget = Budget(MAX_ARTIFACT_BYTES);
    serde_json::to_writer(&mut budget, canonical)?;
    serde_json::to_writer(&mut budget, shaping)?;
    ensure!(
        canonical["schema_version"] == 1 && shaping["schema_version"] == 1,
        "unsupported compilation artifact schema"
    );
    let identity = canonical["ingress_identity"]
        .as_str()
        .context("compilation ingress identity missing")?;
    ensure!(
        identity.len() == 64
            && identity.bytes().all(|b| b.is_ascii_hexdigit())
            && shaping["ingress_identity"].as_str() == Some(identity),
        "compilation ingress identity conflict"
    );
    ensure!(
        canonical["source"] == "legacy/manual" && shaping["source"] == "legacy/manual",
        "unsupported compiled source"
    );
    ensure!(
        shaping["circuit_anchors"]["anchors"]
            .as_array()
            .is_some_and(Vec::is_empty)
            && shaping["circuit_matches"]
                .as_object()
                .is_some_and(serde_json::Map::is_empty),
        "compilation introduced unsupported circuit matching"
    );
    let expected = crate::import_evidence::ExpectedImport::new(network_text, rows)?;
    ensure!(
        expected.matches_parts(
            &canonical["compatibility_network_json"],
            &shaping["shaped_devices"]
        ),
        "compiled pair differs from requested policy or endpoints"
    );
    let mut nodes = HashMap::new();
    let mut work: Vec<_> = network
        .as_object()
        .context("candidate network missing")?
        .iter()
        .map(|(name, node)| (name.as_str(), node, None::<(&str, &str)>))
        .collect();
    let mut cursor = 0;
    while cursor < work.len() {
        let (name, node, parent) = work[cursor];
        let id = node["id"]
            .as_str()
            .context("candidate node identity missing")?;
        ensure!(
            !id.trim().is_empty() && nodes.insert(id, (name, node, parent)).is_none(),
            "candidate node identity is empty or duplicated"
        );
        work.extend(
            node["children"]
                .as_object()
                .context("candidate children missing")?
                .iter()
                .map(|(child_name, child)| (child_name.as_str(), child, Some((id, name)))),
        );
        cursor += 1;
    }
    let compiled = canonical["nodes"]
        .as_array()
        .context("compiled nodes missing")?;
    ensure!(
        compiled.len() == nodes.len(),
        "compiled node inventory differs"
    );
    let mut seen = HashSet::new();
    for node in compiled {
        let id = node["node_id"]
            .as_str()
            .context("compiled node identity missing")?;
        let &(name, wanted, parent) = nodes.get(id).context("unexpected compiled node")?;
        ensure!(seen.insert(id), "duplicate compiled node");
        ensure!(
            node["node_name"].as_str() == Some(name)
                && node["node_kind"] == wanted["type"]
                && node["current_parent_node_id"].as_str() == parent.map(|(id, _)| id)
                && node["current_parent_node_name"].as_str() == parent.map(|(_, name)| name),
            "compiled node identity or ancestry changed"
        );
        ensure!(
            node["can_move"] == false
                && node["allowed_parents"]
                    .as_array()
                    .is_some_and(Vec::is_empty)
                && node["current_attachment_id"].as_str() == parent.map(|(id, _)| id)
                && node["current_attachment_name"].as_str() == parent.map(|(_, name)| name),
            "compiler introduced unrequested movement or attachment"
        );
        ensure!(
            node["rate_input"]["source"] == "imported_network_json",
            "unsupported compiled node rate authority"
        );
        for (expected, actual) in [
            ("downloadBandwidthMbps", "intrinsic_download_mbps"),
            ("uploadBandwidthMbps", "intrinsic_upload_mbps"),
        ] {
            let wanted = wanted[expected]
                .as_f64()
                .context("candidate node rate missing")?;
            let actual = node["rate_input"][actual]
                .as_f64()
                .context("compiled node rate missing")?;
            // Use the same checked conversion as preparation; never accept an integer-Mbps clamp.
            ensure!(
                super::rates::RateBps::try_from(crate::speed::Rate {
                    down: wanted,
                    up: wanted
                })? == super::rates::RateBps::try_from(crate::speed::Rate {
                    down: actual,
                    up: actual
                })?,
                "compiled intrinsic node rate changed"
            );
        }
    }
    Ok(())
}

#[cfg(test)]
#[path = "admission_tests.rs"]
pub(crate) mod tests;
