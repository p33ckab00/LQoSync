# DHCP Identity Overrides Implementation Plan

Execute this plan inline with test-driven development and verification-before-completion.

**Goal:** Apply DHCP speed overrides through explicit independent subscriber references.

**Architecture:** Extend source policy with durable bindings from legacy list positions to verified account references and exact rates. Never select subscribers through MAC, IP, hostname, or lease locators. Validate all bindings before collection. Bind prepared output to the current effective rates.

**Tech Stack:** Existing Rust, Serde, Tokio, and registry storage. No new dependency.

**Spec:** `docs/design/TOPOLOGY_MANAGER_IMPLEMENTATION.md`.

## Constraints

- Preserve the legacy collector and current configuration format.
- Require an enrolled identity in the selected DHCP scope, also when subscribers are offline.
- Reject duplicate positions, references, unknown identities, changed rates, and missing coverage.
- Require source-read base authority so local profiles cannot hide an override.
- Preserve the source deadline, endpoint freshness, and single-writer checks.
- Keep service activation, deployment, TUI, schedules, and other migration gaps outside this task.

## Task 1: Durable bindings and source checks

Files: `source_policy.rs`, `model.rs`, `settings.rs`, `preflight.rs`, and focused tests under `src/topology_manager/`.

- [x] Add failing tests for complete explicit bindings and invalid bindings.
- [x] Add `DhcpOverrideBinding { legacy_client_index, account_ref, rate }` to source policy.
- [x] Default an absent binding list to empty. Omit empty lists from serialized output.
- [x] Validate protocol, uniqueness, positive rates, enrolled identities, exact configured rate coverage, and effective base authority.
- [x] Index bindings and enrolled identities once for each check.

## Task 2: Fresh authorization and publication

Files: `identity.rs`, `source_policy.rs`, `preflight.rs`, and coordinator tests.

- [x] Add failing reconnect tests with changed IP/MAC and a second subscriber that receives the old MAC.
- [x] Resolve overrides only after independent reference authorization.
- [x] Apply override rates before fallback source parsing, as the legacy collector does.
- [x] Extend the transient source-policy digest to include effective configured DHCP override rates.
- [x] Reject stale prepared output after changes or removal of override rates.
- [x] Prove offline removal and fresh reconnect behavior through the configured coordinator.

## Task 3: Check and capture

- [x] Run focused tests, the full suite, format, Clippy, and whitespace checks.
- [x] Review callers, compatibility, policy integrity, and allocation behavior.
- [x] Update design, discussion, session, Home, and handoff with results and remaining gates.
- [x] Leave changes local for review. Do not deploy or change the live service.


## Results

The full suite passes: 1,118 tests, zero failures, and 12 ignored tests.
Ten new active tests cover this task. All 383 topology tests pass in the full suite.
The installed-library sync test passes in temporary directories.
Format, Clippy with warnings denied, and whitespace checks pass.
An initial full run found an outdated error-message assertion. The corrected assertion passes in the final full run.
The author reviewed the changes. No independent review or production benchmark occurred.
Proof: `.superpowers/sdd/2026-11-17-topology-dhcp-overrides/`.
Final logs: `full-complete.log`, `clippy-complete.log`, and `native-sync.log`.
The source graph remains the earlier baseline. Current source files take precedence.
