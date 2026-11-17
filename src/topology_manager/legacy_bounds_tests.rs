use super::super::rates::{new_policy_defaults, resolve_subscriber};
use super::*;

fn defaults() -> crate::config::BandwidthDefaults {
    let mut defaults = crate::config::Config::default().bandwidth_defaults;
    defaults.min_rate_percentage = 0.1;
    defaults.safe_min_mbps = 0.1;
    defaults.max_floor_mbps = 0.2;
    defaults
}

#[test]
fn imported_controls_preserve_legacy_floors_and_asymmetric_rates() {
    let controls = import(&defaults()).unwrap();
    for (base, min, max) in [
        (
            RateBps {
                down: 50_000_000,
                up: 10_000_000,
            },
            RateBps {
                down: 5_000_000,
                up: 1_000_000,
            },
            RateBps {
                down: 50_000_000,
                up: 10_000_000,
            },
        ),
        (
            RateBps {
                down: 100_000,
                up: 199_000,
            },
            RateBps {
                down: 100_000,
                up: 100_000,
            },
            RateBps {
                down: 200_000,
                up: 200_000,
            },
        ),
    ] {
        let actual = resolve_subscriber(base, &new_policy_defaults(), &controls).unwrap();
        assert_eq!(actual.min, min);
        assert_eq!(actual.max, max);
    }
}

#[test]
fn imported_controls_match_legacy_at_rounding_and_floor_boundaries() {
    for percentage in [0.000001, 0.01, 0.1, 0.125, 0.333333, 1.0] {
        for safe_min in [0.1, 0.128, 0.5] {
            let mut defaults = defaults();
            defaults.min_rate_percentage = percentage;
            defaults.safe_min_mbps = safe_min;
            let controls = import(&defaults).unwrap();
            for down in [
                1,
                99_499,
                99_500,
                100_000,
                100_500,
                199_499,
                199_500,
                200_000,
                200_500,
                1_235_000,
                1_245_000,
                12_345_678,
                50_000_000,
                1_000_000_001,
            ] {
                let base = RateBps {
                    down,
                    up: 50_000_001,
                };
                let actual = resolve_subscriber(base, &new_policy_defaults(), &controls).unwrap();
                let legacy = crate::speed::circuit_rates(
                    base.try_into().unwrap(),
                    percentage,
                    safe_min,
                    0.2,
                );
                let min = RateBps::try_from(crate::speed::Rate {
                    down: legacy.down_min,
                    up: legacy.up_min,
                })
                .unwrap();
                let max = RateBps::try_from(crate::speed::Rate {
                    down: legacy.down_max,
                    up: legacy.up_max,
                })
                .unwrap();
                assert_eq!(
                    (actual.min, actual.max),
                    (min, max),
                    "base {down}, percentage {percentage}, floor {safe_min}"
                );
            }
        }
    }
}

#[test]
fn imports_reject_loss_of_precision_nonfinite_values_and_invalid_factors() {
    for value in [
        0.0,
        -0.1,
        1.1,
        0.0000001,
        0.10000000000000002,
        f64::NAN,
        f64::INFINITY,
    ] {
        let mut defaults = defaults();
        defaults.min_rate_percentage = value;
        assert!(import(&defaults).is_err(), "{value}");
    }
    for value in [0.0, -1.0, 0.05, 0.10000001, f64::NAN, f64::INFINITY] {
        let mut defaults = defaults();
        defaults.safe_min_mbps = value;
        assert!(import(&defaults).is_err(), "{value}");
    }
    let mut defaults = defaults();
    defaults.max_floor_mbps = 0.1;
    assert!(import(&defaults).is_err());
}

#[test]
fn imported_rules_are_strict_and_do_not_change_exact_percentage_rules() {
    assert!(
        serde_json::from_value::<RateRule>(serde_json::json!({"LegacyRoundedBase":{
            "factor":{"numerator":1,"denominator":10},"floor_bps":100_000,"unexpected":1
        }}))
        .is_err()
    );
    let controls = import(&defaults()).unwrap();
    let text = serde_json::to_string(&controls).unwrap();
    assert_eq!(
        serde_json::from_str::<SpeedOverrides>(&text).unwrap(),
        controls
    );
    let mut exact = new_policy_defaults();
    exact.down_min = RateRule::BaseFraction(Fraction {
        numerator: 1,
        denominator: 10,
    });
    let rates = resolve_subscriber(
        RateBps {
            down: 12_345_678,
            up: 10_000_000,
        },
        &exact,
        &SpeedOverrides::default(),
    )
    .unwrap();
    assert_eq!(rates.min.down, 1_234_567);
    assert_eq!(rates.max.down, 12_345_678);
    let base = RateBps {
        down: 1u64 << 53,
        up: 10_000_000,
    };
    assert!(resolve_subscriber(base, &exact, &controls).is_err());
}
