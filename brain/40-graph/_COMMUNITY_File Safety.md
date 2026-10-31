---
type: community
members: 38
---

# File Safety

**Members:** 38 nodes

## Members
- [[CircuitNames]] - code - src/config.rs
- [[File_1]] - code
- [[HDR]] - code - src/files.rs
- [[HDR_1]] - code - src/state.rs
- [[Live]] - code - src/files.rs
- [[Value_5]] - code
- [[Value_6]] - code
- [[backup()]] - code - src/files.rs
- [[backup_keeps_the_last_n()]] - code - src/files.rs
- [[backup_keeps_the_new_folder_and_other_folders()]] - code - src/files.rs
- [[bad_network_json_stops_when_needed()]] - code - src/files.rs
- [[bail]] - concept
- [[files.rs]] - code - src/files.rs
- [[load_state()]] - code - src/files.rs
- [[load_state_seeds_on_first_run_and_reads_after()]] - code - src/files.rs
- [[lock()]] - code - src/files.rs
- [[network_json_must_be_an_object()]] - code - src/files.rs
- [[read_csv()]] - code - src/state.rs
- [[read_csv_accepts_bom_crlf_and_missing_columns()]] - code - src/state.rs
- [[read_csv_skips_blank_names_and_keeps_repeated_names()]] - code - src/state.rs
- [[read_live()]] - code - src/files.rs
- [[read_live_splits_static_rows()]] - code - src/files.rs
- [[reload_ok_and_failure_and_timeout()]] - code - src/files.rs
- [[run_reload()]] - code - src/files.rs
- [[second_lock_fails()]] - code - src/files.rs
- [[seed_infers_source_and_keeps_ids()]] - code - src/state.rs
- [[seed_nodes()]] - code - src/state.rs
- [[seed_nodes_reads_the_tree_with_parents_and_ids()]] - code - src/state.rs
- [[seed_state()]] - code - src/state.rs
- [[seed_uses_the_configured_names_and_comments()]] - code - src/state.rs
- [[split_static()]] - code - src/state.rs
- [[split_static_ignores_case()]] - code - src/state.rs
- [[state.rs]] - code - src/state.rs
- [[state_json_round_trip_and_version_check()]] - code - src/state.rs
- [[write]] - concept
- [[write_if_changed()]] - code - src/files.rs
- [[write_if_changed_keeps_the_mode_of_the_old_file()]] - code - src/files.rs
- [[write_if_changed_skips_same_bytes_and_leaves_no_temp_file()]] - code - src/files.rs

## Live Query (requires Dataview plugin)

```dataview
TABLE source_file, type FROM #community/File_Safety
SORT file.name ASC
```

## Connections to other communities
- 19 edges to [[_COMMUNITY_Community 7]]
- 4 edges to [[_COMMUNITY_Telegram Notifier 2]]
- 3 edges to [[_COMMUNITY_Topology]]
- 3 edges to [[_COMMUNITY_Telegram Notifier]]
- 2 edges to [[_COMMUNITY_Config]]
- 2 edges to [[_COMMUNITY_Status and Diff]]
- 2 edges to [[_COMMUNITY_Router State]]
- 1 edge to [[_COMMUNITY_Config 3]]
- 1 edge to [[_COMMUNITY_hashmap]]
- 1 edge to [[_COMMUNITY_Community 15]]

## Top bridge nodes
- [[files.rs]] - degree 33, connects to 5 communities
- [[bail]] - degree 6, connects to 5 communities
- [[state.rs]] - degree 22, connects to 4 communities
- [[read_csv()]] - degree 10, connects to 2 communities
- [[CircuitNames]] - degree 6, connects to 2 communities