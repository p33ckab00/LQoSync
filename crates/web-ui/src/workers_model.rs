//! The Workers screen: per-worker session history and phase shares. Pure, so native tests cover it.
use crate::worker_ring_model::RING;
use std::collections::{BTreeMap, VecDeque};
use web_contract::dpdk::Worker;

/// Samples that the trend of each worker keeps: about one minute at one sample each second.
pub(crate) const KEEP: usize = 60;

/// The busy share of each worker in this browser session, one point for each distinct sample.
#[derive(Clone, Debug, Default, PartialEq)]
pub(crate) struct History {
    last: Option<u64>,
    pub points: BTreeMap<u16, VecDeque<u64>>,
}
impl History {
    /// Add one sample. A repeated sample time (the same snapshot again) adds nothing.
    pub(crate) fn observe(&mut self, sampled_at: u64, workers: &[Worker]) {
        if sampled_at == 0 || self.last == Some(sampled_at) {
            return;
        }
        self.last = Some(sampled_at);
        for w in workers {
            let p = self.points.entry(w.id).or_default();
            p.push_back(w.busy_ppm.min(1_000_000));
            while p.len() > KEEP {
                p.pop_front();
            }
        }
    }
}

/// A sparkline path in a `KEEP x 20` box: 0 % at the bottom, 100 % at the top. Fewer than two
/// points give no line.
pub(crate) fn sparkline(points: &VecDeque<u64>) -> String {
    if points.len() < 2 {
        return String::new();
    }
    let start = KEEP - points.len();
    points
        .iter()
        .enumerate()
        .map(|(i, ppm)| {
            let x = (start + i) as f64;
            let y = 20.0 - *ppm as f64 / 50_000.0;
            format!("{}{x:.0} {y:.1}", if i == 0 { "M" } else { "L" })
        })
        .collect()
}

/// One phase share: the ring label, the ring index (its color), the cycles, and the share 0..1.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Share {
    pub label: &'static str,
    pub index: usize,
    pub cycles: u64,
    pub share: f64,
}

/// The cycle shares of the ring phases since the bridge started. "Empty RX" (idle polling) is not a
/// ring phase, so it is not in the total. Unknown phases give no share.
pub(crate) fn shares(w: &Worker) -> Vec<Share> {
    let found: Vec<(usize, &'static str, u64)> = RING
        .iter()
        .enumerate()
        .filter_map(|(i, (source, label, _))| {
            w.phases
                .iter()
                .find(|p| p.name == *source)
                .map(|p| (i, *label, p.cycles))
        })
        .collect();
    let total: u64 = found.iter().map(|(_, _, c)| *c).sum();
    if total == 0 {
        return Vec::new();
    }
    found
        .into_iter()
        .map(|(index, label, cycles)| Share {
            label,
            index,
            cycles,
            share: cycles as f64 / total as f64,
        })
        .collect()
}

pub(crate) fn pct(ppm: Option<u64>) -> String {
    ppm.map(|p| format!("{:.1}%", p.min(1_000_000) as f64 / 10_000.0))
        .unwrap_or("—".into())
}

#[cfg(test)]
mod tests {
    use super::*;
    use web_contract::dpdk::Phase;

    fn worker(id: u16, busy: u64, phases: &[(&str, u64)]) -> Worker {
        Worker {
            id,
            busy_ppm: busy,
            phases: phases
                .iter()
                .map(|(n, c)| Phase {
                    name: (*n).into(),
                    cycles: *c,
                })
                .collect(),
            ..Default::default()
        }
    }

    #[test]
    fn history_keeps_distinct_samples_and_a_bounded_trend() {
        let mut h = History::default();
        h.observe(10, &[worker(0, 250_000, &[])]);
        h.observe(10, &[worker(0, 900_000, &[])]);
        assert_eq!(h.points[&0], [250_000], "a repeated snapshot adds nothing");
        for t in 11..100 {
            h.observe(t, &[worker(0, 2_000_000, &[])]);
        }
        assert_eq!(h.points[&0].len(), KEEP);
        assert_eq!(
            h.points[&0].back(),
            Some(&1_000_000),
            "busy is capped at 100 %"
        );
        let path = sparkline(&h.points[&0]);
        assert!(
            path.starts_with("M0 0.0") && path.ends_with("L59 0.0"),
            "{path}"
        );
        assert_eq!(sparkline(&VecDeque::from([5])), "");
    }

    #[test]
    fn shares_cover_the_ring_phases_without_idle_polling() {
        let w = worker(
            0,
            0,
            &[("RX", 30), ("Empty RX", 900), ("TX", 10), ("Charge", 0)],
        );
        let s = shares(&w);
        let labels: Vec<_> = s.iter().map(|x| x.label).collect();
        assert_eq!(labels, ["RX", "TX", "Charge"]);
        assert!((s[0].share - 0.75).abs() < 1e-9 && (s[1].share - 0.25).abs() < 1e-9);
        assert_eq!(s[1].index, 4, "TX keeps its ring color");
        assert!(shares(&worker(0, 0, &[])).is_empty());
        assert_eq!(pct(Some(750_000)), "75.0%");
        assert_eq!(pct(None), "—");
    }
}
