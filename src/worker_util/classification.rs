//! Whole-bridge enqueue classes. Never infer these totals from delivered or capped host rows.
use super::traffic::{self, Counters, Point};
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(default)]
pub struct Series {
    pub bytes: Option<Counters>,
    pub history: VecDeque<Point>,
}

impl Series {
    pub fn current(&self, now: u64) -> Option<&Point> {
        self.bytes?;
        self.history.back().filter(|p| {
            p.down_mbps.is_some()
                && p.up_mbps.is_some()
                && now >= p.at
                && now - p.at <= super::MAX_AGE
        })
    }

    fn valid(&self) -> bool {
        self.history.len() <= traffic::MAX_POINTS
            && self.history.iter().all(Point::valid)
            && self
                .history
                .iter()
                .zip(self.history.iter().skip(1))
                .all(|(a, b)| a.at < b.at)
    }
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(default)]
pub struct Classification {
    pub mapped: Series,
    pub unmapped: Series,
}

impl Classification {
    pub(super) fn sanitize(&mut self) {
        for series in [&mut self.mapped, &mut self.unmapped] {
            if !series.valid() {
                *series = Series::default();
            }
        }
    }

    pub fn measurements(&self, live: bool, now: u64) -> Vec<String> {
        let mut lines = vec!["Classification | whole bridge | enqueued".into()];
        for (name, series) in [("Mapped", &self.mapped), ("Unmapped", &self.unmapped)] {
            let point = series.current(now).filter(|_| live);
            let state = if series.bytes.is_none() {
                "unavailable"
            } else if !live {
                "STALE (last known)"
            } else if point.is_some() {
                "LIVE"
            } else {
                "warming / gap"
            };
            let rate =
                |value: Option<f64>| value.map_or("unknown".into(), |n| format!("{n:.3} Mbps"));
            lines.push(format!(
                "{name} {state}: Down {} | Up {}",
                rate(point.and_then(|p| p.down_mbps)),
                rate(point.and_then(|p| p.up_mbps))
            ));
            let bytes = |value: Option<u64>| value.map_or("unknown".into(), |n| n.to_string());
            lines.push(format!(
                "{name} bytes D/U: {} / {}",
                bytes(series.bytes.map(|c| c.download_bytes)),
                bytes(series.bytes.map(|c| c.upload_bytes))
            ));
        }
        lines
    }
}

#[derive(Default)]
pub(super) struct Tracker {
    mapped: traffic::History,
    unmapped: traffic::History,
}

impl Tracker {
    pub fn record(
        &mut self,
        report: &mut Classification,
        at: u64,
        mapped: Option<Counters>,
        unmapped: Option<Counters>,
        now: u64,
    ) {
        for (history, series, counters) in [
            (&mut self.mapped, &mut report.mapped, mapped),
            (&mut self.unmapped, &mut report.unmapped, unmapped),
        ] {
            if counters.is_some() || !series.history.is_empty() {
                history.record(&mut series.history, counters.map(|c| (at, c)), now);
            }
            series.bytes = counters;
        }
    }

