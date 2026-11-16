# Topology Publication Boundary Implementation Plan

> **For agentic workers:** Use superpowers:executing-plans. Execute the tasks in order.

**Goal:** Prepare and publish fresh manager output through the current pair/receipt path, with an isolated legacy parity check.

**Architecture:** Consume authorized reconciliation, then reuse placement, aggregate checks, pair rendering, validation, import comparison, and the receipt tracker. Keep manager mode disconnected from the live runner.

**Tech Stack:** Rust, serde_json, existing hash, pair writer, and native evidence types. No new dependency.

**Spec:** `docs/design/TOPOLOGY_MANAGER_IMPLEMENTATION.md`.

## Global Constraints

- Keep the original source deadline and expected registry revision through publication.
- Preserve subscriber maxima. Do not round a node ceiling upward.
- Store identity/policy intent before file side effects. Keep network-first file order.
- Keep one receipt owner. Do not claim atomic pair replacement or native acceptance from a file write.
- Preserve the live legacy path and unrelated local work. Do not deploy, commit, push, or write routers.

## Review Focus

- Repeated labels and a synthetic root must export without an artificial shared capacity.
- Current multi-session endpoints must remain separate device rows under one circuit.
- Sub-bps conversion or float-import precision loss must block rather than change a policy rate.
- Expired evidence, stale revisions, and critical placement/aggregate conflicts must stop publication.
- Legacy parity must expose unresolved identity/static ownership instead of adopting endpoints as identity.

## Task 1: Prepared pair and parity

**Files:** add `publication.rs`, `publication_tests.rs`, `parity.rs`, and `parity_tests.rs`. Extend module exports.

**Interfaces:** `prepare(AuthorizedReconciliation) -> PreparedPair`, read-only pair getters, and `compare_legacy`.

- [x] Write failing checks for fresh output, multi-WAN forests, duplicate labels, multiple sessions, offline removal, and rate precision.
- [x] Reuse resolver/aggregate/validator/renderers. Use exact decimal Mbps text and check the f32 import conversion at integer-bps precision.
- [x] Export stable node IDs with distinct display keys. Keep the synthetic root as policy metadata.
- [x] Write failing parity checks for formatting-only equivalence, changed rates/endpoints, missing IDs, duplicates, and unmanaged rows.
- [x] Report differences without source identity inference or legacy source-priority conversion.

## Task 2: Existing owner and evidence metadata

**Files:** extend publication, write_receipt, `apply_evidence/verify`, their fixtures, and the public topology integration test.

**Interfaces:** optional receipt `TopologyContext`, prepared publication through `RegistryStore` and `Tracker`, and native proof correlation.

- [x] Write failing checks for receipt context, old receipt compatibility, and proof rejection after context changes.
- [x] Commit the candidate with the expected revision. Check the source deadline before/after commit and before the tracker call.
- [x] Add optional policy revision, observation generation, and registry digest metadata to the existing receipt.
- [x] Bind Prepared/Verified evidence to that metadata. Preserve current import, hash, generation, and native transaction checks.
- [x] Prove network-first publication and unchanged-content receipt reuse in temporary directories.

## Task 3: Check and capture

- [x] Review references, privacy, precision, deadline fences, no-change writes, and receipt proof correlation.
- [x] Run focused/full tests, format, Clippy, and whitespace checks.
- [x] Update design, session, Home, and handoff with results and remaining activation gates.

## Scope limit

This slice creates an isolated owner-to-pair boundary. It does not activate manager mode or convert legacy policy automatically.
Parity remains diagnostic until explicit legacy identity/ownership mappings and source-priority conversions prove migration parity.
Exact native admission and a live DPDK acceptance test remain activation gates. Preserve the current verifier instead of inventing a second ACK loop.
Execute inline. Use author review. Keep all local work for a later authorized commit.

## Results and review

The initial preparation run had three failures and two passes. The parity red run had two failures and one pass.
The publication red run failed before the adapter could commit and call the tracker.
Native context changes incorrectly reused earlier evidence before the context binding fix.

Two shared tracker regressions failed before their fixes.
A durable-intent save failure previously permitted file replacement. The tracker now stops both writes.
Matching files without a matching receipt previously had no trackable attempt. The tracker now records unchanged outcomes.
Optional event-history failure still permits output replacement and reports degraded notification state.

This slice adds 19 checks. Full suite: 1,012 passed, zero failed, 9 ignored.
All 290 topology checks pass. Format, Clippy with denied warnings, and whitespace checks pass.
The public restart test now reaches temporary pair publication and the existing Prepared builder with both Local and SourceRead settings.
Native context tests preserve all existing file/import/generation/transaction checks.
No live DPDK acceptance test occurred.

The first parity test fixture needed wider test-only visibility before its behavioral red run.
The fractional-rate fixture first hit a legitimate aggregate minimum conflict. Explicit unit factors isolate its export precision check.
The full build also found two example Meta constructors. Both now explicitly use no topology context.
Clippy fixes use is_multiple_of and avoid a cloned reference slice.

Author review covered node identity, current endpoints, rate precision, bounded JSON depth, source deadlines, revision fences, receipts, and parity.
No independent reviewer participated. No code finding remains open in this slice.
Stop here. Explicit legacy identity/label/static ownership conversion, native admission, owner wiring, enrollment, TUI, and schedules remain separate gates.
No dependency, legacy source/config parser, router reader, cycle, deployment, service, or router changed.
The legacy runner only gains the optional metadata initializer. Shared tracker correctness changes are intentional and covered.
No commit or push occurred. Preserve unrelated work and the prior local slices.
Logs: `.superpowers/sdd/2026-11-17-topology-publication/`.
