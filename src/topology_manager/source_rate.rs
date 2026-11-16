//! Exact sustained rate input. Native/export admission is a later check.

use super::rates::RateBps;
use crate::config::{RateLimitOrder, UnitlessUnit};
use serde::{Deserialize, Serialize, Serializer};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RateFormat {
    #[serde(serialize_with = "serialize_order")]
    pub order: RateLimitOrder,
    #[serde(serialize_with = "serialize_units")]
    pub unitless: UnitlessUnit,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RateParseError {
    InvalidFormat,
    FractionalBps,
    OutOfRange,
    NonPositive,
}

impl std::fmt::Display for RateParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "invalid sustained source rate: {self:?}")
    }
}
impl std::error::Error for RateParseError {}

pub fn parse_rate_limit(
    text: Option<&str>,
    format: RateFormat,
) -> Result<Option<RateBps>, RateParseError> {
    let mut words = text.unwrap_or_default().split_whitespace();
    let Some(base) = words.next() else {
        return Ok(None);
    };
    // A separated slash is not a valid symmetric rate followed by burst fields.
    if words.next().is_some_and(|word| word.starts_with('/')) {
        return Err(RateParseError::InvalidFormat);
    }
    let (first, second) = match base.split_once('/') {
        Some((a, b)) => (bps(a, format.unitless)?, bps(b, format.unitless)?),
        None => {
            let value = bps(base, format.unitless)?;
            (value, value)
        }
    };
    Ok(Some(match format.order {
        RateLimitOrder::DownloadUpload => RateBps {
            down: first,
            up: second,
        },
        RateLimitOrder::RouterosRxTx => RateBps {
            down: second,
            up: first,
        },
    }))
}

fn bps(text: &str, unitless: UnitlessUnit) -> Result<u64, RateParseError> {
    let default = match unitless {
        UnitlessUnit::Bps => 1,
        UnitlessUnit::Mbps => 1_000_000,
    };
    let (number, scale) = match text.as_bytes().last().copied() {
        Some(b'k' | b'K') => (&text[..text.len() - 1], 1_000u128),
        Some(b'm' | b'M') => (&text[..text.len() - 1], 1_000_000u128),
        Some(b'g' | b'G') => (&text[..text.len() - 1], 1_000_000_000u128),
        _ => (text, default),
    };
    let (whole, fraction) = match number.split_once('.') {
        Some((whole, fraction)) if !fraction.is_empty() => (whole, fraction),
        Some(_) => return Err(RateParseError::InvalidFormat),
        None => (number, ""),
    };
    if whole.is_empty()
        || !whole.bytes().all(|b| b.is_ascii_digit())
        || !fraction.bytes().all(|b| b.is_ascii_digit())
    {
        return Err(RateParseError::InvalidFormat);
    }
    let integer = whole
        .parse::<u128>()
        .map_err(|_| RateParseError::OutOfRange)?;
    let integer = integer
        .checked_mul(scale)
        .ok_or(RateParseError::OutOfRange)?;
    let fraction = fraction.trim_end_matches('0');
    let fractional = if fraction.is_empty() {
        0
    } else {
        if fraction.len() > 9 {
            return Err(RateParseError::FractionalBps);
        }
        let denominator = 10u128.pow(fraction.len() as u32);
        let numerator = fraction
            .parse::<u128>()
            .map_err(|_| RateParseError::InvalidFormat)?
            * scale;
        if numerator % denominator != 0 {
            return Err(RateParseError::FractionalBps);
        }
        numerator / denominator
    };
    let value = integer
        .checked_add(fractional)
        .ok_or(RateParseError::OutOfRange)?;
    let value = u64::try_from(value).map_err(|_| RateParseError::OutOfRange)?;
    if value == 0 {
        return Err(RateParseError::NonPositive);
    }
    Ok(value)
}

fn serialize_order<S: Serializer>(
    value: &RateLimitOrder,
    serializer: S,
) -> Result<S::Ok, S::Error> {
    serializer.serialize_str(match value {
        RateLimitOrder::RouterosRxTx => "routeros_rx_tx",
        RateLimitOrder::DownloadUpload => "download_upload",
    })
}
fn serialize_units<S: Serializer>(value: &UnitlessUnit, serializer: S) -> Result<S::Ok, S::Error> {
    serializer.serialize_str(match value {
        UnitlessUnit::Bps => "bps",
        UnitlessUnit::Mbps => "mbps",
    })
}

#[cfg(test)]
#[path = "source_rate_tests.rs"]
mod tests;
