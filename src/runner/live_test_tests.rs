//! Tests of the live acceptance test. A fake LibreQoS state directory and a fake acknowledgement
//! stand in for LibreQoS. No test touches a live system.
use super::enrollment::cli_flow::{AGE, confirm, setup};
use super::*;
use crate::apply_evidence::{Kind, Proof, TakeEffect, Waiting};
use crate::runner::live_test::{LiveIo, proof_result, restore_backup, restore_pair};
use crate::topology_manager::live_check;
use crate::topology_manager::publication::PreparedPair;
use serde_json::{Value, json};

const NOW: u64 = 1_800_000_000;

/// A fake LibreQoS. It imports the pair on disk when it sees a new reload request.
struct FakeLqos {
    lqos: PathBuf,
    state: PathBuf,
    nonce: Option<String>,
    imports: usize,
    /// The import with this index gets no state (an import timeout).
    stall: Option<usize>,
    /// The import with this index gets a changed circuit ceiling.
    skew: Option<usize>,
    /// The import with this index keeps the old shaping inputs and transaction.
    stale: Option<usize>,
    /// The import with this index keeps the old source generation.
    keep_source: Option<usize>,
    ack: bool,
    acks: Vec<Vec<(PathBuf, Vec<u8>)>>,
}

impl FakeLqos {
    fn new(c: &Config) -> Self {
        FakeLqos {
            lqos: PathBuf::from(&c.libreqos.directory),
            state: PathBuf::from(&c.libreqos.state_directory),
            nonce: None,
            imports: 0,
            stall: None,
            skew: None,
            stale: None,
            keep_source: None,
            ack: true,
            acks: vec![],
        }
    }

    /// Write the five fresh state files for the pair on disk, as LibreQoS does.
    /// The generations are content hashes: the same pair gives the same generations.
    /// `index` is the import number that the knobs refer to. The baseline has no index.
    fn simulate(&self, index: Option<usize>) {
        let knob = |k: Option<usize>| index.is_some() && k == index;
        let (csv, network_text) = (
            read(&self.lqos.join("ShapedDevices.csv")),
            read(&self.lqos.join("network.json")),
        );
        let rows = crate::state::read_csv(&csv).unwrap();
        let network: Value = serde_json::from_str(&network_text).unwrap();
        let hash = crate::write_receipt::sha256(format!("{network_text}\n{csv}").as_bytes());
        let mut source = format!("src-{}", &hash[..12]);
        if knob(self.keep_source) {
            let old: Value = serde_json::from_str(&read(
                &self.state.join("topology/topology_runtime_status.json"),
            ))
            .unwrap();
            source = old["source_generation"].as_str().unwrap().to_string();
        }
        let mut x = live_check::expected(&rows, &network).unwrap();
        if knob(self.skew) {
            x.circuits.values_mut().next().unwrap().download.ceil_kbps += 1000;
        }
        let rate = |s: &str| s.trim().parse::<f32>().unwrap_or(0.0);
        let ips = |s: &str, width: u8| -> Vec<Value> {
            s.split(',')
                .map(str::trim)
                .filter(|a| !a.is_empty())
                .map(|a| {
                    let (addr, prefix) = a.split_once('/').unwrap_or((a, ""));
                    json!([addr, prefix.parse::<u8>().unwrap_or(width)])
                })
                .collect()
        };
        let devices: Vec<Value> = rows
            .iter()
            .map(|r| {
                json!({"circuit_id": r.circuit_id, "circuit_name": r.circuit_name,
                "device_id": r.device_id, "device_name": r.device_name,
                "parent_node": r.parent_node,
                "parent_node_id": (!r.parent_node_id.is_empty()).then(|| r.parent_node_id.clone()),
                "anchor_node_id": (!r.anchor_node_id.is_empty()).then(|| r.anchor_node_id.clone()),
                "mac": r.mac, "comment": r.comment,
                "ipv4": ips(&r.ipv4, 32), "ipv6": ips(&r.ipv6, 128),
                "download_min_mbps": rate(&r.download_min), "upload_min_mbps": rate(&r.upload_min),
                "download_max_mbps": rate(&r.download_max), "upload_max_mbps": rate(&r.upload_max)})
            })
            .collect();
        let rates =
            |r: &live_check::Rates| json!({"ceil_kbps": r.ceil_kbps, "commit_kbps": r.commit_kbps});
        let generation = format!("gen-{}", &hash[..12]);
        let files = [
            (
                "topology/topology_import.json",
                json!({"imported": {"compatibility_network_json": network, "shaped_devices": devices}}),
            ),
            (
                "topology/topology_runtime_status.json",
                json!({"ready": true, "source_generation": source,
                    "shaping_generation": generation}),
            ),
            (
                "shaping/topology_compiled_shaping.json",
                json!({"shaped_devices": rows.iter().map(|r| json!({"circuit_id": r.circuit_id})).collect::<Vec<_>>()}),
            ),
            (
                "shaping/shaping_inputs.json",
                json!({"shaping_generation": generation,
                    "circuits": x.circuits.keys().map(|c| json!({"circuit_id": c})).collect::<Vec<_>>()}),
            ),
            (
                "shaping/dataplane_transaction.json",
                json!({"generation": generation,
                    "inner_ops": x.nodes.iter().map(|(id, n)| json!({"node_id": id, "parent_node_id": n.parent,
                        "download": rates(&n.download), "upload": rates(&n.upload)})).collect::<Vec<_>>(),
                    "leaf_ops": x.circuits.iter().map(|(id, c)| json!({"circuit_id": id, "parent_node_id": c.parent,
                        "download": rates(&c.download), "upload": rates(&c.upload)})).collect::<Vec<_>>()}),
            ),
        ];
        for (rel, v) in files {
            if knob(self.stale)
                && rel.starts_with("shaping/")
                && rel != "shaping/topology_compiled_shaping.json"
            {
                continue;
            }
            let path = self.state.join(rel);
            std::fs::create_dir_all(path.parent().unwrap()).unwrap();
            std::fs::write(path, v.to_string()).unwrap();
        }
    }
}

