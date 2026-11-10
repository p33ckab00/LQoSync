//! Read-only DPDK worker utilization for the service and TUI.

pub mod classification;
pub mod detail;
pub mod health;
mod ipc;
pub mod pressure;
pub mod queue;
pub mod traffic;
pub use ipc::{read_health, read_snapshot};
use pressure::{Diagnostics, Pressure, TransitionLog};

use anyhow::ensure;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::path::Path;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tokio::sync::watch;

pub const CACHE: &str = "worker-utilization.json";
pub const MAX_AGE: u64 = 6;
const MAX_WORKERS: usize = 4096;
const MAX_CACHE_BYTES: u64 = 1024 * 1024;

#[derive(Debug, Clone, Default, Deserialize, Serialize, PartialEq, Eq)]
pub struct WorkerRow {
    pub queue_id: u16,
    pub busy_ppm: u64,
    pub backlog_mean: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub idle_ppm: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub laps_per_s: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub busy_laps: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cycles_per_busy_lap: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub phase_cycles: Option<Vec<u64>>,
}

#[derive(Debug, Deserialize)]
pub struct Snapshot {
    pub captured_at_millis: u64,
    #[serde(default)]
    pub workers: Vec<WorkerRow>,
    pub diagnostics: Option<Diagnostics>,
    pub actual: Option<traffic::Counters>,
    pub mapped: Option<traffic::Counters>,
    pub unmapped: Option<traffic::Counters>,
    pub qdisc_totals: Option<queue::Totals>,
}

fn validate_rows(rows: &[WorkerRow]) -> anyhow::Result<()> {
    ensure!(
        !rows.is_empty(),
        "no worker utilization rows (older bridge or no rollup)"
    );
    ensure!(rows.len() <= MAX_WORKERS, "too many worker rows");
    let mut ids = HashSet::with_capacity(rows.len());
    for row in rows {
        ensure!(
            row.busy_ppm <= 1_000_000,
            "worker busy_ppm is above 100 percent"
        );
        ensure!(ids.insert(row.queue_id), "duplicate worker id");
    }
    Ok(())
}

