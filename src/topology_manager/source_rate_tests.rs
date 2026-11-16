use super::*;
fn ros() -> RateFormat {
    RateFormat {
        order: RateLimitOrder::RouterosRxTx,
        unitless: UnitlessUnit::Bps,
    }
}
fn rate(down: u64, up: u64) -> Result<Option<RateBps>, RateParseError> {
    Ok(Some(RateBps { down, up }))
}

#[test]
fn router_order_and_explicit_legacy_order_are_distinct() {
    assert_eq!(
        parse_rate_limit(Some("2M/50M"), ros()),
        rate(50_000_000, 2_000_000)
    );
    assert_eq!(
        parse_rate_limit(
            Some("2M/50M"),
            RateFormat {
                order: RateLimitOrder::DownloadUpload,
                ..ros()
            }
        ),
        rate(2_000_000, 50_000_000)
    );
}
#[test]
fn single_values_are_symmetric_and_units_keep_decimal_scales() {
    for (text, bps) in [
        ("1", 1),
        ("2k", 2_000),
        ("3K", 3_000),
        ("4m", 4_000_000),
        ("5M", 5_000_000),
        ("6G", 6_000_000_000),
    ] {
        assert_eq!(parse_rate_limit(Some(text), ros()), rate(bps, bps));
    }
}
#[test]
fn unitless_mbps_is_an_explicit_compatibility_choice() {
    assert_eq!(parse_rate_limit(Some("2/10"), ros()), rate(10, 2));
    assert_eq!(
        parse_rate_limit(
            Some("2/10"),
            RateFormat {
                unitless: UnitlessUnit::Mbps,
                ..ros()
            }
        ),
        rate(10_000_000, 2_000_000)
    );
}
#[test]
fn decimal_values_are_exact_to_one_bps_without_rounding() {
    assert_eq!(
        parse_rate_limit(Some("0.000001M/1.234567M"), ros()),
        rate(1_234_567, 1)
    );
    assert_eq!(parse_rate_limit(Some("0.000000001G"), ros()), rate(1, 1));
    assert_eq!(
        parse_rate_limit(Some("1.0000000000000000000000M"), ros()),
        rate(1_000_000, 1_000_000)
    );
    for text in ["0.1", "0.0000001M", "0.0000000001G"] {
        assert_eq!(
            parse_rate_limit(Some(text), ros()),
            Err(RateParseError::FractionalBps)
        );
    }
}
#[test]
fn absent_text_stays_absent_without_a_default_rate() {
    for text in [None, Some(""), Some(" \n\t ")] {
        assert_eq!(parse_rate_limit(text, ros()), Ok(None));
    }
}
#[test]
fn zero_negative_and_malformed_values_fail_without_raw_input_in_errors() {
    for text in ["0", "0/1M", "1M/0"] {
        assert_eq!(
            parse_rate_limit(Some(text), ros()),
            Err(RateParseError::NonPositive)
        );
    }
    for text in [
        "-1M",
        "+1M",
        "1e6",
        "NaN",
        "inf",
        "1/2/3",
        "/1",
        "1/",
        "1..2M",
        ".5M",
        "1.M",
        "1T",
        "1M / 2M",
        "private-value",
    ] {
        let error = parse_rate_limit(Some(text), ros()).unwrap_err();
        assert_eq!(error, RateParseError::InvalidFormat, "{text}");
        assert!(!error.to_string().contains(text));
    }
}
#[test]
fn sustained_rate_does_not_use_burst_or_router_minimum_fields_as_policy() {
    assert_eq!(
        parse_rate_limit(Some("2M/10M 4M/20M 1M/8M 10/10 8 100k/200k"), ros()),
        rate(10_000_000, 2_000_000)
    );
}
#[test]
fn integer_rates_above_float_exactness_remain_exact_and_overflow_fails() {
    assert_eq!(
        parse_rate_limit(Some("9007199254740993/1"), ros()),
        rate(1, 9_007_199_254_740_993)
    );
    assert_eq!(
        parse_rate_limit(Some("18446744073709551615/1"), ros()),
        rate(1, u64::MAX)
    );
    for text in [
        "18446744073709551616",
        "18446744073709551615G",
        "99999999999999999999999999999999999999999999999999999",
    ] {
        assert_eq!(
            parse_rate_limit(Some(text), ros()),
            Err(RateParseError::OutOfRange)
        );
    }
}
#[test]
fn decimal_roundtrip_matches_integer_bps_across_a_large_input_set() {
    for bps in 1..=2_000u64 {
        let text = format!("0.{bps:06}M");
        assert_eq!(parse_rate_limit(Some(&text), ros()), rate(bps, bps));
    }
}
