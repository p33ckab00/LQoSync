---
type: community
cohesion: 0.15
members: 19
---

# Config Loading

**Cohesion:** 0.15 - loosely connected
**Members:** 19 nodes

## Members
- [[dot-from_toml_str()]] - code - src/config.rs
- [[DEFAULTS_TOML]] - code - src/config.rs
- [[MIN]] - code - src/config.rs
- [[PppoeCfg]] - code - src/config.rs
- [[btreeset]] - concept
- [[config.rs]] - code - src/config.rs
- [[default_from_file!]] - code - src/config.rs
- [[every_default_comes_from_the_defaults_file()]] - code - src/config.rs
- [[exclusions_parse_and_bad_ip_is_rejected()]] - code - src/config.rs
- [[minimal_config_gets_documented_defaults()]] - code - src/config.rs
- [[node_type_defaults_follow_target()]] - code - src/config.rs
- [[non_finite_values_are_rejected()]] - code - src/config.rs
- [[out_of_range_values_are_rejected()]] - code - src/config.rs
- [[per_site_rejects_lease_comment_and_defaults_follow_mode()]] - code - src/config.rs
- [[router_defaults_apply_to_each_router_and_a_router_overrides_them()]] - code - src/config.rs
- [[router_list_problems_are_rejected()]] - code - src/config.rs
- [[unknown_key_is_rejected_with_its_name()]] - code - src/config.rs
- [[unknown_template_fields_are_rejected()]] - code - src/config.rs
- [[v3_target_rejects_no_parent()]] - code - src/config.rs

## Live Query (requires Dataview plugin)

```dataview
TABLE source_file, type FROM #community/Config_Loading
SORT file.name ASC
```

## Connections to other communities
- 9 edges to [[_COMMUNITY_Config Root and Checks]]
- 7 edges to [[_COMMUNITY_DHCP Settings]]
- 6 edges to [[_COMMUNITY_Speed and DHCP Sources]]
- 5 edges to [[_COMMUNITY_Embedded Defaults Merge]]
- 3 edges to [[_COMMUNITY_Router Circuit Collection]]
- 3 edges to [[_COMMUNITY_Runtime Settings]]
- 2 edges to [[_COMMUNITY_Network Modes]]
- 2 edges to [[_COMMUNITY_LibreQoS Targets]]
- 2 edges to [[_COMMUNITY_Offline Action]]
- 2 edges to [[_COMMUNITY_Backstop Caps]]
- 2 edges to [[_COMMUNITY_State File and CSV Read]]
- 1 edge to [[_COMMUNITY_Topology and Node Templates]]
- 1 edge to [[_COMMUNITY_Validation]]
- 1 edge to [[_COMMUNITY_Model Types]]
- 1 edge to [[_COMMUNITY_CSV Writer and Columns]]
- 1 edge to [[_COMMUNITY_Config Doc Test]]

## Top bridge nodes
- [[config.rs]] - degree 59, connects to 16 communities
- [[dot-from_toml_str()]] - degree 13, connects to 3 communities
- [[PppoeCfg]] - degree 2, connects to 1 community