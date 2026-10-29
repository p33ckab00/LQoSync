use crate::routeros::Row;
use anyhow::{Context, Result, bail, ensure};
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::sync::LazyLock;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct SpeedRules {
    pub min_rate_fraction: f64,
    pub min_floor_mbps: f64,
    pub profile_name_fallback: bool,
}
impl Default for SpeedRules {
    fn default() -> Self {
        Self {
            min_rate_fraction: 0.5,
            min_floor_mbps: 0.128,
            profile_name_fallback: true,
        }
    }
}
impl SpeedRules {
    pub fn validate(&self) -> Result<()> {
        ensure!(
            self.min_rate_fraction.is_finite() && (0.0..=1.0).contains(&self.min_rate_fraction),
            "Minimum rate fraction must be 0–1"
        );
        ensure!(
            self.min_floor_mbps.is_finite() && self.min_floor_mbps >= 0.0,
            "Minimum floor must be finite and nonnegative"
        );
        Ok(())
    }
}
#[derive(Debug, Clone, Serialize)]
pub struct Rates {
    pub download: f64,
    pub upload: f64,
    pub download_min: f64,
    pub upload_min: f64,
}
#[derive(Debug, Clone, Serialize)]
pub struct ResolvedSpeed {
    pub rates: Rates,
    pub source: &'static str,
    pub raw: String,
    pub burst_ignored: bool,
}

fn scalar(raw: &str, bare_multiplier: f64) -> Result<f64> {
    let (number, multiplier) = match raw.as_bytes().last().map(u8::to_ascii_lowercase) {
        Some(b'k') => (&raw[..raw.len() - 1], 0.001),
        Some(b'm') => (&raw[..raw.len() - 1], 1.0),
        Some(b'g') => (&raw[..raw.len() - 1], 1000.0),
        _ => (raw, bare_multiplier),
    };
    ensure!(
        !number.is_empty()
            && number.bytes().all(|b| b.is_ascii_digit() || b == b'.')
            && number.bytes().filter(|b| *b == b'.').count() <= 1,
        "Invalid rate number or unit"
    );
    let value = number.parse::<f64>()? * multiplier;
    ensure!(
        value.is_finite() && value >= 0.0,
        "Rate must be finite and nonnegative"
    );
    Ok(value)
}
fn pair(raw: &str, bare_multiplier: f64) -> Result<(f64, f64)> {
    let mut values = raw.split('/');
    let first = scalar(values.next().context("Missing rate")?, bare_multiplier)?;
    let second = values
        .next()
        .map(|s| scalar(s, bare_multiplier))
        .transpose()?
        .unwrap_or(first);
    ensure!(values.next().is_none(), "Rate pair has too many directions");
    Ok((first, second))
}
fn resolved(
    download: f64,
    upload: f64,
    minimum: Option<(f64, f64)>,
    source: &'static str,
    raw: &str,
    burst_ignored: bool,
    rules: &SpeedRules,
) -> Result<ResolvedSpeed> {
    rules.validate()?;
    ensure!(
        download.is_finite() && upload.is_finite() && download > 0.0 && upload > 0.0,
        "No finite positive maximum; unlimited rates need an explicit plan"
    );
    let (download_min, upload_min) = minimum.unwrap_or_else(|| {
        (
            (download * rules.min_rate_fraction)
                .max(rules.min_floor_mbps)
                .min(download),
            (upload * rules.min_rate_fraction)
                .max(rules.min_floor_mbps)
                .min(upload),
        )
    });
    ensure!(
        [download_min, upload_min]
            .iter()
            .all(|v| v.is_finite() && *v >= 0.0)
            && download_min <= download
            && upload_min <= upload,
        "Minimum exceeds maximum or is invalid"
    );
    Ok(ResolvedSpeed {
        rates: Rates {
            download,
            upload,
            download_min,
            upload_min,
        },
        source,
        raw: raw.into(),
        burst_ignored,
    })
}

pub fn parse_router_rate(raw: &str, rules: &SpeedRules) -> Result<ResolvedSpeed> {
    ensure!(raw.len() <= 4096, "Rate field is too long");
    let fields: Vec<_> = raw.split_whitespace().collect();
    ensure!(
        !fields.is_empty() && fields.len() <= 6,
        "Invalid RouterOS rate fields"
    );
    // Router receive is customer upload; transmit is customer download.
    let (upload, download) = pair(fields[0], 0.000001)?;
    for field in fields.iter().take(3).skip(1) {
        pair(field, 0.000001)?;
    }
    if let Some(time) = fields.get(3) {
        for part in time.split('/') {
            let seconds = part.strip_suffix('s').unwrap_or(part);
            ensure!(
                seconds.bytes().all(|b| b.is_ascii_digit() || b == b'.'),
                "Invalid burst time unit"
            );
            scalar(seconds, 1.0)?;
        }
        ensure!(time.split('/').count() <= 2, "Invalid burst time pair");
    }
    if let Some(priority) = fields.get(4) {
        ensure!(
            (1..=8).contains(&priority.parse::<u8>()?),
            "Invalid RouterOS priority"
        );
    }
    let minimum = fields
        .get(5)
        .map(|s| pair(s, 0.000001).map(|(u, d)| (d, u)))
        .transpose()?;
    resolved(
        download,
        upload,
        minimum,
        "router_rate_limit",
        raw,
        fields.len() > 1,
        rules,
    )
}

