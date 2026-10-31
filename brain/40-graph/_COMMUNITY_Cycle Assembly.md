---
type: community
cohesion: 0.10
members: 56
---

# Cycle Assembly

**Cohesion:** 0.10 - loosely connected
**Members:** 56 nodes

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
- [[Value_2]] - code
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
- [[names()]] - code - src/cycle.rs
- [[new_circuit_gets_new_ids_and_existing_keeps_them()]] - code - src/cycle.rs
- [[offline_remove_ignores_grace_and_keep_keeps()]] - code - src/cycle.rs
- [[online_router_removes_rows_not_seen()]] - code - src/cycle.rs
- [[ppp()]] - code - src/cycle.rs
- [[ppp_owned()]] - code - src/cycle.rs
- [[remove_inactive_entries_false_removes_nothing()]] - code - src/cycle.rs
- [[rows_of_a_router_not_in_config_are_removed()]] - code - src/cycle.rs
- [[rp()]] - code - src/cycle.rs
- [[rp_toml_with_guard()]] - code - src/cycle.rs
- [[run()]] - code - src/cycle.rs
- [[run_full()]] - code - src/cycle.rs
- [[same_name_on_two_routers_later_router_wins()]] - code - src/cycle.rs
- [[seed_rows_go_only_when_all_routers_online()]] - code - src/cycle.rs
- [[small_removal_is_not_held()]] - code - src/cycle.rs
- [[state()]] - code - src/cycle.rs
- [[static_row_passes_through_and_collected_replaces_it()]] - code - src/cycle.rs
- [[stored()]] - code - src/cycle.rs
- [[v3_with_no_parent_stops()]] - code - src/cycle.rs

## Live Query (requires Dataview plugin)

```dataview
TABLE source_file, type FROM #community/Cycle_Assembly
SORT file.name ASC
```

## Connections to other communities
- 25 edges to [[_COMMUNITY_Scenario Test Harness]]
- 14 edges to [[_COMMUNITY_Topology and Node IDs]]
- 11 edges to [[_COMMUNITY_Router Circuit Collection]]
- 6 edges to [[_COMMUNITY_Speed Parsing]]
- 5 edges to [[_COMMUNITY_Validation and Network Modes]]
- 1 edge to [[_COMMUNITY_Offline Action]]

## Top bridge nodes
- [[cycle.rs]] - degree 78, connects to 6 communities
- [[assemble()]] - degree 15, connects to 4 communities
- [[RouterResult]] - degree 13, connects to 2 communities
- [[run_full()]] - degree 13, connects to 2 communities
- [[CycleInput]] - degree 7, connects to 2 communities