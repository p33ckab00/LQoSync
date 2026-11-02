---
type: session
created: 2026-11-02
tags: [libreqos, upgrade, rollback]
status: complete
related: ["[[2026-11-01-libreqos-v3-upgrade-investigation]]"]
---

# LibreQoS v3 upgrade preparation

Final result: the live cutover succeeded. Read [[2026-11-02-libreqos-v3-live-cutover]] for the current state.
The sections below retain the preparation and recovery history. Earlier states are historical.

Inspection time: 2026-11-02T12:15:41+08:00 (Asia/Shanghai).
Scope: resume preparation from the previous chat. No package or runtime changes.

## Confirmed host state

- `libreqos` is `2.2.202608181454` and remains held.
- `libreqos-preflight` is `3.0.7.202610310038`.
- `libreqos-api` and `libreqos-dataplane` are absent.
- `lqosd`, `lqos_scheduler`, and `lqosync3` are active.
- Both ConnectX-4 Lx ports remain members of `br0`.
- Both ports use `mlx5_core` and firmware `14.25.1020 (MT_2420110034)`.
- Their PCI device ID is `15b3:1015`.
- The default IPv4 route uses the separate `enp3s0` port.
- A ZeroTier interface also exists. Its complete dependency path remains unverified.
- No hugepages are reserved. Available memory was approximately 6.5 GiB at inspection.
- Protected netplan files require root access. `sudo -n true` requires a password.

The local APT index lists `3.0.7.202610310038` for all three v3 packages.
This inspection did not refresh the APT index.
It lists archive version `2.2.202609081854`, which differs from the installed version.
The APT cache contains no `libreqos` package for an exact rollback.

## Preparation sequence

1. Inspect the protected netplan files through an operator root session.
   Check the management paths, the orientation of the ports, and the plan for bridge adoption.
2. Secure a verified v2 rollback package before any upgrade.
   Prefer the installed version. Treat a different archive version as a separate compatibility check.
3. Prepare a root-only backup of config, netplan, units, package holds, topology, and runtime state.
   Include `/etc/lqos.conf`, `/etc/libreqos`, `/etc/lqosync3`, and `/var/lib/lqosync3`.
   Check the LibreQoS state directory before the backup. Do not copy secrets into this vault.
4. Check the matched v3 artifacts and their exact install and rollback helpers.
   Check firmware support for `15b3:1015` with firmware `14.25.1020`.
5. Review persistent hugepages and memory use before reservation.
   The previous checker proposed 3072 pages of 2 MiB, equivalent to 6 GiB.
   Current free memory does not prove that this allocation will succeed under load.
6. Back up config before a root preflight with `--pair-dir /opt/libreqos/src`.
   The previous investigation confirmed that the loader can write migration files.
7. Prepare the LQoSync transition from explicit `v2_pair` to `v3_pair`.
   Check the current config before a change. Use `source = "shaped_devices"` for the first v3 transition.
8. Schedule the cutover after these checks. Check console access and a rollback deadline.
   During cutover, use shadow mode before the package and topology transition.
9. Check management access, traffic in both directions, v3 services, and topology acceptance.
   Restore bridge adoption state before the packaged rollback if the cutover fails.

This sequence is a preparation checklist. Exact cutover commands remain unfinished.

## Recovered artifacts

The user stated that earlier work already prepared the cutover.
A wider search found these artifacts on 2026-11-02:

- `/home/pi/libreqos-v2-rollback/libreqos_2.2.202609081854_amd64.deb`
- `/tmp/libreqos-v3-review-3h5ymeTW/libreqos_3.0.7.202610310038_amd64.deb`
- `/tmp/libreqos-v3-review-3h5ymeTW/libreqos-dataplane_3.0.7.202610310038_amd64.deb`
- Extracted control and dataplane files under the same temporary directory.

The SHA-256 hashes of all three packages match the local APT metadata.
The v2 artifact differs from the installed version. Exact rollback remains unproven.
The packaged rollback helper accepted this artifact in `--dry-run` mode.
This check proves the plan only. It does not prove runtime restoration.

The matched APT simulation passed: one upgrade, two new packages, and no removals.
No actual installation occurred.
The initial cache check did not cover the rollback directory. Use this wider result.

The user requested the actual cutover.
Root authentication remains unavailable to this agent.
The agent requested a filtered list of relevant paths under `/root` from the operator.
Recover any existing backup and cutover procedure there before new preparation.
Do not treat unreadable root files as absent.

