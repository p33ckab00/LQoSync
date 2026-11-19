# Fallback group and new sources implementation plan (Phase 2c-2)

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Keep shaping when a router gets a source that the registry does not know, by placing those subscribers in an operator-set fallback group, and let the operator add the new source from the Builder.

**Architecture:** A new optional `catalog.fallback` names fallback group nodes and a default plan. The source policy accepts unknown PPP profiles when a fallback exists, the resolver places unmatched subscribers in the fallback and re-places unpinned subscribers whose saved assignment is no longer eligible. A candidate read diffs the enrollment draft against the catalog, and `source.add` copies one draft source into the registry. The TUI adds the fallback form, a New sources section, and warnings.

**Tech Stack:** Rust 2024, Tokio, ratatui, serde_json. No new dependencies. MSRV 1.89.

**Spec:** `docs/superpowers/specs/2026-11-19-topology-fallback-and-new-sources-design.md`

## Global Constraints

- Do not stop, restart, deploy, or reconfigure `lqosync3.service`. Do not touch `/etc/lqosync3`, `/var/lib/lqosync3`, `/opt/libreqos/src`, or routers.
- No new dependencies. Keep `cargo +1.89 build --locked` working.
- Memory on the host is tight: run cargo with `nice -n 19`, `CARGO_BUILD_JOBS=1`, and tests with `-- --test-threads=2`. Gate for every task: `cargo fmt --check`, `cargo clippy --all-targets --locked -- -D warnings`, `cargo test --locked -- --test-threads=2`.
- Separate worktree, branch `topology-fallback` from `lqosync3`. Do not edit `docs/OPERATIONS.md`.
- Persisted text follows ASD-STE100.
- Schema: `TopologyCatalog.fallback: Option<FallbackPolicy>` with `#[serde(default, skip_serializing_if = "Option::is_none")]`; `FallbackPolicy { node_ids: Vec<String>, default_plan: RateBps }` with `deny_unknown_fields`. A registry without it serializes byte for byte as before.
- Fallback rate: only the router profile `rate-limit` (scope `RateFormat`, else the RouterOS default format), else `default_plan`. An attached LQoSync plan wins.
- Owner decision: an ineligible saved assignment without a pin is dropped and the subscriber is placed again; an ineligible pin stays `InvalidPin`.
- Hotspot candidates and Hotspot fallback are out of scope.
- Exact texts (spec): "move the fallback subscribers first", "the source is no longer on the router".

## Review Focus

1. A customer moved on the router from a known plan to a new plan: no `InvalidAssignment`, the subscriber lands in the fallback, and publication continues. Test in Task 3.
2. The reverse: after `source.add` of that plan, the fallback subscribers move to the new group without a manual rebalance. Test in Task 4.
3. A new profile without a fallback policy: still `MissingProfileMapping` and a held publication (Decision 10). Test in Task 2.
4. A pinned subscriber whose pin becomes ineligible: still `InvalidPin`, not the fallback. Test in Task 3.
5. A registry written before this slice loads, validates, and serializes unchanged. Test in Task 1.

---

### Task 1: Fallback policy, group nodes, and fallback edits

**Files:**
- Modify: `src/topology_manager/catalog.rs`, `src/topology_manager/edits.rs`
- Test: `src/topology_manager/catalog_tests.rs`, `src/topology_manager/edits_tests.rs`

**Interfaces:**
- Produces: `FallbackPolicy`, `TopologyCatalog.fallback`. Validation: non-empty `node_ids`, each a `SubscriberGroup` node without `source_id`, unique, positive `default_plan`; a removed node cannot stay in the list.
- `Edit::NodeCreate` gets `#[serde(default)] kind: Option<String>` (`"intermediate"` default, `"group"`). A group node has no source and capacity `automatic` or `fixed`.
- `Edit::FallbackSet { node_ids: Vec<String>, default_down_mbps: u64, default_up_mbps: u64 }` (`kind: "fallback.set"`), `Edit::FallbackClear` (`kind: "fallback.clear"`). `FallbackClear` sets `Applied.clear_fallback: bool` so the runner can apply the online check; `node.remove` of a fallback node is refused ("remove it from the fallback first").
- `tree(doc)` adds `fallback: {node_ids, default_mbps} | null`.

