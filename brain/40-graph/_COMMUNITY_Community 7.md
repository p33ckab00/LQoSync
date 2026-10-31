---
type: community
members: 96
---

# Community 7

**Members:** 96 nodes

## Members
- [[dot-default()_1]] - code - src/model.rs
- [[dot-names()]] - code - src/model.rs
- [[Columns]] - code - src/model.rs
- [[CsvRow]] - code - src/model.rs
- [[CycleInput]] - code - src/cycle.rs
- [[CycleOutput]] - code - src/cycle.rs
- [[Dhcp]] - code - src/model.rs
- [[Entry]] - code - src/cycle.rs
- [[Failed]] - code - src/cycle.rs
- [[GuardHold]] - code - src/model.rs
- [[Hotspot_1]] - code - src/model.rs
- [[NOW]] - code - src/cycle.rs
- [[Offline]] - code - src/cycle.rs
- [[Online]] - code - src/cycle.rs
- [[Ppp_1]] - code - src/model.rs
- [[RouterResult]] - code - src/cycle.rs
- [[Snapshot]] - code - tests/scenarios.rs
- [[Source]] - code - src/model.rs
- [[State]] - code - src/model.rs
- [[StoredCircuit]] - code - src/model.rs
- [[Unknown]] - code - src/model.rs
- [[Unreachable_1]] - code - src/cycle.rs
- [[V2]] - code - src/model.rs
- [[V2_COLUMNS]] - code - src/model.rs
- [[V3]] - code - src/model.rs
- [[V3_COLUMNS]] - code - src/model.rs
- [[Value_3]] - code
- [[Value_4]] - code
- [[a_row_that_stays_keeps_its_group_node()]] - code - src/cycle.rs
- [[assemble()]] - code - src/cycle.rs
- [[cfg()_3]] - code - src/cycle.rs
- [[changed_columns()]] - code - src/cycle.rs
- [[changed_columns_compare_rates_as_numbers_and_keep_values()]] - code - src/cycle.rs
- [[cycle()]] - code - tests/scenarios.rs
- [[cycle.rs]] - code - src/cycle.rs
- [[deserialize]] - concept
- [[duplicate_ip_existing_vs_existing_keeps_collected()]] - code - src/cycle.rs
- [[duplicate_ip_new_vs_existing_keeps_existing()]] - code - src/cycle.rs
- [[empty_online_router_removes_its_rows()]] - code - src/cycle.rs
- [[failed_and_unreachable_router_keep_rows_and_nodes()]] - code - src/cycle.rs
- [[first_forty()]] - code - src/cycle.rs
- [[flat_promotion_has_no_client_node_for_a_static_row()]] - code - src/cycle.rs
- [[flat_promotion_parent_is_own_client_node()]] - code - src/cycle.rs
- [[fmt_rate()]] - code - src/speed.rs
- [[fnv1a64()]] - code - src/topology.rs
- [[fs]] - concept
- [[get()]] - code - src/cycle.rs
- [[grace_also_applies_to_a_router_not_in_the_config()]] - code - src/cycle.rs
- [[grace_keeps_a_missing_subscriber()]] - code - src/cycle.rs
- [[group_counts_skip_duplicate_ip_circuits()]] - code - src/cycle.rs
- [[guard_cfg()]] - code - src/cycle.rs
- [[guard_holds_a_mass_removal_and_keeps_other_changes()]] - code - src/cycle.rs
- [[guard_off_removes_at_once()]] - code - src/cycle.rs
- [[guard_releases_on_force()]] - code - src/cycle.rs
- [[guard_releases_on_same_fingerprint_next_cycle()]] - code - src/cycle.rs
- [[guard_restarts_on_a_different_fingerprint()]] - code - src/cycle.rs
- [[hundred()]] - code - src/cycle.rs
- [[id_collision_asks_for_another_id()]] - code - src/cycle.rs
- [[ids()]] - code - src/cycle.rs
- [[many_duplicate_ip_drops_raise_an_alert()]] - code - src/cycle.rs
- [[model.rs]] - code - src/model.rs
- [[name()]] - code - src/routerstate.rs
- [[names()]] - code - src/cycle.rs
- [[new_circuit_gets_new_ids_and_existing_keeps_them()]] - code - src/cycle.rs
- [[node_name_collision_stops_with_clear_message()]] - code - src/cycle.rs
- [[offline_remove_ignores_grace_and_keep_keeps()]] - code - src/cycle.rs
- [[online_router_removes_rows_not_seen()]] - code - src/cycle.rs
- [[pathbuf]] - concept
- [[ppp()]] - code - src/cycle.rs
- [[ppp_owned()]] - code - src/cycle.rs
- [[read()_1]] - code - tests/scenarios.rs
- [[recycled_ip_goes_to_the_new_subscriber()]] - code - src/cycle.rs
- [[remove_inactive_entries_false_removes_nothing()]] - code - src/cycle.rs
- [[render_csv()]] - code - src/writer/pair.rs
- [[render_network_json()]] - code - src/writer/pair.rs
- [[report_json()]] - code - tests/scenarios.rs
- [[results()]] - code - tests/scenarios.rs
- [[rows_of_a_router_not_in_config_are_removed()]] - code - src/cycle.rs
- [[rp()]] - code - src/cycle.rs
- [[rp_toml_with_guard()]] - code - src/cycle.rs
- [[run()_2]] - code - src/cycle.rs
- [[run_full()]] - code - src/cycle.rs
- [[same_name_on_two_routers_later_router_wins()]] - code - src/cycle.rs
- [[same_name_twice_in_one_router_keeps_first_and_warns()]] - code - src/cycle.rs
- [[scenarios.rs]] - code - tests/scenarios.rs
- [[scenarios_match_expected_output()]] - code - tests/scenarios.rs
- [[seed_rows_go_only_when_all_routers_online()]] - code - src/cycle.rs
- [[seeded_nodes_survive_preserve_mode()]] - code - src/cycle.rs
- [[small_removal_is_not_held()]] - code - src/cycle.rs
- [[state()]] - code - src/cycle.rs
- [[static_multi_device_circuit_keeps_all_rows()]] - code - src/cycle.rs
- [[static_row()]] - code - src/cycle.rs
- [[static_row_passes_through_and_collected_replaces_it()]] - code - src/cycle.rs
- [[static_row_wins_a_duplicate_ip()]] - code - src/cycle.rs
- [[stored()]] - code - src/cycle.rs
- [[v3_with_no_parent_stops()]] - code - src/cycle.rs

