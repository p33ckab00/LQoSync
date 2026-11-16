---
type: session
created: 2026-11-16
tags: [topology, source, identity, reliability, performance, ste]
status: id-and-read-boundary-complete
related: ["[[2026-11-16-topology-rebalance]]", "[[HANDOFF]]"]
---

# Catalog IDs and current router reads

The owner directs the next task after scoped rebalance.
Worktree: `/home/pi/LQoSync3.0`. Branch: `lqosync3`. Base: `b94ed7c` plus prior uncommitted slices.

Agent: Codex `/root`. Timestamp: 2026-11-16T21:47:14+08:00, Asia/Shanghai.
No commit, push, deployment, live configuration, router, or service changed. Unrelated work remains intact.

## Changes

Added `src/topology_manager/ids.rs`, `ids_tests.rs`, `reader.rs`, and `reader_tests.rs`.
Updated `model.rs` validation and module exports. No dependency or legacy path changed.

ID batches check the expected revision and share the durable counter with subscriber allocation.
The allocator validates the draft before mutation. Overflow and stale revisions leave the draft unchanged.

Only a store commit makes a reservation durable. Tests cover restart, deletion, custom IDs, and shared subscriber allocation.
Generated catalog prefixes require the correct kind, format, and counter range.

The reader derives required tables from router roles and reuses `RouterSession`.
It requests no passwords and no configured DHCP endpoint fallbacks.
It checks row locators, unexpected fields, command timeout, whole-read age, and monotonic generations.
It reads inventory before and after active tables and compares rows by locator, independent of order.
Failure or cancellation drops the API session. Success returns it for reuse.
Evidence expires from read start and has no Clone, serde, or Debug implementation.

## Owner clarification

The owner identifies PPP secrets, current DHCP hostname/MAC, and Hotspot active rows as available sources.
There is no RADIUS service. Local reads do not require one.

PPP needs secrets for account configuration and active rows for current presence.
DHCP needs bound leases and current active endpoint fields. Hotspot needs authenticated active sessions and account checks.

These source tables do not prove continuity after ambiguous account replacement or DHCP endpoint changes.
Do not transfer old pins or policies through hostname, MAC, IP, or RouterOS locators alone.
The approved identity-conflict guard remains necessary. No protocol normalizer claims verified identity yet.

## Checks

The ID red run had 7 failures and 1 pass. All 8 ID checks now pass.
The reader red run had 5 failures and 4 passes. All 11 reader checks now pass.

The full suite has 903 passed, zero failed, and 9 ignored.
`cargo fmt --check` and `cargo clippy --all-targets -- -D warnings` passed.

Scale checks cover 10,000 reserved IDs and 10,000 reordered inventory rows. They are not production benchmarks.
The STE checker is heuristic. Manual review does not establish certified compliance.

Plan: `docs/superpowers/plans/2026-11-16-topology-source-boundary.md`.
Logs: `.superpowers/sdd/2026-11-16-topology-source-boundary/`.
Design: `docs/design/TOPOLOGY_MANAGER_IMPLEMENTATION.md`.

## Limits and next action

No live caller uses these modules. The reader does not create `SourceRead::Complete`.
Multiple RouterOS commands do not form an atomic snapshot. Current sessions can change after their active-table reply.
The adapter must check evidence age after normalization and before acceptance.
WAN discovery does not infer true capacity, subscriber routing, or selected WAN membership.

Next: typed protocol normalization with explicit account continuity and conflict rules.
Then complete migration/recovery, export/native integration, TUI, and scheduler behavior.
Keep task 3 normalization open. Do not wait for RADIUS or infer permanent identity from current DHCP fields.
