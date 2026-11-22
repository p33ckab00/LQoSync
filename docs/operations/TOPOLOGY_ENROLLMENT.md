# Topology Manager enrollment runbook

This runbook installs the first Topology Manager registry from the current legacy deployment.
Enrollment installs policy only. The legacy loop continues to write `ShapedDevices.csv` and `network.json` until Phase 4 activation.
Do these steps in a maintenance window. Steps 4 to 7 need the service to be stopped. Step 2 restarts the service with the new configuration.

Design: [enrollment CLI design](../superpowers/specs/2026-11-18-topology-enrollment-cli-design.md).

## Before the window

1. Install the lqosync3 release that contains the `lqosync3 topology` commands.
2. Make sure that each PPP account has one active session only. An account with two or more current sessions stops enrollment.
3. Make sure that the router configuration has no `[[routers.dhcp.clients]]` entries. The first policy proposal does not convert DHCP client overrides.

## Configuration changes for enrollment

The manager reads only DHCP leases with status `bound`. The legacy default `require_bound = false` also shapes leases with other status values.
Enrollment needs this value for each router:

```toml
[routers.dhcp]
require_bound = true
```

WARNING: After this change, the legacy loop stops shaping DHCP leases that are not `bound`. These leases are normally offline clients. The owner approved this change on 2026-11-18.

A selected DHCP server can have bound leases that are not in the legacy files. Production server `ipoevlan10` is an example on 2026-11-18.
`topology check` reports these accounts as "enter after activation". Enrollment cannot adopt them.
For enrollment, add such a server to the router exclusions:

```toml
[routers.exclude]
dhcp_servers = ["ipoevlan10"]
```

The legacy files have no rows for that server, so live shaping does not change. Remove the exclusion after Phase 4 activation.

## Procedure

1. Stop the service. Back up the state directory, the configuration, and the live LibreQoS pair:

   ```bash
   sudo systemctl stop lqosync3
   sudo mkdir -p /var/backups/lqosync3/pre-enroll
   sudo cp -a /var/lib/lqosync3 /var/backups/lqosync3/pre-enroll/state
   sudo cp -a /etc/lqosync3/lqosync3.toml /var/backups/lqosync3/pre-enroll/
   sudo cp -a /opt/libreqos/src/ShapedDevices.csv /opt/libreqos/src/network.json /var/backups/lqosync3/pre-enroll/
   ```

2. Make the configuration changes in the section above. Start the service. Wait for one complete cycle, so that the legacy files use the new configuration:

   ```bash
   sudo systemctl start lqosync3
   sudo journalctl -u lqosync3 -f
   ```

   A cycle line with `reload: imported by LibreQoS` or `files changed: false` shows a complete cycle.

3. Make the draft policy and run the readiness check while the service runs. These commands take no lock:

   ```bash
   sudo lqosync3 topology policy propose --out /root/topology-policy.json
   sudo lqosync3 topology check --policy /root/topology-policy.json
   ```

   Review the policy file. It contains the catalog (routers, group nodes, rates) and the source selections with their identity rules.
   The first proposal makes a group only for an active source. These sources are active: a PPP profile with an active session, and a DHCP server with a shaped lease. A router with an active Hotspot session gets one Hotspot group. The legacy tree has the same nodes, so parity passes. The source selection keeps every profile and every server. After enrollment, the service adds the group of a new active source in each cycle.
   Exit status 0 means ready. Exit status 2 means a `CRITICAL` line. Correct each `CRITICAL` line, then run the check again.
   A `WARN` line for a locally administered DHCP MAC is not a blocker. A random MAC can give a new identity when it changes.
   A `rounded node` line shows a node capacity that the manager exports in whole Mbps, for example 225.75 Mbps to 225 Mbps.

4. Stop the service:

   ```bash
   sudo systemctl stop lqosync3
   ```

5. Install the policy, then make the mapping file:

   ```bash
   sudo lqosync3 topology policy install --policy /root/topology-policy.json
   sudo lqosync3 topology propose --out /root/topology-mapping.json
   ```

