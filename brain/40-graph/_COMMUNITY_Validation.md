---
type: community
cohesion: 0.23
members: 20
---

# Validation

**Cohesion:** 0.23 - loosely connected
**Members:** 20 nodes

## Members
- [[Map_1]] - code
- [[Value_4]] - code
- [[bail]] - concept
- [[check()]] - code - src/validate.rs
- [[collect_nodes()]] - code - src/validate.rs
- [[duplicate_circuit_id_stops()]] - code - src/validate.rs
- [[duplicate_node_name_at_any_level_stops()]] - code - src/validate.rs
- [[hashmap]] - concept
- [[hashset]] - concept
- [[json]] - concept
- [[min_above_max_or_bad_number_stops()]] - code - src/validate.rs
- [[missing_parent_stops_except_in_no_parent()]] - code - src/validate.rs
- [[multi_device_circuit_passes_but_shared_device_id_stops()]] - code - src/validate.rs
- [[net()_1]] - code - src/validate.rs
- [[parent_node_id_must_match_the_parent_node()]] - code - src/validate.rs
- [[rate()_1]] - code - src/validate.rs
- [[rates_below_libreqos_floor_stop()]] - code - src/validate.rs
- [[row()_1]] - code - src/validate.rs
- [[valid_rows_pass()]] - code - src/validate.rs
- [[validate.rs]] - code - src/validate.rs

## Live Query (requires Dataview plugin)

```dataview
TABLE source_file, type FROM #community/Validation
SORT file.name ASC
```

## Connections to other communities
- 5 edges to [[_COMMUNITY_CSV Writer and Columns]]
- 4 edges to [[_COMMUNITY_Cycle Assembly]]
- 3 edges to [[_COMMUNITY_Network Modes]]
- 3 edges to [[_COMMUNITY_Topology and Node Templates]]
- 2 edges to [[_COMMUNITY_State File and CSV Read]]
- 1 edge to [[_COMMUNITY_Config Loading]]

## Top bridge nodes
- [[validate.rs]] - degree 26, connects to 4 communities
- [[bail]] - degree 4, connects to 3 communities
- [[check()]] - degree 12, connects to 2 communities
- [[hashmap]] - degree 3, connects to 2 communities
- [[row()_1]] - degree 9, connects to 1 community