//! Sample-triggered LEDs. Link state and counter activity are independent.
use web_contract::dpdk::Port;
#[derive(Default)]
pub struct Activity {
    previous: Option<u64>,
}
impl Activity {
    pub fn observe(&mut self, port: &Port, live: bool) -> Option<[bool; 2]> {
        let at = port.activity.sampled_at;
        // A new source timestamp can arrive before the server-clock anchor updates.
        // Do not consume that sample until freshness is established.
        if !live {
            return Some([false; 2]);
        }
        let previous = self.previous;
        self.previous = at;
        if !port.state.eq_ignore_ascii_case("up") || port.reset {
            return Some([false; 2]);
        }
        let at = at?;
        if previous.is_none_or(|old| at <= old) {
            return None;
        }
        Some(active(port))
    }
}
pub fn active(port: &Port) -> [bool; 2] {
    let a = &port.activity;
    [
        a.rx_packets.is_some_and(|n| n > 0) || a.rx_bytes.is_some_and(|n| n > 0),
        a.tx_packets.is_some_and(|n| n > 0) || a.tx_bytes.is_some_and(|n| n > 0),
    ]
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn pulses_follow_distinct_live_samples_and_ignore_counter_resets() {
        let mut p = Port {
            state: "UP".into(),
            ..Default::default()
        };
        p.activity.sampled_at = Some(100);
        p.activity.rx_packets = Some(7);
        let mut a = Activity::default();
        assert_eq!(a.observe(&p, true), None);
        p.activity.sampled_at = Some(110);
        assert_eq!(a.observe(&p, true), Some([true, false]));
        assert_eq!(a.observe(&p, true), None);
        p.activity.sampled_at = Some(120);
        p.activity.tx_packets = Some(2);
        assert_eq!(a.observe(&p, true), Some([true, true]));
        p.reset = true;
        assert_eq!(a.observe(&p, true), Some([false, false]));
        p.reset = false;
        p.state = "DOWN".into();
        assert_eq!(a.observe(&p, true), Some([false, false]));
        p.state = "UP".into();
        assert_eq!(a.observe(&p, false), Some([false, false]));
        p.activity.sampled_at = Some(130);
        assert_eq!(a.observe(&p, false), Some([false, false]));
        assert_eq!(a.observe(&p, true), Some([true, true]));
        p.activity.sampled_at = Some(99);
        assert_eq!(a.observe(&p, true), None);
    }
}
