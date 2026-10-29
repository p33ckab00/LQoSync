# Build scope and extraction limits

This build covers 97 files: 59 code files and 38 documents, about 64,448 words.
All 38 documents have complete line coverage. The code extractor reported no failed source files.
The source baseline is `32eb2552a000544c59947720a5a6cffc40dde00f`. Local shared-workflow documentation was also present during extraction.
Read [SCOPE.md](SCOPE.md) for included sources, exclusions, queries, and update commands.

## Graph health

The raw extraction contains 369 unresolved endpoint references.
Graphify represents them with 62 external concept nodes. Some are unresolved local modules or types.
There are 0 missing-endpoint records and 0 self-loops.
The default undirected representation combines 42 repeated same-endpoint relationships.
Distinct evidence can therefore share a displayed link. See [HEALTH.txt](HEALTH.txt) for the diagnostic.
These limits affect completeness. This graph does not prove a complete call graph or production readiness.

## Cost accounting

Semantic extraction used three session agents. The collaboration tool did not expose token usage.
Token counters in the generated section are placeholders, not measured cost or evidence of free extraction.

---

# Graph Report - LQoSync3.0  (2026-10-29)

## Corpus Check
- 97 files · ~64,448 words
- Verdict: corpus is large enough that graph structure adds value.
- Unclassified: 9 file(s) not represented in the graph (top: (none) 5, .toml 2, .example 1)

## Summary
- 892 nodes · 2384 edges · 54 communities (38 shown, 16 thin omitted)
- Extraction: 90% EXTRACTED · 10% INFERRED · 0% AMBIGUOUS · INFERRED: 232 edges (avg confidence: 0.81)
- Token cost: 0 input · 0 output

## Graph Freshness
- Built from commit: `32eb2552`
- Run `git rev-parse HEAD` and compare to check if the graph is stale.
- Run `graphify update .` after code changes (no API cost).

## Community Hubs (Navigation)
- Subscriber Speed Resolution
- Scoped Core Policy Analysis
- Core Plans and Validation
- Durable Local Generations
- Router Inventory and Adapters
- Topology Web Forms
- Stable WAN Assignment
- Generation and Source Tests
- Subscriber Normalization
- Review Pages and Credentials
- Router Protocol Tests
- Normalization and Error Context
- Console Route Tests
- Speed Rule Web Forms
- Speed and Allocation Plans
- Console Form Protection
- Shared Workflow and Handoff
- Minimum Allocation Tests
- Collection Orchestration Tests
- Historical Setup Reviews
- Scheduled Collection Orchestration
- Assignment Review Pages
- Core Review Integration Tests
- Protected Ownership Plan Tests
- Topology Model and Compiler
- Synthetic Core Test Helpers
- Historical Core Review
- Core Binding Web Forms
- Connection Mark Path Tests
- Source Normalization History
- Connection Validation Plans
- Core Diagnostic Tests
- Sanitized Policy Fixtures
- Proportional Minimum Allocation
- WAN Interface Web Forms
- Core Setup UI Tests
- Rust Foundation Design
- DHCP Speed Web Forms
- Source Selection Web Forms
- Topology Compiler Tests
- Scoped Routing History
- DHCP Speed UI Tests
- WAN UI Tests
- Speed Policy History
- Router Transport History
- Project Knowledge Index
- WAN Assignment Policy
- Core Validation Design
- Router Adapter Design
- WAN Assignment Design
- Application Package Manifest
- Patched Transport Manifest

## God Nodes (most connected - your core abstractions)
1. `Inventory` - 60 edges
2. `Settings` - 54 edges
3. `Shared` - 44 edges
4. `Subscriber` - 34 edges
5. `error()` - 31 edges
6. `shell()` - 25 edges
7. `App` - 24 edges
8. `routes()` - 24 edges
9. `Scoped routing and minimum allocation` - 23 edges
10. `check()` - 22 edges

## Surprising Connections (you probably didn't know these)
- `Observed-only custom-table paths` --conceptually_related_to--> `observed()`  [INFERRED]
  v3/CORE_REVIEW.md → v3/src/core_diagnostics.rs
- `Core Address-List Planner Implementation Plan` --references--> `publish_orchestrated()`  [EXTRACTED]
  docs/superpowers/plans/2026-10-29-core-list-planner.md → v3/src/store.rs
