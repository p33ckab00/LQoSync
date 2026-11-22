# Bridge structural retry storm runbook

Status: operator runbook. Not sent to LibreQoS.
Date of the event: 2026-11-21 to 2026-11-22 (Asia/Manila).
Shaper: production shaper `libreqos`, packages `libreqos` and `libreqos-dataplane` `3.0.7.202611201638`.

Related documents:

- [LibreQoS v3 dataplane hold report](LIBREQOS_V3_DATAPLANE_HOLD_REPORT.md)
- [LibreQoS v3 dataplane update settings](LIBREQOS_V3_DATAPLANE_SETTINGS.md)

## 1. Summary

The LibreQoS DPDK bridge (`libreqos-bridge.service`, log tag `bridge_wrapper`) can enter a retry loop with no backoff.
In the loop, the bridge tries one structural change again and again.
Each try writes two log lines.
The rate was approximately 100,000 lines each second.

On 2026-11-22, the loop filled the systemd journal to its limit of approximately 4 GB in 3 hours.
journald dropped more than 3.6 million messages in each 30-second interval.
journald also deleted all older logs to make space.
The shaper then restarted without a clean shutdown at 07:52.
The cause of the restart is not known.

## 2. Symptoms

Look for these symptoms:

- The disk use of `/var/log/journal` increases quickly. Many new 128 MB `system@*.journal` files appear in one hour.
- `journalctl --disk-usage` shows approximately 4 GB.
- `systemd-journald` reports `Suppressed <N> messages from libreqos-bridge.service`.
- `rsyslogd` reports `high activity`.
- The bridge log shows these two lines many times with the same `gen=` value:

```text
WARN ... worker_loop.rs:5126: Worker 0: lqos-zxc P3a lap-head structural apply gen=6 failed
  (structural add: leaf Upsert pairing ambiguous or missing); RE-ARMED (pause deferred)
INFO ... worker_loop.rs:4906: lqos-zxc P2/P3a/P3b1 lap-head observe gen=6 ... structural_ok=true move_ok=false ...
```

A precondition can exist before the storm starts.
`lqosd` writes this warning every 2 seconds:

```text
WARN lqosd/src/bridge_broker/mod.rs:2370: structural remove set is not a contiguous suffix of the bridge
slot table — emitted only the removable tail run and FENCED the remainder (held, not spinning;
clears on a structural change) ... held=["7KRKYCVW", "CT2AQRFD", ...]
```

This warning shows that old circuits stay in the bridge slot table.
The next structural change can start the storm.

## 3. Cause

The storm has two parts.
LQoSync causes the input condition.
The bridge has the retry defect.

### 3.1 The input condition from LQoSync

LQoSync gives each circuit a random Circuit ID when the circuit is new.
LQoSync keeps the ID only while the circuit name stays in the LQoSync state.

The default value of `subscriber_grace` is `0` (`config/defaults.toml`).
Thus, LQoSync can remove a subscriber after one cycle without the subscriber.
The removal guard stops a large removal only.
The limits are `max_removed_percent = 20` and `max_removed_count = 50`.

After a shaper restart, the PPPoE subscribers connect again over approximately one minute.
This sequence occurred on 2026-11-22:

| Time | Router rows | LQoSync result |
|---|---|---|
| 07:55:59 | 45 | Guard held 22 removals. |
| 07:56:19 | 56 | Guard held 11 removals. |
| 07:56:39 | 62 | 5 removals were below the guard limit. LQoSync removed 5 circuits. |
| 07:56:59 | 67 | The 5 subscribers connected again. LQoSync added them with new Circuit IDs. |

Most subscribers also got a different IP address from the PPPoE pool.
Some new circuits got an IP address of an old circuit.
Example: `MarilynOliveros` had `10.0.100.237`. After the restart, `LuisFillarca` had `10.0.100.237`.

### 3.2 The effect in lqosd and the bridge

`lqosd` can remove a circuit from the bridge only from the end of the slot table.
The old circuits were not at the end.
Thus, `lqosd` held ("FENCED") 7 old circuits in the slot table.

The bridge then received a structural add.
The add failed with `leaf Upsert pairing ambiguous or missing`.
The bridge re-armed the same change with no delay and no limit.

NOTE: The link between the held circuits and the failed add is a hypothesis.
The `bridge_rs` source is not on the shaper.
The log data agrees with the hypothesis.

## 4. Check for the precondition

Do these checks after each shaper restart and after each large change of subscribers.

1. Count the `lqosd` fence warnings in the current boot:

   ```bash
   journalctl -b 0 -u libreqos-lqosd -o cat | grep -c 'not a contiguous suffix'
   ```

   If the count increases, the precondition exists.

2. Read the held circuit IDs:

   ```bash
   journalctl -u libreqos-lqosd -n 1 -o cat | grep -o 'held=\[[^]]*\]'
   ```

3. Look for the retry storm in the last 5 minutes:

   ```bash
   journalctl -t bridge_wrapper --since '-5 min' -o cat | grep -c 'structural apply gen=.* failed'
   ```

   If the count is more than 100, the storm occurs now. Go to section 5.

