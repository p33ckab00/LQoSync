# Executor acceptance (Approach B) with one live subscriber

Status: approved by the operator in chat, 2026-10-31.

## Intent

Phase 1 and Phase 2 used the scoped transport directly.
This test runs the full guarded path on the production core: reconciliation preview, prepared batch,
exclusive executor lease, durable journal, forward, rollback, and finish.

## Scope

The live settings select whole groups. Most selected subscribers overlap protected ranges, so they are Blocked.
The executor rejects any batch with a Blocked decision. The batch limit is 20 operations.
The test therefore uses a separate acceptance data directory: `v3/data/state/acceptance/b-root`.
It holds a copy of `credentials.toml`, the TLS anchors, and changed settings:

- `core_control.mode = "preview"`.
- Only selection `0b01777b-98e4-515b-88ec-6b08504d6e39`, changed to `selector = "subscriber"`, `value = "RowenaEstilloso"`.
- Only policy `source-pppoe-tier-15m-wan2`, and only its replacements.
- A pin for `6c607ca4-1da1-51e6-a402-802aa35bd43b` where the batch needs one.

The executor lease, journal, and recovery marker belong to that directory.
The live console and live settings do not change. The console has no router write path.
This replaces the earlier plan to stop the console and narrow the live settings. It changes less.

## Batches

1. Pin to wan1 (`source-pppoe-tier-15m-wan2--wan1`). Expected: `add` to `WAN1-CUSTOMERS`. Forward, then finish.
   Traffic does not change, because wan1 is also the `main` default.
2. Pin to wan2 (`source-pppoe-tier-15m-wan2`). Expected: `move` to `WAN2-CUSTOMERS`.
   Forward, hold 10 seconds, roll back, then finish. The subscriber uses wan2 during the hold.
   The hold must stay short. The source continuity window is at most 30 seconds, and each capture takes time.
3. Cleanup: the operator removes the owned row on the router. A `remove` decision needs 30 seconds of offline evidence.
   Removal of the owned wan1 row does not change traffic.

## Command

`core-worker-run ROOT --expect KIND:SUBSCRIBER:LIST [--rollback-after SECONDS] [--execute]`

- It runs up to 12 preview rounds. It stops at the first round whose plan has exactly the expected operation,
  no other mutation, and no Blocked decision.
- Without `--execute`, it prints the batch revision and sends no write.
- With `--execute`, it runs the preview again and builds a new batch. The approval is the `--expect` operation:
  the command opens the worker only for a batch with exactly that operation. It then runs forward, an optional
  rollback of at most 10 seconds after the forward, and finish.
- The approval window is 180 seconds. A failure keeps the recovery marker. Do not delete the marker.

## Limits found in review

- Reviewed recovery cannot run from a new process. The source continuity window is at most 30 seconds,
  and a new source needs about 50 seconds of probe rounds before WAN health is Healthy.
  A live test of a recovery command stopped with "Source continuity window expired". The command was removed.
- `core-worker-run` therefore tries an immediate rollback in the same process when the forward fails.
  The library refuses that rollback after an unknown outcome or a failed verification, because both revoke the grant.
- After any other failure, or a process crash, remove the owned row on the router by its `.id`.
  The subscriber then uses the `main` default (wan1). Keep the acceptance directory state as evidence.
- The acceptance directory has its own fence and marker. The live console cannot see them.
  Before any live write path is enabled, check `v3/data/state/acceptance/b-root/state` for a marker.
- The first run of batch 1 stopped before any send. The guard digest included mangle and filter rule counters.
  Those counters change on every read. The guard now ignores `bytes` and `packets` only.

## Approaches

- **Selected:** a separate acceptance data directory with one subscriber in scope.
- **Rejected:** narrow the live settings and stop the console. It changes production settings and the console.
- **Rejected:** a partial batch that skips Blocked decisions. It changes the executor safety model.