impl LiveIo for FakeLqos {
    fn now(&self) -> u64 {
        NOW
    }
    async fn admit(&mut self, _cfg: &Config, _pair: PreparedPair) -> anyhow::Result<()> {
        Ok(())
    }
    async fn pause(&mut self) {
        let request = read(&self.state.join("scheduler/reload_request.json"));
        let nonce = serde_json::from_str::<Value>(&request)
            .ok()
            .and_then(|v| v["nonce"].as_str().map(str::to_string));
        if nonce.is_some() && nonce != self.nonce {
            self.nonce = nonce;
            let n = self.imports;
            self.imports += 1;
            if self.stall != Some(n) {
                self.simulate(Some(n));
            }
        }
        tokio::time::sleep(Duration::from_millis(5)).await;
    }
    async fn acknowledge(
        &mut self,
        _cfg: &Config,
        written: &[(PathBuf, Vec<u8>)],
        _started: u64,
    ) -> anyhow::Result<()> {
        self.acks.push(written.to_vec());
        anyhow::ensure!(self.ack, "lqosd is not reachable");
        Ok(())
    }
}

/// An enrolled registry, the legacy pair on disk, and a LibreQoS that applied the legacy pair.
async fn enrolled(e: &Env) -> (Config, FakeLqos) {
    let c = setup(e, None);
    let mut r = runner(e, c.clone());
    let draft = r.propose_policy(AGE).await.unwrap();
    r.install_topology_policy(&draft).unwrap();
    let mut file = r.propose_enrollment(None, AGE, false).await.unwrap();
    confirm(&mut file);
    r.enroll_with_mapping(&file, AGE, true, false)
        .await
        .unwrap();
    drop(r);
    let fake = FakeLqos::new(&c);
    fake.simulate(None);
    (c, fake)
}