    pub fn fail(&mut self, report: &mut Classification, now: u64) {
        for (history, series) in [
            (&mut self.mapped, &mut report.mapped),
            (&mut self.unmapped, &mut report.unmapped),
        ] {
            if !series.history.is_empty() {
                history.record(&mut series.history, None, now);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::worker_util::{CACHE, Telemetry, WorkerRow, read_cache};
    use serde_json::json;

    #[test]
    fn classification_measurements_distinguish_zero_unknown_warming_and_stale() {
        let mut tracker = Tracker::default();
        let mut report = Classification::default();
        tracker.record(
            &mut report,
            1000,
            Some(Counters {
                download_bytes: 0,
                upload_bytes: 0,
            }),
            None,
            10,
        );
        let text = report.measurements(true, 10).join("\n");
        assert!(text.contains("Mapped warming / gap") && text.contains("Unmapped unavailable"));
        tracker.record(
            &mut report,
            3000,
            Some(Counters {
                download_bytes: 0,
                upload_bytes: 0,
            }),
            None,
            12,
        );
        let text = report.measurements(true, 12).join("\n");
        assert!(
            text.contains("Mapped LIVE: Down 0.000 Mbps")
                && text.contains("Mapped bytes D/U: 0 / 0")
        );
        let text = report.measurements(false, 12).join("\n");
        assert!(text.contains("STALE (last known): Down unknown") && !text.contains("0.000 Mbps"));
        assert!(report.mapped.current(19).is_none() && report.mapped.current(11).is_none());
    }

    #[test]
    fn classification_histories_remain_bounded_and_absent_sources_do_not_accumulate_empty_rows() {
        let mut tracker = Tracker::default();
        let mut report = Classification::default();
        for n in 1..3000 {
            tracker.record(
                &mut report,
                n * 2000,
                Some(Counters {
                    download_bytes: n * 1000,
                    upload_bytes: n * 500,
                }),
                None,
                n * 2,
            );
        }
        assert!(report.mapped.history.len() <= traffic::MAX_POINTS);
        assert!(report.mapped.history.front().unwrap().at >= 5998 - 3600);
        assert!(report.unmapped.history.is_empty());
        assert!(report.mapped.valid());
        tracker.fail(&mut report, 1);
        assert_eq!(report.mapped.history.len(), 1);
        assert!(report.mapped.history[0].down_mbps.is_none());
    }

    #[test]
    fn classification_cache_preserves_workers_and_valid_class_when_one_history_is_invalid() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join(CACHE);
        let mut value = json!({"version":1,"updated":100,"stale":false,
            "workers":[{"queue_id":0,"busy_ppm":125000}]});
        std::fs::write(&path, value.to_string()).unwrap();
        let old = read_cache(&path);
        assert!(old.live(100) && old.classification.mapped.history.is_empty());
        value["classification"] = json!({
            "mapped":{"bytes":{"download_bytes":1,"upload_bytes":1},"history":[{"at":100,"down_mbps":1.0,"up_mbps":1.0}]},
            "unmapped":{"bytes":{"download_bytes":1,"upload_bytes":1},"history":[{"at":100,"down_mbps":-1.0,"up_mbps":1.0}]}
        });
        for invalid in [
            json!([{"at":100,"down_mbps":-1.0,"up_mbps":1.0}]),
            json!([{"at":100,"down_mbps":1.0,"up_mbps":1.0},{"at":100,"down_mbps":1.0,"up_mbps":1.0}]),
            serde_json::to_value(vec![
                Point {
                    at: 100,
                    down_mbps: Some(1.0),
                    up_mbps: Some(1.0)
                };
                traffic::MAX_POINTS + 1
            ])
            .unwrap(),
        ] {
            value["classification"]["unmapped"]["history"] = invalid;
            std::fs::write(&path, value.to_string()).unwrap();
            let r = read_cache(&path);
            assert!(r.live(100));
            assert_eq!(r.classification.mapped.history.len(), 1);
            assert!(
                r.classification.unmapped.history.is_empty()
                    && r.classification.unmapped.bytes.is_none()
            );
        }
    }

    #[test]
    fn classification_maximum_histories_and_workers_fit_existing_cache_bound() {
        let history: VecDeque<_> = (0..traffic::MAX_POINTS)
            .map(|i| Point {
                at: 1 + i as u64 * 2,
                down_mbps: Some(f64::MAX),
                up_mbps: Some(f64::MAX),
            })
            .collect();
        let series = Series {
            bytes: Some(Counters {
                download_bytes: u64::MAX,
                upload_bytes: u64::MAX,
            }),
            history: history.clone(),
        };
        let r = Telemetry {
            updated: 3601,
            stale: false,
            reason: Some("界".repeat(180)),
            workers: (0..4096)
                .map(|id| WorkerRow {
                    queue_id: id,
                    busy_ppm: 1000000,
                    backlog_mean: Some(u64::MAX),
                })
                .collect(),
            traffic: history,
            classification: Classification {
                mapped: series.clone(),
                unmapped: series,
            },
            ..Telemetry::default()
        };
        let encode_start = std::time::Instant::now();
        let bytes = serde_json::to_vec(&r).unwrap();
        let encode_time = encode_start.elapsed();
        assert!(
            bytes.len() as u64 <= super::super::MAX_CACHE_BYTES,
            "{} bytes",
            bytes.len()
        );
        let dir = tempfile::tempdir().unwrap();
        super::super::publish(dir.path(), &r).unwrap();
        let decode_start = std::time::Instant::now();
        let read = read_cache(&dir.path().join(CACHE));
        println!(
            "Synthetic debug cache: {} bytes; encode {:?}; read/decode {:?}; 4096 workers and three 1801-point histories",
            bytes.len(),
            encode_time,
            decode_start.elapsed()
        );
        assert!(read.live(3601));
        assert_eq!(read.workers.len(), 4096);
        assert_eq!(
            read.classification.mapped.history.len(),
            traffic::MAX_POINTS
        );
        assert_eq!(
            read.classification.unmapped.history.len(),
            traffic::MAX_POINTS
        );
    }
}
