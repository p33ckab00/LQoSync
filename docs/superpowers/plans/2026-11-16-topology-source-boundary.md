# Implementation plan for source evidence and IDs

> **For agentic workers:** Use superpowers:executing-plans inline. Do not delegate unless the owner requests delegation.

**Goal:** Add stable catalog ID allocation and complete current RouterOS reads before identity normalization.

**Architecture:** Reuse the durable document counter and revision checks for ID batches. Add an isolated reader through the current RouterSession trait. Keep legacy reads and live callers unchanged. Identity normalization requires an explicit authoritative reference contract.

**Tech Stack:** Rust, current tokio/serde/anyhow stack, and existing RouterSession transport.

**Spec:** `docs/design/TOPOLOGY_MANAGER_IMPLEMENTATION.md` and `docs/design/TOPOLOGY_MANAGER_DISCUSSION.md`, identity detail 3.

## Global Constraints

- Preserve prior implementation and unrelated edits. Do not commit, push, deploy, or change live state.
- IP, MAC, display names, and session locators cannot establish durable subscriber identity by themselves.
- Allocate catalog IDs through a checked monotonic counter. Do not derive them from labels or addresses.
- Preserve schema-1/2 compatibility and existing subscriber IDs. Keep current endpoints outside the durable document.
- Read only tables required by configured roles and never request passwords.
- Failed, partial, changed, or expired evidence cannot become a complete source read.
- Reuse the current RouterSession transport. Do not add dependencies or another collector service.

## Review Focus

- ID allocation must survive restart and deletion without counter rollback or label-dependent changes.
- A stale revision or counter exhaustion must leave the candidate unchanged.
- Configuration changes during a multi-table read must fail closed rather than mix incompatible rows.
- Router errors and timeouts must not leave a reusable complete result.
- Authority for subscriber continuity must be explicit before a normalizer can claim verified identity.

## Task 1: Allocate durable catalog IDs

**Files:** add `src/topology_manager/ids.rs` and tests. Extend `model.rs` validation and module exports.

- [x] Add failing tests for ID batches, reserved formats, restart, deletion, stale revisions, and exhaustion.
- [x] Share `RegistryDocument.next_id` across subscriber and catalog ID allocation. Preserve all existing values.
- [x] Add typed router/source/profile/node ID allocation with expected revisions and atomic batch counter updates.
- [x] Validate generated catalog ID formats and counters without restricting existing custom IDs.
- [x] Keep persistence behind the existing revision-checked store.

## Task 2: Guard complete current RouterOS reads

**Files:** add `src/topology_manager/reader.rs` and tests.

- [x] Add failing tests for role plans, table fields, completeness, generation replay, age, timeout, and changed configuration.
- [x] Read inventory before and after current active tables. Reject changed inventories independent of reply order.
- [x] Require current row locators and reject duplicates. Treat these locators as ephemeral, not permanent subscriber IDs.
- [x] Return opaque bounded-age evidence only after all required commands succeed.
- [x] Keep the legacy reader and RouterCfg flow unchanged.

## Task 3: Record sources and define the next identity slice

- [x] Record the owner's available sources: PPP secrets, current DHCP hostname/MAC, and Hotspot active rows. No RADIUS service exists.
- [x] Add an explicit opt-in operator-reference lifecycle contract in the identity plan. Live authority settings and enrollment still require integration.
- [x] Keep identity authority separate from current read evidence. Do not treat RouterOS locators as permanent subscriber IDs.

## Task 4: Check and capture

- [x] Run focused tests, full tests, format, denied-warning Clippy, and whitespace checks for completed work.
- [x] Review privacy, ID compatibility, current-read ownership, and failure paths.
- [x] Capture completed scope and any unresolved identity dependency in design and handoff notes.

## Execution Rulings

Worktree: `/home/pi/LQoSync3.0`, branch `lqosync3`, base `b94ed7c` plus prior uncommitted slices.
The approved identity rules require an operator reference or verified independent identifier when continuity is ambiguous.

The owner identifies local account/session tables and current DHCP hostname/MAC. There is no RADIUS service.
This answers source availability, but it does not establish durable continuity for ambiguous account replacement or DHCP changes.
Complete the ID/read boundary now. Protocol normalization and the account lifecycle contract form the next slice.

The reader establishes current transport/configuration evidence. It does not by itself establish subscriber identity authority.
Source normalization and live activation must keep that distinction explicit.

## Local verification

- ID red run: 7 failed and 1 passed against the stub. Green run: all 8 passed.
- Reader red run: 5 failed and 4 passed against the stub. All 11 reader checks now pass.

- The public reader drops failed/cancelled sessions and preserves successful sessions for reuse.
- Synthetic checks cover 10,000 reserved IDs and 10,000 reordered inventory rows. They are not production benchmarks.
- Full suite: 903 passed, zero failed, 9 ignored. Format and Clippy with denied warnings passed.

Current normalization now exists in the normalization plan. The identity plan adds opt-in operator-reference authorization.
Live enrollment and durable authority settings remain integration gates. No fallback claims identity from addresses or row locators.
