---
type: session
created: 2026-11-09
tags: [collection, tui, subscriber-state]
status: deployment-pending
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
