//! Independent read-only bridge health. Snapshot freshness is not a health verdict.
use anyhow::ensure;
use serde::{Deserialize, Serialize};

pub const POLL_SECS: u64 = 10;
pub const MAX_AGE: u64 = 15;

#[derive(Debug, Deserialize)]
pub struct Report {
    pub status: String,
    pub version: String,
    pub uptime_ms: u64,
    pub workers: Vec<Worker>,
    pub classification_empty: Option<bool>,
    pub topology_tier: Option<String>,
    pub degraded_subsystems: Option<Vec<String>>,
}

#[derive(Debug, Deserialize)]
pub struct Worker {
    pub queue_id: u16,
    pub rollup_age_ms: Option<u64>,
    pub deadline_aqm_degraded: Option<bool>,
}

impl Report {
    pub(super) fn validate(&self) -> anyhow::Result<()> {
        ensure!(self.workers.len() <= 4096, "too many health workers");
        let mut ids = std::collections::HashSet::new();
        ensure!(
            self.workers.iter().all(|w| ids.insert(w.queue_id)),
            "duplicate health worker"
        );
        ensure!(
            text_valid(&self.status, 32) && text_valid(&self.version, 128),
            "invalid health text"
        );
        ensure!(
            self.topology_tier
                .as_ref()
                .is_none_or(|v| text_valid(v, 64)),
            "invalid topology tier"
        );
        ensure!(
            self.degraded_subsystems
                .as_ref()
                .is_none_or(|v| v.len() <= 32 && v.iter().all(|s| text_valid(s, 64))),
            "invalid degraded subsystems"
        );
        Ok(())
    }
}

fn text_valid(s: &str, max: usize) -> bool {
    s.len() <= max && !s.chars().any(char::is_control)
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(default)]
pub struct Health {
    pub updated: u64,
    pub stale: bool,
    pub reason: Option<String>,
    pub summary: Option<Summary>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Summary {
    pub status: String,
    pub version: String,
    pub uptime_ms: u64,
    pub topology_tier: Option<String>,
    pub classification_empty: Option<bool>,
    pub degraded_subsystems: Option<Vec<String>>,
    pub worker_count: usize,
    pub missing_rollups: usize,
    pub max_rollup_age_ms: Option<u64>,
    pub aqm_degraded_workers: Option<usize>,
}

impl Health {
    pub fn live(&self, now: u64) -> bool {
        !self.stale
            && self.summary.is_some()
            && self.updated > 0
            && now >= self.updated
            && now - self.updated <= MAX_AGE
    }

    pub(super) fn valid(&self) -> bool {
        self.reason.as_ref().is_none_or(|v| text_valid(v, 180))
            && self.summary.as_ref().is_none_or(|s| {
                text_valid(&s.status, 32)
                    && text_valid(&s.version, 128)
                    && s.topology_tier.as_ref().is_none_or(|v| text_valid(v, 64))
                    && s.degraded_subsystems
                        .as_ref()
                        .is_none_or(|v| v.len() <= 32 && v.iter().all(|v| text_valid(v, 64)))
                    && s.worker_count <= 4096
                    && s.missing_rollups <= s.worker_count
                    && s.aqm_degraded_workers.is_none_or(|v| v <= s.worker_count)
            })
    }