fn pair(e: &Env) -> (String, String) {
    (
        read(&e.lqos.join("network.json")),
        read(&e.lqos.join("ShapedDevices.csv")),
    )
}

/// All files under the state directory, except the live test backups.
fn state_files(e: &Env) -> Vec<(PathBuf, Vec<u8>)> {
    let mut out = vec![];
    let mut work = vec![e.state.clone()];
    while let Some(d) = work.pop() {
        for entry in std::fs::read_dir(&d).unwrap().flatten() {
            let p = entry.path();
            if p.ends_with("topology-manager/live-test") {
                continue;
            }
            if p.is_dir() {
                work.push(p);
            } else {
                out.push((p.clone(), std::fs::read(&p).unwrap()));
            }
        }
    }
    out.sort();
    out
}

fn backups(e: &Env) -> PathBuf {
    e.state.join("topology-manager/live-test")
}

const SHORT: Duration = Duration::from_millis(300);

#[tokio::test]
async fn a_passing_run_restores_the_legacy_pair_byte_for_byte() {
    let e = env();
    let (c, mut fake) = enrolled(&e).await;
    let legacy = pair(&e);
    let mut r = runner(&e, c);
    let report = r.live_test_with(&mut fake, SHORT).await.unwrap();
    assert!(report.passed, "{}", report.summary());
    assert_eq!(report.error, None);
    assert!(report.restored);
    assert_eq!(pair(&e), legacy);
    let dir = report.backup_dir.clone().unwrap();
    assert_eq!(dir, backups(&e).join(NOW.to_string()));
    assert_eq!(read(&dir.join("network.json")), legacy.0);
    assert_eq!(read(&dir.join("ShapedDevices.csv")), legacy.1);
    assert!(
        dir.join("libreqos-state/topology/topology_import.json")
            .exists()
    );
    assert!(
        dir.join("libreqos-state/shaping/dataplane_transaction.json")
            .exists()
    );
    assert_eq!(fake.imports, 2);
    assert_eq!(fake.acks.len(), 1);
    let written: Vec<_> = fake.acks[0].iter().map(|(p, _)| p.clone()).collect();
    assert_eq!(
        written,
        [
            e.lqos.join("network.json"),
            e.lqos.join("ShapedDevices.csv")
        ]
    );
    // The manager pair differs from the legacy pair in bytes, so the test wrote it.
    assert_ne!(
        (fake.acks[0][0].1.clone(), fake.acks[0][1].1.clone()),
        (legacy.0.into_bytes(), legacy.1.into_bytes())
    );
    assert!(report.manager.as_ref().unwrap().differences.is_empty());
    assert!(report.restore.as_ref().unwrap().differences.is_empty());
    assert!(report.steps.iter().all(|s| s.ok));
    serde_json::to_string(&report).unwrap();
}

#[tokio::test]
async fn a_parity_difference_writes_nothing() {
    let e = env();
    let (c, mut fake) = enrolled(&e).await;
    let mut rows = crate::state::read_csv(&read(&e.lqos.join("ShapedDevices.csv"))).unwrap();
    rows[0].download_max = "999".into();
    std::fs::write(
        e.lqos.join("ShapedDevices.csv"),
        render_csv(&rows, Columns::V3),
    )
    .unwrap();
    let legacy = pair(&e);
    let lqstate = std::fs::read_dir(e.lqos.join("lqstate/topology"))
        .unwrap()
        .count();
    let mut r = runner(&e, c);
    let report = r.live_test_with(&mut fake, SHORT).await.unwrap();
    assert!(!report.passed);
    assert!(!report.parity.is_empty(), "{}", report.summary());
    assert_eq!(report.backup_dir, None);
    assert_eq!(pair(&e), legacy);
    assert!(!backups(&e).exists());
    assert!(
        !e.lqos
            .join("lqstate/scheduler/reload_request.json")
            .exists()
    );
    assert_eq!(
        std::fs::read_dir(e.lqos.join("lqstate/topology"))
            .unwrap()
            .count(),
        lqstate
    );
    assert_eq!((fake.imports, fake.acks.len()), (0, 0));
}

