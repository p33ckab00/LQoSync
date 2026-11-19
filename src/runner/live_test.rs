//! One-shot live acceptance test of the manager pair on the production LibreQoS.
//! It writes the manager pair, reads back what LibreQoS applied, and restores the legacy pair.
//! It never claims the writer, never writes a receipt or a notification, and never commits the registry.

use super::Runner;
use crate::config::{Config, RuntimeMode, Target};
use crate::files;
use crate::model::CsvRow;
use crate::routeros::Connector;
use crate::topology_manager::live_check::{self, Diff, Expected, Observed};
use crate::topology_manager::publication::PreparedPair;
use anyhow::{Context, ensure};
use serde::Serialize;
use std::os::unix::fs::DirBuilderExt;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

/// The source age limit of the prepared pair. It also limits the compilation.
const MAX_AGE: Duration = Duration::from_secs(120);
/// The input pair, in write order: JSON first, then CSV.
const PAIR: [&str; 2] = ["network.json", "ShapedDevices.csv"];

/// The parts of the test that touch LibreQoS. Tests use a fake LibreQoS.
pub trait LiveIo {
    /// The time in Unix seconds.
    fn now(&self) -> u64;
    /// Compile the pair with the controlled compiler and run the compiler admission.
    fn admit(
        &mut self,
        cfg: &Config,
        pair: PreparedPair,
    ) -> impl Future<Output = anyhow::Result<()>>;
    /// Wait a short time before the next check of the LibreQoS state.
    fn pause(&mut self) -> impl Future<Output = ()>;
    /// Read the native acknowledgement of the written pair.
    fn acknowledge(
        &mut self,
        cfg: &Config,
        written: &[(PathBuf, Vec<u8>)],
        started: u64,
    ) -> impl Future<Output = anyhow::Result<()>>;
}

/// The production LibreQoS: the installed compiler and the lqosd acknowledgement.
pub struct NativeIo;

impl LiveIo for NativeIo {
    fn now(&self) -> u64 {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs()
    }
    async fn admit(&mut self, cfg: &Config, pair: PreparedPair) -> anyhow::Result<()> {
        use crate::topology_manager::compiler::ControlledCompiler;
        let scratch = Path::new(&cfg.runtime.state_dir).join("topology-manager/live-test-compiler");
        let result = async {
            let mut compiler = ControlledCompiler::open(&cfg.libreqos, &scratch)?;
            compiler.check_config(&cfg.libreqos)?;
            compiler.compile(pair).await?.check()
        }
        .await;
        // The compiler removes its own workspace. Remove the empty scratch folder too.
        let _ = std::fs::remove_dir(&scratch);
        result
    }
    async fn pause(&mut self) {
        tokio::time::sleep(Duration::from_secs(1)).await;
    }
    async fn acknowledge(
        &mut self,
        cfg: &Config,
        written: &[(PathBuf, Vec<u8>)],
        started: u64,
    ) -> anyhow::Result<()> {
        crate::apply_evidence::acknowledged(cfg, written, started)
            .await
            .map(drop)
    }
}

/// The result of one step.
#[derive(Debug, Clone, Serialize)]
pub struct Step {
    pub name: &'static str,
    pub ok: bool,
    pub ms: u64,
    pub detail: Option<String>,
}

/// The report of one live test.
#[derive(Debug, Clone, Default, Serialize)]
pub struct LiveTestReport {
    pub passed: bool,
    /// The first failure. A failed restore is added with "RESTORE FAILED".
    pub error: Option<String>,
    /// The backup of the legacy pair and the LibreQoS state. `live-test-restore` reads it.
    pub backup_dir: Option<PathBuf>,
    pub steps: Vec<Step>,
    /// Nodes that the planner admission refused.
    pub admission: Vec<String>,
    /// Differences between the manager pair and the legacy pair.
    pub parity: Vec<String>,
    /// The legacy state before the test, compared with the legacy model.
    pub baseline: Option<Diff>,
    /// The manager model compared with the legacy state before the test.
    pub legacy_to_manager: Vec<String>,
    /// The manager state, compared with the manager model.
    pub manager: Option<Diff>,
    /// The legacy state after the restore, compared with the legacy model.
    pub restore: Option<Diff>,
    /// True when the restored pair is imported, equals the baseline, and equals the backup bytes.
    pub restored: bool,
    pub warnings: Vec<String>,
}

impl LiveTestReport {
    fn record<T>(
        &mut self,
        name: &'static str,
        started: Instant,
        result: anyhow::Result<T>,
    ) -> anyhow::Result<T> {
        self.steps.push(Step {
            name,
            ok: result.is_ok(),
            ms: started.elapsed().as_millis() as u64,
            detail: result.as_ref().err().map(|e| format!("{e:#}")),
        });
        result
    }

