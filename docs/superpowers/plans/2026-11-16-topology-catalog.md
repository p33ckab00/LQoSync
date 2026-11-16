# Topology Catalog Implementation Plan

> **For agentic workers:** Use superpowers:executing-plans inline. Do not delegate unless the owner requests delegation.

**Goal:** Persist the source catalog, speed profiles, hierarchy, and subscriber intent in one checked revision.

**Architecture:** Add a catalog model and reuse the pure layout/rate validators. Extend the existing private document to schema 2. Read schema 1 through an explicit lossless upgrade and save the upgrade only through a revision-checked commit.

**Tech Stack:** Rust 2024, standard library, existing serde/anyhow/tempfile stack.

**Spec:** `docs/design/TOPOLOGY_MANAGER_IMPLEMENTATION.md` and `docs/design/TOPOLOGY_MANAGER_DISCUSSION.md`.

## Global Constraints

- Preserve prior uncommitted implementation and unrelated edits. Do not commit, push, deploy, or change live configuration.
- Store configured inputs and factor choices separately from calculated rates.
- WAN capacity is explicit. Its effective shaping factor cannot exceed 100 percent.
- Shared factors belong to source definitions. An instance override replaces the inherited factor.
- Source IDs, instance IDs, local profile IDs, and subscriber IDs are different references. Labels are not identity keys.
- Persist the synthetic root label without a synthetic capacity node.
- Preserve explicit target sets. New WAN discovery must not expand them.
- Keep all observed endpoint data outside the durable document.
- Retain one writer, expected revisions, uncertainty blocking, and current native verification protections.
- Use STE guidance for technical prose. Do not add dependencies.

## Review Focus

- Catalog removal or a profile restriction must not orphan a saved pin. Tasks 1 and 2 own these tests.
- Repeated source instances must share definition defaults but keep their IDs and parent references. Task 1 owns these tests.
- A schema upgrade must preserve all IDs, counters, and intent and must reject unsupported/ambiguous input. Task 2 owns these tests.
- SourceRead and Local speed authorities must remain explicit, with no automatic overwrite. Task 1 owns these tests.
- Labels, source availability, and current membership must not change durable identity or silently broaden placement. Task 3 owns these tests.

## Task 1: Model and validate the catalog

**Files:** create `src/topology_manager/catalog.rs`; change `mod.rs` and numeric validation helpers in `rates.rs`.

**Produces:**

- `RouterRole::{Wan,Ppp,Dhcp,Hotspot}` and `CatalogRouter { id, name, roles }`.
- `Factors { down: Fraction, up: Fraction }` and `FactorChoice::{Inherit,Override(Factors)}`.
- `GroupFormula::{PlanSum,CountTimesUnit(RateBps)}`.
- `BackstopPolicy { base: RateBps, factor: Fraction }`.
- `GroupDefaults { formula, factors, backstop: Option<BackstopPolicy> }`.
- `SourceOrigin::{Wan { router_id, endpoint_ref },Subscribers { scope: SourceScope, profile_ref: Option<String>, group: GroupDefaults }}`.
- `CatalogSource { id, name, origin, profile_id: Option<String>, targets: Vec<String> }`.
- `BaseRateAuthority::{Local(RateBps),SourceRead}`.
- `SubscriberProfile { id, name, base: BaseRateAuthority, speed: SpeedOverrides, eligible_targets: Option<Vec<String>> }`.
- `CapacityPolicy::{Inherit,Fixed { base, factors: FactorChoice },WanPercentage { down, up },Automatic { factors: FactorChoice }}`.
- `CatalogNode { id, name, parent: Option<String>, source_id: Option<String>, kind: NodeKind, capacity: CapacityPolicy }`.
- `TopologyCatalog { root_label, routers, sources, profiles, defaults: SpeedPolicy, nodes }`.
- `TopologyCatalog::{validate,validate_references,layout,required_scopes}`.
- `RateBps::apply_factors(self, down: Fraction, up: Fraction) -> anyhow::Result<RateBps>`.

`validate_references(&self, subscribers: &[SubscriberRecord])` checks known references, leaf destinations, configured potential source targets, and explicit local profile restrictions.
Current source-profile membership still needs fresh adapter evidence. A static check does not prove it.
`layout()` returns `Vec<PolicyNode>` after validation. Current direct membership is zero here; the future resolver must check actual membership.
`required_scopes()` returns sorted unique subscriber scopes selected through nodes or source targets. WAN inventory is a separate adapter read.
Custom automatic groups use PlanSum. Source-backed groups retain their declared GroupFormula and backstop inputs for the later evaluator.

- [x] Test multiple WANs, profile-only topology, WAN-only direct destinations, custom groups, and repeated sources under explicit parents.
- [x] Test unknown/duplicate IDs and origins, wrong selected router roles, invalid parent graphs, duplicate source/parent instances, and illegal source/node kinds.
- [x] Test shared factors, replacement overrides, manual base retention, WAN percentages, and WAN factors above 100 percent.
- [x] Test local/read speed authorities, malformed speed rules, explicit target restrictions, and pinned destination removal.
- [x] Test new unselected WAN discovery without target expansion and duplicate labels without identity merging.
- [x] Run `cargo test --lib topology_manager::catalog` and observe missing behavior.
- [x] Implement indexed validation and reuse the existing iterative layout validator. Run the same command and require success.

