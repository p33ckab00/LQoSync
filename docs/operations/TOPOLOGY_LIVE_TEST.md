# Topology live acceptance runbook

This runbook proves on production that LibreQoS accepts the Topology Manager pair and applies it correctly.
The test writes the manager pair, reads back what LibreQoS does with it, and puts the legacy pair back. It does not activate the manager.
The window includes the production enrollment. The enrollment registry stays after the window.

Design: [live acceptance design](../superpowers/specs/2026-11-19-topology-live-acceptance-design.md).
Enrollment commands: [enrollment runbook](TOPOLOGY_ENROLLMENT.md).

## Rules

- Claude runs the window only after the owner says "go". Claude asks for the "go" right before step 2.
- Routers stay read-only. LQoSync reads them and never writes them.
- Claude starts `lqosync3.service` again after the window, also after a failure.
- The production service runs an older build. Run the new binary from its build path, for example `target/release/lqosync3`. Do not install it over `/usr/local/bin/lqosync3`.
- Use the configuration `/etc/lqosync3/lqosync3.toml` for every command: add `--config /etc/lqosync3/lqosync3.toml`.
- Run the commands as root. The service runs as `root`.

## Before the window

1. Make sure that the enrollment configuration changes are in place (`require_bound = true` and the temporary `ipoevlan10` exclusion). See the enrollment runbook. Without them, `check` reports a `CRITICAL` line.
2. Build the new binary on the shaper with one build job and a low priority:

   ```bash
   cd /home/pi/LQoSync3.0
   CARGO_BUILD_JOBS=1 nice -n 19 cargo build --release --locked
   LQ=/home/pi/LQoSync3.0/target/release/lqosync3
   ```

   WARNING: The shaper has little free memory. Do not build while free memory is low (`free -m`). A build that uses too much memory can stop processes on the shaper.
3. Choose a fresh path for the report, for example `/var/backups/lqosync3/live-test/report-2026-11-19.json`. Use a new path for each run. An old report at a reused path stays when a run fails early.

## Procedure

### Step 1: Read-only checks and backup

1. Make sure that the service runs and LibreQoS is `ready`:

   ```bash
   systemctl is-active lqosync3
   grep -E '"ready"|source_generation' /opt/libreqos/state/topology/topology_runtime_status.json
   free -m
   ```

   Stop here if the service is not active, `ready` is not `true`, or free memory is low.

2. Make a tar backup of the LQoSync state and of the two legacy files:

   ```bash
   sudo mkdir -p /var/backups/lqosync3/live-test
   sudo tar -cpf /var/backups/lqosync3/live-test/pre-window-2026-11-19.tar \
     /var/lib/lqosync3 /opt/libreqos/src/ShapedDevices.csv /opt/libreqos/src/network.json
   sudo tar -tf /var/backups/lqosync3/live-test/pre-window-2026-11-19.tar | head
   ```

3. Wait for the owner to say "go".

WARNING: Step 2 stops shaping updates. LibreQoS keeps the last pair and keeps forwarding. New subscribers wait until step 6.

### Step 2: Stop the service

```bash
sudo systemctl stop lqosync3
```

### Step 3: Enrollment with the new binary

WARNING: If a command in step 3 fails unexpectedly, the window ends. Go to step 6. The exit status 2 of the dry run in item 4 is expected and is not a failure.

1. Make the policy and run the check:

   ```bash
   sudo $LQ topology policy propose --out /root/topology-policy.json --config /etc/lqosync3/lqosync3.toml
   sudo $LQ topology check --policy /root/topology-policy.json --config /etc/lqosync3/lqosync3.toml
   ```

   Exit status 0 means ready. Exit status 2 means a `CRITICAL` line. Stop the window on exit status 2.

2. Install the policy and make the mapping file:

   ```bash
   sudo $LQ topology policy install --policy /root/topology-policy.json --config /etc/lqosync3/lqosync3.toml
   sudo $LQ topology propose --out /root/topology-mapping.json --config /etc/lqosync3/lqosync3.toml
   ```

3. Review the mapping file as in the enrollment runbook. Show the mapping summary to the owner. Wait until the owner confirms it.

4. Do the dry run. Exit status 2 with "dry run" is correct:

   ```bash
   sudo $LQ topology enroll --mapping /root/topology-mapping.json --config /etc/lqosync3/lqosync3.toml
   ```