    fn fail(&mut self, error: &anyhow::Error) {
        self.error.get_or_insert_with(|| format!("{error:#}"));
    }

    /// A short text summary.
    pub fn summary(&self) -> String {
        let mut s = format!(
            "live test {}\n",
            if self.passed { "PASSED" } else { "FAILED" }
        );
        if let Some(e) = &self.error {
            s += &format!("error: {e}\n");
        }
        if let Some(d) = &self.backup_dir {
            s += &format!("backup: {}\n", d.display());
        }
        for st in &self.steps {
            let state = if st.ok { "ok" } else { "FAIL" };
            s += &format!("{state} {} ({} ms)", st.name, st.ms);
            if let Some(d) = &st.detail {
                s += &format!(": {d}");
            }
            s.push('\n');
        }
        s
    }
}

/// Write the backup pair (JSON first, then CSV) and check that the files equal the backup.
/// It reads both backup files before it writes, so an incomplete backup changes nothing.
pub fn restore_pair(backup: &Path, lqos_dir: &Path) -> anyhow::Result<()> {
    let bytes = PAIR
        .iter()
        .map(|n| {
            let path = backup.join(n);
            std::fs::read(&path).with_context(|| format!("cannot read {}", path.display()))
        })
        .collect::<anyhow::Result<Vec<_>>>()?;
    for (name, b) in PAIR.iter().zip(&bytes) {
        files::write_if_changed(&lqos_dir.join(name), b)?;
    }
    same_bytes(backup, lqos_dir)
}

fn same_bytes(backup: &Path, lqos_dir: &Path) -> anyhow::Result<()> {
    for name in PAIR {
        ensure!(
            std::fs::read(lqos_dir.join(name))? == std::fs::read(backup.join(name))?,
            "{name} differs from the backup"
        );
    }
    Ok(())
}

/// Copy the legacy pair and the LibreQoS state folders into a new private folder.
fn backup(dir: &Path, lqos_dir: &Path, lqos_state: &Path) -> anyhow::Result<()> {
    let private = |d: &Path, recursive: bool| {
        std::fs::DirBuilder::new()
            .recursive(recursive)
            .mode(0o700)
            .create(d)
            .with_context(|| format!("cannot make {}", d.display()))
    };
    private(dir.parent().context("backup folder has no parent")?, true)?;
    private(dir, false)?;
    for name in PAIR {
        std::fs::copy(lqos_dir.join(name), dir.join(name))
            .with_context(|| format!("cannot back up {name}"))?;
        std::fs::File::open(dir.join(name))?.sync_all()?;
    }
    // Fresh and stale LibreQoS state files. The test reads only the fresh ones.
    for sub in ["topology", "shaping"] {
        let to = dir.join("libreqos-state").join(sub);
        private(&to, true)?;
        let Ok(entries) = std::fs::read_dir(lqos_state.join(sub)) else {
            continue;
        };
        for entry in entries {
            let entry = entry?;
            if entry.file_type()?.is_file() {
                std::fs::copy(entry.path(), to.join(entry.file_name()))?;
            }
        }
    }
    std::fs::File::open(dir)?.sync_all()?;
    Ok(())
}

/// The readiness and the source generation of the LibreQoS runtime.
fn runtime_status(lqos_state: &Path) -> Option<(bool, String)> {
    let path = lqos_state.join("topology/topology_runtime_status.json");
    let v: serde_json::Value = serde_json::from_slice(&std::fs::read(path).ok()?).ok()?;
    Some((v["ready"] == true, v["source_generation"].to_string()))
}

fn source_generation(lqos_state: &Path) -> String {
    runtime_status(lqos_state).map(|s| s.1).unwrap_or_default()
}

/// The pair as the test writes it and as LibreQoS must import it.
struct Pair {
    rows: Vec<CsvRow>,
    network: String,
    csv: String,
}

struct Live<'a> {
    cfg: &'a Config,
    lqos_dir: PathBuf,
    lqos_state: PathBuf,
    timeout: Duration,
}

