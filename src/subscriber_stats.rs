//! Bounded shaped-host observations, joined to the bridge's applied slot names.
use anyhow::{Context, ensure};
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, HashSet},
    net::IpAddr,
    path::Path,
};

pub const LIMIT: usize = 512;
pub const MAX_AGE: u64 = 15;
pub const CACHE: &str = "subscriber-telemetry.json";
const MAX_BYTES: u64 = 1024 * 1024;

#[derive(Debug, Deserialize, Serialize)]
pub struct Snapshot {
    pub captured_at_millis: u64,
    pub hosts: Option<Vec<Host>>,
}
#[derive(Debug, Deserialize, Serialize)]
pub struct Id {
    pub circuit_id: u64,
    pub subscriber_ip: IpAddr,
}
#[derive(Debug, Deserialize, Serialize)]
pub struct Host {
    pub id: Id,
    pub actual: Option<crate::worker_util::traffic::Counters>,
    pub last_seen_millis: Option<u64>,
    pub rtt_samples_us: Option<Vec<u32>>,
    pub rtt_samples_down_us: Option<Vec<u32>>,
    pub rtt_samples_up_us: Option<Vec<u32>>,
    pub tcp_retransmits: Option<u64>,
    pub tcp_retransmits_down: Option<u64>,
    pub tcp_retransmits_up: Option<u64>,
}
#[derive(Debug, Deserialize)]
pub enum State {
    Report(Applied),
    Unavailable(String),
}
#[derive(Debug, Deserialize)]
pub struct Applied {
    pub generation: String,
    pub report_schema: u32,
    pub slot_order: Vec<u8>,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Mapping {
    pub generation: String,
    pub names: Vec<String>,
}
impl Applied {
    pub fn mapping(&self) -> anyhow::Result<Mapping> {
        ensure!(
            self.report_schema == 1 && text(&self.generation, 128) && !self.generation.is_empty(),
            "unsupported applied state"
        );
        let mut bytes = self.slot_order.as_slice();
        let names: Vec<String> =
            ciborium::from_reader(&mut bytes).context("slot order unavailable")?;
        ensure!(
            bytes.is_empty() && !names.is_empty() && names.len() <= 65536,
            "invalid slot order size"
        );
        let mut unique = HashSet::new();
        ensure!(
            names.iter().all(|n| text(n, 256) && unique.insert(n)),
            "invalid or duplicate slot names"
        );
        Ok(Mapping {
            generation: self.generation.clone(),
            names,
        })
    }
}
impl Snapshot {
    pub(crate) fn validate(&self) -> anyhow::Result<()> {
        ensure!(
            self.captured_at_millis > 0,
            "missing host producer timestamp"
        );
        let hosts = self.hosts.as_ref().context("host fields unavailable")?;
        ensure!(hosts.len() <= LIMIT, "too many host rows");
        let mut ids = HashSet::new();
        for h in hosts {
            ensure!(
                ids.insert((h.id.circuit_id, h.id.subscriber_ip)),
                "duplicate host identity"
            );
            ensure!(
                [
                    &h.rtt_samples_us,
                    &h.rtt_samples_down_us,
                    &h.rtt_samples_up_us
                ]
                .into_iter()
                .flatten()
                .all(|r| r.len() <= 512),
                "too many RTT samples"
            );
        }
        Ok(())
    }
}
fn text(s: &str, max: usize) -> bool {
    s.len() <= max && !s.chars().any(char::is_control)
}

pub async fn sample(path: &Path) -> anyhow::Result<(Mapping, Snapshot)> {
    let before = crate::worker_util::ipc::read_mapping(path).await?;
    let hosts = crate::worker_util::ipc::read_hosts(path).await?;
    let after = crate::worker_util::ipc::read_mapping(path).await?;
    ensure!(before == after, "applied mapping changed during host read");
    Ok((after, hosts))
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct Metrics {
    pub actual: Option<crate::worker_util::traffic::Counters>,
    pub down_mbps: Option<f64>,
    pub up_mbps: Option<f64>,
    pub rtt_ms: Option<f64>,
    pub rtt_down_ms: Option<f64>,
    pub rtt_up_ms: Option<f64>,
    pub retransmits: Option<u64>,
    pub retransmits_down: Option<u64>,
    pub retransmits_up: Option<u64>,
    pub seen_age_ms: Option<u64>,
}
impl Metrics {
    fn valid(&self) -> bool {
        [
            self.down_mbps,
            self.up_mbps,
            self.rtt_ms,
            self.rtt_down_ms,
            self.rtt_up_ms,
        ]
        .into_iter()
        .flatten()
        .all(|v| v.is_finite() && v >= 0.0)
    }
}
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Report {
    pub version: u32,
    pub updated: u64,
    pub stale: bool,
    pub reason: Option<String>,
    pub hosts: BTreeMap<String, BTreeMap<IpAddr, Metrics>>,
}
impl Default for Report {
    fn default() -> Self {
        Self {
            version: 1,
            updated: 0,
            stale: true,
            reason: Some("no subscriber sample yet".into()),
            hosts: BTreeMap::new(),
        }
    }
}
impl Report {
    pub fn live(&self, now: u64) -> bool {
        !self.stale && self.updated > 0 && now >= self.updated && now - self.updated <= MAX_AGE
    }
    fn valid(&self) -> bool {
        self.version == 1
            && self.hosts.len() <= LIMIT
            && self.hosts.values().map(BTreeMap::len).sum::<usize>() <= LIMIT
            && self.reason.as_ref().is_none_or(|s| text(s, 180))
            && self
                .hosts
                .iter()
                .all(|(c, ips)| !c.is_empty() && text(c, 256) && ips.values().all(Metrics::valid))
    }
    fn fail(&mut self, error: String) {
        self.stale = true;
        let mut reason: String = error
            .chars()
            .filter(|c| !c.is_control())
            .take(180)
            .collect();
        while reason.len() > 180 {
            reason.pop();
        }
        self.reason = Some(reason);
    }
}
type Key = (String, IpAddr);
type Baseline = (
    Mapping,
    u64,
    u64,
    BTreeMap<Key, crate::worker_util::traffic::Counters>,
);
#[derive(Default)]
pub(crate) struct Tracker {
    pub report: Report,
    previous: Option<Baseline>,
}
fn median(values: Option<&Vec<u32>>) -> Option<f64> {
    let mut v: Vec<_> = values?.iter().copied().filter(|v| *v > 0).collect();
    if v.is_empty() {
        return None;
    }
    v.sort_unstable();
    Some((u64::from(v[(v.len() - 1) / 2]) + u64::from(v[v.len() / 2])) as f64 / 2000.0)
}
impl Tracker {
    pub fn record(&mut self, sample: anyhow::Result<(Mapping, Snapshot)>, now: u64) {
        let result = sample.and_then(|(map, s)| {
            s.validate()?;
            let mut keys = HashSet::new();
            for h in s
                .hosts
                .as_ref()
                .unwrap()
                .iter()
                .filter(|h| h.id.circuit_id != 0)
            {
                let slot = usize::try_from(h.id.circuit_id - 1)?;
                let name = map
                    .names
                    .get(slot)
                    .context("host slot outside applied mapping")?;
                ensure!(
                    !name.is_empty() && text(name, 256) && keys.insert((name, h.id.subscriber_ip)),
                    "invalid host slot identity"
                );
            }
            Ok((map, s))
        });
        let (map, s) = match result {
            Ok(s) => s,
            Err(e) => {
                self.previous = None;
                self.report.fail(e.to_string());
                return;
            }
        };
        let same = self.previous.as_ref().is_some_and(|(m, at, wall, _)| {
            *m == map && s.captured_at_millis > *at && now >= *wall
        });
        let duplicate = self
            .previous
            .as_ref()
            .is_some_and(|(m, at, _, _)| *m == map && s.captured_at_millis == *at);
        if duplicate {
            self.report
                .fail("host producer snapshot has not advanced".into());
            return;
        }
        let mut baselines = BTreeMap::new();
        let mut hosts: BTreeMap<String, BTreeMap<IpAddr, Metrics>> = BTreeMap::new();
        for h in s
            .hosts
            .unwrap()
            .into_iter()
            .filter(|h| h.id.circuit_id != 0)
        {
            let circuit = map.names[h.id.circuit_id as usize - 1].clone();
            let key = (circuit.clone(), h.id.subscriber_ip);
            let mut m = Metrics {
                actual: h.actual,
                rtt_ms: median(h.rtt_samples_us.as_ref()),
                rtt_down_ms: median(h.rtt_samples_down_us.as_ref()),
                rtt_up_ms: median(h.rtt_samples_up_us.as_ref()),
                retransmits: h.tcp_retransmits,
                retransmits_down: h.tcp_retransmits_down,
                retransmits_up: h.tcp_retransmits_up,
                seen_age_ms: h
                    .last_seen_millis
                    .filter(|v| *v > 0)
                    .and_then(|at| s.captured_at_millis.checked_sub(at)),
                ..Metrics::default()
            };
            if let Some(current) = h.actual {
                if same
                    && let Some((_, at, wall, old)) = &self.previous
                    && s.captured_at_millis - *at <= MAX_AGE * 1000
                    && now - *wall <= MAX_AGE
                    && let Some(prev) = old.get(&key)
                    && let (Some(d), Some(u)) = (
                        current.download_bytes.checked_sub(prev.download_bytes),
                        current.upload_bytes.checked_sub(prev.upload_bytes),
                    )
                {
                    let elapsed = (s.captured_at_millis - *at) as f64;
                    m.down_mbps = Some(d as f64 * 0.008 / elapsed);
                    m.up_mbps = Some(u as f64 * 0.008 / elapsed);
                }
                baselines.insert(key, current);
            }
            if same {
                hosts
                    .entry(circuit)
                    .or_default()
                    .insert(h.id.subscriber_ip, m);
            }
        }
        self.previous = Some((map, s.captured_at_millis, now, baselines));
        self.report = Report {
            version: 1,
            updated: now,
            stale: !same,
            reason: (!same)
                .then(|| "waiting for stable applied mapping and producer progress".into()),
            hosts,
        };
    }
}
pub fn read_cache(path: &Path) -> Report {
    use std::io::Read;
    let read = || -> anyhow::Result<Report> {
        let mut bytes = Vec::new();
        std::fs::File::open(path)?
            .take(MAX_BYTES + 1)
            .read_to_end(&mut bytes)?;
        ensure!(
            bytes.len() as u64 <= MAX_BYTES,
            "subscriber cache too large"
        );
        let report: Report = serde_json::from_slice(&bytes)?;
        ensure!(report.valid(), "invalid subscriber cache");
        Ok(report)
    };
    read().unwrap_or_else(|_| Report {
        reason: Some("subscriber cache unavailable or invalid".into()),
        ..Report::default()
    })
}
fn publish(dir: &Path, report: &Report) -> anyhow::Result<()> {
    use std::os::unix::fs::PermissionsExt;
    ensure!(report.valid(), "invalid subscriber report");
    let bytes = serde_json::to_vec(report)?;
    ensure!(
        bytes.len() as u64 <= MAX_BYTES,
        "subscriber cache too large"
    );
    let tmp = dir.join(format!(".{CACHE}.tmp-{}", std::process::id()));
    std::fs::write(&tmp, bytes)?;
    std::fs::set_permissions(&tmp, std::fs::Permissions::from_mode(0o644))?;
    std::fs::rename(tmp, dir.join(CACHE))?;
    Ok(())
}

pub(crate) fn start(
    mut changes: tokio::sync::watch::Receiver<crate::worker_util::Settings>,
) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        let mut current = changes.borrow().clone();
        let mut tracker = Tracker::default();
        let mut tick = tokio::time::interval(std::time::Duration::from_secs(10));
        tick.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
        let mut warned = false;
        loop {
            tokio::select! {
                changed=changes.changed()=>{
                    if changed.is_err() {break;}
                    current=changes.borrow_and_update().clone();tracker=Tracker::default();tick.reset_immediately();
                }
                _=tick.tick()=>{
                    if !current.v3 {continue;}
                    let result=sample(Path::new(&current.socket)).await;
                    if *changes.borrow()!=current {continue;}
                    let now=std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_secs();
                    tracker.record(result,now);
                    if let Err(e)=publish(Path::new(&current.state),&tracker.report) {
                        if !warned {eprintln!("subscriber telemetry cache: {e}");}warned=true;
                    } else {warned=false;}
                }
            }
        }
    })
}

