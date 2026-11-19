# LibreQoS v3 dataplane hold report

Status: internal report. Not sent to LibreQoS.
Date: 2026-11-20 (Asia/Manila).
Shaper: production shaper `libreqos`, packages `libreqos` and `libreqos-dataplane` `3.0.7.202611181749`.

Related documents:

- [LibreQoS v3 dataplane update settings](LIBREQOS_V3_DATAPLANE_SETTINGS.md)
- [Topology live acceptance runbook](TOPOLOGY_LIVE_TEST.md)
- [Post-reboot native ACK diagnosis](POST_REBOOT_NATIVE_ACK_DIAGNOSIS.md) (2026-11-16, a related hold)

## 1. Summary

Since the reboot on 2026-11-19 16:15, the LibreQoS DPDK bridge has applied no new shaping world.
LibreQoS imports each new `network.json` and `ShapedDevices.csv` correctly. The hold occurs after the import, inside LibreQoS:

1. The `lqosd` bridge broker classifies each change as "full-world bulk/reload", because it has more ops than the bulk threshold.
2. The maintenance window holds the change until 03:00.
3. At 03:00, the broker sends the change. `bridge_rs` rejects it with `NotExplicitFullWorld`.

Thus the dataplane keeps the world from the reboot. The LQoSync live test found the hold, because its `acknowledgement` step compares the generation that the bridge applied with the generation that LibreQoS published.

The cause is in LibreQoS, not in LQoSync. The same hold applies to the pair from the legacy LQoSync writer.

## 2. What happened

| Item | Observation |
|---|---|
| Desired world | `desired_generation` `f79f6c634e8784825ed98a9168ec2e3aabc6eadf9bc7e31ae20ac0e8c656a88b`, 27 leaf ops, 5 inner ops |
| Applied world | Bridge `state` report generation `068fa7a9b453aea2b74d762b6266fb0aa59b5fcef11798879c6935c64b075112`, 28 leaf ops, 4 inner ops |
| Last apply | `last_applied_unix` 1795076122 (2026-11-19 16:15:22), `applied_provenance` `BASE_ADOPT` |
| Held change | `deferred_maintenance`: `class` "full-world bulk/reload", `held_ops` 44, `first_deferred_unix` 1795120627 (2026-11-20 04:37:07), `forced` false |
| Last reject | `REJECTED_LPM_PREPARE @ f79f6c634e87...` |
| Last remove set | `ND9DNB79`, `SAAHDK74`, `TAGZJFN9`, `suffix` false, `slot_table_len` 28 |
| Bulk threshold | `bulk_generation_threshold=8`, `bulk_generation_threshold_set=false` (bridge posture at boot) |
| Maintenance window | `enabled = true`, 03:00 to 04:00 server time, `force_apply_hours = 24` |

Effects on the dataplane:

- The DHCP factor change of 2026-11-19 23:25 (0.5 to 1.0) is not in the applied world.
- The three removed circuits stay in the applied world.
- A subscriber added after the reboot has no leaf in the applied world.

NOTE: The op counts show that the two worlds are different. They do not identify each different circuit.

## 3. When it happened

All times are Asia/Manila. The journal of `bridge_rs` shows UTC in the message text.

| Time | Unit | Event |
|---|---|---|
| 2026-11-19 08:20 | `libreqos-lqosd` | First `NON_SUFFIX_STRUCTURAL_REMOVE` warning. It repeats every 2 s until 2026-11-20 05:08 (approximately 29,500 lines). |
| 2026-11-19 08:44 | dpkg | Upgrade to `3.0.7.202611181749`. |
| 2026-11-19 16:15 | host | Reboot. The bridge boots from the last-known-good world with 28 LPM prefix entries. |
| 2026-11-19 16:17:16 | `libreqos-lqosd` | `MAINTENANCE_WINDOW_HOLD class="full-world bulk/reload" held_ops=34`. |
| 2026-11-19 23:25:56 | `lqosync3` | Config reload with the new DHCP factor. LQoSync writes the pair and LibreQoS imports it. |
| 2026-11-20 03:00:02 | `libreqos-bridge` | The window opens. Generation `2569e8e6...`: `LPM full-world compile at mapping overlay: REFUSED`. |
| 2026-11-20 03:20:45 | `libreqos-bridge` | Generation `f79f6c63...`: `F3-B LPM prepare rejected (NotExplicitFullWorld)`. |
| 2026-11-20 04:37:07 | `libreqos-lqosd` | `MAINTENANCE_WINDOW_HOLD class="full-world bulk/reload" held_ops=44`. |
| 2026-11-20 05:50 to 05:56 | `lqosync3` | Live test run 2. The `acknowledgement` step fails after 120 s. |

The full bridge message at 03:20:45:

