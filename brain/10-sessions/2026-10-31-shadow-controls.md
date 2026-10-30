---
type: session
created: 2026-10-31
tags: [lqosync-v3, core-control, recovery]
status: active
related:
  - "[[10-sessions/2026-10-31-core-worker]]"
---

# Shadow pause and recovery inspection

The operator authorized code and mock tests. No lab router is available.
No live router access, service restart, production configuration, or shaping change occurred.

## Changes

- `core_coordination` serializes shadow work per canonical root with a process lock.
- Local pause and settings changes invalidate tickets. Shadow tickets cannot authorize mutations.
- `core_console` checks persisted settings before and after computation. An external persisted pause blocks stale results.
- `core_recovery` reads bounded, validated saved journals without an executor or router client.
- `core_recovery_web` separates pending intent, known replies, and historical saved verification.
- `core-recovery-view` serves GET-only reports on loopback when the normal console remains blocked by a recovery marker.
- Approval, resume, rollback, and marker removal have no HTTP action. Automatic mode remains rejected.

The writer retains its exclusive offline fence. Pure journal validation moved to the journal type without behavior changes.
The separate viewer does not create App, load credentials, or modify saved records.
Inspection allows 16 journals, 128 directory entries, and 64 MiB per report. Concurrent asynchronous inspection is bounded to one per process.
A failed text read consumes the remaining byte budget. A limited report remains incomplete.

## Review and proof

The fresh reviewer found two defects: persisted pause drift and corrupt UTF-8 budget accounting.
Both regression tests failed before the fixes and passed afterward.
Focused checks passed: nine console tests, six recovery tests, six shadow tests, and 21 writer tests.
Chromium passed at widths 1440, 1100, 720, 390, and 320 pixels.
The viewer had 14px body text, no horizontal overflow, no forms, and a disabled approval button.
POST requests to mutation paths rejected. The fixture used synthetic local records.

Final checks passed: format, 308 tests, zero failures, 12 ignored, and strict all-target Clippy.
Documentation links and diff checks passed.
See [the operator guide](../../v3/SHADOW_CONTROLS.md) and [the execution ledger](../../docs/superpowers/plans/2026-10-31-shadow-controls.md).

## Remaining work

Online mutation grants, console coordination for actual sends, and recovery approval remain incomplete.
Isolated RouterOS acceptance and a separately reviewed production scope remain necessary.
Mocks cannot prove uninterrupted traffic, actual probe fields, or RouterOS connection behavior.
The existing project graph predates this stage. Targeted graph recall informed the implementation. No broad graph rebuild occurred.
