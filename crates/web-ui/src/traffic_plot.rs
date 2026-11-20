//! Mirrored geometry uses one time domain and independent direction capacity scales.
use web_contract::Point;

pub fn domain(series: &[&[Point]]) -> Option<(u64, u64)> {
    let first = series
        .iter()
        .filter_map(|s| s.first().map(|p| p.at))
        .min()?;
    let last = series.iter().filter_map(|s| s.last().map(|p| p.at)).max()?;
    Some((first, last.max(first + 1)))
}
pub fn maximum(series: &[&[Point]], down: bool, capacity_bps: Option<u64>) -> f64 {
    let peak = series
        .iter()
        .flat_map(|s| s.iter())
        .map(|p| if down { p.down_mbps } else { p.up_mbps })
        .fold(0.0_f64, f64::max);
    match capacity_bps.filter(|c| *c > 0) {
        Some(c) => {
            let limit = c as f64 / 1_000_000.0;
            if peak > limit {
                (peak * 1.05).min(f64::MAX)
            } else {
                limit
            }
        }
        None => (peak.max(0.001) * 1.15).min(f64::MAX),
    }
}
pub fn y(rate: f64, maximum: f64, down: bool) -> f64 {
    120.0 + if down { -1.0 } else { 1.0 } * rate / maximum * 120.0
}
pub fn x(at: u64, domain: (u64, u64)) -> f64 {
    at.saturating_sub(domain.0) as f64 / domain.1.saturating_sub(domain.0).max(1) as f64 * 900.0
}
pub fn path(points: &[Point], down: bool, maximum: f64, area: bool, domain: (u64, u64)) -> String {
    let mut result = String::new();
    let mut start_x = 0.0;
    for (i, point) in points.iter().enumerate() {
        let split = i == 0 || point.gap_before || point.at.saturating_sub(points[i - 1].at) > 3;
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
    fn independent_scales_mirror_and_preserve_capacity_on_overflow() {
        let points = vec![p(10, 100.0, 25.0)];
        let series = [points.as_slice()];
        assert_eq!(maximum(&series, true, Some(1_000_000_000)), 1000.0);
        assert_eq!(maximum(&series, false, Some(250_000_000)), 250.0);
        assert_eq!(y(25.0, 250.0, false), 132.0);
        assert_eq!(y(100.0, 1000.0, true), 108.0);
        let overflow = vec![p(10, 1200.0, 300.0)];
        let max = maximum(&[&overflow], true, Some(1_000_000_000));
        assert!(max > 1200.0);
        assert!(y(1000.0, max, true) > 0.0);
    }
    #[test]
    fn shared_domain_does_not_stretch_shorter_classification_history() {
        let actual = vec![p(10, 1.0, 0.5), p(20, 1.0, 0.5)];
        let mapped = vec![p(15, 0.5, 0.1), p(20, 0.5, 0.1)];
        let domain = domain(&[&actual, &mapped]).unwrap();
        assert_eq!(domain, (10, 20));
        assert!(path(&mapped, true, 1.0, false, domain).starts_with(" M450.0"));
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
        assert_eq!(path(&points, true, 10.0, false, d).matches('M').count(), 2);
        assert_eq!(path(&points, false, 10.0, true, d).matches('Z').count(), 2);
        points[1].gap_before = true;
        assert_eq!(path(&points, true, 10.0, false, d).matches('M').count(), 3);
    }
}
