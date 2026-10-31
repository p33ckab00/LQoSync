---
type: community
members: 22
---

# File Safety

**Members:** 22 nodes

## Members
- [[CsvRow_1]] - code
- [[File_1]] - code
- [[HDR]] - code - src/files.rs
- [[Live]] - code - src/files.rs
- [[Value_2]] - code
- [[backup()]] - code - src/files.rs
- [[backup_keeps_the_last_n()]] - code - src/files.rs
- [[bad_network_json_stops_when_needed()]] - code - src/files.rs
- [[bail]] - concept
- [[files.rs]] - code - src/files.rs
- [[load_state()]] - code - src/files.rs
- [[load_state_seeds_on_first_run_and_reads_after()]] - code - src/files.rs
- [[lock()]] - code - src/files.rs
- [[network_json_must_be_an_object()]] - code - src/files.rs
- [[read_live()]] - code - src/files.rs
- [[read_live_splits_static_rows()]] - code - src/files.rs
- [[reload_ok_and_failure_and_timeout()]] - code - src/files.rs
- [[run_reload()]] - code - src/files.rs
- [[second_lock_fails()]] - code - src/files.rs
- [[write]] - concept
- [[write_if_changed()]] - code - src/files.rs
- [[write_if_changed_skips_same_bytes_and_leaves_no_temp_file()]] - code - src/files.rs

## Live Query (requires Dataview plugin)

```dataview
TABLE source_file, type FROM #community/File_Safety
SORT file.name ASC
```

## Connections to other communities
- 10 edges to [[_COMMUNITY_CircuitNames]]
- 2 edges to [[_COMMUNITY_Runner Cycle]]
- 2 edges to [[_COMMUNITY_Scenario Tests]]
- 2 edges to [[_COMMUNITY_CLI and Checks]]
- 1 edge to [[_COMMUNITY_Cycle Assembly]]
- 1 edge to [[_COMMUNITY_Status and Diff]]
- 1 edge to [[_COMMUNITY_MikroTik Session 2]]
- 1 edge to [[_COMMUNITY_Topology]]
- 1 edge to [[_COMMUNITY_Config]]
- 1 edge to [[_COMMUNITY_hashmap]]

## Top bridge nodes
- [[files.rs]] - degree 31, connects to 6 communities
- [[bail]] - degree 6, connects to 5 communities
- [[load_state()]] - degree 7, connects to 2 communities
- [[read_live()]] - degree 6, connects to 1 community