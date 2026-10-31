---
type: community
cohesion: 0.15
members: 19
---

# v3 Node Rules

**Cohesion:** 0.15 - loosely connected
**Members:** 19 nodes

## Members
- [[Snapshot]] - code - tests/scenarios.rs
- [[Value_2]] - code
- [[assemble]] - concept
- [[cycle()]] - code - tests/scenarios.rs
- [[cycleinput_1]] - concept
- [[fs]] - concept
- [[path]] - concept
- [[pathbuf]] - concept
- [[read()]] - code - tests/scenarios.rs
- [[read_csv]] - concept
- [[render_csv]] - concept
- [[render_network_json]] - concept
- [[render_network_json()]] - code - src/writer/pair.rs
- [[report_json()]] - code - tests/scenarios.rs
- [[results()]] - code - tests/scenarios.rs
- [[scenarios.rs]] - code - tests/scenarios.rs
- [[scenarios_match_expected_output()]] - code - tests/scenarios.rs
- [[seed_state]] - concept
- [[split_static]] - concept

## Live Query (requires Dataview plugin)

```dataview
TABLE source_file, type FROM #community/v3_Node_Rules
SORT file.name ASC
```

## Connections to other communities
- 6 edges to [[_COMMUNITY_Cycle Assembly]]
- 5 edges to [[_COMMUNITY_CSV Writer and Columns]]
- 4 edges to [[_COMMUNITY_Model Types]]
- 3 edges to [[_COMMUNITY_Config Root and Checks]]
- 3 edges to [[_COMMUNITY_State File and CSV Read]]
- 2 edges to [[_COMMUNITY_Router Circuit Collection]]

## Top bridge nodes
- [[scenarios.rs]] - degree 25, connects to 5 communities
- [[cycle()]] - degree 9, connects to 4 communities
- [[scenarios_match_expected_output()]] - degree 9, connects to 2 communities
- [[results()]] - degree 5, connects to 2 communities
- [[render_network_json()]] - degree 4, connects to 1 community