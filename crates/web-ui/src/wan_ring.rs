//! WAN allocation ring geometry. Pure functions, so native tests cover them.
/// Ring segments as `(offset, length)` in percent of the circle, with a small gap between WANs.
/// A zero, negative, or non-finite value gets no arc.
pub(crate) fn arcs(values: &[f64]) -> Vec<(f64, f64)> {
    let valid = |v: f64| v.is_finite() && v > 0.0;
    let total: f64 = values.iter().copied().filter(|v| valid(*v)).sum();
    if total <= 0.0 {
        return Vec::new();
    }
    let gap = if values.len() > 1 { 1.2 } else { 0.0 };
    let mut start = 0.0;
    values
        .iter()
        .map(|v| {
            let share = if valid(*v) { v / total * 100.0 } else { 0.0 };
            let arc = (start, (share - gap).max(0.0));
            start += share;
            arc
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::arcs;
    #[test]
    fn arcs_split_the_circle_by_share_and_skip_invalid_values() {
        let a = arcs(&[150.0, 50.0]);
        assert_eq!(a[0], (0.0, 75.0 - 1.2));
        assert_eq!(a[1], (75.0, 25.0 - 1.2));
        assert_eq!(arcs(&[200.0]), vec![(0.0, 100.0)]);
        assert!(arcs(&[0.0, 0.0]).is_empty());
        let b = arcs(&[f64::NAN, 10.0]);
        assert_eq!(b[0].1, 0.0);
        assert_eq!(b[1], (0.0, 100.0 - 1.2));
    }
}
