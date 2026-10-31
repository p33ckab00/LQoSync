---
type: session
created: 2026-10-31
tags: [session, libreqos-v3, setup]
status: done
related: ["[[ADR-0001-one-model-many-writers]]", "[[ADR-0002-v3-target-selection]]", "[[libreqos-v3-ingress-facts]]", "[[v3-offline-simulation-method]]"]
---

# 2026-10-31 — Project setup and LibreQoS v3 simulation

## Goal

Prepare a Rust replacement for `/opt/libreqos/src/updatecsv.py`. The replacement must operate on LibreQoS 2.2 and on LibreQoS 3.x. The settings go in TOML.

## Changes

- Made the project directory `/home/pi/LQoSync3.0`.
- Made the vault `brain/` and the documents in `docs/`.
- Made no change on the production shaper. Read `updatecsv.py`, the LibreQoS 2.2 files, and `/etc/lqos.conf` only.

## Results

- The installed LibreQoS is `2.2.202608181454` (package on hold). The beta candidate is `3.0.7`.
- Downloaded and extracted `libreqos_3.0.7.202610310038_amd64.deb` to a scratch directory. The SHA-256 value agreed with the index. The package was not installed.
- `libreqos-v3-preflight --pair-dir <copy of live pair>` gave `PASS` and `source=legacy-pair`.
- An offline simulation with the v3 Python binding compiled the live pair and a converted `libreqos_network.json`. Both gave the same 27 devices. Only the device comment was different.
- The operator selected option A: one model, one writer for each target. Refer to [[ADR-0001-one-model-many-writers]].

## Open items

- [ ] Select the speed rule behavior: the `updatecsv.py` behavior, or the RouterOS rate direction and units from `/opt/LQoSync3.0/v3/SPEED_RULES.md`.
- [ ] Select the location of the crate.
- [ ] Find the path that `lqosd` watches for `libreqos_network.json`.
- [ ] Test the v3 watcher on a test shaper. Then accept [[ADR-0002-v3-target-selection]].
- [ ] Select the repository URL for the one-line installer.
- [ ] Delete the copy of the live pair in the session scratch directory.

## Links

- [[libreqos-v3-ingress-facts]]
- [[v3-offline-simulation-method]]
