# Source policy mapping and exact rates implementation plan

> **For agentic workers:** Use superpowers:executing-plans inline. Do not delegate unless the owner requests delegation.

**Goal:** Map current source profiles to explicit stable references and supply exact integer rate evidence to the existing policy pipeline.

**Architecture:** Extend the isolated source selection API with explicit profile bindings and rate read modes. Compile mappings against current profile inventory once per router. Reuse identity authorization, observation, resolver, and aggregate stages.

**Tech Stack:** Current Rust, config rate-order/unit enums, integer rates, and anyhow. Add no dependency or durable schema.

**Spec:** `docs/design/TOPOLOGY_MANAGER_IMPLEMENTATION.md`, discussion details 2/3, and the identity plan.

## Global Constraints

- Preserve unrelated work and local normalization/identity slices. Do not commit, push, deploy, or write routers.
- Profile names are explicit selection rules, never subscriber identity proof.
- Current row locators cannot become stable profile references through an implicit fallback.
- Bind each selected current profile name to one stable source reference. Reject blank and ambiguous bindings.
- Keep current service selection and source-scope checks. Bindings for selected catalog profile filters must exist.
- Rate reads have explicit Disabled or RateLimit modes per mapped profile and for unprofiled sources.
- Use exact integer bps. Reject fractional bps, zero, malformed values, and overflow. Do not round, clamp, or invent a default.
- Preserve configured Rx/Tx or download/upload order and explicit unitless units. Reuse the current config enums.
- Read the first sustained rate token. A single value is symmetric. Burst/minimum fields do not replace subscriber policy controls.
- Missing rate text stays absent. Later SourceRead authority must conflict when its required numeric evidence is missing.
- Keep current deadlines/generations and independent identity authorization. No stored endpoint or speed replay.
- Source policy settings remain isolated API inputs until durable configuration/UI migration exists.

## Review Focus

- Recreated profile locators with the same explicit selector must keep the stable reference and use fresh rates.
- A profile rename without a revised selector must not inherit the old mapping by row locator.
- Invalid raw rates for Disabled mode must not override local policy or create numeric evidence.
- Mixed local/source profiles in one scope must use their own explicit rate read modes.
- Shared profile rates must compile once rather than parse for every subscriber.
- SourceRead numeric evidence must reach placement/aggregate evaluation without changing subscriber maxima through WAN clipping.

## Task 1: Exact sustained rate parser

**Files:** add `src/topology_manager/source_rate.rs` and `source_rate_tests.rs`.

- [x] Write failing tests for direction, units, symmetry, decimal exactness, overflow, malformed values, absence, and large integers.
- [x] Implement integer decimal parsing with checked arithmetic. Leave legacy `speed.rs` unchanged.
- [x] Keep error text free of raw input. Document the sustained-only grammar and native/export limits.

## Task 2: Current profile mapping and authorization integration

**Files:** add `source_policy.rs` and tests. Extend `identity.rs`, its fixtures, public catalog integration, and module exports.

- [x] Write failing tests for explicit mapping, recreated locators, rename, ambiguity, read modes, missing rates, and mixed profiles.
- [x] Index current profiles once per router. Compile each selected source policy against that index.
- [x] Require bindings for selected catalog profile filters. Preserve empty/offline reads without invented membership.
- [x] Carry mapped profile and exact plan observations through identity authorization.
- [x] Add typed active-source conflicts for missing mappings and invalid rates. Withhold the collection on conflicts.
- [x] Prove SourceRead rates reach the existing placement/capacity pipeline and retain current identity/deadline guards.

## Task 3: Check and capture

- [x] Run focused/full tests, format, Clippy with denied warnings, and whitespace checks.
- [x] Review mapping ownership, native limits, source rate absence, freshness, and indexed costs.
- [x] Capture results and remaining durable configuration/migration/publication/UI work in design and brain notes.

## Execution Rulings

Worktree: `/home/pi/LQoSync3.0`, branch `lqosync3`, base `489acc7` plus prior local slices.
The owner directs the next source-scope/rate integration task.
The scope is a working isolated input-to-placement path. It does not enable manager mode.
All mode and mapping choices are explicit API input, with no automatic import from legacy settings.

MikroTik documents Rx as client upload, Tx as download, and a symmetric Tx default when the second rate is absent.
Reference: [PPP AAA](https://help.mikrotik.com/docs/spaces/ROS/pages/132350049/PPP%2BAAA) and [DHCP](https://help.mikrotik.com/docs/spaces/ROS/pages/24805500/DHCP).
The parser also retains the legacy k/M/G and lowercase compatibility for input. It does not write RouterOS syntax.
Native rate bounds, export precision, admission, and DPDK proof remain later gates.

Stop when exact source observations reach the existing pipeline and all required checks pass. Do not add another collector or writer.

## Review correction — Service selectors

The service regression reproduced empty source membership when a configured DHCP server name was compared with its ephemeral row locator.
SourceSelection now uses an explicit `service_names` field. The API has no live callers or serialized settings to migrate.
Names resolve through fresh DHCP/Hotspot server inventory before membership checks. Unavailable selectors fail instead of asserting an empty source.
PPP selectors use documented transport names. Empty name lists still select the whole router/protocol.

Hotspot reads now include `/ip/hotspot` inventory before/after active tables, with no password fields.
Normalization checks active Hotspot server membership and exposes current service metadata under the original deadline.
The two extra Hotspot inventory commands preserve the existing timeout/age guards. They do not alter the legacy reader.
Tests reproduce missing/disabled Hotspot service membership and verify recreated service locators with the same explicit selector.

## Local results

Initial source parser/policy red run: 16 failed and 1 passed. All 17 parser/policy checks now pass.
The authorization integration regression failed before mapped membership/rates were connected.
The DHCP service-name regression reproduced empty membership from an ephemeral locator comparison.
The Hotspot inventory and missing/disabled-server regressions failed before the service fix.

Added 23 checks in this slice. All 256 topology checks pass.
Full suite: 973 passed, zero failed, 9 ignored. Format and Clippy with denied warnings passed.
Whitespace and stub checks passed. No dependency, durable schema, legacy parser/reader, live caller, or configuration changed.

The public restart/store test now runs both explicit Local and SourceRead profile authority.
It uses a fresh profile locator with a separate stable binding and preserves the 50/10 Mbps subscriber maximum.
Missing SourceRead rate text blocks placement without a fallback. The first negative assertion incorrectly entered aggregate evaluation after that block.
The corrected assertion checks the blocked resolver result. The final focused and full runs pass.

Service-name selectors resolve recreated DHCP/Hotspot locators from current inventory.
The preserved 10,000-account/10,000-service test confirms order-independent IDs/output. The 2,000-value decimal test confirms exact bps results.
These synthetic checks do not establish production throughput.

This slice remains local and uncommitted. Next: durable source/authority settings and migration/recovery, then publication/native integration and TUI.
Legacy comment/name/default source priorities need an explicit migration path. No implicit import or manager-mode activation exists.
