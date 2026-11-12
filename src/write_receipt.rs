//! Durable operation identity and per-file outcomes. Notification faults never roll back shaping.
use crate::system_health::{Domain, Hub, Observation, Phase, State, Update};
use anyhow::{Context, ensure};
use serde::{Deserialize, Serialize};
use std::{
    io::Read,
    path::{Path, PathBuf},
};
const MAX_RECEIPT: u64 = 256 * 1024;
pub const MAX_EVENTS: usize = 256;
pub const MAX_EVENT_BYTES: u64 = 8 * 1024 * 1024;
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum FileOutcome {
    Pending,
    Written,
    Unchanged,
    Reconciled,
    Failed,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileReceipt {
    pub path: PathBuf,
    pub sha256: String,
    pub outcome: FileOutcome,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Meta {
    pub cycle_seq: u64,
    pub config_digest: String,
    #[serde(default)]
    pub notice: Option<crate::notify::outbox::Frozen>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Receipt {
    pub version: u32,
    pub write_id: String,
    pub phase: Phase,
    pub started: u64,
    pub updated: u64,
    pub meta: Meta,
    pub files: Vec<FileReceipt>,
    pub error: Option<String>,
    #[serde(default)]
    pub acceptance: Option<crate::apply_evidence::Acceptance>,
    #[serde(default)]
    pub superseded_by: Option<String>,
}
impl Receipt {
    pub fn update(&self) -> Update {
        Update {
            write_id: self.write_id.clone(),
            phase: self.phase,
            observed_at: self.updated,
        }
    }
    fn valid(&self) -> bool {
        self.version == 1
            && self.write_id.len() == 24
            && self
                .write_id
                .bytes()
                .all(|b| b.is_ascii_uppercase() || b.is_ascii_digit())
            && !self.files.is_empty()
            && self.files.len() <= 8
            && self.files.iter().all(|f| {
                f.sha256.len() == 64
                    && f.sha256.bytes().all(|b| b.is_ascii_hexdigit())
                    && f.path.as_os_str().len() <= 4096
            })
            && self.meta.config_digest.len() <= 128
            && self.error.as_ref().is_none_or(|s| s.len() <= 512)
    }
}
#[derive(Debug, Serialize, Deserialize)]
pub struct Event {
    pub version: u32,
    pub captured_at: u64,
    pub id: String,
    pub receipt: Receipt,
    pub health: crate::system_health::Report,
}
#[derive(Debug, Default)]
pub struct WriteResult {
    pub changed: bool,
    pub error: Option<String>,
    pub tracking_error: Option<String>,
}
pub struct Tracker {
    dir: PathBuf,
    current: Option<Receipt>,
}

/// Reuse the existing crypto implementation; no new hashing dependency or subprocess.
pub fn sha256(bytes: &[u8]) -> String {
    use std::fmt::Write;
    let suite = rustls::crypto::ring::cipher_suite::TLS13_AES_128_GCM_SHA256
        .tls13()
        .expect("TLS13 suite");
    let digest = suite.common.hash_provider.hash(bytes);
    let mut out = String::with_capacity(64);
    for b in digest.as_ref() {
        write!(out, "{b:02x}").expect("String write");
    }
    out
}
fn bounded_error(error: impl std::fmt::Display) -> String {
    error.to_string().chars().take(120).collect()
}
impl Tracker {
    pub fn load(dir: &Path) -> anyhow::Result<Self> {
        let path = dir.join("write-receipt.json");
        let current = match std::fs::File::open(&path) {
            Ok(file) => {
                let mut bytes = Vec::new();
                file.take(MAX_RECEIPT + 1).read_to_end(&mut bytes)?;
                ensure!(bytes.len() as u64 <= MAX_RECEIPT, "write receipt too large");
                let mut r: Receipt =
                    serde_json::from_slice(&bytes).context("invalid write receipt")?;
                ensure!(r.valid(), "invalid write receipt fields");
                if r.phase == Phase::Intent {
                    r.phase = Phase::Interrupted;
                }
                Some(r)
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => None,
            Err(e) => return Err(e.into()),
        };
        Ok(Self {
            dir: dir.into(),
            current,
        })
    }
    fn files_match(&self) -> bool {
        self.current.as_ref().is_some_and(|receipt| {
            receipt.files.iter().all(|file| {
                let read = || -> anyhow::Result<bool> {
                    let mut bytes = Vec::new();
                    std::fs::File::open(&file.path)?
                        .take(64 * 1024 * 1024 + 1)
                        .read_to_end(&mut bytes)?;
                    Ok(bytes.len() <= 64 * 1024 * 1024 && sha256(&bytes) == file.sha256)
                };
                read().unwrap_or(false)
            })
        })
    }
    pub fn reconcile(&mut self, now: u64, hub: &Hub) -> anyhow::Result<bool> {
        if !self
            .current
            .as_ref()
            .is_some_and(|r| matches!(r.phase, Phase::Intent | Phase::Interrupted))
            || !self.files_match()
        {
            return Ok(false);
        }
        for file in &mut self.current.as_mut().expect("interrupted receipt").files {
            if file.outcome == FileOutcome::Pending {
                file.outcome = FileOutcome::Reconciled;
            }
        }
        self.change_phase(Phase::Pending, now, hub)?;
        Ok(true)
    }
    pub fn restore_health(&self, hub: &Hub) {
        if self.dir.join("write-event-gap.json").exists() {
            self.notification_fault(hub);
        }
        if let Some(receipt) = &self.current {
            if self.files_match() && receipt.phase == Phase::Applied && receipt.acceptance.is_some()
            {
                self.publish(hub, receipt.updated);
            } else {
                let _ = hub.set_update(Some(receipt.update()));
                let _ = hub.observe(Observation {
                    domain: Domain::Write,
                    component: "managed_files".into(),
                    state: State::Unknown,
                    reason: "restart_reconciliation_required".into(),
                    observed_at: receipt.updated,
                    max_age: crate::runner::IMPORT_LIMIT_SECS,
                    source_epoch: receipt.write_id.clone(),
                    last_success: None,
                });
            }
        }
    }
    /// Replay the current durable operation after a crash between its receipt and event writes.
    pub fn recover_events(&self, hub: &Hub) -> anyhow::Result<()> {
        let Some(receipt) = &self.current else {
            return Ok(());
        };
        if matches!(receipt.phase, Phase::Intent | Phase::Interrupted) {
            return Ok(());
        }
        if receipt.phase != Phase::WriteFailed {
            let mut pending = receipt.clone();
            pending.phase = Phase::Pending;
            self.event_receipt(&pending, hub)?;
        }
        if !matches!(receipt.phase, Phase::Pending | Phase::ImportConfirmed) {
            self.event_receipt(receipt, hub)?;
        }
        Ok(())
    }
    fn notification_fault(&self, hub: &Hub) {
        let _ = hub.observe(Observation {
            domain: Domain::Notification,
            component: "write_event_store".into(),
            state: State::Degraded,
            reason: "write_event_storage_gap".into(),
            observed_at: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
            max_age: u64::MAX,
            source_epoch: "service".into(),
            last_success: None,
        });
        let marker = serde_json::json!({"version":1,"write_id":self.current.as_ref().map(|r|&r.write_id),"reason":"write_event_storage_gap"});
        let _ = crate::files::write_secret(
            &self.dir.join("write-event-gap.json"),
            &serde_json::to_vec(&marker).unwrap_or_default(),
        );
    }
    pub fn current(&self) -> Option<&Receipt> {
        self.current.as_ref()
    }
    fn save(&self) -> anyhow::Result<()> {
        let Some(receipt) = &self.current else {
            return Ok(());
        };
        let bytes = serde_json::to_vec(receipt)?;
        ensure!(bytes.len() as u64 <= MAX_RECEIPT, "write receipt too large");
        crate::files::write_secret(&self.dir.join("write-receipt.json"), &bytes)
    }
    pub fn accept(
        &mut self,
        proof: &crate::apply_evidence::Verified,
        now: u64,
        hub: &Hub,
    ) -> anyhow::Result<()> {
        let receipt = self.current.as_mut().context("no pending write")?;
        ensure!(
            receipt.phase.pending() && proof.matches(receipt),
            "acceptance operation mismatch"
        );
        receipt.acceptance = Some(proof.acceptance().clone());
        self.change_phase(Phase::Applied, now, hub)
    }
    pub fn transition(&mut self, phase: Phase, now: u64, hub: &Hub) -> anyhow::Result<()> {
        ensure!(
            phase != Phase::Applied,
            "native acceptance requires verified evidence"
        );
        self.change_phase(phase, now, hub)
    }
    fn change_phase(&mut self, phase: Phase, now: u64, hub: &Hub) -> anyhow::Result<()> {
        let Some(receipt) = &mut self.current else {
            return Ok(());
        };
        if receipt.phase == phase {
            return Ok(());
        }
        if phase == Phase::ImportConfirmed {
            ensure!(
                matches!(receipt.phase, Phase::Pending | Phase::ImportConfirmed)
                    && receipt.files.iter().all(|f| matches!(
                        f.outcome,
                        FileOutcome::Written | FileOutcome::Unchanged | FileOutcome::Reconciled
                    )),
                "write reconciliation required before import confirmation"
            );
        }
        ensure!(
            receipt.phase.pending(),
            "cannot change a terminal write receipt"
        );
        receipt.phase = phase;
        receipt.updated = now;
        self.publish(hub, now);
        let result = self.save().and_then(|_| {
            if !matches!(phase, Phase::Intent | Phase::ImportConfirmed) {
                self.event(hub, now)
            } else {
                Ok(())
            }
        });
        if let Err(error) = result {
            self.notification_fault(hub);
            return Err(error);
        }

        Ok(())
    }
    fn publish(&self, hub: &Hub, now: u64) {
        let Some(receipt) = &self.current else { return };
        let update = receipt.update();
        if let Err(e) = hub.set_update(Some(update)) {
            eprintln!("health update: {e}");
        }
        hub.set_acceptance(receipt.acceptance.clone());
        if receipt.phase == Phase::Applied {
            hub.remove(Domain::Apply, "scheduler_request");
        }
        let state = if receipt.phase == Phase::WriteFailed {
            State::Failed
        } else {
            State::Healthy
        };
        let _ = hub.observe(Observation {
            domain: Domain::Write,
            component: "managed_files".into(),
            state,
            reason: if state == State::Failed {
                "partial_or_failed_write"
            } else {
                "files_written"
            }
            .into(),
            observed_at: now,
            max_age: if receipt.phase == Phase::Applied {
                u64::MAX
            } else {
                crate::runner::IMPORT_LIMIT_SECS
            },
            source_epoch: receipt.write_id.clone(),
            last_success: None,
        });
        if let Some(proof) = &receipt.acceptance {
            let (state, reason) = match proof.circuit_capacity {
                Some(actual) if actual == proof.desired_circuit_capacity => {
                    (State::Healthy, "native_capacity_matches")
                }
                Some(_) => (State::Degraded, "native_capacity_differs_from_generated"),
                None => (State::Unknown, "native_capacity_not_reported"),
            };
            let _ = hub.observe(Observation {
                domain: Domain::Apply,
                component: "allocation_capacity".into(),
                state,
                reason: reason.into(),
                observed_at: now,
                max_age: u64::MAX,
                source_epoch: receipt.write_id.clone(),
                last_success: None,
            });
        } else {
            hub.remove(Domain::Apply, "allocation_capacity");
        }
        let state = match receipt.phase {
            Phase::ApplyFailed | Phase::VerifyTimeout => State::Degraded,
            Phase::Intent | Phase::Pending => State::Updating,
            Phase::Applied if receipt.acceptance.is_some() => State::Healthy,
            Phase::Candidate | Phase::Superseded => State::NotApplicable,
            _ => State::Unknown,
        };
        let _ = hub.observe(Observation {
            domain: Domain::Apply,
            component: "latest_update".into(),
            state,
            reason: if receipt.phase == Phase::Applied && receipt.acceptance.is_some() {
                "matching_native_ack"
            } else if receipt.phase == Phase::ImportConfirmed {
                "import_confirmed_ack_unproven"
            } else {
                "acceptance_unproven"
            }
            .into(),
            observed_at: now,
            max_age: if receipt.phase == Phase::Applied {
                u64::MAX
            } else {
                crate::runner::IMPORT_LIMIT_SECS
            },
            source_epoch: receipt.write_id.clone(),
            last_success: None,
        });
    }
    fn event(&self, hub: &Hub, _now: u64) -> anyhow::Result<()> {
        let receipt = self.current.as_ref().context("no write receipt")?;
        self.event_receipt(receipt, hub)
    }
    fn event_receipt(&self, receipt: &Receipt, hub: &Hub) -> anyhow::Result<()> {
        use std::os::unix::fs::{DirBuilderExt, PermissionsExt};
        let suffix = serde_json::to_value(receipt.phase)?
            .as_str()
            .context("invalid phase")?
            .to_string();
        let id = format!("{}-{suffix}", receipt.write_id);
        let delivered = self
            .dir
            .join("telegram-delivery")
            .join(format!("{id}.json"));
        if delivered.exists() {
            let mut bytes = Vec::new();
            std::fs::File::open(delivered)?
                .take(MAX_RECEIPT + 1)
                .read_to_end(&mut bytes)?;
            ensure!(
                bytes.len() as u64 <= MAX_RECEIPT,
                "delivery receipt exceeds bound"
            );
            let v: serde_json::Value = serde_json::from_slice(&bytes)?;
            if v["completed"] == true {
                return Ok(());
            }
        }
        let dir = self.dir.join("write-events");
        std::fs::DirBuilder::new()
            .recursive(true)
            .mode(0o700)
            .create(&dir)?;
        std::fs::set_permissions(&dir, std::fs::Permissions::from_mode(0o700))?;
        let path = dir.join(format!("{id}.json"));
        if path.exists() {
            return Ok(());
        }
        let captured_at = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        let event = Event {
            version: 1,
            captured_at,
            id,
            receipt: receipt.clone(),
            health: hub.snapshot(captured_at, Some(receipt.update())),
        };
        let bytes = serde_json::to_vec(&event)?;
        ensure!(
            bytes.len() as u64 <= MAX_RECEIPT,
            "write event exceeds its record budget"
        );
        let mut count = 0;
        let mut size = 0u64;
        for entry in std::fs::read_dir(&dir)? {
            let entry = entry?;
            if entry.path().extension().is_some_and(|x| x == "json") {
                count += 1;
                size = size.saturating_add(entry.metadata()?.len());
            }
        }
        ensure!(
            count < MAX_EVENTS && size.saturating_add(bytes.len() as u64) <= MAX_EVENT_BYTES,
            "write event capacity exceeded"
        );
        crate::files::write_secret(&path, &bytes)
    }
    pub fn write(
        &mut self,
        outputs: &[(PathBuf, String)],
        meta: Meta,
        now: u64,
        hub: &Hub,
    ) -> WriteResult {
        let differs: Vec<_> = outputs
            .iter()
            .map(|(p, t)| !std::fs::read(p).is_ok_and(|old| old == t.as_bytes()))
            .collect();
        if !differs.iter().any(|x| *x) {
            return WriteResult::default();
        }
        let mut result = WriteResult::default();
        let write_id = crate::runner::random_id(24);
        if self.current.as_ref().is_some_and(|r| r.phase.pending()) {
            self.current
                .as_mut()
                .expect("pending receipt")
                .superseded_by = Some(write_id.clone());
            if let Err(e) = self.transition(Phase::Superseded, now, hub) {
                result.tracking_error = Some(bounded_error(e));
            }
        }
        self.current = Some(Receipt {
            version: 1,
            write_id,
            phase: Phase::Intent,
            started: now,
            updated: now,
            meta,
            files: outputs
                .iter()
                .zip(&differs)
                .map(|((path, text), different)| FileReceipt {
                    path: path.clone(),
                    sha256: sha256(text.as_bytes()),
                    outcome: if *different {
                        FileOutcome::Pending
                    } else {
                        FileOutcome::Unchanged
                    },
                })
                .collect(),
            error: None,
            acceptance: None,
            superseded_by: None,
        });
        if let Err(e) = self.save() {
            result.tracking_error = Some(bounded_error(e));
        }
        for (index, (path, text)) in outputs.iter().enumerate() {
            let outcome = crate::files::write_if_changed(path, text.as_bytes());
            let receipt = self.current.as_mut().expect("write intent");
            match outcome {
                Ok(changed) => {
                    result.changed |= changed;
                    receipt.files[index].outcome = if changed {
                        FileOutcome::Written
                    } else {
                        FileOutcome::Unchanged
                    };
                }
                Err(error) => {
                    receipt.files[index].outcome = FileOutcome::Failed;
                    receipt.error = Some(bounded_error(&error));
                    result.error = Some(format!("{error:#}"));
                }
            }
            if let Err(e) = self.save() {
                result.tracking_error = Some(bounded_error(e));
            }
            if result.error.is_some() {
                break;
            }
        }
        let phase = if result.error.is_some() {
            Phase::WriteFailed
        } else {
            Phase::Pending
        };
        if let Err(e) = self.transition(phase, now, hub) {
            result.tracking_error = Some(bounded_error(e));
        }
        if result.tracking_error.is_some() {
            let _ = hub.observe(Observation {
                domain: Domain::Notification,
                component: "write_event_store".into(),
                state: State::Degraded,
                reason: "write_receipt_storage_failed".into(),
                observed_at: now,
                max_age: crate::runner::IMPORT_LIMIT_SECS,
                source_epoch: "service".into(),
                last_success: None,
            });
        }
        result
    }
}