#[tokio::test]
async fn an_import_timeout_restores_and_reports() {
    let e = env();
    let (c, mut fake) = enrolled(&e).await;
    fake.stall = Some(0);
    let legacy = pair(&e);
    let mut r = runner(&e, c);
    let report = r.live_test_with(&mut fake, SHORT).await.unwrap();
    assert!(!report.passed);
    let error = report.error.clone().unwrap();
    assert!(error.contains("import timeout"), "{error}");
    assert!(!error.contains("RESTORE FAILED"), "{error}");
    assert!(report.restored);
    assert_eq!(pair(&e), legacy);
    assert!(fake.acks.is_empty());
    assert!(report.backup_dir.is_some());
}

fn source_generation(e: &Env) -> Value {
    let status = read(&e.lqos.join("lqstate/topology/topology_runtime_status.json"));
    serde_json::from_str::<Value>(&status).unwrap()["source_generation"].clone()
}

#[tokio::test]
async fn a_manager_stall_then_a_restore_with_the_baseline_generation_passes_the_restore() {
    let e = env();
    let (c, mut fake) = enrolled(&e).await;
    fake.stall = Some(0);
    let baseline = source_generation(&e);
    let mut r = runner(&e, c);
    let report = r.live_test_with(&mut fake, SHORT).await.unwrap();
    // The restored legacy pair gives the baseline generation again, as on production.
    assert_eq!(source_generation(&e), baseline);
    // The legacy state with the baseline generation is good at once, with or without a new import.
    assert!(fake.imports >= 1);
    assert!(report.restored, "{}", report.summary());
    assert!(!report.error.unwrap().contains("RESTORE FAILED"));
}

#[tokio::test]
async fn a_manager_pair_equal_to_the_legacy_bytes_writes_nothing() {
    let e = env();
    let (c, mut fake) = enrolled(&e).await;
    let mut r = runner(&e, c.clone());
    let manager = r.prepare_topology(AGE).await.unwrap();
    drop(r);
    std::fs::write(e.lqos.join("network.json"), manager.network_text()).unwrap();
    std::fs::write(e.lqos.join("ShapedDevices.csv"), manager.csv()).unwrap();
    fake.simulate(None);
    let legacy = pair(&e);
    let mut r = runner(&e, c);
    let report = r.live_test_with(&mut fake, SHORT).await.unwrap();
    assert!(!report.passed);
    let error = report.error.unwrap();
    assert!(error.contains("byte for byte"), "{error}");
    assert_eq!(report.backup_dir, None);
    assert_eq!(pair(&e), legacy);
    assert!(!backups(&e).exists());
    assert!(
        !e.lqos
            .join("lqstate/scheduler/reload_request.json")
            .exists()
    );
}

#[tokio::test]
async fn a_stale_transaction_generation_does_not_pass() {
    let e = env();
    let (c, mut fake) = enrolled(&e).await;
    fake.stale = Some(0);
    let legacy = pair(&e);
    let mut r = runner(&e, c);
    let report = r.live_test_with(&mut fake, SHORT).await.unwrap();
    assert!(!report.passed, "{}", report.summary());
    let error = report.error.clone().unwrap();
    assert!(error.contains("import timeout"), "{error}");
    assert!(report.restored, "{}", report.summary());
    assert_eq!(pair(&e), legacy);
}

#[tokio::test]
async fn the_same_source_generation_for_the_manager_pair_is_reported() {
    let e = env();
    let (c, mut fake) = enrolled(&e).await;
    fake.keep_source = Some(0);
    let mut r = runner(&e, c);
    let report = r.live_test_with(&mut fake, SHORT).await.unwrap();
    assert!(!report.passed);
    let error = report.error.clone().unwrap();
    assert!(error.contains("same source generation"), "{error}");
    assert!(report.restored, "{}", report.summary());
}

