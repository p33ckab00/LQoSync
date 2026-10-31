---
type: session
created: 2026-10-31
tags: [session, updatecsv, configuration]
status: done
related: ["[[updatecsv-py-dissection]]", "[[ADR-0003-router-failed-vs-offline]]", "[[ADR-0004-toml-policy-python-defaults]]"]
---

# 2026-10-31 — Dissection of `updatecsv.py` and TOML policy

## Goal

Take `updatecsv.py` apart before a new Rust build from scratch. Put all policy values in TOML.

## Changes

- Read `/opt/libreqos/src/updatecsv.py` in full. Read the live `config.json` with the secrets removed from the output.
- Published a private artifact "updatecsv.py Anatomy". Later the watch reported that the artifact does not exist.
- Updated `docs/CONFIGURATION.md`: `[router_access]`, `[speed]`, `[nodes]`, `[circuits]`, `routers.dhcp.enabled`, `require_bound`, `speed_sources`, router state, and subscriber presence.
- Updated `docs/ARCHITECTURE.md`: state store, router state diagram, and change detection.
- Added [[ADR-0003-router-failed-vs-offline]], [[ADR-0004-toml-policy-python-defaults]], and [[updatecsv-py-dissection]].

## Results

- Made a Git repository in `/home/pi/LQoSync3.0` with the orphan branch `lqosync3` and the remote `origin`. Moved `README.md`, `docs/`, `brain/`, and `graphify-out/` into `lqosync3/`. No commit.
- Renamed the tool to `lqosync3` in the documents. The Python unit `lqosync-updatecsv.service` keeps its name.

- The operator decided: a failed router keeps its subscribers. An offline router loses them.
- The operator decided: the speed rules try comments first, then the settings. For DHCP, the server comment comes first.
- `mikrotik-rs` 0.7.0 supports streamed replies and cancel. The RouterOS `listen` command is possible, but not tested.

- Wrote the core design `docs/superpowers/specs/2026-10-31-lqosync3-core-design.md` and Plan 1 `docs/superpowers/plans/2026-10-31-lqosync3-core.md` (8 tasks).
- Checked that Rust `{:.3}` and `{:?}` give the same values as Python `round(x, 3)` and `repr` for rates of 0.001 Mbps and more.

## Open items

- [x] `offline_after = 300` is the default. The unit is seconds (5 minutes). The operator confirmed it.
- [x] The PPPoE list keeps `profile_rate_limit` and `profile_name` after the comment sources. The operator confirmed it.
- [x] Repository: `p33ckab00/LQoSync`, orphan branch `lqosync3`, folder `lqosync3/`. Name `lqosync3` for the crate, binary, and service.
- [ ] Test `listen` on a router.

## Links

- [[updatecsv-py-dissection]]
- [[2026-10-31-project-setup-and-v3-simulation]]
