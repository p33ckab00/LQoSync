//! Read-only DPDK worker utilization for the service and TUI.

mod ipc;
pub mod pressure;
pub mod traffic;
pub use ipc::read_snapshot;
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

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
pub struct WorkerRow {
    pub queue_id: u16,
    pub busy_ppm: u64,
    pub backlog_mean: Option<u64>,
}

#[derive(Debug, Deserialize)]
pub struct Snapshot {
    pub captured_at_millis: u64,
    #[serde(default)]
    pub workers: Vec<WorkerRow>,
    pub diagnostics: Option<Diagnostics>,
    pub actual: Option<traffic::Counters>,
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
    traffic: traffic::History,
}

impl Tracker {
    fn record(&mut self, sample: anyhow::Result<Snapshot>, now: u64) {
        match sample.and_then(|sample| {
            sample.validate()?;
            Ok(sample)
        }) {
            Ok(mut sample) => {
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
                    self.previous_diagnostics = sample.diagnostics;
                    self.previous_at = now;
                    self.report
                        .fail("waiting for next producer snapshot".into());
                } else if self.captured != Some(sample.captured_at_millis) {
                    if now < self.previous_at || now - self.previous_at > MAX_AGE {
                        self.previous_diagnostics = None;
                    }
                    self.captured = Some(sample.captured_at_millis);
                    sample.workers.sort_unstable_by_key(|w| w.queue_id);
                    self.report = Telemetry {
                        version: 1,
                        updated: now,
                        stale: false,
                        reason: None,
                        workers: sample.workers,
                        traffic: std::mem::take(&mut self.report.traffic),
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
                self.traffic.record(&mut self.report.traffic, None, now);
                self.previous_diagnostics = None;
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
        if !report.workers.is_empty() {
            validate_rows(&report.workers)?;
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
    let tmp = dir.join(format!(".{CACHE}.tmp-{}", std::process::id()));
    std::fs::write(&tmp, serde_json::to_vec(report)?)?;
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

/// One poll task per service. The TUI reads its public, worker-only cache.
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
            let mut pressure_log = TransitionLog::default();
            let mut tick = tokio::time::interval(Duration::from_secs(2));
            tick.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
            loop {
                tokio::select! {
                    changed = changes.changed() => {
                        if changed.is_err() { break; }
                        current = changes.borrow_and_update().clone();
                        tracker = Tracker::default();
                    }
                    _ = tick.tick() => {
                        if !current.v3 { continue; }
                        let sample = read_snapshot(Path::new(&current.socket)).await;
                        let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_secs();
                        tracker.record(sample, now);
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