#[tokio::test]
async fn a_huge_import_timeout_does_not_panic() {
    let e = env();
    let (c, mut fake) = enrolled(&e).await;
    let mut r = runner(&e, c);
    let report = r.live_test_with(&mut fake, Duration::MAX).await.unwrap();
    assert!(report.passed, "{}", report.summary());
}

#[tokio::test]
async fn a_stalled_restore_import_says_the_files_are_restored() {
    let e = env();
    let (c, mut fake) = enrolled(&e).await;
    fake.stall = Some(1);
    let legacy = pair(&e);
    let mut r = runner(&e, c);
    let report = r.live_test_with(&mut fake, SHORT).await.unwrap();
    assert!(!report.restored);
    let error = report.error.unwrap();
    assert!(error.contains("RESTORE FAILED"), "{error}");
    assert!(error.contains("files are restored"), "{error}");
    assert!(!error.contains("can still be applied"), "{error}");
    assert_eq!(pair(&e), legacy);
}

#[tokio::test]
async fn a_read_back_difference_restores_and_reports() {
    let e = env();
    let (c, mut fake) = enrolled(&e).await;
    fake.skew = Some(0);
    let legacy = pair(&e);
    let mut r = runner(&e, c);
    let report = r.live_test_with(&mut fake, SHORT).await.unwrap();
    assert!(!report.passed);
    assert!(!report.manager.as_ref().unwrap().differences.is_empty());
    assert!(report.error.clone().unwrap().contains("read-back"));
    assert!(report.restored);
    assert!(report.restore.as_ref().unwrap().differences.is_empty());
    assert_eq!(pair(&e), legacy);
}

#[tokio::test]
async fn an_unavailable_acknowledgement_restores() {
    let e = env();
    let (c, mut fake) = enrolled(&e).await;
    fake.ack = false;
    let legacy = pair(&e);
    let mut r = runner(&e, c);
    let report = r.live_test_with(&mut fake, SHORT).await.unwrap();
    assert!(!report.passed);
    let error = report.error.clone().unwrap();
    assert!(
        error.contains("native acknowledgement unavailable"),
        "{error}"
    );
    assert!(!fake.acks.is_empty());
    assert!(report.restored);
    assert_eq!(pair(&e), legacy);
}

#[test]
fn restore_pair_brings_back_the_backup_after_an_interruption() {
    let dir = tempfile::tempdir().unwrap();
    let (backup, lqos) = (dir.path().join("backup"), dir.path().join("lqos"));
    std::fs::create_dir_all(&backup).unwrap();
    std::fs::create_dir_all(&lqos).unwrap();
    std::fs::write(backup.join("network.json"), "{\"legacy\":1}").unwrap();
    std::fs::write(backup.join("ShapedDevices.csv"), "legacy csv\n").unwrap();
    // The interruption came after the manager JSON and before the manager CSV.
    std::fs::write(lqos.join("network.json"), "{\"manager\":1}").unwrap();
    std::fs::write(lqos.join("ShapedDevices.csv"), "legacy csv\n").unwrap();
    restore_pair(&backup, &lqos).unwrap();
    assert_eq!(read(&lqos.join("network.json")), "{\"legacy\":1}");
    assert_eq!(read(&lqos.join("ShapedDevices.csv")), "legacy csv\n");
    // An incomplete backup changes nothing.
    std::fs::write(lqos.join("network.json"), "{\"manager\":1}").unwrap();
    std::fs::remove_file(backup.join("ShapedDevices.csv")).unwrap();
    assert!(restore_pair(&backup, &lqos).is_err());
    assert_eq!(read(&lqos.join("network.json")), "{\"manager\":1}");
}

