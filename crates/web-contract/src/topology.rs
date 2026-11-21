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
    #[serde(default)]
    pub frontend_initialized: bool,
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
    /// Global subscriber speed rules. `None` without a catalog.
    #[serde(default)]
    pub speed: Option<SpeedDefaults>,
    /// The group factor of each subscriber source.
    #[serde(default)]
    pub factors: Vec<SourceFactor>,
    /// The latest shaping write. `None` when the service has no write record.
    #[serde(default)]
    pub publication: Option<Publication>,
    /// Recent web changes, newest first. Changes from the TUI or the CLI are not recorded here.
    #[serde(default)]
    pub history: Vec<WebChange>,
    /// Decimal string. The canvas layout revision, independent of `registry_revision`.
    #[serde(default)]
    pub layout_revision: Option<String>,
}

/// The latest shaping write and its evidence. Saved policy, files, import, and native
/// verification are separate facts; none of them stands in for another.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Publication {
    pub write_id: String,
    /// `intent`, `pending`, `import_confirmed`, `applied`, `write_failed`, `apply_failed`,
    /// `verify_timeout`, `superseded`, `interrupted`, or `candidate`.
    pub phase: String,
    pub started: u64,
    pub updated: u64,
    /// The policy revision that the write published. `None`: a legacy writer write.
    pub policy_revision: Option<String>,
    /// File names (never paths) and their outcome: `pending`, `written`, `unchanged`,
    /// `reconciled`, or `failed`.
    pub files: Vec<PublishedFile>,
    /// The write recorded an error. The text stays in the service journal.
    pub failed: bool,
    /// The native runtime accepted this write.
    pub accepted: bool,
    pub superseded_by: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PublishedFile {
    pub name: String,
    pub outcome: String,
}

/// One web change receipt.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct WebChange {
    pub at: u64,
    pub actor: String,
    /// What changed, in words.
    pub change: String,
    /// `saved`, `failed`, `not_committed`, or `outcome_unknown`.
    pub state: String,
    pub base_revision: String,
    pub revision: Option<String>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct SpeedDefaults {
    pub down_min: SpeedRule,
    pub up_min: SpeedRule,
    pub down_max: SpeedRule,
    pub up_max: SpeedRule,
    /// Profiles with their own speed rules. A defaults edit resets them.
    pub overrides: u64,
}

/// One rule as exact decimal text, for display and for a form start value.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct SpeedRule {
    /// `percent` of the plan, fixed `mbps`, or `legacy` (an imported rounded rule).
    pub unit: String,
    pub value: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SourceFactor {
    pub source_id: String,
    pub name: String,
    /// Exact decimal factors, for example `0.5`.
    pub down: String,
    pub up: String,
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
    /// The LibreQoS map type and location. Independent of `kind`.
    #[serde(default)]
    pub geo: Option<NodeGeo>,
    /// Canvas `[x, y]`. Layout only; it never changes shaping or `location`.
    #[serde(default)]
    pub position: Option<[f64; 2]>,
}

/// The `type` of a node in `network.json`. Separate from the policy role.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GeoType {
    Site,
    Ap,
}

/// A Site or AP node. A location without a type cannot exist.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NodeGeo {
    #[serde(rename = "type")]
    pub geo_type: GeoType,
    #[serde(default)]
    pub location: Option<Location>,
}

/// A node location as the operator entered it. The text keeps every decimal place.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Location {
    pub latitude: String,
    pub longitude: String,
}

/// Longest accepted coordinate text. Google Maps copies about 18 characters.
pub const MAX_COORDINATE: usize = 32;

impl Location {
    /// Parse a Google Maps `latitude, longitude` paste. The order is never swapped.
    pub fn parse_pair(text: &str) -> Result<Self, String> {
        let mut parts = text.split(',');
        let (Some(lat), Some(lon), None) = (parts.next(), parts.next(), parts.next()) else {
            return Err("paste a latitude and a longitude, separated by a comma".into());
        };
        let location = Self {
            latitude: lat.trim().into(),
            longitude: lon.trim().into(),
        };
        location.validate()?;
        Ok(location)
    }

