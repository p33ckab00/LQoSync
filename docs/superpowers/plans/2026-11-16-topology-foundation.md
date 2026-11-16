# Topology Foundation Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task by task. Do not delegate unless the owner requests delegation.

**Goal:** Build and test the pure rate and hierarchy foundation without changes to live exports.

**Architecture:** Add `topology_manager` beside the legacy builder. Keep calculations pure and deterministic. Reuse the current writer and verifier in later integration work.

**Tech Stack:** Rust 2024, standard library, current serde dependencies, and the current test harness.

**Spec:** `docs/design/TOPOLOGY_MANAGER_IMPLEMENTATION.md` and `docs/design/TOPOLOGY_MANAGER_DISCUSSION.md`.

## Global Constraints

- Use ASD-STE100 guidance for prose, comments, errors, and UI labels. Preserve exact identifiers and commands.
- New-policy maxima: Percentage at 100 percent, independently for download and upload.
- New-policy minima: Fixed Mbps at 1 Mbps, independently for download and upload.
- Preserve current effective values during migration.
- Only an immediate subscriber group uses subscriber-based automatic calculation.
- WAN capacity is explicit. Intermediate capacity is from its parent, fixed Mbps, or a WAN percentage.
- Do not mix direct subscribers and group children in the first version.
- Keep subscriber maxima independent of aggregate ceilings.
- Do not change production files, services, or the topology source selector in this slice.
- Do not add dependencies for pure model and rate calculations.
- Preserve unrelated work. Do not commit or push without authorization.

## Review Focus

- Overflow and integer rate conversion must not exceed a WAN ceiling. Task 1 owns these tests.
- A min/max override replaces the inherited control instead of multiplying it. Task 1 owns these tests.
- A WAN with direct subscribers stays manual. Task 2 owns this test.
- Duplicate labels must not merge identities or cause silent promotion. Task 2 owns these tests.
- Conversion of legacy formulas must preserve fixture results. Task 3 owns these tests.

## Types and file boundaries

Create `src/topology_manager/mod.rs` when Task 1 starts. Export it from `src/lib.rs` without a live-cycle caller.
Create `rates.rs` for these numeric types:

```rust
pub struct RateBps { pub down: u64, pub up: u64 }
pub struct Fraction { pub numerator: u64, pub denominator: u64 }
pub enum RateRule { FixedBps(u64), BaseFraction(Fraction) }
pub enum RuleValue { Inherit, Override(RateRule) }
pub struct SpeedPolicy {
    pub down_min: RateRule,
    pub up_min: RateRule,
    pub down_max: RateRule,
    pub up_max: RateRule,
}
pub struct SpeedOverrides {
    pub down_min: RuleValue,
    pub up_min: RuleValue,
    pub down_max: RuleValue,
    pub up_max: RuleValue,
}
pub struct SubscriberRates { pub min: RateBps, pub max: RateBps }
pub enum AggregateRule {
    Inherit,
    Fixed(RateBps),
    WanFraction { down: Fraction, up: Fraction },
    Automatic { down: Fraction, up: Fraction },
}
pub struct AggregateRates { pub local: RateBps, pub ceiling: RateBps }
```

Create `layout.rs` for these hierarchy types:

```rust
pub enum NodeKind { Wan, Intermediate, SubscriberGroup }
pub struct PolicyNode {
    pub id: String,
    pub name: String,
    pub parent: Option<String>,
    pub kind: NodeKind,
    pub capacity: AggregateRule,
    pub direct_subscribers: usize,
}
```

`direct_subscribers` is evaluation input, not a persistent session cache.
Derive the serde and comparison traits required by tests. Use the current `anyhow::Result` convention.
Persistent policy and registry schemas belong to the next slice.

## Task 1: Resolve rates

**Files:** create `src/topology_manager/mod.rs` and `rates.rs`. Change `src/lib.rs`.

**Produces:**

- `resolve_subscriber(base: RateBps, defaults: &SpeedPolicy, overrides: &SpeedOverrides) -> anyhow::Result<SubscriberRates>`.
- `resolve_aggregate(rule: &AggregateRule, direct_basis: RateBps, wan: Option<RateBps>, parent: Option<RateBps>) -> anyhow::Result<AggregateRates>`.
- `new_policy_defaults() -> SpeedPolicy`.