#[tokio::test]
async fn the_test_never_writes_the_owner_file_receipts_or_registry() {
    let e = env();
    let (c, mut fake) = enrolled(&e).await;
    let before = state_files(&e);
    let mut r = runner(&e, c);
    let report = r.live_test_with(&mut fake, SHORT).await.unwrap();
    assert!(report.passed, "{}", report.summary());
    assert_eq!(state_files(&e), before);
    assert!(!e.state.join("topology-manager/runtime-owner.json").exists());
    assert!(!e.state.join("write-receipt.json").exists());
    assert!(!e.state.join("write-events").exists());
    assert!(r.pending_tracker().is_none());
    drop(r);
    let store = crate::topology_manager::store::RegistryStore::open(&e.state).unwrap();
    assert_eq!(store.document().unwrap().revision, 2);
}

#[tokio::test]
async fn the_test_needs_the_instance_lock() {
    let e = env();
    let (c, mut fake) = enrolled(&e).await;
    let legacy = pair(&e);
    let _held = crate::files::InstanceLock::acquire(&e.state).unwrap();
    let mut r = runner(&e, c);
    let error = r.live_test_with(&mut fake, SHORT).await.unwrap_err();
    assert!(
        format!("{error:#}").contains("another lqosync3"),
        "{error:#}"
    );
    assert_eq!(pair(&e), legacy);
    assert!(!backups(&e).exists());
    assert_eq!((fake.imports, fake.acks.len()), (0, 0));
}

/// Change one LibreQoS state file in place.
fn edit_state(e: &Env, rel: &str, f: impl FnOnce(&mut Value)) {
    let path = e.lqos.join("lqstate").join(rel);
    let mut v: Value = serde_json::from_str(&read(&path)).unwrap();
    f(&mut v);
    std::fs::write(path, v.to_string()).unwrap();
}

/// Nothing reached LibreQoS: the pair is the legacy pair and no import was requested.
fn nothing_written(e: &Env, fake: &FakeLqos, legacy: &(String, String)) {
    assert_eq!(&pair(e), legacy);
    assert!(
        !e.lqos
            .join("lqstate/scheduler/reload_request.json")
            .exists()
    );
    assert_eq!((fake.imports, fake.acks.len()), (0, 0));
}

#[tokio::test]
async fn a_baseline_difference_stops_before_the_manager_write() {
    let e = env();
    let (c, mut fake) = enrolled(&e).await;
    edit_state(&e, "shaping/dataplane_transaction.json", |v| {
        v["leaf_ops"][0]["download"]["ceil_kbps"] = json!(1);
    });
    let legacy = pair(&e);
    let mut r = runner(&e, c);
    let report = r.live_test_with(&mut fake, SHORT).await.unwrap();
    assert!(!report.passed);
    let error = report.error.clone().unwrap();
    assert!(error.contains("legacy model"), "{error}");
    assert!(error.contains("changed"), "{error}");
    assert!(!report.baseline.as_ref().unwrap().differences.is_empty());
    nothing_written(&e, &fake, &legacy);
}

#[tokio::test]
async fn a_baseline_that_is_not_the_imported_legacy_pair_stops_before_the_write() {
    let e = env();
    let (c, mut fake) = enrolled(&e).await;
    // LibreQoS imported an older pair: one device less than the pair on disk.
    edit_state(&e, "topology/topology_import.json", |v| {
        v["imported"]["shaped_devices"]
            .as_array_mut()
            .unwrap()
            .pop();
    });
    let legacy = pair(&e);
    let mut r = runner(&e, c);
    let report = r.live_test_with(&mut fake, SHORT).await.unwrap();
    assert!(!report.passed);
    let error = report.error.clone().unwrap();
    assert!(error.contains("import timeout"), "{error}");
    assert!(error.contains("baseline"), "{error}");
    nothing_written(&e, &fake, &legacy);
}