5. After the owner confirms, install the enrollment:

   ```bash
   sudo $LQ topology enroll --mapping /root/topology-mapping.json --yes --config /etc/lqosync3/lqosync3.toml
   ```

   The command prints `installed registry revision N`.

### Step 4: Run the live test

WARNING: Do not stop the command while it runs. The manager pair is applied until the restore step ends.

With the default import timeout, the command can run for up to about 10 minutes: the prepare (up to 120 s) and four waits of up to 120 s each (baseline import, manager import, acknowledgement, restore import).
A closed terminal or a tool time limit must not stop the command. Thus run it detached from the terminal, with its output in a log file:

```bash
LOG=/var/backups/lqosync3/live-test/run-2026-11-19.log
sudo setsid -f sh -c "$LQ topology live-test --report /var/backups/lqosync3/live-test/report-2026-11-19.json --import-timeout 120 --config /etc/lqosync3/lqosync3.toml < /dev/null > $LOG 2>&1; echo \"exit status \$?\" >> $LOG"
sudo tail -f $LOG
```

- Use a new log path for each run, as for the report.
- `tail -f` shows the output while the command runs. When the last line is `exit status N`, the command has ended. Stop `tail` with Ctrl-C. This stops only `tail`, not the command.
- To read the result later, run `sudo cat $LOG`. The last line gives the exit status.
- The acknowledgement passes when the bridge applied each identity of the pair. The command checks every circuit and node of the pair.
- `--import-timeout` is the wait for each LibreQoS import, in seconds. The default is 120. The range is 1 to 3600.
- The command writes `live test backup: <dir>` to the log before it writes the manager pair. Keep this path. Backups are in `<state_dir>/topology-manager/live-test/<unix time>/`.
- The command writes the report for each run that gets to the compile step, also when the test fails. It writes the step results, the number of differences and warnings, and the report path to the log.
- When the instance lock, a precondition (enrollment, apply mode, v3 pair), or the prepare step fails, the command writes only the error, gives exit status 1, and writes no report.
- If the command cannot write the report, it writes the summary first and then the error, and gives exit status 1.
- Exit status 0 means `passed`. Exit status 1 means the test failed. The report field `error` holds the first failure.

The command refuses before any write when:

- the instance lock is held (the service runs),
- the installation is not enrolled,
- the installation is not an apply-mode v3 pair,
- planner admission refuses the pair (the report names the nodes),
- parity with the legacy pair differs,
- the manager bytes equal the legacy bytes,
- LibreQoS did not import the legacy pair on disk in the import timeout (baseline),
- the legacy state of LibreQoS differs from the legacy model (baseline).

A refusal writes no pair and requests no import. A baseline refusal keeps the backup folder. Go to step 6.

The command restores the legacy pair by itself, also after a failure.

If the exit status is 1 and the output has no `RESTORE FAILED`, go to step 6.
If the output has `RESTORE FAILED`, or the command stopped early, do step 5.

### Step 5: Restore by hand, only when needed

Do this step if the output contains `RESTORE FAILED`, or if the command stopped before it ended. Use the backup path from `live test backup:`.

```bash
sudo $LQ topology live-test-restore --backup /var/lib/lqosync3/topology-manager/live-test/<unix time> --config /etc/lqosync3/lqosync3.toml
```

The command writes the backup pair byte for byte, requests a LibreQoS import, and waits up to 120 s for that import. It holds the instance lock.
Exit status 0 means that LibreQoS imported the legacy pair and is `ready`. Exit status 1 means that the restore failed or LibreQoS did not confirm the import. The error tells which.

Confirm that LibreQoS imported the legacy pair: `topology_runtime_status.json` shows `ready` and the baseline `source_generation` from the report.
If the restore passes, go to step 6.
If the restore fails, also go to step 6. The legacy service writes the legacy pair at its first cycle, so the start is itself a restore path. In step 6, confirm the pair and report the failure to the owner.

### Step 6: Start the service and check one cycle

The installed service build (`e8da230`) has no `api` command and no API socket. Read its state from `/var/lib/lqosync3/status.json` and from the journal.

```bash
sudo systemctl start lqosync3
systemctl is-active lqosync3
sleep 60   # wait for the first cycle; repeat the next commands until one cycle shows
sudo journalctl -u lqosync3 -n 50 --no-pager
sudo grep -E '^  "(updated|files_changed|reload|error|cycle_seq|trigger|program_version)":' /var/lib/lqosync3/status.json
grep -E '"ready"|source_generation' /opt/libreqos/state/topology/topology_runtime_status.json
sudo grep -oE '^\{"schema_version":[0-9]+,"revision":[0-9]+' /var/lib/lqosync3/topology-manager/registry.json
```

