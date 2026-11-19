# Take-effect check implementation plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace the bridge generation gate and the whole-world equality in `apply_evidence` with a per-identity take-effect check, and use it in the live test and the service health.

**Architecture:** A new pure module `apply_evidence/take_effect.rs` gets the LQoSync identities from the written pair, checks that `dataplane_transaction.json` has exactly these identities, and compares the bridge ops with the generated ops by identity. `verify.rs` returns a typed `Proof` (verified or waiting). The monitor and the live test consume `Proof`.

**Tech Stack:** Rust 2024, `serde_json`, `anyhow`, `tokio`. No new dependency.

**Spec:** `docs/superpowers/specs/2026-11-20-take-effect-check-design.md`

## Global Constraints

- Read-only access to LibreQoS files and to the analytics socket.
- Rust 1.89 compatible. No new dependency.
- No new notification. `VerifyTimeout` after `IMPORT_LIMIT_SECS` (40 min) stays.
- An unknown op tag, a duplicate identity, or a non-object op fails closed: it is an error, never "applied".
- Keep: compiler binding check, receipt and file hash checks, import check, `before == after` bridge reads, producer path check, `circuit_capacity` evidence.
- Persisted text (docs, comments, commits) in ASD-STE100.

## Review Focus

- An IPv6 address or a CIDR prefix in the CSV (`2001:db8::/64`, `10.0.0.0/24`): the mapping identity uses the parsed prefix, default 32 or 128. Test in Task 1.
- A CSV device with no IP: it has no mapping identity, and its circuit still has a leaf identity. Test in Task 1.
- Two devices in one circuit: one leaf identity, two mapping identities. Test in Task 1.
- The bridge report has an extra field in an op (beta format change): the op is waiting with that field name, not applied. Test in Task 1.
- The pair changes again while an old write waits: the monitor drops the old receipt as today and never accepts a `VerifyTimeout` receipt of an older write. Test in Task 3.

---

### Task 1: Take-effect module

**Files:**
- Create: `src/apply_evidence/take_effect.rs` (tests in the same file, `#[cfg(test)] mod tests`, as `transaction.rs` does)
- Modify: `src/apply_evidence.rs` (add `mod take_effect;` and `pub use take_effect::{Kind, TakeEffect, Waiting};`)

**Interfaces:**
- Produces:
  - `pub(crate) struct Identities` (private fields) and `pub(crate) fn identities(network: &str, rows: &[crate::model::CsvRow]) -> anyhow::Result<Identities>`
  - `pub(crate) fn coverage(ids: &Identities, generated: &serde_json::Value) -> anyhow::Result<()>`
  - `pub(crate) fn compare(generated: &serde_json::Value, applied: &serde_json::Value) -> anyhow::Result<TakeEffect>`
  - `#[derive(Debug, Clone, PartialEq, Eq, Serialize)] pub enum Kind { Circuit, Address, Node }`
  - `#[derive(Debug, Clone, PartialEq, Eq, Serialize)] pub struct Waiting { pub kind: Kind, pub identity: String, pub fields: Vec<String> }` (`fields` is `["missing"]` for an absent op)
  - `#[derive(Debug, Clone, PartialEq, Eq, Serialize)] pub struct TakeEffect { pub applied: usize, pub waiting: Vec<Waiting>, pub waiting_removal: Vec<(Kind, String)> }` with `pub fn took_effect(&self) -> bool` and `impl Display` (count, then at most 20 identities with their fields).

Identity rules (from the spec):

| Kind | From the pair | Key in the transaction |
|---|---|---|
| Circuit | each CSV `circuit_id`, with parent = `id` of the `network.json` node whose name is `parent_node` | leaf op `circuit_id`, `parent_node_id` |
| Address | each CSV device IP in `ipv4` and `ipv6` (comma list, optional `/prefix`, default 32 or 128), with `circuit_id` and `device_id` | mapping op `ip`, `prefix_len`, `circuit_id`, `device_id` |
| Node | each `network.json` node `id`, with parent = `id` of the enclosing node, or `root` at the top | inner op `node_id`, `parent_node_id` |

Ops: leaf and inner ops must have `"op": "upsert"`, mapping ops `"op": "map"`. Any other tag is an error. Reuse the address parser of `import_evidence.rs` (`addresses`) by making it `pub(crate)`. Do not write a second parser.

- [ ] **Step 1: Write the failing tests**

Use a fixture pair: `network.json` with node `A` (`id` `n-a`) and child `B` (`id` `n-b`), CSV with circuit `C1` (two devices, `10.0.0.1` and `2001:db8::/64`, parent `B`) and circuit `C2` (one device without IP, parent `A`). Build the matching `generated` value by hand.

