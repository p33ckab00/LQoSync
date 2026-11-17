//! Explicit compatibility controls. New policies keep their exact fixed/fraction behavior.
use super::rates::{Fraction, RateBps, RateRule, RuleValue, SpeedOverrides, SpeedPolicy};
use anyhow::ensure;

const SCALE: u64 = 1_000_000;

/// Return a draft edit only. Persist the completed profile edit through RegistryStore.
pub fn import(defaults: &crate::config::BandwidthDefaults) -> anyhow::Result<SpeedOverrides> {
    ensure!(
        defaults.safe_min_mbps >= 0.1,
        "legacy subscriber minimum floor must be at least 0.1 Mbps"
    );
    ensure!(
        defaults.max_floor_mbps >= 0.2,
        "legacy subscriber maximum floor must be at least 0.2 Mbps"
    );
    let minimum = RateBps::try_from(crate::speed::Rate {
        down: defaults.min_rate_percentage,
        up: defaults.min_rate_percentage,
    })?
    .down;
    let factor = Fraction {
        numerator: minimum,
        denominator: SCALE,
    };
    factor_value(factor)?;
    let floors = RateBps::try_from(crate::speed::Rate {
        down: defaults.safe_min_mbps,
        up: defaults.max_floor_mbps,
    })?;
    let min = RuleValue::Override(RateRule::LegacyRoundedBase {
        factor,
        floor_bps: floors.down,
    });
    let max = RuleValue::Override(RateRule::LegacyRoundedBase {
        factor: Fraction {
            numerator: 1,
            denominator: 1,
        },
        floor_bps: floors.down.max(floors.up),
    });
    Ok(SpeedOverrides {
        down_min: min,
        up_min: min,
        down_max: max,
        up_max: max,
    })
}

pub(super) fn factor_value(factor: Fraction) -> anyhow::Result<f64> {
    factor.validate()?;
    ensure!(
        factor.numerator <= factor.denominator,
        "legacy subscriber factor must not exceed one"
    );
    let scaled = u128::from(factor.numerator) * u128::from(SCALE);
    ensure!(
        scaled % u128::from(factor.denominator) == 0,
        "legacy subscriber factor requires exact six-decimal precision"
    );
    Ok((scaled / u128::from(factor.denominator)) as f64 / SCALE as f64)
}

pub(super) fn mbps(bps: u64) -> anyhow::Result<f64> {
    let rate = crate::speed::Rate::try_from(RateBps { down: bps, up: bps })?;
    Ok(rate.down)
}

pub(super) fn resolve(base: u64, factor: Fraction, floor_bps: u64) -> anyhow::Result<u64> {
    let factor = factor_value(factor)?;
    let value = crate::speed::clamp(mbps(base)? * factor, mbps(floor_bps)?);
    Ok(RateBps::try_from(crate::speed::Rate {
        down: value,
        up: value,
    })?
    .down)
}

pub(super) fn matches(
    defaults: &SpeedPolicy,
    actual: &SpeedOverrides,
    expected: &SpeedOverrides,
) -> bool {
    [
        (defaults.down_min, actual.down_min, expected.down_min),
        (defaults.up_min, actual.up_min, expected.up_min),
        (defaults.down_max, actual.down_max, expected.down_max),
        (defaults.up_max, actual.up_max, expected.up_max),
    ]
    .into_iter()
    .all(|(default, actual, expected)| {
        match (actual.select(default), expected.select(default)) {
            (
                RateRule::LegacyRoundedBase {
                    factor: a,
                    floor_bps: x,
                },
                RateRule::LegacyRoundedBase {
                    factor: b,
                    floor_bps: y,
                },
            ) => {
                x == y
                    && u128::from(a.numerator) * u128::from(b.denominator)
                        == u128::from(b.numerator) * u128::from(a.denominator)
            }
            _ => false,
        }
    })
}

#[cfg(test)]
#[path = "legacy_bounds_tests.rs"]
mod tests;
