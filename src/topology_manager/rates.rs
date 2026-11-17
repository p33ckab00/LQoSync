//! Integer rates for subscriber policies and aggregate ceilings.

use anyhow::{Context, ensure};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RateBps {
    pub down: u64,
    pub up: u64,
}

/// This conversion does not validate native export bounds or CSV precision.
impl TryFrom<crate::speed::Rate> for RateBps {
    type Error = anyhow::Error;

    fn try_from(rate: crate::speed::Rate) -> anyhow::Result<Self> {
        fn bps(mbps: f64) -> anyhow::Result<u64> {
            ensure!(
                mbps.is_finite() && mbps > 0.0,
                "legacy Mbps rate must be finite and positive"
            );
            let scaled = (mbps * 1_000_000.0).round();
            // Above this bound, f64 cannot distinguish all adjacent integer rates.
            ensure!(
                scaled > 0.0 && scaled < (1u64 << 53) as f64,
                "legacy rate is outside the exact conversion range"
            );
            let integer = scaled as u64;
            ensure!(
                integer as f64 / 1_000_000.0 == mbps,
                "legacy Mbps rate loses precision in integer bps"
            );
            Ok(integer)
        }
        Ok(Self {
            down: bps(rate.down).context("legacy download conversion")?,
            up: bps(rate.up).context("legacy upload conversion")?,
        })
    }
}

impl TryFrom<RateBps> for crate::speed::Rate {
    type Error = anyhow::Error;