```rust
#[test] fn identities_cover_circuits_addresses_and_nodes()      // coverage(&identities(..)?, &generated) is Ok
#[test] fn a_generated_world_of_another_pair_fails_coverage()  // generated has extra leaf C3: error names "C3"
#[test] fn equal_worlds_took_effect()                          // compare(&g, &g)?.took_effect(), applied == 2 + 2 + 2
#[test] fn a_changed_leaf_rate_waits()                         // applied C1 download.ceil_kbps differs: waiting == [Waiting{Circuit,"C1",["download"]}]
#[test] fn a_missing_address_waits()                           // applied lacks 10.0.0.1/32: Waiting{Address,"10.0.0.1/32",["missing"]}
#[test] fn an_old_circuit_in_the_bridge_waits_for_removal()    // applied has leaf C9: waiting_removal == [(Circuit,"C9")]
#[test] fn a_changed_node_rate_waits()                         // applied n-a upload differs: Waiting{Node,"n-a",["upload"]}
#[test] fn an_unknown_bridge_field_waits()                     // applied C2 leaf has extra "shard": Waiting{Circuit,"C2",["shard"]}
#[test] fn an_unknown_op_tag_or_duplicate_identity_is_an_error() // {"op":"remove"} and two leaf C1: compare is Err
```

- [ ] **Step 2: Run the tests and make sure that they fail**

Run: `CARGO_BUILD_JOBS=1 cargo test --locked --lib apply_evidence::take_effect`
Expected: compile error, `take_effect` does not exist.

- [ ] **Step 3: Implement the module**

`compare` builds a `BTreeMap` from key to op for each kind on each side (key: `circuit_id`, `"{ip}/{prefix_len}"`, `node_id`). For a key on both sides, `fields` is the sorted list of top-level keys whose values differ, with keys present on one side only. `waiting` and `waiting_removal` are sorted by kind, then identity.

- [ ] **Step 4: Run the tests and make sure that they pass**

Run: `CARGO_BUILD_JOBS=1 cargo test --locked --lib apply_evidence::take_effect`
Expected: 9 passed.

- [ ] **Step 5: Commit**

```bash
git add src/apply_evidence/take_effect.rs src/apply_evidence.rs src/import_evidence.rs
git commit -m "feat(apply): add the per-identity take-effect check"
```

### Task 2: Verify with the take-effect check

**Files:**
- Modify: `src/apply_evidence/verify.rs` (`Prepared`, `prepare`, `verify_prepared`, `validate`, `verify_receipt`, `acknowledged`, tests)
- Delete: `src/apply_evidence/transaction.rs` (no caller is left after this task; move its duplicate-identity test idea to Task 1, already covered)
- Modify: `src/apply_evidence.rs` (remove `mod transaction;`, export `Proof`)

**Interfaces:**
- Consumes: `identities`, `coverage`, `compare`, `TakeEffect` from Task 1.
- Produces: `pub enum Proof { Verified(Verified), Waiting(TakeEffect) }`. `verify_prepared`, `verify_receipt` and `acknowledged` return `anyhow::Result<Proof>`. `Prepared` gets a field `identities: std::sync::Arc<take_effect::Identities>`, made in `prepare` from the same network and CSV bytes.

Changes in `verify_prepared` and `validate`:

1. Remove the gate `runtime["shaping_generation"] == first.generation` ("native generation still pending").
2. In `validate`, keep `generation == artifacts.inputs["shaping_generation"] == artifacts.generated["generation"]` (published files agree). Remove `generation == before.generation`.
3. Replace `transaction::matches(&artifacts.generated, &before.transaction)` with `coverage(&prepared.identities, &artifacts.generated)?` and then `compare(&artifacts.generated, &before.transaction)?`.
4. If the result `took_effect()`, return `Proof::Verified(..)` as today. Else return `Proof::Waiting(result)`. Do the `before == after` check before the compare.

- [ ] **Step 1: Update the tests in `verify.rs` first**

Change the existing fake-lqosd tests to expect `Proof`. Add:

```rust
#[tokio::test] async fn an_old_native_generation_with_the_current_world_is_verified()
// fake bridge generation "old", its transaction equal to generated: Proof::Verified
#[tokio::test] async fn a_different_native_world_is_waiting_with_its_identities()
// fake bridge transaction with one leaf rate changed: Proof::Waiting, waiting[0].identity == circuit id
#[tokio::test] async fn a_generated_world_of_another_pair_is_an_error()
// generated file with an extra leaf: Err, not Waiting
```

- [ ] **Step 2: Run them and make sure that they fail**

Run: `CARGO_BUILD_JOBS=1 cargo test --locked --lib apply_evidence::verify`
Expected: compile errors (`Proof` does not exist).

- [ ] **Step 3: Implement the changes above and delete `transaction.rs`**

- [ ] **Step 4: Run the module tests**

Run: `CARGO_BUILD_JOBS=1 cargo test --locked --lib apply_evidence`
Expected: all pass.

- [ ] **Step 5: Commit**

```bash
git add -A src/apply_evidence src/apply_evidence.rs
git commit -m "feat(apply): verify the native world per identity"
```

### Task 3: Monitor and late apply