- `Core Address-List Planner Implementation Plan` --references--> `load_core_plan()`  [EXTRACTED]
  docs/superpowers/plans/2026-10-29-core-list-planner.md → v3/src/store.rs
- `Compatible source pools` --conceptually_related_to--> `eligible_targets()`  [INFERRED]
  v3/WAN_ASSIGNMENTS.md → v3/src/assignment.rs
- `Explicit WAN interface mapping` --conceptually_related_to--> `validate_wans()`  [INFERRED]
  v3/WAN_ASSIGNMENTS.md → v3/src/assignment.rs

## Import Cycles
- 2-file cycle: `v3/src/speed_preview.rs -> v3/src/speeds.rs -> v3/src/speed_preview.rs`

## Hyperedges (group relationships)
- **Core list publication integrity** — brain_10_sessions_2026_10_29_core_list_planner_owned_hosts, brain_10_sessions_2026_10_29_core_list_planner_protected_rows, brain_10_sessions_2026_10_29_core_list_planner_five_file_generation, brain_10_sessions_2026_10_29_core_list_planner_final_freshness [EXTRACTED 1.00]
- **Assignment-to-compiler minimum flow** — brain_10_sessions_2026_10_29_scoped_routing_minimums_minimum_allocation, brain_10_sessions_2026_10_29_scoped_routing_minimums_independent_admission, brain_10_sessions_2026_10_29_scoped_routing_minimums_blocked_publication [EXTRACTED 1.00]
- **Source collection and committed output integrity** — brain_10_sessions_2026_10_29_subscriber_normalization_source_bindings, brain_10_sessions_2026_10_29_subscriber_normalization_scheduled_collection, brain_10_sessions_2026_10_29_subscriber_normalization_fail_closed_ingestion, brain_10_sessions_2026_10_29_subscriber_normalization_generation_integrity [EXTRACTED 1.00]
- **Owned member and protected-row planning contract** — v3_core_planner_owned_ipv4_members, v3_core_planner_protected_address_list_members, v3_core_planner_add_keep_remove_plan, v3_core_planner_wan_core_binding [EXTRACTED 1.00]
- **Shared candidate/publication validation pipeline** — v3_subscribers_selected_source_normalization, v3_wan_assignments_assignment_priority, v3_speed_rules_automatic_minimum_allocation, v3_validation_end_to_end_validation, v3_core_planner_atomic_core_plan_generation [EXTRACTED 1.00]
- **Bounded transport and receiver-lifetime contract** — v3_vendor_mikrotik_tokio_readme_background_actor, v3_vendor_mikrotik_tokio_patches_awaited_bounded_sends, v3_vendor_mikrotik_tokio_readme_drop_based_cancellation, v3_vendor_mikrotik_tokio_patches_burst_regression_removal_gate [EXTRACTED 1.00]

## Communities (54 total, 16 thin omitted)

### Community 0 - "Subscriber Speed Resolution"
Cohesion: 0.06
Nodes (60): Speed Rules, key(), Cache, cached(), duplicate_keys(), eligible_lease(), entry(), index() (+52 more)

### Community 1 - "Scoped Core Policy Analysis"
Cohesion: 0.06
Nodes (62): Analysis, analyze(), Findings, Analysis, analyze(), ConnectionPath, fields(), MANGLE (+54 more)

### Community 2 - "Core Plans and Validation"
Cohesion: 0.06
Nodes (36): Core Address-List Planner Implementation Plan, Core Planner, Core Review, Scoped Routing, main(), Direction, RouterReply, SubscriberForward (+28 more)

### Community 3 - "Durable Local Generations"
Cohesion: 0.14
Nodes (29): AssignmentState, Subscriber, valid_id(), atomic_write(), CommittedCorePlan, Current, current_generation(), durable_directory() (+21 more)

### Community 4 - "Router Inventory and Adapters"
Cohesion: 0.10
Nodes (15): Readme, Adapter, Legacy, V3, select(), main(), Router, Client (+7 more)

### Community 5 - "Topology Web Forms"
Cohesion: 0.21
Nodes (15): check(), commit(), CSS, delete_node(), delete_router(), error(), inspect_router(), NodeForm (+7 more)

