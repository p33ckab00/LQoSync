//! Correlate current imported inputs, the producer's read-only source check, and native ACK.
use super::{Applied, read_applied, transaction};
use crate::{
    config::{Config, RuntimeMode, Target},
    import_evidence::ExpectedImport,
    write_receipt::{FileOutcome, Receipt, sha256},
};
use anyhow::{Context, ensure};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{
    io::Read,
    path::{Path, PathBuf},
    time::Duration,
};
use tokio::io::AsyncReadExt;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Acceptance {
    pub source_generation: String,
    pub shaping_generation: String,
    pub peer_pid: u32,
    pub circuit_capacity: Option<u32>,
    pub desired_circuit_capacity: u32,
}
/// Only a successful read-only correlation can construct this receipt.
#[derive(Debug)]
pub struct Verified {
    pub(super) write_id: String,
    pub(super) config_digest: String,
    pub(super) file_hashes: Vec<String>,
    acceptance: Acceptance,
}
impl Verified {
    pub fn acceptance(&self) -> &Acceptance {
        &self.acceptance
    }
    pub(crate) fn matches(&self, receipt: &Receipt) -> bool {
        self.write_id == receipt.write_id
            && self.config_digest == receipt.meta.config_digest
            && self.file_hashes
                == receipt
                    .files
                    .iter()
                    .map(|f| f.sha256.clone())
                    .collect::<Vec<_>>()
    }
}
/// One normalization per pending operation; acceptance still checks fresh input hashes.
#[derive(Clone)]
pub struct Prepared {
    write_id: String,
    file_hashes: Vec<String>,
    expected: std::sync::Arc<ExpectedImport>,
}
impl Prepared {
    pub(crate) fn import_matches(&self, path: &Path) -> anyhow::Result<bool> {
        let bytes = read_files(&[path.to_path_buf()])?;
        Ok(self.expected.matches(&serde_json::from_slice(&bytes[0])?))
    }
}
pub fn prepare(receipt: &Receipt) -> anyhow::Result<Prepared> {
    ensure!(receipt.files.len() == 2, "managed pair required");
    let paths: Vec<_> = receipt.files.iter().map(|f| f.path.clone()).collect();
    let bytes = read_files(&paths)?;
    let network = receipt
        .files
        .iter()
        .position(|f| f.path.file_name().is_some_and(|n| n == "network.json"))
        .context("network input missing")?;
    let csv = receipt
        .files
        .iter()
        .position(|f| f.path.file_name().is_some_and(|n| n == "ShapedDevices.csv"))
        .context("CSV input missing")?;
    for (i, file) in receipt.files.iter().enumerate() {
        ensure!(sha256(&bytes[i]) == file.sha256, "managed input changed");
    }
    let rows = crate::state::read_csv(std::str::from_utf8(&bytes[csv])?)?;
    Ok(Prepared {
        write_id: receipt.write_id.clone(),
        file_hashes: receipt.files.iter().map(|f| f.sha256.clone()).collect(),
        expected: std::sync::Arc::new(ExpectedImport::new(
            std::str::from_utf8(&bytes[network])?,
            &rows,
        )?),
    })
}
struct Artifacts {
    bytes: Vec<Vec<u8>>,
    imported: Value,
    runtime: Value,
    generated: Value,
    inputs: Value,
    paths: Vec<PathBuf>,
}
impl Artifacts {
    fn read(cfg: &Config, receipt: &Receipt) -> anyhow::Result<Self> {
        let base = Path::new(&cfg.libreqos.state_directory);
        let mut paths: Vec<_> = receipt.files.iter().map(|f| f.path.clone()).collect();
        paths.extend([
            base.join("topology/topology_import.json"),
            base.join("topology/topology_runtime_status.json"),
            base.join("shaping/dataplane_transaction.json"),
            base.join("shaping/shaping_inputs.json"),
            PathBuf::from(&cfg.libreqos.v3_config),
        ]);
        let bytes = read_files(&paths)?;
        Ok(Self {
            imported: serde_json::from_slice(&bytes[2])?,
            runtime: serde_json::from_slice(&bytes[3])?,
            generated: serde_json::from_slice(&bytes[4])?,
            inputs: serde_json::from_slice(&bytes[5])?,
            bytes,
            paths,
        })
    }
}
fn read_files(paths: &[PathBuf]) -> anyhow::Result<Vec<Vec<u8>>> {
    const TOTAL: u64 = 64 * 1024 * 1024;
    let mut result = Vec::new();
    let mut used = 0u64;
    for path in paths {
        let mut bytes = Vec::new();
        std::fs::File::open(path)?
            .take(TOTAL.saturating_sub(used) + 1)
            .read_to_end(&mut bytes)?;
        used = used.saturating_add(bytes.len() as u64);
        ensure!(used <= TOTAL, "apply evidence exceeds its byte budget");
        result.push(bytes);
    }
    Ok(result)
}
const READ_ONLY: &str = r#"
import json,sys
sys.path.insert(0,sys.argv[1])
import liblqos_python as m
print(json.dumps(m.validated_runtime_shaping_inputs_path()))
"#;
async fn validated_path(cfg: &Config) -> anyhow::Result<PathBuf> {
    let work = async {
        let mut child = tokio::process::Command::new("python3")
            .args(["-I", "-c", READ_ONLY, &cfg.libreqos.directory])
            .current_dir(&cfg.libreqos.directory)
            .env_clear()
            .env("PATH", "/usr/bin:/bin")
            .env("LQOS_CONFIG", &cfg.libreqos.v3_config)
            .stdin(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .stdout(std::process::Stdio::piped())
            .kill_on_drop(true)
            .spawn()
            .context("cannot start read-only producer validation")?;
        let mut bytes = Vec::new();
        child
            .stdout
            .take()
            .context("missing producer response")?
            .take(8193)
            .read_to_end(&mut bytes)
            .await?;
        ensure!(bytes.len() <= 8192, "producer response exceeds its bound");
        ensure!(
            child.wait().await?.success(),
            "read-only producer validation unavailable"
        );
        let path: Option<String> =
            serde_json::from_slice(&bytes).context("invalid producer validation response")?;
        path.map(PathBuf::from)
            .context("producer runtime is not current")
    };
    tokio::time::timeout(Duration::from_secs(3), work)
        .await
        .context("producer validation timed out")?
}
fn validate(
    receipt: &Receipt,
    prepared: &Prepared,
    artifacts: &Artifacts,
    before: &Applied,
    after: &Applied,
    first_path: &Path,
    last_path: &Path,
) -> anyhow::Result<Verified> {
    ensure!(
        matches!(
            receipt.phase,
            crate::system_health::Phase::Pending | crate::system_health::Phase::ImportConfirmed
        ) && receipt.files.iter().all(|f| matches!(
            f.outcome,
            FileOutcome::Written | FileOutcome::Unchanged | FileOutcome::Reconciled
        )),
        "write reconciliation required"
    );
    for (index, file) in receipt.files.iter().enumerate() {
        ensure!(
            sha256(&artifacts.bytes[index]) == file.sha256,
            "managed input changed"
        );
    }
    ensure!(
        prepared.write_id == receipt.write_id
            && prepared.file_hashes
                == receipt
                    .files
                    .iter()
                    .map(|f| f.sha256.clone())
                    .collect::<Vec<_>>(),
        "prepared operation mismatch"
    );
    ensure!(
        prepared.expected.matches(&artifacts.imported),
        "current inputs are not imported"
    );
    ensure!(
        before == after,
        "native acknowledgement changed during verification"
    );
    ensure!(
        artifacts.runtime["schema_version"] == 1 && artifacts.runtime["ready"] == true,
        "runtime is not ready"
    );
    let source = artifacts.runtime["source_generation"]
        .as_str()
        .filter(|s| s.len() == 64 && s.bytes().all(|b| b.is_ascii_hexdigit()))
        .context("source generation missing")?;
    let generation = artifacts.runtime["shaping_generation"]
        .as_str()
        .context("shaping generation missing")?;
    ensure!(
        generation == before.generation
            && artifacts.inputs["shaping_generation"] == generation
            && artifacts.generated["generation"] == generation,
        "runtime/native generation mismatch"
    );
    let published = artifacts.runtime["shaping_inputs_path"]
        .as_str()
        .context("published shaping input path missing")?;
    ensure!(
        first_path == last_path
            && first_path == Path::new(published)
            && first_path == artifacts.paths[5],
        "producer validation path mismatch"
    );
    ensure!(
        transaction::matches(&artifacts.generated, &before.transaction),
        "native applied world differs"
    );
    Ok(Verified {
        write_id: receipt.write_id.clone(),
        config_digest: receipt.meta.config_digest.clone(),
        file_hashes: receipt.files.iter().map(|f| f.sha256.clone()).collect(),
        acceptance: Acceptance {
            source_generation: source.into(),
            shaping_generation: generation.into(),
            peer_pid: before.peer_pid,
            circuit_capacity: before.applied_circuit_capacity,
            desired_circuit_capacity: artifacts.generated["circuit_capacity"]
                .as_u64()
                .and_then(|v| v.try_into().ok())
                .context("invalid desired capacity")?,
        },
    })
}

pub async fn verify_receipt(cfg: &Config, receipt: &Receipt) -> anyhow::Result<Verified> {
    let prepared = prepare(receipt)?;
    verify_prepared(cfg, receipt, &prepared).await
}

pub async fn verify_prepared(
    cfg: &Config,
    receipt: &Receipt,
    prepared: &Prepared,
) -> anyhow::Result<Verified> {
    ensure!(
        cfg.runtime.mode == RuntimeMode::Apply
            && crate::detect::resolve(&cfg.libreqos)?.target == Target::V3Pair,
        "unsupported apply evidence target"
    );
    let base = Path::new(&cfg.libreqos.directory);
    ensure!(
        receipt.files.len() == 2
            && receipt
                .files
                .iter()
                .all(|f| f.path == base.join("network.json")
                    || f.path == base.join("ShapedDevices.csv")),
        "managed input paths differ from active configuration"
    );
    let runtime_path =
        Path::new(&cfg.libreqos.state_directory).join("topology/topology_runtime_status.json");
    let mut runtime_bytes = Vec::new();
    std::fs::File::open(runtime_path)?
        .take(64 * 1024 + 1)
        .read_to_end(&mut runtime_bytes)?;
    ensure!(
        runtime_bytes.len() <= 64 * 1024,
        "runtime status exceeds its bound"
    );
    let runtime: Value = serde_json::from_slice(&runtime_bytes)?;
    ensure!(runtime["ready"] == true, "runtime is not ready");
    let first = read_applied(Path::new(&cfg.libreqos.v3_analytics_socket)).await?;
    ensure!(
        runtime["shaping_generation"].as_str() == Some(first.generation.as_str()),
        "native generation still pending"
    );
    let config = cfg.clone();
    let pending = receipt.clone();
    let expected = prepared.clone();
    let artifacts = tokio::task::spawn_blocking(move || -> anyhow::Result<Artifacts> {
        let artifacts = Artifacts::read(&config, &pending)?;
        ensure!(
            expected.expected.matches(&artifacts.imported),
            "current inputs are not imported"
        );
        Ok(artifacts)
    })
    .await
    .context("apply evidence reader failed")??;
    let first_path = validated_path(cfg).await?;
    let second = read_applied(Path::new(&cfg.libreqos.v3_analytics_socket)).await?;
    let last_path = validated_path(cfg).await?;
    let receipt = receipt.clone();
    let prepared = prepared.clone();
    tokio::task::spawn_blocking(move || {
        ensure!(
            read_files(&artifacts.paths)? == artifacts.bytes,
            "apply inputs changed during verification"
        );
        validate(
            &receipt,
            &prepared,
            &artifacts,
            &first,
            &second,
            &first_path,
            &last_path,
        )
    })
    .await
    .context("apply evidence comparison failed")?
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        model::{Columns, CsvRow},
        system_health::{Hub, Phase},
        write_receipt::{Meta, Tracker},
    };
    use serde_json::json;
    fn fixture() -> (
        tempfile::TempDir,
        Tracker,
        Prepared,
        Artifacts,
        Applied,
        PathBuf,
    ) {
        let dir = tempfile::tempdir().unwrap();
        let row = CsvRow {
            circuit_id: "C1".into(),
            device_id: "D1".into(),
            circuit_name: "Client".into(),
            device_name: "Device".into(),
            parent_node: "P".into(),
            ipv4: "100.64.0.1".into(),
            download_min: "1".into(),
            upload_min: "1".into(),
            download_max: "20".into(),
            upload_max: "10".into(),
            ..Default::default()
        };
        let files = vec![
            (dir.path().join("network.json"), "{}".into()),
            (
                dir.path().join("ShapedDevices.csv"),
                crate::writer::pair::render_csv(&[row], Columns::V3),
            ),
        ];
        let mut tracker = Tracker::load(dir.path()).unwrap();
        tracker.write(
            &files,
            Meta {
                cycle_seq: 1,
                config_digest: "cfg".into(),
                notice: Some(crate::notify::outbox::Frozen::new(
                    &crate::config::Telegram {
                        enabled: true,
                        chat_id: "123".into(),
                        ..Default::default()
                    },
                    &Default::default(),
                )),
            },
            10,
            &Hub::default(),
        );
        let receipt = tracker.current().unwrap();
        let prepared = prepare(receipt).unwrap();
        let path = dir.path().join("shaping/shaping_inputs.json");
        let imported = json!({"imported":{"compatibility_network_json":{},"shaped_devices":[{
            "circuit_id":"C1","device_id":"D1","circuit_name":"Client","device_name":"Device","parent_node":"P","mac":"",
            "ipv4":[["100.64.0.1",32]],"ipv6":[],"download_min_mbps":1,"upload_min_mbps":1,"download_max_mbps":20,"upload_max_mbps":10}]}});
        let generated = json!({"schema_version":6,"generation":"g1","apply_mode":"incremental","circuit_capacity":1000,
            "inner_ops":[],"leaf_ops":[],"mapping_ops":[],"lpm_delta":[],"steering_vlans":[]});
        let mut native = generated.clone();
        native.as_object_mut().unwrap().remove("circuit_capacity");
        let applied = Applied {
            peer_pid: 1,
            generation: "g1".into(),
            transaction: native,
            content_key: "key".into(),
            desired_world_content_key: "key".into(),
            applied_circuit_capacity: Some(1000),
        };
        let mut paths: Vec<_> = files.iter().map(|(p, _)| p.clone()).collect();
        paths.extend([
            dir.path().join("import"),
            dir.path().join("runtime"),
            dir.path().join("generated"),
            path.clone(),
        ]);
        let artifacts = Artifacts {
            bytes: files.iter().map(|(_, s)| s.as_bytes().to_vec()).collect(),
            imported,
            runtime: json!({"schema_version":1,"ready":true,"source_generation":"a".repeat(64),"shaping_generation":"g1","shaping_inputs_path":path}),
            generated,
            inputs: json!({"schema_version":1,"shaping_generation":"g1"}),
            paths,
        };
        (dir, tracker, prepared, artifacts, applied, path)
    }
    #[tokio::test]
    async fn only_correlated_evidence_can_accept_the_same_operation() {
        let (_dir, mut tracker, prepared, artifacts, applied, path) = fixture();
        let proof = validate(
            tracker.current().unwrap(),
            &prepared,
            &artifacts,
            &applied,
            &applied,
            &path,
            &path,
        )
        .unwrap();
        let hub = Hub::default();
        tracker.accept(&proof, 11, &hub).unwrap();
        assert_eq!(tracker.current().unwrap().phase, Phase::Applied);
        assert_eq!(
            tracker
                .current()
                .unwrap()
                .acceptance
                .as_ref()
                .unwrap()
                .peer_pid,
            1
        );
        let tg = crate::config::Telegram {
            enabled: true,
            chat_id: "123".into(),
            ..Default::default()
        };
        let mut outbox = crate::notify::outbox::Outbox::new(_dir.path(), hub).unwrap();
        let mut sent = String::new();
        for _ in 0..100 {
            if !outbox
                .step(&tg, 12, |text| {
                    sent.push_str(&text);
                    std::future::ready(Ok(crate::notify::delivery::ApiReceipt { message_id: 1 }))
                })
                .await
                .unwrap()
            {
                break;
            }
        }
        assert!(sent.contains("WRITTEN / PENDING"));
        assert!(sent.contains("<b>lqosync3 APPLIED</b>"));
    }
    #[test]
    fn policy_acceptance_keeps_allocation_mismatch_degraded() {
        let (_dir, mut tracker, prepared, artifacts, mut applied, path) = fixture();
        applied.applied_circuit_capacity = Some(64);
        let proof = validate(
            tracker.current().unwrap(),
            &prepared,
            &artifacts,
            &applied,
            &applied,
            &path,
            &path,
        )
        .unwrap();
        let hub = Hub::default();
        tracker.accept(&proof, 11, &hub).unwrap();
        assert_eq!(tracker.current().unwrap().phase, Phase::Applied);
        assert_eq!(
            hub.snapshot(11, None)
                .components
                .iter()
                .find(|c| c.component == "allocation_capacity")
                .unwrap()
                .state,
            crate::system_health::State::Degraded
        );
    }
    #[test]
    fn input_changes_restarts_and_wrong_runtime_evidence_cannot_accept() {
        let (_dir, tracker, prepared, mut artifacts, applied, path) = fixture();
        let receipt = tracker.current().unwrap();
        let mut second = Applied {
            peer_pid: 2,
            generation: applied.generation.clone(),
            transaction: applied.transaction.clone(),
            content_key: applied.content_key.clone(),
            desired_world_content_key: applied.desired_world_content_key.clone(),
            applied_circuit_capacity: Some(1000),
        };
        assert!(
            validate(
                receipt, &prepared, &artifacts, &applied, &second, &path, &path
            )
            .is_err()
        );
        second.peer_pid = 1;
        artifacts.runtime["shaping_generation"] = json!("old");
        assert!(
            validate(
                receipt, &prepared, &artifacts, &applied, &second, &path, &path
            )
            .is_err()
        );
        artifacts.runtime["shaping_generation"] = json!("g1");
        assert!(
            validate(
                receipt,
                &prepared,
                &artifacts,
                &applied,
                &second,
                Path::new("wrong"),
                &path
            )
            .is_err()
        );
        artifacts.bytes[0] = b"{\"changed\":true}".to_vec();
        assert!(
            validate(
                receipt, &prepared, &artifacts, &applied, &second, &path, &path
            )
            .is_err()
        );
    }
    #[test]
    fn superseded_operation_cannot_consume_an_older_proof() {
        let (dir, mut tracker, prepared, artifacts, applied, path) = fixture();
        let proof = validate(
            tracker.current().unwrap(),
            &prepared,
            &artifacts,
            &applied,
            &applied,
            &path,
            &path,
        )
        .unwrap();
        let outputs = vec![
            (dir.path().join("network.json"), "{\"new\":true}".into()),
            (
                dir.path().join("ShapedDevices.csv"),
                String::from_utf8(artifacts.bytes[1].clone()).unwrap(),
            ),
        ];
        tracker.write(
            &outputs,
            Meta {
                cycle_seq: 2,
                config_digest: "cfg".into(),
                notice: None,
            },
            11,
            &Hub::default(),
        );
        assert!(tracker.accept(&proof, 12, &Hub::default()).is_err());
    }
}
