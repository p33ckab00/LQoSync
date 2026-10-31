---
type: community
cohesion: 0.19
members: 15
---

# State File and CSV Read

**Cohesion:** 0.19 - loosely connected
**Members:** 15 nodes

## Members
- [[HDR]] - code - src/state.rs
- [[Value_3]] - code
- [[context]] - concept
- [[ensure]] - concept
- [[read_csv()]] - code - src/state.rs
- [[read_csv_accepts_bom_crlf_and_missing_columns()]] - code - src/state.rs
- [[read_csv_skips_blank_names_and_keeps_repeated_names()]] - code - src/state.rs
- [[seed_infers_source_and_keeps_ids()]] - code - src/state.rs
- [[seed_nodes()]] - code - src/state.rs
- [[seed_nodes_reads_the_tree_with_parents_and_ids()]] - code - src/state.rs
- [[seed_state()]] - code - src/state.rs
- [[split_static()]] - code - src/state.rs
- [[split_static_ignores_case()]] - code - src/state.rs
- [[state.rs]] - code - src/state.rs
- [[state_json_round_trip_and_version_check()]] - code - src/state.rs

## Live Query (requires Dataview plugin)

```dataview
TABLE source_file, type FROM #community/State_File_and_CSV_Read
SORT file.name ASC
```

## Connections to other communities
- 5 edges to [[_COMMUNITY_CSV Writer and Columns]]
- 3 edges to [[_COMMUNITY_Model Types]]
- 3 edges to [[_COMMUNITY_v3 Node Rules]]
- 2 edges to [[_COMMUNITY_Topology and Node Templates]]
- 2 edges to [[_COMMUNITY_Config Loading]]
- 2 edges to [[_COMMUNITY_Validation]]
- 1 edge to [[_COMMUNITY_Cycle Assembly]]
- 1 edge to [[_COMMUNITY_Circuit Sources]]
- 1 edge to [[_COMMUNITY_Speed and DHCP Sources]]

## Top bridge nodes
- [[state.rs]] - degree 20, connects to 5 communities
- [[seed_state()]] - degree 6, connects to 3 communities
- [[context]] - degree 4, connects to 3 communities
- [[read_csv()]] - degree 5, connects to 2 communities
- [[split_static()]] - degree 4, connects to 2 communities