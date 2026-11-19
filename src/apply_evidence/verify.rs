//! Correlate current imported inputs, the producer's read-only source check, and native ACK.
use super::{
    Applied, TakeEffect, read_applied,
    take_effect::{Identities, compare, coverage, identities},
};
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
    topology: Option<crate::write_receipt::TopologyContext>,
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
        check_compiler(receipt).is_ok()
            && self.topology == receipt.meta.topology
            && self.write_id == receipt.write_id
            && self.config_digest == receipt.meta.config_digest
            && self.file_hashes
                == receipt
                    .files
                    .iter()
                    .map(|f| f.sha256.clone())
                    .collect::<Vec<_>>()
    }
}
/// The result of one verification: the world took effect, or it still waits.
#[derive(Debug)]
// One short-lived value per check: boxing would only add noise.
#[allow(clippy::large_enum_variant)]
pub enum Proof {
    Verified(Verified),
    Waiting(TakeEffect),
}
/// One normalization per pending operation; acceptance still checks fresh input hashes.
#[derive(Clone)]
pub struct Prepared {
    topology: Option<crate::write_receipt::TopologyContext>,
    write_id: String,
    file_hashes: Vec<String>,
    expected: std::sync::Arc<ExpectedImport>,
    identities: std::sync::Arc<Identities>,
}
impl Prepared {
    pub(crate) fn import_matches(&self, path: &Path) -> anyhow::Result<bool> {
        let bytes = read_files(&[path.to_path_buf()])?;
        Ok(self.expected.matches(&serde_json::from_slice(&bytes[0])?))
    }
}
pub fn prepare(receipt: &Receipt) -> anyhow::Result<Prepared> {
    check_compiler(receipt)?;
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
        topology: receipt.meta.topology.clone(),
        write_id: receipt.write_id.clone(),
        file_hashes: receipt.files.iter().map(|f| f.sha256.clone()).collect(),
        expected: std::sync::Arc::new(ExpectedImport::new(
            std::str::from_utf8(&bytes[network])?,
            &rows,
        )?),
        identities: std::sync::Arc::new(identities(std::str::from_utf8(&bytes[network])?, &rows)?),
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
) -> anyhow::Result<Proof> {
    check_compiler(receipt)?;
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
        prepared.topology == receipt.meta.topology
            && prepared.write_id == receipt.write_id
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
        artifacts.inputs["shaping_generation"] == generation
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
    coverage(&prepared.identities, &artifacts.generated)?;
    let effect = compare(&artifacts.generated, &before.transaction)?;
    if !effect.took_effect() {
        return Ok(Proof::Waiting(effect));
    }
    Ok(Proof::Verified(Verified {
        topology: receipt.meta.topology.clone(),
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
    }))
}

pub async fn verify_receipt(cfg: &Config, receipt: &Receipt) -> anyhow::Result<Proof> {
    let prepared = prepare(receipt)?;
    verify_prepared(cfg, receipt, &prepared).await
}

/// Check the native acknowledgement of a pair that the live test just wrote.
/// The receipt is only in memory: it never reaches the disk or the write tracker.
pub async fn acknowledged(
    cfg: &Config,
    written: &[(PathBuf, Vec<u8>)],
    started: u64,
) -> anyhow::Result<Proof> {
    let receipt = Receipt {
        version: 1,
        write_id: crate::runner::random_id(24),
        phase: crate::system_health::Phase::Pending,
        started,
        updated: started,
        meta: crate::write_receipt::Meta {
            topology: None,
            cycle_seq: 0,
            config_digest: String::new(),
            notice: None,
        },
        files: written
            .iter()
            .map(|(path, bytes)| crate::write_receipt::FileReceipt {
                path: path.clone(),
                sha256: sha256(bytes),
                outcome: FileOutcome::Written,
            })
            .collect(),
        error: None,
        acceptance: None,
        superseded_by: None,
    };
    verify_receipt(cfg, &receipt).await
}