The operator supplied these paths from a root `find` command:

- `/root/libreqos-before-v3-RRb5MEle`
- `/root/libreqos-before-root-preflight-0EN8CBr9`
- `/root/libreqos-before-root-preflight-0EN8CBr9/libreqos`
- `/root/libreqos-before-root-preflight-0EN8CBr9/lqos.conf`

These paths confirm that backup directories exist. Their contents still need inspection.
The operator terminal authenticated sudo. This agent's `sudo -n` still requires a password.
Use the operator terminal for the protected inspection and the eventual cutover commands.

The operator listed the backup files:

- `libreqos-before-v3-RRb5MEle/configs.tar.gz`: 27957 bytes.
- `backup-contents.txt`: 10258 bytes.
- `packages.tsv`, `package-selections.txt`, and `holds.txt`.
- `bridge-links.txt`, `addresses.txt`, and `routes.txt`.
- The separate backup contains `lqos.conf` at 4494 bytes and the prior `/etc/libreqos` example.

The archive member list and a current root preflight are the next checks.
The inspected v3 `preinst` stops the old daemon before its checker gate.
Run the standalone root checker before the package transaction, with the explicit pair directory.
Keep the existing v2 services active until the standalone result supports the cutover.

## Root preflight result and installation stage

The operator supplied the archive list and root preflight output.
The archive contains the LibreQoS config, netplan, service units, LQoSync config and secrets,
APT config, `ShapedDevices.csv`, and `network.json`.
The archive list does not include `/opt/libreqos/state` or `/var/lib/lqosync3`.
Back up these runtime directories before the schema transition.

The root checker returned `WARN`, with no `BLOCK`.
It validated the legacy pair and reported at least 26 circuits.
It explicitly identified `br0` adoption as required before port binding.
Its generic plan selected `uio_pci_generic`. Do not apply that plan manually to these mlx5 ports.
The inspected dataplane bind helper selects `none` for `mlx5_core` and keeps the kernel driver.
The systemd hotfix remains installed at `255.4-1ubuntu9999+libreqos1`.

The live config has `source = "shaped_devices"`, `compile_mode = "ap_only"`,
and dataplane bandwidth of 620 Mbps in both directions.
Review `compile_mode` and the LQoSync target before LQoSync resumes on v3.

Next stage: stop the old writers and daemon, archive both runtime directories,
then install the exact matched control, API, and dataplane packages at `3.0.7.202610310038`.
Set `LQOS_PREFLIGHT_PAIR_DIR=/opt/libreqos/src` for the package gate.
Keep LQoSync stopped during installation and bridge adoption.
Inspect the installation result before adoption, final service activation, or reboot.
The agent has not executed this stage. Root execution must use the operator terminal.

## 2026-11-02T12:32:10+08:00 — packages installed, dataplane failed

The operator executed the installation stage and supplied its complete output.
All three packages now have version `3.0.7.202610310038` and dpkg status `ii`.
The runtime archive command completed before APT started.
The installer reserved 3072 pages of 2 MiB and wrote the persistent sysctl setting.
It migrated config to `/etc/libreqos/lqos.conf` and retained `/etc/lqos.conf`.
It wrote `bridge.conf`, the explicit BDF pair, and `bridge.env.d/50-ports.conf`.
The nested dataplane fetch reported failure, but the outer APT transaction installed the dataplane successfully.

Direct checks show active `libreqos-lqosd`, `libreqos-scheduler`, and `libreqos-api`.
LQoSync remains inactive. `libreqos-bridge` is failed.
Both dataplane ports still belong to the live kernel bridge `br0`.
The bind helper selected `none` for mlx5 and exited before its bridge guard.
The dataplane log also missed the actual upper bridge during its port check.
Do not rely on these checks to protect the existing kernel bridge.

The bridge recognized both PCI ports and the expected 620/620 Mbps rates.
It failed at Tx queue startup with these errors:

```text
mlx5_net: Failed to register unique umem for all SQs.
mlx5_net: port 0 Tx queues memory allocation failed: Protocol not supported
Interface startup failed: Port 0 start failed: Protocol not supported
```