4. Look for LQoSync removals after a restart:

   ```bash
   journalctl -b 0 -t lqosync3 -o cat | grep -E 'removal guard|removed [1-9]'
   ```

5. Compare the applied files. LQoSync keeps each applied pair in `/var/lib/lqosync3/applied/<id>/`:

   ```bash
   ls -lt /var/lib/lqosync3/applied/ | head
   diff <(sort /var/lib/lqosync3/applied/<old>/ShapedDevices.csv) <(sort /var/lib/lqosync3/applied/<new>/ShapedDevices.csv)
   ```

   If the same Circuit Name has a new Circuit ID, LQoSync removed and added the subscriber again.

## 5. Stop an active storm

CAUTION: DO NOT DELETE JOURNAL FILES BY HAND. USE `journalctl --vacuum-size`. A manual delete can corrupt the active journal.

1. Measure the free disk space:

   ```bash
   df -h /
   journalctl --disk-usage
   ```

2. If you need the evidence, export the storm window before you vacuum. Use a short window:

   ```bash
   journalctl -t bridge_wrapper --since '<start>' --until '<start + 1 min>' | gzip > ~/bridge-storm-sample.log.gz
   journalctl -u libreqos-lqosd -u lqosync3 --since '<start - 30 min>' | gzip > ~/bridge-storm-context.log.gz
   ```

3. Release the journal space:

   ```bash
   sudo journalctl --vacuum-size=500M
   ```

   NOTE: `--vacuum-time=2d` does not help during a storm. All files are newer than 2 days.

4. Restart the shaper services in a quiet period. This step clears the held circuits from the slot table.

   CAUTION: THIS STEP STOPS SHAPING AND FORWARDING FOR A SHORT TIME. TELL THE SUBSCRIBERS OR USE A MAINTENANCE WINDOW.

   ```bash
   sudo systemctl restart libreqos-bridge libreqos-lqosd
   ```

5. Do step 6.1 before step 4 if possible. Without a grace time, LQoSync makes the same condition again in approximately one minute.

6. Do the checks in section 4 again. Make sure that the fence warning count stays at 0.

## 6. Prevent a new storm

### 6.1 Set a grace time in LQoSync

This change has the highest priority.
A grace time keeps a subscriber and its Circuit ID during a short disconnect or a shaper restart.

1. Open `/etc/lqosync3/lqosync3.toml`.
2. Add this line in the `[runtime]` section:

   ```toml
   subscriber_grace = 600   # seconds; keeps the Circuit ID during a short disconnect or a restart
   ```

3. Restart LQoSync:

   ```bash
   sudo systemctl restart lqosync3
   ```

4. Make sure that LQoSync started without a config error:

   ```bash
   journalctl -u lqosync3 -n 20 -o cat
   ```

NOTE: A removed subscriber now stays in `ShapedDevices.csv` for 10 minutes after the last session. Use a different value if your policy needs it.

### 6.2 Limit the journal size and the bridge log rate

These settings do not stop the storm.
They protect the disk if the storm occurs again.

```bash
sudo mkdir -p /etc/systemd/journald.conf.d /etc/systemd/system/libreqos-bridge.service.d
printf '[Journal]\nSystemMaxUse=1G\n' | sudo tee /etc/systemd/journald.conf.d/size.conf
printf '[Service]\nLogRateLimitIntervalSec=30s\nLogRateLimitBurst=500\n' | sudo tee /etc/systemd/system/libreqos-bridge.service.d/ratelimit.conf
sudo systemctl daemon-reload
sudo systemctl restart systemd-journald
```

The bridge rate limit starts at the next restart of `libreqos-bridge.service`.

### 6.3 Report the bridge defect to LibreQoS

Send these facts to the LibreQoS developers:

- The bridge re-arms a failed structural apply with no backoff and no retry limit.
- Each try writes one `INFO` line and one `WARN` line with no rate limit.
- The failure reason is `structural add: leaf Upsert pairing ambiguous or missing`.
- The failure occurred while `lqosd` held non-suffix removals (`NON_SUFFIX_STRUCTURAL_REMOVE`, `lqos-74t.54`).
- The log tag is `lqos-zxc`. The package version is `3.0.7.202611201638`.
- Attach the files from step 5.2.

## 7. LibreQoS reload and the risk

| Action | Structural change | Risk |
|---|---|---|
| Reload with the same `ShapedDevices.csv` and `network.json` | No | Low. LQoSync reports `reload: no change`. |
| Reload after a subscriber joins or leaves | Yes | High while `lqosd` holds old circuits. |
| Restart of `libreqos-bridge` and `libreqos-lqosd` | Full rebuild | Low for the rebuild. High one minute later if `subscriber_grace = 0`. |
| Shaper reboot | Full rebuild and PPPoE reconnects | High if `subscriber_grace = 0`. |

## 8. Open items

- The cause of the restart without a clean shutdown at 2026-11-22 07:52 is not known.
- The start time of the first storm is not known. journald deleted the logs before 2026-11-21 19:51.
- The link between the held circuits and the failed add needs confirmation from the `bridge_rs` source.
- `lqosd` used 1.7 GB of swap in the previous boot. Nobody examined this.