```text
LPM full-world compile at mapping overlay: REFUSED — from-view builds are disabled in production (wire apply_mode=Incremental); typed reject (the previous classifier is kept)
Control channel: generation f79f6c634e8784825ed98a9168ec2e3aabc6eadf9bc7e31ae20ac0e8c656a88b F3-B LPM prepare rejected (NotExplicitFullWorld) — typed reject (the previous classifier is kept)
```

## 4. Where it happened

| Part | Owner | Role |
|---|---|---|
| `/opt/libreqos/src/network.json`, `/opt/libreqos/src/ShapedDevices.csv` | LQoSync writes, LibreQoS reads | The input pair |
| `/opt/libreqos/state/scheduler/reload_request.json` | LQoSync writes, `libreqos-scheduler` reads | The import request |
| `/opt/libreqos/state/topology/topology_import.json` | LibreQoS | The imported pair. LQoSync reads it to make sure of the import. |
| `/opt/libreqos/state/shaping/dataplane_transaction.json` | LibreQoS compiler | The desired world |
| `lqosd` bridge broker | LibreQoS | Delta, classification, maintenance gate |
| `bridge_rs` | LibreQoS (`libreqos-dataplane`, source not public) | The DPDK dataplane |
| `/run/libreqos/seam_state.json` | LibreQoS broker | The broker state: desired, applied, held, reject |
| `/dev/shm/libreqos-analytics.sock` | `bridge_rs` | The bridge `state` report (applied generation) |

The hold is in the broker and the bridge. Both are internal to LibreQoS.

## 5. Who is involved

- **LibreQoS (LibreQoE):** owns `lqosd`, the broker, the compiler, `bridge_rs`, and the two settings. Only LibreQoS can repair the reject.
- **The shaper operator:** owns `/etc/libreqos/bridge.env` and `/etc/libreqos/lqos.conf`. The operator can change the bulk threshold and the maintenance window.
- **LQoSync:** writes the pair and requests the import. It does not change LibreQoS files, settings, or services.

## 6. How the workflow operates

### 6.1 LQoSync scope

1. LQoSync reads the MikroTik routers (read-only).
2. LQoSync writes `network.json` and `ShapedDevices.csv`.
3. LQoSync writes the reload token (`src/files.rs`, `request_v3_import`). The LibreQoS UI reload button writes the same token.
4. LQoSync reads `topology_import.json` until the imported pair is equal to the written pair. Then the status shows `imported by LibreQoS`.

In LibreQoS v3, `LibreQoS.py --updateonly` has no effect. The reload token replaces it.

### 6.2 LibreQoS scope

1. `libreqos-scheduler` reads the token in approximately 1 s and runs one `importAndShapeFullReload`. The scheduler source says: "The DPDK bridge is never touched."
2. The compiler writes the desired world to `dataplane_transaction.json`.
3. The broker compares the desired world with the applied world of the bridge and calculates a delta.
4. The broker classifies the delta. A delta with more ops than the bulk threshold is "full-world bulk/reload", a pause class.
5. If the delta is pause-class and the maintenance window is closed, the broker holds it. It merges later changes into the held delta.
6. When the window opens, or after `force_apply_hours`, the broker sends the delta to the bridge.
7. The bridge applies the delta, or it rejects it and keeps its previous world.

### 6.3 The bulk threshold for this shaper

The default threshold is `max(floor(world^0.9 / 2), 8)`. The world is the count of LPM prefixes.
For 28 prefixes, the threshold is 10. A normal router change, or a group factor change, has more than 10 ops.
Thus almost each change on this shaper becomes "full-world bulk/reload".

## 7. Why it is a bug

The shared code in `lqos_dataplane_types` says that the producer (broker) and the consumer (bridge) must use the same payload decision (R-WORLD-PARITY, R-FLOOR-PARITY).
The observed messages show a disagreement:

- The broker sent the held change with `apply_mode=Incremental`.
- The bridge decided to rebuild the LPM classifier from the world view, because the delta is above the threshold.
- In production, the bridge does not do a rebuild from the view. It requires an explicit full-world payload. It rejected the change with `NotExplicitFullWorld`.

Thus a held "full-world bulk/reload" change has no path to the dataplane in this build. The maintenance window opens, the broker sends the change, and the bridge rejects it. Each new change merges into the same held class, so the hold continues.

NOTE: The source of `bridge_rs` is not public. The statement about the bridge decision comes from its log messages, not from its source.

### 7.1 Related condition: non-suffix removes

The slot table of the bridge is LIFO. The bridge accepts a circuit remove only at the end of the table.
The three removed circuits are not at the end (`suffix` false). The broker logged `NON_SUFFIX_STRUCTURAL_REMOVE` from 2026-11-19 08:20 to 2026-11-20 05:08.
In the public source (`828666a37`), this capture is for diagnostics only. The installed build can be different.
The same family of hold occurred on 2026-11-16. See the post-reboot diagnosis.

## 8. How we tested

### 8.1 Background of the test