fn text<'a>(row: &'a Row, key: &str) -> &'a str {
    row.get(key).map(|v| v.trim()).unwrap_or("")
}
// Compile once. Never compile expressions inside subscriber/profile loops.
static COMMENT: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)[+-]?(?:[0-9]+(?:\.[0-9]+)?|\.[0-9]+)[kmg][^\s|,;]*")
        .expect("constant comment expression")
});
static PROFILE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)((?:[0-9]+(?:\.[0-9]+)?|\.[0-9]+)[kmg])$")
        .expect("constant profile expression")
});
pub(crate) fn comment(
    raw: &str,
    source: &'static str,
    rules: &SpeedRules,
) -> Result<Option<ResolvedSpeed>> {
    ensure!(raw.len() <= 4096, "Comment field is too long");
    let mut matches = COMMENT.find_iter(raw);
    let Some(found) = matches.next() else {
        return Ok(None);
    };
    let prefix = &raw[..found.start()];
    ensure!(
        !prefix
            .chars()
            .next_back()
            .is_some_and(|c| c.is_alphanumeric() || matches!(c, '/' | '.' | '−' | '–'))
            && !prefix.trim_end().ends_with('/'),
        "Malformed comment speed token"
    );
    ensure!(
        matches.next().is_none(),
        "Ambiguous comment contains multiple speed rules"
    );
    let (download, upload) = pair(found.as_str(), 1.0)?;
    resolved(download, upload, None, source, raw, false, rules).map(Some)
}
fn rate(raw: &str, source: &'static str, rules: &SpeedRules) -> Result<ResolvedSpeed> {
    let mut resolved = parse_router_rate(raw, rules)?;
    resolved.source = source;
    Ok(resolved)
}

pub fn resolve_ppp(
    secret: &Row,
    active: &Row,
    profile: Option<&Row>,
    rules: &SpeedRules,
) -> Result<ResolvedSpeed> {
    for (row, source) in [
        (secret, "ppp_secret_comment"),
        (active, "ppp_active_comment"),
    ] {
        if let Some(speed) = comment(text(row, "comment"), source, rules)? {
            return Ok(speed);
        }
    }
    let profile = profile.context("PPP profile is missing; no default maximum is assigned")?;
    if let Some(speed) = comment(text(profile, "comment"), "ppp_profile_comment", rules)? {
        return Ok(speed);
    }
    let raw = text(profile, "rate-limit");
    if !raw.is_empty() {
        return rate(raw, "ppp_profile_rate_limit", rules);
    }
    if rules.profile_name_fallback {
        let name = text(profile, "name");
        ensure!(name.len() <= 4096, "Profile name is too long");
        if let Some(capture) = PROFILE.captures(name) {
            let prefix = &name[..capture.get(1).expect("capture exists").start()];
            let last = prefix.chars().next_back();
            let exponent_prefix = prefix.strip_suffix(['+', '-']).unwrap_or(prefix);
            let mut chars = exponent_prefix.chars().rev();
            let exponent = chars.next().is_some_and(|c| c == 'e' || c == 'E')
                && chars.next().is_some_and(|c| c.is_ascii_digit() || c == '.');
            ensure!(
                !last.is_some_and(|c| c.is_ascii_digit() || matches!(c, '.' | '/')) && !exponent,
                "Malformed profile speed token"
            );
            let value = scalar(&capture[1], 1.0)?;
            return resolved(value, value, None, "ppp_profile_name", name, false, rules);
        }
    }
    bail!("PPP has no finite router-authored speed rule")
}
pub fn resolve_dhcp(lease: &Row, rules: &SpeedRules) -> Result<ResolvedSpeed> {
    if let Some(speed) = comment(text(lease, "comment"), "dhcp_lease_comment", rules)? {
        return Ok(speed);
    }
    rate(text(lease, "rate-limit"), "dhcp_lease_rate_limit", rules)
}
pub fn resolve_hotspot(
    user: &Row,
    profile: Option<&Row>,
    rules: &SpeedRules,
) -> Result<ResolvedSpeed> {
    if let Some(speed) = comment(text(user, "comment"), "hotspot_user_comment", rules)? {
        return Ok(speed);
    }
    let profile = profile.context("Hotspot profile is missing; no default maximum is assigned")?;
    if let Some(speed) = comment(text(profile, "comment"), "hotspot_profile_comment", rules)? {
        return Ok(speed);
    }
    rate(
        text(profile, "rate-limit"),
        "hotspot_profile_rate_limit",
        rules,
    )
}

pub use crate::speed_preview::{SpeedEntry, SpeedPreview, preview};
