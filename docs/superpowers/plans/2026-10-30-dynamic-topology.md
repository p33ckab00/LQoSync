# Dynamic Topology Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans or superpowers:subagent-driven-development to implement this plan task-by-task. Track the steps with the checkboxes below.

**Status:** Complete. Code `9070d1b` is pushed and verified on `origin/lqosyncv3`.

**Goal:** Let operators select sources and destinations, edit arbitrary trees, and configure the console through compact modal forms.

**Architecture:** Separate inventory selection, exported ancestry, and routing WAN identity. Add an optional versioned configuration to existing settings and preserve the legacy path until an explicit validated migration. Use one assignment result for candidate sessions, capacity allocation, core plans, exports, and validation.

**Tech Stack:** Rust, Axum, Maud, TOML, existing same-origin JavaScript, native modal dialogs. No new dependency.

**Spec:** `docs/superpowers/specs/2026-10-30-dynamic-topology-design.md`.

## Global constraints

- Work in `v3/`. Keep the legacy root application and `/opt/libreqos/src/updatecsv.py` unchanged.
- Preserve subscriber UUIDs, manual pins, source exclusions, and protected manual RouterOS members.
- Keep candidate, committed, and applied state distinct. A GET cannot migrate or publish configuration.
- Keep the current form tokens, ownership checks, publication gates, and Content Security Policy.
- Keep private settings, credentials, inventory, backups, and customer records out of Git and notes.
- Do not change dependencies, restart the live console, write router configuration, or apply LibreQoS output.
- Commit and push only after required checks and the final code/performance review pass, under the existing user authorization.

## Execution and isolation

Recommended execution: native implementation in this session, with one fresh review of the completed change.
Confirm the execution method at the plan handoff. Do not request approval between implementation tasks.
Before implementation, use `using-git-worktrees` to check isolation and establish the workspace.
Preserve the current uncommitted design and activation notes. Copy only the relevant sanitized notes into an isolated workspace if used.
Keep runtime data out of the implementation workspace. Run browser checks with separate synthetic data and a separate port.
Run the existing locked Rust tests to establish the baseline before product edits.

## Review focus

1. A promoted branch and its descendants must keep their routing WAN even though they no longer have a WAN ancestor.
2. Several roots on one WAN must share its minimum budget. One root across several WANs must retain per-client core membership.
3. Inactive pins and legacy session evidence must survive migration without a fabricated reconnect or silent replacement.
4. Stale edits or unresolved references must preserve the prior settings, candidate evidence, and committed generation.
5. A modal must remain usable above mobile navigation, sticky content, menus, and notices, including after a failed save.

## Data interfaces

Implement the following types in a new `v3/src/dynamic.rs`, registered in `lib.rs`.
Use the existing identifier rules, strict unknown-field rejection, and explicit reference checks.

| Type | Fields and purpose |
| --- | --- |
| `Config` | `version: u8` fixed to `1`, `wans: Vec<Wan>`, `policies: Vec<Policy>`, `pins: BTreeMap<String, DynamicPin>`, `replacements: Vec<Replacement>` |
| `Wan` | `id`, `name`, `router`, `interface` as strings, plus positive finite `download`, `upload` capacities |
| `Target` | `node: String`, `wan: String`, with equality and deterministic ordering |
| `Policy` | `id: String`, `name: String`, `selections: Vec<String>`, `targets: Vec<Target>`, `default: Target`, `balance: bool`, `metric: String` |
| `DynamicPin` | `policy: String`, `target: Target`, keyed by normalized subscriber UUID |
| `Route` | `selection: String`, `policy: String`, `node: String`, `wan: String`, keyed by subscriber UUID in persisted assignments |
| `Collection` | `subscribers: Vec<Subscriber>`, `sources: BTreeMap<String, String>` mapping subscriber UUID to selection ID |

Add `Settings.dynamic: Option<Config>` with a default of `None` and omission when absent.
Keep `Settings.nodes` as the exported tree. Keep old public node/subscriber fields readable.
Allow `Selection.node` to default to an empty string in the new configuration. Dynamic policies supply its actual destination.
Add `AssignmentState.routes: BTreeMap<String, Route>` with a default empty map and omission when empty.
Keep legacy assignment records readable. Dynamic output requires exact agreement between the route, assigned node, and subscriber snapshot.

