//! Speed rules and rate values.

use crate::config::{RateLimitOrder, Speed, UnitlessUnit};
use anyhow::Context;
use regex::Regex;

/// A download and upload rate in Mbps.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Rate {
    pub down: f64,
    pub up: f64,
}

/// Round to 3 decimals, half to even on the exact value.
pub fn round3(x: f64) -> f64 {
    format!("{x:.3}").parse().unwrap_or(x)
}

/// Rate text for the CSV: 3 decimals at most, no trailing zeros ("50", "157.5").
pub fn fmt_rate(x: f64) -> String {
    let s = format!("{x:.3}");
    s.trim_end_matches('0').trim_end_matches('.').to_string()
}

/// `round3(x)`, but not below `safe_min`.
pub fn clamp(x: f64, safe_min: f64) -> f64 {
    round3(x).max(safe_min)
}

/// The four rates of one circuit.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CircuitRates {
    pub down_min: f64,
    pub up_min: f64,
    pub down_max: f64,
    pub up_max: f64,
}

/// Circuit rates from a base rate. LibreQoS rejects a max below 0.2 and a min below 0.1.
pub fn circuit_rates(base: Rate, min_pct: f64, safe_min: f64, max_floor: f64) -> CircuitRates {
    let down_max = clamp(base.down, safe_min).max(max_floor);
    let up_max = clamp(base.up, safe_min).max(max_floor);
    CircuitRates {
        down_min: clamp(base.down * min_pct, safe_min).min(down_max),
        up_min: clamp(base.up * min_pct, safe_min).min(up_max),
        down_max,
        up_max,
    }
}

/// Mbps for a number and a unit letter. An empty unit uses `unitless`.
fn to_mbps(number: f64, unit: &str, unitless: UnitlessUnit) -> f64 {
    match unit.to_ascii_lowercase().as_str() {
        "k" => number / 1000.0,
        "m" => number,
        "g" => number * 1000.0,
        _ => match unitless {
            UnitlessUnit::Mbps => number,
            UnitlessUnit::Bps => number / 1_000_000.0,
        },
    }
}

/// Parse "A/B" values with an optional unit. Returns the two values in text order.
fn parse_pair(token: &str, unitless: UnitlessUnit) -> Option<(f64, f64)> {
    let (a, b) = token.split_once('/')?;
    let one = |t: &str| -> Option<f64> {
        let t = t.trim();
        let split = t
            .find(|c: char| !(c.is_ascii_digit() || c == '.'))
            .unwrap_or(t.len());
        let (num, unit) = t.split_at(split);
        if num.is_empty() || unit.len() > 1 || !unit.chars().all(|c| "kKmMgG".contains(c)) {
            return None;
        }
        Some(to_mbps(num.parse().ok()?, unit, unitless))
    };
    Some((one(a)?, one(b)?))
}

/// `A/B` in download/upload order. A number with no unit is Mbps.
pub fn parse_default_rate(s: &str) -> Option<Rate> {
    let (down, up) = parse_pair(s.split_whitespace().next()?, UnitlessUnit::Mbps)?;
    usable(Rate { down, up })
}

/// A rate is usable if both values are finite. A very long digit run parses to infinity.
fn finite(r: Rate) -> Option<Rate> {
    (r.down.is_finite() && r.up.is_finite()).then_some(r)
}

/// A usable rate is finite and above zero.
fn usable(r: Rate) -> Option<Rate> {
    finite(r).filter(|r| r.down > 0.0 && r.up > 0.0)
}

/// Speed parser with the regexes compiled one time.
#[derive(Debug, Clone)]
pub struct SpeedParser {
    pipe: Regex,
    pair: Regex,
    single: Regex,
    profile_name: Regex,
    order: RateLimitOrder,
    unitless: UnitlessUnit,
}

impl SpeedParser {
    pub fn new(cfg: &Speed) -> anyhow::Result<SpeedParser> {
        let num = r"(\d+(?:\.\d+)?)";
        Ok(SpeedParser {
            pipe: Regex::new(&format!(r"(?i)\|\s*{num}([kmg])/{num}([kmg])\b"))?,
            pair: Regex::new(&format!(r"(?i){num}([kmg])/{num}([kmg])\b"))?,
            single: Regex::new(&format!(r"(?i){num}m\b"))?,
            profile_name: Regex::new(&cfg.profile_name_regex)
                .context("speed.profile_name_regex does not compile")?,
            order: cfg.rate_limit_order,
            unitless: cfg.unitless_unit,
        })
    }

    /// The first speed in a comment, also a zero speed, in download/upload order.
    fn comment_raw(&self, s: &str) -> Option<Rate> {
        for re in [&self.pipe, &self.pair] {
            if let Some(c) = re.captures(s) {
                let v =
                    |i: usize, u: usize| to_mbps(c[i].parse().unwrap_or(0.0), &c[u], self.unitless);
                return finite(Rate {
                    down: v(1, 2),
                    up: v(3, 4),
                });
            }
        }
        let c = self.single.captures(s)?;
        let v: f64 = c[1].parse().ok()?;
        finite(Rate { down: v, up: v })
    }

    /// Speed from a comment, in download/upload order.
    pub fn comment(&self, s: &str) -> Option<Rate> {
        self.comment_raw(s).and_then(usable)
    }

    /// The comment has a speed with a zero value, for example `10M/0M`.
    pub fn zero_rate(&self, s: &str) -> bool {
        self.comment_raw(s)
            .is_some_and(|r| r.down <= 0.0 || r.up <= 0.0)
    }

