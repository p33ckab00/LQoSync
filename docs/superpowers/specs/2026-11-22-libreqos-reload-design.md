# LibreQoS reload after a pair change: design

Date: 2026-11-22. Status: design for implementation. No plan yet.
Related: [LibreQoS v3 ingress](../../LIBREQOS-V3-INGRESS.md), [architecture](../../ARCHITECTURE.md).

## Intent

LibreQoS must apply a new `network.json` and `ShapedDevices.csv` soon after LQoSync writes them.
This design keeps two paths:

- **Path 1 (primary).** LQoSync writes a reload token. The LibreQoS scheduler starts a full reload in 1 second or less.
- **Path 2 (safety net).** The scheduler partial reload runs at a shorter interval. It applies the pair if Path 1 fails.

Path 1 is in the code now. Path 2 is a change to the LibreQoS configuration.
The tasks in this document measure Path 1 on the live system and add Path 2.

## Facts about LibreQoS 3.0.7

The installed package is `libreqos 3.0.7.202611201638`. These facts come from `/opt/libreqos/src/scheduler.py`.

- With `source = "shaped_devices"`, the scheduler imports the pair into `state/topology/topology_import.json`.
- The scheduler has no file watcher for the pair. A file write alone does not start an import.
- `LibreQoS.py --updateonly` does nothing in v3. It prints `refreshShapersUpdateOnly: skipping the legacy file chain.`
- The scheduler uses one worker thread (`ThreadPoolExecutor(1)`). Two jobs never run at the same time.
- The scheduler reads `queue_refresh_interval_mins` one time, at start. A change needs a restart of `lqos_scheduler`.

The scheduler has these jobs:

| Job | Interval | What it does |
|-----|----------|--------------|
| `importAndShapeFullReload` | One time, at start | Imports the pair. Always runs `refreshShapers()`. |
| `importAndShapePartialReload` | `queue_refresh_interval_mins` (now 30) | Imports the pair. Runs `refreshShapers()` only if the shaping hash changed. |
| `reload_request_tick` | 1 second | Reads `state/scheduler/reload_request.json`. A valid token starts one full reload. |
| `topology_runtime_refresh_tick` | 3 seconds | Publishes the topology runtime state. |
| `not_dead_yet` | 1 minute | Publishes a heartbeat. |

**CAUTION:** The full reload always rebuilds all shapers. It does not compare the shaping hash first.

## Path 1: reload token

### Requirements

All requirements are true on the live system now.

- LibreQoS v3 with `source = "shaped_devices"` in `/etc/lqos.conf`.
- The `lqos_scheduler` service is active.
- The LQoSync service user can write to `<state_directory>/scheduler/`.
- `network.json` and `ShapedDevices.csv` are regular files. The `lqosd` watcher refuses a symbolic link.

### Behavior

1. LQoSync calculates the hash of the pair on each cycle. The cycle interval is 20 seconds (`scan_interval_active = 20`).
2. If the hash changed, LQoSync writes the pair. It writes each file to a temporary file, then renames it.
3. LQoSync writes the token `{"requested_at": <seconds>, "nonce": <write_id>, "requested_by": "lqosync3"}` (`src/files.rs:388`).
4. In 1 second or less, the scheduler reads the token, deletes the file, and runs one full reload.
5. LQoSync compares the imported pair with the pair that it wrote.
   - Until they agree, the status shows `waiting for the LibreQoS import (<n> s)`.
   - When they agree, the publication phase is `IMPORT_CONFIRMED`.
   - After 40 minutes (`IMPORT_LIMIT_SECS`) without agreement, the cycle reports `LibreQoS did not import the new files`.

The nonce is the `write_id` of the write. A retry of the same write uses the same nonce, so the scheduler ignores it.
The scheduler also ignores a token that is older than its start.

There is only one token file. If LQoSync writes several tokens during one full reload, the last token replaces the others.
Thus, the scheduler runs a maximum of one more full reload after the current one.

### Risks

- **Frequent full rebuilds.** If subscribers connect and disconnect often, the pair can change on each 20-second cycle.
  Each change starts a full rebuild of all shapers. The effect on traffic during a rebuild is not measured.
- **Internal interface.** `reload_request.json` is internal to LibreQoS. A LibreQoS upgrade can change or remove it.
  Then Path 1 stops without an error in LibreQoS. Only the 40-minute import limit of LQoSync shows the failure.
