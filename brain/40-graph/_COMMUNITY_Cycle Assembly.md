---
type: community
members: 88
---

# Cycle Assembly

**Members:** 88 nodes

## Members
- [[dot-default()]] - code - src/model.rs
- [[dot-names()]] - code - src/model.rs
- [[Columns]] - code - src/model.rs
- [[CsvRow]] - code - src/model.rs
- [[CycleInput]] - code - src/cycle.rs
- [[CycleOutput]] - code - src/cycle.rs
- [[Dhcp]] - code - src/model.rs
- [[Entry]] - code - src/cycle.rs
- [[Failed]] - code - src/cycle.rs
- [[GuardHold]] - code - src/model.rs
- [[Hotspot]] - code - src/model.rs
- [[NOW]] - code - src/cycle.rs
- [[Offline]] - code - src/cycle.rs
- [[Online]] - code - src/cycle.rs
- [[Ppp]] - code - src/model.rs
- [[Report]] - code - src/cycle.rs
- [[RosRow]] - code - src/model.rs
- [[RouterMemo]] - code - src/model.rs
- [[RouterResult]] - code - src/cycle.rs
- [[RouterSnapshot]] - code - src/model.rs
- [[Source]] - code - src/model.rs
- [[State]] - code - src/model.rs
- [[StoredCircuit]] - code - src/model.rs
- [[Unknown]] - code - src/model.rs
- [[Unreachable]] - code - src/cycle.rs
- [[V2]] - code - src/model.rs
- [[V2_COLUMNS]] - code - src/model.rs
- [[V3]] - code - src/model.rs
- [[V3_COLUMNS]] - code - src/model.rs
- [[Value]] - code
- [[a_row_that_stays_keeps_its_group_node()]] - code - src/cycle.rs
- [[assemble()]] - code - src/cycle.rs
- [[btreemap]] - concept
- [[cfg()]] - code - src/cycle.rs
- [[changed_columns()]] - code - src/cycle.rs
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
- [[names()]] - code - src/cycle.rs
- [[new_circuit_gets_new_ids_and_existing_keeps_them()]] - code - src/cycle.rs
- [[node_name_collision_stops_with_clear_message()]] - code - src/cycle.rs
- [[offline_remove_ignores_grace_and_keep_keeps()]] - code - src/cycle.rs
- [[online_router_removes_rows_not_seen()]] - code - src/cycle.rs
- [[ppp()]] - code - src/cycle.rs
- [[ppp_owned()]] - code - src/cycle.rs
- [[recycled_ip_goes_to_the_new_subscriber()]] - code - src/cycle.rs
- [[remove_inactive_entries_false_removes_nothing()]] - code - src/cycle.rs
- [[rows_of_a_router_not_in_config_are_removed()]] - code - src/cycle.rs
- [[rp()]] - code - src/cycle.rs
- [[rp_toml_with_guard()]] - code - src/cycle.rs
- [[run()]] - code - src/cycle.rs
- [[run_full()]] - code - src/cycle.rs
- [[same_name_on_two_routers_later_router_wins()]] - code - src/cycle.rs
- [[same_name_twice_in_one_router_keeps_first_and_warns()]] - code - src/cycle.rs
- [[seed_rows_go_only_when_all_routers_online()]] - code - src/cycle.rs
- [[seeded_nodes_survive_preserve_mode()]] - code - src/cycle.rs
- [[serialize]] - concept
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
TABLE source_file, type FROM #community/Cycle_Assembly
SORT file.name ASC
```

## Connections to other communities
- 20 edges to [[_COMMUNITY_Topology]]
- 10 edges to [[_COMMUNITY_Circuit Collection]]
- 8 edges to [[_COMMUNITY_hashmap]]
- 5 edges to [[_COMMUNITY_Secrets]]
- 4 edges to [[_COMMUNITY_Runner Cycle]]
- 2 edges to [[_COMMUNITY_Scenario Tests]]
- 1 edge to [[_COMMUNITY_OfflineAction]]
- 1 edge to [[_COMMUNITY_File Safety]]
- 1 edge to [[_COMMUNITY_CircuitNames]]
- 1 edge to [[_COMMUNITY_Status and Diff 2]]
- 1 edge to [[_COMMUNITY_LibreQoS v3 2]]
- 1 edge to [[_COMMUNITY_Config]]

## Top bridge nodes
- [[cycle.rs]] - degree 87, connects to 7 communities
- [[btreemap]] - degree 9, connects to 6 communities
- [[assemble()]] - degree 15, connects to 3 communities
- [[CsvRow]] - degree 16, connects to 2 communities
- [[deserialize]] - degree 3, connects to 2 communities