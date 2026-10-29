# Core Address-List Planner Implementation Plan

> **For agentic workers:** Use `superpowers:executing-plans` for inline implementation and one final independent review.

**Goal:** Add automatic local per-subscriber core list plans and conservative route checks to the Rust UI.
**Architecture:** Derive one plan from the normalized subscribers and persisted assignments used for the same shaping generation.
Store policy in application TOML and `core-plan.json` in the immutable generation before its pointer switches.
Reuse Axum/Maud forms, selected read rounds, health gates, and guarded publication.
**Tech Stack:** Existing Rust, Axum, Maud, Serde, and RouterOS read-only client. No new dependency.
**Spec:** The user-approved next slice in this session and `v3/WAN_ASSIGNMENTS.md`.

## Global constraints

- Change only `/opt/LQoSync3.0`. Do not write to routers, LibreQoS, packages, or services.
- RouterOS 7 structural checks only. Do not claim end-to-end path verification or simulate arbitrary firewall policy.
- Bind each WAN to its mapped core/both router, a unique positive list name, and a unique custom routing table.
- Require fresh complete inventory and matching direct prerouting list-to-table marks with `passthrough=false`.
- Block complex prerouting rules, active routing rules, FastTrack, missing tables, and ambiguous or wrong-WAN defaults.
- Route checks establish limited structural evidence only. NAT, VRF, destination exceptions, and existing connections need isolated validation.
- List ownership uses a persistent application UUID and exact subscriber/WAN marker. Never remove manual or other-owner rows.
- Desired addresses are individual IPv4 `/32` hosts. Reject duplicate subscriber identities and IPs.
- Protect manual overlaps, broad ranges, DNS entries, malformed markers, dynamic/disabled owned rows, and duplicate owned rows.
- Incomplete/failed/stale inputs retain the entire last valid generation. No deletion plan from a stale snapshot.
- Persist the plan with the snapshot and assignments. Legacy generations remain readable. Declared missing/corrupt plans fail closed.
- Run heavy work outside async workers and application locks. Paginate previews and preserve encoded search.
- Commit and push `lqosyncv3` after tests and performance/integrity review, as explicitly authorized.

## Review focus

1. A stale saved policy or generation must not authorize a plan from current-looking old data.
2. A manual `/24`, IP range, or DNS member must not become a system-owned removal.
3. An ownership-like comment is not ownership unless the entire marker and list/WAN scope match.
4. A valid default route does not prove traffic uses it. Unsupported routing constructs must remain visibly unresolved.
5. A 50k-subscriber plan must avoid per-subscriber scans of all list members or routes.

## Task 1: Pure planner and read inventory

**Files:** Create `v3/src/core_plan.rs`, `v3/tests/core_plan.rs`, and shared core fixtures. Modify `model.rs`, `lib.rs`, `routeros.rs`.
**Interfaces:** `core_plan::Policy`, `Binding`, `Plan`, `Entry`, `Action`; `build(settings, subscribers, assignments, inventories, errors, now) -> Result<Plan>`.

- [x] Write failing tests for valid `/32` diffs, exact ownership, manual overlaps, deterministic ordering, invalid routes, health failures, and rate preservation.
- [x] Add default-empty policy and static binding validation. Keep existing settings readable.
- [x] Extend allowlisted print inventory for core/both routers. Read complete firewall match fields and reject unknown conditional matchers.
- [x] Implement indexed host/range comparison and one route audit per core, without mutation commands.
- [x] Run focused tests until they pass.

## Task 2: Atomic publication and orchestration

**Files:** Modify `store.rs`, `orchestrator.rs`, storage/orchestration tests.
**Interfaces:** `publish_orchestrated(..., Option<&core_plan::Plan>)`; `load_core_plan(root)` reads one committed generation.

- [x] Write failing tests for plan/snapshot consistency, declared missing/corrupt plans, legacy reads, and failed-read generation retention.
- [x] Build the core plan from the same fresh normalized/assigned inputs before publication commits.
- [x] Persist a fifth file and pointer flag. Compare the plan when reusing generations.
- [x] Poll required core routers and retain existing pause, concurrency, and stale-result guards.
- [x] Run storage and orchestration checks.

## Task 3: UI, review, and delivery

**Files:** Create `core_web.rs`, `tests/core_ui.rs`, `CORE_PLANNER.md`. Modify navigation, project docs, and brain notes.

- [x] Write failing HTTP tests for forms, token checks, invalid-edit preservation, automatic plans, and search pagination.
- [x] Add per-WAN forms and a paginated add/keep/remove plan preview with generation, inventory times, and protected counts.
- [x] Display route/read failures and clearly label the last valid plan and disabled production execution.
- [x] Measure 50k subscribers and large list inventories. Check stable output and ownership preservation.
- [x] Request an independent performance/integrity review and fix material findings with regression tests.
- [x] Run the full suite, formatting, strict Clippy, and diff checks. Update notes and prepare the authorized commit and push.

## Ledger

Initial: clean `lqosyncv3`, base `7c7b68c`. Existing suite: 68 functional tests and three ignored manual benchmarks.
The task continues in the authorized checkout. The user already approved implementation, review, commit, and push.

Task 1: pure planner and complete-field read tests pass. No new dependency or mutation command.
Task 2: atomic plan publication, compatibility, corruption, and retained-generation tests pass.
Task 3: mapping forms, stale revisions, automatic planning, escaped paginated previews, and actual HTTP smoke pass.
Independent review found one source-freshness gap. Its regression failed before the fix and passed after it.
The plan now records all selected source/core timestamps. The publisher checks them immediately before the pointer switch.
Final checks: 81 functional tests passed. Four manual benchmarks are ignored by default.
The new benchmark passed separately: 50k subscribers /60k members, plan 876 ms, integrity 211 ms, JSON 791 ms, debug.
Formatting, strict Clippy, build, and diff checks passed. Collection and disk publication are excluded from the benchmark.
