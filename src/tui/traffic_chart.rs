//! Bounded traffic plot preparation and unit labels. Source values stay in Mbps.
use crate::worker_util::traffic::Point;

pub(super) use crate::worker_util::traffic::format_rate as rate;
pub struct Scale {
    pub top: f64,
    pub top_label: String,
}
impl Scale {
    pub fn new(peak: f64, floor: f64) -> Self {
        let peak = peak.max(floor);
        let decade = 10f64.powf(peak.log10().floor());
        let step = [1.0, 2.0, 5.0, 10.0]
            .into_iter()
            .find(|s| *s * decade >= peak)
            .unwrap_or(10.0);
        let top = (step * decade).min(f64::MAX);
        Self {
            top,
            top_label: rate(Some(top), false),
        }
    }
}
#[derive(Default)]
pub struct Plot {
    pub down: Vec<Vec<(f64, f64)>>,
    pub up: Vec<Vec<(f64, f64)>>,
    pub down_peak: Option<f64>,
    pub up_peak: Option<f64>,
    pub count: usize,
}
impl Plot {
    pub fn new<'a>(points: impl Iterator<Item = &'a Point>, now: u64, seconds: u64) -> Self {
        let mut plot = Self::default();
        let mut previous = None;
        for v in points.filter(|v| v.at <= now && now - v.at <= seconds) {
            let (Some(down), Some(up)) = (v.down_mbps, v.up_mbps) else {
                previous = None;
                continue;
            };
            if !v.valid() {
                previous = None;
                continue;
            }
            if previous.is_none_or(|at| v.at.saturating_sub(at) > crate::worker_util::MAX_AGE) {
                plot.down.push(Vec::new());
                plot.up.push(Vec::new());
            }
            plot.down.last_mut().unwrap().push((v.at as f64, down));
            plot.up.last_mut().unwrap().push((v.at as f64, up));
            plot.down_peak = Some(plot.down_peak.unwrap_or(0.0).max(down));
            plot.up_peak = Some(plot.up_peak.unwrap_or(0.0).max(up));
            plot.count += 1;
            previous = Some(v.at);
        }
        plot
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::worker_util::traffic::Point;
    #[test]
    fn chart_units_show_small_traffic_without_rounding_it_to_zero() {
        assert_eq!(rate(Some(0.025), false), "25.0 kbps");
        assert_eq!(rate(Some(0.000025), false), "25.0 bps");
        assert_eq!(rate(Some(2500.0), false), "2.500 Gbps");
        assert_eq!(rate(Some(8.0), true), "8.000 Mbps");
        assert_eq!(rate(Some(0.0), true), "0.000 Mbps");
        assert_eq!(rate(None, false), "unknown");
        assert_eq!(rate(Some(f64::NAN), false), "unknown");
    }
    #[test]
    fn chart_axis_labels_match_the_plotted_bounds_with_stable_steps() {
        for (peak, expected) in [(0.025, 0.05), (1.1, 2.0), (98.0, 100.0), (2500.0, 5000.0)] {
            let a = Scale::new(peak, 0.001);
            assert_eq!(a.top, expected);
            assert_eq!(a.top_label, rate(Some(expected), false));
        }
        assert_eq!(Scale::new(0.0, 1.0).top, 1.0);
        assert!(Scale::new(f64::MAX, 1.0).top.is_finite());
    }
    #[test]
    fn chart_segments_preserve_missing_samples_time_gaps_spikes_and_selected_window() {
        let points = [
            (90, Some(500.0)),
            (100, Some(1.0)),
            (102, Some(100.0)),
            (104, None),
            (106, Some(2.0)),
            (120, Some(3.0)),
            (122, Some(4.0)),
            (124, Some(999.0)),
        ]
        .map(|(at, n)| Point {
            at,
            down_mbps: n,
            up_mbps: n,
        });
        let plot = Plot::new(points.iter(), 122, 22);
        assert_eq!(
            plot.down.iter().map(Vec::len).collect::<Vec<_>>(),
            vec![2, 1, 2]
        );
        assert_eq!(plot.count, 5);
        assert_eq!(plot.down_peak, Some(100.0));
        assert_eq!(plot.up_peak, Some(100.0));
    }
}
