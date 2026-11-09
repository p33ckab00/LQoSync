//! Optional buffer diagnostics. Counters describe increments between fresh snapshots.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct Diagnostics {
    pub rx_nombuf: Option<u64>,
    pub pool_drop_packets: Option<u64>,
    pub pool_total_mbufs: Option<u64>,
    pub pool_free_mbufs: Option<u64>,
    pub ring_count: Option<u64>,
    pub ring_capacity: Option<u64>,
    pub ring_used: Option<u64>,
    pub rings_over_75: Option<u64>,
}

impl Diagnostics {
    fn valid(&self) -> bool {
        [
            (self.pool_free_mbufs, self.pool_total_mbufs),
            (self.ring_used, self.ring_capacity),
            (self.rings_over_75, self.ring_count),
        ]
        .into_iter()
        .all(|(used, total)| used.zip(total).is_none_or(|(used, total)| used <= total))
    }

    fn pool(&self) -> Option<(u64, u64)> {
        self.pool_free_mbufs
            .zip(self.pool_total_mbufs)
            .filter(|&(_, total)| total > 0)
    }

    fn ring_known(&self) -> bool {
        self.ring().is_some()
            || (self.ring_count == Some(0)
                && self.ring_capacity == Some(0)
                && self.ring_used == Some(0)
                && self.rings_over_75 == Some(0))
    }

    fn ring(&self) -> Option<(u64, u64)> {
        self.ring_used
            .zip(self.ring_capacity)
            .filter(|&(_, total)| total > 0)
    }
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct Pressure {
    pub diagnostics: Option<Diagnostics>,
    pub rx_nombuf_delta: Option<u64>,
    pub pool_drop_delta: Option<u64>,
}

impl Pressure {
    pub(super) fn between(current: Option<&Diagnostics>, previous: Option<&Diagnostics>) -> Self {
        let Some(current) = current.filter(|d| d.valid()) else {
            return Self::default();
        };
        let delta = |value: Option<u64>, old: Option<u64>| {
            value.zip(old).and_then(|(n, p)| n.checked_sub(p))
        };
        let previous = previous.filter(|d| d.valid());
        Self {
            diagnostics: Some(current.clone()),
            rx_nombuf_delta: delta(current.rx_nombuf, previous.and_then(|p| p.rx_nombuf)),
            pool_drop_delta: delta(
                current.pool_drop_packets,
                previous.and_then(|p| p.pool_drop_packets),
            ),
        }
    }

    pub(super) fn valid(&self) -> bool {
        self.diagnostics.as_ref().is_none_or(|d| d.valid())
    }

    pub fn alerts(&self) -> Vec<&'static str> {
        let mut alerts = Vec::new();
        for (value, label) in [
            (self.rx_nombuf_delta, "RX buffer exhaustion"),
            (self.pool_drop_delta, "pool pressure drops"),
        ] {
            if value.is_some_and(|n| n > 0) {
                alerts.push(label);
            }
        }
        if let Some(d) = &self.diagnostics {
            // u128 keeps comparisons exact even with u64 counters at their limit.
            if d.pool()
                .is_some_and(|(free, total)| u128::from(free) * 10 <= u128::from(total))
            {
                alerts.push("pool free <=10%");
            }
            if d.rings_over_75.is_some_and(|n| n > 0)
                || d.ring().is_some_and(|(used, capacity)| {
                    u128::from(used) * 4 >= u128::from(capacity) * 3
                })
            {
                alerts.push("handoff ring pressure >=75%");
            }
        }
        alerts
    }

    pub fn complete(&self) -> bool {
        self.diagnostics
            .as_ref()
            .is_some_and(|d| d.pool().is_some() && d.ring_known() && d.rings_over_75.is_some())
            && [self.rx_nombuf_delta, self.pool_drop_delta]
                .iter()
                .all(Option::is_some)
    }

    pub fn measurements(&self) -> Vec<String> {
        let number = |n: Option<u64>| n.map_or("unknown".into(), |n| n.to_string());
        let ratio = |pair: Option<(u64, u64)>| {
            pair.map_or("unknown".into(), |(n, total)| {
                format!("{n}/{total} ({:.1}%)", n as f64 * 100.0 / total as f64)
            })
        };
        let d = self.diagnostics.as_ref();
        vec![
            format!(
                "Pool free: {} | Ring used: {}",
                ratio(d.and_then(Diagnostics::pool)),
                if d.is_some_and(|d| d.ring_count == Some(0) && d.ring_known()) {
                    "no rings".into()
                } else {
                    ratio(d.and_then(Diagnostics::ring))
                }
            ),
            format!(
                "Rings >75%: {} | New drops: RX {} / pool {}",
                number(d.and_then(|d| d.rings_over_75)),
                number(self.rx_nombuf_delta),
                number(self.pool_drop_delta)
            ),
        ]
    }
}

