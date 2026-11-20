//! Bounded traffic history for the 1h, 6h, and 24h chart windows.
//! The broker feeds it from the samples that it already reads. Each window keeps a fixed bucket count.
use serde::{Deserialize, Serialize};
use std::{collections::VecDeque, path::Path};
use web_contract::{Point, TrafficWindow};

/// Window name and bucket length in seconds. Each window holds at most `BUCKETS` buckets.
pub const WINDOWS: [(&str, u64); 3] = [("1h", 15), ("6h", 90), ("24h", 360)];
pub const BUCKETS: usize = 240;
/// Actual, Mapped, Unmapped.
const SOURCES: usize = 3;

#[derive(Clone, Default, Debug, PartialEq, Serialize, Deserialize)]
struct Bucket {
    start: u64,
    down: f64,
    up: f64,
    n: u32,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct History {
    /// `rings[source][window]`.
    rings: Vec<Vec<VecDeque<Bucket>>>,
    /// The newest sample time added per source. Older or repeated samples are ignored.
    seen: Vec<u64>,
}

/// The default history has every source and window ring, so `observe` can index it.
impl Default for History {
    fn default() -> Self {
        Self::new()
    }
}
impl History {
    pub fn new() -> Self {
        Self {
            rings: vec![vec![VecDeque::new(); WINDOWS.len()]; SOURCES],
            seen: vec![0; SOURCES],
        }
    }
    fn valid(&self) -> bool {
        self.rings.len() == SOURCES
            && self.rings.iter().all(|r| r.len() == WINDOWS.len())
            && self.seen.len() == SOURCES
    }
    /// Add the samples of one source that are newer than the last added sample.
    pub fn observe(&mut self, source: usize, points: &[Point]) {
        if source >= SOURCES {
            return;
        }
        for p in points {
            if p.at <= self.seen[source]
                || !p.down_mbps.is_finite()
                || !p.up_mbps.is_finite()
                || p.down_mbps < 0.0
                || p.up_mbps < 0.0
            {
                continue;
            }
            self.seen[source] = p.at;
            for (ring, (_, size)) in self.rings[source].iter_mut().zip(WINDOWS) {
                let start = p.at - p.at % size;
                match ring.back_mut() {
                    Some(b) if b.start == start => {
                        b.down += p.down_mbps;
                        b.up += p.up_mbps;
                        b.n += 1;
                    }
                    _ => ring.push_back(Bucket {
                        start,
                        down: p.down_mbps,
                        up: p.up_mbps,
                        n: 1,
                    }),
                }
                let oldest = start.saturating_sub(size * (BUCKETS as u64 - 1));
                while ring.len() > BUCKETS || ring.front().is_some_and(|b| b.start < oldest) {
                    ring.pop_front();
                }
            }
        }
    }
    /// The bucket means of one window. A missing bucket marks the next point with `gap_before`.
    pub fn window(&self, name: &str, now: u64) -> Option<TrafficWindow> {
        let index = WINDOWS.iter().position(|(n, _)| *n == name)?;
        let size = WINDOWS[index].1;
        let oldest = now.saturating_sub(size * BUCKETS as u64);
        let round = |v: f64| (v * 1000.0).round() / 1000.0;
        let series = |source: usize| {
            let mut previous: Option<u64> = None;
            self.rings[source][index]
                .iter()
                .filter(|b| b.n > 0 && b.start >= oldest)
                .map(|b| {
                    let gap = previous.is_some_and(|p| b.start > p + size);
                    previous = Some(b.start);
                    Point {
                        at: b.start,
                        down_mbps: round(b.down / f64::from(b.n)),
                        up_mbps: round(b.up / f64::from(b.n)),
                        gap_before: gap,
                    }
                })
                .collect::<Vec<_>>()
        };
        Some(TrafficWindow {
            window: name.into(),
            bucket_secs: size,
            actual: series(0),
            mapped: series(1),
            unmapped: series(2),
        })
    }
    /// Read a saved history. A missing, unreadable, or invalid file gives an empty history.
    pub fn load(path: &Path) -> Self {
        std::fs::read(path)
            .ok()
            .and_then(|b| serde_json::from_slice::<History>(&b).ok())
            .filter(History::valid)
            .unwrap_or_default()
    }
    /// Write atomically: a temporary file, then a rename.
    pub fn save(&self, path: &Path) -> anyhow::Result<()> {
        let tmp = path.with_extension("tmp");
        std::fs::write(&tmp, serde_json::to_vec(self)?)?;
        std::fs::rename(&tmp, path)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn p(at: u64, down: f64) -> Point {
        Point {
            at,
            down_mbps: down,
            up_mbps: down / 10.0,
            gap_before: false,
        }
    }
    #[test]
    fn buckets_hold_means_and_ignore_repeated_samples() {
        let mut h = History::new();
        h.observe(0, &[p(1000, 10.0), p(1001, 20.0), p(1014, 30.0)]);
        // The broker reads the same newest samples again each second.
        h.observe(0, &[p(1001, 99.0), p(1014, 99.0), p(1015, 40.0)]);
        let w = h.window("1h", 1020).unwrap();
        assert_eq!(w.bucket_secs, 15);
        let means: Vec<_> = w.actual.iter().map(|p| (p.at, p.down_mbps)).collect();
        // 1000 % 15 = 10, so the first bucket starts at 990. 1014 and 1015 share the bucket at 1005.
        assert_eq!(means, [(990, 15.0), (1005, 35.0)]);
        assert!(w.mapped.is_empty() && w.unmapped.is_empty());
    }
    #[test]
    fn a_missing_bucket_is_a_gap_and_old_buckets_leave() {
        let mut h = History::new();
        h.observe(1, &[p(15, 1.0), p(60, 2.0)]);
        let w = h.window("1h", 60).unwrap();
        assert!(!w.mapped[0].gap_before && w.mapped[1].gap_before);
        // 240 buckets of 15 s is one hour. A sample one hour later removes the first buckets.
        h.observe(1, &[p(60 + 3600, 3.0)]);
        let w = h.window("1h", 60 + 3600).unwrap();
        assert_eq!(w.mapped.iter().map(|p| p.at).collect::<Vec<_>>(), [3660]);
        assert!(h.rings[1][0].len() <= BUCKETS);
    }
    #[test]
    fn invalid_samples_and_unknown_windows_are_refused() {
        let mut h = History::new();
        h.observe(2, &[p(10, f64::NAN), p(11, -1.0), p(12, 5.0)]);
        assert_eq!(h.window("6h", 20).unwrap().unmapped.len(), 1);
        assert!(h.window("7d", 20).is_none());
        h.observe(9, &[p(30, 1.0)]);
    }
    #[test]
    fn the_saved_history_round_trips_and_a_bad_file_gives_an_empty_one() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("traffic-history.json");
        let mut h = History::new();
        h.observe(0, &[p(100, 8.0)]);
        h.save(&path).unwrap();
        assert_eq!(History::load(&path), h);
        std::fs::write(&path, b"{\"rings\":[],\"seen\":[]}").unwrap();
        assert_eq!(History::load(&path), History::new());
        assert_eq!(History::load(&dir.path().join("missing")), History::new());
        assert_eq!(History::default(), History::new());
    }
    #[test]
    fn a_full_day_window_fits_the_broker_reply() {
        let mut h = History::new();
        for source in 0..3 {
            let points: Vec<_> = (0..86_400u64)
                .step_by(30)
                .map(|at| p(at + 1, 123_456.789_123))
                .collect();
            h.observe(source, &points);
        }
        for (name, _) in WINDOWS {
            let w = h.window(name, 86_400).unwrap();
            assert!(w.actual.len() <= BUCKETS);
            assert!(serde_json::to_vec(&w).unwrap().len() < 60 * 1024, "{name}");
        }
    }
}