impl Snapshot {
    fn validate(&self) -> anyhow::Result<()> {
        ensure!(self.captured_at_millis > 0, "missing producer timestamp");
        validate_rows(&self.workers)
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Telemetry {
    pub version: u32,
    pub updated: u64,
    pub stale: bool,
    pub reason: Option<String>,
    pub workers: Vec<WorkerRow>,
    #[serde(default)]
    pub pressure: Option<Pressure>,
    #[serde(default)]
    pub traffic: std::collections::VecDeque<traffic::Point>,
    #[serde(default)]
    pub queue: Option<queue::Queue>,
    #[serde(default)]
    pub classification: classification::Classification,
    #[serde(default)]
    pub health: health::Health,
}

impl Default for Telemetry {
    fn default() -> Self {
        Self {
            version: 1,
            updated: 0,
            stale: true,
            reason: Some("no sample yet".into()),
            workers: Vec::new(),
            pressure: None,
            traffic: Default::default(),
            queue: None,
            classification: Default::default(),
            health: Default::default(),
        }
    }
}

impl Telemetry {
    pub fn live(&self, now: u64) -> bool {
        !self.stale && self.updated > 0 && now >= self.updated && now - self.updated <= MAX_AGE
    }

    fn fail(&mut self, reason: String) {
        self.stale = true;
        self.reason = Some(
            reason
                .chars()
                .filter(|c| !c.is_control())
                .take(180)
                .collect(),
        );
    }
}

#[derive(Default)]
struct Tracker {
    captured: Option<u64>,
    report: Telemetry,
    previous_diagnostics: Option<Diagnostics>,
    previous_at: u64,
    previous_queue: Option<queue::Totals>,
    traffic: traffic::History,
    classification: classification::Tracker,
}

impl Tracker {
    fn record(&mut self, sample: anyhow::Result<Snapshot>, now: u64) {
        match sample.and_then(|sample| {
            sample.validate()?;
            Ok(sample)
        }) {
            Ok(mut sample) => {
                for worker in &mut sample.workers {
                    worker.sanitize_extended();
                }
                if self.captured != Some(sample.captured_at_millis) {
                    self.classification.record(
                        &mut self.report.classification,
                        sample.captured_at_millis,
                        sample.mapped,
                        sample.unmapped,
                        now,
                    );
                }
                self.traffic.record(
                    &mut self.report.traffic,
                    sample.actual.map(|c| (sample.captured_at_millis, c)),
                    now,
                );
                if self
                    .captured
                    .is_none_or(|last| sample.captured_at_millis < last)
                {
                    self.captured = Some(sample.captured_at_millis);
                    self.previous_queue = sample.qdisc_totals;
                    self.previous_diagnostics = sample.diagnostics;
                    self.previous_at = now;
                    self.report
                        .fail("waiting for next producer snapshot".into());
                } else if self.captured != Some(sample.captured_at_millis) {
                    if now < self.previous_at || now - self.previous_at > MAX_AGE {
                        self.previous_diagnostics = None;
                        self.previous_queue = None;
                    }
                    if self
                        .captured
                        .is_some_and(|last| sample.captured_at_millis - last > MAX_AGE * 1000)
                    {
                        self.previous_queue = None;
                    }
                    let queue = sample
                        .qdisc_totals
                        .clone()
                        .map(|totals| queue::Queue::between(totals, self.previous_queue.as_ref()));
                    self.previous_queue = sample.qdisc_totals;
                    self.captured = Some(sample.captured_at_millis);
                    sample.workers.sort_unstable_by_key(|w| w.queue_id);
                    self.report = Telemetry {
                        version: 1,
                        updated: now,
                        stale: false,
                        reason: None,
                        workers: sample.workers,
                        traffic: std::mem::take(&mut self.report.traffic),
                        queue,
                        classification: std::mem::take(&mut self.report.classification),
                        health: std::mem::take(&mut self.report.health),
                        pressure: Some(Pressure::between(
                            sample.diagnostics.as_ref(),
                            self.previous_diagnostics.as_ref(),
                        )),
                    };
                    self.previous_diagnostics = sample.diagnostics;
                    self.previous_at = now;
                } else if !self.report.live(now) {
                    self.report
                        .fail("producer snapshot has not advanced".into());
                }
            }
            Err(error) => {
                self.classification
                    .fail(&mut self.report.classification, now);
                self.traffic.record(&mut self.report.traffic, None, now);
                self.previous_diagnostics = None;
                self.previous_queue = None;
                self.report.fail(error.to_string());
            }
        }
    }
}

pub fn read_cache(path: &Path) -> Telemetry {
    use std::io::Read;
    let read = || -> anyhow::Result<Telemetry> {
        let mut bytes = Vec::new();
        std::fs::File::open(path)?
            .take(MAX_CACHE_BYTES + 1)
            .read_to_end(&mut bytes)?;
        ensure!(
            bytes.len() as u64 <= MAX_CACHE_BYTES,
            "worker cache is too large"
        );
        let mut report: Telemetry = serde_json::from_slice(&bytes)?;
        ensure!(
            report.traffic.len() <= traffic::MAX_POINTS
                && report.traffic.iter().all(traffic::Point::valid),
            "invalid traffic history"
        );
        ensure!(
            report
                .traffic
                .iter()
                .zip(report.traffic.iter().skip(1))
                .all(|(a, b)| a.at < b.at),
            "unordered traffic history"
        );
        ensure!(report.version == 1, "unsupported worker cache version");
        if report.pressure.as_ref().is_some_and(|p| !p.valid()) {
            report.pressure = Some(Pressure::default());
        }
        if report.queue.as_ref().is_some_and(|q| !q.valid()) {
            report.queue = None;
        }
        report.classification.sanitize();
        if !report.health.valid() {
            report.health = health::Health::default();
        }
        if !report.workers.is_empty() {
            validate_rows(&report.workers)?;
            for worker in &mut report.workers {
                worker.sanitize_extended();
            }
        }
        ensure!(
            report.stale || !report.workers.is_empty(),
            "live cache has no worker rows"
        );
        Ok(report)
    };
    read().unwrap_or_else(|error| {
        let mut report = Telemetry::default();
        report.fail(format!("worker cache unavailable: {error}"));
        report
    })
}

fn publish(dir: &Path, report: &Telemetry) -> anyhow::Result<()> {
    use std::os::unix::fs::PermissionsExt;
    let mut bytes = serde_json::to_vec(report)?;
    if bytes.len() as u64 > MAX_CACHE_BYTES {
        let mut basic = report.clone();
        for worker in &mut basic.workers {
            worker.clear_extended();
        }
        let notice = "Extended worker metrics omitted: 1 MiB cache budget";
        basic.reason = Some(match &report.reason {
            Some(reason) => format!("{reason}; {notice}"),
            None => notice.into(),
        });
        bytes = serde_json::to_vec(&basic)?;
    }
    ensure!(
        bytes.len() as u64 <= MAX_CACHE_BYTES,
        "worker cache exceeds its size budget"
    );
    let tmp = dir.join(format!(".{CACHE}.tmp-{}", std::process::id()));
    std::fs::write(&tmp, bytes)?;
    std::fs::set_permissions(&tmp, std::fs::Permissions::from_mode(0o644))?;
    std::fs::rename(tmp, dir.join(CACHE))?;
    Ok(())
}

#[derive(Clone, PartialEq, Eq)]
struct Settings {
    v3: bool,
    socket: String,
    state: String,
}

impl Settings {
    fn from_config(cfg: &crate::config::Config) -> Self {
        Self {
            v3: crate::detect::resolve(&cfg.libreqos)
                .is_ok_and(|d| d.target != crate::config::Target::V2Pair),
            socket: cfg.libreqos.v3_analytics_socket.clone(),
            state: cfg.runtime.state_dir.clone(),
        }
    }
}

/// One monitor per service. The TUI reads aggregate telemetry from its public cache.
pub struct Monitor {
    settings: watch::Sender<Settings>,
    task: tokio::task::JoinHandle<()>,
}

impl Monitor {
    pub fn start(cfg: &crate::config::Config) -> Self {
        let (settings, mut changes) = watch::channel(Settings::from_config(cfg));
        let task = tokio::spawn(async move {
            let mut current = changes.borrow().clone();
            let mut tracker = Tracker::default();
            let mut warned = false;
            let mut next_health = tokio::time::Instant::now();
            let mut pressure_log = TransitionLog::default();
            let mut tick = tokio::time::interval(Duration::from_secs(2));
            tick.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
            loop {
                tokio::select! {
                    changed = changes.changed() => {
                        if changed.is_err() { break; }
                        current = changes.borrow_and_update().clone();
                        tracker = Tracker::default();
                        next_health = tokio::time::Instant::now();
                    }
                    _ = tick.tick() => {
                        if !current.v3 { continue; }
                        let due = tokio::time::Instant::now() >= next_health;
                        if due { next_health = tokio::time::Instant::now() + Duration::from_secs(health::POLL_SECS); }
                        let path = Path::new(&current.socket);
                        let (sample, health) = tokio::join!(read_snapshot(path), async {
                            if due { Some(read_health(path).await) } else { None }
                        });
                        let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_secs();
                        tracker.record(sample, now);
                        if let Some(health) = health { tracker.report.health.record(health, now); }
                        if let Some(message) = pressure_log.record(&tracker.report, now) {
                            eprintln!("{message}");
                        }
                        if let Err(error) = publish(Path::new(&current.state), &tracker.report) {
                            if !warned { eprintln!("worker telemetry cache: {error}"); }
                            warned = true;
                        } else { warned = false; }
                    }
                }
            }
        });
        Self { settings, task }
    }

    pub fn reconfigure(&self, cfg: &crate::config::Config) {
        let next = Settings::from_config(cfg);
        self.settings.send_if_modified(|old| {
            if *old == next {
                false
            } else {
                *old = next;
                true
            }
        });
    }
}

impl Drop for Monitor {
    fn drop(&mut self) {
        self.task.abort();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn dashboard_actual_rates_need_two_samples_and_rebase_after_failure() {
        let mut tracker = Tracker::default();
        let traffic = |at, down, up| {
            serde_json::from_value(json!({
            "captured_at_millis":at,"workers":[{"queue_id":0,"busy_ppm":125000}],
            "actual":{"download_bytes":down,"upload_bytes":up}}))
            .unwrap()
        };
        tracker.record(Ok(traffic(1000, 0, 0)), 10);
        tracker.record(Ok(traffic(3000, 2000000, 250000)), 12);
        let report = serde_json::to_value(&tracker.report).unwrap();
        assert_eq!(report["traffic"][1]["down_mbps"], 8.0);
        assert_eq!(report["traffic"][1]["up_mbps"], 1.0);
        tracker.record(Err(anyhow::anyhow!("offline")), 13);
        tracker.record(Ok(traffic(5000, 3000000, 500000)), 14);
        let report = serde_json::to_value(&tracker.report).unwrap();
        assert!(report["traffic"].as_array().unwrap().last().unwrap()["down_mbps"].is_null());
    }

    #[test]
    fn bridge_health_survives_independent_snapshot_failures_and_public_cache_roundtrip() {
        let mut tracker = Tracker::default();
        let report = |up| {
            serde_json::from_value(
                json!({"status":"healthy", "version":"v3", "uptime_ms":up,"workers":[]}),
            )
            .unwrap()
        };
        tracker.report.health.record(Ok(report(1)), 10);
        tracker.report.health.record(Ok(report(10001)), 20);
        tracker.record(Ok(sample(1000)), 20);
        tracker.record(Ok(sample(3000)), 22);
        assert!(tracker.report.health.live(22) && tracker.report.live(22));
        tracker
            .report
            .health
            .record(Err(anyhow::anyhow!("health unavailable")), 23);
        assert!(tracker.report.live(23) && !tracker.report.health.live(23));
        tracker.report.health.record(Ok(report(20001)), 30);
        tracker.record(Err(anyhow::anyhow!("snapshot unavailable")), 30);
        assert!(tracker.report.health.live(30) && !tracker.report.live(30));
        let dir = tempfile::tempdir().unwrap();
        publish(dir.path(), &tracker.report).unwrap();
        let r = read_cache(&dir.path().join(CACHE));
        assert!(r.health.live(30));
        assert_eq!(r.health.summary.as_ref().unwrap().uptime_ms, 20001);
        let mut value = serde_json::to_value(&tracker.report).unwrap();
        value.as_object_mut().unwrap().remove("health");
        std::fs::write(dir.path().join(CACHE), serde_json::to_vec(&value).unwrap()).unwrap();
        let r = read_cache(&dir.path().join(CACHE));
        assert!(r.health.summary.is_none());
        assert_eq!(r.workers.len(), 1);
        value["health"] = serde_json::to_value(&tracker.report.health).unwrap();
        value["health"]["summary"]["status"] = json!("x".repeat(33));
        std::fs::write(dir.path().join(CACHE), serde_json::to_vec(&value).unwrap()).unwrap();
        let r = read_cache(&dir.path().join(CACHE));
        assert!(r.health.summary.is_none());
        assert_eq!(r.workers.len(), 1);
    }

    #[test]
    fn extended_worker_cache_budget_preserves_basic_rows_and_histories_without_mutating_source() {
        let dir = tempfile::tempdir().unwrap();
        let mut report = Telemetry {
            updated: 3601,
            stale: false,
            workers: (0..4096)
                .map(|queue_id| WorkerRow {
                    queue_id,
                    busy_ppm: 1000000,
                    backlog_mean: Some(u64::MAX),
                    idle_ppm: Some(0),
                    laps_per_s: Some(u64::MAX),
                    busy_laps: Some(u64::MAX),
                    cycles_per_busy_lap: Some(u64::MAX),
                    phase_cycles: Some(vec![u64::MAX; 10]),
                })
                .collect(),
            ..Telemetry::default()
        };
        let history: std::collections::VecDeque<_> = (0..traffic::MAX_POINTS)
            .map(|i| traffic::Point {
                at: 1 + i as u64 * 2,
                down_mbps: Some(f64::MAX),
                up_mbps: Some(f64::MAX),
            })
            .collect();
        report.traffic = history.clone();
        report.classification.mapped.history = history.clone();
        report.classification.unmapped.history = history;
        report.reason = Some("source warning".into());
        report.health = serde_json::from_value(json!({"updated":3601,"stale":false,"summary":{
            "status":"degraded","version":"v".repeat(128),"uptime_ms":u64::MAX,
            "topology_tier":"t".repeat(64),"classification_empty":true,
            "degraded_subsystems":vec!["s".repeat(64);32],"worker_count":4096,
            "missing_rollups":0,"max_rollup_age_ms":u64::MAX,"aqm_degraded_workers":4096}}))
        .unwrap();
        assert!(serde_json::to_vec(&report).unwrap().len() as u64 > MAX_CACHE_BYTES);
        let start = std::time::Instant::now();
        publish(dir.path(), &report).unwrap();
        let path = dir.path().join(CACHE);
        assert!(std::fs::metadata(&path).unwrap().len() <= MAX_CACHE_BYTES);
        let r = read_cache(&path);
        println!(
            "Extended-worker cache fallback: {} bytes; publish/read {:?}",
            std::fs::metadata(&path).unwrap().len(),
            start.elapsed()
        );
        assert!(r.live(3601));
        assert!(r.health.live(3601));
        assert_eq!(
            r.health
                .summary
                .as_ref()
                .unwrap()
                .degraded_subsystems
                .as_ref()
                .unwrap()
                .len(),
            32
        );
        assert_eq!(r.workers.len(), 4096);
        assert_eq!(r.workers[0].busy_ppm, 1000000);
        assert!(r.workers[0].phase_cycles.is_none() && r.workers[0].laps_per_s.is_none());
        assert!(r.reason.as_deref().unwrap().contains("cache budget"));
        assert!(r.reason.as_deref().unwrap().contains("source warning"));
        assert_eq!(r.traffic.len(), traffic::MAX_POINTS);
        assert_eq!(r.classification.mapped.history.len(), traffic::MAX_POINTS);
        assert_eq!(r.classification.unmapped.history.len(), traffic::MAX_POINTS);
        assert!(report.workers[0].phase_cycles.is_some());
        report.workers.truncate(1);
        publish(dir.path(), &report).unwrap();
        let restored = read_cache(&path);
        assert!(restored.workers[0].phase_cycles.is_some());
        assert_eq!(restored.reason.as_deref(), Some("source warning"));
    }

    #[test]
    fn cache_budget_failure_keeps_the_previous_public_cache_unchanged() {
        let dir = tempfile::tempdir().unwrap();
        let mut report = Telemetry::default();
        publish(dir.path(), &report).unwrap();
        let path = dir.path().join(CACHE);
        let before = std::fs::read(&path).unwrap();
        report.reason = Some("x".repeat(MAX_CACHE_BYTES as usize));
        assert!(publish(dir.path(), &report).is_err());
        assert_eq!(std::fs::read(&path).unwrap(), before);
    }

    #[test]
    fn extended_worker_metrics_survive_advanced_samples_and_old_rows_remain_unknown() {
        let mut tracker = Tracker::default();
        let sample = |at| {
            serde_json::from_value(json!({"captured_at_millis":at,"workers":[{
            "queue_id":0,"busy_ppm":125000,"backlog_mean":3,"idle_ppm":875000,
            "laps_per_s":100000,"busy_laps":42,"cycles_per_busy_lap":250,
            "phase_cycles":[1,2,3,4,5,6,7,8,9,10]}]}))
            .unwrap()
        };
        tracker.record(Ok(sample(1000)), 10);
        tracker.record(Ok(sample(3000)), 12);
        let r = serde_json::to_value(&tracker.report).unwrap();
        let w = &r["workers"][0];
        assert_eq!(w["idle_ppm"], 875000);
        assert_eq!(w["laps_per_s"], 100000);
        assert_eq!(w["busy_laps"], 42);
        assert_eq!(w["cycles_per_busy_lap"], 250);
        assert_eq!(w["phase_cycles"].as_array().unwrap().len(), 10);
        tracker.record(
            Ok(
                serde_json::from_value(json!({"captured_at_millis":3000,"workers":[{
            "queue_id":0,"busy_ppm":125000,"laps_per_s":999999}]}))
                .unwrap(),
            ),
            14,
        );
        assert_eq!(
            serde_json::to_value(&tracker.report).unwrap()["workers"][0],
            w.clone()
        );
        tracker.record(Ok(super::tests::sample(5000)), 16);
        let old = serde_json::to_value(&tracker.report).unwrap();
        assert!(old["workers"][0]["idle_ppm"].is_null());
        assert!(old["workers"][0]["laps_per_s"].is_null());
        assert!(old["workers"][0]["phase_cycles"].is_null());
    }

    #[test]
    fn invalid_extended_metrics_do_not_hide_basic_worker_values() {
        let mut tracker = Tracker::default();
        let sample = |at| {
            serde_json::from_value(json!({"captured_at_millis":at,"workers":[{
            "queue_id":0,"busy_ppm":125000,"idle_ppm":1000000,
            "laps_per_s":0,"busy_laps":0,"cycles_per_busy_lap":0,"phase_cycles":[1,2]}]}))
            .unwrap()
        };
        tracker.record(Ok(sample(1000)), 10);
        tracker.record(Ok(sample(3000)), 12);
        let r = serde_json::to_value(&tracker.report).unwrap();
        assert!(tracker.report.live(12));
        assert_eq!(r["workers"][0]["busy_ppm"], 125000);
        assert_eq!(r["workers"][0]["laps_per_s"], 0);
        assert!(r["workers"][0]["idle_ppm"].is_null());
        assert!(r["workers"][0]["phase_cycles"].is_null());
    }

    fn class_sample(at: u64, mapped: Option<u64>, unmapped: Option<u64>) -> Snapshot {
        serde_json::from_value(json!({"captured_at_millis":at,
            "workers":[{"queue_id":0,"busy_ppm":125000}],
            "actual":{"download_bytes":0,"upload_bytes":0},
            "mapped":mapped.map(|n|json!({"download_bytes":n,"upload_bytes":n/2})),
            "unmapped":unmapped.map(|n|json!({"download_bytes":n,"upload_bytes":n/2})),
            "hosts":[{"actual":{"download_bytes":999999999999u64}}]}))
        .unwrap()
    }

    #[test]
    fn classified_traffic_uses_independent_source_totals_not_actual_or_capped_host_rows() {
        let mut t = Tracker::default();
        t.record(Ok(class_sample(1000, Some(0), Some(0))), 10);
        t.record(Ok(class_sample(3000, Some(2000000), Some(500000))), 12);
        let r = serde_json::to_value(&t.report).unwrap();
        assert_eq!(
            r["classification"]["mapped"]["bytes"]["download_bytes"],
            2000000
        );
        assert_eq!(
            r["classification"]["mapped"]["history"][1]["down_mbps"],
            8.0
        );
        assert_eq!(r["classification"]["mapped"]["history"][1]["up_mbps"], 4.0);
        assert_eq!(
            r["classification"]["unmapped"]["history"][1]["down_mbps"],
            2.0
        );
        assert_eq!(r["traffic"][1]["down_mbps"], 0.0);
        t.record(Ok(class_sample(3000, Some(9999999), Some(9999999))), 14);
        assert_eq!(
            serde_json::to_value(&t.report).unwrap()["classification"],
            r["classification"]
        );
        t.record(Ok(class_sample(5000, None, Some(1000000))), 16);
        let r = serde_json::to_value(&t.report).unwrap();
        assert!(r["classification"]["mapped"]["bytes"].is_null());
        assert!(r["classification"]["mapped"]["history"][2]["down_mbps"].is_null());
        assert_eq!(
            r["classification"]["unmapped"]["history"][2]["down_mbps"],
            2.0
        );
    }

    #[test]
    fn classified_traffic_rebases_after_failure_restart_counter_reset_and_gap() {
        let mut t = Tracker::default();
        t.record(Ok(class_sample(1000, Some(0), Some(0))), 10);
        t.record(Ok(class_sample(3000, Some(100), Some(100))), 12);
        t.record(Err(anyhow::anyhow!("offline")), 13);
        t.record(Ok(class_sample(3000, Some(100), Some(100))), 14);
        assert!(!t.report.live(14));
        t.record(Ok(class_sample(5000, Some(300), Some(300))), 16);
        let rate = |t: &Tracker, class: &str| {
            serde_json::to_value(&t.report).unwrap()["classification"][class]["history"]
                .as_array()
                .unwrap()
                .last()
                .unwrap()["down_mbps"]
                .clone()
        };
        assert!(rate(&t, "mapped").is_null());
        t.record(Ok(class_sample(7000, Some(1), Some(400))), 18);
        assert!(rate(&t, "mapped").is_null());
        assert!(rate(&t, "unmapped").as_f64().is_some());
        t.record(Ok(class_sample(15000, Some(500), Some(500))), 20);
        assert!(rate(&t, "mapped").is_null() && rate(&t, "unmapped").is_null());
        t.record(Ok(class_sample(100, Some(10), Some(10))), 22);
        assert!(!t.report.live(22));
        assert!(rate(&t, "mapped").is_null());
        t.record(Ok(class_sample(200, Some(20), Some(20))), 24);
        assert!(rate(&t, "mapped").as_f64().is_some());
        t.record(Ok(class_sample(300, Some(30), Some(30))), 23);
        assert!(rate(&t, "mapped").is_null());
    }

    fn queue_sample(at: u64, drops: u64, marks: u64, backlog: u64) -> Snapshot {
        serde_json::from_value(json!({"captured_at_millis":at,
            "workers":[{"queue_id":0,"busy_ppm":125000}],
            "qdisc_totals":{"drop_pkts":drops,"aqm_marks":marks,"backlog_bytes":backlog}}))
        .unwrap()
    }

    #[test]
    fn queue_totals_follow_producer_samples_and_keep_gauges_separate() {
        let mut t = Tracker::default();
        t.record(Ok(queue_sample(1000, 100, 200, 8192)), 10);
        assert!(!t.report.live(10));
        t.record(Ok(queue_sample(3000, 103, 205, 4096)), 12);
        let report = serde_json::to_value(&t.report).unwrap();
        assert_eq!(report["queue"]["totals"]["drop_pkts"], 103);
        assert_eq!(report["queue"]["drop_delta"], 3);
        assert_eq!(report["queue"]["aqm_delta"], 5);
        assert_eq!(report["queue"]["totals"]["backlog_bytes"], 4096);
        t.record(Ok(queue_sample(3000, 999, 999, 0)), 14);
        assert_eq!(
            serde_json::to_value(&t.report).unwrap()["queue"],
            report["queue"]
        );
        t.record(Ok(queue_sample(5000, 103, 205, 0)), 16);
        let q = serde_json::to_value(&t.report).unwrap()["queue"].clone();
        assert_eq!(q["drop_delta"], 0);
        assert_eq!(q["aqm_delta"], 0);
        assert_eq!(q["totals"]["backlog_bytes"], 0);
    }

    #[test]
    fn queue_totals_rebase_after_failure_gap_restart_missing_data_and_counter_reset() {
        let mut t = Tracker::default();
        t.record(Ok(queue_sample(1000, 100, 200, 1)), 10);
        t.record(Ok(queue_sample(3000, 103, 205, 2)), 12);
        t.record(Err(anyhow::anyhow!("disconnected")), 13);
        assert!(!t.report.live(13));
        t.record(Ok(queue_sample(5000, 500, 600, 3)), 14);
        let check_unknown = |t: &Tracker| {
            let q = serde_json::to_value(&t.report).unwrap()["queue"].clone();
            assert!(q["drop_delta"].is_null());
            assert!(q["aqm_delta"].is_null());
        };
        check_unknown(&t);
        t.record(Ok(queue_sample(7000, 800, 900, 4)), 30);
        check_unknown(&t);
        t.record(Ok(queue_sample(100, 1000, 1100, 5)), 31);
        assert!(!t.report.live(31));
        t.record(Ok(queue_sample(200, 1002, 1103, 6)), 33);
        assert_eq!(
            serde_json::to_value(&t.report).unwrap()["queue"]["drop_delta"],
            2
        );
        t.record(Ok(queue_sample(300, 1, 2, 7)), 35);
        check_unknown(&t);
        t.record(Ok(sample(400)), 37);
        assert!(serde_json::to_value(&t.report).unwrap()["queue"].is_null());
        t.record(Ok(queue_sample(500, 10, 20, 8)), 39);
        check_unknown(&t);
        t.record(Ok(queue_sample(600, 12, 23, 9)), 41);
        assert_eq!(
            serde_json::to_value(&t.report).unwrap()["queue"]["drop_delta"],
            2
        );
        t.record(Ok(queue_sample(700, 14, 25, 10)), 40);
        check_unknown(&t);
    }

    #[test]
    fn queue_cache_is_optional_and_preserves_partial_unknown_fields() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join(CACHE);
        let mut value = json!({"version":1,"updated":100,"stale":false,"reason":null,
            "workers":[{"queue_id":0,"busy_ppm":125000}]});
        std::fs::write(&path, value.to_string()).unwrap();
        assert!(read_cache(&path).live(100));
        value["queue"] = json!({"totals":{"backlog_bytes":10},"drop_delta":null,"aqm_delta":null});
        std::fs::write(&path, value.to_string()).unwrap();
        let r = read_cache(&path);
        assert!(r.live(100));
        let q = serde_json::to_value(r).unwrap()["queue"].clone();
        assert_eq!(q["totals"]["backlog_bytes"], 10);
        assert!(q["totals"]["drop_pkts"].is_null());
        value["queue"]["drop_delta"] = json!(1);
        std::fs::write(&path, value.to_string()).unwrap();
        let r = read_cache(&path);
        assert!(r.live(100), "queue validation must not hide worker data");
        assert!(serde_json::to_value(r).unwrap()["queue"].is_null());
    }

    fn sample(at: u64) -> Snapshot {
        serde_json::from_value(json!({"captured_at_millis":at,"workers":[{"queue_id":0,"busy_ppm":125000,"backlog_mean":3}]})).unwrap()
    }

    fn pressure_sample(at: u64, drops: u64, free: u64, rings_over: u64) -> Snapshot {
        serde_json::from_value(json!({"captured_at_millis":at,
            "workers":[{"queue_id":0,"busy_ppm":125000}],
            "diagnostics":{"rx_nombuf":drops,"pool_drop_packets":drops * 2,
                "pool_total_mbufs":1000,
                "pool_free_mbufs":free,"ring_count":2,"ring_capacity":200,
                "ring_used":50,"rings_over_75":rings_over}}))
        .unwrap()
    }

    #[test]
    fn pressure_tracks_new_drops_without_replaying_totals_or_repeated_frames() {
        let mut tracker = Tracker::default();
        tracker.record(Ok(pressure_sample(100, 100, 500, 0)), 10);
        tracker.record(Ok(pressure_sample(200, 102, 500, 0)), 12);
        let report = serde_json::to_value(&tracker.report).unwrap();
        assert_eq!(report["pressure"]["rx_nombuf_delta"], 2);
        assert_eq!(report["pressure"]["pool_drop_delta"], 4);
        tracker.record(Ok(pressure_sample(200, 102, 500, 0)), 14);
        assert_eq!(
            serde_json::to_value(&tracker.report).unwrap()["pressure"],
            report["pressure"]
        );
        tracker.record(Ok(pressure_sample(300, 102, 500, 0)), 16);
        assert_eq!(
            serde_json::to_value(&tracker.report).unwrap()["pressure"]["rx_nombuf_delta"],
            0
        );
        tracker.record(Ok(pressure_sample(400, 1, 500, 0)), 18);
        assert!(
            serde_json::to_value(&tracker.report).unwrap()["pressure"]["rx_nombuf_delta"].is_null()
        );
        tracker.record(Err(anyhow::anyhow!("disconnected")), 19);
        tracker.record(Ok(pressure_sample(500, 10, 500, 0)), 20);
        assert!(
            serde_json::to_value(&tracker.report).unwrap()["pressure"]["rx_nombuf_delta"].is_null()
        );
    }

    #[test]
    fn pressure_rebases_after_a_gap_or_producer_restart() {
        let mut tracker = Tracker::default();
        tracker.record(Ok(pressure_sample(100, 100, 500, 0)), 10);
        tracker.record(Ok(pressure_sample(200, 102, 500, 0)), 12);
        tracker.record(Ok(pressure_sample(300, 500, 500, 0)), 30);
        assert!(
            serde_json::to_value(&tracker.report).unwrap()["pressure"]["rx_nombuf_delta"].is_null(),
            "a gap needs a new counter baseline"
        );
        tracker.record(Ok(pressure_sample(50, 600, 500, 0)), 31);
        assert!(!tracker.report.live(31));
        tracker.record(Ok(pressure_sample(60, 602, 500, 0)), 33);
        assert_eq!(
            serde_json::to_value(&tracker.report).unwrap()["pressure"]["rx_nombuf_delta"],
            2
        );
    }

    #[test]
    fn pressure_cache_keeps_worker_data_with_old_or_invalid_diagnostics() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join(CACHE);
        let old = json!({"version":1,"updated":100,"stale":false,"reason":null,
            "workers":[{"queue_id":0,"busy_ppm":125000}]});
        std::fs::write(&path, old.to_string()).unwrap();
        let report = read_cache(&path);
        assert!(report.live(100) && report.pressure.is_none());
        let mut invalid = old;
        invalid["pressure"] = json!({"diagnostics":{"pool_free_mbufs":101,"pool_total_mbufs":100},
            "rx_nombuf_delta":0,"pool_drop_delta":0});
        std::fs::write(&path, invalid.to_string()).unwrap();
        let report = read_cache(&path);
        assert!(
            report.live(100),
            "invalid pressure must not hide valid worker values"
        );
        assert!(!report.pressure.unwrap().complete());
    }

    #[test]
    fn worker_util_needs_an_advancing_producer_and_expires() {
        let mut tracker = Tracker::default();
        tracker.record(Ok(sample(100)), 10);
        assert!(!tracker.report.live(10));
        tracker.record(Ok(sample(200)), 12);
        assert!(tracker.report.live(12));
        assert_eq!(tracker.report.workers[0].busy_ppm, 125000);
        tracker.record(Ok(sample(200)), 19);
        assert!(!tracker.report.live(19));
        assert_eq!(tracker.report.updated, 12);
        assert_eq!(tracker.report.workers[0].backlog_mean, Some(3));
        tracker.record(Ok(sample(300)), 20);
        assert!(tracker.report.live(20));
        assert!(!tracker.report.live(27));
        assert!(!tracker.report.live(19));
    }

    #[test]
    fn worker_util_failure_keeps_last_good_values_but_marks_them_stale() {
        let mut tracker = Tracker::default();
        tracker.record(Ok(sample(100)), 10);
        tracker.record(Ok(sample(200)), 12);
        tracker.record(Err(anyhow::anyhow!("socket unavailable")), 13);
        assert!(!tracker.report.live(13));
        assert_eq!(tracker.report.updated, 12);
        assert_eq!(tracker.report.workers[0].busy_ppm, 125000);
        tracker.record(Ok(sample(200)), 14);
        assert!(
            !tracker.report.live(14),
            "a repeated frame cannot clear a read failure"
        );
        tracker.record(Ok(sample(50)), 15);
        assert!(!tracker.report.live(15), "a restart needs a new baseline");
        tracker.record(Ok(sample(60)), 17);
        assert!(tracker.report.live(17));
    }

    #[test]
    fn worker_util_rejects_missing_busy_invalid_percentages_and_duplicate_workers() {
        assert!(
            serde_json::from_value::<Snapshot>(
                json!({"captured_at_millis":1,"workers":[{"queue_id":0}]})
            )
            .is_err()
        );
        let mut s = sample(1);
        s.workers[0].busy_ppm = 1000001;
        assert!(s.validate().is_err());
        let mut s = sample(1);
        s.workers.push(s.workers[0].clone());
        assert!(s.validate().is_err());
        let old: Snapshot = serde_json::from_value(json!({"captured_at_millis":1})).unwrap();
        assert!(old.validate().is_err());
    }

    #[test]
    fn worker_util_cache_is_public_bounded_and_rejects_corrupt_values() {
        use std::os::unix::fs::PermissionsExt;
        let dir = tempfile::tempdir().unwrap();
        let mut tracker = Tracker::default();
        tracker.record(Ok(sample(100)), 10);
        tracker.record(Ok(sample(200)), 12);
        publish(dir.path(), &tracker.report).unwrap();
        let path = dir.path().join(CACHE);
        assert!(read_cache(&path).live(12));
        assert_eq!(
            std::fs::metadata(&path).unwrap().permissions().mode() & 0o777,
            0o644
        );
        for text in [
            "broken",
            r#"{"version":2,"updated":12,"stale":false,"reason":null,"workers":[]}"#,
            r#"{"version":1,"updated":12,"stale":false,"reason":null,"workers":[{"queue_id":0,"busy_ppm":1000001}]}"#,
        ] {
            std::fs::write(&path, text).unwrap();
            let report = read_cache(&path);
            assert!(report.workers.is_empty() && !report.live(12));
        }
        std::fs::write(&path, vec![b' '; MAX_CACHE_BYTES as usize + 1]).unwrap();
        assert!(read_cache(&path).reason.unwrap().contains("too large"));
    }

    #[tokio::test(start_paused = true)]
    async fn bridge_health_monitor_requests_every_ten_seconds_and_keeps_snapshot_cadence() {
        use tokio::io::AsyncReadExt;
        let dir = tempfile::tempdir().unwrap();
        let socket = dir.path().join("analytics.sock");
        let listener = tokio::net::UnixListener::bind(&socket).unwrap();
        let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
        let server = tokio::spawn(async move {
            loop {
                let (mut stream, _) = listener.accept().await.unwrap();
                let mut head = [0; 4];
                stream.read_exact(&mut head).await.unwrap();
                let mut bytes = vec![0; u32::from_be_bytes(head) as usize];
                stream.read_exact(&mut bytes).await.unwrap();
                let v: serde_json::Value = ciborium::from_reader(bytes.as_slice()).unwrap();
                tx.send(v["kind"].as_str().unwrap().to_string()).unwrap();
                // Closing without a reply also verifies retry cadence after source failures.
            }
        });
        let mut cfg = crate::config::defaults().clone();
        cfg.libreqos.target = crate::config::Target::V3Pair;
        cfg.libreqos.v3_analytics_socket = socket.display().to_string();
        cfg.runtime.state_dir = dir.path().display().to_string();
        let monitor = Monitor::start(&cfg);
        for n in 0..=5 {
            if n > 0 {
                tokio::time::advance(Duration::from_secs(2)).await;
            }
            for _ in 0..50 {
                tokio::task::yield_now().await;
            }
            let mut kinds = Vec::new();
            while let Ok(kind) = rx.try_recv() {
                kinds.push(kind);
            }
            kinds.sort();
            assert_eq!(
                kinds,
                if n == 0 || n == 5 {
                    vec!["health", "snapshot"]
                } else {
                    vec!["snapshot"]
                },
                "tick {n}"
            );
        }
        drop(monitor);
        server.abort();
    }

    #[tokio::test(start_paused = true)]
    async fn worker_util_monitor_reloads_its_destination_and_stops_on_drop() {
        let first = tempfile::tempdir().unwrap();
        let second = tempfile::tempdir().unwrap();
        let mut cfg = crate::config::defaults().clone();
        cfg.libreqos.target = crate::config::Target::V3Pair;
        cfg.libreqos.v3_analytics_socket = first.path().join("missing.sock").display().to_string();
        cfg.runtime.state_dir = first.path().display().to_string();
        let monitor = Monitor::start(&cfg);
        for _ in 0..50 {
            tokio::task::yield_now().await;
        }
        assert!(first.path().join(CACHE).exists());
        cfg.runtime.state_dir = second.path().display().to_string();
        monitor.reconfigure(&cfg);
        for _ in 0..10 {
            tokio::task::yield_now().await;
        }
        tokio::time::advance(Duration::from_secs(2)).await;
        for _ in 0..50 {
            tokio::task::yield_now().await;
        }
        assert!(second.path().join(CACHE).exists());
        drop(monitor);
        std::fs::write(second.path().join(CACHE), "stopped").unwrap();
        tokio::time::advance(Duration::from_secs(10)).await;
        for _ in 0..10 {
            tokio::task::yield_now().await;
        }
        assert_eq!(
            std::fs::read_to_string(second.path().join(CACHE)).unwrap(),
            "stopped"
        );
    }
}
