//! Input import comparison. Import confirmation alone is not dataplane acceptance.
use crate::model::CsvRow;
use anyhow::{Context, ensure};
use serde::Deserialize;
use serde_json::Value;
use std::{collections::BTreeMap, net::IpAddr};

#[derive(Debug, Clone, PartialEq, Deserialize)]
struct Device {
    circuit_id: String,
    circuit_name: String,
    device_id: String,
    device_name: String,
    parent_node: String,
    #[serde(default)]
    parent_node_id: Option<String>,
    #[serde(default)]
    anchor_node_id: Option<String>,
    mac: String,
    #[serde(default)]
    comment: String,
    ipv4: Vec<(std::net::Ipv4Addr, u8)>,
    ipv6: Vec<(std::net::Ipv6Addr, u8)>,
    download_min_mbps: f32,
    upload_min_mbps: f32,
    download_max_mbps: f32,
    upload_max_mbps: f32,
}
impl Device {
    fn normalize(&mut self) -> anyhow::Result<()> {
        ensure!(
            [
                self.download_min_mbps,
                self.upload_min_mbps,
                self.download_max_mbps,
                self.upload_max_mbps
            ]
            .into_iter()
            .all(|n| n.is_finite() && n >= 0.0),
            "invalid imported rate"
        );
        ensure!(
            self.ipv4.iter().all(|(_, p)| *p <= 32) && self.ipv6.iter().all(|(_, p)| *p <= 128),
            "invalid imported prefix"
        );
        self.ipv4.sort_unstable();
        self.ipv4.dedup();
        self.ipv6.sort_unstable();
        self.ipv6.dedup();
        let mac = self.mac.replace('-', ":");
        if mac.len() == 17
            && mac.split(':').count() == 6
            && mac
                .split(':')
                .all(|octet| octet.len() == 2 && octet.bytes().all(|b| b.is_ascii_hexdigit()))
        {
            self.mac = mac.to_ascii_lowercase();
        }
        Ok(())
    }
}

/// Parse desired data once per operation; compare a parsed import without rebuilding it.
#[derive(Debug)]
pub struct ExpectedImport {
    network: Value,
    devices: BTreeMap<(String, String), Device>,
}
impl ExpectedImport {
    pub fn new(network: &str, rows: &[CsvRow]) -> anyhow::Result<Self> {
        let network: Value = serde_json::from_str(network).context("invalid expected network")?;
        ensure!(network.is_object(), "expected network must be an object");
        let mut devices = BTreeMap::new();
        for r in rows {
            let rate = |s: &str| -> anyhow::Result<f32> {
                Ok(if s.trim().is_empty() {
                    0.0
                } else {
                    s.trim().parse()?
                })
            };
            let mut d = Device {
                circuit_id: r.circuit_id.trim().to_string(),
                circuit_name: r.circuit_name.trim().to_string(),
                device_id: r.device_id.trim().to_string(),
                device_name: r.device_name.trim().to_string(),
                parent_node: r.parent_node.trim().to_string(),
                parent_node_id: (!r.parent_node_id.trim().is_empty())
                    .then(|| r.parent_node_id.trim().to_string()),
                anchor_node_id: (!r.anchor_node_id.trim().is_empty())
                    .then(|| r.anchor_node_id.trim().to_string()),
                mac: r.mac.trim().to_string(),
                comment: r.comment.trim().to_string(),
                ipv4: addresses(&r.ipv4, 32)?,
                ipv6: addresses(&r.ipv6, 128)?,
                download_min_mbps: rate(&r.download_min)?,
                upload_min_mbps: rate(&r.upload_min)?,
                download_max_mbps: rate(&r.download_max)?,
                upload_max_mbps: rate(&r.upload_max)?,
            };
            d.normalize()?;
            ensure!(
                devices
                    .insert((d.circuit_id.clone(), d.device_id.clone()), d)
                    .is_none(),
                "duplicate expected device identity"
            );
        }
        Ok(Self { network, devices })
    }

    pub fn matches(&self, artifact: &Value) -> bool {
        self.compare(artifact).unwrap_or(false)
    }

