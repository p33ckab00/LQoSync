# Topology re-enrollment before activation plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development or superpowers:executing-plans. Steps use checkbox (`- [ ]`) syntax.

**Goal:** Let the operator enroll an already enrolled registry again with `lqosync3 topology enroll --replace`, before activation, so that new legacy IDs (for example a DHCP client that came back with a new random Circuit ID) get aliases.

**Architecture:** With `--replace`, the runner prepares the pair from a copy of the registry with `export_ownership = None`, then uses the normal enrollment path (`build_manifest`, `enrollment::prepare`, `adopt_legacy`, staging). Subscribers, catalog, source settings, plans and pins stay, because they live outside `export_ownership`. The operator confirms the whole mapping file again (owner decision 2026-11-20: designer choice, full mapping).

**Tech Stack:** Rust 2024. No new dependency.

## Owner decision (2026-11-20)

Option A: a code path for re-enrollment before activation. The designer chose a full mapping file again (same flow as the first enrollment: `propose`, review, dry run, `--yes`).

## Global Constraints

- Rust 1.89 compatible. No new dependency.
- Without `--replace`, an enrolled registry still refuses enrollment as today (`legacy ownership is already configured`).
- `--replace` refuses when the manager owns the writer: `crate::topology_manager::runtime::topology_owns(state_dir)` is `Ok(true)`, or returns `Err`. Error text: `the Topology Manager is active; re-enrollment is not allowed`.
- `--replace` refuses on a registry that is not enrolled. Error text: `the topology registry is not enrolled; use enroll without --replace`.
- `--replace` without `--yes` is a dry run: exit status 2 and nothing installed, as today.
- Keep every existing check of the enrollment path (parity, fresh pair, stale evidence, static roots, attachments).
- `topology propose --replace` prepares the pair the same way, so the proposed file matches the enroll check.
- Persisted text in ASD-STE100.

## Review Focus

- A registry edit made after the first enrollment (a subscriber plan, a pin) is still in the registry after `enroll --replace --yes`. Test.
- A legacy row whose Circuit ID changed gets the new alias, and the old alias is gone. Test.
- `--replace` on an active manager (`runtime-owner.json` present) changes nothing. Test.
- The registry revision increases by one per `--replace --yes`, and the store commit uses the current revision (no lost update). Test.

---

### Task 1: `--replace` for propose and enroll

**Files:**
- Modify: `src/cli.rs` (`TopologyCommand::Propose` and `TopologyCommand::Enroll` get `replace: bool`; parse `--replace`; usage text)
- Modify: `src/topology_cli.rs` (pass `replace`)
- Modify: `src/runner.rs` (`propose_enrollment`, `enroll_with_mapping`, `prepare_topology`)
- Test: `src/runner/tests/enrollment.rs`, the `cli.rs` parser tests
- Modify: `docs/operations/TOPOLOGY_ENROLLMENT.md` (a section "Enroll again before activation")

**Interfaces:**
- `Runner::prepare_topology(&mut self, max_age: Duration)` stays. Add `async fn prepare_topology_for(&mut self, max_age: Duration, replace: bool) -> anyhow::Result<PreparedPair>`. With `replace`, it checks the two refusals above, clones the store document, sets `export_ownership = None`, and prepares from the clone. `prepare_topology` calls it with `false`.
- `propose_enrollment(&mut self, previous, max_age, replace: bool)` and `enroll_with_mapping(&mut self, file, max_age, install: bool, replace: bool)`.

- [ ] **Step 1: Write the failing tests** in `src/runner/tests/enrollment.rs`, with the existing fixture and `enroll` helper:

```rust
#[test] fn a_second_enrollment_without_replace_is_refused()     // error contains "legacy ownership is already configured"
#[test] fn replace_enrolls_a_changed_legacy_circuit_id()          // change one legacy Circuit ID in the live CSV, propose --replace, confirm, enroll --replace --yes:
                                                                  // the new id is in export_ownership.circuits, the old id is not, revision == old + 1
#[test] fn replace_keeps_registry_edits()                        // set a subscriber plan through the existing API after the first enrollment, then replace: the plan is still there
#[test] fn replace_refuses_an_active_manager()                    // write runtime-owner.json as the api tests do: error text above, revision unchanged
#[test] fn replace_refuses_a_registry_that_is_not_enrolled()      // error text above
#[test] fn replace_without_yes_installs_nothing()                 // revision unchanged
```

And a parser test in `cli.rs`: `topology enroll --mapping F --replace` gives `replace: true`; without the flag, `false`.

- [ ] **Step 2: Run them and make sure that they fail**

Run: `CARGO_BUILD_JOBS=1 nice -n 19 cargo test --locked --lib -- enrollment cli`
Expected: compile errors or FAIL.

- [ ] **Step 3: Implement**

- [ ] **Step 4: Update `TOPOLOGY_ENROLLMENT.md`**: when to use it (a legacy ID changed after the enrollment, before activation), the commands (`propose --replace`, review, `enroll --replace`, `enroll --replace --yes`), and that the service must be stopped, as for the first enrollment. Add a NOTE: after activation, the manager keeps stable IDs and `--replace` is refused.

- [ ] **Step 5: Run the full checks**

```bash
CARGO_BUILD_JOBS=1 nice -n 19 cargo test --locked
cargo fmt --check
CARGO_BUILD_JOBS=1 nice -n 19 cargo clippy --locked --all-targets -- -D warnings
git diff --check
```

Expected: all pass and clean.

- [ ] **Step 6: Commit** with the subject `feat(topology): enroll again before activation`.
