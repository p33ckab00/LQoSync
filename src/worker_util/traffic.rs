//! Delivered whole-bridge byte counters. Never derive totals from capped host rows.
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;

pub const MAX_POINTS: usize = 1801;

/// Format observed rates without changing the stored Mbps value.
pub fn format_rate(n: Option<f64>, precise: bool) -> String {
    match n.filter(|n| n.is_finite() && *n >= 0.0) {
        None => "unknown".into(),
        Some(n) if n > 0.0 && n < 0.001 => format!("{:.1} bps", n * 1_000_000.0),
        Some(n) if n > 0.0 && n < 1.0 => format!("{:.1} kbps", n * 1000.0),
        Some(n) if n >= 1000.0 => format!("{:.3} Gbps", n / 1000.0),
        Some(n) if precise => format!("{n:.3} Mbps"),
        Some(n) => format!("{n:.1} Mbps"),
    }
}
#[derive(Debug, Clone, Copy, Deserialize, Serialize)]
pub struct Counters {
    pub download_bytes: u64,
    pub upload_bytes: u64,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Point {
    pub at: u64,
    pub down_mbps: Option<f64>,
    pub up_mbps: Option<f64>,
}

impl Point {
    pub fn valid(&self) -> bool {
        self.at > 0
            && [self.down_mbps, self.up_mbps]
                .into_iter()
                .flatten()
                .all(|n| n.is_finite() && n >= 0.0)
    }
}

#[derive(Default)]
pub(super) struct History {
    baseline: Option<(u64, u64, Counters)>,
}

impl History {
    pub fn record(
        &mut self,
        points: &mut VecDeque<Point>,
        sample: Option<(u64, Counters)>,
        now: u64,
    ) {
        // Wall-clock changes must not leave future points before a new sample or gap.
        if points.back().is_some_and(|p| p.at > now) {
            points.clear();
            self.baseline = None;
        }
        let mut point = Point {
            at: now,
            down_mbps: None,
            up_mbps: None,
        };
        if let Some((at, counters)) = sample {
            if let Some((prev_at, prev_wall, prev)) = self.baseline {
                if at == prev_at && now >= prev_wall && now - prev_wall <= super::MAX_AGE {
                    return;
                }
                if at < prev_at || now < prev_wall {
                    points.clear();
                }
                if at > prev_at
                    && at - prev_at <= super::MAX_AGE * 1000
                    && now >= prev_wall
                    && now - prev_wall <= super::MAX_AGE
                {
                    let rate = |n: u64, p: u64| {
                        n.checked_sub(p)
                            .map(|delta| delta as f64 * 0.008 / (at - prev_at) as f64)
                    };
                    point.down_mbps = rate(counters.download_bytes, prev.download_bytes);
                    point.up_mbps = rate(counters.upload_bytes, prev.upload_bytes);
                    // Either direction resetting invalidates the whole interval.
                    if point.down_mbps.is_none() || point.up_mbps.is_none() {
                        point.down_mbps = None;
                        point.up_mbps = None;
                    }
                }
            }
            self.baseline = Some((at, now, counters));
        } else {
            self.baseline = None;
        }
        if points.back().is_some_and(|p| p.at >= now) {
            points.pop_back();
        }
        points.push_back(point);
        while points.len() > MAX_POINTS
            || points
                .front()
                .is_some_and(|p| now.saturating_sub(p.at) > 3600)
        {
            points.pop_front();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn wall_clock_regression_during_read_failure_resets_history() {
        let mut h = History::default();
        let mut points = VecDeque::new();
        for now in [10, 12, 14] {
            h.record(
                &mut points,
                Some((
                    now * 1000,
                    Counters {
                        download_bytes: now * 1000,
                        upload_bytes: now * 1000,
                    },
                )),
                now,
            );
        }
        h.record(&mut points, None, 8);
        assert_eq!(points.len(), 1);
        assert_eq!(points.front().unwrap().at, 8);
        assert!(points.front().unwrap().down_mbps.is_none());
        h.record(
            &mut points,
            Some((
                16000,
                Counters {
                    download_bytes: 16000,
                    upload_bytes: 16000,
                },
            )),
            10,
        );
        assert!(points.back().unwrap().down_mbps.is_none());
        assert!(
            points
                .iter()
                .zip(points.iter().skip(1))
                .all(|(a, b)| a.at < b.at)
        );
    }

    #[test]
    fn repeated_frames_resets_gaps_and_retention() {
        let mut h = History::default();
        let mut p = VecDeque::new();
        let mut record = |h: &mut History, at, n, now| {
            h.record(
                &mut p,
                Some((
                    at,
                    Counters {
                        download_bytes: n,
                        upload_bytes: n,
                    },
                )),
                now,
            )
        };
        record(&mut h, 1000, 0, 1);
        record(&mut h, 3000, 250000, 3);
        record(&mut h, 3000, 250000, 4);
        assert_eq!(p.len(), 2);
        assert_eq!(p.back().unwrap().down_mbps, Some(1.0));
        h.record(
            &mut p,
            Some((
                5000,
                Counters {
                    download_bytes: 1,
                    upload_bytes: 1,
                },
            )),
            5,
        );
        assert!(p.back().unwrap().down_mbps.is_none());
        h.record(
            &mut p,
            Some((
                13000,
                Counters {
                    download_bytes: 1000000,
                    upload_bytes: 1000000,
                },
            )),
            13,
        );
        assert!(p.back().unwrap().down_mbps.is_none());
        for at in 14..5000 {
            h.record(&mut p, None, at);
        }
        assert!(p.len() <= MAX_POINTS);
        assert!(p.front().unwrap().at >= 1399);
    }
}
