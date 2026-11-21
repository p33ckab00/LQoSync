//! Mirrored geometry uses one time domain and independent observed direction scales.
use web_contract::{Point, WanCapacity};

pub fn domain(series: &[&[Point]]) -> Option<(u64, u64)> {
    let first = series
        .iter()
        .filter_map(|s| s.first().map(|p| p.at))
        .min()?;
    let last = series.iter().filter_map(|s| s.last().map(|p| p.at)).max()?;
    Some((first, last.max(first + 1)))
}
pub fn peak(series: &[&[Point]], down: bool) -> f64 {
    series
        .iter()
        .flat_map(|s| s.iter())
        .map(|p| if down { p.down_mbps } else { p.up_mbps })
        .fold(0.0_f64, f64::max)
}
/// The ceiling follows the visible Actual, Mapped, and Unmapped peak, not WAN capacity.
pub fn maximum(series: &[&[Point]], down: bool) -> f64 {
    (peak(series, down).max(0.001) * 1.15).min(f64::MAX)
}
/// Labeled capacity references in Mbps, `[down, up]`. Traffic may exceed them.
pub fn references(c: &WanCapacity) -> Vec<(String, [f64; 2])> {
    let mbps = |d: u64, u: u64| [d as f64 / 1_000_000.0, u as f64 / 1_000_000.0];
    let combined =
        (c.wans.len() != 1).then(|| ("Combined WAN".to_string(), mbps(c.down_bps, c.up_bps)));
    combined
        .into_iter()
        .chain(
            c.wans
                .iter()
                .map(|w| (w.name.clone(), mbps(w.down_bps, w.up_bps))),
        )
        .collect()
}
/// The plot y of a reference, or `None` when it is above the observed scale.
pub fn placed(mbps: f64, maximum: f64, down: bool) -> Option<f64> {
    (mbps <= maximum).then(|| y(mbps, maximum, down))
}
pub fn y(rate: f64, maximum: f64, down: bool) -> f64 {
    120.0 + if down { -1.0 } else { 1.0 } * rate / maximum * 120.0
}
pub fn x(at: u64, domain: (u64, u64)) -> f64 {
    at.saturating_sub(domain.0) as f64 / domain.1.saturating_sub(domain.0).max(1) as f64 * 900.0
}
/// `step` is the largest sample spacing, in seconds, that still joins two points.
pub fn path(
    points: &[Point],
    down: bool,
    maximum: f64,
    area: bool,
    domain: (u64, u64),
    step: u64,
) -> String {
    let mut result = String::new();
    let mut start_x = 0.0;
    for (i, point) in points.iter().enumerate() {
        let split = i == 0 || point.gap_before || point.at.saturating_sub(points[i - 1].at) > step;
        if split && area && i > 0 {
            result.push_str(&format!(
                " L{:.1},120 L{start_x:.1},120 Z",
                x(points[i - 1].at, domain)
            ));
        }
        if split {
            start_x = x(point.at, domain);
        }
        let value = if down { point.down_mbps } else { point.up_mbps };
        result.push_str(&format!(
            "{}{:.1},{:.1}",
            if split { " M" } else { " L" },
            x(point.at, domain),
            y(value, maximum, down)
        ));
    }
    if area && let Some(last) = points.last() {
        result.push_str(&format!(
            " L{:.1},120 L{start_x:.1},120 Z",
            x(last.at, domain)
        ));
    }
    result
}
/// Exact timestamps preserve explicit missing samples rather than filling them from a neighbor.
pub fn sample(points: &[Point], at: u64) -> Option<&Point> {
    points
        .binary_search_by_key(&at, |p| p.at)
        .ok()
        .map(|i| &points[i])
}

