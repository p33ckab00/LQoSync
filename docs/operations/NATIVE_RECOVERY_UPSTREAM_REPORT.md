# Upstream report draft: non-suffix removal and maintenance-deferred reload

Status: local draft only. No issue or message has been sent.

## Environment

- LibreQoS control/preflight/dataplane package version: `3.0.7.202611141944`.
- Bridge build revision: `193d067a6ad9`.
- Installed bridge and control binaries match the cached package contents.
- Host reboot: 2026-11-16T09:39:36+08:00.

## Observed condition

The bridge booted with 27 slots. Later input/import/generated policy contains 28 leaf and mapping operations.
Three native samples confirm stable, differing generations and policies.
The acceptance probe returns `native generation still pending`.
The broker repeatedly logs `NON_SUFFIX_STRUCTURAL_REMOVE` for one non-tail removal.
Fresh `/run/libreqos/seam_state.json` samples report:

- `state=healthy`, no in-flight transaction, and no current reject.
- One non-suffix removal in a 27-entry slot table.
- `deferred_maintenance.class=full-world bulk/reload`, 48 held operations, `forced=false`.
- No `pending_non_suffix` field.

The installed control binary has the legacy non-suffix warning and maintenance override markers.
It lacks the public R3 pending/complete strings and `non_suffix_rebuild_trigger` marker.
No R3 recovery event appeared in the sampled boot journal.
The public beta19 projection contains the separate non-suffix R3/re-adopt path.
The bridge read-only boot-input check exits zero.

## Questions

1. Which matched control-plane/dataplane package pair supports the public non-suffix R3/re-adopt path?
2. Can maintenance apply-now recover this exact combined hold on the installed build?
3. What full-world install pause and verification should the operator expect?
4. What is the supported rollback after a one-shot apply is consumed?
5. Is `state=healthy` with a maintenance-deferred, non-converged desired world the intended status contract?

No forced writes, service restarts, LKG changes, package updates, or maintenance overrides occurred during diagnosis.
Subscriber identifiers, private policy artifacts, credentials, and packet payloads are excluded from this draft.
The separate native capacity discrepancy remains deferred.
