---
type: community
members: 13
---

# Status and Diff 2

**Members:** 13 nodes

## Members
- [[2026-10-31 First live shadow run]] - document - brain/10-sessions/2026-10-31-first-live-shadow-run.md
- [[ADR-0004 TOML policy, Python defaults]] - rationale - brain/HANDOFF.md
- [[ADR-0005 Topology follows v3]] - rationale - brain/HANDOFF.md
- [[ADR-0007 Fresh design, not a port]] - rationale - brain/HANDOFF.md
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
- 8 edges to [[_COMMUNITY_Community 36]]
- 7 edges to [[_COMMUNITY_Community 16]]
- 6 edges to [[_COMMUNITY_Session Notes]]
- 4 edges to [[_COMMUNITY_Topology 2]]
- 1 edge to [[_COMMUNITY_Topology 3]]
- 1 edge to [[_COMMUNITY_Community 18]]
- 1 edge to [[_COMMUNITY_Speed Rules 2]]
- 1 edge to [[_COMMUNITY_Topology 4]]
- 1 edge to [[_COMMUNITY_Speed Rules 3]]

## Top bridge nodes
- [[Handoff]] - degree 28, connects to 8 communities
- [[2026-10-31 First live shadow run]] - degree 9, connects to 3 communities
- [[Differences from updatecsv.py]] - degree 5, connects to 2 communities
- [[ADR-0007 Fresh design, not a port]] - degree 4, connects to 1 community
- [[DHCP speed sources (server_comment, lease_comment, settings)]] - degree 3, connects to 1 community