- [ ] **Step 1: Write the failing tests:** `a_registry_without_fallback_round_trips_unchanged` (Review Focus 5), `fallback_validation_rules`, `node_create_makes_a_group_without_a_source`, `fallback_set_and_clear_edits`, `a_fallback_node_cannot_be_removed`, `tree_shows_the_fallback`.
- [ ] **Step 2: Run** `cargo test --locked --lib topology_manager -- --test-threads=2 fallback round_trips group_without` — Expected: FAIL.
- [ ] **Step 3: Implement.**
- [ ] **Step 4: Run** the tests and the full gate — Expected: PASS.
- [ ] **Step 5: Commit** `feat(topology): add the fallback policy and group nodes`.

### Task 2: Unknown PPP profiles with a fallback

**Files:**
- Modify: `src/topology_manager/source_policy.rs` and the caller that builds it from the registry (`identity.rs` or `normalize.rs`)
- Test: `src/topology_manager/source_policy_tests.rs`

**Interfaces:**
- The source policy gets `accept_unknown: bool` (true when `catalog.fallback` is set). An unbound PPP profile then gives `ObservedPolicy { profile: Known(Some(<router profile name>)), plan }`, `plan` from that profile `rate-limit` only.

- [ ] **Step 1: Write the failing tests:** `an_unknown_profile_with_a_fallback_reads_its_rate_limit`, `an_unknown_profile_without_rate_limit_gives_no_plan`, `an_unknown_profile_without_a_fallback_is_still_missing` (Review Focus 3), `known_profiles_are_unchanged`.
- [ ] **Step 2: Run** `cargo test --locked --lib topology_manager::source_policy -- --test-threads=2` — Expected: FAIL.
- [ ] **Step 3: Implement.**
- [ ] **Step 4: Run** the tests and the full gate — Expected: PASS.
- [ ] **Step 5: Commit** `feat(topology): accept unknown profiles when a fallback exists`.

### Task 3: Fallback placement and re-placement of stale assignments

**Files:**
- Modify: `src/topology_manager/resolver.rs`
- Test: `src/topology_manager/resolver_tests.rs`

**Interfaces:**
- In `Index::prepare`: when no source matches an account and `catalog.fallback` is set, the eligible set is the fallback nodes; the profile is the record profile when set, else none; the base is the observed plan, else `default_plan`; rates use the global defaults.
- A saved assignment outside the eligible set, without a pin, is ignored (the subscriber is placed as new). A pin outside the eligible set stays `InvalidPin`.
- The preview marks fallback placements (`fallback: true` on preview subscribers) so the TUI can count them.

- [ ] **Step 1: Write the failing tests:** `an_unmatched_subscriber_goes_to_a_fallback_node_with_the_router_plan`, `without_a_router_plan_the_default_plan_applies`, `an_attached_plan_wins_in_the_fallback`, `a_moved_customer_is_placed_again_instead_of_invalid_assignment` (Review Focus 1), `an_ineligible_pin_still_conflicts` (Review Focus 4), `the_fallback_balances_across_its_instances`, `the_preview_marks_fallback_subscribers`.
- [ ] **Step 2: Run** `cargo test --locked --lib topology_manager::resolver -- --test-threads=2` — Expected: FAIL.
- [ ] **Step 3: Implement.** Existing tests that expect `InvalidAssignment` for an unpinned record get the re-placement result, each with a ledger ruling.
- [ ] **Step 4: Run** the tests and the full gate — Expected: PASS.
- [ ] **Step 5: Commit** `feat(topology): place unmatched subscribers in the fallback`.

### Task 4: Source candidates and `source.add`

**Files:**
- Modify: `src/topology_manager/edits.rs`, `src/runner.rs`, `src/api/dispatch.rs`
- Test: `src/topology_manager/edits_tests.rs`, `src/runner/tests/api.rs`