/// Only exact host addresses are eligible. Do not attribute a whole routed prefix to one host.
pub fn addresses(row: &crate::model::CsvRow) -> Option<Vec<IpAddr>> {
    let mut ips = HashSet::new();
    for part in row
        .ipv4
        .split(',')
        .chain(row.ipv6.split(','))
        .map(str::trim)
        .filter(|v| !v.is_empty())
    {
        let (ip, prefix) = part
            .split_once('/')
            .map_or((part, None), |(ip, p)| (ip, Some(p)));
        let ip: IpAddr = ip.parse().ok()?;
        if let Some(prefix) = prefix {
            let n: u8 = prefix.parse().ok()?;
            if n != if ip.is_ipv4() { 32 } else { 128 } {
                return None;
            }
        }
        if ip.is_unspecified() || ip.is_multicast() {
            return None;
        }
        ips.insert(ip);
        if ips.len() > 16 {
            return None;
        }
    }
    let mut ips: Vec<_> = ips.into_iter().collect();
    ips.sort_unstable();
    (!ips.is_empty()).then_some(ips)
}
pub fn owners(rows: &[crate::model::CsvRow]) -> BTreeMap<IpAddr, usize> {
    // Sweep prefix ranges once on row reload. Rendering only performs exact map lookups.
    let mut events: BTreeMap<(bool, u128), i64> = BTreeMap::new();
    let mut candidates = BTreeMap::new();
    for row in rows {
        if let Some(ips) = addresses(row) {
            for ip in ips {
                candidates.insert(ip, 0);
            }
        }
        let mut ranges = HashSet::new();
        for part in row
            .ipv4
            .split(',')
            .chain(row.ipv6.split(','))
            .map(str::trim)
            .filter(|p| !p.is_empty())
        {
            let (addr, prefix) = part
                .split_once('/')
                .map_or((part, None), |(a, p)| (a, Some(p)));
            let Ok(ip) = addr.parse::<IpAddr>() else {
                continue;
            };
            let (v6, n, bits) = match ip {
                IpAddr::V4(ip) => (false, u32::from(ip) as u128, 32u32),
                IpAddr::V6(ip) => (true, u128::from(ip), 128u32),
            };
            let prefix = match prefix {
                Some(p) => match p.parse::<u32>() {
                    Ok(p) if p <= bits => p,
                    _ => continue,
                },
                None => bits,
            };
            let mask = if prefix == 0 {
                0
            } else {
                u128::MAX << (bits - prefix)
            };
            let start = n & mask;
            let suffix = if bits - prefix == 128 {
                u128::MAX
            } else {
                (1u128 << (bits - prefix)) - 1
            };
            let end = start | suffix;
            if ranges.insert((v6, start, end)) {
                *events.entry((v6, start)).or_default() += 1;
                if let Some(next) = end.checked_add(1) {
                    *events.entry((v6, next)).or_default() -= 1;
                }
            }
        }
    }
    let mut sweep = events.into_iter().peekable();
    let mut count = 0i64;
    for (ip, owners) in &mut candidates {
        let key = match ip {
            IpAddr::V4(ip) => (false, u32::from(*ip) as u128),
            IpAddr::V6(ip) => (true, u128::from(*ip)),
        };
        while sweep.peek().is_some_and(|(at, _)| *at <= key) {
            let (_, delta) = sweep.next().unwrap();
            count += delta;
        }
        *owners = count.max(0) as usize;
    }
    candidates
}

