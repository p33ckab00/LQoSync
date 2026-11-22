//! WAN availability and quality as the placement sees them. The WAN monitor produces them.
use super::rates::Fraction;
use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WanState {
    Up,
    /// No current probe result. Placement treats it as available, without quality.
    #[default]
    Unknown,
    /// A problem shorter than the move delay. Subscribers stay.
    Degraded,
    Unavailable,
    /// Healthy again, but not yet stable for the failback time.
    Recovering,
    /// Stable again with automatic failback off. Only a Rebalance releases it.
    WaitingForRebalance,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct WanStatus {
    pub state: WanState,
    /// 0 to 1. `None` when the data is stale: quality mode uses capacity.
    pub quality: Option<Fraction>,
}

impl WanStatus {
    /// True when placement must not use this WAN.
    pub fn excluded(&self) -> bool {
        matches!(
            self.state,
            WanState::Unavailable | WanState::Recovering | WanState::WaitingForRebalance
        )
    }
}

/// WAN node ID to its status. A WAN without an entry is `Unknown`.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct WanSnapshot(pub HashMap<String, WanStatus>);

/// One probe result of one WAN.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Sample {
    Reply {
        rtt_us: u64,
    },
    Lost,
    /// The WAN interface is disabled or not running.
    InterfaceDown,
    /// The router did not answer or refused the probe. It is no evidence about the WAN.
    ProbeError,
}

/// The health of one WAN from its probe samples. Time is in seconds; this type reads no clock.
#[derive(Debug, Clone)]
pub struct WanHealth {
    interval: u64,
    window: u64,
    loss_percent: u64,
    latency_factor: u64,
    move_after: u64,
    failback: u64,
    auto_failback: bool,
    /// Evidence samples (no `ProbeError`) of the last window, oldest first.
    samples: std::collections::VecDeque<(u64, Sample)>,
    /// Round-trip times of the last 10 minutes while the WAN was healthy.
    normal: std::collections::VecDeque<(u64, u64)>,
    bad_since: Option<u64>,
    unavailable: bool,
    good_since: Option<u64>,
    waiting: bool,
}

const NORMAL_SECS: u64 = 600;
const MILLE: u64 = 1000;

fn median(mut values: Vec<u64>) -> Option<u64> {
    values.sort_unstable();
    values.get(values.len() / 2).copied()
}

impl WanHealth {
    pub fn new(cfg: &crate::config::WanMonitor, auto_failback: bool) -> Self {
        Self {
            interval: cfg.interval_secs.max(1),
            window: cfg.window_secs,
            loss_percent: cfg.loss_percent,
            latency_factor: cfg.latency_factor,
            move_after: cfg.move_after_secs,
            failback: cfg.failback_secs,
            auto_failback,
            samples: Default::default(),
            normal: Default::default(),
            bad_since: None,
            unavailable: false,
            good_since: None,
            waiting: false,
        }
    }

    pub fn set_auto_failback(&mut self, on: bool) {
        self.auto_failback = on;
    }

    /// The window samples are enough to judge loss and latency: one for each interval.
    fn judged(&self) -> Vec<(u64, Sample)> {
        let probes: Vec<_> = self
            .samples
            .iter()
            .copied()
            .filter(|(_, s)| matches!(s, Sample::Reply { .. } | Sample::Lost))
            .collect();
        if (probes.len() as u64) < self.window / self.interval {
            return Vec::new();
        }
        probes
    }

    fn normal_rtt(&self) -> Option<u64> {
        median(self.normal.iter().map(|(_, r)| *r).collect())
    }

    fn bad(&self) -> bool {
        if matches!(self.samples.back(), Some((_, Sample::InterfaceDown))) {
            return true;
        }
        let probes = self.judged();
        if probes.is_empty() {
            return false;
        }
        let lost = probes.iter().filter(|(_, s)| *s == Sample::Lost).count() as u64;
        if lost * 100 >= self.loss_percent * probes.len() as u64 {
            return true;
        }
        let rtts = probes
            .iter()
            .filter_map(|(_, s)| match s {
                Sample::Reply { rtt_us } => Some(*rtt_us),
                _ => None,
            })
            .collect();
        matches!((median(rtts), self.normal_rtt()), (Some(now), Some(normal)) if now > normal * self.latency_factor)
    }

