---
type: community
members: 9
---

# Community 41

**Members:** 9 nodes

## Members
- [[etclqosync3lqosync3.toml config file]] - concept - docs/CONFIGURATION.md
- [[ADR-0004 TOML policy, Python defaults_1]] - rationale - brain/HANDOFF.md
- [[Configfrom_toml_str (defaults merge)]] - code - docs/superpowers/specs/2026-10-31-lqosync3-core-design.md
- [[DHCP speed sources (per_site  per_plan)]] - rationale - docs/CONFIGURATION.md
- [[Deferred minor items from defaults review]] - concept - brain/HANDOFF.md
- [[TLS (tls_verify, tls_ca_file, webpki-roots)]] - concept - docs/superpowers/specs/2026-10-31-lqosync3-runtime-design.md
- [[Task 1 Crate scaffold and config]] - concept - docs/superpowers/plans/2026-10-31-lqosync3-core.md
- [[router_defaults per-router defaults]] - concept - docs/CONFIGURATION.md
- [[configdefaults.toml embedded defaults]] - concept - docs/CONFIGURATION.md

## Live Query (requires Dataview plugin)

```dataview
TABLE source_file, type FROM #community/Community_41
SORT file.name ASC
```

## Connections to other communities
- 6 edges to [[_COMMUNITY_CLI and Checks 2]]
- 1 edge to [[_COMMUNITY_Topology 4]]
- 1 edge to [[_COMMUNITY_Speed Rules 2]]
- 1 edge to [[_COMMUNITY_MikroTik Session 4]]
- 1 edge to [[_COMMUNITY_MikroTik Session 5]]

## Top bridge nodes
- [[TLS (tls_verify, tls_ca_file, webpki-roots)]] - degree 3, connects to 2 communities
- [[ADR-0004 TOML policy, Python defaults_1]] - degree 3, connects to 2 communities
- [[configdefaults.toml embedded defaults]] - degree 5, connects to 1 community
- [[router_defaults per-router defaults]] - degree 5, connects to 1 community
- [[Configfrom_toml_str (defaults merge)]] - degree 3, connects to 1 community