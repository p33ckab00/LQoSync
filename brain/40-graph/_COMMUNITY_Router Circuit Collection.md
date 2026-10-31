---
type: community
cohesion: 0.19
members: 33
---

# Router Circuit Collection

**Cohesion:** 0.19 - loosely connected
**Members:** 33 nodes

## Members
- [[Collected]] - code - src/sources.rs
- [[Config]] - code - src/config.rs
- [[DhcpPlan]] - code - src/sources.rs
- [[DhcpSite]] - code - src/sources.rs
- [[PppPlan]] - code - src/sources.rs
- [[ROUTER]] - code - src/sources.rs
- [[RosRow]] - code - src/model.rs
- [[RouterCfg]] - code - src/config.rs
- [[RouterSnapshot]] - code - src/model.rs
- [[by_name()]] - code - src/sources.rs
- [[cfg()_2]] - code - src/sources.rs
- [[collect_dhcp()]] - code - src/sources.rs
- [[collect_hotspot()]] - code - src/sources.rs
- [[collect_ppp()]] - code - src/sources.rs
- [[collect_router()]] - code - src/sources.rs
- [[dhcp_per_plan_uses_lease_comment()]] - code - src/sources.rs
- [[dhcp_per_site_filters_and_names()]] - code - src/sources.rs
- [[dhcp_server_comment_beats_settings()]] - code - src/sources.rs
- [[disabled_sources_give_nothing()]] - code - src/sources.rs
- [[f()]] - code - src/sources.rs
- [[flag_on()]] - code - src/sources.rs
- [[hotspot_key_rules()]] - code - src/sources.rs
- [[mac_key()]] - code - src/sources.rs
- [[parse_default_rate()]] - code - src/speed.rs
- [[plan_label()]] - code - src/sources.rs
- [[plan_label_truncates()]] - code - src/sources.rs
- [[ppp_mac_falls_back_to_secret_caller_id()]] - code - src/sources.rs
- [[ppp_needs_active_and_enabled_secret()]] - code - src/sources.rs
- [[ppp_speed_chain_order()]] - code - src/sources.rs
- [[rate()]] - code - src/sources.rs
- [[run()_1]] - code - src/sources.rs
- [[snap()]] - code - src/sources.rs
- [[sources.rs]] - code - src/sources.rs

## Live Query (requires Dataview plugin)

```dataview
TABLE source_file, type FROM #community/Router_Circuit_Collection
SORT file.name ASC
```

## Connections to other communities
- 12 edges to [[_COMMUNITY_Topology and Node IDs]]
- 12 edges to [[_COMMUNITY_Speed Parsing]]
- 11 edges to [[_COMMUNITY_Scenario Test Harness]]
- 11 edges to [[_COMMUNITY_Cycle Assembly]]
- 7 edges to [[_COMMUNITY_Config Loading]]
- 4 edges to [[_COMMUNITY_DHCP Settings]]
- 3 edges to [[_COMMUNITY_DHCP Speed Sources]]
- 1 edge to [[_COMMUNITY_Runtime Settings]]
- 1 edge to [[_COMMUNITY_Node Templates]]
- 1 edge to [[_COMMUNITY_LibreQoS Targets]]
- 1 edge to [[_COMMUNITY_PPPoE Speed Sources]]
- 1 edge to [[_COMMUNITY_Backstop Caps]]
- 1 edge to [[_COMMUNITY_Offline Action]]
- 1 edge to [[_COMMUNITY_Validation and Network Modes]]

## Top bridge nodes
- [[Config]] - degree 41, connects to 13 communities
- [[sources.rs]] - degree 38, connects to 6 communities
- [[Collected]] - degree 10, connects to 3 communities
- [[RouterSnapshot]] - degree 13, connects to 2 communities
- [[collect_router()]] - degree 12, connects to 2 communities