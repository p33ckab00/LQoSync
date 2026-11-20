//! Read-only physical identity. Never infer a connector from link speed or port order.
use super::Port;
use serde::{Deserialize, Serialize};
use std::{io::Read, path::Path, process::Stdio};
use tokio::io::AsyncReadExt;

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(default)]
pub struct Identity {
    pub pci: Option<String>,
    pub netdev: Option<String>,
    pub role: Option<String>,
    pub connector: Option<String>,
    pub confidence: Option<String>,
    pub evidence: Option<String>,
}
impl Identity {
    pub(super) fn valid(&self) -> bool {
        [
            &self.pci,
            &self.netdev,
            &self.role,
            &self.connector,
            &self.confidence,
            &self.evidence,
        ]
        .into_iter()
        .all(|v| v.as_ref().is_none_or(|v| super::text(v)))
    }
}
// PCI BDF is dddd:bb:ss.f (12 characters, with the dot at index 10).
fn canonical_pci(value: &str) -> bool {
    value.len() == 12
        && value.bytes().enumerate().all(|(i, b)| match i {
            4 | 7 => b == b':',
            10 => b == b'.',
            11 => matches!(b, b'0'..=b'7'),
            _ => b.is_ascii_hexdigit(),
        })
}
fn read(path: &Path, limit: u64) -> Option<String> {
    let mut bytes = Vec::new();
    std::fs::File::open(path)
        .ok()?
        .take(limit + 1)
        .read_to_end(&mut bytes)
        .ok()?;
    (bytes.len() as u64 <= limit)
        .then(|| String::from_utf8(bytes).ok())
        .flatten()
}
#[derive(Default, Deserialize)]
struct Config {
    #[serde(default)]
    bridge: Bridge,
}
#[derive(Default, Deserialize)]
struct Bridge {
    to_internet: Option<String>,
    to_network: Option<String>,
}
fn role(names: &[String], bridge: &Bridge) -> Option<String> {
    let wan = bridge
        .to_internet
        .as_ref()
        .is_some_and(|n| names.contains(n));
    let lan = bridge
        .to_network
        .as_ref()
        .is_some_and(|n| names.contains(n));
    match (wan, lan) {
        (true, true) => Some("WAN/LAN".into()),
        (true, false) => Some("WAN".into()),
        (false, true) => Some("LAN".into()),
        _ => None,
    }
}
/// SFP family alone does not establish the cage's speed capability.
pub(super) fn module(value: &str) -> Option<(&'static str, &'static str)> {
    let text = value.to_ascii_uppercase();
    if text.contains("QSFP28") {
        Some(("qsfp28", "confirmed"))
    } else if text.contains("QSFP+") || text.starts_with("0X0D") {
        Some(("qsfp+", "confirmed"))
    } else if text.contains("QSFP") {
        Some(("qsfp", "confirmed"))
    } else if text.contains("SFP28") {
        Some(("sfp28", "confirmed"))
    } else if text.contains("SFP+") {
        Some(("sfp+", "confirmed"))
    } else if text.contains("SFP") {
        Some(("sfp", "confirmed"))
    } else {
        None
    }
}
/// The utility and arguments are fixed. Kernel-owned interfaces only; never bind or reset a NIC.
async fn ethtool(name: &str, module: bool) -> Option<String> {
    if name.is_empty()
        || name.starts_with('-')
        || name.len() > 15
        || !name
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'_' | b'-' | b'.'))
    {
        return None;
    }
    tokio::time::timeout(std::time::Duration::from_millis(250), async {
        let mut command = tokio::process::Command::new("/usr/sbin/ethtool");
        if module {
            command.arg("-m");
        }
        let mut child = command
            .arg(name)
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .kill_on_drop(true)
            .spawn()
            .ok()?;
        let mut output = Vec::new();
        child
            .stdout
            .take()?
            .take(16385)
            .read_to_end(&mut output)
            .await
            .ok()?;
        if output.len() > 16384 {
            return None;
        }
        if !child.wait().await.ok()?.success() {
            return None;
        }
        String::from_utf8(output).ok()
    })
    .await
    .ok()
    .flatten()
}
pub(super) async fn enrich(ports: &mut [Port], config: &Path) {
    let config = read(config, 262144)
        .and_then(|s| toml::from_str::<Config>(&s).ok())
        .unwrap_or_default();
    for port in ports.iter_mut() {
        let Some(address) = port.name.as_deref().filter(|name| canonical_pci(name)) else {
            continue;
        };
        port.identity.pci = Some(address.to_ascii_lowercase());
        let root = Path::new("/sys/bus/pci/devices").join(address).join("net");
        let mut names = std::fs::read_dir(root)
            .into_iter()
            .flatten()
            .filter_map(|v| v.ok()?.file_name().into_string().ok())
            .take(16)
            .collect::<Vec<_>>();
        names.sort();
        let mut mapping = vec![address.to_string()];
        if names.len() == 1 {
            mapping.extend(names.iter().cloned());
        }
        port.identity.role = role(&mapping, &config.bridge);
        if names.len() != 1 {
            continue;
        }
        port.identity.netdev = Some(names[0].clone());
    }
    // Complete all basic identities before optional driver reads can time out.
    for port in ports {
        let Some(name) = port.identity.netdev.clone() else {
            continue;
        };
        if port.identity.connector.is_some() {
            continue;
        }
        if let Some(output) = ethtool(&name, true).await {
            let identifier = output
                .lines()
                .find(|line| line.trim_start().starts_with("Identifier"));
            if let Some((connector, confidence)) = identifier.and_then(module) {
                port.identity.connector = Some(connector.into());
                port.identity.confidence = Some(confidence.into());
                port.identity.evidence = Some("Kernel module identifier".into());
                continue;
            }
        }
        if let Some(output) = ethtool(&name, false).await
            && output
                .lines()
                .any(|line| line.trim() == "Port: Twisted Pair")
        {
            port.identity.connector = Some("rj45".into());
            port.identity.confidence = Some("inferred".into());
            port.identity.evidence = Some("Kernel twisted-pair media; cage not verified".into());
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn identity_never_guesses_from_port_number_or_rate() {
        assert!(canonical_pci("0000:01:00.0"));
        assert!(!canonical_pci("../../etc/shadow"));
        assert!(module("10000Mb/s").is_none());
        assert_eq!(module("0x03 (SFP)"), Some(("sfp", "confirmed")));
        assert_eq!(module("0x11 (QSFP28)"), Some(("qsfp28", "confirmed")));
        assert!(module("unknown").is_none());
    }
    #[test]
    fn roles_require_explicit_interface_mapping() {
        let b = Bridge {
            to_internet: Some("wan0".into()),
            to_network: Some("lan0".into()),
        };
        assert_eq!(role(&["lan0".into()], &b), Some("LAN".into()));
        assert_eq!(role(&["other".into()], &b), None);
        assert_eq!(
            role(&["wan0".into(), "lan0".into()], &b),
            Some("WAN/LAN".into())
        );
    }
}
