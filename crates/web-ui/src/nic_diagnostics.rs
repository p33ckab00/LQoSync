//! Interval-aware NIC figures. Missing or reset samples never become zero rates.
use std::collections::VecDeque;
use web_contract::dpdk::Port;

pub fn counter(port: &Port, label: &str) -> Option<u64> {
    port.details
        .iter()
        .find(|r| r.label == label)?
        .value
        .parse()
        .ok()
}
pub fn rates(port: &Port, live: bool) -> Option<[f64; 4]> {
    if !live || port.reset || !port.state.eq_ignore_ascii_case("up") {
        return None;
    }
    let a = &port.activity;
    let ms = a.interval_ms.filter(|v| *v > 0)? as f64;
    Some([
        a.rx_bytes? as f64 * 0.008 / ms,
        a.tx_bytes? as f64 * 0.008 / ms,
        a.rx_packets? as f64 * 1000.0 / ms,
        a.tx_packets? as f64 * 1000.0 / ms,
    ])
}
pub fn composition(port: &Port) -> Option<[u64; 3]> {
    let total = counter(port, "ipackets (total)")?;
    let broadcast = counter(port, "NIC rx_broadcast_packets")?;
    let multicast = counter(port, "NIC rx_multicast_packets")?;
    let other = total.checked_sub(broadcast.checked_add(multicast)?)?;
    (total > 0).then_some([other, broadcast, multicast])
}
#[derive(Clone, Default, PartialEq)]
pub struct History {
    pub points: VecDeque<(u64, [f64; 4])>,
    last: Option<u64>,
    gap: bool,
}
impl History {
    pub fn observe(&mut self, port: &Port, live: bool) {
        let Some(at) = port.activity.sampled_at else {
            return;
        };
        // Wait for the overview clock anchor before consuming a future-dated sample.
        if !live {
            return;
        }
        if self.last.is_some_and(|old| at <= old) {
            return;
        }
        self.last = Some(at);
        let Some(value) = rates(port, live) else {
            self.gap = true;
            return;
        };
        if self.gap
            || self
                .points
                .back()
                .is_some_and(|p| at.saturating_sub(p.0) > 3)
        {
            self.points.clear();
        }
        self.gap = false;
        self.points.push_back((at, value));
        while self.points.len() > 60
            || self
                .points
                .front()
                .is_some_and(|p| at.saturating_sub(p.0) > 60)
        {
            self.points.pop_front();
        }
    }
    pub fn invalidate(&mut self) {
        self.gap = true;
    }
    pub fn path(&self, direction: usize) -> String {
        if self.points.len() < 2 {
            return String::new();
        }
        let max = self
            .points
            .iter()
            .map(|p| p.1[direction])
            .fold(0.001_f64, f64::max);
        let first = self.points.front().unwrap().0;
        let span = self.points.back().unwrap().0.saturating_sub(first).max(1) as f64;
        self.points
            .iter()
            .enumerate()
            .map(|(i, (at, rate))| {
                format!(
                    "{}{:.1},{:.1}",
                    if i == 0 { "M" } else { "L" },
                    (*at - first) as f64 / span * 240.0,
                    36.0 - rate[direction] / max * 32.0
                )
            })
            .collect()
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn rates_use_the_actual_interval_and_histories_do_not_bridge_invalid_samples() {
        let mut p = Port {
            state: "up".into(),
            ..Default::default()
        };
        p.activity = web_contract::dpdk::PortActivity {
            sampled_at: Some(100),
            interval_ms: Some(2000),
            rx_bytes: Some(250000),
            tx_bytes: Some(125000),
            rx_packets: Some(100),
            tx_packets: Some(50),
        };
        assert_eq!(rates(&p, true), Some([1.0, 0.5, 50.0, 25.0]));
        let mut h = History::default();
        h.observe(&p, true);
        h.observe(&p, true);
        assert_eq!(h.points.len(), 1);
        p.activity.sampled_at = Some(101);
        h.observe(&p, true);
        assert!(!h.path(0).is_empty());
        p.activity.sampled_at = Some(102);
        h.observe(&p, false);
        h.observe(&p, true);
        assert_eq!(
            h.points.len(),
            3,
            "clock-anchor ordering must not clear the trend"
        );
        h.invalidate();
        p.reset = true;
        p.activity.sampled_at = Some(103);
        h.observe(&p, true);
        assert!(rates(&p, true).is_none());
        p.reset = false;
        p.activity.sampled_at = Some(104);
        h.observe(&p, true);
        assert_eq!(h.points.len(), 1);
        assert!(rates(&p, false).is_none());
        p.activity.interval_ms = Some(0);
        assert!(rates(&p, true).is_none());
        assert!(composition(&p).is_none());
        for (label, value) in [
            ("ipackets (total)", "10"),
            ("NIC rx_broadcast_packets", "7"),
            ("NIC rx_multicast_packets", "4"),
        ] {
            p.details.push(web_contract::dpdk::Reading {
                label: label.into(),
                value: value.into(),
            });
        }
        assert!(composition(&p).is_none());
        p.details[0].value = "20".into();
        assert_eq!(composition(&p), Some([9, 7, 4]));
    }
}