## Live Query (requires Dataview plugin)

```dataview
TABLE source_file, type FROM #community/Community_7
SORT file.name ASC
```

## Connections to other communities
- 19 edges to [[_COMMUNITY_Topology]]
- 19 edges to [[_COMMUNITY_File Safety]]
- 13 edges to [[_COMMUNITY_Telegram Notifier]]
- 12 edges to [[_COMMUNITY_Status and Diff]]
- 11 edges to [[_COMMUNITY_Router State]]
- 10 edges to [[_COMMUNITY_Config 3]]
- 10 edges to [[_COMMUNITY_Circuit Collection]]
- 8 edges to [[_COMMUNITY_hashmap]]
- 5 edges to [[_COMMUNITY_Router State 2]]
- 5 edges to [[_COMMUNITY_Secrets]]
- 1 edge to [[_COMMUNITY_OfflineAction]]
- 1 edge to [[_COMMUNITY_Config]]
- 1 edge to [[_COMMUNITY_Telegram Notifier 2]]
- 1 edge to [[_COMMUNITY_Community 15]]

## Top bridge nodes
- [[cycle.rs]] - degree 89, connects to 8 communities
- [[CsvRow]] - degree 33, connects to 5 communities
- [[scenarios.rs]] - degree 25, connects to 5 communities
- [[model.rs]] - degree 15, connects to 5 communities
- [[assemble()]] - degree 19, connects to 4 communities