### Community 6 - "Stable WAN Assignment"
Cohesion: 0.18
Nodes (20): add_load(), assign(), eligible_targets(), Pin, Policy, Pool, score(), Topology (+12 more)

### Community 7 - "Generation and Source Tests"
Cohesion: 0.12
Nodes (6): discovered_source_form_checks_token_persists_and_publishes_without_an_apply_button(), request(), end_to_end_review_checks_current_outputs_without_publishing_and_detects_corruption(), missing_setup_and_saved_inputs_do_not_report_live_validation_success(), page(), queued_validation_rejects_changed_settings_read_epochs_and_generation_pointer()

### Community 8 - "Subscriber Normalization"
Cohesion: 0.12
Nodes (10): canonical_mac(), normalize(), source_table(), Subscribers, exact_source_and_account_names_with_spaces_are_never_silently_omitted(), hotspot_supersedes_dhcp_only_on_same_router_ip_and_mac(), ids_are_distinct_across_routers_and_dhcp_ids_ignore_ip_hostname_and_mac_case(), normalize_and_compile_50k_subscribers() (+2 more)

### Community 9 - "Review Pages and Credentials"
Cohesion: 0.17
Nodes (6): Page, page(), Credentials, Secret, Page, page()

### Community 10 - "Router Protocol Tests"
Cohesion: 0.22
Nodes (9): burst_response_preserves_every_row_and_completion(), external_client_logs_in_and_reads_tagged_rows(), full_mangle_reads_preserve_unknown_matchers_for_fail_closed_route_checks(), incomplete_and_rejected_reads_never_become_successful_tables(), reply(), router(), sentence(), tls_checks_trust_and_certificate_name_before_login() (+1 more)

### Community 11 - "Normalization and Error Context"
Cohesion: 0.16
Nodes (9): detect(), Detection, installed_version(), Candidate, has_bindings(), ipv4(), MAX_INVENTORY_AGE_SECONDS, service() (+1 more)

### Community 12 - "Console Route Tests"
Cohesion: 0.19
Nodes (13): routes(), source_managed_topology_edits_validate_an_allocated_copy_then_publish_effective_minimums(), add_preview_router(), all_pages_render_and_reject_mutation_without_token(), futures_poll_once(), manual_child_over_computed_parent_does_not_change_settings(), preview_inventory(), preview_rejects_results_if_policy_changes_while_worker_is_queued() (+5 more)

### Community 13 - "Speed Rule Web Forms"
Cohesion: 0.29
Nodes (11): router_speeds(), rules(), RulesForm, save_rules(), overview(), routers(), settings(), Shared (+3 more)

### Community 14 - "Speed and Allocation Plans"
Cohesion: 0.15
Nodes (3): DHCP server speed sources, Scoped routing and minimum allocation, Speed Rules Implementation Plan

### Community 15 - "Console Form Protection"
Cohesion: 0.15
Nodes (8): App, csrf(), field(), headers(), node_form(), node_tree(), router_form(), select()

### Community 16 - "Shared Workflow and Handoff"
Cohesion: 0.18
Nodes (5): LQoSync v3 agent instructions, Shared skills for LQoSync v3, LQoSync 3 handoff, Claude entry point, Previous LQoSync application README

### Community 17 - "Minimum Allocation Tests"
Cohesion: 0.23
Nodes (10): allocate_50k_subscribers_without_repeated_global_scans(), allocation_is_deterministic_and_floors_never_exceed_a_subscriber_maximum(), compiler_rejects_aggregate_minimums_above_any_ancestor(), exhausted_floors_fail_without_partial_input_mutation(), floors_at_the_exact_budget_do_not_fail_due_to_float_sum_rounding(), nested_asymmetric_caps_and_proportional_headroom_are_respected(), overloaded_minimums_fit_both_directions_without_changing_maxima_or_identity(), settings() (+2 more)

### Community 18 - "Collection Orchestration Tests"
Cohesion: 0.24
Nodes (11): app(), clearing_all_bindings_does_not_republish_managed_state_as_an_offline_snapshot(), failed_scheduled_round_cannot_replace_healthy_snapshot(), healthy_selected_inventory_automatically_publishes_and_failed_reads_keep_last_good(), mapped_wan_health_failure_retains_the_last_consistent_assignment_generation(), normalization_discards_a_calculation_if_a_new_read_starts_while_it_is_queued(), paused_sources_do_not_collect_or_publish(), read_sentence() (+3 more)

