//! Read-only input/runtime/native-ACK contract probe. Output contains no subscriber identifiers.
use anyhow::{Context, ensure};
use lqosync3::{apply_evidence::read_applied, import_evidence::ExpectedImport};
use serde_json::{Value, json};
use std::{
    collections::BTreeMap,
    io::Read,
    path::{Path, PathBuf},
};

fn read(path: &Path) -> anyhow::Result<Vec<u8>> {
    const LIMIT: u64 = 32 * 1024 * 1024;
    let mut bytes = Vec::new();
    std::fs::File::open(path)
        .with_context(|| format!("cannot read {}", path.display()))?
        .take(LIMIT + 1)
        .read_to_end(&mut bytes)?;
    ensure!(bytes.len() as u64 <= LIMIT, "artifact exceeds probe bound");
    Ok(bytes)
}
fn artifacts(paths: &[PathBuf]) -> anyhow::Result<Vec<Vec<u8>>> {
    paths.iter().map(|p| read(p)).collect()
}
fn operation_counts(transaction: &Value) -> BTreeMap<&'static str, usize> {
    [
        "inner_ops",
        "leaf_ops",
        "mapping_ops",
        "lpm_delta",
        "steering_vlans",
    ]
    .into_iter()
    .map(|k| (k, transaction[k].as_array().map_or(0, Vec::len)))
    .collect()
}
fn private_capture(
    dir: &Path,
    applied: &lqosync3::apply_evidence::Applied,
) -> anyhow::Result<PathBuf> {
    use std::os::unix::fs::{OpenOptionsExt, PermissionsExt};
    let path = dir.join(format!(
        "native-applied-{}.json",
        lqosync3::runner::random_id(12)
    ));
    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .custom_flags(libc::O_NOFOLLOW)
        .open(&path)?;
    file.set_permissions(std::fs::Permissions::from_mode(0o600))?;
    if let Some(uid) = std::env::var("SUDO_UID")
        .ok()
        .and_then(|s| s.parse::<u32>().ok())
    {
        std::os::unix::fs::fchown(&file, Some(uid), None)?;
    }
    serde_json::to_writer(
        &mut file,
        &json!({"peer_pid":applied.peer_pid,"generation":applied.generation,
        "content_key":applied.content_key,"desired_world_content_key":applied.desired_world_content_key,
        "applied_circuit_capacity":applied.applied_circuit_capacity,
        "transaction":applied.transaction}),
    )?;
    Ok(path)
}
#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let raw_args: Vec<_> = std::env::args().skip(1).collect();
    let verify_current = raw_args.iter().any(|s| s == "--verify-current");
    let capture_dir = raw_args
        .iter()
        .find_map(|s| s.strip_prefix("--capture="))
        .map(PathBuf::from);
    let args: Vec<_> = raw_args
        .into_iter()
        .filter(|s| !s.starts_with("--capture=") && s != "--verify-current")
        .collect();
    let socket = Path::new(
        args.first()
            .map(String::as_str)
            .unwrap_or("/dev/shm/libreqos-analytics.sock"),
    );
    let source = Path::new(
        args.get(1)
            .map(String::as_str)
            .unwrap_or("/opt/libreqos/src"),
    );
    let state = Path::new(
        args.get(2)
            .map(String::as_str)
            .unwrap_or("/opt/libreqos/state"),
    );
    let paths = [
        source.join("network.json"),
        source.join("ShapedDevices.csv"),
        state.join("topology/topology_import.json"),
        state.join("topology/topology_runtime_status.json"),
        state.join("shaping/dataplane_transaction.json"),
    ];
    if verify_current {
        let input = artifacts(&paths[..2])?;
        let receipt = lqosync3::write_receipt::Receipt {
            version: 1,
            write_id: lqosync3::runner::random_id(24),
            phase: lqosync3::system_health::Phase::Pending,
            started: 0,
            updated: 0,
            meta: lqosync3::write_receipt::Meta {
                cycle_seq: 0,
                config_digest: "read-only-probe".into(),
                notice: None,
            },
            files: paths[..2]
                .iter()
                .zip(&input)
                .map(|(path, bytes)| lqosync3::write_receipt::FileReceipt {
                    path: path.clone(),
                    sha256: lqosync3::write_receipt::sha256(bytes),
                    outcome: lqosync3::write_receipt::FileOutcome::Unchanged,
                })
                .collect(),
            error: None,
            acceptance: None,
            superseded_by: None,
        };
        let mut cfg = lqosync3::config::defaults().clone();
        cfg.runtime.mode = lqosync3::config::RuntimeMode::Apply;
        cfg.libreqos.target = lqosync3::config::Target::V3Pair;
        cfg.libreqos.directory = source.to_string_lossy().into();
        cfg.libreqos.state_directory = state.to_string_lossy().into();
        cfg.libreqos.v3_analytics_socket = socket.to_string_lossy().into();
        let proof = match lqosync3::apply_evidence::verify_receipt(&cfg, &receipt).await {
            Ok(proof) => proof,
            Err(error) => {
                let applied = read_applied(socket).await?;
                eprintln!(
                    "native capacity {:?}; requested capacity {}; native schema {}; generated schema {}",
                    applied.applied_circuit_capacity,
                    serde_json::from_slice::<Value>(&read(&paths[4])?)?["circuit_capacity"],
                    applied.transaction["schema_version"],
                    serde_json::from_slice::<Value>(&read(&paths[4])?)?["schema_version"]
                );
                if let Some(dir) = &capture_dir {
                    let path = private_capture(dir, &applied)?;
                    eprintln!("Private diagnostic capture saved: {}", path.display());
                }
                return Err(error);
            }
        };
        println!(
            "{}",
            json!({"read_only_acceptance_verified":true,"evidence":proof.acceptance(),"managed_files_written":false})
        );
        return Ok(());
    }
    for index in 0..3 {
        let before = artifacts(&paths)?;
        let first = read_applied(socket).await?;
        let second = read_applied(socket).await?;
        let after = artifacts(&paths)?;
        let rows = lqosync3::state::read_csv(std::str::from_utf8(&before[1])?)?;
        let expected = ExpectedImport::new(std::str::from_utf8(&before[0])?, &rows)?;
        let imported: Value = serde_json::from_slice(&before[2])?;
        let runtime: Value = serde_json::from_slice(&before[3])?;
        let transaction: Value = serde_json::from_slice(&before[4])?;
        let stable = before == after && first == second;
        let capture = if index == 0 {
            capture_dir
                .as_ref()
                .map(|dir| private_capture(dir, &first))
                .transpose()?
        } else {
            None
        };
        println!(
            "{}",
            json!({
                "sample":index+1,"private_capture":capture,"artifacts_and_ack_stable":stable,"source_rows":rows.len(),
                "import_matches_pair":expected.matches(&imported),
                "runtime_ready":runtime["ready"].as_bool(),
                "runtime_schema_supported":runtime["schema_version"]==1,
                "generated_transaction_schema":transaction["schema_version"],
            "native_transaction_schema":first.transaction["schema_version"],
            "transaction_schemas_match":transaction["schema_version"]==first.transaction["schema_version"],
                "ack_matches_shaping_generation":runtime["shaping_generation"].as_str()==Some(first.generation.as_str()),
                "generated_matches_shaping_generation":transaction["generation"]==runtime["shaping_generation"],
                "native_transaction_equals_generated":first.transaction==transaction,
                "native_applied_circuit_capacity":first.applied_circuit_capacity,
                "native_content_key_present":!first.content_key.is_empty(),
                "native_desired_key_present":!first.desired_world_content_key.is_empty(),
                "native_and_desired_keys_equal":first.content_key==first.desired_world_content_key,
                "native_operation_counts":operation_counts(&first.transaction),
                "generated_operation_counts":operation_counts(&transaction),
                "peer_pid":first.peer_pid,
                "applied_contract_verified":false,
                "verification_gap":"Input-to-generation correlation and folded transaction semantics need producer validation."
            })
        );
        if index < 2 {
            tokio::time::sleep(std::time::Duration::from_secs(2)).await;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn native_capture_is_private_and_preserves_transaction_representation() {
        use std::os::unix::fs::PermissionsExt;
        let dir = tempfile::tempdir().unwrap();
        let applied = lqosync3::apply_evidence::Applied {
            peer_pid: 1,
            generation: "g1".into(),
            transaction: json!({"schema_version":6,"generation":"g1","mapping_ops":[]}),
            content_key: "key".into(),
            desired_world_content_key: "desired".into(),
            applied_circuit_capacity: None,
        };
        let path = private_capture(dir.path(), &applied).unwrap();
        assert_eq!(
            std::fs::metadata(&path).unwrap().permissions().mode() & 0o777,
            0o600
        );
        let value: Value = serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
        assert_eq!(value["transaction"], applied.transaction);
        assert_ne!(private_capture(dir.path(), &applied).unwrap(), path);
    }
}
