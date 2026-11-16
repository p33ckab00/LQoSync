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

## Public release and support search

Checked: 2026-11-16T10:46:56+08:00 (Asia/Shanghai).

- The public main branch still points to `9ebe29bc9d49013aff9b0544de3c6f6fc6ba2f1d`, the beta19 source projection dated 2026-11-08.
- The GitHub releases endpoint lists v2 releases. It supplies no v3 recovery-build mapping.
- Public issue searches for `non-suffix`, `NON_SUFFIX_STRUCTURAL_REMOVE`, and maintenance with 3.0 returned no matching issue.
  This search does not cover private support channels or prove that upstream has no report.
- The official v3 installation guide documents beta-to-beta package backup and rollback.
  It requires config, LKG, and all three old package files before an upgrade.
  The v2 migration rollback tool serves a different transition.
- This generic package procedure does not confirm recovery from the combined non-suffix and maintenance hold on this build.
- Direct reads of both beta `InRelease` and uncompressed `Packages` returned HTTP 403.
  The local APT list dates from 2026-11-15. It cannot establish the current repository candidate.

No public source found in this search confirms the exact supported recovery build, action, or rollback for this incident.
The next step requires a maintainer answer to the prepared report. The report remains unsent.
No package installation, APT refresh, service action, or maintenance override occurred.

References:

- [Public source projection](https://github.com/LibreQoE/LibreQoS/commit/9ebe29bc9d49013aff9b0544de3c6f6fc6ba2f1d).
- [Release list](https://github.com/LibreQoE/LibreQoS/releases).
- [Official v3 installation and beta rollback guide](https://github.com/LibreQoE/LibreQoS/blob/9ebe29bc9d49013aff9b0544de3c6f6fc6ba2f1d/docs/v3/install.md#beta-to-beta-upgrade).
- [Official v2-to-v3 migration guide](https://github.com/LibreQoE/LibreQoS/blob/9ebe29bc9d49013aff9b0544de3c6f6fc6ba2f1d/docs/v3/upgrade-from-v2.md).

## Internal report expansion — 2026-11-16T11:02:40+08:00

The owner prohibited external publication and requested detailed internal context.
Expanded `docs/operations/NATIVE_RECOVERY_UPSTREAM_REPORT.md` with purpose, project background, requirements, chronology, tests, findings, questions, and acceptance criteria.
The filename stays unchanged to preserve links. The document is an internal report, not an outbound draft.
Separated historical success from post-reboot evidence, policy acceptance from capacity, and observations from unproved recovery capability.
Reviewed the report against the unified-health, explicit-ID, ACK, and recovery records.
Documentation checks cover whitespace, relative links, and required sections. The STE checker supplied heuristic findings for manual review.
No product code or live state changed. No tests were rerun, report sent, commit made, or deployment performed.
Next: keep the report internal and compare relevant future beta updates against its acceptance requirements.

## Repository publication authorization

The owner authorized commit and push of the detailed report and related documentation to the LQoSync `lqosync3` branch.
This permits repository publication. It does not authorize an upstream issue or a message to maintainers.
The report header now states this distinction. Existing unrelated work stays outside this documentation commit.