## Task 2: Upgrade the single private document

**Files:** change `model.rs` and `store.rs`.

**Consumes:** Task 1 catalog and prior registry storage.

**Produces:** `RegistryDocument { schema_version: 2, revision, next_id, subscribers, catalog: Option<TopologyCatalog> }`.

`catalog: None` means unconfigured/inert, including a schema-1 upgrade. It is not an active default topology.
Store loaders strictly decode schema 1 through a private legacy struct, then construct schema 2 without guessing mappings.
Open does not rewrite schema 1. A completed explicit commit saves schema 2 at revision + 1, even when intent is otherwise unchanged.
Retain the exact legacy IDs, references, pins, counter, and revision during in-memory upgrade.
A saved schema-2 catalog cannot revert to None through an ordinary edit.
Schema 1 and schema 2 use the same file, lock, private mode, and synced replacement mechanism.
Before production migration, retain a verified private backup. Restore that policy backup through the supported reader and recollect sources for rollback.
A configured catalog has no lossless schema-1 representation. Do not strip it for a downgrade. No production rollback or backup tool is added here.
Unknown future versions and duplicate/unknown fields are errors. Do not deserialize through a lossy generic JSON value.

- [x] Test schema-1 open without byte changes, explicit durable upgrade, restart, and stale-revision rejection.
- [x] Test unsupported schemas, duplicate fields, unknown endpoint fields, and falsely downgraded schema-2 documents.
- [x] Test atomic rejection of orphaned profiles, assignments, and pins. Keep saved bytes and revision unchanged on rejection.
- [x] Test that a complete edit can change hierarchy and reconcile its references in the same candidate.
- [x] Test that catalog removal cannot bypass reference checks.
- [x] Bind complete snapshots to the catalog-selected scopes and reject a mismatched source set.
- [x] Canonicalize ID tables and target sets so representation-only order changes do not create revisions.
- [x] Run focused model/store tests and observe failure before the changes.
- [x] Implement the schema transition, then require the focused tests to pass.

## Task 3: Prove the public path and scale

**Files:** create `tests/topology_catalog.rs`; update implementation records.

**Consumes:** Tasks 1 and 2.

- [x] Persist a two-WAN catalog, shared source instances, a local plan, and a subscriber pin together. Reopen and prove exact identity and input preservation.
- [x] Change a node label and parent with the same instance ID. Preserve the pin and show the new path through the saved hierarchy.
- [x] Add an unselected WAN source. Prove that the target list and existing instances remain unchanged.
- [x] Reject an orphaning edit without file changes. Reject stale candidates.
- [x] Add a synthetic catalog with 10,000 nodes. Validate without per-subscriber catalog scans or recursive ancestor walks.
- [x] Run full `cargo test`, formatting, Clippy with denied warnings, whitespace, and no-live-caller checks.

## Stop conditions

This slice ends with a durable, checked catalog and an explicit schema upgrade.
It does not collect RouterOS data, allocate subscribers, render live shaping files, or enable schedules.
The next slice must resolve fresh membership, profile authority, placement, rates, and conflicts before publication.
Record results and remaining gates in the vault.

## Execution record

This slice adds 23 checks: 15 catalog, 5 schema/reference/snapshot checks, 1 source-origin transition check, and 2 public integration tests.
The first 14 catalog tests and 5 schema/reference/snapshot checks failed before implementation.
Final review added failing formula and source-origin regressions before their guards.
The synthetic case includes 10,000 nodes and 10,000 subscriber bindings.

Full `cargo test`: 793 passed, zero failed, 9 prior ignored tests.
Formatting, Clippy with denied warnings, whitespace, and no-live-caller checks passed.
The first Clippy run found fixture initialization outside its struct literal. The corrected fixture requires no lint suppression.

One test assumed source array position 2 after canonical persistence. It now locates the source by stable ID.
The valid atomic edit keeps the same expected behavior.
Catalog tables, roles, and target lists use canonical order before persistence.
An order-only edit preserves the revision, file bytes, and inode.

DHCP grouping remains an explicit GroupFormula choice because local per-plan groups need no RouterOS profile object.
PPP requires PlanSum. Hotspot requires CountTimesUnit. Custom automatic groups use PlanSum.
Source-origin keys exclude mutable group factors and plan policy, so factor edits remain valid.

A compiled WAN layout rate is the effective local shaping target.
WAN percentages must use the configured base in CatalogNode.capacity.Fixed, then apply effective ancestor ceilings.
The future resolver must prove this distinction with combined WAN-factor and percentage fixtures.
Policy history, verified migration backups, production rollback, fresh source authority, live export, and native acceptance remain later gates.

Session record: `brain/10-sessions/2026-11-16-topology-catalog.md`.
