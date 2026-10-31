---
type: community
members: 18
---

# Speed Rules 3

**Members:** 18 nodes

## Members
- [[etclqosync3lqosync3.toml]] - concept - docs/CONFIGURATION.md
- [[ADR-0008 Terminal UI (lqosync3 tui)]] - rationale - brain/HANDOFF.md
- [[Comment speed format (Name25M5M, 25M5M, 25M)]] - concept - docs/CONFIGURATION.md
- [[Configfrom_toml_str (defaults merge)]] - code - docs/superpowers/specs/2026-10-31-lqosync3-core-design.md
- [[DHCP server discovery (ipdhcp-server)]] - concept - docs/CONFIGURATION.md
- [[DHCP speed_sources (server_comment, lease_comment, settings)]] - concept - docs/CONFIGURATION.md
- [[Exclusions (exclude and routers.exclude)]] - concept - docs/CONFIGURATION.md
- [[Name templates (nodes, circuits)]] - concept - docs/CONFIGURATION.md
- [[PPPoE speed order (comments, profile_rate_limit, profile_name, default)]] - rationale - brain/HANDOFF.md
- [[Removal guard (safety)]] - rationale - docs/CONFIGURATION.md
- [[Removal guard decision (20% or 50 rows)]] - rationale - brain/HANDOFF.md
- [[TLS (tls_verify, tls_ca_file, webpki-roots)]] - concept - docs/superpowers/specs/2026-10-31-lqosync3-runtime-design.md
- [[Task 1 Crate scaffold and config]] - concept - docs/superpowers/plans/2026-10-31-lqosync3-core.md
- [[backstop_caps per-source caps]] - concept - docs/CONFIGURATION.md
- [[bandwidth_defaults rate floors and factors]] - concept - docs/CONFIGURATION.md
- [[router_defaults overlay rules]] - concept - docs/CONFIGURATION.md
- [[configdefaults.toml (embedded defaults)]] - concept - docs/CONFIGURATION.md
- [[speed.pppoe_sources]] - concept - docs/CONFIGURATION.md

## Live Query (requires Dataview plugin)

```dataview
TABLE source_file, type FROM #community/Speed_Rules_3
SORT file.name ASC
```

## Connections to other communities
- 8 edges to [[_COMMUNITY_Status and Diff]]
- 3 edges to [[_COMMUNITY_MikroTik Session 5]]
- 3 edges to [[_COMMUNITY_Status and Diff 3]]
- 2 edges to [[_COMMUNITY_Topology 2]]
- 2 edges to [[_COMMUNITY_Config 2]]
- 2 edges to [[_COMMUNITY_Speed Rules 4]]
- 1 edge to [[_COMMUNITY_Router State 2]]
- 1 edge to [[_COMMUNITY_Decisions]]
- 1 edge to [[_COMMUNITY_MikroTik Session 4]]
- 1 edge to [[_COMMUNITY_Mock Router Tests 2]]

## Top bridge nodes
- [[etclqosync3lqosync3.toml]] - degree 13, connects to 6 communities
- [[Removal guard (safety)]] - degree 9, connects to 5 communities
- [[configdefaults.toml (embedded defaults)]] - degree 8, connects to 4 communities
- [[router_defaults overlay rules]] - degree 5, connects to 1 community
- [[Configfrom_toml_str (defaults merge)]] - degree 3, connects to 1 community