pub async fn verify_prepared(
    cfg: &Config,
    receipt: &Receipt,
    prepared: &Prepared,
) -> anyhow::Result<Proof> {
    ensure!(
        cfg.runtime.mode == RuntimeMode::Apply
            && crate::detect::resolve(&cfg.libreqos)?.target == Target::V3Pair,
        "unsupported apply evidence target"
    );
    if let Some(binding) = receipt
        .meta
        .topology
        .as_ref()
        .and_then(|c| c.compiler.as_ref())
    {
        binding.check_config(&cfg.libreqos)?;
    }
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

fn check_compiler(receipt: &Receipt) -> anyhow::Result<()> {
    if let Some(binding) = receipt
        .meta
        .topology
        .as_ref()
        .and_then(|c| c.compiler.as_ref())
    {
        binding.check_current()?;
    }
    Ok(())
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
    /// A fake lqosd that answers every state request with one canned reply.
    fn serve(path: PathBuf, reply: Value) {
        use nix::sys::socket::{ControlMessage, MsgFlags, sendmsg};
        use std::{io::IoSlice, io::Write, os::fd::AsRawFd};
        let listener = std::os::unix::net::UnixListener::bind(path).unwrap();
        std::thread::spawn(move || {
            for stream in listener.incoming() {
                let mut socket = stream.unwrap();
                let mut head = [0; 4];
                socket.read_exact(&mut head).unwrap();
                let mut request = vec![0; u32::from_be_bytes(head) as usize];
                socket.read_exact(&mut request).unwrap();
                let mut bytes = Vec::new();
                ciborium::into_writer(&reply, &mut bytes).unwrap();
                let mut file = tempfile::tempfile().unwrap();
                file.write_all(&bytes).unwrap();
                let mut head = Vec::new();
                ciborium::into_writer(
                    &json!({"kind":"state","snapshot_len":bytes.len()}),
                    &mut head,
                )
                .unwrap();
                let size = (head.len() as u32).to_be_bytes();
                let fds = [file.as_raw_fd()];
                sendmsg::<()>(
                    socket.as_raw_fd(),
                    &[IoSlice::new(&size)],
                    &[ControlMessage::ScmRights(&fds)],
                    MsgFlags::empty(),
                    None,
                )
                .unwrap();
                socket.write_all(&head).unwrap();
            }
        });
    }
    fn desired() -> Value {
        let rate = json!({"ceil_kbps":20000,"commit_kbps":1000});
        json!({"schema_version":6,"generation":"g1","apply_mode":"incremental","circuit_capacity":1000,
            "inner_ops":[{"op":"upsert","node_id":"n-p","parent_node_id":"root","download":rate,"upload":rate}],
            "leaf_ops":[{"op":"upsert","circuit_id":"C1","parent_node_id":"n-p","download":rate,"upload":rate}],
            "mapping_ops":[{"op":"map","circuit_id":"C1","device_id":"D1","ip":"100.64.0.1","prefix_len":32}],
            "lpm_delta":[],"steering_vlans":[]})
    }
    /// Unwrap a proof that must be verified.
    fn verified(proof: anyhow::Result<Proof>) -> Verified {
        match proof.unwrap() {
            Proof::Verified(v) => v,
            Proof::Waiting(w) => panic!("still waiting: {w}"),
        }
    }
    fn state(generation: &str, desired: &Value) -> Value {
        let mut native = desired.clone();
        native.as_object_mut().unwrap().remove("circuit_capacity");
        native["generation"] = json!(generation);
        let mut transaction = Vec::new();
        ciborium::into_writer(&native, &mut transaction).unwrap();
        json!({"Report":{"generation":generation,"report_schema":1,"transaction":transaction,
            "applied_circuit_capacity":1000}})
    }
    /// A complete fake LibreQoS tree. Returns the config and the written pair.
    fn live(
        reply: impl Fn(&Value) -> Value,
    ) -> (tempfile::TempDir, Config, Vec<(PathBuf, Vec<u8>)>) {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        let (lq, st) = (root.join("lq"), root.join("state"));
        for d in ["lq", "state/topology", "state/shaping"] {
            std::fs::create_dir_all(root.join(d)).unwrap();
        }
        let csv = "Circuit ID,Device ID,Circuit Name,Device Name,Parent Node,MAC,IPv4,IPv6,Download Min Mbps,Upload Min Mbps,Download Max Mbps,Upload Max Mbps\nC1,D1,Client,Device,P,,100.64.0.1,,1,1,20,10\n";
        let written = vec![
            (lq.join("network.json"), br#"{"P":{"id":"n-p"}}"#.to_vec()),
            (lq.join("ShapedDevices.csv"), csv.as_bytes().to_vec()),
        ];
        for (p, b) in &written {
            std::fs::write(p, b).unwrap();
        }
        let inputs = st.join("shaping/shaping_inputs.json");
        let write = |p: PathBuf, v: Value| std::fs::write(p, v.to_string()).unwrap();
        write(
            st.join("topology/topology_import.json"),
            json!({"imported":{"compatibility_network_json":{"P":{"id":"n-p"}},"shaped_devices":[{
            "circuit_id":"C1","device_id":"D1","circuit_name":"Client","device_name":"Device","parent_node":"P","mac":"",
            "ipv4":[["100.64.0.1",32]],"ipv6":[],"download_min_mbps":1,"upload_min_mbps":1,"download_max_mbps":20,"upload_max_mbps":10}]}}),
        );
        write(
            st.join("topology/topology_runtime_status.json"),
            json!({"schema_version":1,"ready":true,
            "source_generation":"a".repeat(64),"shaping_generation":"g1","shaping_inputs_path":inputs}),
        );
        let desired = desired();
        write(
            st.join("shaping/dataplane_transaction.json"),
            desired.clone(),
        );
        write(
            inputs.clone(),
            json!({"schema_version":1,"shaping_generation":"g1"}),
        );
        std::fs::write(root.join("lqos.conf"), "").unwrap();
        std::fs::write(
            lq.join("liblqos_python.py"),
            format!(
                "def validated_runtime_shaping_inputs_path():\n    return {:?}\n",
                inputs.to_str().unwrap()
            ),
        )
        .unwrap();
        serve(root.join("lqosd.sock"), reply(&desired));
        let mut cfg = Config::default();
        cfg.runtime.mode = RuntimeMode::Apply;
        cfg.libreqos.target = Target::V3Pair;
        cfg.libreqos.directory = lq.to_str().unwrap().into();
        cfg.libreqos.state_directory = st.to_str().unwrap().into();
        cfg.libreqos.v3_config = root.join("lqos.conf").to_str().unwrap().into();
        cfg.libreqos.v3_analytics_socket = root.join("lqosd.sock").to_str().unwrap().into();
        (dir, cfg, written)
    }
    #[tokio::test]
    async fn an_acknowledgement_for_the_pair_passes() {
        let (_dir, cfg, written) = live(|d| state("g1", d));
        let proof = verified(acknowledged(&cfg, &written, 5).await);
        assert_eq!(proof.acceptance().desired_circuit_capacity, 1000);
    }
    #[tokio::test]
    async fn an_old_native_generation_with_the_current_world_is_verified() {
        let (_dir, cfg, written) = live(|d| state("old", d));
        let proof = verified(acknowledged(&cfg, &written, 5).await);
        assert_eq!(proof.acceptance().shaping_generation, "g1");
    }
    #[tokio::test]
    async fn a_different_native_world_is_waiting_with_its_identities() {
        let (_dir, cfg, written) = live(|d| {
            let mut other = d.clone();
            other["leaf_ops"][0]["download"]["ceil_kbps"] = json!(1);
            state("old", &other)
        });
        let Proof::Waiting(w) = acknowledged(&cfg, &written, 5).await.unwrap() else {
            panic!("must wait");
        };
        assert_eq!(w.waiting[0].identity, "C1");
        assert_eq!(w.waiting[0].fields, ["download"]);
    }
    #[tokio::test]
    async fn a_generated_world_of_another_pair_is_an_error() {
        let (dir, cfg, written) = live(|d| state("g1", d));
        let path = dir.path().join("state/shaping/dataplane_transaction.json");
        let mut generated = desired();
        generated["leaf_ops"][0]["circuit_id"] = json!("X");
        std::fs::write(path, generated.to_string()).unwrap();
        assert!(acknowledged(&cfg, &written, 5).await.is_err());
    }
    #[tokio::test]
    async fn an_unavailable_acknowledgement_fails() {
        let (_dir, cfg, written) = live(|_| json!({"Unavailable":"x"}));
        let error = acknowledged(&cfg, &written, 5).await.unwrap_err();
        assert_eq!(error.to_string(), "native acknowledgement unavailable");
    }
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
            (
                dir.path().join("network.json"),
                r#"{"P":{"id":"n-p"}}"#.into(),
            ),
            (
                dir.path().join("ShapedDevices.csv"),
                crate::writer::pair::render_csv(&[row], Columns::V3),
            ),
        ];
        let mut tracker = Tracker::load(dir.path()).unwrap();
        tracker.write(
            &files,
            Meta {
                topology: None,
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
        let imported = json!({"imported":{"compatibility_network_json":{"P":{"id":"n-p"}},"shaped_devices":[{
            "circuit_id":"C1","device_id":"D1","circuit_name":"Client","device_name":"Device","parent_node":"P","mac":"",
            "ipv4":[["100.64.0.1",32]],"ipv6":[],"download_min_mbps":1,"upload_min_mbps":1,"download_max_mbps":20,"upload_max_mbps":10}]}});
        let generated = desired();
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
        let proof = verified(validate(
            tracker.current().unwrap(),
            &prepared,
            &artifacts,
            &applied,
            &applied,
            &path,
            &path,
        ));
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
        let proof = verified(validate(
            tracker.current().unwrap(),
            &prepared,
            &artifacts,
            &applied,
            &applied,
            &path,
            &path,
        ));
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
    fn boot_restored_partial_world_cannot_accept_a_newer_import() {
        let (_dir, mut tracker, prepared, artifacts, mut applied, path) = fixture();
        applied.generation = "boot-world".into();
        applied.transaction["leaf_ops"] = json!([]);
        let receipt = tracker.current().unwrap();
        let check = |native: &Applied| {
            validate(receipt, &prepared, &artifacts, native, native, &path, &path)
        };
        let Ok(Proof::Waiting(w)) = check(&applied) else {
            panic!("partial policy must wait");
        };
        assert_eq!(w.waiting[0].identity, "C1");
        assert_eq!(tracker.current().unwrap().phase, Phase::Pending);
        assert!(tracker.current().unwrap().acceptance.is_none());
        applied.transaction = artifacts.generated.clone();
        applied
            .transaction
            .as_object_mut()
            .unwrap()
            .remove("circuit_capacity");
        let proof = verified(check(&applied));
        tracker.accept(&proof, 11, &Hub::default()).unwrap();
        assert_eq!(tracker.current().unwrap().phase, Phase::Applied);
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
    fn topology_receipt_context_survives_the_existing_native_proof_path() {
        let (_dir, tracker, _prepared, artifacts, applied, path) = fixture();
        let mut receipt = tracker.current().unwrap().clone();
        receipt.meta.topology = Some(crate::write_receipt::TopologyContext {
            static_source: Some(crate::topology_manager::static_input::StaticBinding {
                path: "/owner/topology-manager/static-input.json".into(),
                revision: 1,
                digest: "a".repeat(64),
            }),
            compiler: None,
            policy_revision: 8,
            observation_generation: 4,
            registry_digest: "b".repeat(64),
        });
        let prepared = prepare(&receipt).unwrap();
        let proof = verified(validate(
            &receipt, &prepared, &artifacts, &applied, &applied, &path, &path,
        ));
        assert!(proof.matches(&receipt));
        for field in 0..6 {
            let mut changed = receipt.clone();
            let context = changed.meta.topology.as_mut().unwrap();
            match field {
                0 => context.policy_revision += 1,
                1 => context.observation_generation += 1,
                2 => context.registry_digest = "c".repeat(64),
                3 => context.static_source.as_mut().unwrap().revision += 1,
                4 => context.static_source.as_mut().unwrap().digest = "d".repeat(64),
                _ => {
                    context.static_source.as_mut().unwrap().path = "/other/static-input.json".into()
                }
            }
            assert!(!proof.matches(&changed));
            assert!(
                validate(
                    &changed, &prepared, &artifacts, &applied, &applied, &path, &path
                )
                .is_err()
            );
        }
    }

    #[test]
    fn topology_context_changes_cannot_reuse_prepared_or_verified_evidence() {
        let (_dir, tracker, prepared, artifacts, applied, path) = fixture();
        let receipt = tracker.current().unwrap();
        let proof = verified(validate(
            receipt, &prepared, &artifacts, &applied, &applied, &path, &path,
        ));
        let mut value = serde_json::to_value(receipt).unwrap();
        value["meta"]["topology"] = serde_json::json!({"policy_revision": 7, "observation_generation": 3, "registry_digest": "a".repeat(64)});
        let changed: Receipt = serde_json::from_value(value).unwrap();
        assert!(
            validate(
                &changed, &prepared, &artifacts, &applied, &applied, &path, &path
            )
            .is_err()
        );
        assert!(!proof.matches(&changed));
    }

    #[test]
    fn superseded_operation_cannot_consume_an_older_proof() {
        let (dir, mut tracker, prepared, artifacts, applied, path) = fixture();
        let proof = verified(validate(
            tracker.current().unwrap(),
            &prepared,
            &artifacts,
            &applied,
            &applied,
            &path,
            &path,
        ));
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
                topology: None,
                cycle_seq: 2,
                config_digest: "cfg".into(),
                notice: None,
            },
            11,
            &Hub::default(),
        );
        assert!(tracker.accept(&proof, 12, &Hub::default()).is_err());
    }
    #[test]
    fn compiler_drift_invalidates_prepared_and_already_constructed_proofs() {
        let (_native, cfg, compiler) = crate::topology_manager::compiler::tests::fixture();
        let (_dir, tracker, _prepared, artifacts, applied, path) = fixture();
        let mut receipt = tracker.current().unwrap().clone();
        receipt.meta.topology = Some(crate::write_receipt::TopologyContext {
            static_source: None,
            policy_revision: 1,
            observation_generation: 1,
            registry_digest: "a".repeat(64),
            compiler: Some(crate::topology_manager::compiler::tests::binding(&compiler)),
        });
        let prepared = prepare(&receipt).unwrap();
        let proof = verified(validate(
            &receipt, &prepared, &artifacts, &applied, &applied, &path, &path,
        ));
        assert!(proof.matches(&receipt));
        std::fs::write(cfg.v3_config, b"changed native configuration").unwrap();
        assert!(prepare(&receipt).is_err());
        assert!(!proof.matches(&receipt));
        assert!(
            validate(
                &receipt, &prepared, &artifacts, &applied, &applied, &path, &path
            )
            .is_err()
        );
    }
}
