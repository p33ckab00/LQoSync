---
type: community
members: 42
---

# Community 18

**Members:** 42 nodes

## Members
- [[Applier (hash, atomic write, reload)]] - concept - docs/ARCHITECTURE.md
- [[Circuit Model]] - concept - docs/ARCHITECTURE.md
- [[Collector (RouterOS API reads)]] - concept - docs/ARCHITECTURE.md
- [[Config Loader]] - concept - docs/ARCHITECTURE.md
- [[CycleSummarynext_sleep]] - concept - docs/superpowers/plans/2026-10-31-lqosync3-runtime.md
- [[MikroTik RouterOS API]] - concept - docs/ARCHITECTURE.md
- [[Performance Rules]] - rationale - docs/ARCHITECTURE.md
- [[Plan 2 review focus (5 risks)]] - rationale - docs/superpowers/plans/2026-10-31-lqosync3-runtime.md
- [[Proposed Crate Layout]] - concept - docs/ARCHITECTURE.md
- [[Router State Machine (onlinefailedunreachableoffline)]] - rationale - docs/ARCHITECTURE.md
- [[RouterCfg.tls_verify  tls_ca_file]] - concept - docs/superpowers/plans/2026-10-31-lqosync3-runtime.md
- [[RouterSession  Connector traits]] - concept - docs/superpowers/plans/2026-10-31-lqosync3-runtime.md
- [[ShapedDevices.csv]] - concept - docs/ARCHITECTURE.md
- [[Signals (SIGHUP reload, USR1 cycle now)]] - concept - docs/OPERATIONS.md
- [[State Store (state.json)]] - concept - docs/ARCHITECTURE.md
- [[Target Detector]] - concept - docs/ARCHITECTURE.md
- [[Target v2_pair]] - concept - docs/ARCHITECTURE.md
- [[Target v3_network]] - concept - docs/ARCHITECTURE.md
- [[Target v3_pair]] - concept - docs/ARCHITECTURE.md
- [[Validator]] - concept - docs/ARCHITECTURE.md
- [[Writers (one per target)]] - concept - docs/ARCHITECTURE.md
- [[cycleassemble (pure core)]] - concept - docs/superpowers/plans/2026-10-31-lqosync3-runtime.md
- [[filesbackup (keep_backups)]] - concept - docs/superpowers/plans/2026-10-31-lqosync3-runtime.md
- [[filesload_state (state.json or seed)]] - concept - docs/superpowers/plans/2026-10-31-lqosync3-runtime.md
- [[fileslock (lqosync3.lock)]] - concept - docs/superpowers/plans/2026-10-31-lqosync3-runtime.md
- [[filesread_live  Live]] - concept - docs/superpowers/plans/2026-10-31-lqosync3-runtime.md
- [[filesrun_reload]] - concept - docs/superpowers/plans/2026-10-31-lqosync3-runtime.md
- [[fileswrite_if_changed (atomic write)]] - concept - docs/superpowers/plans/2026-10-31-lqosync3-runtime.md
- [[libreqos_network.json]] - concept - docs/ARCHITECTURE.md
- [[lqosd (LibreQoS daemon)]] - concept - docs/ARCHITECTURE.md
- [[lqosync3.service systemd unit]] - concept - docs/INSTALLATION.md
- [[main run loop (tokioselect! on sleep and signals)]] - concept - docs/superpowers/plans/2026-10-31-lqosync3-runtime.md
- [[mikrotikMikrotikConnector  MikrotikSession]] - concept - docs/superpowers/plans/2026-10-31-lqosync3-runtime.md
- [[mikrotiktls_client_config (rustls, webpki-roots)]] - concept - docs/superpowers/plans/2026-10-31-lqosync3-runtime.md
- [[network.json]] - concept - docs/ARCHITECTURE.md
- [[routerosReadError]] - concept - docs/superpowers/plans/2026-10-31-lqosync3-runtime.md
- [[routerosread_router (connect, retries)]] - concept - docs/superpowers/plans/2026-10-31-lqosync3-runtime.md
- [[routerosread_snapshot (TABLES, api_timeout)]] - concept - docs/superpowers/plans/2026-10-31-lqosync3-runtime.md
- [[runnerRunnercycle (one full cycle)]] - concept - docs/superpowers/plans/2026-10-31-lqosync3-runtime.md
- [[secretsread_secret]] - concept - docs/superpowers/plans/2026-10-31-lqosync3-runtime.md
- [[testsmock_router.rs (mock RouterOS server)]] - concept - docs/superpowers/plans/2026-10-31-lqosync3-runtime.md
- [[v2 vs v3 writer differences]] - concept - docs/ARCHITECTURE.md

## Live Query (requires Dataview plugin)

```dataview
TABLE source_file, type FROM #community/Community_18
SORT file.name ASC
```

## Connections to other communities
- 3 edges to [[_COMMUNITY_Session Notes]]
- 3 edges to [[_COMMUNITY_Community 30]]
- 2 edges to [[_COMMUNITY_Community 16]]
- 1 edge to [[_COMMUNITY_Topology 2]]
- 1 edge to [[_COMMUNITY_Community 26]]
- 1 edge to [[_COMMUNITY_Speed Rules 3]]

## Top bridge nodes
- [[runnerRunnercycle (one full cycle)]] - degree 12, connects to 2 communities
- [[Signals (SIGHUP reload, USR1 cycle now)]] - degree 4, connects to 2 communities
- [[Collector (RouterOS API reads)]] - degree 6, connects to 1 community
- [[Target Detector]] - degree 5, connects to 1 community
- [[lqosync3.service systemd unit]] - degree 4, connects to 1 community