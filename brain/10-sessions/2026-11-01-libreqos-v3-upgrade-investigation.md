---
type: session
created: 2026-11-01
tags: [libreqos, preflight, investigation]
status: complete
related: ["[[2026-11-01-preflight-update-prepared]]", "[[libreqos-v3-ingress-facts]]"]
---

# LibreQoS v3 upgrade investigation

Date: 2026-11-01 (Asia/Shanghai).
Scope: inspect upstream changes and the local upgrade status. Do not upgrade the dataplane.

## Upstream snapshot

The GitHub main revision is `2d922cb6d6ceb14b554d12b88d3fcbb902af9919`.
Its date is 2026-10-31T08:38:39+08:00.
The commit describes the public source projection for `v3.0.7-beta17`.
The comparison with `3d30a7bc09be` contains 68 changed files.
The updated checker package, `3.0.7.202610310038`, matches this release timestamp.

The v3 dataplane uses a userspace DPDK bridge instead of the v2 kernel shaping stack.
Control and dataplane packages must form a matched set.
The config path changes to `/etc/libreqos/lqos.conf`.
The config uses `[dataplane]` as the authoritative source for bandwidth.
Dynamic CAKE is enabled by default. Virtual nodes and the Python refresh engine are retired.

The latest change also removes `--pre-install` and simplifies several hardware policy checks.
Removed checks do not prove hardware compatibility.

## Local facts

- LibreQoS `2.2.202608181454` remains installed and held.
- The checker package is `3.0.7.202610310038`.
- Ubuntu is 24.04.4. The kernel is `6.8.0-147-generic`.
- The CPU is an i5-9500 with six logical CPUs. The host has approximately 16 GB RAM.
- Two ConnectX-4 Lx ports use `mlx5_core`: `enp1s0f0np0` and `enp1s0f1np1`.
- Both ports remain members of the live Linux bridge `br0`.
- The management IPv4 address uses the separate `enp3s0` interface.
- No hugepages are reserved. The checker describes an allocation of 3072 pages of 2 MiB each.
- The systemd hotfix is installed: `255.4-1ubuntu9999+libreqos1`.
- The current topology pair validates. The checker reports at least 26 circuits.
- LQoSync remains configured with `target = "v2_pair"`.
- `lqosd`, `lqos_scheduler`, and `lqosync3` remain active after the investigation.

## Warning mechanisms

The install-integrity check always checks two v3 paths:
`/usr/libexec/libreqos/bridge_rs` and `/etc/systemd/system/libreqos-bridge.service`.
Neither path exists on this v2 host. The checker reports an unsupported, non-package-owned v3 tree.
The installed v2 daemon path belongs to the `libreqos` package.
Thus, this warning does not prove that the v2 installation lacks package ownership.
The source makes this warning report-only, not a block.

The normal-user run omits the netplan and NIC plan sections.
`strace` shows an `EACCES` error when the loader tries to create `/etc/lqos.conf.topology_source_backup`.
The standalone checker then suppresses the config-load error and skips those checks.
The config loader invokes migrations, despite a comment that describes it as read-only.

An isolated copy confirmed the behavior without changes to the live config.
The checker added `topology.source` to the copy and created migration stamps and a backup.
It then printed the NIC plan and reported no conflicting netplan bridges.
This bridge result is incomplete: the YAML files are mode `0600`, owned by root.

The source silently skips files that it cannot read. The live bridge membership proves an adoption task remains.
Do not run the checker as root without a config backup and review of its migration effects.

## Remaining upgrade checks

Read the protected netplan files through an authorized root session.
Prepare the bridge adoption, persistent hugepages, matched packages, LQoSync target, and rollback steps.
Make sure the packaged dataplane supports the exact Mellanox card and firmware.
The generic NIC plan selected `uio_pci_generic`.
This is not an approved bind plan for these Mellanox ports.
The management IPv4 uses a separate port, but the complete management route still needs review.

## Evidence

Downloaded sources, comparison data, syscall trace, and output are under `/home/pi/.local/state/libreqos-v3-investigation/`.
The isolated config directory is mode `0700`. Its config copy is mode `0600`.
No runtime settings, network state, services, or packages changed during this investigation.

## Package simulation and cleanup

The user requested a simulation against the installed v2.2 system, followed by removal of the temporary files.
The scope was APT simulation, package file comparison, and inspection of installer scripts.
No maintainer script executed. This was not a complete installation or a boot test in a VM.

The default APT transaction selected one upgrade (`libreqos`) and one new package (`libreqos-api`).
The explicit matched transaction added `libreqos-dataplane`, for one upgrade and two new packages.
Both transactions selected version `3.0.7.202610310038`, with no package removals.
The simulation allowed the change of the held package. The actual hold remains in place.
All three downloaded packages passed SHA-256 verification against the archive metadata.
The maintainer scripts passed shell syntax checks.

The installer scripts describe these actions:

- Stop the old daemon before the install gate.
- Attempt a runtime hugepage reservation and create a persistent sysctl file.
- Create `lqosuser` and rebuild the Python virtual environment.
- Replace the old service names with the canonical v3 service names.
- Invoke `update_bridge.sh` to install the separate dataplane package.
- Activate runtime services when the legacy config exists.

The activation source copies `/etc/lqos.conf` to `/etc/libreqos/lqos.conf` when the latter is absent.
It prepares `bridge.conf`, port orientation, bandwidth migration, and the service handoff.
These actions do not prove that the existing Linux bridge will migrate successfully.
The dataplane package includes the mlx5 PMD and its bind helper selects in-place mode for `mlx5_core`.
This provides stronger evidence than the generic checker plan. It does not prove runtime hardware compatibility.

The bare checker did not find the topology pair and reported no topology source.
The explicit `--pair-dir /opt/libreqos/src` check validated the existing pair.
The installer supports `LQOS_PREFLIGHT_PAIR_DIR` for this input.

After the simulation, config, daemon, unit, package versions, and service active states matched the recorded baseline.
The temporary directory used 310 MiB before cleanup.
The downloads, extracted packages, temporary source copies, and logs from this simulation were removed.
Only this summary remains in the project vault.

- [Latest source](https://github.com/LibreQoE/LibreQoS/commit/2d922cb6d6ceb14b554d12b88d3fcbb902af9919)
- [Install guide](https://github.com/LibreQoE/LibreQoS/blob/2d922cb6d6ceb14b554d12b88d3fcbb902af9919/docs/v3/install.md)
- [Upgrade guide](https://github.com/LibreQoE/LibreQoS/blob/2d922cb6d6ceb14b554d12b88d3fcbb902af9919/docs/v3/upgrade-from-v2.md)
- [Integrity check](https://github.com/LibreQoE/LibreQoS/blob/2d922cb6d6ceb14b554d12b88d3fcbb902af9919/src/rust/lqos_preflight/src/install.rs)
- [Standalone checker](https://github.com/LibreQoE/LibreQoS/blob/2d922cb6d6ceb14b554d12b88d3fcbb902af9919/src/rust/lqos_preflight/src/bin/libreqos-v3-preflight.rs)
