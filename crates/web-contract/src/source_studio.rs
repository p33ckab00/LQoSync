//! Topology Studio source contract, version 1. See `docs/frontend/TOPOLOGY_STUDIO_API.md`.
//! Digests and IDs are opaque strings. Times are Unix seconds. Exact rates, factors, and new
//! policy revisions are decimal strings, so that no value goes through a JavaScript number.
use crate::topology::{Flow, Identity, Node};
use serde::{Deserialize, Serialize};

pub const VERSION: u32 = 1;

/// Version 1 limits. The service checks them again.
pub const MAX_JOB_BODY: usize = 32 * 1024;
pub const MAX_POLICIES: usize = 32;
pub const MAX_POLICY_SET_BYTES: usize = 24 * 1024;
pub const MAX_POLICY_NAME: usize = 128;
pub const MAX_FILTER_LEAVES: usize = 32;
pub const MAX_FILTER_DEPTH: usize = 3;
pub const MAX_PLAN_BINDINGS: usize = 128;
pub const PAGE_DEFAULT: u16 = 50;
pub const PAGE_MAX: u16 = 100;
pub const MAX_TABLE_ROWS: u64 = 20_000;
pub const MAX_SNAPSHOT_BYTES: u64 = 16 * 1024 * 1024;
pub const JOB_DEADLINE_SECS: u64 = 60;
pub const CANDIDATE_TTL_SECS: u64 = 600;
pub const MAX_CANDIDATES_PER_ACTOR: u16 = 4;
pub const MAX_ACCESS_PROFILES: usize = 8;

/// A secret value from the browser. `Debug` never shows it. Responses never contain it.
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct SecretInput(pub String);

impl std::fmt::Debug for SecretInput {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("SecretInput(<redacted>)")
    }
}

// ---- Capabilities and saved configuration ----