    pub fn record(&mut self, report: anyhow::Result<Report>, now: u64) {
        match report.and_then(|r| {
            r.validate()?;
            Ok(r)
        }) {
            Ok(r) => {
                let advanced = self
                    .summary
                    .as_ref()
                    .is_some_and(|s| r.uptime_ms > s.uptime_ms);
                let reset = self
                    .summary
                    .as_ref()
                    .is_none_or(|s| r.uptime_ms < s.uptime_ms);
                if advanced || reset {
                    let aqm = (!r.workers.is_empty()
                        && r.workers.iter().all(|w| w.deadline_aqm_degraded.is_some()))
                    .then(|| {
                        r.workers
                            .iter()
                            .filter(|w| w.deadline_aqm_degraded == Some(true))
                            .count()
                    });
                    self.summary = Some(Summary {
                        status: r.status,
                        version: r.version,
                        uptime_ms: r.uptime_ms,
                        topology_tier: r.topology_tier,
                        classification_empty: r.classification_empty,
                        degraded_subsystems: r.degraded_subsystems,
                        worker_count: r.workers.len(),
                        missing_rollups: r
                            .workers
                            .iter()
                            .filter(|w| w.rollup_age_ms.is_none())
                            .count(),
                        max_rollup_age_ms: r.workers.iter().filter_map(|w| w.rollup_age_ms).max(),
                        aqm_degraded_workers: aqm,
                    });
                    self.updated = now;
                }
                self.stale = !advanced;
                self.reason = (!advanced).then(|| {
                    if reset {
                        "waiting for next health uptime".into()
                    } else {
                        "health uptime has not advanced".into()
                    }
                });
            }
            Err(error) => {
                self.stale = true;
                let mut reason: String = error
                    .to_string()
                    .chars()
                    .filter(|c| !c.is_control())
                    .take(180)
                    .collect();
                while reason.len() > 180 {
                    reason.pop();
                }
                self.reason = Some(reason);
            }
        }
    }

