---
type: session
created: 2026-10-30
tags: [lqosync-v3, core-control, health, preview]
status: verified-preview
---

# Core console preview

Updated: 2026-10-30T23:08:51+08:00 (Asia/Shanghai).
Branch: `codex/core-console`. Worktree: `/tmp/lqosync-core-console`. Base: `b51af71`.
Integration target: `lqosyncv3` in `/opt/LQoSync3.0`.

## Result

Added scheduled health and reconciliation previews with modal policy and probe controls.
The page separates observed counts, desired counts, health reasons, and blocked decisions.
Protected address ranges contribute to observed coverage. Preview decisions cannot authorize writes.
Automatic mode rejects in both settings validation and the HTTP action.
Legacy settings keep the preview disabled. Explicit Preview mode and monitor selections start collection.

Network collection remains outside the App lock, with at most four concurrent reads.
Health ticks do not accelerate normal publication. Token and settings-revision checks protect saves.
Preview computation uses immutable snapshots and the blocking pool.
Changed settings, read errors, read epochs, or inventory identities discard queued results.
Pause clears runtime evidence. A caller cancellation cannot leave the preview worker busy.
Snapshots omit prior decision plans to avoid redundant large copies.

Source failures preserve available health diagnostics and discard the plan and cleanup receipts.
Repeated probe counters cannot establish fresh health. Restart requires new completed rounds.
Actual membership validation continues to guard ordinary local publication.

## Verification

Focused console checks: eight passed. Scheduler checks cover publication suppression and optional Netwatch failure.
The initial full suite passed 286 tests, with zero failures and 12 ignored tests.
A later cancellation regression failed before the fix and passed afterward.
Final full-suite result: 287 passed, zero failures, 12 ignored.
Format, strict Clippy, JavaScript syntax, and diff checks passed.
An isolated Chromium check passed at five widths: fonts, columns, bounds, dialog layer, focus, Escape, retained drafts, and automatic rejection.
The Playwright module was unavailable. The browser check used the installed Chromium through its debugging protocol.
The documentation heuristic ran. Technical-name and list warnings require prose review, not a compliance claim.
Fresh read-only review found no remaining material defect after the cadence and cancellation fixes.

## Boundary and next action

Task 4 remains incomplete. The console has no mutation worker, live verifier, journal controls, or last-operation report.
The exclusive offline writer lease remains unchanged. Console/writer lease coordination still needs implementation and tests.
Isolated RouterOS acceptance and explicit production scope remain required before live activation.
No live router reads or writes, production settings, shaping outputs, credentials, services, or restart changed.
The browser used isolated synthetic data on a separate port and stopped after checks.

See `v3/CORE_CONTROL.md`, the core console ledger, and [[2026-10-30-core-writer]].