All legacy entry points continue to work when `dynamic` is absent.
Use explicit dispatch at shared boundaries. Do not allow partially mixed legacy and dynamic ownership.
After migration, clear legacy node source bindings and legacy assignment pools/pins in the new settings.
Keep source selectors as independent records and retain core binding WAN IDs.

### Task 1: Add the schema and reversible legacy migration

**Files:** `dynamic.rs`, `model.rs`, `selection.rs`, `assignment.rs`, `store.rs`, `lib.rs`; new `tests/dynamic_migration.rs`.

**Interfaces:**

- `dynamic::validate(settings: &Settings) -> anyhow::Result<()>` validates the optional configuration and all references.
- `dynamic::migrate(settings: &Settings) -> anyhow::Result<Settings>` creates a validated in-memory conversion without disk writes.
- `store::save_settings` creates a private durable backup before the first explicit legacy-to-dynamic save.
- Assignment persistence validates the new routes without changing the old generation format flags.

- [x] Write failing tests for old settings round trips, mixed service selectors, legacy shared pools, inactive pins, and unmapped WAN failures.
- [x] Assert migration preserves node IDs, WAN IDs, subscriber identity inputs, eligible targets, modes, metrics, and unrelated settings.
- [x] Run the focused migration tests and record their actual failures before implementation.
- [x] Implement the types and conversion. Derive legacy selection IDs deterministically from exact selector identities with the existing UUID dependency.
- [x] Preserve one shared legacy policy for selectors that previously shared a source node. Convert its targets using their old WAN ancestors.
- [x] Test no backup or mutation on GET, backup preservation on retries, and rejection before replacement on invalid migration.
- [x] Run focused tests and the locked suite. Commit the verified schema/migration slice without private data.

### Task 2: Normalize independent sources and plan explicit destination pairs

**Files:** `dynamic.rs`, `normalizer.rs`, `sessions.rs`, `assignment.rs`, `store.rs`; new `tests/dynamic_assignments.rs`.

**Interfaces:** Use the current inventory and error map types from `normalizer::collect`.

- `normalizer::collect_with_sources(settings, inventories, errors, now) -> anyhow::Result<Collection>` accepts the same argument types as `collect`.
- `dynamic::assign_targets(settings: &Settings, subscribers: &mut [Subscriber], previous: &AssignmentState) -> anyhow::Result<AssignmentState>` plans explicit routes. `sessions::plan` retains ownership of session evidence and inventory guards.
- It updates the subscriber vector only after validation. Its result includes assignments with complete `Route` records.
- Keep the current `sessions::plan` entry point as the shared session planning boundary with explicit dynamic dispatch.

- [x] Write failing cases for a root-only tree, two selected roots, shared mixed-service destinations, and one exported root paired with two WANs.
- [x] Assert the operator can omit the former source node from a policy. Reject an empty set, duplicate pair, or default outside the set.
- [x] Prove individual selectors override groups and exclusions, including healthy disappearance of selected DHCP leases.
- [x] Run the focused cases and observe their failures.
- [x] Implement indexed policy/selection/target lookup. Retain selection IDs during normalization. Resolve destinations in the assignment planner, independently of service-kind ancestry.
- [x] Reuse current session handle, uptime, absence, freshness, revision, and captured-input guards.
- [x] Translate legacy assignments and evidence only when source/policy/WAN ownership is unambiguous. Reject unresolved evidence without writes.
- [x] Test continuous retention, reconnect redistribution, all manual pins counted before placement, failed reads, same-second reads, and healthy empty scans.
- [x] Test malformed route state and conflicting source ownership fail closed. Run focused tests and the locked suite, then commit.

### Task 3: Enforce both WAN and exported-tree budgets through publication

**Files:** `minimums.rs`, `model.rs`, `core_plan.rs`, `core_analysis.rs`, `core_diagnostics.rs`, `validation.rs`, `orchestrator.rs`, `store.rs`, `assignment_web.rs`, `wan_web.rs`; new `tests/dynamic_output.rs`.

**Interfaces:**