**Interfaces:**
- `Runner::api_source_candidates(&mut self) -> anyhow::Result<Value>`: `read_for_proposal`, then the enrollment draft (`policy_draft::propose`), then the draft sources whose origin (`scope`, `profile_ref`) the catalog does not have: `{key, kind, router, name, subscribers}`. `key` is the draft source ID. A draft refusal answers `invalid` with its text. Dispatch: `topology.sources.candidates`.
- `Edit::SourceAdd { key: String, parent: Option<String> }` (`kind: "source.add"`). In `api_edit_propose` the runner builds the draft again, finds the candidate ("the source is no longer on the router" when gone), and calls `edits::add_source(doc, &draft_source_parts, parent) -> anyhow::Result<Applied>`: it copies the source, its profile, and its binding into the matching selection (or a new selection), adds the first instance below `parent`, and marks the new source affected.
- `fallback.clear` in the runner: refuse when an online subscriber is in a fallback node ("move the fallback subscribers first"); clear offline assignments to fallback nodes.

- [ ] **Step 1: Write the failing tests:** `candidates_list_a_new_ppp_profile_and_leave_out_known_sources`, `source_add_makes_the_same_source_as_enrollment`, `source_add_moves_the_fallback_subscribers_to_the_new_group` (Review Focus 2), `a_vanished_candidate_is_refused`, `fallback_clear_is_refused_with_online_fallback_subscribers`, `candidates_and_source_add_are_dispatched`.
- [ ] **Step 2: Run** `cargo test --locked --lib runner::tests::api -- --test-threads=2` — Expected: FAIL.
- [ ] **Step 3: Implement.**
- [ ] **Step 4: Run** the tests and the full gate — Expected: PASS.
- [ ] **Step 5: Commit** `feat(api): list new sources and add them from the draft`.

### Task 5: TUI fallback form, New sources, and warnings

**Files:**
- Modify: `src/tui/topology/node_form.rs`, `src/tui/topology/mod.rs`, `src/tui/topology/builder.rs`, `src/tui/topology/assignments.rs`, `src/tui/app.rs` (hint)
- Test: same files

**Interfaces:**
- The node form for a new node gets a kind row (intermediate or group).
- A Builder key on a selected group node without a source opens the fallback form (default down and up Mbps; the selected node is added to the existing fallback list), and proposes `fallback.set`.
- Builder Inventory: a "New sources" section from `topology.sources.candidates` (a key loads it), lines "<name>  <kind>  <n> subscribers"; Enter on one opens the parent picker for `source.add`.
- Strip: "<n> subscribers in fallback" from preview subscribers with `fallback: true`. Assignments: those rows show "fallback: <profile>".

- [ ] **Step 1: Write the failing tests:** `the_node_form_creates_a_group_node`, `the_fallback_form_proposes_fallback_set`, `new_sources_list_and_enter_proposes_source_add`, `the_strip_counts_fallback_subscribers`, `assignments_show_the_fallback_reason`, `fallback_dialogs_fit_80_columns`.
- [ ] **Step 2: Run** `cargo test --locked --lib tui::topology -- --test-threads=2` — Expected: FAIL.
- [ ] **Step 3: Implement.**
- [ ] **Step 4: Run** the tests and the full gate — Expected: PASS.
- [ ] **Step 5: Commit** `feat(tui): fallback group and new sources in the Builder`.

### Task 6: Records and final verification

**Files:**
- Modify: `docs/design/TOPOLOGY_MANAGER_IMPLEMENTATION.md`, `brain/30-knowledge/topology-manager-status.md`
- Create: `brain/10-sessions/2026-11-19-topology-fallback-and-new-sources.md`

- [ ] **Step 1: Run** the gate and `git diff --check` — Expected: all pass.
- [ ] **Step 2: Update** the records: the fallback policy, the re-placement rule, `source.add`, the rollback note (clear the fallback before a rollback to an older build), and the Phase 3 next step.
- [ ] **Step 3: Commit** `docs(topology): record the fallback group and new sources`.
