//! Session-only worker history. Duplicate, stale, and missing samples are not zeroes.
use std::collections::VecDeque;
use web_contract::dpdk::Dpdk;
#[derive(Clone, Debug, PartialEq)]
pub struct Sample {
    pub at: u64,
    pub busy_ppm: u64,
    pub gap: bool,
}
#[derive(Clone, Default, PartialEq)]
pub struct History {
    pub points: VecDeque<Sample>,
    gap: bool,
}
impl History {
    pub fn observe(&mut self, data: Option<&Dpdk>, now: u64) {
        let Some(d) = data.filter(|d| d.analytics.live(now, 6)) else {
            self.gap = true;
            return;
        };
        let Some(busy) = d.mean_busy_ppm.filter(|v| *v <= 1_000_000) else {
            self.gap = true;
            return;
        };
        let at = d.analytics.updated.unwrap();
        if self.points.back().is_some_and(|p| at < p.at) {
            self.points.clear();
            self.gap = true;
        }
        if self.points.back().is_some_and(|p| at == p.at) {
            return;
        }
        let gap = self.gap || self.points.back().is_some_and(|p| at - p.at > 6);
        self.points.push_back(Sample {
            at,
            busy_ppm: busy,
            gap,
        });
        while self.points.len() > 60 {
            self.points.pop_front();
        }
        self.gap = false;
    }
    pub fn path(&self) -> String {
        let Some(first) = self.points.front() else {
            return String::new();
        };
        let span = self
            .points
            .back()
            .unwrap()
            .at
            .saturating_sub(first.at)
            .max(1) as f64;
        self.points
            .iter()
            .enumerate()
            .map(|(i, p)| {
                format!(
                    "{} {:.2} {:.2}",
                    if i == 0 || p.gap { "M" } else { "L" },
                    (p.at - first.at) as f64 / span * 600.0,
                    100.0 - p.busy_ppm as f64 / 10000.0
                )
            })
            .collect::<Vec<_>>()
            .join(" ")
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn distinct_fresh_samples_preserve_zero_and_gaps_and_stay_bounded() {
        let mut h = History::default();
        let mut d = Dpdk::default();
        d.analytics.updated = Some(10);
        d.mean_busy_ppm = Some(0);
        h.observe(Some(&d), 10);
        h.observe(Some(&d), 11);
        assert_eq!(h.points.len(), 1);
        assert_eq!(h.points[0].busy_ppm, 0);
        h.observe(None, 12);
        d.analytics.updated = Some(13);
        h.observe(Some(&d), 13);
        assert!(h.points[1].gap);
        assert_eq!(h.path().matches('M').count(), 2);
        for at in 14..100 {
            d.analytics.updated = Some(at);
            h.observe(Some(&d), at);
        }
        assert_eq!(h.points.len(), 60);
        d.analytics.updated = Some(5);
        h.observe(Some(&d), 5);
        assert_eq!(h.points.len(), 1);
        d.analytics.updated = Some(101);
        d.mean_busy_ppm = None;
        h.observe(Some(&d), 101);
        d.mean_busy_ppm = Some(1000001);
        h.observe(Some(&d), 101);
        assert_eq!(h.points.len(), 1);
        d.mean_busy_ppm = Some(1);
        d.analytics.updated = Some(200);
        h.observe(Some(&d), 101);
        assert_eq!(h.points.len(), 1);
    }
}