    pub fn validate(&self) -> Result<(), String> {
        let lat = coordinate(&self.latitude, "latitude")?;
        let lon = coordinate(&self.longitude, "longitude")?;
        if !(-90.0..=90.0).contains(&lat) {
            return Err("latitude must be from -90 to 90".into());
        }
        if !(-180.0..=180.0).contains(&lon) {
            return Err("longitude must be from -180 to 180".into());
        }
        Ok(())
    }

    /// Numeric values for export. Call only on a validated location.
    pub fn numeric(&self) -> (f64, f64) {
        (
            self.latitude.parse().unwrap_or(f64::NAN),
            self.longitude.parse().unwrap_or(f64::NAN),
        )
    }
}

/// Plain decimal text only: an optional `-`, digits, and optional `.digits`.
fn coordinate(text: &str, name: &str) -> Result<f64, String> {
    let digits = text.strip_prefix('-').unwrap_or(text);
    let (whole, fraction) = digits.split_once('.').unwrap_or((digits, "0"));
    let plain = |s: &str| !s.is_empty() && s.bytes().all(|b| b.is_ascii_digit());
    if text.len() > MAX_COORDINATE || !plain(whole) || !plain(fraction) {
        return Err(format!("{name} must be a decimal number such as 12.2361"));
    }
    text.parse().map_err(|_| format!("{name} is not a number"))
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
    /// The policy of the linked record. `None` without a verified link.
    #[serde(default)]
    pub plan_id: Option<String>,
    #[serde(default)]
    pub pin: Option<String>,
    /// The preview placement of the linked record.
    #[serde(default)]
    pub node: Option<String>,
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

/// Rows per page. The broker reply limit (64 KiB) can end a page earlier; `next_cursor` continues it.
pub const MAX_PAGE: usize = 200;

/// `GET /api/v1/topology`: the envelope, the tree, and the policy lists, without subscriber rows.
#[derive(Clone, Default, Debug, PartialEq, Serialize, Deserialize)]
pub struct TopologySummary {
    pub updated: u64,
    pub fresh_for: u64,
    pub snapshot_id: String,
    pub registry_revision: Option<String>,
    pub mode: String,
    pub writer: String,
    pub enrolled: bool,
    #[serde(default)]
    pub frontend_initialized: bool,
    pub observed: Option<Observed>,
    pub nodes: Vec<Node>,
    /// The reply limit cut the node list.
    pub nodes_truncated: bool,
    pub sources: Vec<Source>,
    pub fallback: Option<Fallback>,
    pub plans: Vec<Plan>,
    pub collection: Vec<CollectionRouter>,
    /// The first conflicts. `conflicts_total` counts all of them.
    pub conflicts: Vec<Conflict>,
    pub conflicts_total: u64,
    pub subscribers_total: u64,
    pub subscribers_active: u64,
    pub ppp_total: u64,
    pub dhcp_total: u64,
    #[serde(default)]
    pub speed: Option<SpeedDefaults>,
    #[serde(default)]
    pub factors: Vec<SourceFactor>,
    #[serde(default)]
    pub publication: Option<Publication>,
    /// The newest web changes. The cache holds more.
    #[serde(default)]
    pub history: Vec<WebChange>,
}

/// One page of a filtered list. The cursor binds the snapshot and the filter.
#[derive(Clone, Default, Debug, PartialEq, Serialize, Deserialize)]
pub struct Page<T> {
    pub snapshot_id: String,
    /// Rows that match the filter, in all pages.
    pub total: u64,
    pub items: Vec<T>,
    pub next_cursor: Option<String>,
}

/// Filters of the Assignments and source pages. Every field is optional.
#[derive(Clone, Default, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(default)]
pub struct PageQuery {
    /// `ppp` or `dhcp`. Observation pages only.
    pub kind: Option<String>,
    pub router_id: Option<String>,
    /// An observation state, or `active` / `offline` for subscribers.
    pub state: Option<String>,
    /// `verified`, `none`, or `unknown`. Observation pages only.
    pub link: Option<String>,
    pub plan_id: Option<String>,
    /// Text in a name, an account reference, or a subscriber ID.
    pub q: Option<String>,
    pub cursor: Option<String>,
}

/// One saved subscriber and the observation rows that link to it.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SubscriberDetail {
    pub subscriber: Subscriber,
    pub observations: Vec<Observation>,
}

