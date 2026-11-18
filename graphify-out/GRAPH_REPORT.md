# Graph Report - LQoSync3.0  (2026-11-18)

## Corpus Check
- 363 files · ~425,032 words
- Verdict: corpus is large enough that graph structure adds value.

## Summary
- 5528 nodes · 13931 edges · 258 communities (216 shown, 42 thin omitted)
- Extraction: 94% EXTRACTED · 6% INFERRED · 0% AMBIGUOUS · INFERRED: 854 edges (avg confidence: 0.72)
- Token cost: 0 input · 0 output

## Community Hubs (Navigation)
- Runner Service Loop
- Source Policy and Rates
- LibreQoS Data and Network Render
- Registry Store and Staging
- Project Handoff Topology Status
- Settings Draft Model
- Identity Authorization Tests
- Probe, Detect and Diff
- Topology Resolver and Aggregates
- Topology Catalog and Rates
- Sync Cycle
- Core Design and ADRs
- Legacy Topology Builder
- TUI View Navigation
- Subscriber Statistics
- Topology Rebalance
- TUI Dashboard
- TUI Theme and Palette
- Native Topology Upstream Reports
- Topology Registry Model and Preflight
- Subscriber Observation
- State Files and Locking
- RouterOS Normalization
- Vault Home and Decisions
- Operations and DPDK Telemetry
- TUI Journal
- Aggregate Tests
- Telegram Notify
- Settings Key Metadata
- Configuration Loading
- Telegram Outbox
- TUI Devices View
- System Health
- TUI Settings View
- TUI Network View
- Worker Utilization IPC
- Topology Compiler
- Worker Telemetry Cache
- Router Sources
- Repro and Capture Scripts
- legacy_static_tests.rs
- TopologyCatalog
- preview.rs
- CsvRow
- catalog_tests.rs
- resolver_tests.rs
- app.rs
- Configuration
- Config
- cli.rs
- approved_key
- normalize_tests.rs
- Topology Manager discussion
- mock_router.rs
- verify.rs
- coordinator_tests.rs
- speed.rs
- Topology catalog and hierarchy persisten
- context
- 2026-10-31-lqosync3-v3-design.md
- Topology Manager implementation design
- routeros.rs
- RouterResult
- Procedure: remove the mlx5 compatibility
- runner::Runner::cycle (one full cycle)
- LibreQoS v3 ingress
- PreparedPair (prepare(AuthorizedReconcil
- ownership.rs
- journal/approved.rs
- workers/approved.rs
- store_recovery_tests.rs
- collection.rs
- arc
- deserialize
- tui/mod.rs
- import_evidence.rs
- reader_tests.rs
- preflight_tests.rs
- App
- shared_static.rs
- workers.rs
- Topology Foundation plan
- settings/secrets.rs
- Topology Manager initial TUI layout
- traffic_chart.rs
- circuit_queue.rs
- routers.rs
- SettingsView
- src/write_receipt.rs
- classification.rs
- nic_stats.rs
- ADR-0007 Fresh design, not a port
- overview.rs
- BandwidthDefaults
- Status
- admission.rs
- layout.rs
- publication_tests.rs
- tests/write_receipt.rs
- cache.rs
- Role
- PreparedPair preparation (publication.rs
- tests.rs
- ReadError
- prepare_with_static
- Installation
- ci.yml
- Hub
- nic_stats/ipc.rs
- lqosync3 runtime design (Plan 2)
- key_path
- presence.rs
- health.rs
- source_rate_tests.rs
- design.rs
- libreqos.rs
- pressure.rs
- delivery.rs
- lqosync3 terminal UI (Plan 4)
- apply_evidence.rs
- steps
- monitor.rs
- apply.rs
- on_key
- draw
- compiler_sandbox.rs
- topology_foundation.rs
- Mockup generation record (TUI PROMPTS)
- LQoSync3 TUI visual contract
- Source Evidence and IDs Plan
- migration_tests.rs
- node_minimums.rs
- src/validate.rs
- main.rs
- compiler_tests.rs
- PreparedPair
- super
- Monitor
- Legacy configuration preflight (prefligh
- Internal report: native policy acceptanc
- LQoSync3 TUI Cosmetics Implementation Pl
- 2026-10-31-lqosync3-tui-design.md
- TUI Cosmetics Design
- Command
- RouterAccess
- after_change
- tests/system_health.rs
- DPDK statistics delivery plan (6 checkpo
- Topology Settings and Recovery Plan
- LQoSync3 TUI cosmetics design
- Unified System Health and Telegram Repor
- Unified health and managed-write reports
- Plan 4b: TUI Settings editor
- TUI btop++ Refinement
- duration
- normalize_configured
- worker.rs
- src/model.rs
- Tab
- SAction
- LibreQoS v3 upgrade investigation
- Review Focus
- Runner::enroll_topology
- topology_catalog.rs
- Issue
- RouterForm
- verify
- staging::stage / recover / cancel
- native_full_keeps_node_policy.rs
- worker-utilization atomic cache
- lqosync3 DHCP client speed overrides and
- transaction.rs
- btop TUI, review fixes, and a new LibreQ
- TUI cosmetics design handoff
- Plan 2b: lqosync3 Telegram implementatio
- TUI Cosmetics plan (A1-A5, B6-B10)
- 2026-11-01-lqosync3-dhcp-clients-design.
- NetworkMode
- LeaseResult
- Sent
- settings/validate.rs
- Settings editor and functional requireme
- Plan 4c: DHCP Client Overrides
- Review Focus
- Unified health and Telegram plan
- Visibility
- queue.rs
- TUI visual review and transition assessm
- PppSource
- Host storage cleanup
- Unified health implementation
- RAM-backed worker telemetry cache
- Combined mirrored traffic chart
- Explicit native slot mapping
- Post-reboot native ACK diagnosis
- Topology Placement Resolver Plan
- TUI btop++ refinement
- Extra
- LibreQoS v3 live cutover
- 2026-11-02 — v3 import request, native p
- LibreQoS v3 upgrade preparation
- Approved TUI screen implementation
- DPDK statistics: task 5
- Native DPDK NIC diagnostics
- Combined mirrored DPDK chart
- Unified system health and Telegram resea
- Native circuit capacity diagnosis
- Native recovery path review
- Post-reboot native ACK diagnosis
- Aggregate capacities and minimum commitm
- IPv6 status of the production network
- Web-style shell: sidebar, top bar, seven
- LogLevel
- journal_app
- VModal
- 2026-10-31 — First live shadow run
- Native runtime policy validation
- Native topology source validation
- 2026-11-03 — LibreQoS v3 package update 
- LibreQoS beta19 integration review
- Cycle collection and subscriber state
- Approved TUI mockups
- TUI release review
- TUI shared-shell implementation
- Actual worker utilization
- DPDK statistics: task 4
- DPDK statistics: task 6
- DPDK statistics: task 2
- DPDK performance and deployment review
- DPDK statistics: task 1
- DPDK statistics: task 3
- Production performance observation
- Live Telegram delivery verification
- Unified health deployment
- Explicit native slot mapping
- install.sh
- LQoSync3
- Stage
- Journal proposal and TUI transition revi
- Optional node minimums
- pair_input
- report
- State
- lib.rs
- check.sh
- Task 1: Crate scaffold and config
- cli::check / CheckLine
- diff::diff_pair (semantic pair diff)
- lqosync3

## God Nodes (most connected - your core abstractions)
1. `Config` - 129 edges
2. `Handoff` - 99 edges
3. `CsvRow` - 91 edges
4. `RegistryDocument` - 76 edges
5. `Topology Manager implementation design` - 73 edges
6. `SettingsView` - 72 edges
7. `RateBps` - 71 edges
8. `Palette` - 70 edges
9. `runner()` - 63 edges
10. `PreparedPair` - 54 edges

## Surprising Connections (you probably didn't know these)
- `Router state classification (RouterResult, unreachable_since)` --implements--> `ADR-0003 Router failed vs offline`  [INFERRED]
  docs/superpowers/specs/2026-10-31-lqosync3-runtime-design.md → brain/20-decisions/ADR-0003-router-failed-vs-offline.md
- `Task 2: Events and message` --references--> `Notifier`  [EXTRACTED]
  docs/superpowers/plans/2026-10-31-lqosync3-telegram.md → src/notify.rs
- `Task 2: Events and message` --references--> `split()`  [EXTRACTED]
  docs/superpowers/plans/2026-10-31-lqosync3-telegram.md → src/notify.rs
- `Task 3: Transport (post_json, send, token, chat_id)` --references--> `post_json()`  [EXTRACTED]
  docs/superpowers/plans/2026-10-31-lqosync3-telegram.md → src/notify.rs
- `Task 3: Transport (post_json, send, token, chat_id)` --references--> `send()`  [EXTRACTED]
  docs/superpowers/plans/2026-10-31-lqosync3-telegram.md → src/notify.rs

## Import Cycles
- 1-file cycle: `tests/write_receipt.rs -> tests/write_receipt.rs`
- 2-file cycle: `src/tui/app.rs -> src/tui/network.rs -> src/tui/app.rs`
- 2-file cycle: `src/tui/app.rs -> src/tui/devices.rs -> src/tui/app.rs`
- 2-file cycle: `src/apply_evidence.rs -> src/apply_evidence/verify.rs -> src/apply_evidence.rs`
- 3-file cycle: `src/apply_evidence.rs -> src/apply_evidence/monitor.rs -> src/apply_evidence/verify.rs -> src/apply_evidence.rs`

## Hyperedges (group relationships)
- **Plan 1 core tasks building assemble()** — docs_superpowers_plans_2026_10_31_lqosync3_core_task1_config, docs_superpowers_plans_2026_10_31_lqosync3_core_task2_speed, docs_superpowers_plans_2026_10_31_lqosync3_core_task3_state, docs_superpowers_plans_2026_10_31_lqosync3_core_task4_sources, docs_superpowers_plans_2026_10_31_lqosync3_core_task5_topology, docs_superpowers_plans_2026_10_31_lqosync3_core_task6_cycle, docs_superpowers_plans_2026_10_31_lqosync3_core_task7_writer, docs_superpowers_plans_2026_10_31_lqosync3_core_task8_scenarios [EXTRACTED 1.00]
- **Removal guard hold-and-release flow** — docs_configuration_removal_guard, docs_superpowers_specs_2026_10_31_lqosync3_core_design_cycle_assembly_rules, docs_superpowers_specs_2026_10_31_lqosync3_core_design_guard_fingerprint, docs_superpowers_specs_2026_10_31_lqosync3_core_design_state_file [EXTRACTED 1.00]
- **Router failure vs offline handling** — brain_20_decisions_adr_0003_router_failed_vs_offline_router_state_model, brain_20_decisions_adr_0003_router_failed_vs_offline_offline_after, brain_20_decisions_adr_0003_router_failed_vs_offline_offline_action, brain_30_knowledge_updatecsv_py_dissection_defects_to_fix [EXTRACTED 1.00]
- **Crash-safe file writes** — docs_superpowers_specs_2026_10_31_lqosync3_runtime_design_instance_lock, docs_superpowers_specs_2026_10_31_lqosync3_runtime_design_write_order_state_first, docs_superpowers_specs_2026_10_31_lqosync3_runtime_design_atomic_write, docs_superpowers_specs_2026_10_31_lqosync3_runtime_design_first_run_seed [INFERRED 0.85]
- **Plan 2b Telegram notifier design-to-ops chain** — docs_superpowers_specs_2026_10_31_lqosync3_telegram_design, docs_superpowers_plans_2026_10_31_lqosync3_telegram, docs_operations_turn_on_telegram [INFERRED 0.85]
- **Router failed/unreachable/offline state model** — brain_handoff_adr_0003_router_failed_vs_offline, docs_superpowers_plans_2026_10_31_lqosync3_runtime_classify, docs_superpowers_plans_2026_10_31_lqosync3_runtime_read_router, docs_superpowers_plans_2026_10_31_lqosync3_runtime_read_error [INFERRED 0.85]
- **Safe cycle write path (lock, atomic write, backup, reload)** — docs_superpowers_plans_2026_10_31_lqosync3_runtime_runner_cycle, docs_superpowers_plans_2026_10_31_lqosync3_runtime_files_lock, docs_superpowers_plans_2026_10_31_lqosync3_runtime_write_if_changed, docs_superpowers_plans_2026_10_31_lqosync3_runtime_files_backup, docs_superpowers_plans_2026_10_31_lqosync3_runtime_run_reload [INFERRED 0.85]
- **LibreQoS v3 node rule compliance** — brain_20_decisions_adr_0005_topology_follows_v3_stable_node_id, brain_20_decisions_adr_0005_topology_follows_v3_parent_node_id, brain_20_decisions_adr_0005_topology_follows_v3_csv_column_count, brain_20_decisions_adr_0005_topology_follows_v3_max_floor_mbps, brain_20_decisions_adr_0005_topology_follows_v3_flat_promotion_parent_fix, brain_20_decisions_adr_0006_v3_guards_and_node_types_no_parent_guard, brain_20_decisions_adr_0006_v3_guards_and_node_types_site_group_nodes, brain_20_decisions_adr_0006_v3_guards_and_node_types_compile_mode_warning [INFERRED 0.85]
- **DHCP discovery and subscriber exclusion** — brain_20_decisions_adr_0009_dhcp_discovery_and_exclusions_dhcp_server_discovery, brain_20_decisions_adr_0009_dhcp_discovery_and_exclusions_exclusions, docs_configuration_exclusions, docs_superpowers_specs_2026_10_31_lqosync3_core_design_circuit_rules [INFERRED 0.95]
- **Fresh-data topology publication pipeline** — brain_10_sessions_2026_11_16_topology_source_boundary_router_reader, brain_10_sessions_2026_11_17_topology_normalization, brain_10_sessions_2026_11_17_topology_identity_identity_authorization, brain_handoff_placement_resolver, brain_10_sessions_2026_11_17_topology_publication_pair_preparation, brain_10_sessions_2026_11_17_topology_migration_admission_admitted_pair [INFERRED 0.85]
- **Legacy migration parity gates** — brain_10_sessions_2026_11_17_topology_legacy_preflight_preflight_report, brain_10_sessions_2026_11_17_topology_exclusions_exclusion_matcher, brain_10_sessions_2026_11_17_topology_dhcp_overrides_dhcp_override_binding, brain_10_sessions_2026_11_17_topology_dynamic_bounds_legacy_bounds_import, brain_10_sessions_2026_11_17_topology_node_minimums_node_minimum [INFERRED 0.85]
- **Durable recovery and runtime ownership** — brain_10_sessions_2026_11_16_topology_registry_registry_store, brain_10_sessions_2026_11_17_topology_migration_staging_staging_api, brain_handoff_runner_lock_recovery, brain_10_sessions_2026_11_17_topology_runtime_owner_runtime_owner, brain_10_sessions_2026_11_17_topology_migration_staging_backup_publisher [INFERRED 0.85]
- **Topology Manager activation gates** — brain_30_knowledge_topology_manager_status_manager_service_activation_gate, brain_10_sessions_2026_11_17_topology_shared_static_native_implicit_commitments, brain_30_knowledge_topology_manager_status_live_dpdk_acceptance, brain_10_sessions_2026_11_17_topology_shared_adoption_schema2_device_id_limit [INFERRED 0.85]
- **Static branch ownership pipeline (input, shared attachments, legacy adoption)** — src_topology_manager_static_input, src_topology_manager_shared_static, src_topology_manager_legacy_static, brain_10_sessions_2026_11_17_topology_shared_static_shared_attachments [EXTRACTED 1.00]
- **LibreQoS v3 write and reload safety decisions** — brain_20_decisions_adr_0002_v3_target_selection_target_auto, brain_20_decisions_adr_0011_v3_import_request_and_check_reload_request_json, brain_20_decisions_adr_0013_native_probe_native_probe, brain_20_decisions_adr_0010_first_full_libreqos_run_lastloaded_csv_rule [INFERRED 0.75]
- **Topology migration pipeline: preflight, adoption, staging, enrollment** — brain_handoff_legacy_preflight, brain_handoff_shared_legacy_adoption, brain_handoff_migration_staging, brain_handoff_runner_lock_recovery, brain_handoff_topology_enrollment [EXTRACTED 1.00]
- **Topology candidate pipeline: registry, catalog, resolver, aggregates, admission, publication** — brain_handoff_subscriber_registry, brain_handoff_catalog_schema, brain_handoff_placement_resolver, brain_handoff_aggregate_boundary, brain_handoff_compiled_admission, brain_handoff_publication_parity [INFERRED 0.85]
- **DPDK telemetry surfaced in TUI** — brain_handoff_dpdk_statistics_tasks, brain_handoff_bridge_health, brain_handoff_circuit_queue_diagnostics, brain_handoff_nic_diagnostics, brain_handoff_mirrored_traffic_chart, brain_handoff_seven_screen_tui [INFERRED 0.85]
- **LQoSync3 sync cycle data flow** — docs_architecture_config_loader, docs_architecture_collector, docs_architecture_circuit_model, docs_architecture_validator, docs_architecture_target_detector, docs_architecture_writers, docs_architecture_applier [EXTRACTED 1.00]
- **Topology Manager fresh-evidence evaluation and publication chain** — docs_design_topology_manager_implementation_reader_rs, docs_design_topology_manager_implementation_normalize_rs, docs_design_topology_manager_implementation_identity_rs, docs_design_topology_manager_implementation_observation_rs, brain_handoff_placement_resolver, docs_design_topology_manager_implementation_aggregate_rs, docs_design_topology_manager_implementation_publication_rs, docs_design_topology_manager_implementation_compiled_admission, docs_design_topology_manager_implementation_write_receipt_tracker [INFERRED 0.85]
- **DPDK telemetry features fed by the analytics monitor** — brain_handoff_worker_utilization, docs_operations_buffer_ring_pressure, docs_operations_whole_bridge_queue_statistics, docs_operations_mapped_unmapped_traffic, docs_operations_extended_worker_measurements, docs_operations_bridge_health_uptime, docs_operations_subscriber_dpdk_observations, docs_operations_on_demand_circuit_queues, docs_operations_native_nic_diagnostics [INFERRED 0.85]
- **Native policy acceptance and recovery investigation** — docs_operations_native_recovery_upstream_report, docs_operations_post_reboot_native_ack_diagnosis, docs_operations_explicit_slot_mapping, docs_operations_native_recovery_path_review, docs_operations_unified_health_task1, docs_operations_post_reboot_native_ack_diagnosis_non_suffix_structural_remove, docs_operations_native_recovery_upstream_report_allocation_capacity_64_vs_1000 [EXTRACTED 1.00]
- **ConnectX-4 Lx mlx5 dataplane remediation** — docs_reports_2026_11_02_hardware_upgrade_options_connectx_4_lx_nic, docs_reports_2026_11_03_mlx5_override_removal_txq_mem_algn_0, docs_reports_2026_11_03_mlx5_override_removal_mlx5_compat_dropin, docs_reports_2026_11_03_nic_firmware_upgrade_firmware_14_32_1912, docs_reports_2026_11_03_nic_firmware_upgrade_kernel_lockdown_blocks_mstflint [INFERRED 0.85]
- **Seven approved TUI screen bodies** — docs_design_tui_readme_dashboard_screen, docs_design_tui_readme_subscribers_screen, docs_design_tui_readme_routers_screen, docs_design_tui_readme_libreqos_screen, docs_design_tui_readme_workers_screen, docs_design_tui_readme_settings_editor, docs_design_tui_readme_journal_screen [EXTRACTED 1.00]
- **Topology Manager implementation slices** — docs_superpowers_plans_2026_11_16_topology_foundation, docs_superpowers_plans_2026_11_16_topology_registry, docs_superpowers_plans_2026_11_16_topology_catalog, docs_superpowers_plans_2026_11_16_topology_aggregates, docs_superpowers_plans_2026_11_16_topology_rebalance, docs_superpowers_plans_2026_11_16_topology_normalization [EXTRACTED 1.00]
- **DPDK analytics telemetry pipeline** — docs_superpowers_plans_2026_11_09_worker_utilization_analytics_ipc, docs_superpowers_plans_2026_11_09_worker_utilization_worker_cache, docs_superpowers_plans_2026_11_09_buffer_pressure_pressure, docs_superpowers_plans_2026_11_10_dpdk_statistics_qdisc_totals, docs_superpowers_plans_2026_11_12_worker_ram_cache_tmpfs_cache, docs_superpowers_plans_2026_11_10_mirrored_dpdk_chart_mirror_chart [INFERRED 0.85]
- **Settings edit-validate-preview-apply-verify flow** — docs_superpowers_plans_2026_11_01_lqosync3_tui_4b_draft, docs_superpowers_plans_2026_11_01_lqosync3_tui_4b_validate, docs_superpowers_plans_2026_11_01_lqosync3_tui_4b_preview, docs_superpowers_plans_2026_11_01_lqosync3_tui_4b_apply, docs_superpowers_plans_2026_11_01_lqosync3_tui_4b_verify [EXTRACTED 1.00]
- **Runner Topology Owner APIs** — docs_superpowers_plans_2026_11_17_topology_source_coordinator_runner_sync_topology, docs_superpowers_plans_2026_11_17_topology_enrollment_runner_enroll_topology, docs_superpowers_plans_2026_11_17_topology_runtime_owner_instance_lock_guard, docs_superpowers_plans_2026_11_17_topology_publication_receipt_tracker, docs_superpowers_plans_2026_11_17_topology_runtime_owner_controlled_compiler [INFERRED 0.85]
- **Legacy Migration and Adoption Flow** — docs_superpowers_plans_2026_11_17_topology_legacy_preflight_compatibility_report, docs_superpowers_plans_2026_11_17_topology_legacy_preflight_adopt_configured_legacy, docs_superpowers_plans_2026_11_17_topology_shared_adoption_legacy_static_rs, docs_superpowers_plans_2026_11_17_topology_migration_staging_migration_journal, docs_superpowers_plans_2026_11_17_topology_enrollment_runner_enroll_topology [INFERRED 0.85]
- **Fresh Source to Publication Pipeline** — docs_superpowers_plans_2026_11_17_topology_source_coordinator_coordinator_rs, docs_superpowers_plans_2026_11_17_topology_identity_identity_rs, docs_superpowers_plans_2026_11_16_topology_resolver_capacity_weighted_placement, docs_superpowers_plans_2026_11_17_topology_publication_preparedpair, docs_superpowers_plans_2026_11_17_topology_migration_admission_compiled_candidate_admission, docs_superpowers_plans_2026_11_17_topology_publication_receipt_tracker [INFERRED 0.85]
- **LibreQoS Target Selection and Writers** — docs_superpowers_specs_2026_10_31_lqosync3_v3_design_target_selection_auto, docs_superpowers_specs_2026_10_31_lqosync3_v3_design_v2_pair, docs_superpowers_specs_2026_10_31_lqosync3_v3_design_v3_pair, docs_superpowers_specs_2026_10_31_lqosync3_v3_design_v3_network [EXTRACTED 1.00]
- **Write-to-APPLIED Verification Flow** — docs_superpowers_specs_2026_11_11_unified_system_health_telegram_design_write_id, docs_superpowers_specs_2026_11_11_unified_system_health_telegram_design_write_apply_lifecycle, docs_superpowers_specs_2026_11_11_unified_system_health_telegram_design_pending_checker, docs_superpowers_specs_2026_11_11_unified_system_health_telegram_design_acceptance_evidence, docs_superpowers_specs_2026_11_11_unified_system_health_telegram_design_telegram_outbox [EXTRACTED 1.00]
- **TUI Theme Evolution toward btop++** — docs_superpowers_specs_2026_11_01_lqosync3_tui_cosmetics_design_theme_enum, docs_superpowers_specs_2026_11_01_lqosync3_tui_cosmetics_design_semantic_roles, docs_superpowers_specs_2026_11_01_lqosync3_btop_refinement_rgb_palette, docs_superpowers_specs_2026_11_01_lqosync3_btop_refinement_btop_default_theme, docs_superpowers_specs_2026_11_01_lqosync3_btop_refinement_btop_box [INFERRED 0.85]

## Communities (258 total, 42 thin omitted)

### Community 0 - "Runner Service Loop"
Cohesion: 0.06
Nodes (95): sync_topology(compiler, max_age, now), Runner topology APIs (prepare/enroll/lock/sync_topology), target_name(), Runtime, RuntimeMode, Apply, Shadow, Connector (+87 more)

### Community 1 - "Source Policy and Rates"
Cohesion: 0.04
Nodes (76): Session: Durable topology settings and forward recovery, authorize_configured, history_cleanup_pending, Topology store schema 3, Session: Source policy mapping and exact rates, Profile bindings to stable source profile references, service_names selectors, SourceRead / Local rate authority (+68 more)

### Community 2 - "LibreQoS Data and Network Render"
Cohesion: 0.06
Nodes (60): a_bad_network_json_keeps_the_last_data_and_shows_an_error(), a_missing_csv_is_an_error(), collect_names(), Data, Env, env(), FLAG_DUP_IP, FLAG_NO_PARENT (+52 more)

### Community 3 - "Registry Store and Staging"
Cohesion: 0.07
Nodes (60): absent(), BLOCKER, cancel(), check_ready(), directory(), Journal, JOURNAL, LIMIT (+52 more)

### Community 4 - "Project Handoff Topology Status"
Cohesion: 0.05
Nodes (73): Handoff, ADR-0004 TOML policy, Python defaults, ADR-0005 topology follows v3, ADR-0006 v3 guards and node types, ADR-0007 fresh design not a port, ADR-0008 terminal UI, ADR-0012 subscriber_grace = 0, Aggregate boundary (aggregate.rs) (+65 more)

### Community 5 - "Settings Draft Model"
Cohesion: 0.08
Nodes (31): a_bad_router_name_is_rejected(), a_change_keeps_comments_and_key_order(), add_and_remove_a_dhcp_server(), add_and_remove_a_router(), changed_counts_the_keys(), diff_lines_show_old_and_new(), Draft, FILE (+23 more)

### Community 6 - "Identity Authorization Tests"
Cohesion: 0.09
Nodes (62): CatalogRouter, authorize(), conflict(), IdentityIssue, AmbiguousScope, DuplicateReference, InvalidReference, InvalidSourceRate (+54 more)

### Community 7 - "Probe, Detect and Diff"
Cohesion: 0.06
Nodes (54): Libreqos, Target, Auto, V2Pair, V3Network, V3Pair, Detected, Env (+46 more)

### Community 8 - "Topology Resolver and Aggregates"
Cohesion: 0.06
Nodes (50): AggregateConflict, AggregateConflictReason, FloorExceedsCeiling, MinimumBelowDescendants, MinimumExceedsCeiling, AggregateNode, AggregateResolution, AggregateWarning (+42 more)

### Community 9 - "Topology Catalog and Rates"
Cohesion: 0.08
Nodes (50): imported_compares_devices_rates_and_nodes(), BackstopPolicy, CatalogIndex, GroupDefaults, SubscriberProfile, validate_speed(), factor_value(), import() (+42 more)

### Community 10 - "Sync Cycle"
Cohesion: 0.10
Nodes (52): a_row_that_stays_keeps_its_group_node(), AAA, an_override_gives_the_same_rate_in_v2_pair_v3_pair_and_v3_network(), cfg(), dhcp_cfg(), duplicate_ip_existing_vs_existing_keeps_collected(), duplicate_ip_new_vs_existing_keeps_existing(), empty_online_router_removes_its_rows() (+44 more)

### Community 11 - "Core Design and ADRs"
Cohesion: 0.07
Nodes (46): 2026-10-31 Plan 1: lqosync3 core library session, lqosync3 core crate (config, speed, model, state, sources, topology, cycle, validate, writer), Plan 1 review fixes (commit 529fa36), ADR-0005 Topology follows v3, 14/15 CSV columns (v2_pair/v3_pair), max_floor_mbps (0.2) / safe_min_mbps (0.1), Parent Node ID column, ADR-0006 v3 guards and node types (+38 more)

### Community 12 - "Legacy Topology Builder"
Cohesion: 0.10
Nodes (49): NodeTemplate, StoredNode, GroupKey, DhcpPlan, DhcpSite, Hotspot, Ppp, PppPlan (+41 more)

### Community 13 - "TUI View Navigation"
Cohesion: 0.14
Nodes (45): a_bad_number_shows_the_key_error(), a_changed_file_shows_the_disk_diff_and_can_restart(), a_failed_preview_blocks_apply(), a_failed_preview_shows_that_apply_is_blocked(), a_rejected_apply_says_the_new_file_is_on_disk(), a_secret_for_a_removed_router_is_dropped(), a_skipped_lease_cannot_get_an_override(), a_typed_password_is_never_in_the_buffer() (+37 more)

### Community 14 - "Subscriber Statistics"
Cohesion: 0.08
Nodes (39): addresses(), Applied, Baseline, CACHE, encoded_slots(), explicit_slots_keep_the_existing_mapping_size_bound(), explicit_slots_require_a_valid_present_field_and_preserve_legacy_absence(), explicit_slots_route_sparse_hosts_and_reset_rates_when_ids_change() (+31 more)

### Community 15 - "Topology Rebalance"
Cohesion: 0.11
Nodes (48): change(), Changes, compare_changes(), Delta, Loads, ProposedMove, rebalance(), RebalanceRequest (+40 more)

### Community 16 - "TUI Dashboard"
Cohesion: 0.08
Nodes (31): a_very_tall_area_does_not_overflow(), bridge_and_native_lines_show_only_on_libreqos_v3(), card_areas(), CPU_SCALE, dashboard_card_geometry(), dashboard_tab_renders(), DashboardView, data() (+23 more)

### Community 17 - "TUI Theme and Palette"
Cohesion: 0.09
Nodes (20): a_box_title_has_the_btop_brackets_and_hotkey(), a_panel_has_the_box_colour_and_marks_focus_in_the_title(), BoxKind, Cpu, Mem, Net, Proc, btop_gradient_ends() (+12 more)

### Community 18 - "Native Topology Upstream Reports"
Cohesion: 0.05
Nodes (49): [topology] compile_mode = full required for node tree, LibreQoS 3.0.7 native topology compiler loses node kinds and rates, compile_full_like (lqos_topology_compile compile_mode.rs), TopologyCanonicalStateFile::from_editor_and_network, native_full_keeps_node_policy.rs proposed upstream test, Defect: libreqos_network full/full2 compile exports nodes at 1 Mbps, Proposed fix: keep the native node policy in compile_full_like, Maintainer verification steps (+41 more)

### Community 19 - "Topology Registry Model and Preflight"
Cohesion: 0.07
Nodes (46): BaseRateAuthority, Local, SourceRead, issues(), SourceSelection, blank_policy_ids_and_conflicting_pin_are_rejected(), current_schema_checks_catalog_references_without_guessing_legacy_bindings(), duplicate_subscriber_ids_and_identity_ownership_are_errors() (+38 more)

### Community 20 - "Subscriber Observation"
Cohesion: 0.09
Nodes (42): Authorization, IdentityConflict, required_profile_refs(), identity(), IdentityRef, SourceScope, SubscriberPolicy, a_configured_catalog_requires_its_selected_collection_scopes() (+34 more)

### Community 21 - "State Files and Locking"
Cohesion: 0.07
Nodes (34): CircuitNames, a_replaced_lock_path_cannot_retain_writer_authority(), backup(), backup_keeps_the_last_n(), backup_keeps_the_new_folder_and_other_folders(), finish_secret(), HDR, imported() (+26 more)

### Community 22 - "RouterOS Normalization"
Cohesion: 0.12
Nodes (33): RosRow, Excluder, ips(), claim(), CurrentRouter, dhcp_accounts(), disabled(), endpoint() (+25 more)

### Community 23 - "Vault Home and Decisions"
Cohesion: 0.06
Nodes (34): Session 2026-10-31 project setup and v3 simulation, Preflight package update, Preflight update prepared, Preflight warnings, Changes, Checks, Compact layout regressions, Goal (+26 more)

### Community 24 - "Operations and DPDK Telemetry"
Cohesion: 0.07
Nodes (49): Buffer and ring pressure, Counter freshness, Buffer and ring pressure alerts, Pressure deployment pending, Worker utilization (Workers tab), [tui] settings, DPDK telemetry review, Shared adaptive rate formatter (bps/kbps/Mbps/Gbps) (+41 more)

### Community 25 - "TUI Journal"
Cohesion: 0.09
Nodes (26): git(), main(), buffer_keeps_the_last_n_lines(), journal_start_error_is_shown(), journal_stderr_is_shown(), journal_tab_renders(), JournalHandle, JournalMsg (+18 more)

### Community 26 - "Aggregate Tests"
Cohesion: 0.14
Nodes (47): a_deep_reversed_graph_uses_iterative_evaluation(), a_floor_cannot_increase_the_wan_or_bypass_a_backstop(), a_floor_does_not_create_an_empty_automatic_branch(), a_manual_group_uses_its_configured_factors_once(), a_stage_input_without_current_endpoint_evidence_cannot_supply_a_candidate(), aliases_and_endpoints_count_once_for_minimum_commitments(), an_explicit_parent_guarantee_must_cover_child_guarantees(), an_explicit_source_floor_raises_local_capacity_without_changing_the_calculation() (+39 more)

### Community 27 - "Telegram Notify"
Cohesion: 0.10
Nodes (38): a_bridge_counter_that_goes_up_gives_one_alert(), a_config_error_is_an_error_event(), a_long_multi_line_error_splits_into_valid_html_parts(), a_native_probe_that_becomes_ready_gives_one_notice(), a_very_long_error_is_cut(), an_empty_token_is_an_error(), an_error_is_sent_once_then_cleared(), an_error_with_other_numbers_is_not_a_new_event() (+30 more)

### Community 28 - "Settings Key Metadata"
Cohesion: 0.07
Nodes (42): DEFAULTS_TOML, all(), Base, Float, List, Text, build(), CHOICES (+34 more)

### Community 29 - "Configuration Loading"
Cohesion: 0.07
Nodes (36): a_bad_or_duplicate_mac_is_an_error(), a_circuit_comment_must_not_be_the_static_comment(), a_client_rate_below_the_floor_is_an_error(), ClientTemplate, deep_merge(), defaults(), defaults_table(), dhcp_client_mac_forms_load() (+28 more)

### Community 30 - "Telegram Outbox"
Cohesion: 0.11
Nodes (25): Telegram, drain(), enqueue(), event_text(), fixture(), Frozen, Generic, Job (+17 more)

### Community 31 - "TUI Devices View"
Cohesion: 0.12
Nodes (27): circuits_count_distinct_ids(), Column, .ALL, Down, Ipv4, Name, Parent, Source (+19 more)

### Community 32 - "System Health"
Cohesion: 0.07
Nodes (27): Acceptance, CACHE, Domain, .ALL, Apply, Collection, Dataplane, Notification (+19 more)

### Community 33 - "TUI Settings View"
Cohesion: 0.12
Nodes (4): path_text(), concrete(), SettingsView, Shown

### Community 34 - "TUI Network View"
Cohesion: 0.12
Nodes (22): a_deep_node_keeps_its_name_on_a_narrow_screen(), a_narrow_screen_keeps_a_short_rate(), children(), collapsed_tree_shows_only_roots(), expand_all_and_collapse_all(), key(), names(), narrow() (+14 more)

### Community 35 - "Worker Utilization IPC"
Cohesion: 0.10
Nodes (23): circuit_queue_request_uses_one_selected_nonzero_slot_and_strict_reply_kind(), decode(), health_uses_the_bounded_descriptor_transport_and_requires_its_reply_kind(), MAX_SNAPSHOT, native_ack_unavailable_does_not_leak_producer_reason(), native_applied_report_keeps_transaction_and_peer_identity(), read_health(), read_hosts() (+15 more)

### Community 36 - "Topology Compiler"
Cohesion: 0.08
Nodes (15): AdmittedPair, absolute(), check_directories(), CHILD, CompiledPair, CompilerBinding, CONFIG_LIMIT, ControlledCompiler (+7 more)

### Community 37 - "Worker Telemetry Cache"
Cohesion: 0.12
Nodes (34): bridge_health_survives_independent_snapshot_failures_and_public_cache_roundtrip(), cache_budget_failure_keeps_the_previous_public_cache_unchanged(), class_sample(), classified_traffic_rebases_after_failure_restart_counter_reset_and_gap(), classified_traffic_uses_independent_source_totals_not_actual_or_capped_host_rows(), dashboard_actual_rates_need_two_samples_and_rebase_after_failure(), extended_worker_cache_budget_preserves_basic_rows_and_histories_without_mutating_source(), extended_worker_metrics_survive_advanced_samples_and_old_rows_remain_unknown() (+26 more)

### Community 38 - "Router Sources"
Cohesion: 0.14
Nodes (35): parse_ipv4_net(), a_zero_speed_profile_comment_gives_one_line(), an_override_changes_only_that_client(), an_override_for_a_missing_mac_has_no_effect(), an_override_in_per_plan_moves_to_its_plan_node(), an_override_on_a_skipped_lease_gives_no_circuit(), by_name(), cfg() (+27 more)

### Community 39 - "Repro and Capture Scripts"
Cohesion: 0.10
Nodes (11): circuit_plan(), find_compat(), main(), make_conf(), native_input(), node_policy(), outside_writes(), policy_kept() (+3 more)

### Community 40 - "legacy_static_tests.rs"
Cohesion: 0.13
Nodes (26): MAX_PAIR_BYTES, read_regular(), prepare(), adopt(), adopt_configured(), extract(), StaticAdoption, adoption_rejects_stale_evidence_and_fresh_snapshot_policy_or_endpoint_drift() (+18 more)

### Community 41 - "TopologyCatalog"
Cohesion: 0.08
Nodes (21): CapacityPolicy, Automatic, Fixed, Inherit, WanPercentage, CatalogNode, FactorChoice, Inherit (+13 more)

### Community 42 - "preview.rs"
Cohesion: 0.15
Nodes (30): a_failed_preview_is_failed_not_zero(), a_new_router_effect_is_not_known(), a_rate_change_gives_the_expected_circuit_changes(), a_router_that_was_not_online_gives_a_warning(), a_source_turned_on_is_not_known(), ALICE, an_old_snapshot_gives_the_stale_warning(), cfg() (+22 more)

### Community 43 - "CsvRow"
Cohesion: 0.10
Nodes (26): Columns, V2, V3, CsvRow, compare_mapped(), fingerprint(), nodes(), remap_tree() (+18 more)

### Community 44 - "catalog_tests.rs"
Cohesion: 0.12
Nodes (35): a_large_catalog_uses_iterative_validation(), automatic(), catalog(), custom_group_defaults_are_independent_of_discovered_group_factors(), duplicate_ids_origins_and_source_parent_instances_are_rejected(), extra_wan_inventory_does_not_expand_existing_targets_or_instances(), factor_overrides_replace_shared_values_and_preserve_manual_bases(), fixed() (+27 more)

### Community 45 - "resolver_tests.rs"
Cohesion: 0.18
Nodes (35): a_single_profile_balances_plan_demand_across_wans_without_changing_maxima(), absence_keeps_intent_but_removes_active_placement_and_return_restores_it(), an_ancestor_target_restriction_cannot_be_bypassed_by_a_broader_source(), an_explicit_target_set_restricts_other_instances_of_the_same_source(), checked_placements_persist_across_restart_without_current_observations(), collect(), conflicting_active_alias_plans_block_a_single_circuit(), conflicting_source_defaults_need_an_explicit_profile_policy() (+27 more)

### Community 46 - "app.rs"
Cohesion: 0.16
Nodes (32): a_click_on_a_tab_title_selects_it(), a_wide_header_is_one_line(), all_tabs_themes_and_size_matrix(), app(), app_dialogs_show_their_keys_and_only_y_n_esc_act(), approved_shell_starts_on_dashboard_and_has_seven_routes(), bridge_health_dashboard_retains_freshness_when_worker_snapshot_is_stale(), circuit_queue_action_is_available_in_read_only_tui_without_a_write_confirmation() (+24 more)

### Community 47 - "Configuration"
Cohesion: 0.08
Nodes (35): Architecture, Applier (hash, atomic write, reload), Circuit model (BTreeMap), Collector (RouterOS API sessions), Config loader, lqosync3 crate layout (proposed), Six network modes (from updatecsv.py), Notifier (Telegram) (+27 more)

### Community 48 - "Config"
Cohesion: 0.13
Nodes (21): Config, Exclude, normalize_mac(), RouterCfg, RouterSnapshot, collect_dhcp(), collect_hotspot(), collect_ppp() (+13 more)

### Community 49 - "cli.rs"
Cohesion: 0.12
Nodes (26): check(), check_finds_plain_api_and_no_source_router(), check_reads_the_telegram_token_when_enabled(), check_reports_a_bad_secret_and_a_missing_lqos_dir(), check_shows_the_native_probe_of_libreqos_v3(), check_shows_the_target_or_the_detection_error(), check_warns_when_the_file_is_not_loaded(), check_warns_when_the_libreqos_state_directory_is_missing() (+18 more)

### Community 50 - "approved_key"
Cohesion: 0.13
Nodes (12): circuit_queue_mouse_action_is_manual_bound_to_selection_and_copyable(), DevicesView, filters_copy_and_visible_row_work_are_bounded(), Input, Copy, None, Notice, Parent (+4 more)

### Community 51 - "normalize_tests.rs"
Cohesion: 0.20
Nodes (34): normalize(), all_documented_unbound_lease_states_are_absent_and_unknown_states_fail(), conflicting_current_ip_ownership_across_protocols_is_not_hidden(), dhcp(), dhcp_bound_uses_current_fields_and_retains_endpoint_only_evidence(), direct_dhcp_rate_input_stays_current_and_uninterpreted_without_a_default(), disabled_missing_or_ambiguous_ppp_accounts_fail_closed(), duplicate_or_missing_profiles_do_not_invent_current_membership_or_rates() (+26 more)

### Community 52 - "Topology Manager discussion"
Cohesion: 0.10
Nodes (26): Fresh membership and stable placement, Placement resolver (resolver.rs), Scoped rebalance (rebalance.rs), DHCP client speed override ([[routers.dhcp.clients]] by MAC), Topology Manager discussion, Possible CRM (future extension, not authorized), Decision 7: custom-node capacity policy, Decision 9: discovery changes and reconciliation (+18 more)

### Community 53 - "mock_router.rs"
Cohesion: 0.15
Nodes (27): MikrotikConnector, read_router(), Answer, Empty, Raw, Rows, Silent, Trap (+19 more)

### Community 54 - "verify.rs"
Cohesion: 0.15
Nodes (23): Artifacts, boot_restored_partial_world_cannot_accept_a_newer_import(), check_compiler(), compiler_drift_invalidates_prepared_and_already_constructed_proofs(), fixture(), input_changes_restarts_and_wrong_runtime_evidence_cannot_accept(), only_correlated_evidence_can_accept_the_same_operation(), policy_acceptance_keeps_allocation_mismatch_degraded() (+15 more)

### Community 55 - "coordinator_tests.rs"
Cohesion: 0.12
Nodes (28): cancelled_batches_close_sessions_and_next_attempt_reads_again(), changed_router_binding_cannot_reuse_the_previous_routers_session(), clearing_sessions_reconnects_without_reusing_generation_numbers(), configured_filtered_adoption_requires_current_filter_proof_and_exact_mappings(), Connection, connection_time_counts_against_the_batch_deadline(), Data, data() (+20 more)

### Community 56 - "speed.rs"
Cohesion: 0.12
Nodes (15): circuit_rates(), CircuitRates, clamp(), default_rate_and_circuit_rates(), finite(), legacy(), p(), parse_default_rate() (+7 more)

### Community 57 - "Topology catalog and hierarchy persisten"
Cohesion: 0.11
Nodes (26): Topology catalog and hierarchy persistence, Local vs SourceRead rate authority, Topology catalog (catalog.rs), Topology foundation, ID-based forest layout validator (layout.rs), Integer bps rate arithmetic (rates.rs), Topology Manager adoption research, DHCPv4 presence as status=bound (+18 more)

### Community 58 - "context"
Cohesion: 0.11
Nodes (16): main(), read(), claim(), Owner, topology_owns(), CurrentStatic, Document, document() (+8 more)

### Community 59 - "2026-10-31-lqosync3-v3-design.md"
Cohesion: 0.09
Nodes (24): ADR-0002 v3 Target Selection, CLI, Config changes, libreqos.dpkg_status Key, libreqos_network.json (schema_version 2), lqosd File Watcher, lqosync3 detect CLI, lqosync3 LibreQoS v3 targets (Plan 3) (+16 more)

### Community 60 - "Topology Manager implementation design"
Cohesion: 0.12
Nodes (30): Configured source coordinator (Runner::sync_topology), topology-manager-status knowledge note, Topology Manager implementation design, Catalog boundary (catalog.rs), Compiled-candidate admission (check_compiled), ControlledCompiler (Landlock/seccomp sandboxed child), Current-source exclusions (reused Excluder, filter digest), Independent reference authorization (identity.rs) (+22 more)

### Community 61 - "routeros.rs"
Cohesion: 0.11
Nodes (21): a_dead_kept_session_reconnects_without_a_retry(), a_login_error_is_not_retried(), Fake, FakeSession, gives_last_error_after_all_retries(), hotspot_inventory_is_read_only_for_enabled_source_without_password_fields(), ppp_router(), read_snapshot() (+13 more)

### Community 62 - "RouterResult"
Cohesion: 0.11
Nodes (25): assemble(), Change, changed_columns(), CycleInput, CycleOutput, dhcp_snap(), names(), node_rate() (+17 more)

### Community 63 - "Procedure: remove the mlx5 compatibility"
Cohesion: 0.09
Nodes (21): LibreQoS v3 mlx5 compatibility override, ConnectX-4 Lx NIC (mlx5_core), DPDK 25.11, Live DPDK acceptance gate, Hardware audit and CPU/RAM upgrade options for the LibreQoS host, bridge_rs 100% core is DPDK poll mode, not overload, Mellanox ConnectX-4 Lx dataplane NIC (fw 14.25.1020), HP ProDesk 400 G5 SFF (i5-9500, 16 GB) (+13 more)

### Community 64 - "runner::Runner::cycle (one full cycle)"
Cohesion: 0.08
Nodes (26): ADR-0003 router failed vs offline, lqosync3.service systemd unit, Operational rules (one sync tool in apply mode), routerstate::classify, Core design spec (2026-10-31-lqosync3-core-design.md), cycle::assemble (pure core), files::backup (keep_backups), files::lock (lqosync3.lock) (+18 more)

### Community 65 - "LibreQoS v3 ingress"
Cohesion: 0.10
Nodes (28): Target detector, v2_pair target (ShapedDevices.csv + network.json, LibreQoS.py --updateonly), v3_network target (libreqos_network.json), v3_pair target (15-column pair, lqosd watcher), Writers (one per target), LibreQoS target selection (target = auto), Optional node minimums (download_min_mbps), Journal mockup generation record (+20 more)

### Community 66 - "PreparedPair (prepare(AuthorizedReconcil"
Cohesion: 0.11
Nodes (24): Native node export precision, Compiler rounds node 100.5 Mbps to intrinsic 101 Mbps, node_export_adjustments preview field, Current-Source Exclusion Filters Plan, Legacy Excluder Reuse (IP/PPP profile/DHCP server), Prepared Pair Exclusion-Policy Binding, Explicit Migration and Compilation Admission Plan, Compiled Candidate Admission (admission.rs) (+16 more)

### Community 67 - "ownership.rs"
Cohesion: 0.13
Nodes (21): adopt_configured_legacy(), adopt_legacy(), AliasIndex, CircuitAlias, DeviceAlias, ExportOwnership, merge_static(), NodeAlias (+13 more)

### Community 68 - "journal/approved.rs"
Cohesion: 0.11
Nodes (12): controls_wheel_drag_and_display_preserve_original_log_copy(), JournalAction, Clear, Copy, Key, None, Select, JournalUi (+4 more)

### Community 69 - "workers/approved.rs"
Cohesion: 0.15
Nodes (11): bridge_health_copy_uses_independent_health_freshness_and_preserves_source_status(), classified_copy_preserves_scope_and_current_data_age(), extended_worker_details_and_copy_keep_units_phase_names_and_stale_labels(), nic_copy_has_independent_freshness_and_preserves_vendor_raw_values(), queue_copy_preserves_scope_and_suppresses_stale_increments(), sample_filters_expire_on_ticks_and_copy_only_selected_worker_evidence(), worker_detail_mouse_focus_and_page_keys_scroll_without_changing_worker(), WorkerAction (+3 more)

### Community 70 - "store_recovery_tests.rs"
Cohesion: 0.14
Nodes (23): error_never_contains_the_secret(), file(), missing_file_names_the_path(), read_secret(), reads_and_trims_a_good_secret(), rejects_group_or_other_bits(), rejects_wrong_owner(), backup_path() (+15 more)

### Community 71 - "collection.rs"
Cohesion: 0.13
Nodes (24): CACHE, cache_round_trip_is_public_sanitized_bounded_and_versioned(), collection_uses_raw_inventory_sessions_and_lease_state_without_secrets(), config(), disabled(), failed_reads_retain_evidence_but_never_turn_subscribers_offline(), field(), mac_key() (+16 more)

### Community 72 - "arc"
Cohesion: 0.10
Nodes (4): AnyCertificate, map_err(), MikrotikSession, tls_client_config()

### Community 73 - "deserialize"
Cohesion: 0.10
Nodes (7): Coordinator, Coordinator<C>, same_connection(), SourceSession, ReferenceAuthority, OperatorManagedComment, Unconfigured

### Community 74 - "tui/mod.rs"
Cohesion: 0.09
Nodes (9): action_timeout(), fresh_pid(), install_panic_hook(), panic_hook_restores_only_on_its_own_thread(), restore_terminal(), RESTORED, run(), TerminalGuard (+1 more)

### Community 75 - "import_evidence.rs"
Cohesion: 0.17
Nodes (15): addresses(), artifact(), Device, device(), duplicate_identities_cannot_hide_a_missing_device(), equivalent_address_order_and_numeric_notation_match(), expected_fields_follow_the_producer_csv_trimming(), ExpectedImport (+7 more)

### Community 76 - "reader_tests.rs"
Cohesion: 0.16
Nodes (22): RouterRole, Dhcp, Hotspot, Ppp, Wan, ReadPlan, all_roles_complete_once_with_inventory_checks_and_empty_presence(), changed_inventory_rejects_the_entire_read_but_reply_order_does_not() (+14 more)

### Community 77 - "preflight_tests.rs"
Cohesion: 0.15
Nodes (26): configured_adoption_requires_both_policy_and_exact_snapshot_checks(), dhcp_aliases_cannot_hold_conflicting_override_rates(), dhcp_endpoint_overrides_require_identity_policy_even_when_no_matching_lease_is_online(), dhcp_identity_override_accepts_enrolled_offline_subscriber_without_mac_lookup(), dhcp_identity_override_rejects_incomplete_duplicate_or_stale_bindings(), dhcp_identity_override_requires_durable_identity_and_round_trips_without_endpoints(), dhcp_membership_and_server_rules_cannot_be_silently_dropped(), dhcp_override_coverage_is_shared_across_disjoint_service_scopes() (+18 more)

### Community 78 - "App"
Cohesion: 0.15
Nodes (12): Action, ApplySettings, Copy, None, Preview, QueueDiagnostics, Quit, Refresh (+4 more)

### Community 79 - "shared_static.rs"
Cohesion: 0.12
Nodes (20): Session: Exact shared legacy adoption proposals, legacy_static::adopt / adopt_configured, Schema-2 circuit 64-char device ID limit, Snapshot parity and tree remap, StaticAdoption, Session: Explicit shared static branches, Native implicit commitments gate, ExportOwnership::shared_attachments (+12 more)

### Community 80 - "workers.rs"
Cohesion: 0.13
Nodes (13): Theme, Dark, Light, Mono, Tui, WorkerUi, live(), pressure_view_shows_causes_measurements_and_never_calls_stale_values_clear() (+5 more)

### Community 81 - "Topology Foundation plan"
Cohesion: 0.14
Nodes (26): Cycle collection and subscriber state plan, Sanitized collection cache from sync-cycle RouterOS reads, Optional node minimums plan, Optional whole-Mbps node minimums (Network, NodeTemplate, StoredNode), Aggregate capacity plan, aggregate.rs pure stage after placement resolver, GroupDefaults.floor optional explicit floor, Topology Catalog plan (+18 more)

### Community 82 - "settings/secrets.rs"
Cohesion: 0.16
Nodes (13): a_failed_rollback_is_reported(), a_failed_secret_write_rolls_back_the_earlier_ones(), a_symlink_or_a_shared_file_is_not_a_secret_target(), check_target(), old_link(), Pending, pending(), rollback_renames_the_old_file_back_and_commit_removes_the_copy() (+5 more)

### Community 83 - "Topology Manager initial TUI layout"
Cohesion: 0.09
Nodes (20): Topology Manager initial TUI layout, Assignments view (pins, scoped rebalance), Builder view (Discovery Inventory, Topology Tree, Details), Changes view (optional diff/history), Policies view (Subscriber Speed Policy min/max), Policies > Speed Schedules (named profiles, timezone default Asia/Manila), LibreQoS Topology Manager adoption research, Acceptance checks before activation (no cycles, rename preserves ID) (+12 more)

### Community 84 - "traffic_chart.rs"
Cohesion: 0.12
Nodes (13): Cache, chart_axis_labels_match_the_plotted_bounds_with_stable_steps(), chart_segments_preserve_missing_samples_time_gaps_spikes_and_selected_window(), history(), independent_rate_ceilings_fill_each_half_without_changing_stored_points(), mirrored_upload_coordinates_do_not_change_rates_or_peaks(), NAMES, Plot (+5 more)

### Community 85 - "circuit_queue.rs"
Cohesion: 0.16
Nodes (16): circuit_queue_fetch_rejects_mapping_changes_and_reads_only_one_slot(), circuit_queue_gate_binds_completions_throttles_requests_and_retains_failed_refreshes(), circuit_queue_labels_keep_source_age_unknown_stale_and_gap_states_distinct(), circuit_queue_maximum_selected_details_stay_bounded(), circuit_queue_unknown_fields_histogram_bounds_and_requested_identity_are_safe(), Direction, fetch(), MAX_AGE_MS (+8 more)

### Community 86 - "routers.rs"
Cohesion: 0.18
Nodes (8): Router, router_filters_navigation_and_snapshot_refresh_preserve_selection(), RouterAction, Copy, None, Settings, Subscribers, RoutersView

### Community 87 - "SettingsView"
Cohesion: 0.19
Nodes (8): CATEGORIES, click(), draw(), dropdown_list_and_scroll_mouse_controls_use_existing_editor_actions(), full_router_form_validates_before_draft_and_keeps_secrets_in_memory(), key(), mouse_apply_cannot_bypass_inline_edit_preview_or_modal_guards(), SettingsView

### Community 88 - "src/write_receipt.rs"
Cohesion: 0.09
Nodes (22): Phase, Applied, ApplyFailed, Candidate, ImportConfirmed, Intent, Interrupted, Pending (+14 more)

### Community 89 - "classification.rs"
Cohesion: 0.15
Nodes (11): Classification, classification_histories_remain_bounded_and_absent_sources_do_not_accumulate_empty_rows(), classification_measurements_distinguish_zero_unknown_warming_and_stale(), Series, Tracker, Counters, format_rate(), History (+3 more)

### Community 90 - "nic_stats.rs"
Cohesion: 0.13
Nodes (14): BASIC, CACHE, MAX_AGE, MAX_BYTES, POLL_SECS, Port, publish(), read_cache() (+6 more)

### Community 91 - "ADR-0007 Fresh design, not a port"
Cohesion: 0.16
Nodes (19): Session: updatecsv.py dissection, Orphan branch lqosync3, RouterOS listen command (untested), ADR-0003 Router failed vs offline, offline_action (default remove), offline_after (default 300 s), ADR-0004 TOML policy, Python defaults, Speed source order TOML lists (+11 more)

### Community 92 - "overview.rs"
Cohesion: 0.23
Nodes (7): bridge_degradation_is_visible_in_operational_alerts_independent_of_snapshot(), chart_dynamic_values_retain_age_zero_missing_and_historical_peak_labels(), chart_maximum_history_redraw_cost_and_point_bounds(), combined_chart_defaults_to_all_sources_and_keeps_mouse_focus_and_resizing_safe(), directional_ceilings_track_their_own_visible_window_peaks(), health_panel_scrolls_wrapped_diagnostics_with_mouse_and_keyboard(), Overview

### Community 93 - "BandwidthDefaults"
Cohesion: 0.11
Nodes (14): BackstopCap, BackstopCaps, BandwidthDefaults, DhcpCfg, DhcpClient, DhcpMode, PerPlan, PerSite (+6 more)

### Community 94 - "Status"
Cohesion: 0.14
Nodes (17): change_details(), circuit_line(), cycle_telemetry(), updated_line(), CircuitInfo, ReportSummary, RouterStatus, Status (+9 more)

### Community 95 - "admission.rs"
Cohesion: 0.16
Nodes (17): Write, Budget, check_artifacts(), check_compiled(), check_content(), check_pair_size(), MAX_ARTIFACT_BYTES, another_pair_or_expired_source_cannot_use_compilation_evidence() (+9 more)

### Community 96 - "layout.rs"
Cohesion: 0.20
Nodes (21): automatic(), deep_tree_uses_bounded_iterative_validation(), direct_wan_subscribers_keep_explicit_capacity(), empty_manual_nodes_still_require_valid_capacity(), intermediate_modes_do_not_recompute_subscriber_demand(), invalid_ids_and_missing_parents_are_rejected(), labels_are_not_identity_keys(), mixed_direct_subscribers_and_child_groups_are_rejected() (+13 more)

### Community 97 - "publication_tests.rs"
Cohesion: 0.21
Nodes (22): admitted(), excessive_hierarchy_depth_is_a_conflict_before_recursive_json_work(), expired_observations_cannot_prepare_a_pair(), explicit_minimums_export_only_the_declared_direction_and_preserve_ceiling(), export_adjustments_are_visible_and_cannot_undercut_a_configured_floor(), export_quantization_cannot_erase_a_subscriber_minimum_commitment(), exported_child_ceilings_stay_within_wan_capacity_without_clamping_subscriber_maxima(), fractional_node_ceiling_uses_supported_whole_mbps_without_rounding_up() (+14 more)

### Community 98 - "tests/write_receipt.rs"
Cohesion: 0.23
Nodes (19): main(), a_crash_between_receipt_and_event_can_recover_the_same_identity(), compiler_binding_changes_create_new_receipts_without_replacing_matching_bytes(), csv_only_update_does_not_claim_a_network_replacement(), event_capacity_is_visible_and_does_not_block_file_replacement(), failed_durable_intent_stops_both_file_replacements(), interrupted_complete_files_can_be_reconciled_without_another_write(), interrupted_intent_cannot_be_promoted_without_reconciliation() (+11 more)

### Community 99 - "cache.rs"
Cohesion: 0.28
Nodes (16): failed_runtime_initialization_preserves_unknown_public_target(), failed_runtime_rename_cleans_temporary_cache_before_disk_fallback(), missing_runtime_data_is_unavailable_until_new_publisher_recovers(), owned_dir(), preexisting_private_runtime_directory_keeps_its_permissions(), public_link(), Publisher, ram() (+8 more)

### Community 100 - "Role"
Cohesion: 0.12
Nodes (11): draw(), enabled(), Shell, sidebar_width(), Role, Accent, Base, Error (+3 more)

### Community 101 - "PreparedPair preparation (publication.rs"
Cohesion: 0.14
Nodes (18): Durable legacy ownership and source priorities, adopt_legacy ownership adoption (ownership.rs), prepare_with_static, Registry schema 4 aliases and backups, Explicit migration mappings and compilation admission, AdmittedPair compilation admission (admission.rs), Migration manifests (migration.rs), Explicit node minimums (+10 more)

### Community 102 - "tests.rs"
Cohesion: 0.16
Nodes (14): absent_counters_stay_unknown_and_new_counters_need_a_baseline(), alerts_require_fresh_source_and_new_error_counters_never_infer_packet_loss(), bounded_cache_rejects_corruption_and_retains_zero_unknown_and_stale(), counters_rebase_on_failures_gaps_resets_restarts_and_port_replacement(), maximum_nic_report_fits_cache_budget_and_bounded_detail_formatting(), native_seqpacket_transport_matches_verified_live_shape_and_only_reads(), response(), server() (+6 more)

### Community 103 - "ReadError"
Cohesion: 0.19
Nodes (9): ReadError, Protocol, Tls, Unreachable, index_rows(), protocol(), read_table(), ReadGuard (+1 more)

### Community 104 - "prepare_with_static"
Cohesion: 0.21
Nodes (17): AuthorizedReconciliation, prepare(), prepare_with_static(), rate_text(), attached_static_guarantees_share_the_wan_budget_with_managed_minimums(), attachment_collisions_and_circuit_parent_splits_block_the_whole_candidate(), attachment_policy_rejects_missing_duplicate_and_subscriber_group_parents(), compiled_evidence_cannot_move_or_remove_an_attached_static_root() (+9 more)

### Community 105 - "Installation"
Cohesion: 0.13
Nodes (19): Runtime modes (shadow / apply), Installation, install.sh release installer, New LibreQoS installation (first full run), Checks after installation, Read-only RouterOS user (policy read,api), Cutover from the Python service, Router TLS with own CA (+11 more)

### Community 106 - "ci.yml"
Cohesion: 0.11
Nodes (19): jobs, msrv, test, runs-on, steps, name, on, pull_request (+11 more)

### Community 107 - "Hub"
Cohesion: 0.25
Nodes (3): Hub, sha256(), Tracker

### Community 108 - "nic_stats/ipc.rs"
Cohesion: 0.19
Nodes (6): Connection, counters(), field(), MAX_REPLY, ports(), sample()

### Community 109 - "lqosync3 runtime design (Plan 2)"
Cohesion: 0.15
Nodes (14): lqosync3 runtime design (Plan 2), Atomic write (temp file, sync_all, rename, fsync folder; only on byte change), Files: lock, state.json, status.json, candidate, applied, First-run seed (seed_state, seed_nodes from live files), Instance lock <state>/lqosync3.lock (File::try_lock), mikrotik-tokio 0.1.0 16-reply burst cancel bug, Plan 2 scope (in / not in), Process: run loop, SIGUSR1, SIGHUP, SIGTERM/SIGINT, once, check, diff, defaults (+6 more)

### Community 110 - "key_path"
Cohesion: 0.12
Nodes (16): KeyPath, export_settings_fixture_frames(), Group, group_change_counts(), group_counts_match_segments_and_removed_children(), Input, AddRouter, AddServer (+8 more)

### Community 111 - "presence.rs"
Cohesion: 0.21
Nodes (12): Subscriber, ambiguous_dhcp_observations_do_not_assign_a_false_state_to_shaping(), count(), Evidence, fresh(), mac(), Projection, projection_adds_inactive_inventory_without_changing_shaping_and_marks_stale() (+4 more)

### Community 112 - "health.rs"
Cohesion: 0.19
Nodes (12): bridge_health_has_independent_progress_expiry_failure_and_restart_states(), bridge_health_missing_worker_fields_and_unicode_errors_remain_bounded_unknowns(), bridge_health_old_fields_zero_and_unknown_status_are_not_healthy_defaults(), bridge_health_rejects_unbounded_text_workers_and_control_sequences(), Health, MAX_AGE, POLL_SECS, Report (+4 more)

### Community 113 - "source_rate_tests.rs"
Cohesion: 0.11
Nodes (8): RateParseError, FractionalBps, InvalidFormat, NonPositive, OutOfRange, rate(), ros(), zero_negative_and_malformed_values_fail_without_raw_input_in_errors()

### Community 114 - "design.rs"
Cohesion: 0.20
Nodes (8): accent(), panel(), role(), Scroll, scrollbar_wheel_track_drag_and_bounds(), selected(), surface(), viewport_rect()

### Community 115 - "libreqos.rs"
Cohesion: 0.22
Nodes (5): LibreAction, None, Sync, LibreView, native_missing_parent_evidence_survives_errors_until_new_valid_recovery()

### Community 116 - "pressure.rs"
Cohesion: 0.21
Nodes (8): Diagnostics, diagnostics(), Pressure, pressure_gauges_detect_local_ring_pressure_and_exact_boundaries_without_overflow(), pressure_journal_deduplicates_causes_and_does_not_call_a_failure_recovery(), pressure_missing_invalid_and_zero_capacity_are_not_healthy_zeros(), pressure_zero_rings_is_a_valid_single_worker_topology(), TransitionLog

### Community 117 - "delivery.rs"
Cohesion: 0.18
Nodes (11): answer(), api_success_requires_message_receipt(), ApiReceipt, DeliveryError, html_parts(), long_subscriber_html_has_bounded_parts(), post_receipt(), unicode_entities_and_tags_survive_parts() (+3 more)

### Community 118 - "lqosync3 terminal UI (Plan 4)"
Cohesion: 0.11
Nodes (18): 10. Phases, 11. Tests, 12. Left to the builder, 1. Intent, 2. Goals, non-goals, anti-goals, 3. Constraints, 4. Approaches, 5. Changes to the service (Plan 4a) (+10 more)

### Community 119 - "apply_evidence.rs"
Cohesion: 0.19
Nodes (12): artifacts(), main(), native_capture_is_private_and_preserves_transaction_representation(), operation_counts(), private_capture(), read(), Applied, read_applied() (+4 more)

### Community 120 - "steps"
Cohesion: 0.11
Nodes (17): GH_TOKEN, jobs, release, name, on, push, permissions, contents (+9 more)

### Community 121 - "monitor.rs"
Cohesion: 0.16
Nodes (6): Monitor, now(), Settings, Stamp, stamp(), timeout()

### Community 122 - "apply.rs"
Cohesion: 0.25
Nodes (15): config_digest(), a_changed_file_on_disk_stops_the_apply(), a_toml_write_error_rolls_back_the_secrets(), apply(), apply_writes_a_backup_and_keeps_n(), ApplyError, Changed, Io (+7 more)

### Community 123 - "on_key"
Cohesion: 0.20
Nodes (15): a_key_press_clears_the_message_so_errors_show(), an_open_field_takes_q_and_digits(), journal_keyboard_focus_and_expansion_use_approved_controls(), key(), keys_1_to_7_select_the_tabs_and_q_quits(), libreqos_sync_click_keeps_read_only_and_confirmation_gates(), light_popup_restores_canvas_after_clear(), minimum_dashboard_keeps_outcomes_and_long_router_states() (+7 more)

### Community 124 - "draw"
Cohesion: 0.14
Nodes (11): a_long_hint_wraps_without_a_lost_key(), action_messages_keep_normal_style(), center(), dialog(), Modal, ConfirmSync, Discard, modal_keys() (+3 more)

### Community 125 - "compiler_sandbox.rs"
Cohesion: 0.12
Nodes (4): Beneath, child_cannot_write_outside_state_or_open_sockets(), command(), Ruleset

### Community 126 - "topology_foundation.rs"
Cohesion: 0.28
Nodes (12): fmt_rate(), aggregate_stage_preserves_legacy_ppp_factors_backstop_and_explicit_floor(), config(), configured_backstop_limits_preserve_local_target_and_rendered_ceiling(), decimal_mbps_round_trips_preserve_integer_bps(), empty_legacy_group_floor_is_an_explicit_migration_target(), fraction(), hotspot_uses_count_times_unit_and_independent_factors() (+4 more)

### Community 127 - "Mockup generation record (TUI PROMPTS)"
Cohesion: 0.13
Nodes (14): Mockup generation record (TUI PROMPTS), Dashboard v2 mockup (LibreQoS DPDK Traffic title), DESIGN PREVIEW | SAMPLE DATA label, image_gen.imagegen reference-image generation, Routers v1 mockup prompt, Bridge health extension (uptime, degraded subsystems), Dashboard screen, Routers screen (+6 more)

### Community 128 - "LQoSync3 TUI visual contract"
Cohesion: 0.16
Nodes (15): LQoSync3 TUI visual contract, Circuit queue diagnostics extension, TUI color tokens (canvas #0b0c0e ... accent #80aaff), Journal screen, LibreQoS screen, LQoSync4 visual system design inspiration, Sidebar navigation order 1-7 (Dashboard..Journal) and roadmap, Bounded OSC 52 copy path (unconfirmed result) (+7 more)

### Community 129 - "Source Evidence and IDs Plan"
Cohesion: 0.18
Nodes (14): Source Evidence and IDs Plan, Guarded Current RouterOS Reader (reader.rs), ids.rs (topology_manager), RouterSession Trait, DHCP Identity Overrides Plan, DhcpOverrideBinding { legacy_client_index, account_ref, rate }, Durable Identity Authorization Plan, identity.rs (topology_manager) (+6 more)

### Community 130 - "migration_tests.rs"
Cohesion: 0.18
Nodes (9): duplicate_or_many_to_one_identity_maps_are_critical_conflicts(), explicit_mappings_allow_identity_and_label_equivalence_without_mutating_inputs(), explicit_node_maps_preserve_ancestry_and_never_hide_rate_or_metadata_changes(), fresh_manifest_cannot_hide_changed_rates_or_endpoints(), manifest_json_has_no_endpoint_or_session_fields_and_rejects_unknown_input(), old_pair(), protected_or_unclassified_rows_stay_visible_and_cannot_be_adopted_implicitly(), stale_generations_revisions_or_content_digests_cannot_claim_parity() (+1 more)

### Community 131 - "node_minimums.rs"
Cohesion: 0.21
Nodes (11): config(), network(), node_minimums_are_absent_by_default_and_old_state_still_loads(), node_minimums_follow_manual_edits_in_preserve_mode(), node_minimums_preserve_operator_values_with_state_from_before_the_upgrade(), node_minimums_preserve_seeded_operator_values(), node_minimums_reject_a_group_ceiling_that_shrinks_below_the_minimum(), node_minimums_reset_removes_generated_fields_in_preserve_mode() (+3 more)

### Community 132 - "src/validate.rs"
Cohesion: 0.31
Nodes (13): check(), collect_nodes(), duplicate_circuit_id_stops(), min_above_max_or_bad_number_stops(), missing_parent_stops_except_in_no_parent(), multi_device_circuit_passes_but_shared_device_id_stops(), net(), parent_node_id_must_match_the_parent_node() (+5 more)

### Community 133 - "main.rs"
Cohesion: 0.26
Nodes (10): exit_with(), main(), notify(), now(), print_summary(), queue_notify(), run(), SECRET_OWNER_UID (+2 more)

### Community 134 - "compiler_tests.rs"
Cohesion: 0.26
Nodes (14): a_child_cannot_change_inputs_or_native_topology_configuration(), compiled(), compiler_snapshot_preserves_exact_bytes_without_a_text_newline(), controlled_child_artifacts_admit_only_the_exact_fresh_pair(), deadline_and_mid_compile_config_drift_reject_otherwise_valid_artifacts(), drift_and_missing_outputs_block_without_a_native_apply(), fixture(), installed_binding_preserves_the_pair_inside_the_kernel_boundary() (+6 more)

### Community 136 - "super"
Cohesion: 0.16
Nodes (5): MAX_COPY, malformed_phase_lengths_and_idle_values_do_not_corrupt_other_fields(), optional_worker_fields_keep_unknown_distinct_from_reported_zero(), PHASES, WorkerRow

### Community 137 - "Monitor"
Cohesion: 0.22
Nodes (7): aggregate_read_started_before_reload_cannot_publish_to_old_destination(), bridge_health_monitor_requests_every_ten_seconds_and_keeps_snapshot_cadence(), Monitor, nic_and_analytics_settings_preserve_each_others_sampler_state(), reset_health(), Settings, worker_util_monitor_reloads_its_destination_and_stops_on_drop()

### Community 138 - "Legacy configuration preflight (prefligh"
Cohesion: 0.22
Nodes (14): DHCP overrides through independent identity, DhcpOverrideBinding, Transient source-policy digest, Dynamic subscriber bounds, legacy_bounds::import, LegacyRoundedBase rule, Current-source exclusion filters, Shared exclusion matcher (sources.rs) (+6 more)

### Community 139 - "Internal report: native policy acceptanc"
Cohesion: 0.21
Nodes (12): Native recovery path review, Broker execution 'healthy' is not convergence proof, bridge --check-boot-inputs (input readiness only), LibreQoS lqosd bridge_broker mod.rs / maintenance.rs (9ebe29bc), maintenance_control.json one-shot apply-now override, MAINTENANCE_WINDOW_HOLD (full-world bulk/reload deferred), Public beta19 non-suffix R3/re-adopt recovery gate (pending_non_suffix), Rollback boundary (consumed one-shot request cannot be undone) (+4 more)

### Community 140 - "LQoSync3 TUI Cosmetics Implementation Pl"
Cohesion: 0.14
Nodes (14): File Map and Shared Interfaces, Global Constraints, Implementation Record, LQoSync3 TUI Cosmetics Implementation Plan, Review Focus, Spec Coverage and Plan Review, Task 1: Theme Configuration and Shared Palette, Task 2: Shell, Key Chips, and Minimum Terminal Size (+6 more)

### Community 141 - "2026-10-31-lqosync3-tui-design.md"
Cohesion: 0.26
Nodes (12): lqosync3 Telegram Notifier Spec (Plan 2b), Notifier Events (changes, error, error cleared, router state, removal guard), status.json (cycle Status), HTTPS sendMessage Delivery (webpki-roots TLS), lqosync3 Telegram Notifier, Telegram Token/Chat ID Secret Rules, lqosync3 tui (ratatui/crossterm), OSC 52 Journal Copy (+4 more)

### Community 142 - "TUI Cosmetics Design"
Cohesion: 0.16
Nodes (11): TUI Cosmetics Design, Cosmetics Acceptance Gates, ShapedDevices Flag Badges, Header Pills, Journal Metadata Spans, Footer Key Chips, Minimum size, Network Tree Guides (+3 more)

### Community 143 - "Command"
Cohesion: 0.15
Nodes (14): Args, Command, Check, Defaults, Detect, Diff, Once, Run (+6 more)

### Community 144 - "RouterAccess"
Cohesion: 0.38
Nodes (11): RouterAccess, RouterMemo, acc(), classify(), first_unreachable_sets_since(), login_tls_trap_timeout_are_failed_and_keep_memo(), memo(), offline_after_zero_is_offline_at_once() (+3 more)

### Community 145 - "after_change"
Cohesion: 0.16
Nodes (8): line_diff(), a_broken_config_file_shows_the_error(), an_override_without_a_lease_shows_no_active_lease(), counts_refresh_after_draft_actions(), discovered_servers_and_leases_are_listed(), dview(), no_snapshot_shows_only_the_toml_servers(), removed_children_count_in_surviving_owner()

### Community 146 - "tests/system_health.rs"
Cohesion: 0.33
Nodes (12): bounded_health_snapshots_reuse_unchanged_cache_writes(), cached_reports_expire_without_a_new_file_write(), disabled_source_is_not_a_failure_but_missing_required_domains_are_unknown(), epoch_changes_do_not_reuse_the_last_success(), failures_keep_component_identity_and_override_pending_presentation(), future_observations_and_unbounded_fields_are_not_healthy(), healthy(), incomplete_or_duplicate_cache_components_cannot_claim_healthy() (+4 more)

### Community 147 - "DPDK statistics delivery plan (6 checkpo"
Cohesion: 0.17
Nodes (13): DPDK review and deployment plan, Release artifact deploy: checksum, backup, atomic install, rollback, DPDK statistics delivery plan (6 checkpoints), Bridge health request (10 s, 15 s expiry), 1 MiB public cache budget with basic-row fallback, On-demand circuit queue diagnostics, Subscriber host observations sampler (512 rows, slot/IP join), Mapped/Unmapped traffic classification (+5 more)

### Community 148 - "Topology Settings and Recovery Plan"
Cohesion: 0.17
Nodes (12): Durable Legacy Ownership and Source Priorities Plan, Strict Export Ownership (schema 4), Source Speed Priority Conversion, Explicit Node Minimums Plan, NodeMinimum (per-direction), Topology Settings and Recovery Plan, authorize_configured, Registry Schema 3 Migration (+4 more)

### Community 149 - "LQoSync3 TUI cosmetics design"
Cohesion: 0.15
Nodes (13): 10. Implementation handoff, 11. Acceptance and verification, 1. Goal and limits, 2. Current implementation, 3. Theme contract — A1 and A5, 4. Header, tabs, footer, and minimum size — A2–A4, 5. ShapedDevices — B6, 6. Network — B7 (+5 more)

### Community 150 - "Unified System Health and Telegram Repor"
Cohesion: 0.24
Nodes (10): Unified System Health and Telegram Reports, Prometheus Alertmanager, Allocation Mismatch (64 vs 1000 slots), Source Freshness Limits, Google SRE: Monitoring Distributed Systems, Notification Policy, Notifier::message (src/notify.rs), system-health.json (+2 more)

### Community 151 - "Unified health and managed-write reports"
Cohesion: 0.23
Nodes (8): Acceptance requirements R1-R9 (write/import/native separation, privacy), Unified health and managed-write reports, Native acceptance verifier (generation correlation, two stable samples), Durable write ID with SHA256 input hashes, Forty-minute verification deadline, Production performance observation 2026-11-12, Seven shared health domains, Material migration differences (identity, fresh observations, pair atomicity)

### Community 152 - "Plan 4b: TUI Settings editor"
Cohesion: 0.29
Nodes (12): Settings tab DHCP view (servers, leases, clients), snapshot.json lease source, Plan 4b: TUI Settings editor, apply (digest check, backup, secrets, write, SIGHUP), runner::cycle_inputs, Draft edit model (toml_edit DocumentMut), KeyMeta / Kind key metadata (meta.rs), Preview (cycle::assemble twice on snapshot.json) (+4 more)

### Community 153 - "TUI btop++ Refinement"
Cohesion: 0.26
Nodes (11): TUI btop++ Refinement, Filled Braille Area Graphs, btop++ Box Panels, btop++ (upstream screenshot reference), export_dashboard_frames Test, Single Process Line Chart, RGB Dark/Light Palette (first refinement), systemctl IO/IP Accounting (+3 more)

### Community 154 - "duration"
Cohesion: 0.24
Nodes (9): BridgeStats, FLOW, ITER, lines(), parse(), parse_takes_the_last_value_of_each_counter(), read(), read_on_this_host() (+1 more)

### Community 155 - "normalize_configured"
Cohesion: 0.35
Nodes (10): all(), config(), current_ipv4_filter_membership_matches_the_legacy_collector(), excluded_sessions_still_require_valid_account_and_endpoint_evidence(), exclusions_cannot_hide_duplicate_current_ips(), filters_cannot_extend_expired_source_evidence(), merged_ip_filters_keep_only_current_unexcluded_sessions(), one_excluded_ppp_session_cannot_leave_its_comments_or_service_membership() (+2 more)

### Community 156 - "worker.rs"
Cohesion: 0.32
Nodes (5): Done, Request, circuit_queue_worker_drop_cancels_an_inflight_socket_without_waiting(), circuit_queue_worker_returns_config_errors_without_blocking_the_caller(), Worker

### Community 157 - "src/model.rs"
Cohesion: 0.21
Nodes (10): GuardHold, Source, Dhcp, Hotspot, Ppp, Unknown, State, StoredCircuit (+2 more)

### Community 158 - "Tab"
Cohesion: 0.17
Nodes (10): Tab, .ALL, Dashboard, Devices, Journal, LibreQoS, Network, Routers (+2 more)

### Community 159 - "SAction"
Cohesion: 0.17
Nodes (11): dsnap(), PreviewDone, PreviewJob, run_preview(), run_preview_allows_a_fix_of_an_invalid_current_file(), run_preview_shows_one_circuit_line_for_an_override(), run_preview_skips_output_free_changes(), SAction (+3 more)

### Community 160 - "LibreQoS v3 upgrade investigation"
Cohesion: 0.18
Nodes (11): Checker migration effects, DPDK migration preparation, Evidence, LibreQoS v3 upgrade investigation, Local facts, Matched package simulation, Package simulation and cleanup, Remaining upgrade checks (+3 more)

### Community 161 - "Review Focus"
Cohesion: 0.18
Nodes (11): Global Constraints, lqosync3 TUI Implementation Plan (Plan 4b: the Settings editor), Review Focus, Review rulings (fact-check of 2026-11-01), Task 1: Key metadata, Task 2: The draft (edit model), Task 3: Validate and secrets, Task 4: Preview (+3 more)

### Community 162 - "Runner::enroll_topology"
Cohesion: 0.24
Nodes (10): Dynamic Subscriber Bounds Plan, legacy_bounds::import, RateRule::LegacyRoundedBase { factor, floor_bps }, Controlled Topology Enrollment Plan, Runner::enroll_topology, staging::pending, Legacy Policy Preflight Plan, ownership::adopt_configured_legacy (+2 more)

### Community 163 - "topology_catalog.rs"
Cohesion: 0.31
Nodes (8): an_orphaning_or_stale_edit_cannot_replace_the_valid_document(), compilation_artifacts(), document(), fraction(), fresh_authorized_return_restores_pins_after_restart_and_reaches_the_capacity_pipeline(), public_scoped_action_keeps_pins_and_persists_through_the_revision_checked_store(), rate(), reopen_move_and_discovery_keep_ids_pins_and_explicit_target_sets()

### Community 164 - "Issue"
Cohesion: 0.18
Nodes (11): Issue, ConflictingIp, DisabledAccount, DisabledServer, DuplicateName, InvalidField, MissingAccount, MissingField (+3 more)

### Community 165 - "RouterForm"
Cohesion: 0.20
Nodes (4): Line, valid_router_name(), SettingsUi, RouterForm

### Community 166 - "verify"
Cohesion: 0.24
Nodes (10): sent(), Verify, verify(), Done, Loaded, verify_loaded_then_done_after_a_higher_cycle_seq(), verify_needs_the_new_digest(), NotVerified (+2 more)

### Community 167 - "staging::stage / recover / cancel"
Cohesion: 0.29
Nodes (10): Refresh the project graph and feature status, Topology Manager feature status, Graphify refresh at checkpoint 79ba5a2, Runner enrollment and automatic recovery, Runner::enroll_topology, Runner::prepare_topology, Controlled migration staging and recovery, Exact-byte no-replacement backup publisher (+2 more)

### Community 168 - "native_full_keeps_node_policy.rs"
Cohesion: 0.31
Nodes (5): check(), find(), full2_keeps_native_node_kind_and_rate(), full_keeps_native_node_kind_and_rate(), INPUT

### Community 169 - "worker-utilization atomic cache"
Cohesion: 0.27
Nodes (10): Buffer and ring pressure plan, worker_util/pressure.rs pressure evaluation, rx_nombuf / pool free <=10% / ring >75% thresholds, Worker utilization plan, Bounded CBOR descriptor IPC to DPDK analytics socket, Producer-timestamp freshness (6 s expiry), worker-utilization atomic cache, Workers tab 6 (busy, idle, backlog, sample age) (+2 more)

### Community 170 - "lqosync3 DHCP client speed overrides and"
Cohesion: 0.20
Nodes (10): 1. Intent, 2. Goals, non-goals, anti-goals, 3. Approaches, 4. Config, 5. Cycle (`sources.rs`), 6. TUI (Settings tab), 7. Output (LibreQoS 2.2 and 3.x), 8. Tests (+2 more)

### Community 171 - "transaction.rs"
Cohesion: 0.38
Nodes (6): changed_rate_generation_and_classification_cannot_match(), duplicate_operation_identities_and_unknown_fields_fail_closed(), generated(), independent(), matches(), observed()

### Community 172 - "btop TUI, review fixes, and a new LibreQ"
Cohesion: 0.22
Nodes (9): btop TUI, btop TUI, review fixes, and a new LibreQoS installation, Changes, Checks, Decisions of the operator, First full LibreQoS run, Goal, Limits and next action (+1 more)

### Community 173 - "TUI cosmetics design handoff"
Cohesion: 0.22
Nodes (9): Capacity semantics, Checks, Cosmetics design specification, Evidence before takeover, Goal, Implementation plan and review, Next action, Result (+1 more)

### Community 174 - "Plan 2b: lqosync3 Telegram implementatio"
Cohesion: 0.22
Nodes (7): Plan 2b: lqosync3 Telegram implementation, Task 1: Config keys, Task 2: Events and message, Task 3: Transport (post_json, send, token, chat_id), chat_id(), token(), token_env_wins_and_empty_env_uses_file()

### Community 175 - "TUI Cosmetics plan (A1-A5, B6-B10)"
Cohesion: 0.42
Nodes (9): TUI Cosmetics plan (A1-A5, B6-B10), Dashboard six cards and fill_ratio gauges, Devices distinct circuit counts and zebra rows, Settings group_change_counts by KeyPath scope, Palette / Role shared theme module, Network tree guides and rate_fill capacity bars, Shell pills, key chips, 80x24 minimum size, Journal styled_line metadata dimming (+1 more)

### Community 176 - "2026-11-01-lqosync3-dhcp-clients-design."
Cohesion: 0.39
Nodes (6): ADR-0009 DHCP Discovery and Exclusions, MAC Normalization (AA:BB:CC:DD:EE:FF), per_site / per_plan Grouping, Lease Rate Source Order, Settings Tab DHCP View, routers.dhcp.shape_unassigned

### Community 177 - "NetworkMode"
Cohesion: 0.22
Nodes (9): Network, NetworkMode, FlatGlobalRoot, FlatPromotion, FlatRouterRoot, GroupedRouterRoot, NoParent, RootPromotion (+1 more)

### Community 178 - "LeaseResult"
Cohesion: 0.22
Nodes (9): LeaseResult, Shaped, Skipped, RateSource, LeaseComment, Override, ServerComment, Settings (+1 more)

### Community 179 - "Sent"
Cohesion: 0.25
Nodes (8): action_result_comes_from_status(), Sent, Wait, Done, Pending, Restarted, wait_result(), TimedOut

### Community 180 - "settings/validate.rs"
Cohesion: 0.42
Nodes (7): Env, env(), parse(), validate(), validate_accepts_a_typed_password_file(), validate_names_the_key(), validate_returns_warnings()

### Community 181 - "Settings editor and functional requireme"
Cohesion: 0.39
Nodes (7): Add Router workflow (draft, validate & preview, save & apply), Edit Values and Save & apply gates, Settings editor and functional requirements, Settings mockup generation record, Final Add Router header correction, Edit Values and Review Changes mockup, Initial Add Router mockup (v0, clipped theme label)

### Community 182 - "Plan 4c: DHCP Client Overrides"
Cohesion: 0.36
Nodes (8): Plan 4c: DHCP Client Overrides, Collected.unit / GroupTotal.unit (server rate), dhcp_leases shared lease list, DhcpClient override (routers.dhcp.clients), LeaseResult / LeaseView / RateSource, normalize_mac, router_leases pub wrapper for TUI, shape_unassigned key

### Community 183 - "Review Focus"
Cohesion: 0.25
Nodes (8): Global Constraints, lqosync3 DHCP Client Overrides Implementation Plan (Plan 4c), Review Focus, Review rulings (fact-check of 2026-11-01), Task 1: Config, Task 2: The shared lease list and the cycle, Task 3: The DHCP view of the Settings tab, Task 4: Docs, review, graph

### Community 184 - "Unified health and Telegram plan"
Cohesion: 0.36
Nodes (8): Unified health and Telegram plan, ExpectedImport / import evidence, system_health seven-domain model, Persistent async Telegram outbox worker, Durable write receipts (WRITTEN/PENDING/APPLIED), Explicit slot mapping plan, Post-reboot native ACK investigation plan, Native policy-proof verifier investigation

### Community 186 - "queue.rs"
Cohesion: 0.46
Nodes (4): partial_totals_resets_and_large_counters_never_fabricate_events(), Queue, stale_measurements_keep_last_totals_but_hide_current_increments(), Totals

### Community 187 - "TUI visual review and transition assessm"
Cohesion: 0.29
Nodes (5): Bounded DPDK traffic history (1,801 samples, 2s poll), TUI visual review and transition assessment, Separate browser product out of scope, Deployment limits (monitor deploy, privileged Settings check pending), 63-frame export matrix (7 routes x 3 themes x 3 sizes)

### Community 188 - "PppSource"
Cohesion: 0.29
Nodes (7): PppSource, ActiveComment, Default, ProfileComment, ProfileName, ProfileRateLimit, SecretComment

### Community 189 - "Host storage cleanup"
Cohesion: 0.33
Nodes (5): Build cache cleanup, Checks and retained files, Host storage cleanup, Removed files, Retained investigation evidence

### Community 192 - "Combined mirrored traffic chart"
Cohesion: 0.40
Nodes (6): Combined mirrored traffic chart, Independent directional dynamic ceiling, Mirrored traffic chart (download above, upload below zero), Prepared curve cache and single-canvas renderer, Combined mirrored traffic chart extension, Combined mirrored traffic

### Community 193 - "Explicit native slot mapping"
Cohesion: 0.47
Nodes (5): Explicit native slot mapping, applied_slot_matches probe diagnostic, Mapping change invalidates sample and resets rate baselines, BTreeMap slot ID -> circuit name mapping, slot_numbers field (nested-CBOR slot IDs) in native applied-state reports

### Community 194 - "Post-reboot native ACK diagnosis"
Cohesion: 0.47
Nodes (5): Post-reboot native ACK diagnosis, boot_restored_partial_world_cannot_accept_a_newer_import regression, import_confirmed_ack_unproven state, native generation still pending verifier result, Generated 28 vs native 27 leaf/mapping operations

### Community 195 - "Topology Placement Resolver Plan"
Cohesion: 0.53
Nodes (4): Topology Placement Resolver Plan, ObservedPolicy, ProfileObservation {Unread, Known}, resolver.rs (topology_manager)

### Community 196 - "TUI btop++ refinement"
Cohesion: 0.33
Nodes (6): Direction, Implementation and proof, Reason, Second correction: btop++ Default theme, Third change: one process chart, TUI btop++ refinement

### Community 197 - "Extra"
Cohesion: 0.33
Nodes (5): Extra, Client, extra_item(), Lease, Note

### Community 198 - "LibreQoS v3 live cutover"
Cohesion: 0.40
Nodes (5): LibreQoS v3 live cutover, mlx5 compatibility override, Netplan editor conflict, Source ownership, v3 live cutover

### Community 199 - "2026-11-02 — v3 import request, native p"
Cohesion: 0.40
Nodes (5): 2026-11-02 — v3 import request, native probe, bridge monitor, DPDK bridge monitor, Native readiness probe, No subscriber grace, Pair import request and verification

### Community 200 - "LibreQoS v3 upgrade preparation"
Cohesion: 0.40
Nodes (5): LibreQoS v3 upgrade preparation, Historical upgrade preparation, Rollback preparation, Successful shadow comparison, Timed dataplane recovery

### Community 201 - "Approved TUI screen implementation"
Cohesion: 0.40
Nodes (5): Bounded rendering and history, Approved TUI screen implementation, Evidence-aware UI, Settings apply gates, Seven screen implementation

### Community 210 - "IPv6 status of the production network"
Cohesion: 0.40
Nodes (4): IPv6 status of the production network, FSE-CORE WAN router, FSE-Distro router, mikrotikFindIPv6.py

### Community 211 - "Web-style shell: sidebar, top bar, seven"
Cohesion: 0.50
Nodes (5): Approved TUI screens plan, docs/design/tui/README.md canonical PNGs, Seven approved screen bodies (Dashboard..Journal), TUI web-style shell plan, Web-style shell: sidebar, top bar, seven routes

### Community 212 - "LogLevel"
Cohesion: 0.40
Nodes (5): LogLevel, Debug, Error, Info, Warn

### Community 213 - "journal_app"
Cohesion: 0.40
Nodes (4): a_too_large_copy_is_not_sent(), journal_app(), s_toggles_select_mode(), y_on_journal_copies_with_osc52()

### Community 214 - "VModal"
Cohesion: 0.50
Nodes (4): VModal, ConfirmApply, RemoveRouter, RemoveServer

### Community 215 - "2026-10-31 — First live shadow run"
Cohesion: 0.50
Nodes (4): Apply cutover, 2026-10-31 — First live shadow run, First live shadow run, Telegram shadow notices

### Community 216 - "Native runtime policy validation"
Cohesion: 0.50
Nodes (4): Canonical reconstruction loss, Native runtime policy validation, Native runtime policy defect, Validation boundary

### Community 217 - "Native topology source validation"
Cohesion: 0.50
Nodes (4): Native topology source validation, Native node policy discrepancy, Native offline source check, Production source guard

### Community 218 - "2026-11-03 — LibreQoS v3 package update "
Cohesion: 0.50
Nodes (4): 2026-11-03 — LibreQoS v3 package update 3.0.7.202611021200, mlx5 override preservation, Native defect still blocked, v3 package consolidation

### Community 219 - "LibreQoS beta19 integration review"
Cohesion: 0.50
Nodes (4): beta19 integration review, LibreQoS beta19 integration review, Native guard retained, Telemetry interface distinction

### Community 220 - "Cycle collection and subscriber state"
Cohesion: 0.50
Nodes (4): Collection deployment verified, Cycle collection state, Cycle collection and subscriber state, Subscriber evidence semantics

### Community 221 - "Approved TUI mockups"
Cohesion: 0.50
Nodes (4): Approved visual baseline, Complete mouse interaction, Approved TUI mockups, Functional Settings acceptance

### Community 222 - "TUI release review"
Cohesion: 0.50
Nodes (4): TUI release review, TUI deployment verified, TUI performance review, Wall-clock history correction

### Community 236 - "LQoSync3"
Cohesion: 0.50
Nodes (4): LQoSync3, Output target selection, RouterOS collection, Shadow safety

### Community 237 - "Stage"
Cohesion: 0.50
Nodes (4): Stage, Edit, Result, Verifying

### Community 238 - "Journal proposal and TUI transition revi"
Cohesion: 0.67
Nodes (3): Journal proposal and TUI transition review, Journal proposal, TUI transition feasibility

### Community 239 - "Optional node minimums"
Cohesion: 0.67
Nodes (3): Minimum deployment boundary, Optional node minimums, Preserve-mode minimums

### Community 241 - "report"
Cohesion: 0.67
Nodes (3): malformed_acknowledgements_fail_closed(), report(), schema_six_transaction_is_observable_without_claiming_acceptance()

## Ambiguous Edges - Review These
- `2026-10-31-lqosync3-v3-design.md` → `Unified System Health and Telegram Reports`  [AMBIGUOUS]
  docs/superpowers/specs/2026-10-31-lqosync3-v3-design.md · relation: references
- `Scenario tests (replace golden parity tests)` → `Stable node id (lq3- FNV-1a 64)`  [AMBIGUOUS]
  brain/20-decisions/ADR-0007-fresh-design-not-a-port.md · relation: conceptually_related_to
- `ADR-0005 Topology follows v3` → `Topology foundation`  [AMBIGUOUS]
  brain/10-sessions/2026-11-16-topology-foundation.md · relation: references
- `Router state machine (online/failed/unreachable/offline)` → `Fresh-only subscriber observations rule`  [AMBIGUOUS]
  docs/ARCHITECTURE.md · relation: conceptually_related_to

## Knowledge Gaps
- **509 isolated node(s):** `Empty`, `Silent`, `cli::parse / cli::Command`, `cycle::assemble (pure core)`, `files::backup (keep_backups)` (+504 more)
  These have ≤1 connection - possible missing edges or undocumented components. (Counts symbols only; 1255 node(s) total have ≤1 connection when file, concept and rationale nodes are included.)
- **42 thin communities (<3 nodes) omitted from report** — run `graphify query` to explore isolated nodes.

## Suggested Questions
_Questions this graph is uniquely positioned to answer:_

- **Why does `LQoSync3 TUI visual contract` connect `LQoSync3 TUI visual contract` to `Combined mirrored traffic chart`, `LibreQoS v3 ingress`, `Installation`, `tui/mod.rs`, `app.rs`, `Configuration`, `Settings editor and functional requireme`, `Operations and DPDK Telemetry`, `TUI visual review and transition assessm`, `Mockup generation record (TUI PROMPTS)`?**
  _High betweenness centrality (0.109) - this node is a cross-community bridge._
- **Are the 16 inferred relationships involving `Config` (e.g. with `diff_config()` and `configured()`) actually correct?**
  _`Config` has 16 INFERRED edges - model-reasoned connections that need verification._
- **What connects `Empty`, `Silent`, `cli::parse / cli::Command` to the rest of the system?**
  _509 weakly-connected nodes found - possible documentation gaps or missing edges._
- **Should `Runner Service Loop` be split into smaller, more focused modules?**
  _Cohesion score 0.058719906048150326 - nodes in this community are weakly interconnected._
- **Why does `Topology Manager implementation design` connect `Topology Manager implementation design` to `Source Evidence and IDs Plan`, `Runner::enroll_topology`, `Topology Placement Resolver Plan`, `PreparedPair (prepare(AuthorizedReconcil`, `PreparedPair preparation (publication.rs`, `Legacy configuration preflight (prefligh`, `Topology Manager initial TUI layout`, `Topology Manager discussion`, `Topology Settings and Recovery Plan`, `Topology catalog and hierarchy persisten`?**
  _High betweenness centrality (0.051) - this node is a cross-community bridge._

### Low-confidence Hints
_AMBIGUOUS edges — the extractor was unsure. Verify before acting on these._

- **What is the exact relationship between `2026-10-31-lqosync3-v3-design.md` and `Unified System Health and Telegram Reports`?**
  _Edge tagged AMBIGUOUS (relation: references) - confidence is low._
- **What is the exact relationship between `Scenario tests (replace golden parity tests)` and `Stable node id (lq3- FNV-1a 64)`?**
  _Edge tagged AMBIGUOUS (relation: conceptually_related_to) - confidence is low._