# Native recovery path review

Reviewed: 2026-11-16T10:36:46+08:00 (Asia/Shanghai).
Status: read-only review complete. Live recovery is not approved or verified.

## Current evidence

Three fresh broker snapshots report the following condition:

- One non-suffix removal remains in a 27-slot native table.
- A `full-world bulk/reload` with 48 operations remains maintenance-deferred.
- No transaction is in flight. The override has not forced the change.
- The execution state says `healthy`, but desired policy convergence is unproven.

The earlier native probe found 27 applied leaf/mapping operations versus 28 generated operations.
The current shared apply health still reports `import_confirmed_ack_unproven`.
Do not use the broker's execution-state label as proof that the desired world is applied.

The installed lqosd and bridge binaries match their cached `3.0.7.202611141944` packages.
This rules out a local binary mismatch against those cached packages. It does not prove protocol compatibility.
The read-only bridge `--check-boot-inputs` exits zero.
That proves input readiness only. It does not prove successful restart, selected boot policy, or traffic continuity.

The boot journal contains one maintenance-window hold and repeated non-suffix removal warnings.
The sampled journal has no R3-pending or R3-complete event for this hold.
The installed lqosd lacks the public recovery event strings and `non_suffix_rebuild_trigger` symbol string.
Absence of strings is supporting evidence, not proof of every compiled branch.

## Public recovery path versus installed behavior

The public beta19 broker source contains a separate non-suffix recovery gate.
It emits a remove-only trigger and waits for full-world R3/re-adopt before admitting new incremental work.
The installed snapshot lacks that public `pending_non_suffix` field.
The installed behavior and markers therefore do not establish that this recovery path is available or active.
Do not assume that a newer package timestamp contains every public source path.
Confirm the matched control-plane/dataplane release and recovery capability with upstream before using that mechanism.

## Available control and its limits

The installed binary contains the maintenance override control and audit markers.
The public control contract uses `maintenance_control.json` beside the generated transaction.
For this host, the path is `/opt/libreqos/state/shaping/maintenance_control.json`.
A one-shot request has this shape:

```json
{"apply_now":true,"requested_by":"operator","requested_at_unix":0}
```

The timestamp above is a placeholder, not a request to publish.
The broker consumes this file once. Apply-now bypasses the maintenance-window gate.
It does not grant proof that a separate structural fence will clear.
Global break-glass exists but bypasses more changes and is not the preferred test.
No control request was written in this investigation.

The configuration has no explicit maintenance-window section.
Public defaults do not prove the installed resolved schedule.
Do not promise that an assumed 03:00 window or a 24-hour timeout will resolve the structural fence.

## Conditional recovery procedure

This procedure remains blocked until its capability and rollback conditions are satisfied.

1. Confirm an upstream-supported recovery path for this exact matched package pair.
   Ask whether apply-now can recover the non-suffix hold, or whether an R3-capable update is required.
2. Select an operator-approved maintenance period. A full-world install can pause traffic.
3. Create a private checkpoint before any mutation.
   Copy the current package artifacts, installed binaries, unit/drop-ins, configuration, generated policy, and LKG state.
   Preserve ownership and permissions. Store subscriber-bearing copies in a root-owned mode-0700 directory.
   Compare native reports and file hashes before and after the checkpoint. Reject an unstable checkpoint.
4. Record a verified rollback procedure for the selected recovery action.
   Identify which services must be coordinated and how the saved applied world can be restored.
   Do not treat an LKG copy or an old binary alone as a proven rollback.
5. Execute only the confirmed action after the operator approves the concrete procedure.
   Do not combine apply-now, bridge restart, LKG deletion, and package replacement as trial steps.
6. Verify current policy through the existing read-only acceptance probe.
   Require current inputs/import, matching generations, complete policy operations, stable native evidence, and producer validation.
   Also check live traffic, mapping, queue/NIC health, and service stability.
7. Retain the existing capacity warning as separate evidence if policy acceptance succeeds.
   The deferred 64-versus-1000 capacity issue is not this recovery's success criterion.

## Rollback boundary

Before any live action, aborting this procedure changes no production state.
After a one-shot request has been consumed, removing its file does not undo an applied policy.
Restoring the window setting also does not reverse completed dataplane changes.
A coordinated restore of saved configuration or LKG can itself interrupt traffic.
That restore must be verified for the exact build before execution approval.
No automatic rollback is claimed here. No backup or live recovery has occurred in this review.

## Recommended next action

Obtain upstream confirmation of the recovery-capable matched package pair and its rollback path.
Use `docs/operations/NATIVE_RECOVERY_UPSTREAM_REPORT.md` as a metadata-only report draft.
Do not send it automatically. Do not change live topology to fabricate a suffix removal.
Keep LQoSync3's generation and policy checks intact while the upstream state is unproven.

Evidence: `/home/pi/.local/state/lqosync3/recovery-path-review/metadata.json`.
The metadata audit passed fresh-state, sanitizer, package-hash, and boot-readiness checks.
No product-code tests apply because no product runtime code changed.

Sources:

- [Public broker recovery and maintenance control](https://github.com/LibreQoE/LibreQoS/blob/9ebe29bc9d49013aff9b0544de3c6f6fc6ba2f1d/src/rust/lqosd/src/bridge_broker/mod.rs).
- [Public maintenance classification](https://github.com/LibreQoE/LibreQoS/blob/9ebe29bc9d49013aff9b0544de3c6f6fc6ba2f1d/src/rust/lqosd/src/bridge_broker/maintenance.rs).
- Installed bridge wrapper, unit, binary strings, and read-only journal/snapshot observations.