    fn compare(&self, artifact: &Value) -> anyhow::Result<bool> {
        let imp = &artifact["imported"];
        let list = imp["shaped_devices"]
            .as_array()
            .context("imported devices missing")?;
        if list.len() != self.devices.len()
            || !same_json(&imp["compatibility_network_json"], &self.network)
        {
            return Ok(false);
        }
        let mut seen = std::collections::HashSet::with_capacity(list.len());
        for item in list {
            let mut got: Device = Device::deserialize(item).context("invalid imported device")?;
            got.normalize()?;
            let key = (got.circuit_id.clone(), got.device_id.clone());
            let Some((identity, want)) = self.devices.get_key_value(&key) else {
                return Ok(false);
            };
            ensure!(seen.insert(identity), "duplicate imported device identity");
            // An absent CSV parent ID is resolved by LibreQoS. An explicit ID must match.
            if want.parent_node_id.is_none() {
                got.parent_node_id = None;
            }
            if &got != want {
                return Ok(false);
            }
        }
        Ok(true)
    }
}

fn addresses<T: std::str::FromStr>(text: &str, width: u8) -> anyhow::Result<Vec<(T, u8)>> {
    text.split(',')
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(|s| {
            let (addr, prefix) = s.split_once('/').map_or((s, None), |(a, p)| (a, Some(p)));
            let parsed: IpAddr = addr.parse().context("invalid desired address")?;
            ensure!(
                parsed.is_ipv4() == (width == 32),
                "wrong desired address family"
            );
            let prefix: u8 = prefix.map(str::parse).transpose()?.unwrap_or(width);
            ensure!(prefix <= width, "invalid desired prefix");
            Ok((
                addr.parse::<T>()
                    .map_err(|_| anyhow::anyhow!("invalid desired address family"))?,
                prefix,
            ))
        })
        .collect()
}

