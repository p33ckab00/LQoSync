# Durable legacy ownership and source priorities

## Goal

Preserve explicit legacy IDs and labels after restart. Keep endpoint data outside the registry.
Preserve explicitly owned static inputs from their current authoritative source.
Convert source speed priorities without an implicit fallback or a change to subscriber rates.

## Boundaries

Reuse registry validation, atomic commits, forward recovery, fresh source authorization, and compiled admission.
Keep runtime activation, deployment, schedules, and the TUI outside this slice.
Do not infer identity from names, IP addresses, MAC addresses, or previous output files.

## Acceptance

- [x] Persist strict export ownership with a compatibility adapter and exact pre-upgrade backup.
- [x] Use stable identity references for device aliases. Reject ambiguous current sessions and namespace collisions.
- [x] Preserve explicit static ownership from current input. Block missing or conflicting input.
- [x] Preserve source order and explicit defaults. Read dynamic values only from current evidence.
- [x] Prove restart, reconnect, empty-source removal, conflicts, and legacy policy parity.
- [x] Run focused and full tests, format, Clippy, and whitespace checks. Capture the handoff.

## Recovery

Old schema files remain unchanged on open. The first changed commit saves an exact backup.
Recovery creates a forward revision. Current identities, counters, and ownership reservations remain intact.
An old executable must reject the new schema. Do not run mixed writers.

## Results

Added 19 tests. Full suite: 1,050 passed, zero failed, 9 ignored. All 328 topology tests pass.
Format, Clippy with denied warnings, and whitespace checks pass.
The final test-fixture cleanup passed all 10 ownership tests after the full run.

The initial ownership tests reproduced lost aliases and missing static-input acceptance.
The initial source-priority tests reproduced absent conversion and ambiguous active-session acceptance.
A later regression reproduced acceptance of a protected static row without a parent. The ownership boundary now rejects it.
Schema assertions and backup paths now use schema 4. The schema-3 test compares exact stored bytes, including the writer newline.
Clippy identified two fixture-only style issues. Both are corrected.

Review found an unnecessary service-index copy per selection. The final path borrows shared indexes for each router.
Ownership reservation checks also use hash indexes. The full synthetic topology tests pass in the normal suite.
No production throughput benchmark or independent review occurred.

Static input supports independent roots. Shared branches need explicit capacity ownership before activation.
Address-based DHCP overrides require independent identity mappings. Complete configuration migration still needs shadow parity.
Runtime compiler/configuration binding, one writer owner, native planner bounds, and live acceptance remain gated.
