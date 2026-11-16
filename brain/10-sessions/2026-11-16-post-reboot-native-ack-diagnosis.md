---
type: session
created: 2026-11-16
tags: [unified-health, native-ack, diagnosis, verification]
status: diagnosis-complete
related: ["[[2026-11-16-explicit-slot-mapping]]", "[[HANDOFF]]"]
---

# Post-reboot native ACK diagnosis

Completed: 2026-11-16T10:20:32+08:00 (Asia/Shanghai). Agent: Codex `/root`.
Branch/worktree: `lqosync3`, `/home/pi/LQoSync3.0`.
The owner authorized Task 2 and performance, integrity, and reliability review.

## Evidence and conclusion

The operator completed the root-only read-only probe.
Three stable samples show 28 generated leaf and mapping operations versus 27 native operations.
The pair files match the import. The runtime is ready and agrees with the generated generation.
The native generation differs. Verification fails with `native generation still pending`.
This is a real applied-policy difference, separate from the deferred capacity mismatch.

The bridge boot log reports a structural rebuild and a persisted 27-slot boot world.
The lqosd journal repeatedly reports `NON_SUFFIX_STRUCTURAL_REMOVE`.
The broker holds removals that are not a contiguous tail of the dense slot table.
This is the observed upstream structural-update hold. The installed recovery path's internal failure is not fully proven.
One topology custody warning also reports `world_generation_mismatch`.
The public beta19 source documents those guard paths but does not prove all current private implementation details.

## Review and regression

No LQoSync3 runtime defect was demonstrated. Keep generation correlation and exact policy validation.
The new regression uses a synthetic 28-entry generated world and a 27-entry boot world.
It rejects the old generation and incomplete content even after generation strings match.
Only complete matching content can produce acceptance in the fixture.

Performance review: no production work was added. Existing ticks skip missed intervals and reuse prepared normalization.
Integrity review: input hashes, producer validation, stable reports, exact operations, and receipt binding remain required.
Reliability review: the real receipt remains IMPORT_CONFIRMED with unproven native ACK. No false APPLIED label was introduced.
The first lint run flagged assertion style in the new test. The final assertions and Clippy checks pass.

Full suite: 705 passed, zero failed, nine ignored across 19 suites.
The focused regression passed again after the assertion-style correction.
Final Clippy with warnings denied, formatting, and diff checks passed.
No product runtime code, production files, configuration, service, or Telegram state changed.
The local commit contains only this task's regression, report, plan, and session note.

## Remaining condition

Native policy recovery is unresolved and requires upstream investigation or a separately reviewed recovery procedure.
Do not force writes, restart the bridge, alter LKG, or weaken acceptance checks to clear this status.
A separate startup Node Manager address-bind failure was observed. Its current state and causal role remain unverified.

Report: `docs/operations/POST_REBOOT_NATIVE_ACK_DIAGNOSIS.md`.
Evidence: `/home/pi/.local/state/lqosync3/post-reboot-ack-task2/`.
Public journal reads are available, but binary MESSAGE arrays need decoding before text matching.
Preserve unrelated profile/WAN notes and earlier release/research documentation.
