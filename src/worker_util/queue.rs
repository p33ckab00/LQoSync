//! Whole-bridge queue counters and the current backlog gauge.
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, Deserialize, Serialize, PartialEq, Eq)]
pub struct Totals {
    pub drop_pkts: Option<u64>,
    pub aqm_marks: Option<u64>,
    pub backlog_bytes: Option<u64>,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
pub struct Queue {
    pub totals: Totals,
    pub drop_delta: Option<u64>,
    pub aqm_delta: Option<u64>,
}

impl Queue {
    pub(super) fn between(current: Totals, previous: Option<&Totals>) -> Self {
        let delta = |value: Option<u64>, before: Option<u64>| {
            value
                .zip(before)
                .and_then(|(now, old)| now.checked_sub(old))
        };
        Self {
            drop_delta: delta(current.drop_pkts, previous.and_then(|p| p.drop_pkts)),
            aqm_delta: delta(current.aqm_marks, previous.and_then(|p| p.aqm_marks)),
            totals: current,
        }
    }

    pub(super) fn valid(&self) -> bool {
        [
            (self.drop_delta, self.totals.drop_pkts),
            (self.aqm_delta, self.totals.aqm_marks),
        ]
        .into_iter()
        .all(|(delta, total)| delta.is_none_or(|d| total.is_some_and(|t| d <= t)))
    }

    pub fn measurements(queue: Option<&Self>, live: bool) -> Vec<String> {
        let number = |n: Option<u64>| n.map_or("unknown".into(), |v| v.to_string());
        let totals = queue.map(|q| &q.totals);
        vec![
            format!(
                "Queues {} | whole bridge",
                if queue.is_none() {
                    "unavailable"
                } else if live {
                    "LIVE"
                } else {
                    "STALE (last known)"
                }
            ),
            format!(
                "Drops: {} | AQM marks: {}",
                number(totals.and_then(|t| t.drop_pkts)),
                number(totals.and_then(|t| t.aqm_marks))
            ),
            format!(
                "New/sample: drops {} | marks {}",
                number(queue.filter(|_| live).and_then(|q| q.drop_delta)),
                number(queue.filter(|_| live).and_then(|q| q.aqm_delta))
            ),
            format!(
                "Backlog: {} bytes",
                number(totals.and_then(|t| t.backlog_bytes))
            ),
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn partial_totals_resets_and_large_counters_never_fabricate_events() {
        let old = Totals {
            drop_pkts: Some(100),
            aqm_marks: Some(u64::MAX - 1),
            backlog_bytes: Some(8192),
        };
        let next = Totals {
            drop_pkts: Some(1),
            aqm_marks: Some(u64::MAX),
            backlog_bytes: Some(0),
        };
        let report = Queue::between(next, Some(&old));
        assert_eq!(report.drop_delta, None);
        assert_eq!(report.aqm_delta, Some(1));
        assert_eq!(report.totals.backlog_bytes, Some(0));
        assert!(report.valid());
        let partial = Queue::between(
            Totals {
                aqm_marks: Some(50),
                ..Default::default()
            },
            Some(&old),
        );
        assert_eq!(partial.drop_delta, None);
        assert_eq!(partial.aqm_delta, None);
        assert!(partial.valid());
        let text = Queue::measurements(Some(&partial), true).join("\n");
        assert!(
            text.contains("Drops: unknown")
                && text.contains("AQM marks: 50")
                && text.contains("Backlog: unknown")
        );
    }

    #[test]
    fn stale_measurements_keep_last_totals_but_hide_current_increments() {
        let q = Queue::between(
            Totals {
                drop_pkts: Some(3),
                aqm_marks: Some(5),
                backlog_bytes: Some(10),
            },
            Some(&Totals::default()),
        );
        let mut q = q;
        q.drop_delta = Some(3);
        q.aqm_delta = Some(5);
        assert!(q.valid());
        let text = Queue::measurements(Some(&q), false).join("\n");
        assert!(
            text.contains("STALE (last known)")
                && text.contains("Drops: 3")
                && text.contains("New/sample: drops unknown | marks unknown")
        );
        let text = Queue::measurements(None, true).join("\n");
        assert!(text.contains("unavailable") && !text.contains("LIVE"));
    }
}