    pub fn observe(&mut self, now: u64, sample: Sample) {
        if sample == Sample::ProbeError {
            return;
        }
        self.samples.push_back((now, sample));
        while self
            .samples
            .front()
            .is_some_and(|(t, _)| *t + self.window <= now)
        {
            self.samples.pop_front();
        }
        if self.bad() {
            self.good_since = None;
            self.waiting = false;
            let since = *self.bad_since.get_or_insert(now);
            if now - since > self.move_after {
                self.unavailable = true;
            }
            return;
        }
        self.bad_since = None;
        if self.unavailable {
            let since = *self.good_since.get_or_insert(now);
            if now - since >= self.failback {
                if self.auto_failback {
                    self.release();
                } else {
                    self.waiting = true;
                }
            }
        } else if let Sample::Reply { rtt_us } = sample {
            self.normal.push_back((now, rtt_us));
            while self
                .normal
                .front()
                .is_some_and(|(t, _)| *t + NORMAL_SECS <= now)
            {
                self.normal.pop_front();
            }
        }
    }

    /// A Rebalance, or automatic failback: the WAN is available again.
    pub fn release(&mut self) {
        if self.waiting || (self.unavailable && self.auto_failback) {
            self.unavailable = false;
            self.waiting = false;
            self.good_since = None;
        }
    }

    pub fn status(&self, now: u64) -> WanStatus {
        let fresh = self
            .samples
            .back()
            .is_some_and(|(t, _)| *t + 3 * self.interval > now);
        if !fresh {
            return WanStatus::default();
        }
        let state = if self.unavailable {
            if self.waiting {
                WanState::WaitingForRebalance
            } else if self.good_since.is_some() {
                WanState::Recovering
            } else {
                WanState::Unavailable
            }
        } else if self.bad_since.is_some() {
            WanState::Degraded
        } else {
            WanState::Up
        };
        WanStatus {
            state,
            quality: Some(self.quality()),
        }
    }

