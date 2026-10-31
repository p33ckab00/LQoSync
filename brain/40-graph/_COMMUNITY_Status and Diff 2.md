---
type: community
members: 13
---

# Status and Diff 2

**Members:** 13 nodes

## Members
- [[2026-10-31 First live shadow run]] - document - brain/10-sessions/2026-10-31-first-live-shadow-run.md
- [[ADR-0004 TOML policy, Python defaults_1]] - rationale - brain/HANDOFF.md
- [[ADR-0005 Topology follows v3_1]] - rationale - brain/HANDOFF.md
- [[ADR-0007 Fresh design, not a port_1]] - rationale - brain/HANDOFF.md
- [[DHCP speed sources (server_comment, lease_comment, settings)]] - rationale - docs/CONFIGURATION.md
- [[DHCP-libreqos 25 to 35 Mbps difference]] - rationale - brain/10-sessions/2026-10-31-first-live-shadow-run.md
- [[Differences from updatecsv.py]] - concept - docs/CONFIGURATION.md
- [[FSEHub-Distro router (10.200.1.28728)]] - concept - brain/10-sessions/2026-10-31-first-live-shadow-run.md
- [[Handoff]] - document - brain/HANDOFF.md
- [[Plain API warning (consider api-ssl 8729 with tls_ca_file)]] - rationale - brain/10-sessions/2026-10-31-first-live-shadow-run.md
- [[Production state (Python live, lqosync3 shadow, LibreQoS 2.2)]] - concept - brain/HANDOFF.md
- [[Shadow run results (62 rows, 167 ms, 26 text-only updates)]] - concept - brain/10-sessions/2026-10-31-first-live-shadow-run.md
- [[Task 1 Crate scaffold and config]] - concept - docs/superpowers/plans/2026-10-31-lqosync3-core.md

## Live Query (requires Dataview plugin)

```dataview
TABLE source_file, type FROM #community/Status_and_Diff_2
SORT file.name ASC
```

## Connections to other communities
- 7 edges to [[_COMMUNITY_Speed Rules]]
- 6 edges to [[_COMMUNITY_Session Notes]]
- 2 edges to [[_COMMUNITY_Telegram Notifier]]
- 2 edges to [[_COMMUNITY_Router State 3]]

## Top bridge nodes
- [[Handoff]] - degree 16, connects to 4 communities
- [[Differences from updatecsv.py]] - degree 5, connects to 2 communities
- [[2026-10-31 First live shadow run]] - degree 8, connects to 1 community
- [[ADR-0007 Fresh design, not a port_1]] - degree 4, connects to 1 community
- [[DHCP speed sources (server_comment, lease_comment, settings)]] - degree 3, connects to 1 community