/// The broker answer to a topology read.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TopologyReply {
    Summary(Box<TopologySummary>),
    Assignments(Page<Subscriber>),
    Observations(Page<Observation>),
    Subscriber(Box<SubscriberDetail>),
    /// The cursor belongs to another snapshot or filter. Load the first page again.
    Expired,
    /// No such subscriber in this snapshot.
    NotFound,
}

/// The closed set of web mutations. The service maps each one to one existing operation.
/// The browser never sends an operation name.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "command", rename_all = "snake_case", deny_unknown_fields)]
pub enum WebCommand {
    PlanCreate {
        expected_revision: u64,
        name: String,
        down_mbps: u64,
        up_mbps: u64,
    },
    /// A field that is `None` keeps its value.
    PlanUpdate {
        expected_revision: u64,
        plan_id: String,
        name: Option<String>,
        down_mbps: Option<u64>,
        up_mbps: Option<u64>,
    },
    PlanDelete {
        expected_revision: u64,
        plan_id: String,
    },
    /// Select (`Some`) or clear (`None`) the local plan of one verified identity.
    SubscriberPlan {
        expected_revision: u64,
        identity: Identity,
        plan_id: Option<String>,
    },
    /// Commit the kept Builder proposal.
    EditConfirm { proposal_id: String },
}

/// The durable result of one web mutation, found again by its request key.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct WebReceipt {
    pub key: String,
    /// `saved`, `not_committed`, `failed`, or `outcome_unknown`.
    pub state: String,
    /// The registry revision before the mutation. Decimal string.
    pub base_revision: String,
    /// The saved revision. Decimal string. Only for `saved`.
    pub revision: Option<String>,
    /// The new plan or subscriber ID, when the operation makes one.
    pub subject_id: Option<String>,
    /// A safe reason for `failed`.
    pub error: Option<String>,
}

/// `POST /api/v1/topology/mutations`. The browser makes the key once per user action and sends it
/// again on a retry; the service runs one command once per key.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MutationRequest {
    pub key: String,
    pub command: WebCommand,
}

/// The state of one web mutation for its actor.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Operation {
    pub key: String,
    /// `queued`, `running`, `saved`, `failed`, `not_committed`, `outcome_unknown`, or `not_found`.
    pub state: String,
    /// The service receipt once the service answered or recovery decided.
    pub receipt: Option<WebReceipt>,
    /// The impact review of a Builder proposal (`review_ready`).
    #[serde(default)]
    pub review: Option<Review>,
    /// A safe reason when the service refused the request.
    #[serde(default)]
    pub error: Option<String>,
    /// The answer of a discovery job (`done`).
    #[serde(default)]
    pub discovery: Option<Discovery>,
}

/// An explicit read that contacts the enabled routers. No GET starts one.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReadJob {
    /// Read the subscriber sources and cache a new preview.
    Refresh,
    /// Read the router interfaces that can be WAN endpoints.
    Wan,
    /// Read the subscriber sources that the policy does not hold yet.
    Sources,
}

