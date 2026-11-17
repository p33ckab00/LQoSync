# Plan for the topology source coordinator

> Execute this plan inline. Use test-driven development and verification before completion.

**Goal:** Connect configured router reads to authorized topology preparation through the runner owner.

**Architecture:** A coordinator holds sessions and observation generations. It collects selected sources in parallel under one deadline. The runner reuses its registry lock and controlled compiler for an explicit sync API.

**Tech stack:** Current Rust and Tokio dependencies. Current RouterOS traits, registry, and compiler.

**Spec:** `docs/design/TOPOLOGY_MANAGER_IMPLEMENTATION.md`.

## Constraints

Keep source rows in memory only. Use current configured identity authority.
Do not infer subscriber identity from IP, MAC, or hostname.
Do not activate the live service loop before all activation gates pass.
Preserve operational edits and the legacy service path.

## Review focus

- Missing or ambiguous router names must block reads before connection.
- Disabled selected protocols must block reads. Unselected protocols must not add required tables.
- A failed, expired, or cancelled batch must not supply retained subscriber data.
- Connection time must count against the batch deadline. Tasks must stop after cancellation.
- Complete disconnects must remove exported rows. Reconnects must use current endpoints with stable authorized identity.

## Task 1: Coordinate selected source reads

Files: `src/topology_manager/coordinator.rs`, `coordinator_tests.rs`, `reader.rs`, and `mod.rs`.


- [x] Add tests for exact configured router mapping, selected protocols, and complete fresh preparation.

- [x] Run the focused tests and make sure the required behavior is absent.

- [x] Add a coordinator with internal monotonic generations, sessions, and the observation gate.

- [x] Bound connection and reads by one deadline. Use cancellation-safe parallel tasks.

- [x] Test failure, cancellation, connection timeout, disconnect, reconnect, and identity conflict.

- [x] Run the focused tests and make sure all pass.

## Task 2: Use the runner owner

Files: `src/runner.rs` and task-specific runner tests.


- [x] Test registry lock reuse and rejection before source reads when the runtime target is unsuitable.

- [x] Add `Runner::sync_topology(compiler, max_age, now)` for collection, authorization, compilation, and publication.

- [x] Reuse the current writer, receipt tracker, and scheduler request. Do not add a second writer.

- [x] Test the configured public sync API with the optional installed library in temporary directories.

- [x] Run the full suite, format, Clippy, and whitespace checks.

- [x] Update design, session, Home, and handoff with proof and activation limits.

## Stop condition

The explicit runner API accepts only complete fresh authorized source batches.
The legacy service loop remains unchanged. All task tests and required checks pass.

## Results


- Full suite: 1,084 passed, zero failed, and 12 ignored.

- Added 15 active tests. All 13 coordinator tests pass.

- The optional installed-library sync test passes in temporary directories.

- Format, Clippy with warnings denied, and whitespace checks pass.

- The connection regression first reproduced a stale router session. The corrected coordinator discards that session.

- The native test checks failed-read fencing, restart generations, disconnects, and reconnects with fresh IP/MAC data.

- A fixture correction follows the established Hotspot group formula and source-instance eligibility rules.

- No dependency, live service path, routing rule, router configuration, or deployment changed.

- Author review only. These tests do not prove production throughput or a hard memory bound for the transport.

## Next action

Prove parity with legacy policies before service activation.
Cover exclusions, address-based DHCP overrides, minimum/factor policies, and authoritative static input.
Independent enrollment and native planner bounds also remain activation gates.
The monitor still requires live DPDK evidence before it accepts an applied policy.