#[tokio::test]
async fn a_manager_pair_that_loses_a_circuit_writes_nothing() {
    let e = env();
    let (c, mut fake) = enrolled(&e).await;
    let mut rows = crate::state::read_csv(&read(&e.lqos.join("ShapedDevices.csv"))).unwrap();
    let mut lost = rows[0].clone();
    lost.circuit_id = "lost-circuit".into();
    lost.device_id = "lost-device".into();
    rows.push(lost);
    std::fs::write(
        e.lqos.join("ShapedDevices.csv"),
        render_csv(&rows, Columns::V3),
    )
    .unwrap();
    let legacy = pair(&e);
    let mut r = runner(&e, c);
    let report = r.live_test_with(&mut fake, SHORT).await.unwrap();
    assert!(!report.passed);
    assert!(
        report
            .parity
            .iter()
            .any(|p| p.contains("MissingRow") && p.contains("lost-circuit")),
        "{:?}",
        report.parity
    );
    assert_eq!(report.backup_dir, None);
    assert!(!backups(&e).exists());
    nothing_written(&e, &fake, &legacy);
}

#[tokio::test]
async fn a_restore_mismatch_names_the_changed_circuit() {
    let e = env();
    let (c, mut fake) = enrolled(&e).await;
    fake.skew = Some(1);
    let mut r = runner(&e, c);
    let report = r.live_test_with(&mut fake, SHORT).await.unwrap();
    assert!(!report.restored);
    let error = report.error.unwrap();
    assert!(error.contains("RESTORE FAILED"), "{error}");
    let changed = &report.restore.as_ref().unwrap().differences[0];
    let id = changed
        .strip_prefix("circuit ")
        .and_then(|s| s.strip_suffix(" changed"))
        .unwrap();
    assert!(
        error.contains(&format!("differs from the baseline: circuit {id}")),
        "{error}"
    );
}

/// A backup folder as the live test makes it: the legacy pair and the LibreQoS status.
fn manual_backup(e: &Env) -> PathBuf {
    let dir = backups(e).join("1");
    std::fs::create_dir_all(dir.join("libreqos-state/topology")).unwrap();
    for n in ["network.json", "ShapedDevices.csv"] {
        std::fs::copy(e.lqos.join(n), dir.join(n)).unwrap();
    }
    let status = "topology/topology_runtime_status.json";
    std::fs::copy(
        e.lqos.join("lqstate").join(status),
        dir.join("libreqos-state").join(status),
    )
    .unwrap();
    dir
}

/// Put the manager pair in place and let the fake LibreQoS apply it, as after an interruption.
async fn apply_manager(e: &Env, c: &Config, fake: &FakeLqos) {
    let mut r = runner(e, c.clone());
    let manager = r.prepare_topology(AGE).await.unwrap();
    drop(r);
    std::fs::write(e.lqos.join("network.json"), manager.network_text()).unwrap();
    std::fs::write(e.lqos.join("ShapedDevices.csv"), manager.csv()).unwrap();
    fake.simulate(None);
}

#[tokio::test]
async fn live_test_restore_waits_for_the_legacy_import() {
    let e = env();
    let (c, mut fake) = enrolled(&e).await;
    let legacy = pair(&e);
    let dir = manual_backup(&e);
    apply_manager(&e, &c, &fake).await;
    restore_backup(&c, &mut fake, &dir, SHORT).await.unwrap();
    assert_eq!(pair(&e), legacy);
    assert_eq!(fake.imports, 1);
    // LibreQoS does not import the restored pair: the files are back, but the command fails.
    apply_manager(&e, &c, &fake).await;
    fake.stall = Some(1);
    let error = restore_backup(&c, &mut fake, &dir, SHORT)
        .await
        .unwrap_err();
    let error = format!("{error:#}");
    assert!(error.contains("import timeout"), "{error}");
    assert!(error.contains("files are restored"), "{error}");
    assert_eq!(pair(&e), legacy);
}

#[test]
fn a_waiting_proof_gives_the_waiting_identities_in_the_error() {
    let waiting = TakeEffect {
        applied: 0,
        waiting: vec![Waiting {
            kind: Kind::Circuit,
            identity: "C1".into(),
            fields: vec!["download".into()],
        }],
        waiting_removal: vec![],
    };
    let text = format!("{}", proof_result(Proof::Waiting(waiting)).unwrap_err());
    assert!(text.contains("waiting to take effect"), "{text}");
    assert!(text.contains("C1"), "{text}");
}