    pub fn measurements(&self, now: u64) -> Vec<String> {
        let Some(s) = &self.summary else {
            return vec![
                "Bridge health unavailable".into(),
                format!(
                    "Health note: {}",
                    self.reason.as_deref().unwrap_or("no health sample yet")
                ),
            ];
        };
        let age = now
            .checked_sub(self.updated)
            .map_or("unknown".into(), |age| format!("{age}s"));
        let number = |v: Option<usize>| v.map_or("unknown".into(), |n| n.to_string());
        let subsystems = s
            .degraded_subsystems
            .as_ref()
            .map_or("unknown".into(), |v| {
                if v.is_empty() {
                    "none reported".into()
                } else {
                    v.join(", ")
                }
            });
        let mut lines = vec![
            format!(
                "Bridge health {} | age {age}",
                if self.live(now) {
                    "LIVE"
                } else {
                    "STALE (last known)"
                }
            ),
            format!(
                "Status: {}",
                if s.status.is_empty() {
                    "unknown"
                } else {
                    &s.status
                }
            ),
            format!(
                "Bridge version: {}",
                if s.version.is_empty() {
                    "unknown"
                } else {
                    &s.version
                }
            ),
            format!(
                "Reported uptime: {}d {:02}:{:02}:{:02}",
                s.uptime_ms / 86400000,
                s.uptime_ms / 3600000 % 24,
                s.uptime_ms / 60000 % 60,
                s.uptime_ms / 1000 % 60
            ),
            format!(
                "Topology tier: {}",
                s.topology_tier
                    .as_deref()
                    .filter(|v| !v.is_empty())
                    .unwrap_or("unknown")
            ),
            format!(
                "Classification empty: {}",
                s.classification_empty
                    .map_or("unknown", |v| if v { "yes" } else { "no" })
            ),
            format!("Degraded subsystems: {subsystems}"),
            format!(
                "Health workers: {} | missing rollups: {}",
                s.worker_count, s.missing_rollups
            ),
            format!(
                "Max rollup age at read: {}",
                s.max_rollup_age_ms
                    .map_or("unknown".into(), |n| format!("{n} ms"))
            ),
            format!("AQM degraded workers: {}", number(s.aqm_degraded_workers)),
        ];
        if let Some(reason) = &self.reason {
            lines.push(format!("Health note: {reason}"));
        }
        lines
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn report(uptime: u64) -> Report {
        serde_json::from_value(serde_json::json!({"status":"degraded", "version":"v3", "uptime_ms":uptime,
          "workers":[{"queue_id":0,"rollup_age_ms":200,"deadline_aqm_degraded":true}],
          "classification_empty":true,"topology_tier":"bootstrap_trunk","degraded_subsystems":["stats_summary"]})).unwrap()
    }
    #[test]
    fn bridge_health_has_independent_progress_expiry_failure_and_restart_states() {
        let mut h = Health::default();
        assert!(!h.live(1));
        h.record(Ok(report(1000)), 10);
        assert!(!h.live(10));
        h.record(Ok(report(2000)), 20);
        assert!(h.live(20) && h.live(35) && !h.live(36) && !h.live(19));
        let text = h.measurements(20).join("\n");
        assert!(
            text.contains("Bridge health LIVE")
                && text.contains("Status: degraded")
                && text.contains("stats_summary")
        );
        assert!(
            text.contains("AQM degraded workers: 1") && text.contains("Classification empty: yes")
        );
        h.record(Ok(report(2000)), 30);
        assert!(!h.live(30));
        assert_eq!(h.updated, 20);
        assert!(h.measurements(30).join("\n").contains("STALE (last known)"));
        h.record(Err(anyhow::anyhow!("offline\n\x1b")), 31);
        assert!(!h.live(31));
        assert_eq!(h.reason.as_deref(), Some("offline"));
        h.record(Ok(report(3000)), 40);
        assert!(h.live(40));
        h.record(Ok(report(1)), 50);
        assert!(!h.live(50));
        assert_eq!(h.summary.as_ref().unwrap().uptime_ms, 1);
        h.record(Ok(report(10001)), 60);
        assert!(h.live(60));
    }
    #[test]
    fn bridge_health_old_fields_zero_and_unknown_status_are_not_healthy_defaults() {
        let mut h = Health::default();
        let old = |uptime| {
            serde_json::from_value(serde_json::json!({"status":"future_state","version":"", "uptime_ms":uptime,"workers":[]})).unwrap()
        };
        h.record(Ok(old(0)), 10);
        h.record(Ok(old(1)), 20);
        let text = h.measurements(20).join("\n");
        assert!(
            text.contains("Status: future_state") && text.contains("Degraded subsystems: unknown")
        );
        assert!(
            text.contains("Classification empty: unknown")
                && text.contains("AQM degraded workers: unknown")
        );
        assert!(!text.contains("Status: healthy"));
        let mut zero = report(2);
        zero.degraded_subsystems = Some(vec![]);
        zero.classification_empty = Some(false);
        zero.workers[0].deadline_aqm_degraded = Some(false);
        h.record(Ok(zero), 30);
        let text = h.measurements(30).join("\n");
        assert!(
            text.contains("Degraded subsystems: none reported")
                && text.contains("Classification empty: no")
                && text.contains("AQM degraded workers: 0")
        );
    }
    #[test]
    fn bridge_health_missing_worker_fields_and_unicode_errors_remain_bounded_unknowns() {
        let mut h = Health::default();
        let partial = |uptime| {
            serde_json::from_value(serde_json::json!({"status":"starting","version":"v3","uptime_ms":uptime,
            "workers":[{"queue_id":0},{"queue_id":1,"rollup_age_ms":0,"deadline_aqm_degraded":false}]})).unwrap()
        };
        h.record(Ok(partial(1)), 10);
        h.record(Ok(partial(10001)), 20);
        let s = h.summary.as_ref().unwrap();
        assert_eq!(s.missing_rollups, 1);
        assert_eq!(s.max_rollup_age_ms, Some(0));
        assert_eq!(s.aqm_degraded_workers, None);
        h.record(Err(anyhow::anyhow!("錯".repeat(200))), 30);
        assert!(h.valid() && !h.live(30));
        assert!(h.reason.as_ref().unwrap().len() <= 180);
        assert_eq!(h.summary.as_ref().unwrap().status, "starting");
    }

    #[test]
    fn bridge_health_rejects_unbounded_text_workers_and_control_sequences() {
        let mut r = report(1);
        r.status = "healthy\x1b".into();
        assert!(r.validate().is_err());
        r.status = "x".repeat(33);
        assert!(r.validate().is_err());
        r.status = "healthy".into();
        r.degraded_subsystems = Some(vec!["x".into(); 33]);
        assert!(r.validate().is_err());
        r.degraded_subsystems = None;
        r.workers.push(Worker {
            queue_id: 0,
            rollup_age_ms: None,
            deadline_aqm_degraded: None,
        });
        assert!(r.validate().is_err());
        r.workers = (0..4097)
            .map(|queue_id| Worker {
                queue_id,
                rollup_age_ms: None,
                deadline_aqm_degraded: None,
            })
            .collect();
        assert!(r.validate().is_err());
    }
}