- **Skipped ticks.** A full reload holds the only worker thread. The scheduler can skip `reload_request_tick` runs during that time.
  The token stays in the file, so the next tick after the reload reads it.

## Path 2: shorter partial reload interval

### Requirements

- Change `queue_refresh_interval_mins` in the `[integration_common]` section of `/etc/lqos.conf` from `30` to `10`.
- Restart `lqos_scheduler`. The restart runs one startup full reload, which rebuilds all shapers one time.
- Do the restart in an owner-approved maintenance window.
- Keep the interval less than 40 minutes, which is the import limit of LQoSync.

### Behavior

- Every 10 minutes, the scheduler imports the pair and calculates the shaping hash.
- If the hash did not change, the scheduler does not rebuild the shapers. The cost is only the import.
- If Path 1 failed, the scheduler applies the pair in 10 minutes or less.
- Path 2 cannot run at the same time as a full reload, because the scheduler has one worker thread.

Do not set the interval to 1 or 2 minutes. Each run imports the full pair, also when nothing changed.

## Combined behavior

| Condition | Result |
|-----------|--------|
| Normal pair change | Path 1 applies the pair in seconds. The next Path 2 run finds the same hash and does not rebuild. |
| Path 1 fails (for example, after an upgrade) | Path 2 applies the pair in 10 minutes or less. LQoSync shows the wait time in the status. |
| Scheduler stopped | No path applies the pair. After 40 minutes, LQoSync reports the import error. |
| Pair changes on each cycle | Path 1 runs full rebuilds back to back. Task 3 measures this case. |

## Tasks

### Task 1: Measure the import delay of Path 1

- Make one subscriber change on the live system.
- Record the time of the LQoSync write and the time of `IMPORT_CONFIRMED`.
- Read the `lqos_scheduler` log for `reload_requested` and for the end of the full reload.
- Close the open item in `docs/LIBREQOS-V3-INGRESS.md` ("Find if a write of the pair starts an import before the 30-minute job").

Acceptance: the ingress document records the delay from the write to `IMPORT_CONFIRMED`.

### Task 2: Measure the cost of a full rebuild

- Record the duration of one full reload from the scheduler log.
- Record the effect on subscriber traffic during the rebuild, for example latency or a short loss of shaping.

Acceptance: a session note records the duration and the traffic effect. The owner reads the result.

### Task 3: Count pair changes on the live system

- Count the reload tokens for each hour, for a minimum of one normal day.
- Use the LQoSync journal or the `topology_reload_request` health component.

Acceptance: a session note records the number of full reloads for each hour.

Decision gate: the owner compares the result of Task 3 with the cost from Task 2.
If the cost is too high, a new design must add a minimum time between two tokens. That design is not part of this document.

### Task 4: Apply Path 2

This task changes the live LibreQoS configuration. Do it only in an owner-approved window.

1. Make a copy of `/etc/lqos.conf`.
2. In `[integration_common]`, set `queue_refresh_interval_mins = 10`.
3. Restart `lqos_scheduler`.
4. Read the scheduler log. Make sure that it shows `importAndShapePartialReload every 10 minutes`.
5. Make sure that LQoSync shows `IMPORT_CONFIRMED` after the startup full reload.

To roll back, put the copy of `/etc/lqos.conf` back and restart `lqos_scheduler`.

Acceptance: the scheduler log shows the 10-minute interval. LQoSync shows `IMPORT_CONFIRMED`.

### Task 5: Test the safety net

- Stop Path 1 for one test. For example, on a test system, make the token directory read-only for LQoSync.
- Make one subscriber change.
- Make sure that the scheduler applies the pair in 10 minutes or less, and that LQoSync shows `IMPORT_CONFIRMED`.

Do not do this test on the live system without owner approval.

Acceptance: Path 2 applies the pair without Path 1.

### Task 6: Update the documents

- In `docs/LIBREQOS-V3-INGRESS.md`, record the results of Tasks 1 to 3 and the new interval.
- In `docs/OPERATIONS.md`, add the Path 2 configuration and the rollback procedure from Task 4.

## Not in scope

- A minimum time between two tokens. It needs the results of Tasks 2 and 3 and an owner decision first.
- A change of `source` to `libreqos_network`. That is a different target with different rules.
- A restart of `lqos_scheduler` as a reload method.
