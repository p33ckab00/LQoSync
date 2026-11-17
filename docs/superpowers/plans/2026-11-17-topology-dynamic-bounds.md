# Plan for Dynamic Subscriber Bounds

Execute inline with test-driven development and verification-before-completion.

**Goal:** Preserve legacy minimum/maximum rules for source-read subscriber profiles, including offline intent.

**Architecture:** Add an explicit imported rate rule with the legacy rounded fraction and floor. Import all four controls through a pure helper. Require structural equivalence during configured adoption. Preserve the current exact Mbps and percentage rules.

**Spec:** `docs/design/TOPOLOGY_MANAGER_IMPLEMENTATION.md`.

## Constraints

- Preserve current unfinished DHCP overrides and unrelated operational edits.
- Use existing dependencies and the existing legacy clamp function.
- Keep imported compatibility rules explicit. Do not replace new policy defaults.
- Represent factors exactly at six decimal places. Reject unsupported values instead of truncation.
- Preserve source deadlines, independent subscriber identity, WAN ceilings, and subscriber policy boundaries.
- Do not activate the service, deploy, or change native group minimums.

## Task 1: Import and resolve controls

Files: `src/topology_manager/rates.rs`, new `legacy_bounds.rs`, module exports, and tests.

- [x] Add tests that fail for dynamic source-read profiles with imported controls.
- [x] Add `RateRule::LegacyRoundedBase { factor, floor_bps }` with positive floors and exact factors in `(0, 1]`.
- [x] Resolve imported controls through the existing legacy rounding function with exact input/output conversions.
- [x] Add `legacy_bounds::import` to produce four explicit profile overrides from configured bandwidth defaults.
- [x] Compare low rates, floor transitions, asymmetric rates, rounding ties, and upper conversion limits with the legacy function.
- [x] Prove current exact percentage rules retain their previous behavior.

## Task 2: Structural compatibility and fresh evidence

Files: `src/topology_manager/preflight.rs`, preflight tests, and coordinator tests.

- [x] Replace unconditional dynamic-bound rejection with a comparison of all four effective controls.
- [x] Require matching factors and floors even when no subscriber is online.
- [x] Reject changed defaults, altered profile overrides, invalid precision, and unresolved source priority.
- [x] Prove reconnect and source-speed changes calculate new subscriber minimums and maximums from fresh data.
- [x] Preserve independent group-factor and native-minimum blockers.

## Task 3: Verify and capture

- [x] Run focused and full tests, format, Clippy, and whitespace checks.
- [x] Run the installed-library sync test in temporary directories.
- [x] Review the numerical boundary and every rate-rule caller.
- [x] Update the design, discussion, session, Home, and handoff.
- [x] Leave changes local. Keep service activation, TUI, and schedules outside this task.


## Results

The full suite passes: 1,127 tests, zero failures, and 12 ignored tests.
Nine new active tests cover this task. All 392 topology tests pass.
The installed-library sync test passes in temporary directories.
Format, Clippy with warnings denied, and whitespace checks pass.
The author reviewed the numerical conversions, rule callers, inheritance, and compatibility checks.
No independent review or production benchmark occurred.
Proof: `.superpowers/sdd/2026-11-17-topology-dynamic-bounds/`.
Final logs: `full-complete.log`, `clippy-complete.log`, and `native-sync.log`.
The graph remains the earlier baseline. Current files take precedence.