impl Live<'_> {
    /// Write the pair (JSON first, then CSV) and request the import. Return the start time.
    fn write(&self, io: &impl LiveIo, network: &[u8], csv: &[u8]) -> anyhow::Result<u64> {
        files::write_if_changed(&self.lqos_dir.join(PAIR[0]), network)?;
        files::write_if_changed(&self.lqos_dir.join(PAIR[1]), csv)?;
        self.request(io)
    }

    fn request(&self, io: &impl LiveIo) -> anyhow::Result<u64> {
        let started = io.now();
        files::request_v3_import(&self.lqos_state, started, &super::random_id(24))?;
        Ok(started)
    }

    /// Wait until LibreQoS imported the pair and is ready with a source generation after `before`.
    async fn wait(&self, io: &mut impl LiveIo, pair: &Pair, before: &str) -> anyhow::Result<()> {
        let end = Instant::now() + self.timeout;
        loop {
            if files::imported(&self.lqos_state, &pair.network, &pair.rows)
                && runtime_status(&self.lqos_state).is_some_and(|(ready, g)| ready && g != before)
            {
                return Ok(());
            }
            ensure!(
                Instant::now() < end,
                "import timeout: LibreQoS did not import the pair and become ready in {:?}",
                self.timeout
            );
            io.pause().await;
        }
    }

    /// Steps 5 and 6: the manager write, the import wait, the read-back, and the acknowledgement.
    async fn manager(
        &self,
        io: &mut impl LiveIo,
        report: &mut LiveTestReport,
        pair: &Pair,
        want: &Expected,
    ) -> anyhow::Result<()> {
        let t = Instant::now();
        let before = source_generation(&self.lqos_state);
        let started = report.record(
            "manager write",
            t,
            self.write(io, pair.network.as_bytes(), pair.csv.as_bytes()),
        )?;
        let t = Instant::now();
        let waited = self.wait(io, pair, &before).await;
        report.record("manager import", t, waited)?;
        let t = Instant::now();
        let observed = live_check::read_observed(&self.lqos_state).map(|o| {
            let diff = live_check::compare(want, &o);
            report.warnings.extend(diff.warnings.iter().cloned());
            report.manager = Some(diff.clone());
            diff
        });
        report.record(
            "manager read-back",
            t,
            observed.and_then(|d| {
                ensure!(
                    d.differences.is_empty(),
                    "manager read-back differs from the model: {}",
                    d.differences.join("; ")
                );
                Ok(())
            }),
        )?;
        let t = Instant::now();
        let written: Vec<(PathBuf, Vec<u8>)> = vec![
            (
                self.lqos_dir.join(PAIR[0]),
                pair.network.clone().into_bytes(),
            ),
            (self.lqos_dir.join(PAIR[1]), pair.csv.clone().into_bytes()),
        ];
        let end = Instant::now() + self.timeout;
        let ack = loop {
            match io.acknowledge(self.cfg, &written, started).await {
                Ok(()) => break Ok(()),
                Err(e) if Instant::now() >= end => {
                    break Err(e.context("native acknowledgement unavailable"));
                }
                Err(_) => io.pause().await,
            }
        };
        report.record("acknowledgement", t, ack)
    }

    /// Step 7: write the backup pair, wait for its import, and compare with the baseline.
    async fn restore(
        &self,
        io: &mut impl LiveIo,
        report: &mut LiveTestReport,
        dir: &Path,
        legacy: &Pair,
        want: &Expected,
        baseline: &Observed,
    ) -> anyhow::Result<()> {
        let t = Instant::now();
        let before = source_generation(&self.lqos_state);
        let written = restore_pair(dir, &self.lqos_dir).and_then(|()| self.request(io).map(drop));
        report.record("restore write", t, written)?;
        let t = Instant::now();
        let waited = self.wait(io, legacy, &before).await;
        report.record("restore import", t, waited)?;
        let t = Instant::now();
        let observed = live_check::read_observed(&self.lqos_state).and_then(|o| {
            report.restore = Some(live_check::compare(want, &o));
            ensure!(
                o.ready && o.circuits == baseline.circuits && o.nodes == baseline.nodes,
                "restore read-back differs from the baseline"
            );
            Ok(())
        });
        report.record("restore read-back", t, observed)?;
        let t = Instant::now();
        report.record("byte check", t, same_bytes(dir, &self.lqos_dir))
    }
}