The test is Phase 3 of the LQoSync Topology Manager plan: live acceptance on production.
It proves that LibreQoS accepts the pair from the Topology Manager and applies it correctly. Then it puts the legacy pair back. It does not activate the manager.
The command is `lqosync3 topology live-test`. The procedure is in the [live acceptance runbook](TOPOLOGY_LIVE_TEST.md).

The steps of the command:

1. Prepare the manager pair and compile it with the LibreQoS compiler in a scratch workspace.
2. Check admission and parity with the legacy pair.
3. Make a backup of the legacy pair. Read back the baseline state of LibreQoS.
4. Write the manager pair and request the import. Wait for the import.
5. Read back the state of LibreQoS and compare it with the model.
6. Wait for the native acknowledgement: the bridge applied generation must be equal to `shaping_generation` in `topology_runtime_status.json`.
7. Write the legacy pair back, request the import, and compare the state with the baseline.

### 8.2 Runs on 2026-11-20

| Run | Result | Cause |
|---|---|---|
| Window 1, `check` | Exit 2, three `GroupFactors` lines | The DHCP factor changed after the policy. Repaired with a new policy (revision 2). |
| Run 1 (`report-2026-11-20.json`) | `FAIL compile`: `native state directory is missing` | LQoSync defect: `lqos.conf` has no `state_directory` key. Repaired in commit `40edf57`. |
| Run 2 (`report-2026-11-20-2.json`) | `FAIL acknowledgement` after 120 s: `native generation still pending` | The LibreQoS hold in this report. |

Run 2 step results:

- Passed: prepare, compile, admission and parity, backup, and baseline read-back.
- Passed: manager write, manager import, and manager read-back (0 differences).
- Passed: restore write, restore import, restore read-back (0 differences), and byte check.
- Failed: acknowledgement.
- The one difference from the legacy to the manager world is expected: node `lq3-f270781791bd2d0a` rounds 225.75 to 225 Mbps.

After run 2, the pair on disk was equal to the backup byte for byte. `lqosync3.service` started again with a good cycle.

### 8.3 How we found the cause

1. Read the bridge `state` report from `/dev/shm/libreqos-analytics.sock` (read-only request `{"kind":"state","limit":1}`). Its generation is different from `shaping_generation`, also in the normal legacy state.
2. Compare the leaf and inner op counts of the bridge report and `dataplane_transaction.json`.
3. Read `/run/libreqos/seam_state.json`: the held change and the last reject.
4. Read the journals of `libreqos-lqosd` and `libreqos-bridge` for the hold and reject events.
5. Read the public LibreQoS source of the broker, the maintenance gate, and the shared threshold.

## 9. Connection to the two projects

### 9.1 LibreQoS

- The hold is a LibreQoS v3 beta defect in the broker-to-bridge path.
- It affects each producer of the pair: the legacy LQoSync writer, the Topology Manager, and a manual edit.
- A small deployment is more exposed, because its threshold is the floor value of 8 to 10 ops.
- The repair belongs to LibreQoS: the broker must send an explicit full-world payload, or the bridge must accept the incremental payload.

### 9.2 LQoSync

- LQoSync does its scope correctly: it writes the pair, requests the import, and makes sure that LibreQoS imported the pair.
- The Phase 3 `acknowledgement` step checks the dataplane, which is outside the LQoSync scope. While the LibreQoS hold continues, this step cannot pass.
- The status `imported by LibreQoS` is correct. It does not mean that the bridge shapes the new world.
- LQoSync does not change LibreQoS settings. The operator decides on the settings in the settings document.

## 10. Recommended actions

1. **Operator:** set `LIBREQOS_DPDK_BULK_GENERATION_THRESHOLD=200` in `/etc/libreqos/bridge.env` and do the procedure in the settings document. This makes normal changes pause-free on this shaper. It does not repair the defect for a real bulk change.
2. **Operator:** after the change, make sure that the bridge applied generation is equal to `shaping_generation`.
3. **Owner:** decide if this report goes to LibreQoS. Include sections 2, 3, 7, and 8.3, and remove subscriber identifiers.
4. **LQoSync:** run the live test again after the bridge applies the desired generation. The enrollment (registry revision 3) stays and is not done again.

## 11. Evidence

| Evidence | Path |
|---|---|
| Live test report, run 2 | `/var/backups/lqosync3/live-test/report-2026-11-20-2.json` |
| Live test log, run 2 | `/var/backups/lqosync3/live-test/run-2026-11-20-2.log` |
| Live test backup | `/var/lib/lqosync3/topology-manager/live-test/1795125007` |
| Pre-window state backup | `/var/backups/lqosync3/live-test/pre-window-2026-11-20.tar` |
| Seam state | `/run/libreqos/seam_state.json` (volatile, read on 2026-11-20 06:01) |
| Bridge state reader | Scratch script, not kept in the repository |
| Public source | LibreQoS `main` `828666a37` |
