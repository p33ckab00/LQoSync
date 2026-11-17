# Plan for legacy policy preflight

**Goal:** Detect legacy policies that the configured manager path cannot preserve before adoption or source collection.

**Architecture:** Add a typed, read-only compatibility report. Reuse exact rate conversion, source settings, and snapshot adoption checks. Reject unsupported source filters and address-based overrides before configured sync reads.

**Spec:** `docs/design/TOPOLOGY_MANAGER_IMPLEMENTATION.md` and `brain/20-decisions/ADR-0009-dhcp-discovery-and-exclusions.md`.

## Boundaries

Do not infer durable identity from IP or MAC. Do not invent exclusion or override migration rules.
Keep intentional manager policy changes separate from claims of exact legacy compatibility.
A clear report is not service activation or native acceptance.
Do not deploy, change router configuration, or activate the manager service loop.

## Task 1: Source-policy guard


- [x] Add tests that reproduce ignored selected-source exclusions and DHCP client overrides.

- [x] Add typed issues for global/router exclusions, unsupported DHCP membership, and client overrides.

- [x] Check selected scopes only. Ignore unrelated protocol settings.

- [x] Call the guard before coordinator connection attempts. Prove that conflicting configuration cannot publish.

## Task 2: Configured legacy adoption


- [x] Test base-rate authority and subscriber bounds. Test group factors, caps, and minimums.

- [x] Compare every selected policy, also when it has no active subscriber.

- [x] Add `ownership::adopt_configured_legacy` with the config preflight before exact snapshot adoption.

- [x] Keep the low-level snapshot adapter available for isolated diagnostics.

- [x] Require explicit review when the model cannot represent a rule or the checks cannot prove it. Do not claim full migration support.

- [x] Test stale evidence and snapshot mismatches after configuration checks pass.

## Task 3: Verification and capture


- [x] Run focused tests, the full suite, format, Clippy, and whitespace checks.

- [x] Run the installed-library source-sync test in temporary directories.

- [x] Update design, discussion, session, Home, and handoff.

## Stop condition

Configured sync cannot silently ignore known unsupported source policies.
Configured adoption checks current and dormant legacy settings before it can create aliases.
Automatic live activation remains gated by complete policy migration, static-input authority, enrollment, and native proof.

## Results

- Full suite: 1,096 passed, zero failed, and 12 ignored.

- Added 12 active tests. All 11 preflight tests and 14 coordinator tests pass.

- The optional installed-library source-sync test passes in temporary directories. It rejects an excluded source before pair publication.

- Format, Clippy with warnings denied, and whitespace checks pass.

- Indexed profile assignments avoid a subscriber scan for each source instance. No production benchmark occurred.

- DHCP fixtures now declare bound-only membership without unassigned leases. The legacy default can produce a compatibility conflict.

- Dynamic profile bounds, native node minimums, unsupported source rules, and fixed group capacities remain explicit issues.

- Factor comparison uses integer products. Configured values must convert exactly at six decimal places for this first checker.

- Author review only. No independent reviewer, live source read, service action, native apply, deployment, or dependency change occurred.

## Next action

Implement exclusion filters against current source evidence with parity tests.
Keep MAC-address overrides blocked until they have explicit independent identity mappings.
Then extend dynamic subscriber bounds and native minimum commitments without silent rate changes.
Static-input authority, shared static branches, enrollment, native planner bounds, and live acceptance remain activation gates.
