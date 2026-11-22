# Topology Manager activation: cutover window

Status: procedure for the production window. Version: 2026-11-22.

After activation, the Topology Manager is the only writer of `network.json` and `ShapedDevices.csv`. The registry holds the topology and the stable IDs of each subscriber. The legacy writer stops. Design: `docs/superpowers/specs/2026-11-20-topology-activation-design.md`.

## What changes

- `topology activate --yes` writes `/var/lib/lqosync3/runtime-owner.json`. Then each cycle writes only the manager pair.
- If the manager fails in a cycle, the service keeps the last good pair. It sets `manager_failed`, sends one Telegram notice, and tries again in the next cycle. The first good cycle clears the failure and sends one notice.
- A router read failure is not a manager failure. The router status shows it.
- `topology deactivate --yes` puts the legacy writer back. It keeps the manager IDs of known subscribers.

## Before the window

You need these items:

- Root access on the LQoSync host (`sudo` needs a password).
- The new release binary and its SHA-256 hash from the release build record.
- A time with low traffic. The window takes about 30 minutes.
- The owner decision on the routing lists. This procedure does not turn on the routing list writer. Do that later, with `docs/operations/WAN_ROUTING_LISTS.md`, when the manager is stable.

> **CAUTION:** The new release changes the registry schema to 8 when it opens the registry. An older release cannot read schema 8. Make the backup in step 1 before the new release starts. A rollback to an older release needs this backup.

## Step 1: Backup

Do these commands as root:

```sh
B=/var/backups/lqosync3/activation-$(date +%Y%m%d%H%M)
install -d -m 0700 "$B"
systemctl stop lqosync3
cp -a /usr/local/bin/lqosync3 /etc/systemd/system/lqosync3.service "$B"/
cp -a /etc/lqosync3 "$B"/etc-lqosync3
cp -a /var/lib/lqosync3 "$B"/state
cp -a /opt/libreqos/src/network.json /opt/libreqos/src/ShapedDevices.csv "$B"/
```

Check: `ls -l "$B"` shows the binary `lqosync3`, the unit file, `etc-lqosync3`, `state`, and the two pair files. Write down the value of `$B`.

## Step 2: Install the new release

1. Make sure that the hash of the new binary is the hash in the release build record: `sha256sum <new binary>`.
2. Install it: `install -m 0755 <new binary> /usr/local/bin/lqosync3`.
3. Check the configuration: `lqosync3 check --config /etc/lqosync3/lqosync3.toml`.

Check: `lqosync3 version` shows the new release. `lqosync3 check` shows no error.

## Step 3: Shadow check with the legacy writer

1. Start the service: `systemctl start lqosync3`. The legacy writer still writes.
2. Wait for two cycles (about four minutes).
3. Run `lqosync3 api status --config /etc/lqosync3/lqosync3.toml`.

Check: `writer` is `legacy`, `enrolled` is `true`, and the TUI status screen shows no cycle error.

## Step 4: Fresh live test

The activation gate needs a live test report of the current registry revision.

1. Stop the service: `systemctl stop lqosync3`.
2. Run the live test:

   ```sh
   lqosync3 topology live-test --report "$B"/live-test.json --config /etc/lqosync3/lqosync3.toml
   ```

Check: the exit status is 0 (`echo $?`). The report has `"passed": true` and a `registry_revision`.

If the test fails, stop here. Start the service: the legacy writer continues. Keep the report for the analysis.

## Step 5: Activation dry run

```sh
lqosync3 topology activate --report "$B"/live-test.json --config /etc/lqosync3/lqosync3.toml
```

Check: the command prints `dry run: every activation gate passed; no file changed` and gives exit status 2. Read the lines about rounded node capacities and multi-identity subscribers.

If a gate refuses, read the text and do this:

| Text | Action |
|---|---|
| `legacy row ... has no alias` | Run `topology propose --replace` and `topology enroll --replace`, then go back to step 4 |
| `the manager pair differs from the legacy pair` | Stop. Start the service. Analyze the differences first |
| `the LibreQoS compiler check failed` | Make sure that LibreQoS is ready (`systemctl status lqosd`), then try again |
| `the live test report does not prove ...` | Go back to step 4. The registry changed after the test |

## Step 6: Activation

```sh
lqosync3 topology activate --report "$B"/live-test.json --yes --config /etc/lqosync3/lqosync3.toml
systemctl start lqosync3
```

Check: the command prints `activated: the Topology Manager is the writer`. The file `/var/lib/lqosync3/runtime-owner.json` exists.

## Step 7: Observe three cycles

Watch the TUI status screen, or run `lqosync3 api status` after each cycle.

Check, for each of three cycles:

- `writer` is `topology`.
- `manager_failed` is `false`. The TUI shows no `manager failed` line.
- The cycle has no error, and the health report shows `apply/writer` HEALTHY.
- The write receipt goes to `APPLIED` when the pair changes.

Do not stop LibreQoS services to cause a failure.

## Step 8: Record

Write the result to a deploy record next to the backup (`$B/README.md`). Include the release, the time, the registry revision, the cycle numbers that you observed, and the backup path.

## Rollback A: back to the legacy writer

Use this rollback when the manager fails for a long time, or when the owner asks.

1. Stop the service: `systemctl stop lqosync3`.
2. Do a dry run: `lqosync3 topology deactivate --config /etc/lqosync3/lqosync3.toml`. Read the counts. A subscriber with more than one device gets new legacy IDs.
3. Deactivate: `lqosync3 topology deactivate --yes --config /etc/lqosync3/lqosync3.toml`.
4. Start the service: `systemctl start lqosync3`.

Check: the command prints `deactivated: the legacy writer is the writer`. `runtime-owner.json` does not exist. The next cycle shows `writer` `legacy`, and the subscribers keep their Circuit IDs.

`deactivate` reads all subscriber routers. If it cannot read a router, it stops and changes nothing. Repair the router access, then try again.

## Rollback B: back to the older release

1. Do rollback A first.
2. Stop the service: `systemctl stop lqosync3`.
3. Copy back the old binary and the state directory:

   ```sh
   install -m 0755 "$B"/lqosync3 /usr/local/bin/lqosync3
   rm -rf /var/lib/lqosync3 && cp -a "$B"/state /var/lib/lqosync3
   ```

4. Start the service: `systemctl start lqosync3`.

Check: `lqosync3 version` shows the old release. One cycle has no error.

> **CAUTION:** The state directory from the backup does not have the changes after the backup. The legacy writer then gives new IDs to subscribers that came after the backup.
