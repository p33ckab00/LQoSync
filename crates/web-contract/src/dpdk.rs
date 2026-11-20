//! Bounded, read-only DPDK views from the same published caches as the TUI.
use serde::{Deserialize, Serialize};

#[derive(Clone, Default, Debug, PartialEq, Serialize, Deserialize)]
pub struct Reading {
    pub label: String,
    pub value: String,
}
#[derive(Clone, Default, Debug, PartialEq, Serialize, Deserialize)]
pub struct Source {
    pub updated: Option<u64>,
    pub stale: bool,
    pub state: String,
    pub details: Vec<Reading>,
}
impl Source {
    pub fn live(&self, now: u64, max_age: u64) -> bool {
        !self.stale
            && self
                .updated
                .is_some_and(|at| at > 0 && now.checked_sub(at).is_some_and(|age| age <= max_age))
    }
}
#[derive(Clone, Default, Debug, PartialEq, Serialize, Deserialize)]
pub struct Dpdk {
    #[serde(default)]
    pub sampled_at: u64,
    pub analytics: Source,
    pub health: Source,
    pub nic: Source,
    pub worker_count: u64,
    pub workers: Vec<Worker>,
    pub mean_busy_ppm: Option<u64>,
    pub pressure: Vec<Gauge>,
    pub ports: Vec<Port>,
    pub details_capped: bool,
    /// Typed instruments for the DPDK health console. Every field is unknown until the source has it.
    #[serde(default)]
    pub console: Console,
}
#[derive(Clone, Default, Debug, PartialEq, Serialize, Deserialize)]
pub struct Console {
    pub pool_free: Option<u64>,
    pub pool_total: Option<u64>,
    pub ring_count: Option<u64>,
    pub ring_used: Option<u64>,
    pub ring_capacity: Option<u64>,
    pub rings_over_75: Option<u64>,
    pub rx_nombuf_per_sample: Option<u64>,
    pub pool_drops_per_sample: Option<u64>,
    /// `CLEAR`, `ALERT`, or `UNKNOWN`.
    pub pressure: Option<String>,
    pub queue_drops_total: Option<u64>,
    pub aqm_marks_total: Option<u64>,
    pub backlog_bytes: Option<u64>,
    pub queue_drops_per_sample: Option<u64>,
    pub aqm_marks_per_sample: Option<u64>,
    pub status: Option<String>,
    pub version: Option<String>,
    pub uptime_ms: Option<u64>,
    pub workers: Option<u64>,
    pub missing_rollups: Option<u64>,
    pub oldest_rollup_ms: Option<u64>,
    pub aqm_degraded_workers: Option<u64>,
    pub classification_empty: Option<bool>,
    pub degraded: Vec<String>,
}
#[derive(Clone, Default, Debug, PartialEq, Serialize, Deserialize)]
pub struct Worker {
    pub id: u16,
    pub busy_ppm: u64,
    pub backlog_mean: Option<u64>,
    pub details: Vec<Reading>,
    /// Cumulative cycles per execution phase, in producer order. Empty when the source has none.
    #[serde(default)]
    pub phases: Vec<Phase>,
    #[serde(default)]
    pub laps_per_s: Option<u64>,
    #[serde(default)]
    pub cycles_per_busy_lap: Option<u64>,
}
#[derive(Clone, Default, Debug, PartialEq, Serialize, Deserialize)]
pub struct Phase {
    pub name: String,
    pub cycles: u64,
}
#[derive(Clone, Default, Debug, PartialEq, Serialize, Deserialize)]
pub struct Gauge {
    pub label: String,
    pub used: u64,
    pub total: u64,
}
#[derive(Clone, Default, Debug, PartialEq, Serialize, Deserialize)]
pub struct Port {
    pub id: u16,
    pub name: String,
    pub state: String,
    pub speed_mbps: Option<u64>,
    pub reset: bool,
    #[serde(default)]
    pub physical: PhysicalPort,
    #[serde(default)]
    pub activity: PortActivity,
    pub details: Vec<Reading>,
}

#[derive(Clone, Default, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct PhysicalPort {
    pub pci: Option<String>,
    pub netdev: Option<String>,
    pub role: Option<String>,
    pub connector: Option<String>,
    pub confidence: Option<String>,
    pub evidence: Option<String>,
}
#[derive(Clone, Default, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct PortActivity {
    pub sampled_at: Option<u64>,
    pub interval_ms: Option<u64>,
    pub rx_packets: Option<u64>,
    pub tx_packets: Option<u64>,
    pub rx_bytes: Option<u64>,
    pub tx_bytes: Option<u64>,
}
