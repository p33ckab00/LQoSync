---
type: session
created: 2026-11-02
tags: [libreqos, cutover, operations]
status: complete
related: ["[[2026-11-02-v3-upgrade-preparation]]", "[[ADR-0005-topology-follows-v3]]"]
---

# LibreQoS v3 live cutover

Verified at 2026-11-02T13:04:36+08:00 (Asia/Shanghai).
The live cutover succeeded. A reboot test remains outside the completed runtime checks.

## Installed and active state

- `libreqos`, `libreqos-api`, and `libreqos-dataplane`: `3.0.7.202610310038`.
- Active and enabled: `libreqos-bridge`, `libreqos-lqosd`, `libreqos-scheduler`, `libreqos-api`, and `lqosync3`.
- LQoSync: `mode = "apply"`, `target = "auto"`, detected target `v3_pair`.
- Canonical config: `/etc/libreqos/lqos.conf`.
- Topology: `source = "shaped_devices"`, `compile_mode = "full"`.
- Bandwidth: 620 Mbps in each direction.
- Hugepages: 3072 pages of 2 MiB, with persistent config in `/etc/sysctl.d/80-libreqos-hugepages.conf`.
- Management remains on the separate `enp3s0` interface.

## Dataplane compatibility

The ConnectX-4 Lx pair retains `mlx5_core`.
Both EAL device arguments require `txq_mem_algn=0` to avoid the shared Tx umem registration failure.
The persistent override is `/etc/libreqos/bridge.env.d/90-mlx5-compat.conf`.
Do not replace the managed `50-ports.conf` to retain this override.

The persistent netplan config has no `br0` entry.
Neither dataplane port has a bridge master. The live `br0` device is absent.
All temporary cutover recovery timers are inactive and absent from the timer list.

## Acceptance evidence

The shadow comparison showed no circuit additions, removals, or updates.
Two node types changed from `plan` to `site`, as required by the v3 writer.
The apply cycle completed without a removal hold.
The activation script verified a fresh ready topology and healthy matching applied state before service start.
The daemon then completed repeated router scans without file changes or errors.
The current status has no sync error or config error.

The final seam is `healthy`, with matching applied and desired world content keys.
It has no last rejection and no applied divergence.
The dataplane reports 26 mappings and traffic in both directions.
The operator confirmed client internet during the initial dataplane trial.
No reboot occurred during this session.

## Netplan editor limitation

The web Network page shows shaping active and Bridge Status `Conflict`.
The helper models `[bridge]` as a Linux bridge and expects the selected ports in `br0`.
The DPDK config deliberately keeps those ports outside a kernel bridge.
Thus, this badge reflects the editor model rather than the observed forwarding state.

Do not use `Take Over`, `Apply Network Changes`, or the managed preview to clear this badge.
The inspected preview recreates `br0` with the dataplane ports.
Review upstream DPDK support in the netplan editor before future network changes through that page.
No upstream UI file was modified.

The operator also reported the sidebar `Update Available` indicator.
A fresh HTTPS fetch of the beta repository index lists `3.0.7.202610310038` for all three runtime packages.
The installed versions match that index. The compressed index hash matches the freshly fetched InRelease metadata.
This inspection did not refresh system APT lists or install packages.
The [version-check source](https://github.com/LibreQoE/LibreQoS/blob/2d922cb6d6ceb14b554d12b88d3fcbb902af9919/src/rust/lqosd/src/version_checks.rs)
obtains a separate Boolean from the Insight service and caches it for twelve hours.
It does not compare the installed Debian version with the beta repository candidate.
The sidebar indicator therefore does not establish that a newer beta package is available.

## Backups and remaining checks

- Original config and network snapshots: `/root/libreqos-before-v3-RRb5MEle`.
- Before root preflight: `/root/libreqos-before-root-preflight-0EN8CBr9`.
- Before the sync apply: `/root/libreqos-before-v3-RRb5MEle/sync-apply-FLarX9AD`.
- V2 archive artifact: `/home/pi/libreqos-v2-rollback/libreqos_2.2.202609081854_amd64.deb`.

The v2 artifact differs from the original installed version. Runtime rollback to v2 remains untested.
The explicit upgrade cleared the earlier `libreqos` hold.
The operator still needs to run `sudo apt-mark hold libreqos libreqos-api libreqos-dataplane` to retain a matched set.
The separate systemd package holds remain unchanged.
Boot persistence has configuration and enabled units, but no reboot proof.
The pre-existing plain RouterOS API warning remains.

Only project notes changed in the repository. No application code or tests changed.
Local cutover scripts remain under `/home/pi/.local/state/libreqos-v3-cutover/`.
Config preservation checks, script syntax checks, runtime acceptance, and `git diff --check` passed.
The project graph was not rebuilt for these session notes.

## Input orchestration check

The user requested a compatibility and ownership check after cutover.
Read-only inspection confirms that LQoSync writes both source files in `v3_pair` mode.
The CSV has 15 columns and 26 circuits. The JSON has five site nodes with unique IDs.
All 26 CSV parent names and IDs resolve to those nodes.
The imported JSON tree equals the live source tree. Imported circuit IDs equal the live CSV IDs.
The LibreQoS operator view contains 26 circuit nodes and five site nodes.
The source remains `shaped_devices`, compile mode is `full`, and the topology is ready.
LQoSync has no current sync or config error.

Updated the ingress reference and knowledge note to distinguish source ownership from generated runtime state.
No live files, config, or services changed during this inspection. No live subscriber mutation test ran.