    fn try_from(rate: RateBps) -> anyhow::Result<Self> {
        rate.validate()?;
        let converted = Self {
            down: rate.down as f64 / 1_000_000.0,
            up: rate.up as f64 / 1_000_000.0,
        };
        ensure!(
            RateBps::try_from(converted)? == rate,
            "integer bps rate loses precision in legacy Mbps"
        );
        Ok(converted)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Fraction {
    pub numerator: u64,
    pub denominator: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub enum RateRule {
    FixedBps(u64),
    BaseFraction(Fraction),
    /// Explicit legacy import: three-decimal Mbps rounding, then a configured floor.
    LegacyRoundedBase {
        factor: Fraction,
        floor_bps: u64,
    },
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum RuleValue {
    #[default]
    Inherit,
    Override(RateRule),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SpeedPolicy {
    pub down_min: RateRule,
    pub up_min: RateRule,
    pub down_max: RateRule,
    pub up_max: RateRule,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SpeedOverrides {
    pub down_min: RuleValue,
    pub up_min: RuleValue,
    pub down_max: RuleValue,
    pub up_max: RuleValue,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SubscriberRates {
    pub min: RateBps,
    pub max: RateBps,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AggregateRule {
    Inherit,
    /// The local target after any explicit manual factors. Preserve the input separately.
    Fixed(RateBps),
    WanFraction {
        down: Fraction,
        up: Fraction,
    },
    Automatic {
        down: Fraction,
        up: Fraction,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AggregateRates {
    pub local: RateBps,
    pub ceiling: RateBps,
}

pub fn new_policy_defaults() -> SpeedPolicy {
    SpeedPolicy {
        down_min: RateRule::FixedBps(1_000_000),
        up_min: RateRule::FixedBps(1_000_000),
        down_max: RateRule::BaseFraction(Fraction {
            numerator: 1,
            denominator: 1,
        }),
        up_max: RateRule::BaseFraction(Fraction {
            numerator: 1,
            denominator: 1,
        }),
    }
}

impl Fraction {
    pub(super) fn validate(self) -> anyhow::Result<()> {
        ensure!(
            self.numerator > 0 && self.denominator > 0,
            "rate fraction values must be positive"
        );
        Ok(())
    }

    fn apply(self, base: u64) -> anyhow::Result<u64> {
        self.validate()?;
        // The product of two u64 values always fits in u128.
        let product = u128::from(base) * u128::from(self.numerator);
        // Round once. An integer ceiling must never exceed its exact fractional target.
        let rate = u64::try_from(product / u128::from(self.denominator))
            .context("rate exceeds the integer bps range")?;
        ensure!(rate > 0, "resolved rate must be positive");
        Ok(rate)
    }
}

impl AggregateRule {
    pub(super) fn validate(self) -> anyhow::Result<()> {
        match self {
            Self::Inherit => Ok(()),
            Self::Fixed(rate) => rate.validate(),
            Self::WanFraction { down, up } | Self::Automatic { down, up } => {
                down.validate().context("download factor")?;
                up.validate().context("upload factor")
            }
        }
    }
}

impl RateBps {
    pub fn apply_factors(self, down: Fraction, up: Fraction) -> anyhow::Result<Self> {
        self.validate()?;
        Ok(Self {
            down: down.apply(self.down)?,
            up: up.apply(self.up)?,
        })
    }

    pub(super) fn validate(self) -> anyhow::Result<()> {
        ensure!(
            self.down > 0 && self.up > 0,
            "download and upload rates must be positive"
        );
        Ok(())
    }

    pub(super) fn constrain(self, ceiling: Self) -> Self {
        Self {
            down: self.down.min(ceiling.down),
            up: self.up.min(ceiling.up),
        }
    }
}

impl RateRule {
    pub(super) fn validate(self) -> anyhow::Result<()> {
        match self {
            Self::FixedBps(rate) => {
                ensure!(rate > 0, "fixed rate must be positive");
                Ok(())
            }
            Self::BaseFraction(fraction) => fraction.validate(),
            Self::LegacyRoundedBase { factor, floor_bps } => {
                super::legacy_bounds::factor_value(factor)?;
                super::legacy_bounds::mbps(floor_bps)?;
                Ok(())
            }
        }
    }

    fn resolve(self, base: u64) -> anyhow::Result<u64> {
        match self {
            Self::FixedBps(rate) => {
                ensure!(rate > 0, "fixed rate must be positive");
                Ok(rate)
            }
            Self::BaseFraction(fraction) => fraction.apply(base),
            Self::LegacyRoundedBase { factor, floor_bps } => {
                super::legacy_bounds::resolve(base, factor, floor_bps)
            }
        }
    }
}

impl RuleValue {
    pub(super) fn select(self, inherited: RateRule) -> RateRule {
        match self {
            Self::Inherit => inherited,
            Self::Override(rule) => rule,
        }
    }
}

/// Resolve each control from the current authoritative base, without ancestor limits.
pub fn resolve_subscriber(
    base: RateBps,
    defaults: &SpeedPolicy,
    overrides: &SpeedOverrides,
) -> anyhow::Result<SubscriberRates> {
    base.validate().context("subscriber base")?;
    let min = RateBps {
        down: overrides
            .down_min
            .select(defaults.down_min)
            .resolve(base.down)
            .context("subscriber download minimum")?,
        up: overrides
            .up_min
            .select(defaults.up_min)
            .resolve(base.up)
            .context("subscriber upload minimum")?,
    };
    let max = RateBps {
        down: overrides
            .down_max
            .select(defaults.down_max)
            .resolve(base.down)
            .context("subscriber download maximum")?,
        up: overrides
            .up_max
            .select(defaults.up_max)
            .resolve(base.up)
            .context("subscriber upload maximum")?,
    };
    ensure!(
        min.down <= max.down,
        "subscriber download minimum exceeds maximum"
    );
    ensure!(
        min.up <= max.up,
        "subscriber upload minimum exceeds maximum"
    );
    Ok(SubscriberRates { min, max })
}

/// `wan` is configured true capacity. `parent` is the effective ancestor ceiling.
/// The caller supplies a source-specific basis that counts each current circuit once.
pub fn resolve_aggregate(
    rule: &AggregateRule,
    direct_basis: RateBps,
    wan: Option<RateBps>,
    parent: Option<RateBps>,
) -> anyhow::Result<AggregateRates> {
    if let Some(rate) = wan {
        rate.validate().context("configured WAN capacity")?;
    }
    if let Some(rate) = parent {
        rate.validate().context("parent ceiling")?;
    }
    let local = match *rule {
        AggregateRule::Inherit => parent.context("inherited capacity requires a parent ceiling")?,
        AggregateRule::Fixed(rate) => rate,
        AggregateRule::WanFraction { down, up } | AggregateRule::Automatic { down, up } => {
            let basis = if matches!(rule, AggregateRule::WanFraction { .. }) {
                wan.context("WAN percentage requires configured WAN capacity")?
            } else {
                direct_basis.validate().context("direct subscriber basis")?;
                direct_basis
            };
            RateBps {
                down: down.apply(basis.down).context("aggregate download")?,
                up: up.apply(basis.up).context("aggregate upload")?,
            }
        }
    };
    local.validate().context("local aggregate capacity")?;
    let mut ceiling = local;
    for bound in [wan, parent].into_iter().flatten() {
        ceiling = ceiling.constrain(bound);
    }
    Ok(AggregateRates { local, ceiling })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rate(down: u64, up: u64) -> RateBps {
        RateBps { down, up }
    }

    fn fraction(numerator: u64, denominator: u64) -> Fraction {
        Fraction {
            numerator,
            denominator,
        }
    }

    fn automatic(down: Fraction, up: Fraction) -> AggregateRule {
        AggregateRule::Automatic { down, up }
    }

    #[test]
    fn new_defaults_keep_base_maxima_and_one_mbps_minima() {
        let base = rate(50_000_000, 10_000_000);
        let resolved =
            resolve_subscriber(base, &new_policy_defaults(), &SpeedOverrides::default()).unwrap();
        assert_eq!(resolved.min, rate(1_000_000, 1_000_000));
        assert_eq!(resolved.max, base);
    }

    #[test]
    fn overrides_replace_each_control_independently() {
        let overrides = SpeedOverrides {
            down_min: RuleValue::Override(RateRule::BaseFraction(fraction(1, 100))),
            up_max: RuleValue::Override(RateRule::FixedBps(7_000_000)),
            ..SpeedOverrides::default()
        };
        let resolved = resolve_subscriber(
            rate(50_000_000, 20_000_000),
            &new_policy_defaults(),
            &overrides,
        )
        .unwrap();
        assert_eq!(resolved.min, rate(500_000, 1_000_000));
        assert_eq!(resolved.max, rate(50_000_000, 7_000_000));
    }

    #[test]
    fn override_percentages_use_base_without_compounding_defaults() {
        let mut defaults = new_policy_defaults();
        defaults.down_max = RateRule::BaseFraction(fraction(1, 2));
        let overrides = SpeedOverrides {
            down_max: RuleValue::Override(RateRule::BaseFraction(fraction(4, 5))),
            ..SpeedOverrides::default()
        };
        assert_eq!(
            resolve_subscriber(rate(50_000_000, 50_000_000), &defaults, &overrides)
                .unwrap()
                .max
                .down,
            40_000_000
        );
    }

    #[test]
    fn aggregate_limits_do_not_change_subscriber_maxima() {
        let resolved = resolve_aggregate(
            &automatic(fraction(4, 5), fraction(1, 2)),
            rate(500_000_000, 200_000_000),
            Some(rate(310_000_000, 80_000_000)),
            None,
        )
        .unwrap();
        assert_eq!(resolved.local, rate(400_000_000, 100_000_000));
        assert_eq!(resolved.ceiling, rate(310_000_000, 80_000_000));
        let subscriber = resolve_subscriber(
            rate(50_000_000, 50_000_000),
            &new_policy_defaults(),
            &SpeedOverrides::default(),
        )
        .unwrap();
        assert_eq!(subscriber.max, rate(50_000_000, 50_000_000));
    }

    #[test]
    fn wan_fraction_uses_configured_wan_then_lower_parent() {
        let resolved = resolve_aggregate(
            &AggregateRule::WanFraction {
                down: fraction(1, 2),
                up: fraction(1, 4),
            },
            rate(0, 0),
            Some(rate(310_000_000, 200_000_000)),
            Some(rate(100_000_000, 40_000_000)),
        )
        .unwrap();
        assert_eq!(resolved.local, rate(155_000_000, 50_000_000));
        assert_eq!(resolved.ceiling, rate(100_000_000, 40_000_000));
    }

    #[test]
    fn inherit_uses_parent_and_fixed_limits_remain_local() {
        let parent = rate(100_000_000, 20_000_000);
        let inherited =
            resolve_aggregate(&AggregateRule::Inherit, rate(0, 0), None, Some(parent)).unwrap();
        assert_eq!(
            inherited,
            AggregateRates {
                local: parent,
                ceiling: parent
            }
        );
        let fixed = resolve_aggregate(
            &AggregateRule::Fixed(rate(120_000_000, 10_000_000)),
            rate(0, 0),
            None,
            Some(parent),
        )
        .unwrap();
        assert_eq!(fixed.local, rate(120_000_000, 10_000_000));
        assert_eq!(fixed.ceiling, rate(100_000_000, 10_000_000));
    }

    #[test]
    fn profile_only_automatic_capacity_needs_no_wan() {
        let resolved = resolve_aggregate(
            &automatic(fraction(1, 1), fraction(1, 1)),
            rate(7, 9),
            None,
            None,
        )
        .unwrap();
        assert_eq!(resolved.local, rate(7, 9));
        assert_eq!(resolved.ceiling, rate(7, 9));
    }

    #[test]
    fn invalid_effective_rates_fail_without_clamping() {
        for invalid in [
            RateRule::FixedBps(0),
            RateRule::BaseFraction(fraction(1, 0)),
            RateRule::BaseFraction(fraction(0, 1)),
            RateRule::BaseFraction(fraction(1, u64::MAX)),
        ] {
            let overrides = SpeedOverrides {
                up_max: RuleValue::Override(invalid),
                ..SpeedOverrides::default()
            };
            assert!(
                resolve_subscriber(
                    rate(50_000_000, 50_000_000),
                    &new_policy_defaults(),
                    &overrides
                )
                .is_err(),
                "{invalid:?}"
            );
        }
        let overrides = SpeedOverrides {
            up_min: RuleValue::Override(RateRule::FixedBps(51_000_000)),
            ..SpeedOverrides::default()
        };
        assert!(
            resolve_subscriber(
                rate(50_000_000, 50_000_000),
                &new_policy_defaults(),
                &overrides
            )
            .is_err()
        );
        assert!(
            resolve_subscriber(
                rate(0, 50_000_000),
                &new_policy_defaults(),
                &SpeedOverrides::default()
            )
            .is_err()
        );
    }

    #[test]
    fn aggregate_requires_real_references_and_positive_inputs() {
        let wan_rule = AggregateRule::WanFraction {
            down: fraction(1, 2),
            up: fraction(1, 2),
        };
        assert!(resolve_aggregate(&wan_rule, rate(50, 50), None, Some(rate(20, 20))).is_err());
        assert!(resolve_aggregate(&AggregateRule::Inherit, rate(50, 50), None, None).is_err());
        assert!(
            resolve_aggregate(
                &AggregateRule::Fixed(rate(50, 50)),
                rate(0, 0),
                Some(rate(0, 50)),
                None
            )
            .is_err()
        );
        assert!(
            resolve_aggregate(
                &automatic(fraction(1, 1), fraction(1, 1)),
                rate(0, 0),
                None,
                None
            )
            .is_err()
        );
        assert!(
            resolve_aggregate(&AggregateRule::Fixed(rate(0, 50)), rate(0, 0), None, None).is_err()
        );
        assert!(
            resolve_aggregate(
                &AggregateRule::Fixed(rate(50, 50)),
                rate(0, 0),
                None,
                Some(rate(50, 0))
            )
            .is_err()
        );
    }

    #[test]
    fn wide_multiplication_accepts_representable_results_and_rejects_overflow() {
        let base = rate(u64::MAX, u64::MAX);
        let rule = automatic(fraction(u64::MAX, u64::MAX), fraction(1, 1));
        assert_eq!(
            resolve_aggregate(&rule, base, None, None).unwrap().local,
            base
        );
        let overflow = automatic(fraction(2, 1), fraction(1, 1));
        assert!(resolve_aggregate(&overflow, base, Some(rate(1, 1)), None).is_err());
    }

    #[test]
    fn fractions_round_down_once_before_ancestor_constraints() {
        let resolved = resolve_aggregate(
            &automatic(fraction(2, 3), fraction(2, 3)),
            rate(10, 10),
            Some(rate(5, 4)),
            None,
        )
        .unwrap();
        assert_eq!(resolved.local, rate(6, 6));
        assert_eq!(resolved.ceiling, rate(5, 4));
    }
}
