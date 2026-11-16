# Topology Registry Implementation Plan

> **For agentic workers:** Use superpowers:executing-plans inline. Do not delegate unless the owner requests delegation.

**Goal:** Persist verified subscriber identities and policy intent, then reconcile them with complete current observations.

**Architecture:** Add a private versioned registry beside legacy state. Keep endpoints in memory only. Reuse the instance lock and synced private-file writer. No live cycle calls the new path yet.

**Tech Stack:** Rust 2024, standard library, current serde, anyhow, and tempfile dependencies.

**Spec:** `docs/design/TOPOLOGY_MANAGER_IMPLEMENTATION.md` and `docs/design/TOPOLOGY_MANAGER_DISCUSSION.md`.

## Global Constraints

- Preserve prior foundation work and unrelated local changes. Do not commit, push, deploy, or change live configuration.
- Persist IDs, verified source references, profile intent, assignment intent, and pins. Never persist endpoint observations in this registry.
- An authoritative reference is scoped by stable router ID, source ID, and source kind. A name, old IP, or old MAC is not an identity adapter.
- Keep registry records when subscribers disappear. Active endpoints come only from the complete current snapshot.
- Reject missing identity evidence, incomplete sources, duplicate endpoint keys, and conflicting current IP ownership.
- Use one owner and expected-revision edits. A durable write failure blocks further use until the store reopens.
- Preserve current legacy behavior. No implicit migration from CSV, MAC, or name keys.
- Write prose, errors, and comments with STE guidance.

## Review Focus

- Restart and late write failure must not permit use of uncertain memory. Task 3 owns these tests.
- Same address, different current subscriber must not restore an old identity or policy. Task 2 owns this test.
- Several current sessions or explicit identity aliases must form one circuit without endpoint loss. Task 2 owns this test.
- Identity ownership must not transfer through an ordinary policy edit. Tasks 1 and 3 own these tests.
- Missing, corrupt, future-schema, or externally changed files must not silently reset a saved registry. Task 3 owns these tests.

## Task 1: Define durable identity and policy records

**Files:** create `src/topology_manager/model.rs`; change `mod.rs`.

**Produces:**

- `SourceKind::{Ppp,Dhcp,Hotspot}` and `SourceScope { router_id, source_id, kind }`.
- `IdentityRef { scope: SourceScope, account_ref: String }`.
- `SubscriberPolicy { profile_id: Option<String>, assignment: Option<String>, pin: Option<String> }`.
- `SubscriberRecord { id: String, identities: Vec<IdentityRef>, policy: SubscriberPolicy }`.
- `RegistryDocument { schema_version: u32, revision: u64, next_id: u64, subscribers: Vec<SubscriberRecord> }`.
- `RegistryDocument::validate() -> anyhow::Result<()>`, with one indexed validation pass.

Profile and instance IDs are opaque intent references in this slice. The full catalog resolver must check their existence and eligibility before publication.
An ordinary commit may add verified aliases, but it may not delete records or transfer existing identity ownership.
Sequential IDs use `tm-sub-<16 lowercase hex digits>`. The persistent counter must remain above every generated ID.

- [x] Test valid documents, explicit aliases, duplicate IDs/references, empty scopes/account references, invalid policy IDs, pin/assignment conflicts, counter rollback, and unknown schema versions.
- [x] Test strict JSON decoding: unknown endpoint fields and duplicate named fields are errors.
- [x] Run `cargo test --lib topology_manager::model` and observe failure before implementation.
- [x] Implement the schema and borrowed ID/reference indexes. Run the same command and require success.

## Task 2: Reconcile complete current observations

**Files:** create `src/topology_manager/observation.rs`; change `mod.rs`.

**Consumes:** Task 1 records and indexes.

**Produces:**

- `Endpoint { session_id: String, ips: Vec<std::net::IpAddr>, mac: Option<String> }`.
- `AccountObservation { account_ref: String, endpoints: Vec<Endpoint> }`.
- `SourceRead::{Complete { scope, accounts }, Partial { scope }, Failed { scope } }`.
- `ObservationGate::accept(generation: u64, required: &[SourceScope], reads: Vec<SourceRead>) -> anyhow::Result<CompleteSnapshot>`.
- `reconcile(registry: &RegistryDocument, snapshot: CompleteSnapshot) -> anyhow::Result<Reconciliation>`.
- `Reconciliation { generation: u64, registry: RegistryDocument, active: Vec<ActiveSubscriber> }`.
- `ActiveSubscriber { subscriber_id: String, endpoints: Vec<ActiveEndpoint> }` and `ActiveEndpoint { identity: IdentityRef, endpoint: Endpoint }`.

