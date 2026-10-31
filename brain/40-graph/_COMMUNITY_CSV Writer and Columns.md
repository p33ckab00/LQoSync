---
type: community
cohesion: 0.22
members: 14
---

# CSV Writer and Columns

**Cohesion:** 0.22 - loosely connected
**Members:** 14 nodes

## Members
- [[dot-names()]] - code - src/model.rs
- [[Columns]] - code - src/model.rs
- [[CsvRow]] - code - src/model.rs
- [[V2]] - code - src/model.rs
- [[V3]] - code - src/model.rs
- [[csv_header_sort_and_quoting()]] - code - src/writer/pair.rs
- [[csv_headers_for_v2_and_v3()]] - code - src/writer/pair.rs
- [[csv_quotes_like_python()]] - code - src/writer/pair.rs
- [[field()]] - code - src/writer/pair.rs
- [[network_json_has_python_layout()]] - code - src/writer/pair.rs
- [[pair.rs]] - code - src/writer/pair.rs
- [[render_csv()]] - code - src/writer/pair.rs
- [[row()]] - code - src/writer/pair.rs
- [[super]] - concept

## Live Query (requires Dataview plugin)

```dataview
TABLE source_file, type FROM #community/CSV_Writer_and_Columns
SORT file.name ASC
```

## Connections to other communities
- 9 edges to [[_COMMUNITY_Cycle Assembly]]
- 5 edges to [[_COMMUNITY_v3 Node Rules]]
- 5 edges to [[_COMMUNITY_State File and CSV Read]]
- 5 edges to [[_COMMUNITY_Validation]]
- 3 edges to [[_COMMUNITY_Model Types]]
- 3 edges to [[_COMMUNITY_Topology and Node Templates]]
- 1 edge to [[_COMMUNITY_Circuit Sources]]
- 1 edge to [[_COMMUNITY_Config Loading]]
- 1 edge to [[_COMMUNITY_Router Circuit Collection]]
- 1 edge to [[_COMMUNITY_Speed and DHCP Sources]]

## Top bridge nodes
- [[CsvRow]] - degree 26, connects to 7 communities
- [[super]] - degree 8, connects to 7 communities
- [[pair.rs]] - degree 12, connects to 2 communities
- [[Columns]] - degree 7, connects to 2 communities
- [[render_csv()]] - degree 6, connects to 1 community