/// Log transitions only. A read failure is unavailable, never recovery.
#[derive(Default)]
pub(super) struct TransitionLog(Option<String>);

impl TransitionLog {
    pub fn record(&mut self, report: &super::Telemetry, now: u64) -> Option<String> {
        let state = if !report.live(now) {
            "unavailable (stale telemetry)".to_owned()
        } else if let Some(p) = &report.pressure {
            let alerts = p.alerts();
            if !alerts.is_empty() {
                format!("ALERT: {}", alerts.join(", "))
            } else if p.complete() {
                "clear".into()
            } else {
                "unavailable (missing diagnostics or counter baseline)".into()
            }
        } else {
            "unavailable (missing diagnostics)".into()
        };
        if self.0.as_ref() == Some(&state) {
            return None;
        }
        self.0 = Some(state.clone());
        Some(format!("buffer/ring pressure: {state}"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn diagnostics() -> Diagnostics {
        serde_json::from_value(serde_json::json!({"rx_nombuf":100,"pool_drop_packets":200,
            "pool_total_mbufs":1000,"pool_free_mbufs":500,"ring_count":2,
            "ring_capacity":200,"ring_used":50,"rings_over_75":0}))
        .unwrap()
    }

    #[test]
    fn pressure_gauges_detect_local_ring_pressure_and_exact_boundaries_without_overflow() {
        let base = diagnostics();
        for (free, used, over, alerts) in [
            (101, 149, 0, 0),
            (100, 149, 0, 1),
            (101, 150, 0, 1),
            (101, 10, 1, 1),
            (0, 200, 2, 2),
        ] {
            let mut d = base.clone();
            d.pool_free_mbufs = Some(free);
            d.ring_used = Some(used);
            d.rings_over_75 = Some(over);
            assert_eq!(
                Pressure::between(Some(&d), Some(&base)).alerts().len(),
                alerts
            );
        }
        let mut d = base.clone();
        d.pool_total_mbufs = Some(u64::MAX);
        d.pool_free_mbufs = Some(u64::MAX);
        d.ring_capacity = Some(u64::MAX);
        d.ring_used = Some(u64::MAX);
        assert_eq!(
            Pressure::between(Some(&d), Some(&base)).alerts(),
            ["handoff ring pressure >=75%"]
        );
    }

    #[test]
    fn pressure_missing_invalid_and_zero_capacity_are_not_healthy_zeros() {
        let base = diagnostics();
        assert!(!Pressure::between(None, Some(&base)).complete());
        let mut d = base.clone();
        d.pool_free_mbufs = Some(1001);
        let p = Pressure::between(Some(&d), Some(&base));
        assert!(!p.complete() && p.alerts().is_empty());
        let mut d = base.clone();
        d.rings_over_75 = Some(3);
        assert!(!Pressure::between(Some(&d), Some(&base)).complete());
        let mut d = base.clone();
        d.pool_total_mbufs = Some(0);
        d.pool_free_mbufs = Some(0);
        assert!(!Pressure::between(Some(&d), Some(&base)).complete());
        let mut d = base.clone();
        d.rx_nombuf = None;
        let p = Pressure::between(Some(&d), Some(&base));
        assert!(!p.complete() && p.rx_nombuf_delta.is_none());
    }

    #[test]
    fn pressure_zero_rings_is_a_valid_single_worker_topology() {
        let mut d = diagnostics();
        d.ring_count = Some(0);
        d.ring_capacity = Some(0);
        d.ring_used = Some(0);
        let p = Pressure::between(Some(&d), Some(&d));
        assert!(p.complete());
        assert!(p.alerts().is_empty());
        assert!(p.measurements().join(" ").contains("no rings"));
    }

    #[test]
    fn pressure_journal_deduplicates_causes_and_does_not_call_a_failure_recovery() {
        let base = diagnostics();
        let mut d = base.clone();
        d.pool_drop_packets = Some(201);
        let mut report = super::super::Telemetry {
            updated: 100,
            stale: false,
            pressure: Some(Pressure::between(Some(&d), Some(&base))),
            ..Default::default()
        };
        let mut log = TransitionLog::default();
        assert!(log.record(&report, 100).unwrap().contains("ALERT"));
        d.pool_drop_packets = Some(300);
        report.pressure = Some(Pressure::between(Some(&d), Some(&base)));
        assert!(log.record(&report, 101).is_none());
        assert!(log.record(&report, 107).unwrap().contains("unavailable"));
        assert!(log.record(&report, 108).is_none());
        report.updated = 109;
        report.pressure = Some(Pressure::between(Some(&d), Some(&d)));
        assert!(log.record(&report, 109).unwrap().ends_with("clear"));
    }
}
