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
        let top = (step * decade).min(f64::MAX / 2.0);
        Self {
            top,
            top_label: rate(Some(top), false),
        }
    }
}
#[derive(Debug, Default)]
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
            plot.up.last_mut().unwrap().push((v.at as f64, -up));
            plot.down_peak = Some(plot.down_peak.unwrap_or(0.0).max(down));
            plot.up_peak = Some(plot.up_peak.unwrap_or(0.0).max(up));
            plot.count += 1;
            previous = Some(v.at);
        }
        plot
    }
}
pub const NAMES: [&str; 3] = ["Actual", "Mapped", "Unmapped"];
pub fn history(
    t: &crate::worker_util::Telemetry,
    source: usize,
) -> &std::collections::VecDeque<Point> {
    match source {
        1 => &t.classification.mapped.history,
        2 => &t.classification.unmapped.history,
        _ => &t.traffic,
    }
}
#[derive(Debug)]
pub struct Visibility {
    visible: u8,
    focus: Option<usize>,
}
impl Default for Visibility {
    fn default() -> Self {
        Self {
            visible: 7,
            focus: None,
        }
    }
}
impl Visibility {
    pub fn mask(&self) -> u8 {
        self.focus.map_or(self.visible, |i| 1 << i)
    }
    pub fn toggle(&mut self, i: usize) {
        if i < 3 {
            self.visible = self.mask() ^ (1 << i);
            self.focus = None;
        }
    }
    pub fn cycle_focus(&mut self) {
        self.focus = match self.focus {
            None => Some(0),
            Some(0) => Some(1),
            Some(1) => Some(2),
            _ => None,
        };
    }
    pub fn all(&mut self) {
        *self = Self::default();
    }
    pub fn focused(&self) -> Option<usize> {
        self.focus
    }
}
#[derive(Debug, Default)]
pub struct Cache {
    report: Option<std::sync::Arc<crate::worker_util::Telemetry>>,
    seconds: u64,
    at: u64,
    until: u64,
    plots: [Plot; 3],
    #[cfg(test)]
    pub builds: usize,
}
impl Cache {
    pub fn prepare(
        &mut self,
        report: Option<&std::sync::Arc<crate::worker_util::Telemetry>>,
        now: u64,
        seconds: u64,
    ) -> &[Plot; 3] {
        let same = match (&self.report, report) {
            (Some(a), Some(b)) => std::sync::Arc::ptr_eq(a, b),
            (None, None) => true,
            _ => false,
        };
        if !same || self.seconds != seconds || now < self.at || now >= self.until {
            self.report = report.cloned();
            self.seconds = seconds;
            self.at = now;
            self.until = u64::MAX;
            self.plots = std::array::from_fn(|i| {
                if let Some(t) = report {
                    for point in history(t, i) {
                        let change = if point.at > now {
                            point.at
                        } else {
                            point.at.saturating_add(seconds).saturating_add(1)
                        };
                        if change > now {
                            self.until = self.until.min(change);
                        }
                    }
                    Plot::new(history(t, i).iter(), now, seconds)
                } else {
                    Plot::default()
                }
            });
            #[cfg(test)]
            {
                self.builds += 1;
            }
        }
        &self.plots
    }
}
pub fn source_style(p: &super::theme::Palette, source: usize) -> ratatui::style::Style {
    use crate::config::Theme;
    use ratatui::style::{Color, Modifier};
    let color = match (p.theme, source) {
        (Theme::Mono, _) => Color::Reset,
        (Theme::Dark, 0) => Color::Rgb(128, 170, 255),
        (Theme::Dark, 1) => Color::Rgb(255, 208, 119),
        (Theme::Dark, _) => Color::Rgb(196, 160, 255),
        (Theme::Light, 0) => Color::Rgb(32, 75, 160),
        (Theme::Light, 1) => Color::Rgb(134, 87, 0),
        (Theme::Light, _) => Color::Rgb(114, 55, 181),
    };
    let style = super::design::surface(p).fg(color);
    if p.theme == Theme::Mono && source == 1 {
        style.add_modifier(Modifier::BOLD)
    } else if p.theme == Theme::Mono && source == 2 {
        style.add_modifier(Modifier::UNDERLINED)
    } else {
        style
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::worker_util::traffic::Point;
    #[test]
    fn mirrored_upload_coordinates_do_not_change_rates_or_peaks() {
        let point = Point {
            at: 100,
            down_mbps: Some(8.0),
            up_mbps: Some(2.0),
        };
        let plot = Plot::new(std::iter::once(&point), 100, 300);
        assert_eq!(plot.down[0][0], (100.0, 8.0));
        assert_eq!(plot.up[0][0], (100.0, -2.0));
        assert_eq!(plot.up_peak, Some(2.0));
        assert_eq!(point.up_mbps, Some(2.0));
    }
    #[test]
    fn combined_visibility_focus_and_empty_selection_are_independent() {
        let mut v = Visibility::default();
        assert_eq!(v.mask(), 7);
        v.toggle(1);
        assert_eq!(v.mask(), 5);
        v.cycle_focus();
        assert_eq!(v.mask(), 1);
        v.cycle_focus();
        assert_eq!(v.mask(), 2);
        v.cycle_focus();
        assert_eq!(v.mask(), 4);
        v.cycle_focus();
        assert_eq!(v.mask(), 5);
        v.cycle_focus();
        v.toggle(2);
        assert_eq!(v.mask(), 5);
        v.all();
        assert_eq!(v.mask(), 7);
        for i in 0..3 {
            v.toggle(i);
        }
        assert_eq!(v.mask(), 0);
        v.all();
        assert_eq!(v.mask(), 7);
    }
    #[test]
    fn prepared_curves_reuse_data_until_window_expiry_and_handle_clock_changes() {
        use std::sync::Arc;
        let mut report = crate::worker_util::Telemetry::default();
        report.traffic.push_back(Point {
            at: 100,
            down_mbps: Some(8.0),
            up_mbps: Some(2.0),
        });
        report.classification.mapped.history = report.traffic.clone();
        let report = Arc::new(report);
        let mut cache = Cache::default();
        assert_eq!(cache.prepare(Some(&report), 100, 10)[0].count, 1);
        let built = cache.builds;
        assert_eq!(cache.prepare(Some(&report), 110, 10)[0].count, 1);
        assert_eq!(cache.builds, built);
        assert_eq!(cache.prepare(Some(&report), 111, 10)[0].count, 0);
        assert_eq!(cache.builds, built + 1);
        assert_eq!(cache.prepare(Some(&report), 99, 10)[0].count, 0);
        assert_eq!(cache.prepare(Some(&report), 100, 10)[0].count, 1);
        let other = Arc::new((*report).clone());
        cache.prepare(Some(&other), 100, 10);
        let built = cache.builds;
        cache.prepare(Some(&other), 100, 20);
        assert_eq!(cache.builds, built + 1);
        assert_eq!(cache.prepare(None, 100, 20)[0].count, 0);
        assert_eq!(
            std::sync::Arc::strong_count(&other),
            1,
            "removed sources must release the cached report"
        );
    }
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
        assert!((Scale::new(f64::MAX, 1.0).top * 2.0).is_finite());
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
