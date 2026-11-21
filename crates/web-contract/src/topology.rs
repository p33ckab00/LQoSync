//! The Topology Manager read view. The service writes it as `topology.json` at the end of each
//! cycle. One file is one snapshot: the registry revision, the cached preview, and the collection
//! of that cycle. It holds no credential, no file path, and no raw registry document.
use serde::{Deserialize, Serialize};

pub const CACHE: &str = "topology.json";
pub const VERSION: u32 = 1;

#[derive(Clone, Default, Debug, PartialEq, Serialize, Deserialize)]
pub struct TopologyView {
    pub version: u32,
    /// Unix seconds when the service wrote the view.
    pub updated: u64,
    /// Seconds after `updated` when the view is stale.
    pub fresh_for: u64,
    /// Opaque. Pages and cursors bind to it.
    pub snapshot_id: String,
    /// Decimal string. `None`: no registry.
    pub registry_revision: Option<String>,
    /// `observe`, `shadow`, `apply`, ...
    pub mode: String,
    /// `topology`, `legacy`, or `unknown`. Unknown disables writes.
    pub writer: String,
    pub enrolled: bool,
    /// The cached preview. `None`: no preview since the service started.
    pub observed: Option<Observed>,
    pub nodes: Vec<Node>,
    pub sources: Vec<Source>,
    pub fallback: Option<Fallback>,
    pub plans: Vec<Plan>,
    /// Saved subscriber records, online and offline.
    pub subscribers: Vec<Subscriber>,
    /// PPP and DHCP rows of the last collection, with their link to a saved record.
    pub observations: Vec<Observation>,
    /// Collection health per router. A failed read is not proof that users disconnected.
    pub collection: Vec<CollectionRouter>,
    /// Preview conflicts. Never truncated.
    pub conflicts: Vec<Conflict>,
}

#[derive(Clone, Default, Debug, PartialEq, Serialize, Deserialize)]
pub struct Observed {
    /// The registry revision that the preview evaluated. Decimal string.
    pub revision: String,
    pub collected_at: u64,
    pub generation: Option<u64>,
    /// `ok` or `conflicts`. A conflict preview has empty node and subscriber lists by design.
    pub state: String,
    /// The preview belongs to an older registry revision.
    pub stale: bool,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "mode", rename_all = "snake_case")]
pub enum Capacity {
    Inherit,
    Automatic,
    Fixed {
        down_mbps: u64,
        up_mbps: u64,
        factor_percent: Option<[u64; 2]>,
    },
    WanPercent {
        down_percent: u64,
        up_percent: u64,
    },
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Node {
    pub id: String,
    pub name: String,
    pub parent: Option<String>,
    /// `wan`, `intermediate`, or `group`.
    pub kind: String,
    pub source_id: Option<String>,
    pub capacity: Capacity,
    /// The node keeps an adopted legacy name.
    pub alias: bool,
    /// From the preview. Display values only; the core decides placement and budgets.
    pub calculated_mbps: Option<[f64; 2]>,
    pub exported_mbps: Option<[f64; 2]>,
    pub members: Option<u64>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Source {
    pub id: String,
    pub name: String,
    /// `wan` or `subscribers`.
    pub kind: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Fallback {
    pub node_ids: Vec<String>,
    pub default_mbps: [u64; 2],
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Plan {
    pub id: String,
    pub name: String,
    pub down_mbps: u64,
    pub up_mbps: u64,
    /// PPP identities that select the plan.
    pub ppp_identities: u64,
    /// DHCP identities that select the plan.
    pub dhcp_identities: u64,
    /// Unique subscribers that select the plan, over all source kinds.
    pub subscribers: u64,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct Identity {
    pub router_id: String,
    pub source_id: String,
    /// `ppp`, `dhcp`, or `hotspot`.
    pub kind: String,
    pub account_ref: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Subscriber {
    /// Stable ID. Every mutation addresses this ID, never an IP or a label.
    pub id: String,
    pub identities: Vec<Identity>,
    pub plan_id: Option<String>,
    pub pin: Option<String>,
    pub assignment: Option<String>,
    /// From the preview. `None`: not active in the preview, or no preview.
    pub active: Option<Active>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Active {
    pub node: String,
    pub ips: Vec<String>,
    pub min_mbps: [f64; 2],
    pub max_mbps: [f64; 2],
    pub fallback: bool,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Observation {
    pub router_id: String,
    pub router: String,
    /// `ppp` or `dhcp`.
    pub kind: String,
    pub name: String,
    pub ipv4: String,
    pub mac: String,
    /// `online`, `offline`, `bound`, `unbound`, `disabled`, `unknown`, or `stale`.
    pub state: String,
    /// `verified`: the subscriber ID comes from the source reference authority.
    /// `none`: no saved record has that reference. `unknown`: the evidence cannot decide.
    pub link: String,
    pub subscriber_id: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct CollectionRouter {
    pub router_id: String,
    pub name: String,
    /// `online`, `failed`, `unreachable`, or `offline`.
    pub state: String,
    pub observed_at: u64,
    pub sources: Vec<CollectionSource>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct CollectionSource {
    pub kind: String,
    pub enabled: bool,
    pub records: u64,
    pub online: Option<u64>,
    pub offline: Option<u64>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Conflict {
    pub kind: String,
    pub subject: String,
    pub detail: String,
}