impl Report {
    pub fn measurements(
        &self,
        row: &crate::model::CsvRow,
        owners: &BTreeMap<IpAddr, usize>,
        now: u64,
    ) -> Vec<String> {
        let mut lines = vec!["DPDK host observations | capped sample (max 512)".into()];
        if row.circuit_id.starts_with("observed:") || row.circuit_id.is_empty() {
            lines.push("Unavailable: no assigned shaping circuit".into());
            return lines;
        }
        let Some(ips) = addresses(row) else {
            lines.push(
                "Unavailable: requires exact host IPs (up to 16); routed prefixes unsupported"
                    .into(),
            );
            return lines;
        };
        lines.push(format!(
            "Host telemetry {} | age {}",
            if self.live(now) {
                "LIVE"
            } else {
                if self.hosts.is_empty() {
                    "unavailable"
                } else {
                    "STALE (last known)"
                }
            },
            now.checked_sub(self.updated)
                .filter(|_| self.updated > 0)
                .map_or("unknown".into(), |n| format!("{n}s"))
        ));
        if let Some(reason) = &self.reason {
            lines.push(format!("Host note: {reason}"));
        }
        for ip in ips {
            lines.push(format!("Host {ip} | exact circuit + IP"));
            if owners.get(&ip) != Some(&1) {
                lines.push("Unavailable: duplicate or unverified local IP ownership".into());
                continue;
            }
            let Some(m) = self
                .hosts
                .get(&row.circuit_id)
                .and_then(|hosts| hosts.get(&ip))
            else {
                lines.push("Not in capped sample; traffic and TCP metrics unknown".into());
                continue;
            };
            let value = |n: Option<f64>| n.map_or("unknown".into(), |v| format!("{v:.3}"));
            let count = |n: Option<u64>| n.map_or("unknown".into(), |v| v.to_string());
            lines.push(format!(
                "Delivered: down {} | up {}",
                crate::worker_util::traffic::format_rate(
                    self.live(now).then_some(m.down_mbps).flatten(),
                    true
                ),
                crate::worker_util::traffic::format_rate(
                    self.live(now).then_some(m.up_mbps).flatten(),
                    true
                )
            ));
            lines.push(format!(
                "Observed bytes: down {} | up {}",
                count(m.actual.map(|v| v.download_bytes)),
                count(m.actual.map(|v| v.upload_bytes))
            ));
            lines.push(format!(
                "RTT sample median ms: pooled {} | down {} | up {}",
                value(m.rtt_ms),
                value(m.rtt_down_ms),
                value(m.rtt_up_ms)
            ));
            lines.push(format!(
                "TCP retransmits observed: pooled {} | down {} | up {}",
                count(m.retransmits),
                count(m.retransmits_down),
                count(m.retransmits_up)
            ));
            lines.push(format!(
                "Last packet age at read: {}",
                m.seen_age_ms
                    .map_or("unknown".into(), |v| format!("{v} ms"))
            ));
        }
        lines.push("RTT uses retained samples; no ping. Retransmits are counters, not packet-loss percent.".into());
        lines.push("Absent hosts do not imply zero traffic or offline state. Per-IP values are not whole-circuit totals.".into());
        lines
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    fn mapping() -> Mapping {
        Mapping {
            generation: "g1".into(),
            names: vec!["c1".into()],
        }
    }
    fn snapshot(at: u64, down: u64, up: u64) -> Snapshot {
        serde_json::from_value(json!({"captured_at_millis":at,"hosts":[{
            "id":{"circuit_id":1,"subscriber_ip":"192.0.2.1"},"actual":{"download_bytes":down,"upload_bytes":up},
            "rtt_samples_us":[0,1000,3000],"rtt_samples_down_us":[1000,2000],"rtt_samples_up_us":[],
            "tcp_retransmits":3,"tcp_retransmits_down":2,"tcp_retransmits_up":1,"last_seen_millis":at.saturating_sub(100)}]})).unwrap()
    }
    #[test]
    fn subscriber_rates_require_stable_mapping_progress_and_consecutive_counter_observations() {
        let mut t = Tracker::default();
        t.record(Ok((mapping(), snapshot(1000, 100, 200))), 10);
        assert!(!t.report.live(10) && t.report.hosts.is_empty());
        t.record(Ok((mapping(), snapshot(11000, 1250100, 2500200))), 20);
        assert!(t.report.live(20) && t.report.live(35) && !t.report.live(36));
        let ip: IpAddr = "192.0.2.1".parse().unwrap();
        let h = &t.report.hosts["c1"][&ip];
        assert_eq!(h.down_mbps, Some(1.0));
        assert_eq!(h.up_mbps, Some(2.0));
        assert_eq!(h.rtt_ms, Some(2.0));
        assert_eq!(h.rtt_down_ms, Some(1.5));
        assert_eq!(h.rtt_up_ms, None);
        assert_eq!(h.seen_age_ms, Some(100));
        t.record(Ok((mapping(), snapshot(11000, 9999999, 9999999))), 30);
        assert!(!t.report.live(30));
        assert_eq!(t.report.updated, 20);
        assert_eq!(t.report.hosts["c1"][&ip].down_mbps, Some(1.0));
        t.record(Err(anyhow::anyhow!("socket unavailable")), 31);
        t.record(Ok((mapping(), snapshot(21000, 2000000, 3000000))), 40);
        assert!(!t.report.live(40));
        t.record(Ok((mapping(), snapshot(31000, 100, 200))), 50);
        assert!(t.report.live(50));
        assert_eq!(t.report.hosts["c1"][&ip].down_mbps, None);
        let mut moved = mapping();
        moved.generation = "g2".into();
        moved.names[0] = "c2".into();
        t.record(Ok((moved, snapshot(41000, 1000, 2000))), 60);
        assert!(!t.report.live(60) && t.report.hosts.is_empty());
        t.record(Ok((mapping(), snapshot(1, 100, 200))), 70);
        assert!(!t.report.live(70));
    }
    #[test]
    fn subscriber_capped_omissions_missing_counters_and_gaps_never_become_zero_rates() {
        let mut t = Tracker::default();
        t.record(Ok((mapping(), snapshot(1000, 0, 0))), 10);
        t.record(Ok((mapping(), snapshot(11000, 0, 0))), 20);
        let ip: IpAddr = "192.0.2.1".parse().unwrap();
        assert_eq!(t.report.hosts["c1"][&ip].down_mbps, Some(0.0));
        let empty = serde_json::from_value(json!({"captured_at_millis":21000,"hosts":[]})).unwrap();
        t.record(Ok((mapping(), empty)), 30);
        assert!(t.report.live(30) && t.report.hosts.is_empty());
        t.record(Ok((mapping(), snapshot(31000, 10, 20))), 40);
        assert_eq!(t.report.hosts["c1"][&ip].down_mbps, None);
        t.record(Ok((mapping(), snapshot(61000, 100, 200))), 70);
        assert_eq!(t.report.hosts["c1"][&ip].down_mbps, None);
        let missing=serde_json::from_value(json!({"captured_at_millis":71000,"hosts":[{"id":{"circuit_id":1,"subscriber_ip":"192.0.2.1"}}]})).unwrap();
        t.record(Ok((mapping(), missing)), 80);
        let h = &t.report.hosts["c1"][&ip];
        assert!(h.actual.is_none() && h.rtt_ms.is_none() && h.retransmits.is_none());
    }
    #[test]
    fn subscriber_identity_checks_canonical_addresses_overlaps_and_stale_measurements() {
        let row = crate::model::CsvRow {
            circuit_id: "c1".into(),
            ipv4: "192.0.2.1/32".into(),
            ..Default::default()
        };
        let mut duplicate = row.clone();
        duplicate.circuit_id = "c2".into();
        duplicate.ipv4 = "192.0.2.1".into();
        let mut t = Tracker::default();
        t.record(Ok((mapping(), snapshot(1000, 0, 0))), 10);
        t.record(Ok((mapping(), snapshot(11000, 1250000, 2500000))), 20);
        let own = owners(std::slice::from_ref(&row));
        let text = t.report.measurements(&row, &own, 20).join("\n");
        assert!(
            text.contains("down 1.000 Mbps | up 2.000 Mbps")
                && text.contains("pooled 2.000")
                && text.contains("pooled 3")
        );
        let text = t.report.measurements(&row, &own, 36).join("\n");
        assert!(
            text.contains("STALE")
                && text.contains("down unknown | up unknown")
                && text.contains("pooled 3")
        );
        assert!(
            t.report
                .measurements(&duplicate, &own, 20)
                .join("\n")
                .contains("Not in capped sample")
        );
        let conflict = owners(&[row.clone(), duplicate.clone()]);
        assert!(
            t.report
                .measurements(&row, &conflict, 20)
                .join("\n")
                .contains("duplicate")
        );
        duplicate.ipv4 = "192.0.2.0/24".into();
        assert!(
            t.report
                .measurements(&row, &owners(&[row.clone(), duplicate]), 20)
                .join("\n")
                .contains("duplicate")
        );
        let mut observed = row.clone();
        observed.circuit_id = "observed:1".into();
        assert!(
            t.report
                .measurements(&observed, &own, 20)
                .join("\n")
                .contains("no assigned shaping circuit")
        );
    }

    #[test]
    fn subscriber_cache_and_ownership_remain_bounded_at_maximum_size() {
        let rows: Vec<_> = (0..75000u32)
            .map(|i| crate::model::CsvRow {
                circuit_id: i.to_string(),
                ipv4: std::net::Ipv4Addr::from(0x0a000000 + i).to_string(),
                ..Default::default()
            })
            .collect();
        let started = std::time::Instant::now();
        let own = owners(&rows);
        let ownership_time = started.elapsed();
        assert_eq!(own.len(), 75000);
        assert!(own.values().all(|v| *v == 1));
        let mut report = Report {
            updated: 100,
            stale: false,
            reason: None,
            ..Report::default()
        };
        let metric = Metrics {
            actual: Some(crate::worker_util::traffic::Counters {
                download_bytes: u64::MAX,
                upload_bytes: u64::MAX,
            }),
            down_mbps: Some(f64::MAX),
            up_mbps: Some(f64::MAX),
            rtt_ms: Some(f64::MAX),
            rtt_down_ms: Some(f64::MAX),
            rtt_up_ms: Some(f64::MAX),
            retransmits: Some(u64::MAX),
            retransmits_down: Some(u64::MAX),
            retransmits_up: Some(u64::MAX),
            seen_age_ms: Some(u64::MAX),
        };
        for i in 0..LIMIT {
            report.hosts.insert(
                format!("{i:04}{}", "x".repeat(252)),
                BTreeMap::from([("2001:db8::1".parse().unwrap(), metric.clone())]),
            );
        }
        let dir = tempfile::tempdir().unwrap();
        let started = std::time::Instant::now();
        publish(dir.path(), &report).unwrap();
        let cache = read_cache(&dir.path().join(CACHE));
        println!(
            "Subscriber fixture: {} bytes; publish/read {:?}; 75,000-row ownership {:?}",
            std::fs::metadata(dir.path().join(CACHE)).unwrap().len(),
            started.elapsed(),
            ownership_time
        );
        assert!(cache.live(100));
        assert_eq!(cache.hosts.len(), LIMIT);
        report.hosts.insert(
            "extra".into(),
            BTreeMap::from([("192.0.2.1".parse().unwrap(), metric)]),
        );
        assert!(publish(dir.path(), &report).is_err());
        assert_eq!(read_cache(&dir.path().join(CACHE)).hosts.len(), LIMIT);
    }
    #[test]
    fn subscriber_ipv6_aliases_host_masks_and_overlapping_prefixes_have_safe_ownership() {
        let row = |ip: &str| crate::model::CsvRow {
            ipv6: ip.into(),
            ..Default::default()
        };
        let own = owners(&[row("2001:db8::1/128,2001:0db8::1"), row("::/0")]);
        assert_eq!(own.get(&"2001:db8::1".parse().unwrap()), Some(&2));
        assert!(addresses(&row("2001:db8::/64")).is_none());
        assert!(addresses(&row("bad")).is_none());
        assert!(addresses(&row("::")).is_none());
        let own = owners(&[row("2001:db8::1/128,2001:0db8::1")]);
        assert_eq!(own.get(&"2001:db8::1".parse().unwrap()), Some(&1));
        assert_eq!(
            addresses(&row("2001:db8::1/128,2001:0db8::1"))
                .unwrap()
                .len(),
            1
        );
    }

    #[test]
    fn subscriber_mapping_bounds_optional_fields_and_cache_preserve_integrity() {
        let mut bytes = Vec::new();
        ciborium::into_writer(&vec!["c1"], &mut bytes).unwrap();
        let a = Applied {
            generation: "g1".into(),
            report_schema: 1,
            slot_order: bytes,
        };
        assert_eq!(a.mapping().unwrap(), mapping());
        let mut bad = snapshot(1000, 0, 0);
        bad.hosts.as_mut().unwrap()[0].id.circuit_id = 2;
        let mut t = Tracker::default();
        t.record(Ok((mapping(), bad)), 10);
        assert!(!t.report.live(10) && t.report.reason.as_deref().unwrap().contains("slot"));
        t.record(Ok((mapping(), snapshot(1000, 0, 0))), 20);
        t.record(Ok((mapping(), snapshot(11000, 10, 20))), 30);
        let dir = tempfile::tempdir().unwrap();
        publish(dir.path(), &t.report).unwrap();
        let r = read_cache(&dir.path().join(CACHE));
        assert!(r.live(30) && r.hosts.len() == 1);
        let before = std::fs::read(dir.path().join(CACHE)).unwrap();
        t.report.reason = Some("x".repeat(MAX_BYTES as usize));
        assert!(publish(dir.path(), &t.report).is_err());
        assert_eq!(std::fs::read(dir.path().join(CACHE)).unwrap(), before);
        std::fs::write(dir.path().join(CACHE), b"{}").unwrap();
        assert!(!read_cache(&dir.path().join(CACHE)).live(30));
    }
}
