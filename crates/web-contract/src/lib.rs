//! Browser-safe views. No configuration or credential records cross this boundary.
use serde::{Deserialize, Serialize};
pub mod dpdk;
pub mod journal;
pub mod routers;
pub mod topology;
#[derive(Clone, Default, Debug, PartialEq, Serialize, Deserialize)]
pub struct Overview {
    pub observed_at: Option<u64>,
    pub stale: bool,
    pub mode: String,
    pub target: String,
    pub subscribers: Option<u64>,
    #[serde(default)]
    pub ipoe_active: Option<u64>,
    pub ppp_online: Option<u64>,
    pub ppp_offline: Option<u64>,
    pub routers: Option<u64>,
    pub routers_online: Option<u64>,
    pub cycle_ms: Option<u64>,
    pub health: String,
    pub publication: String,
    pub components: Vec<Component>,
    pub traffic: Vec<Point>,
    pub traffic_stale: bool,
    #[serde(default)]
    pub classification: Option<ClassificationTraffic>,
    #[serde(default)]
    pub wan_capacity: Option<WanCapacity>,
    #[serde(default)]
    pub dpdk: Option<dpdk::Dpdk>,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct WanCapacity {
    pub down_bps: u64,
    pub up_bps: u64,
    pub wan_count: u64,
    pub registry_revision: u64,
}
#[derive(Clone, Default, Debug, PartialEq, Serialize, Deserialize)]
pub struct Component {
    pub name: String,
    pub state: String,
    pub observed_at: u64,
}
#[derive(Clone, Default, Debug, PartialEq, Serialize, Deserialize)]
pub struct Point {
    #[serde(default)]
    pub gap_before: bool,
    pub at: u64,
    pub down_mbps: f64,
    pub up_mbps: f64,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Principal {
    pub username: String,
    pub csrf: String,
}

/// Histories share the worker cache, but retain independent source availability.
#[derive(Clone, Default, Debug, PartialEq, Serialize, Deserialize)]
pub struct ClassificationTraffic {
    pub mapped: TrafficSeries,
    pub unmapped: TrafficSeries,
    pub history_capped: bool,
}
#[derive(Clone, Default, Debug, PartialEq, Serialize, Deserialize)]
pub struct TrafficSeries {
    pub points: Vec<Point>,
    pub available: bool,
    pub stale: bool,
}
/// Bucketed history for one chart window. Buckets hold mean rates; a missing bucket sets `gap_before`.
#[derive(Clone, Default, Debug, PartialEq, Serialize, Deserialize)]
pub struct TrafficWindow {
    pub window: String,
    pub bucket_secs: u64,
    pub actual: Vec<Point>,
    pub mapped: Vec<Point>,
    pub unmapped: Vec<Point>,
}
