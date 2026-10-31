---
type: community
cohesion: 0.09
members: 65
---

# Cycle Assembly

**Cohesion:** 0.09 - loosely connected
**Members:** 65 nodes

## Members
- [[CycleInput]] - code - src/cycle.rs
- [[CycleOutput]] - code - src/cycle.rs
- [[Failed]] - code - src/cycle.rs
- [[NOW]] - code - src/cycle.rs
- [[Offline]] - code - src/cycle.rs
- [[Online]] - code - src/cycle.rs
- [[Report]] - code - src/cycle.rs
- [[RouterResult]] - code - src/cycle.rs
- [[StoredCircuit]] - code - src/model.rs
- [[Unreachable]] - code - src/cycle.rs
- [[Value]] - code
- [[a_row_that_stays_keeps_its_group_node()]] - code - src/cycle.rs
- [[assemble()]] - code - src/cycle.rs
- [[cfg()]] - code - src/cycle.rs
- [[changed_columns()]] - code - src/cycle.rs
- [[cycle.rs]] - code - src/cycle.rs
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
- 15 edges to [[_COMMUNITY_Topology and Node Templates]]
- 9 edges to [[_COMMUNITY_CSV Writer and Columns]]
- 7 edges to [[_COMMUNITY_Config Root and Checks]]
- 7 edges to [[_COMMUNITY_Model Types]]
- 6 edges to [[_COMMUNITY_v3 Node Rules]]
- 5 edges to [[_COMMUNITY_Speed and DHCP Sources]]
- 4 edges to [[_COMMUNITY_Router Circuit Collection]]
- 4 edges to [[_COMMUNITY_Validation]]
- 3 edges to [[_COMMUNITY_Circuit Sources]]
- 1 edge to [[_COMMUNITY_State File and CSV Read]]
- 1 edge to [[_COMMUNITY_Network Modes]]
- 1 edge to [[_COMMUNITY_Offline Action]]

## Top bridge nodes
- [[cycle.rs]] - degree 87, connects to 10 communities
- [[assemble()]] - degree 16, connects to 4 communities
- [[StoredCircuit]] - degree 8, connects to 4 communities
- [[CycleOutput]] - degree 13, connects to 3 communities
- [[CycleInput]] - degree 7, connects to 3 communities