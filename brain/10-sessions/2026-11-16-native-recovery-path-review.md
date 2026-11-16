---
type: session
created: 2026-11-16
tags: [native-ack, recovery, upstream, research]
status: review-complete
related: ["[[2026-11-16-post-reboot-native-ack-diagnosis]]", "[[HANDOFF]]"]
---

# Native recovery path review

Reviewed: 2026-11-16T10:36:46+08:00 (Asia/Shanghai). Agent: Codex `/root`.
The owner approved read-only recovery research and preparation of a backup/verification/rollback procedure.

Three fresh broker snapshots show one non-tail removal and a 48-operation full-world reload held by maintenance.
No transaction is in flight. The execution label says healthy, but policy convergence remains unproven.
Both installed binaries match cached package contents. The read-only bridge boot-input check exits zero.
No successful restart or selected boot world is inferred from that readiness check.

The public broker contains a non-suffix R3/re-adopt path with a separate recovery gate.
Installed strings and snapshots lack its event/gate markers. No recovery event appeared in the sampled journal.
This is supporting evidence of a capability difference, not proof of every private compiled branch.
The installed maintenance apply-now control exists, but bypasses only its window gate.
Do not claim that it necessarily clears the separate structural fence.

Prepared a conditional recovery runbook and metadata-only upstream report draft.
The runbook requires a matched recovery-capable build and a verified rollback before live execution.
After a control request is consumed, deleting its file cannot reverse an applied policy.
No automatic rollback or recovery success is claimed.

Evidence: `/home/pi/.local/state/lqosync3/recovery-path-review/`.
The audit passed fresh-snapshot, metadata sanitization, package-hash, and boot-readiness checks.
No production code, config, service, shaping input, LKG, or maintenance control changed.
No external report was sent. No package update, restart, backup, or live recovery occurred.

Runbook: `docs/operations/NATIVE_RECOVERY_PATH_REVIEW.md`.
Report draft: `docs/operations/NATIVE_RECOVERY_UPSTREAM_REPORT.md`.
Next: obtain upstream confirmation of the matched recovery build/action and its rollback contract.
Preserve prior local changes and the unrelated profile/WAN notes.
