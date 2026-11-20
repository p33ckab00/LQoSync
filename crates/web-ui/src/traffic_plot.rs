//! Pure SVG geometry. Capacity colors share the rate axis, never a series bounding box.
use web_contract::{Point, WanCapacity};

pub fn maximum(points: &[Point], capacity: Option<&WanCapacity>) -> f64 {
    let peak = points
        .iter()
        .map(|p| p.down_mbps.max(p.up_mbps))
        .fold(0.0_f64, f64::max);
    match capacity {
        Some(c) => {
            let limit = c.down_bps.max(c.up_bps) as f64 / 1_000_000.0;
            if peak > limit { peak * 1.05 } else { limit }
        }
        None => peak.max(1.0) * 1.15,
    }
}
pub fn y(rate: f64, maximum: f64) -> f64 {
    240.0 - rate / maximum * 240.0
}

pub fn path(points: &[Point], down: bool, maximum: f64, area: bool) -> String {
    let first = points.first().map_or(0, |p| p.at);
    let duration = points
        .last()
        .map_or(1, |p| p.at.saturating_sub(first).max(1)) as f64;
    let x = |p: &Point| p.at.saturating_sub(first) as f64 / duration * 900.0;
    let mut result = String::new();
    let mut start_x = 0.0;
    for (i, point) in points.iter().enumerate() {
        let split = i == 0 || point.gap_before || point.at.saturating_sub(points[i - 1].at) > 3;
        if split && area && i > 0 {
            result.push_str(&format!(
                " L{:.1},240 L{start_x:.1},240 Z",
                x(&points[i - 1])
            ));
        }
        if split {
            start_x = x(point);
        }
        let value = if down { point.down_mbps } else { point.up_mbps };
        result.push_str(&format!(
            "{}{:.1},{:.1}",
            if split { " M" } else { " L" },
            x(point),
            y(value, maximum)
        ));
    }
    if area && let Some(last) = points.last() {
        result.push_str(&format!(" L{:.1},240 L{start_x:.1},240 Z", x(last)));
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn low_traffic_keeps_the_capacity_scale_and_overflow_stays_visible() {
        let capacity = WanCapacity {
            down_bps: 1_000_000_000,
            up_bps: 250_000_000,
            wan_count: 2,
            registry_revision: 7,
        };
        let mut points = vec![Point {
            gap_before: false,
            at: 10,
            down_mbps: 100.0,
            up_mbps: 25.0,
        }];
        assert_eq!(maximum(&points, Some(&capacity)), 1000.0);
        assert_eq!(y(250.0, 1000.0), 180.0);
        points[0].down_mbps = 1200.0;
        assert!(maximum(&points, Some(&capacity)) > 1200.0);
        assert!(y(1000.0, maximum(&points, Some(&capacity))) > 0.0);
    }
    #[test]
    fn missing_intervals_split_both_lines_and_areas() {
        let points: Vec<_> = [10, 11, 20, 21]
            .into_iter()
            .map(|at| Point {
                gap_before: false,
                at,
                down_mbps: 1.0,
                up_mbps: 0.5,
            })
            .collect();
        assert_eq!(path(&points, true, 10.0, false).matches('M').count(), 2);
        assert_eq!(path(&points, true, 10.0, true).matches('Z').count(), 2);
    }
}