- [ ] Write tests for the approved defaults and independent overrides.
- [ ] Assert that a 50,000,000 bps base resolves to minimum 1,000,000 and maximum 50,000,000 under new defaults.
- [ ] Assert that a minimum fraction of 1/100 resolves to 500,000 bps from the same base.
- [ ] Assert that a fixed maximum override replaces the global percentage rule.
- [ ] Assert local 400,000,000 and ceiling 310,000,000 for a 500 Mbps basis, factor 4/5, and 310 Mbps WAN.
- [ ] Assert that half a 310 Mbps WAN targets 155 Mbps and becomes 100 Mbps under a lower parent.
- [ ] Assert errors for zero denominator, overflow, unsupported zero rates, and minimum above maximum.
- [ ] Assert that fractional calculations never round a ceiling upward above its bound.
- [ ] Run `cargo test --lib topology_manager::rates`. Make sure the tests fail before implementation.
- [ ] Implement checked arithmetic with wider intermediates and one explicit rounding boundary.
- [ ] Run the same command. Require all tests to pass.

Backend export bounds are adapter checks. Do not invent new native rate limits in these arithmetic functions.

## Task 2: Validate hierarchy

**Files:** create `src/topology_manager/layout.rs`. Change `mod.rs`.

**Consumes:** `AggregateRule` from Task 1.

**Produces:** `validate_layout(nodes: &[PolicyNode]) -> anyhow::Result<()>`.

- [ ] Write tests for multiple top-level WANs and a profile-only tree.
- [ ] Assert errors for duplicate/empty IDs, missing parents, self-parent relationships, and cycles.
- [ ] Assert that distinct IDs with equal display names remain distinct model objects.
- [ ] Assert that a WAN with direct subscribers remains valid with fixed capacity and cannot use Automatic mode.
- [ ] Assert rejection of mixed direct subscribers and group children.
- [ ] Assert that intermediate capacity uses only inherit, fixed, or WAN-percentage mode.
- [ ] Assert that WAN-percentage mode requires a unique WAN ancestor.
- [ ] Run `cargo test --lib topology_manager::layout`. Make sure the tests fail before implementation.
- [ ] Implement ID indexes and bounded graph validation. Do not silently promote missing-parent nodes.
- [ ] Run the same command. Require all tests to pass.

Unique export names remain an adapter responsibility. Display names must not become identity keys.

## Task 3: Prove calculation compatibility

**Files:** create `tests/topology_foundation.rs`. Change new modules only if defects appear.

**Consumes:** Tasks 1 and 2, plus the current fixture calculation inputs.

**Produces:** isolated equivalence evidence, not an active manager path.

- [ ] Add fixture tests for PPP plan sums and DHCP/Hotspot count-times-unit bases.
- [ ] Compare rendered rate values with current `speed.rs` behavior and group fixtures.
- [ ] Treat precision or backend-bound mismatches as explicit failures, not permission to change legacy expectations.
- [ ] Run `cargo test --test topology_foundation`. Make sure the tests fail before the required conversion helpers exist.
- [ ] Add only the conversion helpers required by those fixtures. Leave the live `cycle::assemble` path unchanged.
- [ ] Run `cargo test --test topology_foundation` and `cargo test --lib topology_manager`.
- [ ] Run `cargo fmt --check` and `cargo clippy --all-targets -- -D warnings` after the new code passes.
- [ ] Repeat the focused regression suites in the review report. Explain any changed legacy expectation.

## Later feature slices

Each slice needs a concrete plan after this foundation:

1. Durable catalog/registry, expected-revision private store, migration identities, and fresh source observations.
2. Eligibility, stable assignments, pins, balancing, export names, and shadow pair parity.
3. Publication through the current tracker, interruption tests, and native evidence correlation.
4. Topology TUI views, completed-edit detection, and migration of duplicate controls from general Settings.
5. Timezone-aware schedules for subscriber policies and WAN/group capacities after overlaps, overnight days, and DST behavior are settled.

Do not implement CRM features or use stored CSV rows as the future registry.
Do not weaken native verification for faster completion.
Slice 2 must prove that multiple device rows contribute once to subscriber demand and CIR totals.
Serialized CSV/JSON precision and actual backend rate bounds need adapter tests beyond the pure integer-rate tests in this foundation.

## Stop conditions

The foundation is complete when its tests pass and no production path calls the new module.
It does not complete the Topology Manager feature.
Record checks, changed files, and remaining contract gates in the project vault.
This review creates the plan only. No unchecked implementation step above has executed.
