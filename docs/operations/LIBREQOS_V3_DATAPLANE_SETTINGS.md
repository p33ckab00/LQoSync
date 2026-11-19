# LibreQoS v3 dataplane update settings

This document describes two LibreQoS v3 settings that control when the DPDK bridge applies a new shaping world.
The settings belong to LibreQoS, not to LQoSync. The operator edits them on the shaper.
LQoSync writes `network.json` and `ShapedDevices.csv` and requests the scheduler import. It does not change these settings.

Report of the problem that these settings affect: [LibreQoS v3 dataplane hold report](LIBREQOS_V3_DATAPLANE_HOLD_REPORT.md).

Sources:

- Installed packages `libreqos` and `libreqos-dataplane` `3.0.7.202611181749`.
- Public LibreQoS source, branch `main`, commit `828666a37` ("LibreQoS 3.0.7 -- public source projection", 2026-11-19).
- Files: `src/rust/lqos_dataplane_types/src/lib.rs`, `src/rust/lqosd/src/bridge_broker/mod.rs`, `src/rust/lqosd/src/bridge_broker/maintenance.rs`, `src/rust/lqos_config/src/etc/v15/maintenance_window.rs`.

NOTE: LibreQoS v3 is a beta. The source of `bridge_rs` is not public. The installed build can differ from the public source. Do these checks again after each LibreQoS update.

## How a new pair gets to the dataplane

1. LQoSync writes `network.json` and `ShapedDevices.csv` in `/opt/libreqos/src`.
2. LQoSync writes `/opt/libreqos/state/scheduler/reload_request.json`. The LibreQoS UI reload button writes the same token.
3. `libreqos-scheduler` reads the token in approximately 1 s and runs one `importAndShapeFullReload`. Without the token, the import occurs at the next 30-minute job.
4. The compiler writes the desired world to `/opt/libreqos/state/shaping/dataplane_transaction.json`.
5. The bridge broker in `lqosd` compares the desired world with the applied world that `bridge_rs` reports. It calculates a delta.
6. The broker classifies the delta. A pause-free delta goes to the bridge at once. A pause-class delta waits for the maintenance window.
7. `bridge_rs` applies the delta without a restart.

Steps 1 to 4 are the scope of LQoSync. Steps 5 to 7 are internal to LibreQoS.

## Setting 1: `LIBREQOS_DPDK_BULK_GENERATION_THRESHOLD`

### What it does

The threshold is a count of single operations (ops). It decides the payload shape of a delta:

- A delta with ops equal to or less than the threshold is an incremental sequence. It is pause-free.
- A delta with more ops than the threshold is a "full-world bulk/reload". It is pause-class, so the maintenance window holds it.

Both ends use the same rule. The broker (producer) uses it to select the payload shape. The bridge (consumer) uses it to decide to patch the delta or to rebuild the world. The two ends must use the same value.

### Default value

Without the variable, the threshold comes from the world size (the number of LPM prefixes):

```text
threshold = max( floor(world^0.9 / 2), 8 )
```

The value `8` is `SEQUENCE_FLOOR_OPS`. LibreQoS calibrated the formula for worlds of 10,000 prefixes and more. For a small world, the floor of 8 controls the result.

| World (prefixes) | Default threshold |
|---|---|
| 28 | 10 |
| 1,000 | 250 |
| 10,000 | 1,990 |

With the variable, its value replaces the formula. It is not a floor and not a ceiling. A value of 0 becomes 1.

### Where to set it

Put the variable in `/etc/libreqos/bridge.env`. Do not put it in `bridge.env.d/`.

- `libreqos-lqosd` and `libreqos-bridge` both read `/etc/libreqos/bridge.env`.
- Only `libreqos-bridge` reads `/etc/libreqos/bridge.env.d/*.conf`. A value in that folder gives the two ends different thresholds.

```bash
LIBREQOS_DPDK_BULK_GENERATION_THRESHOLD=200
```

Each process reads the value when it starts. The current value shows in the `RESOLVED_POSTURE` log line of each process:

```bash
sudo journalctl -u libreqos-bridge -u libreqos-lqosd -b --no-pager | grep RESOLVED_POSTURE \
  | grep -oE 'side="[a-z]+"|bulk_generation_threshold(_set)?=[^ ]+'
```

### Recommendation for this shaper

This shaper has approximately 28 circuits, so the default threshold is 10. A normal change from the routers can be more than 10 ops:

- One new or removed circuit gives a leaf op, a mapping op, and an LPM op.
- A change of a group factor changes the rate of each circuit in the group and of each parent node.
- After a bridge restart, the difference between the last-known-good world and the desired world was 34 ops.

Thus most changes become "full-world bulk/reload" and wait for the maintenance window.

The recommended value is `200` for a world of less than 1,000 circuits. With this value, a normal change and a group factor change stay incremental and pause-free.

- Use a value that is more than the largest normal change. Count the ops in `held_ops` of `/run/libreqos/seam_state.json` when a hold occurs.
- Do not use a value that is much larger than the world. A very long incremental sequence takes more time than one full-world install.
- For a world of more than 1,000 circuits, remove the variable and use the default formula, or ask LibreQoS support.

NOTE: This recommendation is not tested on this shaper. Do the procedure below and record the result.