#[cfg(test)]
mod tests {
    use super::*;
    fn p(at: u64, d: f64, u: f64) -> Point {
        Point {
            at,
            down_mbps: d,
            up_mbps: u,
            gap_before: false,
        }
    }
    #[test]
    fn references_label_combined_and_each_wan_and_flag_off_scale() {
        let link = |name: &str, d, u| web_contract::WanLink {
            name: name.into(),
            down_bps: d,
            up_bps: u,
        };
        let mut c = web_contract::WanCapacity {
            down_bps: 300_000_000,
            up_bps: 60_000_000,
            wan_count: 2,
            registry_revision: 1,
            wans: vec![
                link("WAN-A", 200_000_000, 40_000_000),
                link("WAN-B", 100_000_000, 20_000_000),
            ],
        };
        let r = references(&c);
        let labels: Vec<_> = r.iter().map(|r| r.0.as_str()).collect();
        assert_eq!(labels, ["Combined WAN", "WAN-A", "WAN-B"]);
        assert_eq!(r[0].1, [300.0, 60.0]);
        // One WAN is its own combined total, so it draws once.
        c.wans.truncate(1);
        assert_eq!(references(&c).len(), 1);
        // Older core builds send totals only.
        c.wans.clear();
        assert_eq!(references(&c)[0].0, "Combined WAN");
        assert_eq!(placed(150.0, 100.0, true), None);
        assert_eq!(placed(50.0, 100.0, true), Some(60.0));
        assert_eq!(placed(50.0, 100.0, false), Some(180.0));
    }
    #[test]
    fn independent_scales_follow_the_peak_of_every_visible_series() {
        let actual = vec![p(10, 100.0, 25.0)];
        let mapped = vec![p(10, 80.0, 40.0)];
        let series = [actual.as_slice(), mapped.as_slice()];
        // Download peak comes from Actual, upload peak from Mapped. Capacity plays no part.
        assert!((maximum(&series, true) - 115.0).abs() < 1e-9);
        assert!((maximum(&series, false) - 46.0).abs() < 1e-9);
        assert_eq!(maximum(&[], true), 0.001 * 1.15);
        assert_eq!(y(25.0, 250.0, false), 132.0);
        assert_eq!(y(100.0, 1000.0, true), 108.0);
    }
    #[test]
    fn shared_domain_does_not_stretch_shorter_classification_history() {
        let actual = vec![p(10, 1.0, 0.5), p(20, 1.0, 0.5)];
        let mapped = vec![p(15, 0.5, 0.1), p(20, 0.5, 0.1)];
        let domain = domain(&[&actual, &mapped]).unwrap();
        assert_eq!(domain, (10, 20));
        assert!(path(&mapped, true, 1.0, false, domain, 3).starts_with(" M450.0"));
        assert!(sample(&mapped, 16).is_none());
    }
    #[test]
    fn gaps_split_lines_and_area_at_the_shared_zero() {
        let mut points = vec![
            p(10, 1.0, 0.5),
            p(11, 1.0, 0.5),
            p(20, 1.0, 0.5),
            p(21, 1.0, 0.5),
        ];
        let d = (10, 21);
        assert_eq!(
            path(&points, true, 10.0, false, d, 3).matches('M').count(),
            2
        );
        assert_eq!(
            path(&points, false, 10.0, true, d, 3).matches('Z').count(),
            2
        );
        points[1].gap_before = true;
        assert_eq!(
            path(&points, true, 10.0, false, d, 3).matches('M').count(),
            3
        );
        // History buckets join across their own spacing. A marked gap still splits them.
        let mut buckets = vec![p(0, 1.0, 0.1), p(90, 2.0, 0.2), p(180, 3.0, 0.3)];
        assert_eq!(
            path(&buckets, true, 10.0, false, (0, 180), 3)
                .matches('M')
                .count(),
            3
        );
        assert_eq!(
            path(&buckets, true, 10.0, false, (0, 180), 180)
                .matches('M')
                .count(),
            1
        );
        buckets[2].gap_before = true;
        assert_eq!(
            path(&buckets, true, 10.0, false, (0, 180), 180)
                .matches('M')
                .count(),
            2
        );
    }
}
