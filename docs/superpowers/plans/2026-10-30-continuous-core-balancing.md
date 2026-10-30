# Continuous Core Balancing Implementation Plan

> **For agentic workers:** Use superpowers:executing-plans inline. Use TDD for each behavior and verification-before-completion before each completion claim.

**Goal:** Reconcile new, reconnected, offline, and failed-WAN clients through guarded owned address-list operations.

**Architecture:** Reuse dynamic topology and session planning. Add a pure health state machine, a scoped reconciliation planner, and a serialized writer. Keep preview as the initial mode. The existing protected-row migration remains a distinct maintenance operation.

**Tech Stack:** Rust 1.88+, existing Tokio and RouterOS client, private TOML configuration, private durable journals, and existing modal UI.

**Spec:** [Continuous core balancing design](../specs/2026-10-30-continuous-core-balancing-design.md).

## Global constraints

- Live migration remains on hold. Do not write routers, change production shaping, or restart services during implementation.
- Preserve manual pins, protected entries, exact ownership checks, and existing publication gates.
- Default automatic failure behavior evacuates active clients only after confirmed unhealthy status. Unknown is not unhealthy.
- Initial writer mode is preview. Pin failover override defaults off. Recovery does not force clients back.
- First live-move support covers an existing owned host on the same core only. Cross-core moves remain blocked.
- Keep settings, raw inventory, client identities, and private journals out of Git.
- Proposed defaults: health interval 10 seconds, evidence lifetime 30 seconds, failure threshold 3, recovery threshold 3, recovery delay 30 seconds.
- Use two distinct dedicated probe targets per WAN and at least one passing target per successful round.
- Offline cleanup needs two complete absence observations at least 30 seconds apart. A router batch contains at most 20 mutations.

## Review focus

- Repeated probe reads must not count as new completed probes; stale evidence stays unknown.
- A failed source WAN must not block a valid move to a healthy destination through an all-WAN route gate.
- Residual coverage and IP reuse must block conflicting additions without changing manual rows.
- Restarts and settings changes must not replay an uncertain mutation or reuse an old enable approval.
- Connection marks can retain old routing after a list move; never report confirmed traffic failover from list state alone.

## Task 1: Typed health evidence and configuration

Files: create `v3/src/wan_health.rs` and `v3/tests/wan_health.rs`; modify `v3/src/model.rs`, `v3/src/lib.rs`, and `v3/src/routeros.rs`.

Interfaces: define `HealthStatus { Healthy, Unhealthy, Unknown }`, serializable per-WAN policy/state, and `evaluate(policy, previous, evidence, now)` as a pure transition function.
Evidence carries WAN ID, provider identity, settings revision, completed sample identity/time, route attribution, and each target result.

- [x] Write failing tests for threshold transitions, recovery delay, repeated samples, future timestamps, stale evidence, startup, all-target failure, and one-target success.
- [x] Run `cargo test --locked --test wan_health`; confirm the expected RED failure.
- [x] Implement validated settings with the exact proposed defaults. Missing settings preserve preview behavior.
- [x] Add a narrow Netwatch read property allowlist. Preserve existing read authorization and TLS checks.
- [x] Validate selected ICMP probes, dedicated IPv4 targets, unique host-route attribution, and absence of ambiguous output policy.
- [ ] Test unsupported and unavailable evidence as Unknown. Require version-specific fixture proof for completed sample freshness.
- [x] Run focused tests to GREEN, format, and strict Clippy. Commit the self-contained health component.

## Task 2: Reconciliation decisions and eligible destinations

Files: create `v3/src/core_reconcile.rs` and `v3/tests/core_reconcile.rs`; modify `v3/src/sessions.rs`, `v3/src/dynamic.rs`, and route analysis at its narrowest responsible layer.

Interfaces: define `ReconcilePlan` with reasons and typed `Keep`, `Add`, `Remove`, `Move`, and `Blocked` decisions. Inputs include source observation, health state, settings revision, actual owned rows, and prior cleanup evidence.
Moves carry exact before/after members and the unchanged row ID. The planner is pure and does not send commands.

