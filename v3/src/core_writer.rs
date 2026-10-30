//! Bounded owned-host writer. No CLI, scheduler, or web endpoint enables it.
//! The transport verifier must prove current settings, source sessions, health and write authorization.
use crate::{
    core_migration::{Member, RequestNotSent},
    core_plan::{self, Binding},
    migration_fence::Lease,
    store,
};
use anyhow::{Context, Result, ensure};
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, BTreeSet},
    future::Future,
    path::{Path, PathBuf},
};
pub type Rows = BTreeMap<String, Vec<Member>>;
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum Operation {
    Add {
        router: String,
        member: Member,
    },
    Remove {
        router: String,
        before: Member,
    },
    Move {
        router: String,
        before: Member,
        after: Member,
    },
}
impl Operation {
    pub fn router(&self) -> &str {
        match self {
            Self::Add { router, .. } | Self::Remove { router, .. } | Self::Move { router, .. } => {
                router
            }
        }
    }
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Evidence {
    pub context: String,
    pub captured_at: u64,
    pub valid_until: u64,
    pub rows: Rows,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Batch {
    owner: String,
    bindings: Vec<Binding>,
    context: String,
    expires_at: u64,
    created_at: u64,
    baseline: Rows,
    operations: Vec<Operation>,
}
fn digest(v: &impl Serialize) -> Result<String> {
    Ok(uuid::Uuid::new_v5(&uuid::Uuid::NAMESPACE_OID, &serde_json::to_vec(v)?).to_string())
}
fn id_valid(id: &str) -> bool {
    id.strip_prefix('*')
        .is_some_and(|v| !v.is_empty() && v.len() <= 32 && v.bytes().all(|b| b.is_ascii_hexdigit()))
}
fn normalize(rows: &mut Rows) -> Result<()> {
    for members in rows.values_mut() {
        members.sort_by(|a, b| a.id.cmp(&b.id));
        let mut ids = BTreeSet::new();
        for m in members {
            ensure!(
                id_valid(&m.id) && ids.insert(m.id.clone()),
                "Invalid or duplicate observed row ID"
            );
        }
    }
    Ok(())
}
impl Batch {
    pub fn new(
        owner: String,
        bindings: Vec<Binding>,
        context: String,
        expires_at: u64,
        mut baseline: Rows,
        operations: Vec<Operation>,
    ) -> Result<Self> {
        normalize(&mut baseline)?;
        let b = Self {
            owner,
            bindings,
            context,
            expires_at,
            created_at: crate::orchestrator::now(),
            baseline,
            operations,
        };
        b.validate()?;
        Ok(b)
    }
    pub fn revision(&self) -> Result<String> {
        digest(self)
    }
    pub fn baseline(&self) -> &Rows {
        &self.baseline
    }
    fn member(&self, router: &str, m: &Member, added: bool) -> Result<()> {
        ensure!(
            if added {
                m.id.is_empty()
            } else {
                id_valid(&m.id)
            },
            "Invalid owned operation ID"
        );
        ensure!(
            !m.dynamic
                && !m.disabled
                && m.timeout.as_ref().is_none_or(|s| s.is_empty() || s == "0s"),
            "Only enabled permanent static hosts are writable"
        );
        core_plan::host(&m.address)?;
        let (subscriber, wan) = core_plan::owned(&m.comment, &self.owner)?
            .context("Operation does not own this row")?;
        ensure!(
            self.bindings
                .iter()
                .any(|b| b.router == router && b.list == m.list && b.wan == wan),
            "Operation is outside configured ownership scope"
        );
        ensure!(
            !uuid::Uuid::parse_str(subscriber)?.is_nil(),
            "Nil subscriber identity"
        );
        Ok(())
    }
    fn validate(&self) -> Result<()> {
        ensure!(
            !uuid::Uuid::parse_str(&self.owner)?.is_nil() && !self.context.is_empty(),
            "Invalid owner or verification context"
        );
        ensure!(
            (1..=20).contains(&self.operations.len()),
            "A writer batch must contain 1–20 mutations"
        );
        ensure!(
            self.expires_at > self.created_at && self.expires_at - self.created_at <= 300,
            "Writer approval lifetime must be at most 300 seconds"
        );
        let mut lists = BTreeSet::new();
        let mut wans = BTreeSet::new();
        for b in &self.bindings {
            ensure!(
                core_plan::valid_name(&b.list)
                    && core_plan::valid_name(&b.table)
                    && b.table != "main"
                    && crate::model::valid_id(&b.router)
                    && crate::model::valid_id(&b.wan)
                    && lists.insert((&b.router, &b.list))
                    && wans.insert(&b.wan),
                "Invalid or duplicate binding"
            );
        }
        let mut rows = self.baseline.clone();
        normalize(&mut rows)?;
        ensure!(rows == self.baseline, "Noncanonical baseline order");
        let required: BTreeSet<_> = self.bindings.iter().map(|b| b.router.as_str()).collect();
        ensure!(
            required.iter().all(|r| self.baseline.contains_key(*r)),
            "Every configured binding core requires a complete baseline"
        );
        let mut identities = BTreeSet::new();
        let mut host_ips = BTreeSet::new();
        for (router, members) in &self.baseline {
            for m in members {
                if !self
                    .bindings
                    .iter()
                    .any(|b| b.router == *router && b.list == m.list)
                {
                    continue;
                }
                if let Some((subscriber, _)) = core_plan::owned(&m.comment, &self.owner)? {
                    self.member(router, m, false)?;
                    ensure!(
                        identities.insert(subscriber.to_string())
                            && host_ips.insert(core_plan::host(&m.address)?),
                        "Duplicate owned identity or address in baseline"
                    );
                }
            }
        }
        let mut changed = BTreeSet::new();
        for op in &self.operations {
            ensure!(rows.contains_key(op.router()), "Missing baseline router");
            let m = match op {
                Operation::Add { member, .. } => {
                    self.member(op.router(), member, true)?;
                    member
                }
                Operation::Remove { before, .. } => {
                    self.member(op.router(), before, false)?;
                    before
                }
                Operation::Move { before, after, .. } => {
                    self.member(op.router(), before, false)?;
                    self.member(op.router(), after, false)?;
                    ensure!(
                        before.id == after.id
                            && before.address == after.address
                            && before.dynamic == after.dynamic
                            && before.disabled == after.disabled
                            && before.timeout == after.timeout
                            && before.comment.split(':').nth(2) == after.comment.split(':').nth(2)
                            && before.list != after.list,
                        "Move may change only list and WAN ownership comment"
                    );
                    before
                }
            };
            let subscriber = m
                .comment
                .split(':')
                .nth(2)
                .context("Missing subscriber marker")?;
            ensure!(
                changed.insert(subscriber.to_string()),
                "Only one mutation per subscriber is allowed in a batch"
            );
            match op {
                Operation::Add { router, member } => {
                    self.no_overlap(&rows, router, member, None)?;
                    let mut m = member.clone();
                    m.id = format!("*{:X}", u64::MAX - changed.len() as u64);
                    ensure!(
                        !rows[router].iter().any(|r| r.id == m.id),
                        "Synthetic validation ID collision"
                    );
                    rows.get_mut(router).unwrap().push(m);
                }
                Operation::Remove { router, before } => {
                    let r = rows.get_mut(router).unwrap();
                    let pos = r
                        .iter()
                        .position(|v| v == before)
                        .context("Removal precondition is absent")?;
                    r.remove(pos);
                }
                Operation::Move {
                    router,
                    before,
                    after,
                } => {
                    self.no_overlap(&rows, router, after, Some(&before.id))?;
                    let r = rows
                        .get_mut(router)
                        .unwrap()
                        .iter_mut()
                        .find(|v| *v == before)
                        .context("Move precondition is absent")?;
                    *r = after.clone();
                }
            }
        }
        Ok(())
    }
    fn no_overlap(
        &self,
        rows: &Rows,
        router: &str,
        m: &Member,
        except: Option<&str>,
    ) -> Result<()> {
        let ip = core_plan::host(&m.address)?;
        let subscriber = m.comment.split(':').nth(2).context("Invalid marker")?;
        for (r, members) in rows {
            for old in members {
                if r == router && except == Some(old.id.as_str()) {
                    continue;
                }
                if !self
                    .bindings
                    .iter()
                    .any(|b| b.router == *r && b.list == old.list)
                {
                    continue;
                }
                if old.disabled {
                    continue;
                }
                let (a, z) =
                    core_plan::span(&old.address).context("Unresolved protected coverage")?;
                ensure!(
                    !(a <= ip && ip <= z),
                    "Existing membership overlaps a proposed host"
                );
                ensure!(
                    !core_plan::owned(&old.comment, &self.owner)?
                        .is_some_and(|(s, _)| s == subscriber),
                    "Subscriber already owns another row"
                );
            }
        }
        Ok(())
    }
    /// Compile only a completely unblocked planner result; a future integration binds context to fresh source/health/settings evidence.
    pub fn from_reconcile(
        settings: &crate::model::Settings,
        plan: &crate::core_reconcile::ReconcilePlan,
        context: String,
        expires_at: u64,
        baseline: Rows,
    ) -> Result<Self> {
        ensure!(
            plan.settings_revision == crate::wan_health::settings_revision(settings)?,
            "Reconciliation settings changed"
        );
        let mut operations = vec![];
        for d in &plan.decisions {
            use crate::core_reconcile::Decision;
            match d {
                Decision::Add { router, member, .. } => operations.push(Operation::Add {
                    router: router.clone(),
                    member: member.clone(),
                }),
                Decision::Remove { router, member, .. } => operations.push(Operation::Remove {
                    router: router.clone(),
                    before: member.clone(),
                }),
                Decision::Move {
                    router,
                    before,
                    after,
                    ..
                } => operations.push(Operation::Move {
                    router: router.clone(),
                    before: before.clone(),
                    after: after.clone(),
                }),
                Decision::Keep { .. } => {}
                Decision::Blocked { .. } => {
                    anyhow::bail!("Blocked reconciliation cannot become an executable batch")
                }
            }
        }
        Self::new(
            settings.core_lists.owner.clone(),
            settings.core_lists.bindings.clone(),
            context,
            expires_at,
            baseline,
            operations,
        )
    }
}
/// All observations must bind fresh settings, sources, health and enable authorization to context.
/// execute must repeat final checks after connection/awaits and before sending; only pre-send failures carry RequestNotSent.
pub trait Transport {
    fn observe(&mut self) -> impl Future<Output = Result<Evidence>>;
    fn execute(
        &mut self,
        operation: &Operation,
        guard: &Evidence,
    ) -> impl Future<Output = Result<Option<String>>>;
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Receipt {
    operation: Operation,
    returned_id: Option<String>,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
enum Mode {
    Forward,
    Rollback,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Journal {
    schema: u32,
    batch: Batch,
    history: Vec<Receipt>,
    forward: usize,
    undo: usize,
    mode: Mode,
    pending: Option<Operation>,
    verified: bool,
}
pub struct Executor {
    path: PathBuf,
    journal: Journal,
    lease: Option<Lease>,
    poisoned: bool,
    completion_window: Option<(u64, u64)>,
}
fn apply(rows: &mut Rows, op: &Operation, ret: Option<&str>) -> Result<()> {
    let members = rows
        .get_mut(op.router())
        .context("Missing operation router")?;
    match op {
        Operation::Add { member, .. } => {
            let id = ret.context("Missing actual returned add ID")?;
            ensure!(
                id_valid(id) && !members.iter().any(|m| m.id == id),
                "Returned row ID collides or is invalid"
            );
            let mut m = member.clone();
            m.id = id.into();
            members.push(m);
        }
        Operation::Remove { before, .. } => {
            ensure!(ret.is_none(), "Unexpected remove return ID");
            let pos = members
                .iter()
                .position(|m| m == before)
                .context("Exact remove row no longer exists")?;
            members.remove(pos);
        }
        Operation::Move { before, after, .. } => {
            ensure!(ret.is_none(), "Unexpected move return ID");
            let m = members
                .iter_mut()
                .find(|m| *m == before)
                .context("Exact move row no longer exists")?;
            *m = after.clone();
        }
    }
    normalize(rows)
}
impl Executor {
    pub fn open(root: &Path, batch: Batch, approved: &str) -> Result<Self> {
        batch.validate()?;
        let revision = batch.revision()?;
        ensure!(revision == approved, "Exact batch approval required");
        let root = root.canonicalize()?;
        ensure!(
            !root.starts_with("/opt/libreqos") && !root.starts_with("/etc/libreqos"),
            "Use separate application data"
        );
        let path = root
            .join("state/core-writer")
            .join(format!("{revision}.json"));
        if !path.exists() {
            let now = crate::orchestrator::now();
            ensure!(
                batch.created_at <= now && now <= batch.expires_at,
                "New writer batch is expired or future-dated"
            );
        }
        let lease = Lease::maintenance(
            &root,
            path.to_str().context("Invalid journal path")?,
            &revision,
        )?;
        let journal = if path.exists() {
            ensure!(
                std::fs::metadata(&path)?.len() <= 64 * 1024 * 1024,
                "Writer journal is too large"
            );
            let raw = std::fs::read_to_string(&path)?;
            ensure!(raw.len() <= 64 * 1024 * 1024, "Writer journal is too large");
            let j: Journal = serde_json::from_str(&raw)?;
            ensure!(
                j.schema == 1 && j.batch.revision()? == revision,
                "Invalid writer journal identity"
            );
            j
        } else {
            Journal {
                schema: 1,
                batch,
                history: vec![],
                forward: 0,
                undo: 0,
                mode: Mode::Forward,
                pending: None,
                verified: false,
            }
        };
        let mut this = Self {
            path,
            journal,
            lease: Some(lease),
            poisoned: false,
            completion_window: None,
        };
        this.validate_journal()?;
        this.journal.verified = false;
        this.save()?;
        Ok(this)
    }
    fn validate_journal(&self) -> Result<()> {
        self.journal.validate()
    }
    fn save(&mut self) -> Result<()> {
        if let Err(e) =
            store::atomic_write(&self.path, &serde_json::to_string_pretty(&self.journal)?)
        {
            self.poisoned = true;
            return Err(e.context("Writer journal save failed; stop until recovery"));
        }
        Ok(())
    }
    fn ready(&self) -> Result<()> {
        ensure!(!self.poisoned, "Writer storage failed; reopen for recovery");
        ensure!(
            self.journal.pending.is_none(),
            "Unknown write outcome; automatic retry and rollback are blocked"
        );
        Ok(())
    }
    fn rows(&self) -> Result<Rows> {
        let mut rows = self.journal.batch.baseline.clone();
        for r in &self.journal.history {
            apply(&mut rows, &r.operation, r.returned_id.as_deref())?;
        }
        Ok(rows)
    }
    fn inverse(&self, index: usize) -> Result<Operation> {
        self.journal.inverse(index)
    }
    fn guard(&self, e: &mut Evidence, forward: bool) -> Result<()> {
        let now = crate::orchestrator::now();
        normalize(&mut e.rows)?;
        ensure!(
            e.context == self.journal.batch.context && e.rows == self.rows()?,
            "Settings, sources, health, authorization or actual rows changed"
        );
        ensure!(
            e.captured_at <= now
                && now - e.captured_at <= 30
                && e.valid_until >= now
                && e.valid_until >= e.captured_at,
            "Writer verification evidence expired"
        );
        ensure!(
            !forward || now <= self.journal.batch.expires_at,
            "Writer batch expired"
        );
        Ok(())
    }
    async fn step(
        &mut self,
        io: &mut impl Transport,
        operation: Operation,
        forward: bool,
    ) -> Result<()> {
        self.ready()?;
        let mut evidence = io.observe().await?;
        self.guard(&mut evidence, forward)?;
        if let Operation::Add { router, member } = &operation {
            self.journal
                .batch
                .no_overlap(&evidence.rows, router, member, None)?;
        }
        self.journal.pending = Some(operation.clone());
        self.journal.verified = false;
        self.save()?;
        // Saving the intent can itself consume the remaining approval/evidence lifetime.
        if let Err(e) = self.guard(&mut evidence, forward) {
            self.journal.pending = None;
            self.save()?;
            return Err(e);
        }
        if forward {
            evidence.valid_until = evidence.valid_until.min(self.journal.batch.expires_at);
        }
        let result = io.execute(&operation, &evidence).await;
        let returned = match result {
            Ok(v) => v,
            Err(e) => {
                if e.downcast_ref::<RequestNotSent>().is_some() {
                    self.journal.pending = None;
                    self.save()?;
                }
                return Err(e);
            }
        };
        let mut expected = self.rows()?;
        apply(&mut expected, &operation, returned.as_deref())?;
        self.journal.history.push(Receipt {
            operation,
            returned_id: returned,
        });
        if forward {
            self.journal.forward += 1;
        } else {
            self.journal.undo += 1;
        }
        self.journal.pending = None;
        self.save()?;
        let mut after = io.observe().await?;
        self.guard(&mut after, forward)?;
        Ok(())
    }
    pub async fn forward(&mut self, io: &mut impl Transport) -> Result<()> {
        self.ready()?;
        ensure!(
            self.journal.mode == Mode::Forward,
            "Cannot resume forward after rollback started"
        );
        self.journal.verified = false;
        self.completion_window = None;
        self.save()?;
        while self.journal.forward < self.journal.batch.operations.len() {
            let op = self.journal.batch.operations[self.journal.forward].clone();
            self.step(io, op, true).await?;
        }
        let mut e = io.observe().await?;
        self.guard(&mut e, true)?;
        self.completion_window = Some((
            e.captured_at,
            e.valid_until
                .min(e.captured_at.saturating_add(30))
                .min(self.journal.batch.expires_at),
        ));
        self.journal.verified = true;
        self.save()
    }
    pub async fn rollback(&mut self, io: &mut impl Transport) -> Result<()> {
        self.ready()?;
        self.journal.mode = Mode::Rollback;
        self.journal.verified = false;
        self.completion_window = None;
        self.save()?;
        while self.journal.undo < self.journal.forward {
            let op = self.inverse(self.journal.forward - 1 - self.journal.undo)?;
            self.step(io, op, false).await?;
        }
        let mut e = io.observe().await?;
        self.guard(&mut e, false)?;
        self.completion_window = Some((
            e.captured_at,
            e.valid_until.min(e.captured_at.saturating_add(30)),
        ));
        self.journal.verified = true;
        self.save()
    }
    pub fn finish(mut self) -> Result<()> {
        self.ready()?;
        ensure!(
            self.journal.verified
                && completion_fresh(self.completion_window, crate::orchestrator::now()),
            "Writer completion verification is absent or expired"
        );
        self.lease.take().context("Missing writer lease")?.finish()
    }
}

fn completion_fresh(window: Option<(u64, u64)>, now: u64) -> bool {
    window.is_some_and(|(captured, until)| captured <= now && now <= until)
}

#[cfg(test)]
mod tests {
    #[test]
    fn completion_deadline_checks_both_clock_direction_and_expiry() {
        assert!(!super::completion_fresh(None, 100));
        assert!(!super::completion_fresh(Some((100, 110)), 99));
        assert!(super::completion_fresh(Some((100, 110)), 110));
        assert!(!super::completion_fresh(Some((100, 110)), 111));
    }
}

impl Journal {
    fn validate(&self) -> Result<()> {
        let j = self;
        ensure!(
            j.forward <= j.batch.operations.len()
                && j.undo <= j.forward
                && j.history.len() == j.forward + j.undo,
            "Inconsistent writer progress"
        );
        ensure!(
            j.mode == Mode::Rollback || j.undo == 0,
            "Invalid rollback progress"
        );
        let mut rows = j.batch.baseline.clone();
        for (i, r) in j.history.iter().enumerate() {
            let expected = if i < j.forward {
                j.batch.operations[i].clone()
            } else {
                self.inverse(j.forward - 1 - (i - j.forward))?
            };
            ensure!(
                r.operation == expected,
                "Journal operation differs from batch history"
            );
            apply(&mut rows, &r.operation, r.returned_id.as_deref())?;
        }
        if let Some(p) = &j.pending {
            let expected = if j.mode == Mode::Forward {
                j.batch
                    .operations
                    .get(j.forward)
                    .context("Pending past end of batch")?
                    .clone()
            } else {
                ensure!(j.undo < j.forward, "Pending past rollback end");
                self.inverse(j.forward - 1 - j.undo)?
            };
            ensure!(*p == expected && !j.verified, "Invalid pending operation");
        }
        ensure!(
            !j.verified
                || j.pending.is_none()
                    && if j.mode == Mode::Forward {
                        j.forward == j.batch.operations.len()
                    } else {
                        j.undo == j.forward
                    },
            "Invalid verification state"
        );
        Ok(())
    }
    fn inverse(&self, index: usize) -> Result<Operation> {
        let receipt = &self.history[index];
        Ok(match &receipt.operation {
            Operation::Add { router, member } => {
                let mut m = member.clone();
                m.id = receipt
                    .returned_id
                    .clone()
                    .context("Missing returned addition ID")?;
                Operation::Remove {
                    router: router.clone(),
                    before: m,
                }
            }
            Operation::Remove { router, before } => {
                let mut m = before.clone();
                m.id.clear();
                Operation::Add {
                    router: router.clone(),
                    member: m,
                }
            }
            Operation::Move {
                router,
                before,
                after,
            } => Operation::Move {
                router: router.clone(),
                before: after.clone(),
                after: before.clone(),
            },
        })
    }
}

#[derive(Clone, Debug, Serialize)]
pub struct OperationReview {
    pub kind: String,
    pub router: String,
    pub row_id: Option<String>,
}
#[derive(Clone, Debug, Serialize)]
pub struct JournalReview {
    pub revision: String,
    pub direction: String,
    pub operations: usize,
    pub completed: usize,
    pub undone: usize,
    pub pending: Option<OperationReview>,
    pub last_known: Option<OperationReview>,
    pub saved_verified: bool,
}
fn operation_review(op: &Operation, returned: Option<&str>) -> OperationReview {
    let (kind, id) = match op {
        Operation::Add { .. } => ("Add", returned.map(str::to_string)),
        Operation::Remove { before, .. } => ("Remove", Some(before.id.clone())),
        Operation::Move { before, .. } => ("Move", Some(before.id.clone())),
    };
    OperationReview {
        kind: kind.into(),
        router: op.router().into(),
        row_id: id,
    }
}
/// Pure saved-record inspection. This function cannot acquire a writer lease or verify live rows.
pub fn inspect_journal(raw: &str, revision: &str) -> Result<JournalReview> {
    ensure!(
        raw.len() <= 64 * 1024 * 1024,
        "Writer journal exceeds inspection limit"
    );
    let j: Journal = serde_json::from_str(raw)?;
    ensure!(
        j.schema == 1 && j.batch.revision()? == revision,
        "Writer journal identity or schema changed"
    );
    j.batch.validate()?;
    j.validate()?;
    Ok(JournalReview {
        revision: revision.into(),
        direction: if j.mode == Mode::Forward {
            "Forward"
        } else {
            "Rollback"
        }
        .into(),
        operations: j.batch.operations.len(),
        completed: j.forward,
        undone: j.undo,
        pending: j.pending.as_ref().map(|op| operation_review(op, None)),
        last_known: j
            .history
            .last()
            .map(|r| operation_review(&r.operation, r.returned_id.as_deref())),
        saved_verified: j.verified,
    })
}