Wait for one good legacy cycle. In `status.json`, a good cycle shows a new `updated` time, `cycle_seq` 1 or more, `error` `null`, and one of these `reload` values: `no change`, `waiting for the LibreQoS import (N s)`, or `imported by LibreQoS`. Any other `reload` text is an error. `files_changed` is `true` when the cycle wrote the pair. LibreQoS shows `ready`.
The registry line shows the `revision` from `installed registry revision N` in step 3. The installed build does not read the registry.

If the step 4 output has a `live test backup: <dir>` line, compare the pair with that backup:

```bash
sudo cmp /opt/libreqos/src/ShapedDevices.csv <dir>/ShapedDevices.csv
sudo cmp /opt/libreqos/src/network.json <dir>/network.json
```

Use the `<dir>` from that line. If the output has no such line, the test wrote no pair, so no restore and no `cmp` are necessary.
The `cmp` check applies only if the service did not write a new pair yet. A new legacy cycle can change the pair.

If the service does not start, or it stays `failed`:

1. Read the cause in `sudo journalctl -u lqosync3 -n 100 --no-pager`.
2. Do the rollback below with the tar backup from step 1. It puts back the state directory from before the window.
3. Start the service again: `sudo systemctl start lqosync3`. Check it as above.
4. Tell the owner at once, also when the second start passes.

### Step 7: Report to the owner

Give the owner the report path, the result (`passed` and `error`), the number of differences and warnings, and the registry revision.
The enrollment registry stays for Phase 4. The tar backup from step 1 allows its removal.

## Stop conditions

Stop the window and go to step 6 when:

- a pre-check in step 1 fails,
- the `check` command in step 3 gives exit status 2,
- the owner does not confirm the mapping,
- `live-test` refuses or fails.

Never repeat `live-test` in the same window after a failure. Find the cause first.

## Messages in the report

| Message | Meaning and action |
|---|---|
| `import timeout` | LibreQoS did not import the pair and become `ready` in the time limit. The test restores the legacy pair. Ask the owner before you try again with a longer `--import-timeout`. |
| `LibreQoS gives the same source generation ... for the new pair` | LibreQoS did not show a new `source_generation`, so the test cannot confirm the import. The test restores the legacy pair. |
| `waiting to take effect: ...` | LibreQoS imported the pair, but the bridge did not apply the listed identities. The test restores the legacy pair. Read `/run/libreqos/seam_state.json`. See the [dataplane settings document](LIBREQOS_V3_DATAPLANE_SETTINGS.md). |
| `baseline: LibreQoS did not import the legacy pair on disk` | LibreQoS did not import the legacy pair on disk before the test. The test wrote no pair. Go to step 6 and tell the owner. |
| `the legacy state before the test differs from the legacy model` | The LibreQoS rule model does not match the state that LibreQoS applied for the legacy pair. The test wrote no pair. The report field `baseline` lists the differences. Go to step 6 and tell the owner. |
| `restore read-back differs from the baseline: ...` | After the restore, these circuits and nodes differ from the state before the test. Do step 5. |
| `RESTORE FAILED ...` | The restore did not finish. Do step 5 with the backup path in the message. |

After an import timeout of the manager pair, the restore can pass at once. A few minutes later, confirm that `topology_runtime_status.json` shows the baseline `source_generation` from the report.

## What the test does not change

The test does not claim writer ownership (`runtime-owner.json` is not written), does not change the registry revision, does not write write-receipt files, and does not send notifications.
The restore writes the backup pair byte for byte, so the old service receipt stays valid.

## Rollback

The enrollment in this window writes the registry under `/var/lib/lqosync3/topology-manager/`.
The installed build (`e8da230`) does not read the registry, so the registry has no effect on it.

NOTE: The next note does not apply to the installed `e8da230`. Keep it for a later rollback from a newer build to an older build that reads the registry: a registry with `catalog.fallback` does not load in that older build. Clear the fallback (`fallback.clear`) before such a rollback.

WARNING: The next steps replace the whole state directory with the backup from step 1.

To remove the enrollment, do these steps:

```bash
sudo systemctl stop lqosync3
sudo rm -rf /var/lib/lqosync3
sudo tar -xpf /var/backups/lqosync3/live-test/pre-window-2026-11-19.tar -C /
sudo systemctl start lqosync3
```