`CompleteSnapshot` has private fields, no Clone, and no serde implementation. Reconciliation consumes it.
A generation marks one collection attempt. Reject repeat attempts, including attempts that previously failed validation.
Require at least one required scope, exactly one successful complete read per scope, and no unexpected scope.
Adapters must supply verified account references and source-specific activity checks. This slice does not implement RouterOS adapters or certify their evidence.

- [x] Test successful empty reads versus missing/partial/failed reads and reused generations.
- [x] Test reconnect with new IP/MAC, disappearance, current address reassignment, and absent identity evidence.
- [x] Test explicit aliases, several sessions per account, duplicate account records/session keys, conflicting IP ownership, and unchanged inputs.
- [x] Test deterministic allocation for reversed source/account order and monotonic ID allocation.
- [x] Run `cargo test --lib topology_manager::observation` and observe failure before implementation.
- [x] Implement current-only reconciliation with indexes. Sort only where deterministic IDs/output require it. Run the same command and require success.

## Task 3: Persist revision-checked registry edits

**Files:** create `src/topology_manager/store.rs`; change `mod.rs`; create `tests/topology_registry.rs`.

**Consumes:** Tasks 1 and 2.

**Produces:**

- `RegistryStore::open(state_dir: &std::path::Path) -> anyhow::Result<RegistryStore>`.
- `RegistryStore::document(&self) -> anyhow::Result<&RegistryDocument>`.
- `RegistryStore::commit(&mut self, expected_revision: u64, candidate: RegistryDocument) -> anyhow::Result<CommitOutcome>`.
- `CommitOutcome { revision: u64, changed: bool }`.

Store the registry as `state_dir/topology-manager/registry.json` with mode 0600 in a private directory.
Hold the existing `files::lock(state_dir)` for the store lifetime. Before live integration, transfer that ownership into the single daemon path.
A new private directory receives a durable empty registry at revision 0.
A missing file in an existing directory is a recovery error. Corrupt/unsupported files never produce an empty fallback.
Check disk equality and expected revision before each edit. Reject counter rollback, record deletion, and identity transfer.
Use `files::write_secret` for replacement and sync. Advance memory only after success.
On any replacement error, block document access and further commits until reopen; the rename may already have occurred.
Skip unchanged documents without a revision bump or file replacement. No policy history or publication receipt is implemented here.

- [x] Test private modes, lock exclusion, restart round trips, stale edits, no-op inode preservation, revision overflow, and invalid edit rejection.
- [x] Test external change/deletion, corrupt JSON, future schema, and unknown endpoint fields.
- [x] Test write failure before replacement and after replacement through a private I/O seam used by the real commit path.
- [x] Test missing files after restart and nonregular files without a blocking open.
- [x] Run `cargo test --lib topology_manager::store` and observe failure before implementation.
- [x] Implement the store and run the same command; require success.
- [x] Add an integration test: connect, commit, drop store, reopen, empty fresh read, reconnect with changed endpoints, same ID/pin, and unchanged registry bytes.
- [x] Run `cargo test --test topology_registry`, full `cargo test`, formatting, Clippy, and whitespace checks.

## Stop conditions

The isolated registry slice is complete when its checks pass and no live path calls it.
Do not claim that a registry record proves activity, full policy validity, or dataplane application.
Next slices own catalog/hierarchy persistence, source adapters, migration, placements, publication, and TUI.
Record results and remaining gates in the project vault.

## Execution record

The owner authorized this slice after the foundation.
Model checks: 7 passed. Observation checks: 13 passed. Store checks: 14 passed. Public integration: 1 passed.
The model, observation, and store stubs failed before implementation.
Strict serde decoding already rejected unsupported fields through its derive implementation.
The scale and public integration tests exercise composed behavior.

Final review added two recovery regressions before their fixes:

- A deleted registry file after restart caused an empty fallback. Initial durable creation and missing-file rejection correct that case.
- A FIFO blocked during open. The regression reached the test body and timed out. `O_NONBLOCK` permits immediate file-type rejection.

Full `cargo test`: 770 passed, zero failed, 9 prior ignored tests.
`cargo fmt --check`, Clippy with denied warnings, and whitespace checks passed.
A synthetic case covers 10,000 subscribers with two sessions each and changed current addresses.
No production benchmark, identity-adapter proof, or dataplane acceptance is claimed.

Lookups use indexes. Deterministic allocation sorts new identities; canonical active output sorts IDs and endpoint keys.
An ordinary commit validates the full registry and reads the saved document for conflict detection.
The whole state directory still requires normal backup and recovery. This slice detects a missing file within an existing registry directory.
No history, policy catalog, source adapter, live cycle integration, or deployment is included.

Session record: `brain/10-sessions/2026-11-16-topology-registry.md`.

## Catalog follow-up

The next slice extends this document to schema 2 with an optional catalog.
See `2026-11-16-topology-catalog.md`. The loader preserves schema-1 intent and requires a revision-checked commit for persistence.
The existing registry lifecycle and private writer protections remain in use.