**Files:**
- Modify: `src/apply_evidence/monitor.rs`
- Modify: `src/write_receipt.rs` (`accept`, and the `Domain::Apply` observations)
- Test: the existing tests of `monitor.rs` and `write_receipt.rs`

**Interfaces:**
- Consumes: `Proof` from Task 2.
- Produces: health component `Domain::Apply` / `"take_effect"`. `State::Updating` with reason `waiting_to_take_effect:<count>` while the receipt waits. `State::Healthy` with reason `took_effect` after `Applied`. Removed when no receipt is current.

Rules:

1. `Proof::Verified(p)`: `t.accept(&p, ..)` as today.
2. `Proof::Waiting(w)`: observe `take_effect` with the count of `w.waiting.len() + w.waiting_removal.len()`, then call `timeout(..)` as today.
3. Late apply (spec designer call): the monitor also checks a receipt in `Phase::VerifyTimeout` when it is the current receipt (`t.current()`, same `write_id`). `accept` allows `Phase::VerifyTimeout` in addition to `pending()`. Nothing else may go from `VerifyTimeout` to another phase.
4. A superseded or replaced receipt is never accepted (the existing `write_id` checks stay).

- [ ] **Step 1: Write the failing tests**

```rust
#[test] fn accept_moves_a_timed_out_current_receipt_to_applied()      // write_receipt.rs
#[test] fn accept_refuses_a_timed_out_receipt_of_another_write()      // write_receipt.rs: proof of write A, current receipt B: Err
#[tokio::test] async fn a_waiting_world_reports_its_count_and_keeps_the_receipt_pending() // monitor.rs: take_effect reason "waiting_to_take_effect:1", phase ImportConfirmed
#[tokio::test] async fn a_late_native_apply_recovers_a_timed_out_receipt() // monitor.rs: VerifyTimeout, then fake bridge equal: Applied
```

- [ ] **Step 2: Run them and make sure that they fail**

Run: `CARGO_BUILD_JOBS=1 cargo test --locked --lib apply_evidence::monitor write_receipt`
Expected: FAIL.

- [ ] **Step 3: Implement the rules above**

- [ ] **Step 4: Run them and make sure that they pass**

Run: `CARGO_BUILD_JOBS=1 cargo test --locked --lib apply_evidence::monitor write_receipt`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add src/apply_evidence/monitor.rs src/write_receipt.rs
git commit -m "feat(apply): report waiting identities and accept a late apply"
```

### Task 4: Live test acknowledgement and records

**Files:**
- Modify: `src/runner/live_test.rs` (`NativeIo::acknowledge`)
- Modify: `src/runner/live_test_tests.rs` (only if a fake asserts the old error text)
- Modify: `docs/operations/TOPOLOGY_LIVE_TEST.md` (step 4 and the message table)
- Modify: `brain/30-knowledge/topology-manager-status.md` (a short Phase 3 entry)

**Interfaces:**
- Consumes: `acknowledged` returns `Proof` (Task 2).
- Produces: `NativeIo::acknowledge` keeps its signature `anyhow::Result<()>`. `Proof::Waiting(w)` maps to `Err(anyhow!("waiting to take effect: {w}"))`. The retry loop and the report stay as they are, so the last error text in the report lists the waiting identities.

- [ ] **Step 1: Write the failing test** in `live_test.rs` tests or `live_test_tests.rs`:

```rust
#[test] fn a_waiting_proof_gives_the_waiting_identities_in_the_error()
// format the mapped error of Proof::Waiting with one Waiting{Circuit,"C1",["download"]}: text contains "waiting to take effect" and "C1"
```

Put the mapping in a small function `fn proof_result(proof: Proof) -> anyhow::Result<()>` so the test does not need a socket.

- [ ] **Step 2: Run it and make sure that it fails**

Run: `CARGO_BUILD_JOBS=1 cargo test --locked --lib runner::live_test`
Expected: FAIL.

- [ ] **Step 3: Implement `proof_result` and use it in `NativeIo::acknowledge`**

- [ ] **Step 4: Update the runbook and the status note**

In `TOPOLOGY_LIVE_TEST.md`, describe the acknowledgement as: "The bridge applied each identity of the pair." Replace the table row for `native acknowledgement unavailable` with a row for `waiting to take effect: ...`: LibreQoS imported the pair, but the bridge has not applied these identities. The test restores the legacy pair. Read `/run/libreqos/seam_state.json` (see the dataplane settings document).

- [ ] **Step 5: Run the full checks**

Run each, one at a time:

```bash
CARGO_BUILD_JOBS=1 nice -n 19 cargo test --locked
cargo fmt --check
CARGO_BUILD_JOBS=1 nice -n 19 cargo clippy --locked --all-targets -- -D warnings
git diff --check
```

Expected: all tests pass, no format change, no Clippy warning, no whitespace error.

- [ ] **Step 6: Commit**

```bash
git add src/runner docs/operations/TOPOLOGY_LIVE_TEST.md brain/30-knowledge/topology-manager-status.md
git commit -m "feat(topology): acknowledge the live test per identity"
```