The [DPDK 25.11 driver documentation](https://doc.dpdk.org/guides-25.11/nics/mlx5.html)
defines `txq_mem_algn=0` as legacy allocation with separate memory regions for each queue.
The [version-pinned source](https://github.com/DPDK/dpdk/blob/v25.11/drivers/net/mlx5/mlx5_trigger.c)
skips the exact failed shared-umem function when this value is zero.
The installed PMD contains the `txq_mem_algn` argument.
This supports a compatibility trial. It does not prove that the dataplane will start or forward traffic.

Proposed override: `/etc/libreqos/bridge.env.d/90-mlx5-compat.conf`.
Preserve the managed PMD path and BDF order. Add `txq_mem_algn=0` to both `-a` arguments.
The unit loads later environment files after `50-ports.conf`.
The override does not require a daemon reload. A bridge restart would apply it.
Do not restart the bridge with the ports still in `br0`.
First inspect the root netplan state with the packaged `lqos_netplan_helper inspect` command.
Its CLI supports inspection, but its `apply` command does not expose `ApplyMode::Adopt`.
The authenticated web network-mode flow exposes adoption with a backup ID and a confirmation window.
No compatibility override or network change has yet executed.

## Managed Linux bridge and reversible dataplane trial

The operator installed `90-mlx5-compat.conf`. A direct read confirms its exact contents.
Root inspection reports `ManagedByLibreQoS`, `can_adopt = false`, and no conflicts.
The managed preview retains `br0` with both dataplane ports.
Thus, the existing adoption flow does not remove the kernel bridge for this host.
Do not apply that preview as a DPDK migration.

Prepared `/home/pi/.local/state/libreqos-v3-cutover/trial-dataplane.sh` outside the repository.
It checks the current port masters, the management default route, the absence of global addresses on `br0`,
the compatibility file, and the stopped LQoSync service.
It creates a root-only recovery script inside the existing backup directory.
It arms a five-minute systemd timer before either port leaves `br0`.
It then detaches the two ports and attempts DPDK startup with the override.
A command failure invokes the recovery service immediately.
The timer also restores kernel forwarding if the operator does not confirm the trial.
Recovery does not restore v2 shaping. LQoSync remains stopped.

The trial does not edit netplan. A successful trial still needs persistent network configuration,
topology and LQoSync checks, and runtime acceptance.
Only cancel the timer after actual client traffic works in both directions.

Checks: `bash -n` passed. Execution without root rejected the invocation before any mutation.
The operator has not yet executed the root trial. Runtime forwarding remains untested.

## Successful dataplane trial and persistence preparation

The operator executed the trial. The DPDK bridge started successfully with `txq_mem_algn=0` on both ports.
Logs report `readiness: GREEN`, active download and upload queues, and 26 installed mappings.
The seam reports `healthy`, an applied generation, and no rejection.
The operator confirmed that client internet access works and said they would stop the trial recovery timer.
Check the actual timer state before any later assumption that confirmation completed.

Networkd automatically attached the ports to `br0` again after the trial detached them.
The networkd journal confirms configuration from the old generated netplan files.
Therefore, the successful process trial does not prove that the kernel bridge is absent.
Persistent removal of the `br0` entry remains necessary.

Prepared two local files under `/home/pi/.local/state/libreqos-v3-cutover/`:

- `remove-kernel-bridge.py`: remove only the expected bridge and preserve all other netplan data.
- `persist-dataplane.sh`: back up netplan, arm timed recovery, apply the change, and restart the dataplane.

The Python helper refuses an unexpected interface pair or IP configuration on `br0`.
Its checks prove preservation of management settings and an unrelated bridge, plus rejection of unsafe bridge configurations.
The shell script requires the separate management default route and an inactive LQoSync service.
Its recovery restores the original netplan and kernel forwarding. It does not restore v2 shaping.
The new timer replaces any active trial timer only after its own recovery is armed.
After restart, the script checks that neither port has a master device.
The operator must confirm actual client traffic before cancellation of the new recovery timer.

Checks passed: Python preservation and rejection checks, shell syntax, and shell non-root rejection.
The persistent stage has not executed. LQoSync configuration and restart remain unfinished.

## Persistent dataplane stage completed

The operator executed `persist-dataplane.sh` successfully.
Direct checks confirm that `br0` is absent and neither dataplane port has a bridge master.
The DPDK bridge remains active with the compatibility arguments and the restored LKG.
The management address and route remain on `enp3s0`.
The persistent recovery timer remains active at the latest inspection.
The operator has not yet confirmed client traffic after this second network change.

The operator reported `BRIDGE_SEAM_REPLACED` at 2026-11-02T12:39:45+08:00.
The control-plane log identifies the restart and reattachment.
Subsequent seam state is `healthy`. Applied generation equals desired world content key.
There is no last rejection or applied divergence. Dataplane counters increase in both directions.
Thus, the warning alone does not show an ongoing forwarding failure.

The live LQoSync config remains `mode = "apply"`, `target = "v2_pair"`, and network mode `root_promotion`.
LQoSync remains stopped. Do not start it with the old target.
Prepared the next commands to select `target = "auto"` and `mode = "shadow"`, then run detect, check, once, and diff.
An in-memory TOML comparison proves that these two edits preserve all other parsed config values.
The live config has not changed. The canonical LibreQoS `compile_mode` remains `ap_only` pending review before apply.

## Persistence timer expired before confirmation

At 2026-11-02T12:47:19+08:00, direct checks show the DPDK bridge inactive and both ports in `br0` again.
The recovery timer expired before the operator stopped it. The recovery journal confirms restoration.
The operator then ran the shadow command block. Its first active-service check exited under `set -e`.
That explains the empty output. The LQoSync config remains `apply` and `v2_pair`, with the service stopped.

Updated `persist-dataplane.sh` to allow retry after this recovery.
It permits only active or inactive DPDK state, requires an existing `br0`,
and reuses the earlier netplan backup only when the current file matches it byte for byte.
The recovery window is now fifteen minutes.

Prepared `resume-cutover.sh` to run persistence and the shadow checks in one operator invocation.
Its config edit verifies that only runtime mode and target change.
It prints explicit error information instead of an empty failure.
It leaves LQoSync stopped and reminds the operator to confirm client traffic and stop the timer in the same terminal.
The timer remains the fallback if the operator does not confirm.

Checks passed: both shell syntax checks, embedded Python syntax, netplan helper checks, and non-root rejection.
The revised scripts have not executed as root.

## Resume succeeded and shadow comparison passed

The operator executed `resume-cutover.sh`.
Netplan reported a reconfigure error, then restarted networkd and completed successfully.
The management address remains on `enp3s0`. The DPDK bridge is active and `br0` is absent.
The fifteen-minute recovery timer remains active. Its deadline is 2026-11-02T13:04:08+08:00.

LQoSync now has `runtime.mode = "shadow"` and `libreqos.target = "auto"`.
Detection selects `v3_pair` from the installed v3 package and `source = "shaped_devices"`.
The router scan succeeded. The shadow comparison shows no circuit additions, removals, or updates.
Only two node types change from `plan` to `site`, as expected for the v3 writer.
The plain-API warning is pre-existing. The config-digest warning refers to status from the stopped prior process.
Do not send SIGHUP to an old PID for that warning. The next service start reads the current config.

Prepared `/home/pi/.local/state/libreqos-v3-cutover/apply-lqosync-v3.sh` for the next operator stage.
It requires an active dataplane and no kernel bridge or port master.
It backs up both configs and both live topology files into a new root-only directory.
It sets canonical `compile_mode = "full"` for the `root_promotion` topology and LQoSync mode to `apply`.
It preserves every other parsed TOML value and leaves the source and automatic target unchanged.
After config checks, it cancels the network recovery timer and performs one apply cycle.
It restarts only the control plane to load the topology mode, leaving DPDK active.
It requires healthy matching seam state and fresh ready topology before the LQoSync service starts.
A failure restores both configs and both pair files, restarts the control plane, and leaves LQoSync stopped.

Checks passed: shell syntax, embedded Python syntax, and in-memory preservation of all unrelated config values.
The activation script has not run as root. Confirm client traffic before the operator executes this stage.

## Source check

The [version-pinned upstream summary](https://github.com/LibreQoE/LibreQoS/blob/2d922cb6d6ceb14b554d12b88d3fcbb902af9919/docs/v3/upgrade-from-v2.md)
describes bridge adoption before the ports bind and adoption restoration before rollback.
Its link to `packaging/UPGRADE.md` returned HTTP 404 at the same revision.
Resolve that absent reference against the exact package contents before the cutover.

## Checks and limits

Read-only host checks passed for packages, services, routes, bridge membership, driver, firmware, and memory.
The graph query located the previous investigation and target selection symbols.
No application code changed. No application tests ran.
The project graph was not rebuilt for this session note.
Root netplan inspection, backups, package verification, and runtime hardware validation remain open.