/// `POST /api/v1/topology/reads`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReadRequest {
    pub key: String,
    pub job: ReadJob,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Discovery {
    pub collected_at: u64,
    /// WAN discovery: one entry per configured router, with its own state.
    pub routers: Vec<DiscoveredRouter>,
    /// Source discovery: sources that the policy can add.
    pub sources: Vec<SourceCandidate>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct DiscoveredRouter {
    pub router_id: String,
    pub name: String,
    /// `ok` or `failed`. A failed router keeps its own error.
    pub state: String,
    pub error: Option<String>,
    pub candidates: Vec<WanCandidate>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct WanCandidate {
    /// The interface name; `wan.create` uses it as the endpoint reference.
    pub name: String,
    pub kind: String,
    pub running: bool,
    pub disabled: bool,
    pub suggested: bool,
    /// WAN nodes that already use this endpoint.
    pub used_by: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SourceCandidate {
    pub key: String,
    pub kind: String,
    pub router: String,
    pub name: String,
    /// The source exists and changes its binding.
    pub rebind: bool,
}

/// The Builder edits that the web can propose. The JSON is the service `Edit` form.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", deny_unknown_fields)]
pub enum BuilderEdit {
    #[serde(rename = "topology.initialize")]
    Initialize,
    #[serde(rename = "node.create")]
    NodeCreate {
        name: String,
        parent: Option<String>,
        capacity: Capacity,
        /// `intermediate` or `group`.
        node_kind: Option<String>,
    },
    #[serde(rename = "instance.add")]
    InstanceAdd {
        source_id: String,
        parent: Option<String>,
        name: Option<String>,
    },
    #[serde(rename = "node.move")]
    NodeMove {
        node_id: String,
        parent: Option<String>,
    },
    #[serde(rename = "node.update")]
    NodeUpdate {
        node_id: String,
        name: Option<String>,
        capacity: Option<Capacity>,
    },
    #[serde(rename = "node.remove")]
    NodeRemove { node_id: String },
    /// A WAN node on an interface from the latest WAN discovery of the service.
    #[serde(rename = "wan.create")]
    WanCreate {
        router_id: String,
        endpoint_ref: String,
        name: String,
        down_mbps: u64,
        up_mbps: u64,
        factor_percent: Option<[u64; 2]>,
    },
    /// One source from the latest source discovery, under `parent`.
    #[serde(rename = "source.add")]
    SourceAdd { key: String, parent: Option<String> },
    #[serde(rename = "plan.create")]
    PlanCreate {
        name: String,
        down_mbps: u64,
        up_mbps: u64,
    },
    /// A field that is `None` keeps its value.
    #[serde(rename = "plan.update")]
    PlanUpdate {
        plan_id: String,
        name: Option<String>,
        down_mbps: Option<u64>,
        up_mbps: Option<u64>,
    },
    #[serde(rename = "plan.delete")]
    PlanDelete { plan_id: String },
    /// The plan of one saved subscriber, for all its identities. `None` clears it.
    #[serde(rename = "subscriber.plan")]
    SubscriberPlan {
        subscriber_id: String,
        plan_id: Option<String>,
    },
    /// Pin one saved subscriber to a WAN node, or unpin with `None`. A topology pin writes no
    /// router rule.
    #[serde(rename = "subscriber.pin")]
    SubscriberPin {
        subscriber_id: String,
        wan_node_id: Option<String>,
    },
    /// Place the subscribers of one source again, or of all sources with `None`.
    #[serde(rename = "rebalance")]
    Rebalance { source_id: Option<String> },
    /// The four global speed rules. It resets profile rules; the review says how many.
    #[serde(rename = "speed.defaults")]
    SpeedDefaults {
        down_min: RuleInput,
        up_min: RuleInput,
        down_max: RuleInput,
        up_max: RuleInput,
    },
    /// Exact decimal factors of one subscriber source. `None` keeps a direction.
    #[serde(rename = "group.factor")]
    GroupFactor {
        source_id: String,
        down: Option<String>,
        up: Option<String>,
    },
    #[serde(rename = "fallback.set")]
    FallbackSet {
        node_ids: Vec<String>,
        default_down_mbps: u64,
        default_up_mbps: u64,
    },
    #[serde(rename = "fallback.clear")]
    FallbackClear,
}

/// A speed rule as typed: a percent of the plan (one decimal) or whole Mbps. The service checks
/// the precision and the range.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum RuleInput {
    Percent(f64),
    Mbps(u64),
}

/// `POST /api/v1/topology/proposals`. `key` names the proposal for its actor only.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProposalRequest {
    pub key: String,
    pub expected_revision: u64,
    pub edit: BuilderEdit,
}

/// `POST /api/v1/topology/proposals/{proposal}/confirm`. `key` is the receipt key of the save.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ConfirmRequest {
    pub key: String,
}

