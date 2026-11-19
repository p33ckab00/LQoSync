# Speed policy and group factor implementation plan (Phase 2b-4)

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Edit the global Subscriber Speed Policy and the factor of each subscriber group through the propose and confirm flow, with a proposal that shows changed subscriber rates, changed group capacities, and legacy warnings.

**Architecture:** `edits::apply` gets `speed.defaults` and `group.factor` and marks them check-only. For a check-only edit the runner reads the sources once, resolves and evaluates aggregates for the current and the edited catalog over the same active subscribers, refuses only new conflicts, and keeps the catalog edit as the proposal. `topology.policies` exposes the group settings. The Policies view gets two forms.

**Tech Stack:** Rust 2024, Tokio, ratatui, serde_json. No new dependencies. MSRV 1.89.

**Spec:** `docs/superpowers/specs/2026-11-19-topology-speed-policy-design.md`

## Global Constraints

- Do not stop, restart, deploy, or reconfigure `lqosync3.service`. Do not touch `/etc/lqosync3`, `/var/lib/lqosync3`, `/opt/libreqos/src`, or routers.
- No new dependencies. Keep `cargo +1.89 build --locked` working. No registry schema change.
- Gate for every task: `cargo fmt --check`, `cargo clippy --all-targets --locked -- -D warnings`, `cargo test --locked`. Run cargo with `nice -n 19`, `CARGO_BUILD_JOBS=2`.
- Separate worktree, branch `topology-speed-policy` from `lqosync3`. Do not edit `docs/OPERATIONS.md`.
- Persisted text follows ASD-STE100.
- Ranges (spec): minimum percent 0.1 to 100, maximum percent 1 to 1,000, one decimal at most (`Fraction { numerator: p × 10, denominator: 1000 }`); Mbps whole 1 to 1,000,000; group factor a decimal string `"0.001"` to `"1"` with up to six decimals (`Fraction { numerator, denominator: 1_000_000 }`).
- Exact texts (spec): "the source does not exist", "the minimum is above the maximum for: <subscriber IDs>", "resolve the placement conflicts first", "the legacy rounded rules of <n> profiles are replaced; the output no longer matches the legacy loop exactly".
- `speed.defaults` resets every profile `speed` to `SpeedOverrides::default()`.

## Review Focus

1. The legacy PPP factor `0.215` is entered again after a change: it round-trips exactly. Test in Task 1.
2. A fixed 5 Mbps minimum with a 2 Mbps plan subscriber: refused with the `InvalidRates` text naming the subscriber. Test in Task 2.
3. A registry that already has `MinimumExceedsCeiling`: a lower minimum that removes it is accepted. Test in Task 2.
4. `group.factor` with only `down`: `up` keeps its value. Test in Task 1.
5. A distro router that is down: speed and factor edits answer the read error and keep no proposal. Test in Task 2.

---

### Task 1: Speed default and group factor edit kinds

**Files:**
- Modify: `src/topology_manager/edits.rs`
- Test: `src/topology_manager/edits_tests.rs`

**Interfaces:**
- Produces:
  - `Edit::SpeedDefaults { down_min: RuleInput, up_min: RuleInput, down_max: RuleInput, up_max: RuleInput }` (`kind: "speed.defaults"`), with `#[serde(untagged)]`-free `RuleInput` as `{"percent": f64}` or `{"mbps": u64}` (`#[serde(rename_all = "snake_case", deny_unknown_fields)] enum RuleInput { Percent(f64), Mbps(u64) }` in externally tagged form).
  - `Edit::GroupFactor { source_id: String, down: Option<String>, up: Option<String> }` (`kind: "group.factor"`).
  - `Applied` adds `check_only: bool` (true for both kinds) and `legacy_replaced: usize` (profiles whose `speed` held a `LegacyRoundedBase` rule before the reset).
  - `pub(crate) fn factor_text(f: Fraction) -> String`: a decimal without trailing zeros (`0.215`, `0.5`, `1`).
- `affected` stays empty for both kinds.

- [ ] **Step 1: Write the failing tests:**
  - `speed_defaults_set_the_defaults_and_reset_every_profile_override` (10% min and 100% max give `BaseFraction(100/1000)` and `BaseFraction(1000/1000)`; `legacy_replaced` counts the imported profiles).
  - `speed_default_ranges_and_decimals` (0.05, 100.5, 0.15, 1001, 0 Mbps refused; 0.1, 12.5, 1000, 500 Mbps accepted).
  - `a_minimum_above_the_maximum_for_a_local_plan_is_refused` (the existing validation text).
  - `group_factor_round_trips_the_legacy_value_and_keeps_an_absent_direction` (Review Focus 1 and 4: `"0.215"` stores `215000/1000000`, `factor_text` gives `"0.215"`; only `down` keeps `up`).
  - `group_factor_ranges_and_unknown_source` ("0", "1.5", "0.0000001", "abc" refused; unknown source gives "the source does not exist").
  - `instance_factor_overrides_are_kept`.
- [ ] **Step 2: Run** `cargo test --locked --lib topology_manager::edits` — Expected: FAIL.
- [ ] **Step 3: Implement.**
- [ ] **Step 4: Run** the tests and the full gate — Expected: PASS.
- [ ] **Step 5: Commit** `feat(topology): add speed default and group factor edits`.