    /// (1 - loss) x min(1, normal / current median RTT), in thousandths.
    fn quality(&self) -> Fraction {
        if matches!(self.samples.back(), Some((_, Sample::InterfaceDown))) {
            return Fraction {
                numerator: 0,
                denominator: MILLE,
            };
        }
        let probes: Vec<_> = self
            .samples
            .iter()
            .filter(|(_, s)| matches!(s, Sample::Reply { .. } | Sample::Lost))
            .collect();
        let replies: Vec<u64> = probes
            .iter()
            .filter_map(|(_, s)| match s {
                Sample::Reply { rtt_us } => Some(*rtt_us),
                _ => None,
            })
            .collect();
        let delivered = if probes.is_empty() {
            MILLE
        } else {
            replies.len() as u64 * MILLE / probes.len() as u64
        };
        let latency = match (median(replies), self.normal_rtt()) {
            (Some(now), Some(normal)) if now > 0 => (normal * MILLE / now).min(MILLE),
            _ => MILLE,
        };
        Fraction {
            numerator: delivered * latency / MILLE,
            denominator: MILLE,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn settings() -> crate::config::WanMonitor {
        crate::config::Config::from_toml_str(
            "[[routers]]\nname='Core'\naddress='192.0.2.1'\nusername='u'\npassword_file='/x'\n",
        )
        .unwrap()
        .wan_monitor
    }

    fn health(auto: bool) -> WanHealth {
        WanHealth::new(&settings(), auto)
    }

    /// One sample every 10 s from `from` (inclusive) to `to` (exclusive).
    fn feed(h: &mut WanHealth, from: u64, to: u64, sample: Sample) {
        for t in (from..to).step_by(10) {
            h.observe(t, sample);
        }
    }

    const OK: Sample = Sample::Reply { rtt_us: 10_000 };

    #[test]
    fn wan_monitor_config_defaults_match_the_spec() {
        let s = settings();
        assert!(s.enabled);
        assert_eq!(
            (s.interval_secs, s.loss_percent, s.latency_factor),
            (10, 50, 3)
        );
        assert_eq!(
            (s.window_secs, s.move_after_secs, s.failback_secs),
            (60, 120, 600)
        );
        assert_eq!(
            s.targets,
            vec!["1.1.1.1".to_string(), "8.8.8.8".to_string()]
        );
    }

    #[test]
    fn an_interface_down_is_degraded_at_once_and_unavailable_after_two_minutes() {
        let mut h = health(true);
        feed(&mut h, 0, 600, OK);
        h.observe(600, Sample::InterfaceDown);
        assert_eq!(h.status(600).state, WanState::Degraded);
        feed(&mut h, 610, 730, Sample::InterfaceDown);
        assert_eq!(
            h.status(720).state,
            WanState::Degraded,
            "120 s is not more than 2 minutes"
        );
        h.observe(730, Sample::InterfaceDown);
        assert_eq!(h.status(730).state, WanState::Unavailable);
    }

    #[test]
    fn half_lost_probes_for_one_minute_degrade_the_wan() {
        let mut h = health(true);
        feed(&mut h, 0, 600, OK);
        for (i, t) in (600..660).step_by(10).enumerate() {
            h.observe(t, if i % 2 == 0 { Sample::Lost } else { OK });
        }
        assert_eq!(h.status(650).state, WanState::Degraded);
    }

    #[test]
    fn latency_above_three_times_normal_degrades_the_wan() {
        let mut h = health(true);
        feed(&mut h, 0, 600, OK);
        feed(&mut h, 600, 660, Sample::Reply { rtt_us: 25_000 });
        assert_eq!(h.status(650).state, WanState::Up, "2.5 x normal");
        feed(&mut h, 660, 720, Sample::Reply { rtt_us: 40_000 });
        assert_eq!(h.status(710).state, WanState::Degraded, "4 x normal");
    }

    #[test]
    fn a_short_failure_moves_nobody() {
        let mut h = health(true);
        feed(&mut h, 0, 600, OK);
        feed(&mut h, 600, 690, Sample::InterfaceDown);
        feed(&mut h, 690, 1200, OK);
        for t in (600..1200).step_by(10) {
            assert!(!h.status(t).excluded() || t < 600);
        }
        assert_eq!(h.status(1190).state, WanState::Up);
    }

    fn unavailable(auto: bool) -> WanHealth {
        let mut h = health(auto);
        feed(&mut h, 0, 600, OK);
        feed(&mut h, 600, 740, Sample::InterfaceDown);
        assert_eq!(h.status(730).state, WanState::Unavailable);
        h
    }

    #[test]
    fn recovery_needs_ten_stable_minutes() {
        let mut h = unavailable(true);
        feed(&mut h, 740, 1340, OK);
        assert_eq!(h.status(1330).state, WanState::Recovering);
        h.observe(1340, OK);
        assert_eq!(h.status(1340).state, WanState::Up);
    }

    #[test]
    fn manual_failback_waits_in_waiting_for_rebalance() {
        let mut h = unavailable(false);
        feed(&mut h, 740, 1400, OK);
        assert_eq!(h.status(1390).state, WanState::WaitingForRebalance);
        h.release();
        assert_eq!(h.status(1390).state, WanState::Up);
    }

    #[test]
    fn a_probe_error_is_unknown_not_unavailable() {
        let mut h = health(true);
        feed(&mut h, 0, 300, Sample::ProbeError);
        let status = h.status(290);
        assert_eq!((status.state, status.quality), (WanState::Unknown, None));
        assert!(!status.excluded());
    }

    #[test]
    fn quality_score_follows_loss_and_latency() {
        let mut h = health(true);
        feed(&mut h, 0, 600, OK);
        h.observe(640, Sample::Lost);
        feed(&mut h, 650, 700, Sample::Reply { rtt_us: 20_000 });
        // 1 lost of 6 in the window and twice the normal latency: 833/1000 x 500/1000.
        let q = h.status(690).quality.unwrap();
        assert_eq!((q.numerator, q.denominator), (416, 1000));
    }
}