6. Review `/root/topology-mapping.json`. For each legacy row, set one decision:
   - `map`: confirm the suggested account. Set `"confirmed": true`.
   - `protect`: keep the row as a static row. Rows with the static comment get this suggestion.
   - `drop`: leave an offline or grace row out of the registry. A dropped row reconnects as a new subscriber after activation. Do not drop a row whose subscriber is online.

   Set `"confirmed": true` for each node mapping. Then do a dry run:

   ```bash
   sudo lqosync3 topology enroll --mapping /root/topology-mapping.json
   ```

   Exit status 2 with "dry run" is correct.
   A drift report means that the router data changed after the legacy files were written, for example a reconnect with a new IP.
   With the service stopped, the legacy files do not change, so a new proposal gives the same drift. Do these steps:

   ```bash
   sudo systemctl start lqosync3      # wait for one complete cycle
   sudo systemctl stop lqosync3
   sudo lqosync3 topology propose --previous /root/topology-mapping.json --out /root/topology-mapping-2.json
   ```

   Confirmed decisions for the same legacy rows stay in the new file. Do the dry run again with the new file.

7. Install the enrollment:

   ```bash
   sudo lqosync3 topology enroll --mapping /root/topology-mapping.json --yes
   ```

   The command prints `installed registry revision N`.

8. Start the service and make sure that the legacy loop runs as before:

   ```bash
   sudo systemctl start lqosync3
   sudo journalctl -u lqosync3 -f
   sudo lqosync3 api status
   ```

   `lqosync3 api status` must show the new `registry_revision` and `"enrolled": true`.

## Enroll again before activation

Use this procedure when a legacy Circuit ID changed after the enrollment and before activation.
For example, a client got a new random Circuit ID.
The new ID has no alias, so a normal `enroll` fails with "legacy ownership is already configured".

The `--replace` option makes a new mapping and replaces the aliases. The registry keeps the subscribers, the catalog, and the source settings.
The static source must be the same as at the first enrollment. The command refuses if it changed.

1. Stop the service, as for the first enrollment:

   ```bash
   sudo systemctl stop lqosync3
   ```

2. Make a new mapping file:

   ```bash
   sudo lqosync3 topology propose --replace --out /root/topology-mapping-3.json
   ```

3. Review the file and set each decision, as in step 6 of the procedure. Then do a dry run:

   ```bash
   sudo lqosync3 topology enroll --replace --mapping /root/topology-mapping-3.json
   ```

   Exit status 2 with "dry run" is correct.

4. Install the new enrollment. The registry revision increases by one:

   ```bash
   sudo lqosync3 topology enroll --replace --mapping /root/topology-mapping-3.json --yes
   ```

5. Start the service, as in step 8 of the procedure.

Any edit that changes the output compared with the legacy pair blocks `--replace`. Examples are a plan or a pin that you set on a subscriber.
The command then prints a parity error, for example `legacy managed policy parity conflict`, `ChangedRow`, `ChangedNetwork`, `BaseAuthority`, or `SubscriberBounds`.
If you see one of these errors, do these steps:

1. Write down each plan and pin assignment.
2. Remove the assignments.
3. Enroll again with `--replace`.
4. Set the assignments again.

> **NOTE:** After activation, the manager keeps stable IDs. Then `--replace` fails with the error text "the Topology Manager is active; re-enrollment is not allowed".
> `--replace` also fails on a registry that is not enrolled.

## Effects after Phase 4 activation

These effects do not occur at enrollment. They occur when the manager writer is activated:

- Node capacities are exported in whole Mbps. Each `rounded node` line shows the change.
- The `Comment` column in `ShapedDevices.csv` becomes empty. LibreQoS does not use this column for shaping.
- Remove the temporary `[routers.exclude] dhcp_servers` entry. The `ipoevlan10` clients then enter through normal manager collection.

## Rollback

Do these steps to remove the enrollment and the configuration changes:

```bash
sudo systemctl stop lqosync3
sudo rm -rf /var/lib/lqosync3
sudo cp -a /var/backups/lqosync3/pre-enroll/state /var/lib/lqosync3
sudo cp -a /var/backups/lqosync3/pre-enroll/lqosync3.toml /etc/lqosync3/lqosync3.toml
sudo systemctl start lqosync3
```

Enrollment does not change the live LibreQoS pair. Restore the pair from the backup only if another action changed it.
