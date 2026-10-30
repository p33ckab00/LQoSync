//! Explicit library-only recovery. No automatic, HTTP, scheduler, or CLI caller exists.
use super::*;
use crate::{core_recovery, core_writer, migration_fence::Lease};
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub enum RecoveryAction {
    Forward,
    Rollback,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Manifest {
    schema: u32,
    revision: String,
    guard: Guard,
}
/// Concrete review data. This report alone cannot authorize execution.
#[derive(Debug, Serialize)]
pub struct RecoveryReview {
    pub approval: String,
    pub action: RecoveryAction,
    pub manifest_fingerprint: String,
    pub assessment: core_writer::RecoveryAssessment,
}
/// Holds an exclusive read-only review lease until exact approval or cancellation.
pub struct Recovery<S> {
    root: PathBuf,
    batch_revision: String,
    action: RecoveryAction,
    journal: String,
    manifest: String,
    source: S,
    guard: Guard,
    settings: Settings,
    context: String,
    deadline: u64,
    proposal: RecoveryReview,
    lease: Lease,
    cancellation: crate::core_coordination::WriterGuard,
}
impl<S: Source> Recovery<S> {
    pub async fn review(
        root: &Path,
        revision: &str,
        action: RecoveryAction,
        mut source: S,
        seconds: u64,
    ) -> Result<Self> {
        ensure!(
            (1..=30).contains(&seconds),
            "Recovery review lifetime must be 1–30 seconds"
        );
        let root = root.canonicalize()?;
        ensure!(
            !root.starts_with("/opt/libreqos") && !root.starts_with("/etc/libreqos"),
            "Use separate application data"
        );
        let work_root = root.clone();
        let id = revision.to_string();
        let (lease, journal, manifest, batch, original) =
            tokio::task::spawn_blocking(move || -> Result<_> {
                let (lease, journal, manifest) = Lease::recovery(&work_root, &id)?;
                let manifest =
                    manifest.context("Missing persisted worker guard; recovery is blocked")?;
                let batch = core_writer::recovery_batch(&journal, &id)?;
                let saved: Manifest = serde_json::from_str(&manifest)?;
                ensure!(
                    saved.schema == 1 && saved.revision == id,
                    "Unsupported worker guard manifest"
                );
                let original = saved.guard;
                ensure!(
                    original.root == work_root && digest(&original)? == batch.context(),
                    "Persisted worker guard identity changed"
                );
                core_console::persisted_policy(&work_root, &original.revision)?;
                Ok((lease, journal, manifest, batch, original))
            })
            .await??;
        let cancellation =
            crate::core_coordination::WriterGuard::recovery(&root, &original.revision)?;
        let snapshot = source.capture().await?;
        let settings = snapshot.settings.clone();
        let now = crate::orchestrator::now();
        let (guard, rows, until) = tokio::task::spawn_blocking(move || -> Result<_> {
            let ttl = snapshot
                .settings
                .core_control
                .as_ref()
                .context("Missing policy")?
                .options
                .evidence_seconds
                .min(30);
            let (next, rows, until) = Guard::capture(&snapshot, now)?;
            original.accepts(&next, ttl)?;
            Ok((next, rows, until))
        })
        .await??;
        source.confirm(&guard.revision).await?;
        let work_root = root.clone();
        let id = revision.to_string();
        let check = cancellation.clone();
        let (journal, manifest, context, deadline, proposal, guard) =
            tokio::task::spawn_blocking(move || -> Result<_> {
                core_console::persisted_policy(&work_root, &guard.revision)?;
                let deadline = until.min(now.saturating_add(seconds)).min(
                    if action == RecoveryAction::Forward {
                        batch.deadline()
                    } else {
                        u64::MAX
                    },
                );
                let evidence = Evidence {
                    context: batch.context().into(),
                    captured_at: now,
                    valid_until: deadline,
                    rows,
                };
                let assessment = core_writer::assess_recovery(
                    &journal,
                    &id,
                    evidence,
                    crate::orchestrator::now(),
                )?;
                let summary = core_writer::inspect_journal(&journal, &id)?;
                ensure!(
                    action != RecoveryAction::Forward
                        || summary.direction == "Forward"
                            && crate::orchestrator::now() <= batch.deadline(),
                    "Cannot resume expired forward batch or a rollback journal"
                );
                ensure!(
                    check.active() && crate::orchestrator::now() <= deadline,
                    "Recovery review paused or expired"
                );
                let context = batch.context().to_string();
                let manifest_fingerprint = digest(&manifest)?;
                let scope = digest(&(
                    &id,
                    action,
                    digest(&journal)?,
                    &manifest_fingerprint,
                    digest(&guard)?,
                    deadline,
                    uuid::Uuid::new_v4().to_string(),
                ))?;
                let proposal = RecoveryReview {
                    approval: scope,
                    action,
                    manifest_fingerprint,
                    assessment,
                };
                Ok((journal, manifest, context, deadline, proposal, guard))
            })
            .await??;
        Ok(Self {
            root,
            batch_revision: revision.into(),
            action,
            journal,
            manifest,
            source,
            guard,
            settings,
            context,
            deadline,
            proposal,
            lease,
            cancellation,
        })
    }
    pub fn revision(&self) -> Result<String> {
        Ok(self.proposal.approval.clone())
    }
    pub fn proposal(&self) -> &RecoveryReview {
        &self.proposal
    }
    /// Exact new approval is necessary. No original approval is reconstructed automatically.
    pub async fn approve(self, approved: &str) -> Result<Worker<S>> {
        ensure!(
            approved == self.proposal.approval,
            "Exact recovery review approval required"
        );
        ensure!(
            self.cancellation.active() && crate::orchestrator::now() <= self.deadline,
            "Recovery review revoked or expired"
        );
        let approval = Approval {
            enabled: Arc::new(AtomicBool::new(true)),
            expires_at: self.deadline,
            coordination: Some(self.cancellation.clone()),
        };
        let settings_revision = self.guard.revision.clone();
        let mut verifier = GuardedVerifier {
            context: self.context,
            last: self.guard,
            approval: approval.clone(),
            source: self.source,
        };
        let evidence = verifier.verify().await?;
        let io = RouterTransport::new(&self.root, self.settings, verifier)?;
        let root = self.root;
        let id = self.batch_revision;
        let executor = tokio::task::spawn_blocking(move || -> Result<_> {
            let (journal, manifest) = core_recovery::worker_artifacts(&root, &id)?;
            ensure!(
                journal == self.journal && manifest.as_deref() == Some(&self.manifest),
                "Recovery artifacts changed after review"
            );
            core_writer::assess_recovery(&journal, &id, evidence, crate::orchestrator::now())?;
            core_console::persisted_policy(&root, &settings_revision)?;
            ensure!(
                self.cancellation.active() && crate::orchestrator::now() <= self.deadline,
                "Recovery approval revoked or expired"
            );
            let executor = Executor::resume_reviewed(
                &root,
                &journal,
                &id,
                self.lease,
                self.cancellation,
                digest(&self.manifest)?,
            )?;
            Ok(executor)
        })
        .await??;
        ensure!(
            approval.active(),
            "Recovery approval expired during journal save"
        );
        Ok(Worker {
            executor,
            io,
            approval,
            recovery_action: Some(self.action),
        })
    }
}