/// Network equality includes numeric representation differences such as 620 and 620.0.
fn same_json(a: &Value, b: &Value) -> bool {
    use Value::{Array, Number, Object};
    match (a, b) {
        (Number(x), Number(y)) => {
            if x == y {
                return true;
            }
            if x.is_f64() == y.is_f64() {
                return false;
            }
            let integer = if x.is_f64() { y } else { x };
            // Permit representation changes only where f64 preserves every integer bit.
            const EXACT: i64 = 1i64 << 53;
            integer
                .as_i64()
                .is_some_and(|n| (-EXACT..=EXACT).contains(&n))
                && x.as_f64() == y.as_f64()
        }
        (Array(x), Array(y)) => x.len() == y.len() && x.iter().zip(y).all(|(p, q)| same_json(p, q)),
        (Object(x), Object(y)) => {
            x.len() == y.len()
                && x.iter()
                    .all(|(k, p)| y.get(k).is_some_and(|q| same_json(p, q)))
        }
        _ => a == b,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn row() -> CsvRow {
        CsvRow {
            circuit_id: "C".into(),
            device_id: "D".into(),
            circuit_name: "Client".into(),
            device_name: "Device".into(),
            parent_node: "P".into(),
            parent_node_id: "node-id".into(),
            ipv4: "100.64.0.2, 100.64.0.1/24".into(),
            ipv6: "2001:db8::1".into(),
            mac: "AA-BB-CC-DD-EE-FF".into(),
            download_min: "1".into(),
            upload_min: "2".into(),
            download_max: "20".into(),
            upload_max: "10".into(),
            ..CsvRow::default()
        }
    }
    fn device() -> Value {
        json!({"circuit_id":"C","device_id":"D","circuit_name":"Client","device_name":"Device",
            "parent_node":"P","parent_node_id":"node-id","mac":"aa:bb:cc:dd:ee:ff",
            "ipv4":[["100.64.0.1",24],["100.64.0.2",32]],"ipv6":[["2001:db8:0::1",128]],
            "download_min_mbps":1,"upload_min_mbps":2,"download_max_mbps":20,"upload_max_mbps":10})
    }
    fn artifact(devices: Vec<Value>) -> Value {
        json!({"imported":{"compatibility_network_json":{"P":{"downloadBandwidthMbps":20.0}},"shaped_devices":devices}})
    }
    #[test]
    fn equivalent_address_order_and_numeric_notation_match() {
        let expected =
            ExpectedImport::new(r#"{"P":{"downloadBandwidthMbps":20}}"#, &[row()]).unwrap();
        assert!(expected.matches(&artifact(vec![device()])));
    }
    #[test]
    fn missing_fields_and_changed_identity_or_policy_fail_closed() {
        let expected =
            ExpectedImport::new(r#"{"P":{"downloadBandwidthMbps":20}}"#, &[row()]).unwrap();
        for key in [
            "circuit_id",
            "device_id",
            "parent_node_id",
            "circuit_name",
            "device_name",
            "mac",
            "ipv4",
            "ipv6",
            "download_min_mbps",
            "upload_min_mbps",
            "download_max_mbps",
            "upload_max_mbps",
        ] {
            let mut d = device();
            d.as_object_mut().unwrap().remove(key);
            assert!(!expected.matches(&artifact(vec![d])), "missing {key}");
        }
        let mut d = device();
        d["ipv4"] = json!([["100.64.0.2", 33]]);
        assert!(!expected.matches(&artifact(vec![d])));
    }
    #[test]
    fn duplicate_identities_cannot_hide_a_missing_device() {
        let mut second = row();
        second.device_id = "D2".into();
        let expected =
            ExpectedImport::new(r#"{"P":{"downloadBandwidthMbps":20}}"#, &[row(), second]).unwrap();
        assert!(!expected.matches(&artifact(vec![device(), device()])));
        assert!(ExpectedImport::new("{}", &[row(), row()]).is_err());
    }
    #[test]
    fn imported_rates_use_the_producer_float32_precision() {
        let mut r = row();
        r.download_min = "1.1".into();
        let mut d = device();
        d["download_min_mbps"] = json!(f64::from(1.1f32));
        let expected = ExpectedImport::new(r#"{"P":{"downloadBandwidthMbps":20}}"#, &[r]).unwrap();
        assert!(expected.matches(&artifact(vec![d])));
    }
    #[test]
    fn mac_usernames_keep_their_case() {
        let mut r = row();
        r.mac = "SubscriberABC".into();
        let mut d = device();
        d["mac"] = json!("subscriberabc");
        let expected = ExpectedImport::new(r#"{"P":{"downloadBandwidthMbps":20}}"#, &[r]).unwrap();
        assert!(!expected.matches(&artifact(vec![d])));
    }
    #[test]
    fn explicit_anchor_ids_are_compared_in_the_imported_device() {
        let mut r = row();
        r.anchor_node_id = "anchor-A".into();
        let mut d = device();
        d["anchor_node_id"] = json!("anchor-A");
        let expected = ExpectedImport::new(r#"{"P":{"downloadBandwidthMbps":20}}"#, &[r]).unwrap();
        assert!(expected.matches(&artifact(vec![d.clone()])));
        d["anchor_node_id"] = json!("anchor-B");
        assert!(!expected.matches(&artifact(vec![d])));
    }

    #[test]
    fn expected_fields_follow_the_producer_csv_trimming() {
        let mut r = row();
        r.circuit_id = " C ".into();
        r.device_id = " D ".into();
        r.circuit_name = " Client ".into();
        r.device_name = " Device ".into();
        r.parent_node = " P ".into();
        r.mac = " AA-BB-CC-DD-EE-FF ".into();
        let expected = ExpectedImport::new(r#"{"P":{"downloadBandwidthMbps":20}}"#, &[r]).unwrap();
        assert!(expected.matches(&artifact(vec![device()])));
    }

    #[test]
    fn invalid_desired_inputs_cannot_match_an_invalid_import() {
        for (ip, rate) in [
            ("not-ip", "1"),
            ("100.64.0.1/33", "1"),
            ("2001:db8::1", "1"),
            ("100.64.0.1", "NaN"),
            ("100.64.0.1", "-1"),
        ] {
            let mut r = row();
            r.ipv4 = ip.into();
            r.download_min = rate.into();
            assert!(ExpectedImport::new("{}", &[r]).is_err());
        }
        assert!(ExpectedImport::new("[]", &[row()]).is_err());
    }
}

#[cfg(test)]
mod numeric_integrity_tests {
    #[test]
    fn numeric_equivalence_must_not_hide_integer_precision_loss() {
        let value = |s: &str| serde_json::from_str::<serde_json::Value>(s).unwrap();
        assert!(super::same_json(&value("620"), &value("620.0")));
        assert!(!super::same_json(
            &value("9007199254740992"),
            &value("9007199254740993")
        ));
        assert!(!super::same_json(
            &value("9007199254740993"),
            &value("9007199254740992.0")
        ));
    }
}
