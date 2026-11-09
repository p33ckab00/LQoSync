---
type: session
created: 2026-11-09
tags: [collection, tui, subscriber-state]
status: complete
related: ["[[2026-11-09-tui-release-review]]"]
---

# Cycle collection and subscriber state

Updated: 2026-11-09T19:37:05+08:00 (Asia/Shanghai).

The owner reported missing PPPoE, Hotspot, DHCP collection counts and states.
The owner selected refresh with the current sync cycle. Do not add a periodic monitor or change scan intervals.
Root cause: Dashboard/Router views used shorthand count keys, while status rows use actual RouterOS table paths.
Subscribers only read shaping CSV rows and had no connection-state read model.

Added `src/collection.rs`: sanitized cycle observations, source flags/counts, account/session and lease states, bounded read/write.
Runner publishes `collection.json` from the same RouterOS results. Names/IP/MAC/state are public; passwords/tokens/comments are omitted.
Added an optional same-cycle Hotspot account name/disabled read for enabled sources. Inventory failure cannot block shaping reads.
PPP/local Hotspot inactive accounts supply known offline state; active RADIUS users can appear without local accounts.
DHCP BOUND is a lease state, not proof of device reachability. Disabled sources remain OFF/DISABLED.

Added `src/tui/presence.rs`: cached subscriber projection. Observed-only accounts never enter shaping inputs or source previews.
Identity matches include source/router; DHCP joins use MAC/address and reject ambiguous endpoints.
Evidence expires by collection time and rejects an older process/cycle. Failed-router observations are stale, never false offline.
Dashboard retains five cards and shows Hotspot active count in the Routers card note.
Subscribers shows state, observation time, collected-only records, and working state filters.
Router counts now use real table paths. The production Hotspot source was already disabled; no source was enabled by this task.

Regression proof covers raw counts/state, private-field omission, cache bounds/mode/version, failed/disabled sources, cycle refresh, cache expiry,
process/cycle identity, subscriber/shaping separation, ambiguous DHCP identity, and optional Hotspot read failure.
The temporary 50,000-observation / 25,000-shaped debug benchmark measured 845.95 ms initial projection and 750.40 ms next-cycle projection.
Unchanged snapshots averaged 0.0013 ms. Draw means were 2.026–7.710 ms/frame at 80x24/240x40.
Projection work occurs on changed inputs; rows and state summaries remain cached between cycles. These are in-memory debug measurements.
Evidence logs: `/home/pi/.local/state/lqosync3/collection-*.log`.
Captures: `docs/design/tui/captures/*-collection-v2.png`, actual terminal with isolated fixtures.

Next: finish checks and release preparation, then use the authorized deployment workflow. Root installation still requires operator sudo.
The running production binary is unchanged until that operator step succeeds.

Final local verification: 552 tests pass, zero fail, seven manual tests ignored. Formatting, all-target Clippy, and debug build pass.
Concept hashes, design links, whitespace, and implementation PNG decoding pass. Three updated terminal captures follow the approved composition.
The release review also rejects a shared shaping ID or ambiguous DHCP endpoint instead of assigning a false subscriber state.
Publish failure or an older cache sequence cannot claim a fresh collection. Raw observations remain available without a forced shaping match.
No production config, source enablement, scan interval, or removal rule changed during implementation.

## Verified release prepared

Updated: 2026-11-09T19:46:45+08:00 (Asia/Shanghai).
Code commit `39459a7` is pushed to `origin/lqosync3`. GitHub CI run `42611316069` passed.
The clean optimized artifact reports `3.0.157+g39459a7`.
SHA-256: `49fb42b61f22993a7d8c08a8397219414621a701a8b9f47a15dae09c81ef54e5`.
Artifact, manifest, operator instructions, and deployment script: `/home/pi/.local/state/lqosync3/collection-39459a7/`.
Script syntax and checksum checks pass. It includes backup, atomic replacement, rollback, restart, executable/cycle/telemetry checks,
and collection-count comparison against the new cycle's private raw RouterOS snapshot.

Non-interactive sudo failed before mutation: a password is required.
Operator step: `sudo /home/pi/.local/state/lqosync3/collection-39459a7/deploy.sh`.
After execution, verify `DEPLOYED.json`, current process version, fresh source counts, subscriber states, and the installed TUI.
The operator was notified. Do not report activation until these runtime checks pass.

## Deployment verified

Updated: 2026-11-09T20:32:29+08:00 (Asia/Shanghai).
The operator ran the prepared deployment script. Release `3.0.157+g39459a7` is installed and running.
Installed SHA-256 matches the verified artifact. Service PID `61153` is active/running with zero automatic restarts.
The deployment script checked the running executable and compared source counts with the private cycle snapshot.
Configuration is unchanged. Backup: `/var/backups/lqosync3/tui-39459a7-6rmrzs5d`.

Independent verification passed at cycle 2: status and collection PID/sequence agree, with no service/config error.
The latest cycle contains 21 online PPPoE records, three offline PPPoE accounts, and five DHCP BOUND leases.
Hotspot remains disabled. A BOUND lease does not prove device reachability.
Collection refresh follows the existing sync cycle. At verification, collection age was 107.4 seconds within its 190-second freshness limit.
Worker/DPDK telemetry was fresh at 2.4 seconds, with 114 traffic history points.

The installed TUI passed a read-only live-config terminal smoke test: all seven routes rendered,
the DPDK panel and ONLINE/OFFLINE/BOUND states appeared, and quit restored the terminal with no panic.
Evidence: `/home/pi/.local/state/lqosync3/collection-39459a7/DEPLOYED.json` and `installed-tui-smoke.json`.
No raw subscriber identities were saved in smoke-test evidence.
The authorized fix, release, deployment, restart, and verification are complete. Reopen existing TUI sessions to load this release.