/// `GET /api/v1/topology/source-capabilities`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct StudioCapabilities {
    pub version: u32,
    /// Only features whose backend task passed. An absent feature keeps its control unavailable.
    pub features: Vec<StudioFeature>,
    /// Only fields that the current reader can supply.
    pub fields: Vec<FieldSpec>,
    pub limits: StudioLimits,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StudioFeature {
    DraftTest,
    AccessProfiles,
    SourceCollect,
    PolicyPreview,
    PolicySave,
    WanEvidence,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct FieldSpec {
    pub field: FieldKey,
    pub source: SourceKind,
    pub operators: Vec<FilterOp>,
    pub groupable: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct StudioLimits {
    pub job_body_bytes: u64,
    pub policies_per_router: u64,
    pub policy_set_bytes: u64,
    pub policy_name_bytes: u64,
    pub filter_leaves: u64,
    pub filter_depth: u64,
    pub plan_bindings: u64,
    pub access_profiles: u64,
    pub page_default: u16,
    pub page_max: u16,
    pub table_rows: u64,
    pub snapshot_bytes: u64,
    pub job_deadline_secs: u64,
    pub candidate_ttl_secs: u64,
    pub candidates_per_actor: u16,
}

impl Default for StudioLimits {
    fn default() -> Self {
        Self {
            job_body_bytes: MAX_JOB_BODY as u64,
            policies_per_router: MAX_POLICIES as u64,
            policy_set_bytes: MAX_POLICY_SET_BYTES as u64,
            policy_name_bytes: MAX_POLICY_NAME as u64,
            filter_leaves: MAX_FILTER_LEAVES as u64,
            filter_depth: MAX_FILTER_DEPTH as u64,
            plan_bindings: MAX_PLAN_BINDINGS as u64,
            access_profiles: MAX_ACCESS_PROFILES as u64,
            page_default: PAGE_DEFAULT,
            page_max: PAGE_MAX,
            table_rows: MAX_TABLE_ROWS,
            snapshot_bytes: MAX_SNAPSHOT_BYTES,
            job_deadline_secs: JOB_DEADLINE_SECS,
            candidate_ttl_secs: CANDIDATE_TTL_SECS,
            candidates_per_actor: MAX_CANDIDATES_PER_ACTOR,
        }
    }
}

/// `GET /api/v1/routers/{id}/source-config`. A GET starts no router read.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RouterSourceConfig {
    pub version: u32,
    pub router_id: String,
    /// The digest of the saved config file.
    pub config_digest: String,
    /// The digest of the config that the service has loaded. Save a policy that uses new access
    /// profiles only when it equals `config_digest`.
    pub loaded_config_digest: String,
    /// The registry revision. `None` before the current registry exists.
    pub policy_revision: Option<String>,
    /// The registered router connection gate.
    pub enabled: bool,
    pub access_profiles: Vec<AccessProfileView>,
    /// The saved policies, in order.
    pub policies: Vec<SourcePolicyDraft>,
    pub snapshots: Vec<SnapshotRef>,
    pub wan_candidates: Vec<WanEvidence>,
}

/// The service cache `source-studio.json`: one `RouterSourceConfig` for each configured router.
/// The service writes it after each cycle and each collection. A GET reads it without router I/O.
pub const CACHE: &str = "source-studio.json";

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct StudioCache {
    pub version: u32,
    pub updated: u64,
    pub routers: Vec<RouterSourceConfig>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct AccessProfileView {
    pub id: String,
    pub label: String,
    pub credential_set: bool,
    pub supported_writes: Vec<WriteScope>,
}

/// One access profile in `RouterEdit.access_profiles`. An absent password keeps the saved secret.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AccessProfileEdit {
    pub id: String,
    pub label: String,
    pub username: String,
    #[serde(default)]
    pub password: Option<SecretInput>,
    pub supported_writes: Vec<WriteScope>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SnapshotRef {
    pub id: String,
    pub router_id: String,
    pub config_digest: String,
    pub collected_from: u64,
    pub collected_to: u64,
    pub expires_at: u64,
    pub tables: Vec<TableEvidence>,
}

/// A partial or failed table is never an empty complete table.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TableEvidence {
    pub table: TableId,
    pub state: ReadState,
    pub records: u64,
    /// `None` when the table was not requested.
    pub collected_at: Option<u64>,
    pub issue_code: Option<String>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TableId {
    Interfaces,
    Routes,
    PppSecrets,
    PppProfiles,
    PppActive,
    DhcpServers,
    DhcpLeases,
    HotspotUsers,
    HotspotServers,
    HotspotProfiles,
    HotspotActive,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReadState {
    Complete,
    Partial,
    Failed,
    NotRequested,
    Stale,
}

// ---- WAN evidence ----

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct WanEvidence {
    pub interface: String,
    pub routes: Vec<RouteEvidence>,
    pub interface_state: InterfaceState,
    pub read_state: ReadState,
    pub last_observed_at: Option<u64>,
    /// The saved decision. `None`: no decision. Never `clear`.
    pub decision: Option<WanCandidateDecision>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum InterfaceState {
    Running,
    Stopped,
    Disabled,
    Missing,
    Unknown,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RouteEvidence {
    pub routing_table: String,
    pub active: bool,
    pub gateway: Option<String>,
    pub resolved_interface: Option<String>,
    pub observed_at: Option<u64>,
}

/// `wan.candidate_set`. No decision changes RouterOS routes or enables the WAN-list writer.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WanCandidateDecision {
    Pin,
    Ignore,
    Clear,
}

// ---- Source policy ----

/// One source policy. The backend chooses the allowlisted tables and joins from it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SourcePolicyDraft {
    /// Backend-assigned after save. `None` for an unsaved policy.
    pub id: Option<String>,
    /// 1 to 128 UTF-8 bytes.
    pub name: String,
    pub enabled: bool,
    /// A lower value wins.
    pub priority: u16,
    pub source: SourceKind,
    /// An access profile ID of this router.
    pub read_access_ref: String,
    pub members: MemberMode,
    pub filter: FilterExpr,
    pub group: GroupRule,
    pub identity: IdentityMode,
    /// Explicit precedence: the first rate source that resolves wins.
    pub rates: Vec<RateSource>,
    pub parent: ParentRule,
    pub capacity: CapacityRule,
    #[serde(default)]
    pub writes: Vec<WriteGrant>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SourceKind {
    Ppp,
    Dhcp,
    Hotspot,
    Wan,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MemberMode {
    Active,
    Configured,
    Joined,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum IdentityMode {
    Registry,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum FieldKey {
    #[serde(rename = "active.service")]
    ActiveService,
    #[serde(rename = "active.comment")]
    ActiveComment,
    #[serde(rename = "account.profile")]
    AccountProfile,
    #[serde(rename = "account.comment")]
    AccountComment,
    #[serde(rename = "lease.status")]
    LeaseStatus,
    #[serde(rename = "lease.server")]
    LeaseServer,
    #[serde(rename = "server.name")]
    ServerName,
    #[serde(rename = "user.profile")]
    UserProfile,
    #[serde(rename = "hotspot.server")]
    HotspotServer,
    #[serde(rename = "interface.name")]
    InterfaceName,
    #[serde(rename = "route.routing_table")]
    RouteRoutingTable,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FilterOp {
    Eq,
    Ne,
    Prefix,
    Contains,
    In,
}

/// At most 32 leaves and depth 3.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum FilterExpr {
    All {
        rules: Vec<FilterExpr>,
    },
    Any {
        rules: Vec<FilterExpr>,
    },
    Match {
        field: FieldKey,
        op: FilterOp,
        value: FilterValue,
    },
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum FilterValue {
    Text { value: String },
    Texts { values: Vec<String> },
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum GroupRule {
    ByField { field: FieldKey },
    Fixed { name: String },
    Attach { node_id: String },
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum ParentRule {
    Router,
    Node { node_id: String },
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum RateSource {
    SubscriberOverride,
    AccountComment {
        format: CommentRateFormat,
    },
    ActiveComment {
        format: CommentRateFormat,
    },
    ProfileMap {
        bindings: Vec<PlanBinding>,
    },
    ServerMap {
        bindings: Vec<PlanBinding>,
    },
    /// The existing directional/unit parser.
    ProfileRateLimit,
    Fallback {
        plan_id: String,
    },
}

/// The `kind` of the `RateSource` that supplied a resolved rate.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RateKind {
    SubscriberOverride,
    AccountComment,
    ActiveComment,
    ProfileMap,
    ServerMap,
    ProfileRateLimit,
    Fallback,
}

/// An exact selector match.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PlanBinding {
    pub selector: String,
    pub plan_id: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CommentRateFormat {
    LegacyConfigured,
}

/// Factors are exact decimal strings. Rates are decimal bps strings; a minimum can be zero.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum CapacityRule {
    FromPlans {
        down_factor: String,
        up_factor: String,
        minimum_down_bps: String,
        minimum_up_bps: String,
    },
    Fixed {
        down_bps: String,
        up_bps: String,
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WriteScope {
    WanAddressLists,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WriteGrant {
    pub access_ref: String,
    pub scope: WriteScope,
    pub enabled: bool,
}

// ---- Source jobs ----

/// `POST /api/v1/topology/source-jobs`. The answer is `202 Operation`; its `source_result`
/// carries the typed result.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StudioJobRequest {
    pub key: String,
    pub job: StudioJob,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum StudioJob {
    /// An explicit one-shot read. It saves no router and enables no periodic read.
    DraftTest {
        connection: DraftConnection,
        required_sources: Vec<SourceKind>,
    },
    /// Contacts only an enabled registered router.
    Collect {
        router_id: String,
        expected_config_digest: String,
        sources: Vec<SourceKind>,
    },
    /// Never contacts RouterOS.
    Evaluate {
        router_id: String,
        /// The registry revision, as a decimal string.
        expected_revision: String,
        expected_config_digest: String,
        snapshot_id: String,
        policies: Vec<SourcePolicyDraft>,
    },
}

/// A connection draft. The service keeps it in memory only.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DraftConnection {
    pub address: String,
    pub port: u16,
    pub tls: bool,
    pub tls_verify: bool,
    pub username: String,
    pub password: SecretInput,
    #[serde(default)]
    pub ca_pem: Option<String>,
}

/// `Operation.source_result`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum StudioJobResult {
    DraftTest(DraftTestReport),
    Snapshot(SnapshotRef),
    Candidate(CandidateRef),
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct DraftTestReport {
    /// A changed connection draft has another fingerprint and invalidates the result.
    pub connection_fingerprint: String,
    pub at: u64,
    pub elapsed_ms: u64,
    pub checks: Vec<TestCheck>,
}

/// `code` is a safe code, never a raw authentication error.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TestCheck {
    pub stage: TestStage,
    pub state: CheckState,
    pub code: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TestStage {
    Transport,
    Tls,
    Authentication,
    Identity,
    TablePermission,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CheckState {
    Passed,
    Failed,
    NotRun,
}

// ---- Candidates ----

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CandidateRef {
    pub id: String,
    pub base_revision: String,
    pub config_digest: String,
    pub snapshot_id: String,
    pub draft_digest: String,
    pub expires_at: u64,
}

/// `GET /api/v1/topology/source-candidates/{id}`. A display result only: the save path
/// validates current inputs again.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct CandidateSummary {
    pub reference: CandidateRef,
    pub graph: CandidateGraph,
    pub diff: CandidateDiff,
    /// Counted before any page cut.
    pub issues: IssueCounts,
    /// False when mandatory evaluation is incomplete.
    pub publishable: bool,
    pub complete: bool,
}

/// New nodes use `draft:` IDs. A candidate never allocates a durable Circuit ID or Device ID.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct CandidateGraph {
    pub nodes: Vec<Node>,
    pub flows: Vec<Flow>,
    pub nodes_truncated: bool,
    pub flows_truncated: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CandidateDiff {
    pub added: u64,
    pub removed: u64,
    pub changed: u64,
    pub unchanged: u64,
    pub markers: Vec<DiffMarker>,
    pub markers_truncated: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct DiffMarker {
    pub entity_ref: String,
    pub change: ChangeKind,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ChangeKind {
    Added,
    Removed,
    Changed,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct IssueCounts {
    pub blockers: u64,
    pub warnings: u64,
    pub information: u64,
}

/// The query of the candidate `records` and `issues` pages. The cursor is bound to the actor,
/// candidate, draft digest, and filter.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CandidatePageQuery {
    #[serde(default)]
    pub cursor: Option<String>,
    /// 1 to 100. Default 50.
    #[serde(default = "page_default")]
    pub limit: u16,
}

fn page_default() -> u16 {
    PAGE_DEFAULT
}

/// A page can end early to meet the byte budget; `next_cursor` continues after the last item.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CandidatePage<T> {
    pub candidate_id: String,
    pub items: Vec<T>,
    pub next_cursor: Option<String>,
    pub total: u64,
}

/// Unresolved values are `None`, never an invented zero.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResolvedRecord {
    pub record_ref: String,
    /// The saved policy ID, or `draft:<index>` for an unsaved policy.
    pub policy_id: String,
    pub identity_ref: Option<Identity>,
    pub group_ref: Option<String>,
    pub parent_ref: Option<String>,
    /// Decimal bps strings.
    pub down_bps: Option<String>,
    pub up_bps: Option<String>,
    pub speed_source: Option<RateKind>,
    pub provenance: Vec<Provenance>,
    pub issue_codes: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ValidationIssue {
    pub code: String,
    pub severity: Severity,
    pub entity_ref: String,
    pub field: Option<FieldKey>,
    pub message: String,
    pub remedy: Option<String>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Severity {
    Blocker,
    Warning,
    Information,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Provenance {
    pub router_id: String,
    pub table: TableId,
    pub snapshot_id: String,
    pub record_ref: String,
    pub fields: Vec<FieldKey>,
}

/// The error body of the new endpoints, with the HTTP status. Old endpoints keep status-only errors.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct StudioError {
    pub code: String,
    pub message: String,
    pub field: Option<String>,
}
