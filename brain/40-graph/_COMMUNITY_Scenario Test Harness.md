---
type: community
cohesion: 0.06
members: 62
---

# Scenario Test Harness

**Cohesion:** 0.06 - loosely connected
**Members:** 62 nodes

## Members
- [[dot-default()]] - code - src/model.rs
- [[dot-names()]] - code - src/model.rs
- [[Columns]] - code - src/model.rs
- [[CsvRow]] - code - src/model.rs
- [[Dhcp]] - code - src/model.rs
- [[Entry]] - code - src/cycle.rs
- [[GuardHold]] - code - src/model.rs
- [[HDR]] - code - src/state.rs
- [[Hotspot]] - code - src/model.rs
- [[Ppp]] - code - src/model.rs
- [[RouterMemo]] - code - src/model.rs
- [[Snapshot]] - code - tests/scenarios.rs
- [[Source]] - code - src/model.rs
- [[State]] - code - src/model.rs
- [[Unknown]] - code - src/model.rs
- [[V2]] - code - src/model.rs
- [[V2_COLUMNS]] - code - src/model.rs
- [[V3]] - code - src/model.rs
- [[V3_COLUMNS]] - code - src/model.rs
- [[Value]] - code
- [[Value_1]] - code
- [[assemble]] - concept
- [[btreemap]] - concept
- [[csv_header_sort_and_quoting()]] - code - src/writer/pair.rs
- [[csv_headers_for_v2_and_v3()]] - code - src/writer/pair.rs
- [[csv_quotes_like_python()]] - code - src/writer/pair.rs
- [[cycle()]] - code - tests/scenarios.rs
- [[cycleinput_1]] - concept
- [[deserialize]] - concept
- [[field()]] - code - src/writer/pair.rs
- [[fs]] - concept
- [[model.rs]] - code - src/model.rs
- [[network_json_has_python_layout()]] - code - src/writer/pair.rs
- [[pair.rs]] - code - src/writer/pair.rs
- [[path]] - concept
- [[pathbuf]] - concept
- [[read()]] - code - tests/scenarios.rs
- [[read_csv]] - concept
- [[read_csv()]] - code - src/state.rs
- [[read_csv_accepts_bom_crlf_and_missing_columns()]] - code - src/state.rs
- [[read_csv_skips_blank_names_and_later_duplicate_wins()]] - code - src/state.rs
- [[render_csv]] - concept
- [[render_csv()]] - code - src/writer/pair.rs
- [[render_network_json]] - concept
- [[render_network_json()]] - code - src/writer/pair.rs
- [[report_json()]] - code - tests/scenarios.rs
- [[results()]] - code - tests/scenarios.rs
- [[row()]] - code - src/writer/pair.rs
- [[scenarios.rs]] - code - tests/scenarios.rs
- [[scenarios_match_expected_output()]] - code - tests/scenarios.rs
- [[seed_infers_source_and_keeps_ids()]] - code - src/state.rs
- [[seed_nodes()]] - code - src/state.rs
- [[seed_nodes_reads_the_tree_with_parents_and_ids()]] - code - src/state.rs
- [[seed_state]] - concept
- [[seed_state()]] - code - src/state.rs
- [[serialize]] - concept
- [[split_static]] - concept
- [[split_static()]] - code - src/state.rs
- [[split_static_ignores_case()]] - code - src/state.rs
- [[state.rs]] - code - src/state.rs
- [[state_json_round_trip_and_version_check()]] - code - src/state.rs
- [[super]] - concept

## Live Query (requires Dataview plugin)

```dataview
TABLE source_file, type FROM #community/Scenario_Test_Harness
SORT file.name ASC
```

## Connections to other communities
- 25 edges to [[_COMMUNITY_Cycle Assembly]]
- 11 edges to [[_COMMUNITY_Router Circuit Collection]]
- 8 edges to [[_COMMUNITY_Topology and Node IDs]]
- 7 edges to [[_COMMUNITY_Validation and Network Modes]]
- 2 edges to [[_COMMUNITY_Speed Parsing]]
- 2 edges to [[_COMMUNITY_Config Loading]]

## Top bridge nodes
- [[super]] - degree 8, connects to 6 communities
- [[state.rs]] - degree 21, connects to 4 communities
- [[CsvRow]] - degree 25, connects to 3 communities
- [[model.rs]] - degree 15, connects to 3 communities
- [[btreemap]] - degree 6, connects to 3 communities