/// The service impact review of one Builder proposal.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Review {
    /// The proposal key. The service token stays in the broker.
    pub proposal: String,
    /// Unix seconds. Confirm before this time.
    pub expires_at: u64,
    pub base_revision: String,
    /// Subscribers that move to another node.
    pub moves: u64,
    /// The first moves. `moves` counts all of them.
    pub moved: Vec<Moved>,
    /// Offline subscribers that lose their saved placement.
    pub cleared: u64,
    pub assigned: u64,
    pub new_records: u64,
    /// Subscriber sources that the service rebalanced for the review.
    pub sources: Vec<String>,
    /// Nodes that the edit adds, removes, moves, or changes.
    pub nodes: Vec<NodeChange>,
    pub warnings: Vec<String>,
    /// Subscribers whose rate changes. `rates` lists the first ones.
    #[serde(default)]
    pub rates_changed: u64,
    #[serde(default)]
    pub rates: Vec<RateChange>,
    /// Node capacities that change.
    #[serde(default)]
    pub capacities: Vec<CapacityChange>,
}

/// Rates as display text from the service values; the browser computes nothing from them.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RateChange {
    pub subscriber_id: String,
    pub label: Option<String>,
    pub before: String,
    pub after: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CapacityChange {
    pub node_id: String,
    pub before: String,
    pub after: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Moved {
    pub subscriber_id: String,
    pub label: Option<String>,
    pub from: Option<String>,
    pub to: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct NodeChange {
    pub id: String,
    pub name: String,
    /// `added`, `removed`, `moved`, or `changed`.
    pub change: String,
}

#[cfg(test)]
mod canvas_tests {
    use super::*;

    #[test]
    fn a_google_maps_pair_keeps_the_full_decimal_text() {
        let l = Location::parse_pair("12.236199459585354, 122.02419129509887").unwrap();
        assert_eq!(l.latitude, "12.236199459585354");
        assert_eq!(l.longitude, "122.02419129509887");
        assert_eq!(l.numeric(), (12.236199459585354, 122.02419129509887));
    }

    #[test]
    fn a_pair_accepts_spaces_and_negative_values() {
        let l = Location::parse_pair("  -33.86881 ,  -151.2093 ").unwrap();
        assert_eq!(
            (l.latitude.as_str(), l.longitude.as_str()),
            ("-33.86881", "-151.2093")
        );
        assert!(Location::parse_pair("-90, 180").is_ok());
    }

    #[test]
    fn a_location_rejects_out_of_range_or_non_decimal_text() {
        let bad = |lat: &str, lon: &str| {
            Location {
                latitude: lat.into(),
                longitude: lon.into(),
            }
            .validate()
            .is_err()
        };
        assert!(bad("90.0001", "0"));
        assert!(bad("0", "-180.5"));
        assert!(bad("1e2", "0"));
        assert!(bad("NaN", "0"));
        assert!(bad("inf", "0"));
        assert!(bad("", "0"));
        assert!(bad("+1", "0"));
        assert!(bad("1.", "0"));
        assert!(bad(&format!("1.{}", "1".repeat(40)), "0"));
        assert!(Location::parse_pair("12.5").is_err());
        assert!(Location::parse_pair("1, 2, 3").is_err());
    }

    #[test]
    fn an_old_node_without_canvas_fields_still_decodes() {
        let n: Node = serde_json::from_value(serde_json::json!({
            "id": "n1", "name": "A", "parent": null, "kind": "group", "source_id": null,
            "capacity": {"mode": "inherit"}, "alias": false, "calculated_mbps": null,
            "exported_mbps": null, "members": null
        }))
        .unwrap();
        assert_eq!((n.geo, n.position), (None, None));
    }
}