- [ ] Write failing tests for arrivals, reconnects, retained sessions, manual mode, pinned clients, and failed-WAN evacuation.
- [ ] Add tests for unknown health, no healthy destination, pin override off/on, and stable recovery without forced failback.
- [ ] Run `cargo test --locked --test core_reconcile`; confirm RED.
- [ ] Reuse current count scoring and deterministic tie-breaking. Count retained and pinned online clients before assigning arrivals or evacuees.
- [ ] Track confirmed absence separately from a collection failure. Do not treat wholly empty scans as mass-removal authority.
- [ ] Test exact 30-second cleanup boundary, IP changes, IP reuse, overlapping residuals, and cross-core destinations.
- [ ] Separate structural route errors from source availability. Preserve strict destination checks and reject unknown routing structures.
- [ ] Test that a failed source route permits only supported evacuation while malformed destinations still block.
- [ ] Run focused tests to GREEN. Commit the tested planner without a live caller.

## Task 3: Scoped writer and durable recovery

Files: create `v3/src/core_writer.rs` and `v3/tests/core_writer.rs`; modify `v3/src/routeros.rs` and `v3/src/migration_fence.rs` only as required.

Interfaces: the writer consumes one revision-bound `ReconcilePlan` and fresh evidence. A transport exposes exact owned-host add, remove, and same-core list/comment change operations.
Use a separate reconciliation journal schema; do not relax the maintenance engine membership-preservation invariant.

- [ ] Write failing wire tests for allowed field sets, returned add IDs, same-ID moves, complete precondition checks, traps, lost replies, and missing IDs.
- [ ] Confirm RED with `cargo test --locked --test core_writer`.
- [ ] Implement a crate-private scoped move command that permits only `.id`, `list`, and `comment` after full ownership validation.
- [ ] Journal intent before send and result before further work. Known-unsent errors differ from unknown network outcomes.
- [ ] Serialize console writer work and exclude maintenance across processes. Do not upgrade a lifetime shared lease in place without a tested coordination design.
- [ ] Recheck settings revision, source identity, health freshness, and complete actual rows before each mutation.
- [ ] Verify exact actual rows afterward. Stop dependent work on uncertainty; retain recovery state across restart.
- [ ] Test crash boundaries, foreign changes, settings drift, batch limit 20, competing processes, and no marker bypass.
- [ ] Run wire tests to GREEN and review rollback semantics for additions, removals, and list changes. Commit.

## Task 4: Console controls, scheduler, and publication

Files: modify `v3/src/web.rs`, `v3/src/orchestrator.rs`, the existing form/render modules identified by targeted search, and `v3/tests/web.rs`; add focused scheduler tests.

Interfaces: an authenticated settings action selects preview, paused, or automatic mode with a current settings revision. A bounded worker consumes the latest valid reconciliation plan.

- [ ] Write failing tests for default preview, pause, stale forms, token checks, enable prerequisites, and settings changes during queued work.
- [ ] Add modal controls for probes, failure mode, pin override, cleanup grace, thresholds, and batch size.
- [ ] Show desired/observed counts, health reasons, blocked moves, pending journals, and last verified operation separately from applied shaping.
- [ ] Keep network I/O outside the App lock. Bound collection concurrency and serialize mutations per router.
- [ ] Validate actual membership before normal output publication. A candidate or list update must never claim production shaping apply.
- [ ] Test maintenance exclusion, restart with unresolved intent, and disable/pause behavior before the next send.
- [ ] Run focused UI/scheduler tests to GREEN. Update operational guides and commit.

## Task 5: Acceptance and rollout evidence

- [ ] Run `cargo fmt --check`, `cargo test --locked`, and `cargo clippy --locked --all-targets -- -D warnings` from `v3/`.
- [ ] Run existing large synthetic assignment/core fixtures plus bounded-writer measurements. Record measured time, memory, and operation counts; compare with the same baseline workload.
- [ ] Obtain a fresh code review covering ownership, performance, concurrency, and interruption behavior. Fix findings and rerun affected checks.
- [ ] In a separately authorized isolated RouterOS environment, verify completed-probe freshness, WAN attribution, same-ID list changes, and connection-mark limitations.
- [ ] Record unsupported cases as blocked. Do not claim isolated-router proof from a mock protocol server.
- [ ] Update Brain, handoff, and guides with actual check results. Commit and push after verification.
- [ ] Keep production automatic mode disabled until managed address scope, residual handling, probe attribution, and exact rollout authorization are concrete and reviewed.

## Execution status

Health configuration, state machine, conservative provider, and read-only CLI are implemented.
Verification: 243 tests passed, zero failures, 12 manual tests ignored. Format and strict Clippy passed.
Focused health/provider and CLI checks: 11 passed. Fresh review found no material defect.
The provider still requires isolated RouterOS field/completion verification. Tasks 2–5 remain pending.
No production reads, writes, settings changes, or restarts occurred during implementation.
See the WAN health execution ledger and `v3/WAN_HEALTH.md`.
