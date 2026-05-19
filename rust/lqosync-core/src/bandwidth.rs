use regex::Regex;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BandwidthPair {
    pub download_mbps: f64,
    pub upload_mbps: f64,
}

pub fn convert_to_mbps(value: &str) -> f64 {
    let value = value.trim();
    if value.is_empty() {
        return 0.0;
    }
    let re = Regex::new(r"(?i)^(\d+(?:\.\d+)?)([kmg]?)$").expect("valid unit regex");
    let Some(caps) = re.captures(value) else {
        return 0.0;
    };
    let number = caps
        .get(1)
        .and_then(|m| m.as_str().parse::<f64>().ok())
        .unwrap_or(0.0);
    let unit = caps.get(2).map(|m| m.as_str().to_ascii_lowercase()).unwrap_or_default();
    match unit.as_str() {
        "k" => number * 0.001,
        "m" | "" => number,
        "g" => number * 1000.0,
        _ => number,
    }
}

pub fn parse_rate_limit(rate_limit: &str) -> BandwidthPair {
    let primary = rate_limit.trim().split_whitespace().next().unwrap_or("");
    let mut parts = primary.split('/');
    let rx = parts.next().map(convert_to_mbps).unwrap_or(0.0);
    let tx = parts.next().map(convert_to_mbps).unwrap_or(0.0);
    if parts.next().is_some() {
        return BandwidthPair { download_mbps: 0.0, upload_mbps: 0.0 };
    }
    BandwidthPair { download_mbps: rx, upload_mbps: tx }
}

pub fn parse_comment_bandwidth(comment: &str) -> Option<BandwidthPair> {
    let comment = comment.trim();
    if comment.is_empty() {
        return None;
    }
    for pattern in [r"(?i)\|(\d+(?:\.\d+)?)M/(\d+(?:\.\d+)?)M", r"(?i)(\d+(?:\.\d+)?)M/(\d+(?:\.\d+)?)M"] {
        let re = Regex::new(pattern).expect("valid bandwidth regex");
        if let Some(caps) = re.captures(comment) {
            let rx = caps.get(1)?.as_str().parse::<f64>().ok()?;
            let tx = caps.get(2)?.as_str().parse::<f64>().ok()?;
            return Some(BandwidthPair { download_mbps: rx, upload_mbps: tx });
        }
    }
    let single = Regex::new(r"(?i)(\d+(?:\.\d+)?)M").expect("valid single speed regex");
    if let Some(caps) = single.captures(comment) {
        let speed = caps.get(1)?.as_str().parse::<f64>().ok()?;
        return Some(BandwidthPair { download_mbps: speed, upload_mbps: speed });
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn converts_units_to_mbps() {
        assert_eq!(convert_to_mbps("512k"), 0.512);
        assert_eq!(convert_to_mbps("15M"), 15.0);
        assert_eq!(convert_to_mbps("1G"), 1000.0);
        assert_eq!(convert_to_mbps("25"), 25.0);
        assert_eq!(convert_to_mbps("garbage"), 0.0);
    }

    #[test]
    fn parses_routeros_rate_limit() {
        assert_eq!(parse_rate_limit("10M/5M"), BandwidthPair { download_mbps: 10.0, upload_mbps: 5.0 });
        assert_eq!(parse_rate_limit("1024k/512k 1M/1M"), BandwidthPair { download_mbps: 1.024, upload_mbps: 0.512 });
        assert_eq!(parse_rate_limit("bad"), BandwidthPair { download_mbps: 0.0, upload_mbps: 0.0 });
    }

    #[test]
    fn parses_comment_bandwidth() {
        assert_eq!(parse_comment_bandwidth("PLAN|25M/10M").unwrap(), BandwidthPair { download_mbps: 25.0, upload_mbps: 10.0 });
        assert_eq!(parse_comment_bandwidth("profile 30M/30M").unwrap(), BandwidthPair { download_mbps: 30.0, upload_mbps: 30.0 });
        assert_eq!(parse_comment_bandwidth("15M").unwrap(), BandwidthPair { download_mbps: 15.0, upload_mbps: 15.0 });
        assert!(parse_comment_bandwidth("no speed").is_none());
    }
}