- Add `minimums::allocate_assigned(settings: &Settings, subscribers: &mut [Subscriber], assignments: &AssignmentState) -> anyhow::Result<Allocation>`.
- Add `model::compile_assigned(settings: &Settings, subscribers: &[Subscriber], assignments: &AssignmentState) -> anyhow::Result<Plan>`.
- Keep old allocation/compile wrappers for the legacy and offline paths. Managed dynamic output must use the assignment-aware functions.
- `core_plan::build` uses `AssignmentState.routes[id].wan` in dynamic mode. It must reject missing or inconsistent routes.
- Validation, local publication, assignment previews, and edit previews consume the same planned routes.

- [x] Write failing fixtures for two roots sharing a WAN and one root spanning WANs, in both directional limits.
- [x] Assert root-only CSV parents, exact JSON roots, stable IDs, and correct core WAN/list mapping independently of exported ancestry.
- [x] Assert source maximums stay unchanged. Floors that cannot fit either budget must reject the entire proposed allocation.
- [x] Observe the focused failures before implementation.
- [x] Apply proportional reductions against both ancestor groups and WAN groups. Reductions never increase earlier constrained values or drop below floors.
- [x] Validate both constraints before modifying caller-owned records. Keep deterministic ordering and indexed membership.
- [x] Wire all managed output/review paths. Preserve old generations on failure and save healthy candidate evidence under the current guards.
- [x] Test protected manual membership still blocks publication. Test corrupt routes, stale input, and missing WAN evidence.
- [x] Run focused tests and the locked suite, then commit.

### Task 4: Implement atomic hierarchy operations and operator controls

**Files:** new `topology_edits.rs`, `web.rs`, `noc_web.rs`, `sources_web.rs`, `assignment_web.rs`, `wan_web.rs`, `lib.rs`; new `tests/dynamic_edits.rs` and focused web tests.

**Interfaces:**

- `topology_edits::Operation` describes Add, Reparent (including promotion), and Remove with explicit replacement/child destinations.
- `topology_edits::preview(settings, operation) -> anyhow::Result<EditPlan>` returns validated proposed settings and affected counts.
- `EditPlan` includes the prior settings revision and explicit destination replacements. Save checks the revision and current generation/evidence guards.
- Reuse the existing settings commit and form-token boundaries. Node IDs cannot be renamed.

- [x] Write failing cases for arbitrary nested groups, one-level/top-level promotion, demotion, root removal, and conversion to a flat root.
- [x] Assert descendants and WAN assignments survive moves. Assert cycles, unresolved children, references, pins, and stale saves leave settings unchanged.
- [x] Observe the failures, then implement pure edit planning before handlers.
- [x] Add source and target selectors that show names and stable IDs. Sources can attach to existing nodes without creating service layers.
- [x] Expose explicit replacement choices for active destinations. Do not modify candidate files during a preview or fabricate reconnect evidence.
- [x] Ensure the next fresh assignment plan applies authorized replacements consistently while committed output remains unchanged until publication passes.
- [x] Replace legacy-only WAN/source/pool controls with the effective configuration view. Keep existing legacy settings usable before explicit conversion.
- [x] Run focused operation/web tests and the locked suite, then commit.

### Task 5: Use modal forms and cog controls throughout the console

**Files:** `console.js`, `style.css`, all relevant `*_web.rs` presentation modules, `web.rs`, `noc_web.rs`, `lib.rs`; `tests/web.rs` and a synthetic browser check script.

**Interfaces:**

- `configurationEditor(form, index)` progressively enhances existing guarded forms with an accessible cog and native dialog.
- The shared script opens it with `showModal`, contains interaction, preserves drafts, and restores focus/scroll on close.
- The existing POST handlers remain authoritative. Same-origin form submission renders server validation errors within the active dialog and preserves the submitted values.

- [x] Add a browser regression that proves a configuration form expands the page or fails to open as a dialog before implementation.
- [x] Implement the shared primitive and convert node, router, source, WAN, policy, pin, speed, core, and settings edit forms.
- [x] Keep current values/status visible and label every cog by its target. Keep Add/Promote/Remove action labels explicit.
- [x] Use one active modal, with replacement/removal steps in that modal. Handle unsaved dismissal inside it without stacked dialogs.
- [x] Place errors/notices within the dialog. Close mobile navigation first and keep dialog menus within its interaction boundary.
- [x] Check Save, invalid save, retry, Cancel, close, Escape, Tab, restored focus, draft preservation, and stale-token failure.
- [x] Check desktop/mobile widths, 200 percent zoom, internal scroll, sticky content, clipping containers, menus, and modal/backdrop order.
- [x] Run focused Rust web tests, browser checks, JavaScript syntax, and the locked suite, then commit.

