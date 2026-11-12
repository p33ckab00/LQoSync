//! One service-owned delivery worker. Persist API receipts before advancing a multipart event.
use super::{
    delivery::{ApiReceipt, DeliveryError, html_parts},
    esc,
};
use crate::{
    config::Telegram,
    system_health::{Domain, Hub, Observation, Phase, State},
    write_receipt::{Event, sha256},
};
use anyhow::ensure;
use serde::{Deserialize, Serialize};
use std::{
    io::Read,
    path::{Path, PathBuf},
};
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Frozen {
    pub enabled: bool,
    pub on_changes: bool,
    pub on_error: bool,
    pub route: String,
    pub max_chars: usize,
    pub details: String,
}
pub fn route(tg: &Telegram) -> String {
    sha256(format!("{}:{}:{}", tg.api_host, tg.api_port, super::chat_id(tg)).as_bytes())
}
impl Frozen {
    pub fn new(tg: &Telegram, report: &crate::status::ReportSummary) -> Self {
        let details = super::change_details(tg, report);
        Self {
            enabled: tg.enabled,
            on_changes: tg.notify_on_changes,
            on_error: tg.notify_on_error,
            route: route(tg),
            max_chars: tg.max_message_chars,
            details: if details.len() > 96 * 1024 {
                "Subscriber detail exceeds event budget; see local cycle report.".into()
            } else {
                details
            },
        }
    }
    pub fn from_status(tg: &Telegram, status: &crate::status::Status) -> Self {
        let mut value = Self::new(tg, &status.report);
        let telemetry = super::cycle_telemetry(tg, status).join("\n");
        if value.details.len() + telemetry.len() < 96 * 1024 {
            value.details.push('\n');
            value.details.push_str(&telemetry);
        }
        value
    }
    fn allows(&self, phase: Phase) -> bool {
        self.enabled
            && if matches!(
                phase,
                Phase::WriteFailed | Phase::ApplyFailed | Phase::VerifyTimeout | Phase::Interrupted
            ) {
                self.on_error
            } else {
                self.on_changes
            }
    }
}
#[derive(Debug, Serialize, Deserialize)]
struct Generic {
    version: u32,
    id: String,
    captured_at: u64,
    notice: Frozen,
    text: String,
}
#[derive(Debug, Serialize, Deserialize, Default)]
struct Progress {
    parts: Vec<String>,
    receipts: Vec<ApiReceipt>,
    attempts: u32,
    next_attempt: u64,
    paused: bool,
    error: Option<String>,
    completed: bool,
    skipped: bool,
}
struct Job {
    id: String,
    path: PathBuf,
    captured_at: u64,
    notice: Frozen,
    text: String,
    write: Option<(String, Phase)>,
}
fn read<T: serde::de::DeserializeOwned>(path: &Path) -> anyhow::Result<T> {
    let mut bytes = Vec::new();
    std::fs::File::open(path)?
        .take(256 * 1024 + 1)
        .read_to_end(&mut bytes)?;
    ensure!(bytes.len() <= 256 * 1024, "event exceeds bound");
    Ok(serde_json::from_slice(&bytes)?)
}
fn save<T: Serialize>(path: &Path, value: &T) -> anyhow::Result<()> {
    let bytes = serde_json::to_vec(value)?;
    ensure!(bytes.len() <= 256 * 1024, "delivery record exceeds bound");
    crate::files::write_secret(path, &bytes)
}
fn private_dir(path: &Path) -> anyhow::Result<()> {
    use std::os::unix::fs::{DirBuilderExt, PermissionsExt};
    std::fs::DirBuilder::new()
        .recursive(true)
        .mode(0o700)
        .create(path)?;
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o700))?;
    Ok(())
}
fn id_valid(id: &str) -> bool {
    !id.is_empty()
        && id.len() <= 96
        && id
            .bytes()
            .all(|b| b.is_ascii_uppercase() || b.is_ascii_digit() || b == b'_' || b == b'-')
}
fn event_text(event: &Event) -> String {
    let r = &event.receipt;
    let phase = match r.phase {
        Phase::Pending => "WRITTEN / PENDING",
        Phase::ImportConfirmed => "IMPORTED / PENDING",
        Phase::Applied => "APPLIED",
        Phase::WriteFailed => "WRITE FAILED",
        Phase::ApplyFailed => "APPLY FAILED",
        Phase::VerifyTimeout => "VERIFY TIMEOUT",
        Phase::Superseded => "SUPERSEDED",
        _ => "INTERRUPTED",
    };
    let mut lines = vec![
        format!("<b>lqosync3 {phase}</b>"),
        format!(
            "Write: <code>{}</code> | cycle {}",
            esc(&r.write_id),
            r.meta.cycle_seq
        ),
        format!(
            "Snapshot: {} UTC Unix seconds | operation started {}",
            event.captured_at, r.started
        ),
        format!("System health: {:?}", event.health.overall),
    ];
    for file in &r.files {
        lines.push(format!(
            "{}: {:?}",
            esc(&file.path.file_name().unwrap_or_default().to_string_lossy()),
            file.outcome
        ));
    }
    if r.phase == Phase::Pending {
        lines.push("Files are written. Native acceptance is pending.".into());
    }
    if r.phase == Phase::Applied {
        lines.push("Matching imported inputs and native policy acknowledgement verified.".into());
    }
    if r.phase == Phase::VerifyTimeout {
        lines.push("Acceptance was not verified before the deadline. This does not prove the dataplane stopped.".into());
    }
    if r.phase == Phase::WriteFailed {
        lines.push("Partial write possible; change details describe intended changes.".into());
    }
    if let Some(proof) = &r.acceptance {
        lines.push(format!(
            "Circuit-slot capacity: native {} / generated {} (independent allocation evidence)",
            proof
                .circuit_capacity
                .map_or("unknown".into(), |v| v.to_string()),
            proof.desired_circuit_capacity
        ));
    }
    for domain in Domain::ALL {
        let items: Vec<_> = event
            .health
            .components
            .iter()
            .filter(|c| c.domain == domain)
            .collect();
        lines.push(format!("<b>{}</b>", domain.label()));
        for c in items {
            lines.push(format!(
                "{}: {:?} / {} / age {}s",
                esc(&c.component),
                c.state,
                esc(&c.reason),
                event.captured_at.saturating_sub(c.observed_at)
            ));
        }
    }
    if let Some(bridge) = &event.health.bridge
        && let Some(summary) = &bridge.summary
    {
        lines.push(format!(
            "Bridge uptime: {}s | topology {} | degraded: {}{}",
            summary.uptime_ms / 1000,
            esc(summary.topology_tier.as_deref().unwrap_or("unknown")),
            esc(&summary
                .degraded_subsystems
                .as_ref()
                .map(|v| v.join(", "))
                .unwrap_or("unknown".into())),
            if bridge.live(event.captured_at) {
                ""
            } else {
                " (last known)"
            }
        ));
    }
    if matches!(r.phase, Phase::Pending | Phase::WriteFailed)
        && let Some(notice) = &r.meta.notice
    {
        lines.push(notice.details.clone());
    }
    lines.join("\n")
}
fn jobs(dir: &Path) -> anyhow::Result<Vec<Job>> {
    let mut result = Vec::new();
    let mut total = 0u64;
    for (folder, write) in [("write-events", true), ("telegram-events", false)] {
        let path = dir.join(folder);
        if !path.exists() {
            continue;
        }
        for entry in std::fs::read_dir(path)? {
            let path = entry?.path();
            if path.extension().is_none_or(|x| x != "json") {
                continue;
            }
            total = total.saturating_add(std::fs::metadata(&path)?.len());
            ensure!(
                result.len() < crate::write_receipt::MAX_EVENTS * 2
                    && total <= crate::write_receipt::MAX_EVENT_BYTES * 2,
                "event queue exceeds bound"
            );
            if write {
                let event: Event = read(&path)?;
                ensure!(
                    event.version == 1 && id_valid(&event.id),
                    "invalid write event"
                );
                // A receipt from before notice capture is paused locally; it cannot inherit a new recipient.
                let notice = event.receipt.meta.notice.clone().ok_or_else(|| {
                    anyhow::anyhow!("write event has no frozen notification policy")
                })?;
                result.push(Job {
                    id: event.id.clone(),
                    captured_at: event.captured_at,
                    notice,
                    text: event_text(&event),
                    write: Some((event.receipt.write_id, event.receipt.phase)),
                    path,
                });
            } else {
                let e: Generic = read(&path)?;
                ensure!(
                    e.version == 1 && id_valid(&e.id),
                    "invalid notification event"
                );
                result.push(Job {
                    id: e.id,
                    captured_at: e.captured_at,
                    notice: e.notice,
                    text: e.text,
                    write: None,
                    path,
                });
            }
        }
    }
    result.sort_by(|a, b| {
        a.captured_at
            .cmp(&b.captured_at)
            .then_with(|| {
                let rank = |j: &Job| {
                    if j.write.as_ref().is_some_and(|(_, p)| *p == Phase::Pending) {
                        0
                    } else {
                        1
                    }
                };
                rank(a).cmp(&rank(b))
            })
            .then(a.id.cmp(&b.id))
    });
    Ok(result)
}
/// Retain existing router/error notifications, but queue them instead of awaiting the network.
pub fn enqueue(dir: &Path, tg: &Telegram, text: &str, now: u64) -> anyhow::Result<()> {
    let path = dir.join("telegram-events");
    private_dir(&path)?;
    let entries: Vec<_> = std::fs::read_dir(&path)?.collect::<Result<_, _>>()?;
    let size = entries
        .iter()
        .try_fold(0u64, |n, e| e.metadata().map(|m| n.saturating_add(m.len())))?;
    let id = format!("NOTICE-{}", crate::runner::random_id(24));
    let event = Generic {
        version: 1,
        id: id.clone(),
        captured_at: now,
        notice: Frozen::new(tg, &Default::default()),
        text: text.into(),
    };
    let bytes = serde_json::to_vec(&event)?;
    ensure!(
        entries.len() < crate::write_receipt::MAX_EVENTS
            && size + bytes.len() as u64 <= crate::write_receipt::MAX_EVENT_BYTES,
        "notification queue capacity exceeded"
    );
    crate::files::write_secret(&path.join(format!("{id}.json")), &bytes)
}
#[derive(Default, Serialize, Deserialize)]
struct RateGate {
    route: String,
    until: u64,
}
pub struct Outbox {
    dir: PathBuf,
    hub: Hub,
    route_epoch: String,
}
impl Outbox {
    pub fn new(dir: &Path, hub: Hub) -> anyhow::Result<Self> {
        private_dir(&dir.join("telegram-delivery"))?;
        Ok(Self {
            dir: dir.into(),
            hub,
            route_epoch: "unknown".into(),
        })
    }
    fn finish(&self, job: &Job, progress: &Path) -> anyhow::Result<()> {
        std::fs::remove_file(&job.path)?;
        // Keep write tombstones until a newer durable operation replaces this receipt.
        // Tracker recovery can then replay an interrupted event handoff without duplicate sends.
        if job.write.is_none() {
            std::fs::remove_file(progress)?;
        }
        Ok(())
    }
    fn prune(&self) -> anyhow::Result<()> {
        let current = self.dir.join("write-receipt.json");
        let current = if current.exists() {
            read::<serde_json::Value>(&current)?["write_id"]
                .as_str()
                .map(str::to_string)
        } else {
            None
        };
        let pending: std::collections::HashSet<_> = if self.dir.join("write-events").exists() {
            std::fs::read_dir(self.dir.join("write-events"))?
                .filter_map(|e| e.ok())
                .filter_map(|e| {
                    e.file_name()
                        .to_str()
                        .and_then(|n| n.split('-').next())
                        .map(str::to_string)
                })
                .collect()
        } else {
            Default::default()
        };
        let mut count = 0;
        let mut bytes = 0u64;
        for entry in std::fs::read_dir(self.dir.join("telegram-delivery"))? {
            let entry = entry?;
            let path = entry.path();
            if path.extension().is_none_or(|x| x != "json") {
                continue;
            }
            let name = entry.file_name();
            let name = name.to_string_lossy();
            let id = name.split('-').next().unwrap_or_default();
            if id.len() == 24 && Some(id) != current.as_deref() && !pending.contains(id) {
                let p: Progress = read(&path)?;
                if p.completed {
                    std::fs::remove_file(&path)?;
                    continue;
                }
            }
            count += 1;
            bytes = bytes.saturating_add(entry.metadata()?.len());
        }
        ensure!(
            count <= 1024 && bytes <= 16 * 1024 * 1024,
            "delivery receipt capacity exceeded"
        );
        Ok(())
    }
    fn health(&self, state: State, reason: &str, now: u64) {
        let _ = self.hub.observe(Observation {
            domain: Domain::Notification,
            component: "telegram_delivery".into(),
            state,
            reason: reason.into(),
            observed_at: now,
            max_age: 15,
            source_epoch: format!("outbox:{}", self.route_epoch),
            last_success: None,
        });
    }
    /// At most one API request per call. An injected sender permits deterministic offline outage tests.
    pub async fn step<F, Fut>(
        &mut self,
        tg: &Telegram,
        now: u64,
        mut send: F,
    ) -> anyhow::Result<bool>
    where
        F: FnMut(String) -> Fut,
        Fut: std::future::Future<Output = Result<ApiReceipt, DeliveryError>>,
    {
        let delivery = self.dir.join("telegram-delivery");
        if !delivery.is_dir() {
            private_dir(&delivery)?;
        }
        self.route_epoch = route(tg);
        if self.dir.join("write-event-gap.json").exists() {
            let _ = self.hub.observe(Observation {
                domain: Domain::Notification,
                component: "write_event_store".into(),
                state: State::Degraded,
                reason: "write_event_storage_gap".into(),
                observed_at: now,
                max_age: u64::MAX,
                source_epoch: "service".into(),
                last_success: None,
            });
        }
        self.prune()?;
        let gate_path = self.dir.join("telegram-delivery/retry-gate.json");
        let gate: RateGate = if gate_path.exists() {
            read(&gate_path)?
        } else {
            RateGate::default()
        };
        if gate.route == route(tg) && gate.until > now {
            self.health(State::Degraded, "telegram_rate_limit_wait", now);
            return Ok(false);
        }
        let jobs = jobs(&self.dir)?;
        let mut backlog = false;
        let mut fault = None;
        for job in jobs {
            let progress_path = self
                .dir
                .join("telegram-delivery")
                .join(format!("{}.json", job.id));
            let mut p = if progress_path.exists() {
                read::<Progress>(&progress_path)?
            } else {
                Progress::default()
            };
            ensure!(
                (p.receipts.len() < p.parts.len()
                    || p.parts.is_empty() && p.receipts.is_empty()
                    || p.completed)
                    && p.parts.len() <= 2048
                    && p.parts.iter().all(|s| s.len() <= 16384),
                "invalid delivery progress"
            );
            let allowed = job
                .write
                .as_ref()
                .map_or(job.notice.enabled, |(_, phase)| job.notice.allows(*phase));
            if p.completed {
                self.finish(&job, &progress_path)?;
                continue;
            }
            if !allowed {
                p.completed = true;
                p.skipped = true;
                save(&progress_path, &p)?;
                self.finish(&job, &progress_path)?;
                continue;
            }
            backlog = true;
            if !tg.enabled || route(tg) != job.notice.route {
                fault = Some("telegram_route_paused");
                continue;
            }
            if p.paused {
                fault = Some("telegram_delivery_paused");
                continue;
            }
            if let Some((id, phase)) = &job.write
                && !matches!(phase, Phase::Pending | Phase::WriteFailed)
            {
                let pending = self
                    .dir
                    .join("telegram-delivery")
                    .join(format!("{id}-PENDING.json"));
                if !pending.exists() || !read::<Progress>(&pending)?.completed {
                    fault = Some("telegram_predecessor_pending");
                    continue;
                }
            }
            if p.next_attempt > now {
                fault = Some("telegram_retry_wait");
                continue;
            }
            if p.parts.is_empty() {
                let label = if now.saturating_sub(job.captured_at) > 15 {
                    format!(
                        "Historical snapshot; delivery starts {}s later.\n",
                        now.saturating_sub(job.captured_at)
                    )
                } else {
                    String::new()
                };
                p.parts = html_parts(
                    &format!("Event <code>{}</code>\n{label}{}", esc(&job.id), job.text),
                    job.notice.max_chars,
                )
                .map_err(anyhow::Error::msg)?;
                ensure!(
                    !p.parts.is_empty() && p.parts.len() <= 2048,
                    "notification parts exceed bound"
                );
                save(&progress_path, &p)?;
            }
            let part = p.parts[p.receipts.len()].clone();
            match send(part).await {
                Ok(receipt) => {
                    p.receipts.push(receipt);
                    p.attempts = 0;
                    p.error = None;
                    p.completed = p.receipts.len() == p.parts.len();
                }
                Err(e) => {
                    p.attempts = p.attempts.saturating_add(1);
                    p.next_attempt = now.saturating_add(
                        e.retry_after
                            .unwrap_or(2u64.saturating_pow(p.attempts.min(9)))
                            .max(1),
                    );
                    if e.retry_after.is_some() {
                        save(
                            &gate_path,
                            &RateGate {
                                route: route(tg),
                                until: p.next_attempt,
                            },
                        )?;
                    }
                    p.paused = e.permanent;
                    p.error = Some(e.reason);
                }
            }
            save(&progress_path, &p)?;
            if p.completed {
                self.finish(&job, &progress_path)?;
            }
            self.health(
                if p.error.is_some() {
                    State::Degraded
                } else {
                    State::Updating
                },
                if p.error.is_some() {
                    "telegram_send_failed"
                } else {
                    "telegram_queue_draining"
                },
                now,
            );
            return Ok(true);
        }
        self.health(
            if fault.is_some() {
                State::Degraded
            } else if backlog {
                State::Updating
            } else if !tg.enabled {
                State::NotApplicable
            } else {
                State::Healthy
            },
            fault.unwrap_or(if !tg.enabled {
                "telegram_disabled"
            } else {
                "telegram_queue_empty"
            }),
            now,
        );
        Ok(false)
    }
}
/// Aborting this owner cancels in-flight delivery. Saved receipts resume on restart.
pub struct Worker {
    settings: tokio::sync::watch::Sender<Telegram>,
    task: tokio::task::JoinHandle<()>,
}
impl Worker {
    pub fn start(dir: &Path, tg: &Telegram, hub: Hub, owner: u32) -> Self {
        // Queue storage can recover independently. Its initial failure must not stop collection.
        let mut outbox = Outbox {
            dir: dir.into(),
            hub,
            route_epoch: "unknown".into(),
        };
        let (settings, mut changes) = tokio::sync::watch::channel(tg.clone());
        let task = tokio::spawn(async move {
            let mut current = changes.borrow().clone();
            let mut tick = tokio::time::interval(std::time::Duration::from_secs(3));
            tick.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
            loop {
                tokio::select! {r=changes.changed()=>{if r.is_err(){break;}current=changes.borrow_and_update().clone();},_=tick.tick()=>{
                    let now=std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_secs();
                    let tg=current.clone();if outbox.step(&current,now,|text|{let tg=tg.clone();async move{super::send_receipt(&tg,&text,owner).await}}).await.is_err() {outbox.health(State::Degraded,"telegram_store_unavailable",now);}
                }}
            }
        });
        Self { settings, task }
    }
    pub fn reconfigure(&self, tg: &Telegram) {
        self.settings.send_replace(tg.clone());
    }
}
impl Drop for Worker {
    fn drop(&mut self) {
        self.task.abort();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::write_receipt::{Meta, Tracker};
    fn tg() -> Telegram {
        Telegram {
            enabled: true,
            chat_id: "123".into(),
            ..Default::default()
        }
    }
    fn fixture() -> (tempfile::TempDir, Tracker, Hub, Telegram) {
        let dir = tempfile::tempdir().unwrap();
        let hub = Hub::default();
        let tg = tg();
        let mut tracker = Tracker::load(dir.path()).unwrap();
        let outputs = vec![
            (dir.path().join("network.json"), "{}".into()),
            (dir.path().join("ShapedDevices.csv"), "test".into()),
        ];
        tracker.write(
            &outputs,
            Meta {
                cycle_seq: 1,
                config_digest: "cfg".into(),
                notice: Some(Frozen::new(&tg, &Default::default())),
            },
            10,
            &hub,
        );
        (dir, tracker, hub, tg)
    }
    async fn drain(outbox: &mut Outbox, tg: &Telegram, now: u64) -> Vec<String> {
        let mut output = Vec::new();
        for _ in 0..128 {
            let mut sent = None;
            let did = outbox
                .step(tg, now, |text| {
                    sent = Some(text);
                    std::future::ready(Ok(ApiReceipt { message_id: 1 }))
                })
                .await
                .unwrap();
            if let Some(text) = sent {
                output.push(text);
            }
            if !did {
                break;
            }
        }
        output
    }
    #[tokio::test]
    async fn pending_and_terminal_send_without_another_write() {
        let (dir, mut tracker, hub, tg) = fixture();
        let mut outbox = Outbox::new(dir.path(), hub.clone()).unwrap();
        let first = drain(&mut outbox, &tg, 20).await.join("\n");
        assert!(first.contains("WRITTEN / PENDING"));
        assert!(!first.contains("APPLIED</b>"));
        tracker
            .transition(Phase::VerifyTimeout, 2500, &hub)
            .unwrap();
        let last = drain(&mut outbox, &tg, 2500).await.join("\n");
        assert!(last.contains("VERIFY TIMEOUT"));
        assert!(last.contains(&tracker.current().unwrap().write_id));
    }
    #[tokio::test]
    async fn restart_resumes_after_accepted_parts_and_waits_before_terminal() {
        let (dir, mut tracker, hub, mut tg) = fixture();
        tg.max_message_chars = 128;
        // Use small parts frozen at write time.
        let p = std::fs::read_dir(dir.path().join("write-events"))
            .unwrap()
            .next()
            .unwrap()
            .unwrap()
            .path();
        let mut event: Event = read(&p).unwrap();
        event.receipt.meta.notice.as_mut().unwrap().max_chars = 128;
        save(&p, &event).unwrap();
        tracker
            .transition(Phase::VerifyTimeout, 2500, &hub)
            .unwrap();
        let mut outbox = Outbox::new(dir.path(), hub.clone()).unwrap();
        let mut accepted = String::new();
        outbox
            .step(&tg, 20, |text| {
                accepted = text;
                std::future::ready(Ok(ApiReceipt { message_id: 17 }))
            })
            .await
            .unwrap();
        drop(outbox);
        let mut outbox = Outbox::new(dir.path(), hub).unwrap();
        let rest = drain(&mut outbox, &tg, 2500).await;
        assert!(!rest.contains(&accepted));
        let all = rest.join("\n");
        assert!(all.contains("VERIFY TIMEOUT"));
    }
    #[tokio::test]
    async fn retry_and_route_change_never_reroute_a_snapshot() {
        let (dir, _tracker, hub, tg) = fixture();
        let mut outbox = Outbox::new(dir.path(), hub).unwrap();
        outbox
            .step(&tg, 20, |_| {
                std::future::ready(Err(DeliveryError {
                    reason: "telegram_api_429".into(),
                    retry_after: Some(42),
                    permanent: false,
                }))
            })
            .await
            .unwrap();
        enqueue(dir.path(), &tg, "another event", 21).unwrap();
        assert!(
            !outbox
                .step(&tg, 61, |_| async { panic!("retry was early") })
                .await
                .unwrap()
        );
        let mut changed = tg.clone();
        changed.chat_id = "456".into();
        assert!(
            !outbox
                .step(&changed, 100, |_| async { panic!("recipient changed") })
                .await
                .unwrap()
        );
        assert!(!drain(&mut outbox, &tg, 100).await.is_empty());
    }
    #[tokio::test]
    async fn permanent_failures_pause_and_disabled_events_stay_disabled() {
        let dir = tempfile::tempdir().unwrap();
        let hub = Hub::default();
        let mut tg = tg();
        tg.enabled = false;
        enqueue(dir.path(), &tg, "<b>test</b>", 10).unwrap();
        let mut outbox = Outbox::new(dir.path(), hub).unwrap();
        tg.enabled = true;
        assert!(
            !outbox
                .step(&tg, 20, |_| async { panic!("disabled event sent") })
                .await
                .unwrap()
        );
        enqueue(dir.path(), &tg, "<b>test</b>", 20).unwrap();
        outbox
            .step(&tg, 20, |_| {
                std::future::ready(Err(DeliveryError {
                    reason: "telegram_api_403".into(),
                    retry_after: None,
                    permanent: true,
                }))
            })
            .await
            .unwrap();
        assert!(
            !outbox
                .step(&tg, 9999, |_| async { panic!("permanent failure retried") })
                .await
                .unwrap()
        );
    }
    #[test]
    fn queue_capacity_is_explicit_and_contains_no_token() {
        let dir = tempfile::tempdir().unwrap();
        let mut tg = tg();
        tg.bot_token_file = "SECRET".into();
        for n in 0..crate::write_receipt::MAX_EVENTS {
            enqueue(dir.path(), &tg, "test", n as u64).unwrap();
        }
        assert!(enqueue(dir.path(), &tg, "test", 1000).is_err());
        let raw = std::fs::read_dir(dir.path().join("telegram-events"))
            .unwrap()
            .next()
            .unwrap()
            .unwrap()
            .path();
        assert!(
            !String::from_utf8(std::fs::read(raw).unwrap())
                .unwrap()
                .contains("SECRET")
        );
    }
}

#[cfg(test)]
mod shutdown_tests {
    use super::*;
    #[tokio::test]
    async fn shutdown_cancels_a_silent_tls_peer_without_waiting_for_timeout() {
        let dir = tempfile::tempdir().unwrap();
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let tg = Telegram {
            enabled: true,
            chat_id: "123".into(),
            api_host: "127.0.0.1".into(),
            api_port: listener.local_addr().unwrap().port(),
            bot_token_env: "LQOSYNC_TEST_SHUTDOWN_TOKEN".into(),
            timeout: 60,
            ..Default::default()
        };
        // A private fixture secret file avoids process-global environment changes in parallel tests.
        let secret = dir.path().join("token");
        std::fs::write(&secret, "fixture-token").unwrap();
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&secret, std::fs::Permissions::from_mode(0o600)).unwrap();
        let tg = Telegram {
            bot_token_env: String::new(),
            bot_token_file: secret.to_string_lossy().into(),
            ..tg
        };
        enqueue(dir.path(), &tg, "test", 1).unwrap();
        let worker = Worker::start(dir.path(), &tg, Hub::default(), unsafe { libc::getuid() });
        let (_connection, _) =
            tokio::time::timeout(std::time::Duration::from_secs(2), listener.accept())
                .await
                .unwrap()
                .unwrap();
        let start = std::time::Instant::now();
        drop(worker);
        tokio::task::yield_now().await;
        assert!(start.elapsed() < std::time::Duration::from_millis(100));
    }
}

#[cfg(test)]
mod storage_recovery_tests {
    use super::*;
    #[tokio::test(start_paused = true)]
    async fn worker_storage_failure_is_visible_and_recovers_without_a_service_restart() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("telegram-delivery"), "blocked").unwrap();
        let hub = Hub::default();
        let tg = Telegram::default();
        let worker = Worker::start(dir.path(), &tg, hub.clone(), 1000);
        for _ in 0..4 {
            tokio::task::yield_now().await;
        }
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        assert!(
            hub.snapshot(now, None)
                .components
                .iter()
                .any(|c| c.component == "telegram_delivery" && c.state == State::Degraded)
        );
        std::fs::remove_file(dir.path().join("telegram-delivery")).unwrap();
        tokio::time::advance(std::time::Duration::from_secs(3)).await;
        for _ in 0..4 {
            tokio::task::yield_now().await;
        }
        assert!(
            hub.snapshot(now, None)
                .components
                .iter()
                .any(|c| c.component == "telegram_delivery" && c.state == State::NotApplicable)
        );
        drop(worker);
    }
}