    /// Speed from a RouterOS `rate-limit` value. Only the first rate pair counts.
    pub fn rate_limit(&self, s: &str) -> Option<Rate> {
        let (a, b) = parse_pair(s.split_whitespace().next()?, self.unitless)?;
        let rate = match self.order {
            RateLimitOrder::DownloadUpload => Rate { down: a, up: b },
            RateLimitOrder::RouterosRxTx => Rate { down: b, up: a },
        };
        usable(rate)
    }

    /// Speed from a profile name: the last match of `profile_name_regex`.
    pub fn profile_name(&self, s: &str) -> Option<Rate> {
        let c = self.profile_name.captures_iter(s).last()?;
        let number: f64 = c.get(1)?.as_str().parse().ok()?;
        let unit = c.get(2).map_or("", |m| m.as_str());
        let v = to_mbps(number, unit, self.unitless);
        usable(Rate { down: v, up: v })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{RateLimitOrder, Speed, UnitlessUnit};

    fn p() -> SpeedParser {
        SpeedParser::new(&Speed::default()).unwrap()
    }
    fn legacy() -> SpeedParser {
        SpeedParser::new(&Speed {
            rate_limit_order: RateLimitOrder::DownloadUpload,
            unitless_unit: UnitlessUnit::Mbps,
            ..Speed::default()
        })
        .unwrap()
    }
    fn r(down: f64, up: f64) -> Option<Rate> {
        Some(Rate { down, up })
    }

    #[test]
    fn round_and_format() {
        assert_eq!(round3(0.0625), 0.062);
        assert_eq!(round3(2.0005), 2.001);
        assert_eq!(round3(1.2345), 1.234);
        assert_eq!(fmt_rate(50.0), "50");
        assert_eq!(fmt_rate(157.5), "157.5");
        assert_eq!(fmt_rate(0.128), "0.128");
        assert_eq!(fmt_rate(0.1 + 0.2), "0.3");
        assert_eq!(fmt_rate(0.0), "0");
        assert_eq!(fmt_rate(100.0), "100");
    }

    #[test]
    fn comment_forms() {
        assert_eq!(p().comment("Alice|25M/5M"), r(25.0, 5.0));
        assert_eq!(p().comment("x 10M/2M |25M/5M"), r(25.0, 5.0));
        assert_eq!(p().comment("Plan 25m"), r(25.0, 25.0));
        assert_eq!(p().comment("1G/500M"), r(1000.0, 500.0));
        assert_eq!(p().comment("|100k/50k"), r(0.1, 0.05));
        assert_eq!(p().comment("512k"), None);
        assert_eq!(p().comment("Wifi 5G"), None);
        assert_eq!(p().comment("Room 5Main"), None);
        assert_eq!(p().comment(""), None);
        assert_eq!(p().comment("no speed"), None);
    }

    #[test]
    fn rate_limit_routeros_default() {
        // RouterOS rx/tx: the first value is the client upload.
        assert_eq!(p().rate_limit("2M/10M 4M/20M 1M/8M 10/10"), r(10.0, 2.0));
        assert_eq!(p().rate_limit("2m/10m"), r(10.0, 2.0));
        // A number with no unit is bit/s.
        assert_eq!(p().rate_limit("5000000/10000000"), r(10.0, 5.0));
        assert_eq!(p().rate_limit("10M"), None);
        assert_eq!(p().rate_limit("0/0"), None);
    }

    #[test]
    fn rate_limit_legacy_option() {
        assert_eq!(legacy().rate_limit("10M/2M"), r(10.0, 2.0));
        assert_eq!(legacy().rate_limit("10/5"), r(10.0, 5.0));
    }

    #[test]
    fn profile_name_rule() {
        assert_eq!(p().profile_name("Gold-25M"), r(25.0, 25.0));
        assert_eq!(p().profile_name("Tier-15M-WAN1"), r(15.0, 15.0));
        assert_eq!(p().profile_name("1G-Max"), r(1000.0, 1000.0));
        assert_eq!(p().profile_name("Gold-25"), None);
    }

    #[test]
    fn default_rate_and_circuit_rates() {
        assert_eq!(parse_default_rate("10M/10M"), r(10.0, 10.0));
        let c = circuit_rates(
            Rate {
                down: 50.0,
                up: 50.0,
            },
            0.1,
            0.128,
            0.2,
        );
        assert_eq!((c.down_min, c.down_max), (5.0, 50.0));
        let c = circuit_rates(Rate { down: 0.5, up: 0.5 }, 0.1, 0.128, 0.2);
        assert_eq!((c.down_min, c.down_max), (0.128, 0.5));
        // LibreQoS rejects a max below 0.2.
        let c = circuit_rates(Rate { down: 0.1, up: 0.1 }, 0.1, 0.128, 0.2);
        assert_eq!((c.down_min, c.down_max), (0.128, 0.2));
    }

    #[test]
    fn overlong_number_is_no_rate() {
        let big = format!("{}M", "9".repeat(400));
        assert_eq!(p().comment(&big), None);
        assert_eq!(p().rate_limit(&format!("{big}/{big}")), None);
        assert_eq!(p().profile_name(&format!("Gold-{big}")), None);
    }

    #[test]
    fn zero_comment_speed_is_no_rate() {
        assert_eq!(p().comment("0M/0M"), None);
        assert_eq!(p().comment("Plan 0M"), None);
    }

    #[test]
    fn zero_rate_finds_a_zero_speed_comment() {
        assert!(p().zero_rate("|10M/0M"));
        assert!(p().zero_rate("Plan 0M"));
        assert!(!p().zero_rate("10M/2M"));
        assert!(!p().zero_rate("no speed"));
    }
}
