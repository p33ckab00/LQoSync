# Topology Manager enrollment runbook

This runbook installs the first Topology Manager registry from the current legacy deployment.
Enrollment installs policy only. The legacy loop continues to write `ShapedDevices.csv` and `network.json` until Phase 4 activation.
Do these steps in a maintenance window. Steps 5 to 7 need the service to be stopped.

Design: [enrollment CLI design](../superpowers/specs/2026-11-18-topology-enrollment-cli-design.md).

## Before the window

1. Install the lqosync3 release that contains the `lqosync3 topology` commands.
2. Make sure that each PPP account has one active session only. An account with two or more current sessions stops enrollment.
3. Make sure that the router configuration has no `[[routers.dhcp.clients]]` entries. The first policy proposal does not convert DHCP client overrides.

## Configuration changes before enrollment

The manager reads only DHCP leases with status `bound`. The legacy default `require_bound = false` also shapes leases with other status values.
Set this value for each router in `/etc/lqosync3/lqosync3.toml`:

```toml
[routers.dhcp]
require_bound = true
```

WARNING: After this change, the legacy loop stops shaping DHCP leases that are not `bound`. These leases are normally offline clients.

A selected DHCP server can have bound leases that are not in the legacy files. Production server `ipoevlan10` is an example on 2026-11-18.
`topology check` reports these accounts as "enter after activation". Enrollment cannot adopt them.
For enrollment, add such a server to the router exclusions:

```toml
[routers.exclude]
dhcp_servers = ["ipoevlan10"]
```

The legacy files have no rows for that server, so live shaping does not change. Remove the exclusion after Phase 4 activation.

## Procedure

1. Back up the state directory and the live LibreQoS pair:

   ```bash
   sudo mkdir -p /var/backups/lqosync3/pre-enroll
   sudo cp -a /var/lib/lqosync3 /var/backups/lqosync3/pre-enroll/state
   sudo cp -a /opt/libreqos/src/ShapedDevices.csv /opt/libreqos/src/network.json /var/backups/lqosync3/pre-enroll/
   ```

2. Make the draft policy while the service runs. The command takes no lock:

   ```bash
   sudo lqosync3 topology policy propose --out /root/topology-policy.json
   ```

   Review the file. It contains the catalog (routers, group nodes, rates) and the source selections with their identity rules.

3. Run the readiness check while the service runs. The command takes no lock and writes no file:

   ```bash
   sudo lqosync3 topology check --policy /root/topology-policy.json
   ```

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
   - `protect`: keep the row as a static row.
   - `drop`: leave an offline or grace row out of the registry. A dropped row reconnects as a new subscriber after activation.

   Set `"confirmed": true` for each node mapping. Then do a dry run:

   ```bash
   sudo lqosync3 topology enroll --mapping /root/topology-mapping.json
   ```

   Exit status 2 with "dry run" is correct. A drift report means that data changed. Then run `topology propose --previous /root/topology-mapping.json --out /root/topology-mapping-2.json`. Confirmed decisions for the same legacy rows stay in the new file.

7. Install the enrollment:

   ```bash
   sudo lqosync3 topology enroll --mapping /root/topology-mapping.json --yes
   ```

   The command prints `installed registry revision N`.

8. Start the service and make sure that the legacy loop runs as before:

   ```bash
   sudo systemctl start lqosync3
   sudo journalctl -u lqosync3 -f
   ```

## Effects after Phase 4 activation

These effects do not occur at enrollment. They occur when the manager writer is activated:

- Node capacities are exported in whole Mbps. Each `rounded node` line shows the change.
- The `Comment` column in `ShapedDevices.csv` becomes empty. LibreQoS does not use this column for shaping.
- Remove the temporary `[routers.exclude] dhcp_servers` entry. The `ipoevlan10` clients then enter through normal manager collection.

## Rollback

Do these steps while the service is stopped:

```bash
sudo systemctl stop lqosync3
sudo rm -rf /var/lib/lqosync3
sudo cp -a /var/backups/lqosync3/pre-enroll/state /var/lib/lqosync3
sudo systemctl start lqosync3
```

Enrollment does not change the live LibreQoS pair. Restore it from the backup only if another action changed it.