### Task 2: Check-only proposals and the group view

**Files:**
- Modify: `src/runner.rs` (`api_edit_propose`, `api_policies`), `src/topology_manager/observation.rs` (derive `Clone` on the active subscriber types if missing)
- Test: `src/runner/tests/api.rs` (`mod edits`)

**Interfaces:**
- Consumes: Task 1 `Applied.check_only`, `Applied.legacy_replaced`, `factor_text`.
- For `check_only`: one `reconcile_for` on the candidate. Build a second `Reconciliation` with the same `active` and the reconciled registry whose catalog is the current catalog. Run `resolve` then `aggregate::evaluate` for both. Refuse only conflicts whose `Debug` text the current side does not have. A new `InvalidRates` placement conflict gives "the minimum is above the maximum for: <IDs>"; another new placement conflict gives "resolve the placement conflicts first"; a new aggregate conflict gives its text.
- The kept candidate is `Applied.candidate` (the catalog edit only; `new_records` is 0).
- Proposal adds `rates` (up to 8, `{subscriber_id, label, before: {min_mbps, max_mbps}, after: {...}}`), `rates_changed`, `capacities` (`[{node_id, before_mbps, after_mbps}]` for nodes whose exported ceiling changes), and `warnings` (the legacy text when `legacy_replaced > 0`).
- `topology.policies` adds `groups: [{source_id, name, factor: [down, up] as factor_text, formula, backstop_mbps, floor_mbps}]` and keeps the rest.

- [ ] **Step 1: Write the failing tests:**
  - `a_minimum_change_lists_changed_rates_and_the_legacy_warning`.
  - `a_factor_change_lists_changed_group_capacities`.
  - `a_fixed_minimum_above_a_plan_is_refused_with_the_subscribers` (Review Focus 2).
  - `a_lower_minimum_can_fix_an_existing_minimum_conflict` (Review Focus 3).
  - `a_minimum_above_a_wan_ceiling_is_refused_and_keeps_no_proposal`.
  - `a_down_distro_refuses_a_speed_edit` (Review Focus 5).
  - `policies_list_groups_with_factor_backstop_and_floor`.
- [ ] **Step 2: Run** `cargo test --locked --lib runner::tests::api::edits` — Expected: FAIL.
- [ ] **Step 3: Implement.**
- [ ] **Step 4: Run** the tests and the full gate — Expected: PASS.
- [ ] **Step 5: Commit** `feat(api): check-only speed and factor proposals`.

### Task 3: Policies forms and popup lines

**Files:**
- Create: `src/tui/topology/speed_form.rs`
- Modify: `src/tui/topology/policies.rs`, `src/tui/topology/mod.rs`, `src/tui/topology/proposal.rs`, `src/tui/app.rs` (hint)
- Test: same files

**Interfaces:**
- Consumes: Task 2 shapes.
- Policies sections: "Subscriber Speed Policy" with each rule as "10% of plan", "500 Mbps", or "legacy 10% rounded" (from the first profile override when the defaults are not used yet), and "Groups" with name, factor down/up, formula, backstop, floor. Group selection with Up/Down after the plans (one selection over plans then groups).
- Keys on the Policies screen: `s` opens the speed form (four rows: mode percent or Mbps cycled with Left/Right, and a value); `f` on a selected group opens the factor form (down, up; blank keeps the value). Field errors before send use the Task 1 ranges. Both send `topology.edit.propose`.
- Popup change lines: "speed defaults", "factor of <group name>". The popup lists up to 8 rate samples (`<label>  min a -> b  max c -> d`), the capacity changes (`<node name>  a -> b Mbps`), and the warnings.

- [ ] **Step 1: Write the failing tests:**
  - `policies_show_speed_rules_and_groups`.
  - `the_speed_form_validates_and_sends_speed_defaults`.
  - `the_factor_form_sends_only_filled_directions`.
  - `popup_lines_show_rates_capacities_and_warnings`.
  - `forms_fit_80_columns`.
- [ ] **Step 2: Run** `cargo test --locked --lib tui::topology` — Expected: FAIL.
- [ ] **Step 3: Implement.**
- [ ] **Step 4: Run** the tests and the full gate — Expected: PASS.
- [ ] **Step 5: Commit** `feat(tui): edit speed defaults and group factors`.

### Task 4: Records and final verification

**Files:**
- Modify: `docs/design/TOPOLOGY_MANAGER_IMPLEMENTATION.md`, `brain/30-knowledge/topology-manager-status.md`
- Create: `brain/10-sessions/2026-11-19-topology-speed-policy.md`

- [ ] **Step 1: Run** `cargo fmt --check && cargo clippy --all-targets --locked -- -D warnings && cargo test --locked && git diff --check` — Expected: all pass.
- [ ] **Step 2: Update** the records: the two edit kinds, the profile override reset, the check-only proposal, the Phase 4 note about `legacy_issues`, and the 2c next step.
- [ ] **Step 3: Commit** `docs(topology): record speed policy and group factors`.