### Procedure to change the threshold

CAUTION: A RESTART OF `libreqos-bridge` STOPS FORWARDING AND SHAPING FOR SOME SECONDS. DO THIS STEP ONLY IN A MAINTENANCE WINDOW.

1. Make a copy of the file:

   ```bash
   sudo cp -p /etc/libreqos/bridge.env /etc/libreqos/bridge.env.bak-$(date +%F)
   ```

2. Add the line `LIBREQOS_DPDK_BULK_GENERATION_THRESHOLD=200` to `/etc/libreqos/bridge.env`.
3. Restart the bridge first:

   ```bash
   sudo systemctl restart libreqos-bridge
   ```

   The bridge boots from the last-known-good world. The broker still uses the old threshold, so it holds a large delta. It does not send a delta that the bridge rejects.
4. Restart `lqosd`:

   ```bash
   sudo systemctl restart libreqos-lqosd
   ```

   `libreqos-scheduler` requires `libreqos-lqosd`, so systemd also restarts the scheduler. The bridge continues to shape while `lqosd` restarts.
5. Make sure that both posture lines show `bulk_generation_threshold=200` and `bulk_generation_threshold_set=true`. Use the `journalctl` command above.
6. Wait for 1 minute. Then read the seam state:

   ```bash
   sudo python3 -c "import json; d=json.load(open('/run/libreqos/seam_state.json')); \
   print({k: d.get(k) for k in ('state','applied_generation','desired_generation','deferred_maintenance','last_reject')})"
   ```

   The expected result: `deferred_maintenance` is `None`, and `last_reject` does not show a new reject.
7. Make sure that the bridge applied the desired generation. The `generation` in the bridge `state` report must be equal to `shaping_generation` in `/opt/libreqos/state/topology/topology_runtime_status.json`. The LQoSync live test does this check in its `acknowledgement` step.

### Rollback

1. Copy the backup file back to `/etc/libreqos/bridge.env`.
2. Do steps 3 to 5 of the procedure again. Make sure that the posture lines show `bulk_generation_threshold_set=false`.

## Setting 2: `[maintenance_window]`

### What it does

The `[maintenance_window]` section in `/etc/libreqos/lqos.conf` is the publish gate of the broker. When the window is closed, the broker holds a pause-class delta. It examines the held delta again at each tick.

The pause classes are:

- "full-world bulk/reload": the op count is more than the bulk threshold.
- `Baseline`: a complete-world install.
- A circuit-capacity regrow.
- A change of the leaf AQM kind.
- A shard-set change: a circuit add above 4 Gbps, or a rate change across the 4 Gbps boundary.

A delta that is not in this list is pause-free. The broker sends it at once. A pause-free delta also clears a held delta.
Only one change is held at a time. The broker merges each new desired world into the held change.

### Keys

| Key | Current value | Default | Meaning |
|---|---|---|---|
| `enabled` | `true` | `true` | `false` makes the gate inert. The window is then always open. |
| `start_local` | `"03:00"` | `"03:00"` | The start of the window, `HH:MM`, 24-hour. |
| `end_local` | `"04:00"` | `"04:00"` | The end of the window. If `end_local` is equal to or before `start_local`, the window crosses midnight. |
| `timezone` | `""` | `""` | An IANA zone name. `""` uses the server zone (`/etc/localtime`). This shaper uses Asia/Manila. |
| `max_pause_ms` | `25` | `25` | Reserved. It has no effect in this build. |
| `force_apply_hours` | `24` | `24` | The broker sends a held change when it is older than this number of hours, also outside the window. |

A malformed or zero-length window holds nothing. The broker then uses an always-open window.

### Operator overrides

The LibreQoS WebUI writes the override file `/opt/libreqos/state/shaping/maintenance_control.json`. The broker reads it at the next tick:

- `apply_now`: send the held change one time, outside the window.
- `break_glass_until_unix`: send all held changes until this time.

Use the WebUI to set an override. Do not write the file by hand.

### Recommendation for this shaper

- Keep `enabled = true`. The gate prevents a pause of shaping in the busy hours.
- Keep the window at 03:00 to 04:00 if that is the period with the least traffic.
- Keep `force_apply_hours = 24`. It limits the age of a held change.
- Set the bulk threshold as in setting 1. Then normal changes are pause-free and do not wait for the window.

NOTE: Do not use `enabled = false` as a repair for the hold that the report describes. In this build, the bridge rejects the full-world delta also inside the window. An always-open window only sends the same rejected delta sooner.

## How to know that a pair is in the dataplane

The LQoSync status `imported by LibreQoS` shows that the scheduler imported the pair. It does not show that the bridge applied it.
To see the dataplane state, read `/run/libreqos/seam_state.json`:

| Field | Meaning |
|---|---|
| `desired_generation` | The generation of the transaction that the compiler wrote. |
| `applied_generation` | The content key of the world that the bridge applied. |
| `last_applied_unix` | The time of the last apply. |
| `deferred_maintenance` | The held change: `class`, `first_deferred_unix`, `held_ops`, `forced`. `null` means that no change is held. |
| `last_reject` | The last reject from the bridge, with its generation. |
| `last_removal` | The last circuit remove set. `suffix = false` means that the removed circuits are not at the end of the slot table. |