### Community 19 - "Historical Setup Reviews"
Cohesion: 0.18
Nodes (3): Actual local configuration and candidate review, Connection-mark and output validation, DHCP server speed sources

### Community 20 - "Scheduled Collection Orchestration"
Cohesion: 0.23
Nodes (5): after_mutation(), cycle(), now(), refresh(), same_inventories()

### Community 21 - "Assignment Review Pages"
Cohesion: 0.29
Nodes (8): assignments(), Page, PinForm, PoolForm, Row, save_pin(), save_pool(), View

### Community 22 - "Core Review Integration Tests"
Cohesion: 0.21
Nodes (4): get(), live_and_saved_policy_diagnostics_are_distinct_and_get_never_publishes(), queued_review_rejects_changed_settings_or_started_read(), review_shows_multiple_findings_escapes_policy_names_and_hides_passwords()

### Community 23 - "Protected Ownership Plan Tests"
Cohesion: 0.29
Nodes (8): owned(), exact_ownership_diff_is_deterministic_and_preserves_subscriber_data(), malformed_markers_do_not_claim_rows_and_other_scopes_are_preserved(), manual_exact_cidr_range_dns_and_ownership_collisions_block_without_claiming_rows(), pinned_wan_move_plans_only_the_owned_old_host_removal_and_new_list_add(), plan_50k_subscribers_and_60k_existing_members(), routing_conflicts_and_unsupported_matchers_are_explicitly_blocked(), rows()

### Community 24 - "Topology Model and Compiler"
Cohesion: 0.26
Nodes (8): Capacity, compile(), compile_unallocated(), Plan, validate(), assignments_commit_with_the_snapshot_and_survive_restart_and_corruption_is_blocked(), generation_contains_both_artifacts_and_unchanged_data_reuses_it(), mirror_write_failure_leaves_the_authoritative_snapshot_available()

### Community 25 - "Synthetic Core Test Helpers"
Cohesion: 0.18
Nodes (4): fixture(), OWNER, row(), fixture()

### Community 27 - "Core Binding Web Forms"
Cohesion: 0.33
Nodes (6): MappingForm, Page, page(), revision(), save(), tables()

### Community 28 - "Connection Mark Path Tests"
Cohesion: 0.32
Nodes (10): ambiguous_marks_wrong_interfaces_unknown_matchers_and_rule_order_remain_blocked(), classifier(), connection_path_limits_are_visible_and_cannot_create_a_wan_pass(), many_paths(), marked(), review_500_marked_wans_and_100k_routes(), review_rejects_reply_classifier_that_excludes_local_destinations(), source_list_connection_mark_chain_supports_subscriber_plans_and_reports_forward_direction() (+2 more)

### Community 31 - "Core Diagnostic Tests"
Cohesion: 0.33
Nodes (9): actual_api_presence_only_fib_switch_is_enabled_but_other_empty_flags_are_not_guessed(), finding_limits_are_explicit_and_never_convert_an_unsupported_policy_to_pass(), fixture(), historical_stale_partial_and_missing_inventory_are_explicit_and_never_look_live(), observed_path_limit_is_explicit_instead_of_silently_omitting_wans(), pcc_fasttrack_and_jumps_remain_unsupported_and_affect_every_wan(), per_wan_diagnostics_report_all_scoped_failures_without_poisoning_healthy_wan(), review_1000_wans_and_100k_routes() (+1 more)

### Community 32 - "Sanitized Policy Fixtures"
Cohesion: 0.35
Nodes (6): Aliases, Fixture, fixture(), project(), settings(), signature()

### Community 33 - "Proportional Minimum Allocation"
Cohesion: 0.27
Nodes (6): allocate(), Allocation, Ranges, Ranges<'a>, tolerance(), validate()

### Community 34 - "WAN Interface Web Forms"
Cohesion: 0.38
Nodes (6): commit(), Interface, interfaces(), save_wan(), WanForm, wans()

### Community 35 - "Core Setup UI Tests"
Cohesion: 0.33
Nodes (7): encode(), fixture(), get(), post(), preview_is_paginated_search_is_encoded_and_markup_is_escaped(), revision(), ui_saves_bindings_auto_plans_and_protects_invalid_or_stale_edits()