impl<C> Runner<C>
where
    C: Connector + 'static,
    C::Session: 'static,
{
    /// Run the live acceptance test on the production LibreQoS. The service must be stopped.
    pub async fn live_test(&mut self, import_timeout: Duration) -> anyhow::Result<LiveTestReport> {
        self.live_test_with(&mut NativeIo, import_timeout).await
    }

    /// Run the live acceptance test through `io`. An error comes only before any write.
    pub async fn live_test_with(
        &mut self,
        io: &mut impl LiveIo,
        import_timeout: Duration,
    ) -> anyhow::Result<LiveTestReport> {
        // Step 1. Take the instance lock without the write receipt tracker.
        let state = PathBuf::from(&self.cfg.runtime.state_dir);
        let lock = match &self.lock {
            Some(lock) => {
                lock.check(&state)?;
                lock.clone()
            }
            None => files::InstanceLock::acquire(&state)?,
        };
        ensure!(
            self.cfg.runtime.mode == RuntimeMode::Apply
                && crate::detect::resolve(&self.cfg.libreqos)?.target == Target::V3Pair,
            "the live test requires the v3 pair apply target"
        );
        ensure!(
            !crate::topology_manager::runtime::topology_owns(&state)?,
            "the topology manager already owns the writer"
        );
        ensure!(
            state.join("topology-manager/registry.json").exists(),
            "the live test requires an enrolled registry"
        );
        let registry = crate::topology_manager::store::RegistryStore::open_owned(lock.clone())?
            .document()?
            .clone();
        ensure!(
            registry.export_ownership.is_some() && registry.catalog.is_some(),
            "the live test requires an enrolled registry"
        );
        let live = Live {
            cfg: &self.cfg,
            lqos_dir: PathBuf::from(&self.cfg.libreqos.directory),
            lqos_state: PathBuf::from(&self.cfg.libreqos.state_directory),
            timeout: import_timeout,
        };
        // Step 2. Prepare, compile, and check. A failure here writes nothing.
        let mut report = LiveTestReport::default();
        let t = Instant::now();
        let pair = self
            .topology_sources
            .prepare(live.cfg, &registry, self.last_topology_generation, MAX_AGE)
            .await;
        let pair = report.record("prepare", t, pair)?;
        let (legacy_rows, legacy_network) =
            crate::topology_manager::enrollment::read_live_pair(live.cfg)?;
        let parity =
            crate::topology_manager::parity::compare_legacy(&pair, &legacy_rows, &legacy_network)?;
        let manager = Pair {
            rows: pair.rows().to_vec(),
            network: pair.network_text().to_string(),
            csv: pair.csv().to_string(),
        };
        let want = live_check::expected(pair.rows(), pair.network())?;
        let legacy_want = live_check::expected(&legacy_rows, &legacy_network)?;
        let t = Instant::now();
        let admitted = io.admit(live.cfg, pair).await;
        if let Err(e) = report.record("compile", t, admitted) {
            report.fail(&e);
            return Ok(report);
        }
        let t = Instant::now();
        report.admission = live_check::admission(&want);
        report.parity = parity.iter().map(|i| format!("{i:?}")).collect();
        let gate = if !report.admission.is_empty() {
            Err(anyhow::anyhow!(
                "planner admission refused the manager pair"
            ))
        } else if !report.parity.is_empty() {
            Err(anyhow::anyhow!(
                "the manager pair differs from the legacy pair"
            ))
        } else {
            Ok(())
        };
        if let Err(e) = report.record("admission and parity", t, gate) {
            report.fail(&e);
            return Ok(report);
        }
        // Steps 3 and 4. Back up, then read the legacy state. A failure here writes no pair.
        let t = Instant::now();
        let dir = state
            .join("topology-manager/live-test")
            .join(io.now().to_string());
        let saved = backup(&dir, &live.lqos_dir, &live.lqos_state).and_then(|()| {
            let text = |n: &str| {
                String::from_utf8(std::fs::read(dir.join(n))?).context("backup is not UTF-8")
            };
            Ok(Pair {
                rows: legacy_rows,
                network: text(PAIR[0])?,
                csv: text(PAIR[1])?,
            })
        });
        let legacy = match report.record("backup", t, saved) {
            Ok(legacy) => legacy,
            Err(e) => {
                report.fail(&e);
                return Ok(report);
            }
        };
        report.backup_dir = Some(dir.clone());
        let t = Instant::now();
        let baseline = live_check::read_observed(&live.lqos_state).and_then(|o| {
            ensure!(o.ready, "LibreQoS is not ready before the test");
            Ok(o)
        });
        let baseline = match report.record("baseline read-back", t, baseline) {
            Ok(o) => o,
            Err(e) => {
                report.fail(&e);
                return Ok(report);
            }
        };
        report.baseline = Some(live_check::compare(&legacy_want, &baseline));
        report.legacy_to_manager = live_check::compare(&want, &baseline).differences;
        // Steps 5 to 7. Every result of the manager steps goes to the restore.
        if let Err(e) = live.manager(io, &mut report, &manager, &want).await {
            report.fail(&e);
        }
        let restored = live
            .restore(io, &mut report, &dir, &legacy, &legacy_want, &baseline)
            .await;
        report.restored = restored.is_ok();
        if let Err(e) = restored {
            let loud = format!(
                "RESTORE FAILED: {e:#}; the manager pair can still be applied; run `lqosync3 topology live-test-restore --backup {}`",
                dir.display()
            );
            report.error = Some(match report.error.take() {
                Some(first) => format!("{first}; {loud}"),
                None => loud,
            });
        }
        // Step 8.
        report.passed = report.error.is_none();
        drop(lock);
        Ok(report)
    }
}