### Task 6: Review performance, document behavior, and finish

**Files:** `v3/README.md`, `FLEXIBLE_TOPOLOGY.md`, `WAN_ASSIGNMENTS.md`, source/output guides as affected, implementation tests, plan, and sanitized project notes.

- [x] Run `cargo fmt --check`, `cargo test --locked`, and `cargo clippy --locked --all-targets -- -D warnings` from `v3/`.
- [x] Run meaningful 50k-client workloads for new/retained/reconnected assignments, mixed-service selectors, overlapping WAN/tree budgets, and candidate persistence.
- [x] Check bounded depth and collection/assignment indexes. Compare debug timings with the existing baseline without claiming a production guarantee.
- [x] Run browser checks on all console routes with synthetic data. Confirm forms remain hidden until invoked and no CSP or overflow errors occur.
- [x] Obtain a fresh whole-change review against the spec and the five Review Focus conditions. Fix material issues with failing regressions first.
- [x] Run affected checks after any fix. Do not present historical test results as current evidence.
- [x] Update guides, plan status, Home, session note, and HANDOFF with actual results and limits. Keep the Graphify baseline limitation explicit unless updated.
- [x] Review the complete staged diff for private data. Commit the final verified result and push to the authorized GitHub branch.
- [x] Verify the remote commit matches the intended local commit. Report behavior, verification, and remaining operational limits.

## Stop condition

The approved arrangements and modal workflows pass their regressions and current checks, the final review has no unresolved material issue, and the verified code reaches GitHub.
Live activation and production router/shaping execution require their own task authorization.

## Execution rulings and evidence

Tasks 1–3 share the persisted routing identity and were implemented together in the core commit.
Tasks 4–5 reuse existing guarded forms. The shared script creates native dialogs; no separate Rust modal module is needed.
Policy names and explicit replacement history are persisted operator state. Promotion reuses pure reparent planning.
Legacy destination adoption follows explicit replacement history. Source edits that would orphan unresolved legacy evidence are rejected before save.
The assignment view uses complete node/WAN pairs. It hides pin controls when a legacy snapshot cannot resolve the pair.
Policy rendering indexes selections, bounds summaries, computes revisions once, and reuses captured evidence for impact counts.
HTML rendering runs outside the application mutex. Form attributes prevent named controls from shadowing DOM properties.

One fresh whole-change review found five Important and two Minor findings. The correction pass addressed each finding.
Failing regressions preceded the migration, stale-form, exact-pair, unresolved-pair, and dialog fixes.
The existing mock-router timeout occurred once under build/browser load. It passed alone without changing its timeout.
A clean suite rerun then passed 196 tests, zero failures, with 12 manual tests ignored.
Final checks after the captured-evidence optimization passed: format, 196 tests with zero failures and 12 ignored, strict all-target Clippy, locked build, JavaScript syntax, and both packaged browser scripts.
Main was fast-forwarded to the exact verified code; its `v3/` tree matches the isolated worktree.
The GitHub branch was verified at `9070d1b41e626ed6ebcf835d975b6f40ef88d71f`. Documentation capture follows in a separate commit.
The synthetic preview stopped after the checks. The live console remains on the earlier activated build.

The 50k workload also renders the policy page with and without a stored candidate.
Latest debug measurements: empty-evidence page 1.433 s /31,658 bytes; persisted-candidate page 5.651 s /31,646 bytes.
Collection 1.167 s; assignment 1.563 s; retention including collection 3.901 s; allocation/compile 1.554 s; reconnect 2.273 s.
Candidate load 3.985 s. Durable saves varied from 1.520 s to 7.515 s across runs with different host I/O load.
These are synthetic debug timings, not production guarantees. The Graphify map remains a baseline, so current source confirmed relationships.