### Community 37 - "DHCP Speed Web Forms"
Cohesion: 0.33
Nodes (4): controls(), fields(), save(), ServerForm

### Community 38 - "Source Selection Web Forms"
Cohesion: 0.40
Nodes (5): Choice, choices(), save(), SourceForm, sources()

### Community 39 - "Topology Compiler Tests"
Cohesion: 0.47
Nodes (8): computed_directions_cap_independently_and_preserve_subscriber_rates(), cycles_and_missing_parents_are_rejected(), explicit_over_parent_and_duplicate_ip_are_rejected(), idle_computed_branches_are_omitted_from_a_nonempty_complete_snapshot(), nested_subscribers_count_once_and_use_immediate_parent_cap(), node(), settings(), subscriber()

### Community 41 - "DHCP Speed UI Tests"
Cohesion: 0.36
Nodes (4): page(), per_server_form_persists_rates_auto_refreshes_and_preserves_them_on_global_save(), post(), revision()

### Community 42 - "WAN UI Tests"
Cohesion: 0.36
Nodes (4): assignment_pagination_preserves_encoded_search_and_bounds_large_offsets(), encode(), mapped_wans_pool_and_pins_auto_publish_consistent_parents_and_reject_invalid_edits(), post()

## Knowledge Gaps
- **20 isolated node(s):** `lqosync-v3`, `Pass`, `Unsupported`, `Conflict`, `Unavailable` (+15 more)
  These have ≤1 connection - possible missing edges or undocumented components. (Counts symbols only; 175 node(s) total have ≤1 connection when file, concept and rationale nodes are included.)
- **16 thin communities (<3 nodes) omitted from report** — run `graphify query` to explore isolated nodes.

## Suggested Questions
_Questions this graph is uniquely positioned to answer:_

- **Why does `Inventory` connect `Scoped Core Policy Analysis` to `Sanitized Policy Fixtures`, `Subscriber Speed Resolution`, `Core Plans and Validation`, `Router Inventory and Adapters`, `Stable WAN Assignment`, `Subscriber Normalization`, `Normalization and Error Context`, `Console Route Tests`, `Console Form Protection`, `Scheduled Collection Orchestration`, `Protected Ownership Plan Tests`, `Synthetic Core Test Helpers`, `Connection Mark Path Tests`, `Core Diagnostic Tests`?**
  _High betweenness centrality (0.108) - this node is a cross-community bridge._
- **Are the 6 inferred relationships involving `Inventory` (e.g. with `same_inventories()` and `App`) actually correct?**
  _`Inventory` has 6 INFERRED edges - model-reasoned connections that need verification._
- **What connects `lqosync-v3`, `Pass`, `Unsupported` to the rest of the system?**
  _20 weakly-connected nodes found - possible documentation gaps or missing edges._
- **Should `Subscriber Speed Resolution` be split into smaller, more focused modules?**
  _Cohesion score 0.0640218878248974 - nodes in this community are weakly interconnected._
- **Why does `Settings` connect `Stable WAN Assignment` to `Subscriber Speed Resolution`, `Scoped Core Policy Analysis`, `Core Plans and Validation`, `Durable Local Generations`, `Router Inventory and Adapters`, `Topology Web Forms`, `Subscriber Normalization`, `Normalization and Error Context`, `Console Form Protection`, `Minimum Allocation Tests`, `Assignment Review Pages`, `Topology Model and Compiler`, `Synthetic Core Test Helpers`, `Core Binding Web Forms`, `Connection Mark Path Tests`, `Core Diagnostic Tests`, `Sanitized Policy Fixtures`, `Proportional Minimum Allocation`, `WAN Interface Web Forms`?**
  _High betweenness centrality (0.102) - this node is a cross-community bridge._
- **Are the 12 inferred relationships involving `Settings` (e.g. with `revision()` and `SpeedRules`) actually correct?**
  _`Settings` has 12 INFERRED edges - model-reasoned connections that need verification._
- **Should `Scoped Core Policy Analysis` be split into smaller, more focused modules?**
  _Cohesion score 0.06487341772151899 - nodes in this community are weakly interconnected._