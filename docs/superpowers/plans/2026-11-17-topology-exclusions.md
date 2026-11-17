# Plan for current-source exclusion filters

**Goal:** Apply current IP, PPP profile, and DHCP server exclusions to fresh manager observations.

**Architecture:** Reuse the legacy matcher. Filter validated current sessions before account groups form. Bind prepared output to the effective exclusion policy.

**Spec:** `docs/design/TOPOLOGY_MANAGER_IMPLEMENTATION.md` and `brain/20-decisions/ADR-0009-dhcp-discovery-and-exclusions.md`.

## Boundaries

Keep account/profile/server inventory and all current-row validation.
Use IPv4/CIDR rules and case-insensitive name wildcards with the current legacy semantics.
Apply global and router-specific lists together. Do not change unrelated protocols.
Do not use endpoints as durable identity.

Do not migrate MAC-address speed overrides.
Do not activate the live service loop, deploy, or change router configuration.

## Task 1: Fresh session filters


- [x] Add failing tests for IP, profile, server, wildcard, IPv6, and multiple-session behavior.

- [x] Reuse the current Excluder with a parsed IPv4 lookup.

- [x] Add configured normalization. Validate every row before exclusion, then group only the retained sessions.

- [x] Preserve active comments and service membership for retained sessions only.

- [x] Test malformed/duplicate excluded rows and late evidence. They must still block acceptance.

- [x] Prove parity against the legacy collector on complete current IPv4 fixtures.

## Task 2: Runtime and adoption


- [x] Allow supported exclusions through source preflight. Reject malformed filters.

- [x] Connect configured normalization to the source coordinator.

- [x] Test exclusion, removal, and fresh reconnect with stable authorized identity.

- [x] Bind prepared pairs to the effective filter policy. Reject missing or stale bindings when filters apply.

- [x] Enforce the binding in configured publication and configured legacy adoption.

- [x] Extend the native scratch test to check filtered output and rejection after settings change.

## Task 3: Proof and capture


- [x] Run focused tests, the full suite, format, Clippy, and whitespace checks.

- [x] Run the installed-library source-sync test in temporary directories.

- [x] Update design, discussion, session, Home, and handoff.

## Stop condition

Supported exclusions work automatically on fresh observations and preserve validation and identity rules.
Source rows remain transient. DHCP overrides, dynamic bounds, native node minimums, and activation gates remain separate tasks.

## Results

- Full suite: 1,108 passed, zero failed, and 12 ignored. Added 12 active tests.

- All 373 topology tests pass. The topology subset has one optional ignored test.

- Seven new normalization tests prove filter membership, retained-session metadata, row validation, duplicate detection, expiry, and legacy parity.

- Coordinator and runner tests prove removal, current reconnect, missing/stale policy proof, and configured alias adoption.

- The installed-library public sync test passes in temporary directories with an excluded batch and later active batches.

- Format, Clippy with warnings denied, and whitespace checks pass.

- Shared global lists, indexed name matches, and parsed IPv4 lookup limit repeated work. No production benchmark occurred.

- Author review only. No independent reviewer, live RouterOS read, service action, native apply, deployment, dependency change, or schema change occurred.

## Next action

Add explicit identity mappings for DHCP speed overrides. Do not infer continuity from a current or stored MAC address.
Dynamic bounds, native group minimums, static-input authority, shared branches, enrollment, native planner bounds, and live acceptance remain gates.
