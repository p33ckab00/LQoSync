# Graph Report - LQoSync3.0  (2026-11-01)

## Corpus Check
- 85 files · ~107,583 words
- Verdict: corpus is large enough that graph structure adds value.
- Unclassified: 4 file(s) not represented in the graph (top: (none) 2, .toml 1, .service 1)

## Summary
- 1850 nodes · 4757 edges · 80 communities (69 shown, 11 thin omitted)
- Extraction: 96% EXTRACTED · 4% INFERRED · 0% AMBIGUOUS · INFERRED: 176 edges (avg confidence: 0.87)
- Token cost: 0 input · 0 output

## Graph Freshness
- Built from commit: `1a5c6cd0`
- Run `git rev-parse HEAD` and compare to check if the graph is stale.
- Run `graphify update .` after code changes (no API cost).

## Community Hubs (Navigation)
- mock_router.rs
- Telegram Notifier
- Router State
- Topology
- sources.rs
- view.rs
- devices.rs
- cycle.rs
- File Safety
- SettingsView
- Config
- hashmap
- draft.rs
- .on_key
- State
- meta.rs
- Configuration reference
- apply.rs
- runner::Runner::cycle (one full cycle)
- preview.rs
- MikroTik Session 5
- lqosync3 core design spec
- scenarios.rs
- RouterCfg
- Review Focus
- CertificateDer
- Handoff
- run
- Config
- runner::Runner::cycle (one full cycle)
- dhcp_server_gives_override_or_
- routeros.rs
- lqosync3 LibreQoS v3 targets (Plan 3)
- Review Focus
- RouterResult
- lqosync3 core design spec
- data.rs
- journal.rs
- diff.rs
- speed.rs
- dashboard.rs
- app.rs
- State
- v3network.rs
- check.sh
- Telegram Notifier 4
- CLI and Checks 4
- lqosync3
- tui/mod.rs
- lqosync3 terminal UI (Plan 4)
- LibreQoS v3 ingress document
- Operations guide
- Review Focus
- .draw
- mikrotik.rs
- Action
- ADR-0009 DHCP discovery and exclusions
- osc52.rs
- cli.rs
- state.rs
- RouterAccess
- status.rs
- detect.rs
- ADR-0008 Terminal UI
- ReadError
- PppSource
- Tab
- LogLevel
- .after_change
- CsvRow
- Input
- Review Focus
- NetworkMode
- LeaseResult
- VModal
- Stage

## God Nodes (most connected - your core abstractions)
1. `Config` - 88 edges
2. `SettingsView` - 71 edges
3. `CsvRow` - 48 edges
4. `run()` - 32 edges
5. `Draft` - 32 edges
6. `key()` - 32 edges
7. `lqosync3 core design spec` - 32 edges
8. `App` - 31 edges
9. `Configuration reference` - 31 edges
10. `Handoff` - 28 edges

## Surprising Connections (you probably didn't know these)
- `v3 offline simulation method` --semantically_similar_to--> `Offline v3 simulation`  [INFERRED] [semantically similar]
  brain/30-knowledge/v3-offline-simulation-method.md → docs/LIBREQOS-V3-INGRESS.md
- `v3 offline simulation method` --semantically_similar_to--> `/run/lqos/bus 0777 safety caution`  [INFERRED] [semantically similar]
  brain/30-knowledge/v3-offline-simulation-method.md → docs/LIBREQOS-V3-INGRESS.md
- `Task 2: Events and message` --references--> `split()`  [EXTRACTED]
  docs/superpowers/plans/2026-10-31-lqosync3-telegram.md → src/notify.rs
- `Safety rules (shadow default, no router writes, 0600 secrets)` --semantically_similar_to--> `Operational rules`  [INFERRED] [semantically similar]
  README.md → docs/OPERATIONS.md
- `Session 2026-10-31 project setup and v3 simulation` --references--> `libreqos-v3-preflight`  [INFERRED]
  brain/10-sessions/2026-10-31-project-setup-and-v3-simulation.md → docs/LIBREQOS-V3-INGRESS.md

## Import Cycles
- 2-file cycle: `src/tui/app.rs -> src/tui/devices.rs -> src/tui/app.rs`
- 2-file cycle: `src/tui/app.rs -> src/tui/dashboard.rs -> src/tui/app.rs`
- 2-file cycle: `src/tui/app.rs -> src/tui/network.rs -> src/tui/app.rs`
- 2-file cycle: `src/config.rs -> src/speed.rs -> src/config.rs`
- 3-file cycle: `src/tui/app.rs -> src/tui/settings/view.rs -> src/tui/dashboard.rs -> src/tui/app.rs`
- 4-file cycle: `src/tui/app.rs -> src/tui/settings/view.rs -> src/tui/settings/apply.rs -> src/tui/dashboard.rs -> src/tui/app.rs`

## Hyperedges (group relationships)
- **Router-to-LibreQoS cycle data flow** — docs_architecture_collector, docs_architecture_circuit_model, docs_architecture_validator, docs_architecture_target_detector, docs_architecture_writers, docs_architecture_applier [EXTRACTED 1.00]
- **LibreQoS output targets selected by auto** — readme_v2_pair, readme_v3_pair, readme_v3_network, readme_auto_target_selection, docs_operations_libreqos_upgrade_2_to_3 [EXTRACTED 1.00]
- **Plan 1 core tasks building assemble()** — docs_superpowers_plans_2026_10_31_lqosync3_core_task1_config, docs_superpowers_plans_2026_10_31_lqosync3_core_task2_speed, docs_superpowers_plans_2026_10_31_lqosync3_core_task3_state, docs_superpowers_plans_2026_10_31_lqosync3_core_task4_sources, docs_superpowers_plans_2026_10_31_lqosync3_core_task5_topology, docs_superpowers_plans_2026_10_31_lqosync3_core_task6_cycle, docs_superpowers_plans_2026_10_31_lqosync3_core_task7_writer, docs_superpowers_plans_2026_10_31_lqosync3_core_task8_scenarios [EXTRACTED 1.00]
- **Removal guard hold-and-release flow** — docs_configuration_removal_guard, docs_superpowers_specs_2026_10_31_lqosync3_core_design_cycle_assembly_rules, docs_superpowers_specs_2026_10_31_lqosync3_core_design_guard_fingerprint, docs_superpowers_specs_2026_10_31_lqosync3_core_design_state_file [EXTRACTED 1.00]
- **Router failure vs offline handling** — brain_20_decisions_adr_0003_router_failed_vs_offline_router_state_model, brain_20_decisions_adr_0003_router_failed_vs_offline_offline_after, brain_20_decisions_adr_0003_router_failed_vs_offline_offline_action, brain_30_knowledge_updatecsv_py_dissection_defects_to_fix [EXTRACTED 1.00]
- **Crash-safe file writes** — docs_superpowers_specs_2026_10_31_lqosync3_runtime_design_instance_lock, docs_superpowers_specs_2026_10_31_lqosync3_runtime_design_write_order_state_first, docs_superpowers_specs_2026_10_31_lqosync3_runtime_design_atomic_write, docs_superpowers_specs_2026_10_31_lqosync3_runtime_design_first_run_seed [INFERRED 0.85]
- **Shadow-to-apply cutover flow** — docs_configuration_runtime_modes, docs_operations_command_reference, docs_operations_cutover_procedure, docs_operations_rollback_procedure, brain_10_sessions_2026_10_31_first_live_shadow_run [INFERRED 0.85]
- **Plan 2b Telegram notifier design-to-ops chain** — docs_superpowers_specs_2026_10_31_lqosync3_telegram_design, docs_superpowers_plans_2026_10_31_lqosync3_telegram, docs_configuration_telegram_settings, docs_operations_turn_on_telegram, brain_handoff_plan2b_deferred_items [INFERRED 0.85]
- **Router failed/unreachable/offline state model** — brain_handoff_adr_0003_router_failed_vs_offline, docs_configuration_router_state, docs_superpowers_plans_2026_10_31_lqosync3_runtime_classify, docs_superpowers_plans_2026_10_31_lqosync3_runtime_read_router, docs_superpowers_plans_2026_10_31_lqosync3_runtime_read_error [INFERRED 0.85]
- **Safe cycle write path (lock, atomic write, backup, reload)** — docs_superpowers_plans_2026_10_31_lqosync3_runtime_runner_cycle, docs_superpowers_plans_2026_10_31_lqosync3_runtime_files_lock, docs_superpowers_plans_2026_10_31_lqosync3_runtime_write_if_changed, docs_superpowers_plans_2026_10_31_lqosync3_runtime_files_backup, docs_superpowers_plans_2026_10_31_lqosync3_runtime_run_reload [INFERRED 0.85]
- **LibreQoS v3 node rule compliance** — brain_20_decisions_adr_0005_topology_follows_v3_stable_node_id, brain_20_decisions_adr_0005_topology_follows_v3_parent_node_id, brain_20_decisions_adr_0005_topology_follows_v3_csv_column_count, brain_20_decisions_adr_0005_topology_follows_v3_max_floor_mbps, brain_20_decisions_adr_0005_topology_follows_v3_flat_promotion_parent_fix, brain_20_decisions_adr_0006_v3_guards_and_node_types_no_parent_guard, brain_20_decisions_adr_0006_v3_guards_and_node_types_site_group_nodes, brain_20_decisions_adr_0006_v3_guards_and_node_types_compile_mode_warning [INFERRED 0.85]
- **DHCP discovery and subscriber exclusion** — brain_20_decisions_adr_0009_dhcp_discovery_and_exclusions_dhcp_server_discovery, brain_20_decisions_adr_0009_dhcp_discovery_and_exclusions_exclusions, docs_configuration_exclusions, docs_superpowers_specs_2026_10_31_lqosync3_core_design_circuit_rules [INFERRED 0.95]

## Communities (80 total, 11 thin omitted)

### Community 0 - "mock_router.rs"
Cohesion: 0.14
Nodes (25): read_router(), Answer, Empty, Raw, Rows, Silent, Trap, closed_port_is_unreachable() (+17 more)

### Community 1 - "Telegram Notifier"
Cohesion: 0.11
Nodes (34): Task 3: Transport (post_json, send, token, chat_id), Telegram, a_config_error_is_an_error_event(), a_long_multi_line_error_splits_into_valid_html_parts(), a_very_long_error_is_cut(), an_empty_token_is_an_error(), an_error_is_sent_once_then_cleared(), body_complete() (+26 more)

### Community 2 - "Router State"
Cohesion: 0.09
Nodes (53): target_name(), Runtime, RuntimeMode, Apply, Shadow, Connector, RouterSession, a_config_reload_error_shows_in_status_until_a_good_reload() (+45 more)

### Community 3 - "Topology"
Cohesion: 0.10
Nodes (48): NodeTemplate, StoredNode, GroupKey, DhcpPlan, DhcpSite, Hotspot, Ppp, PppPlan (+40 more)

### Community 4 - "sources.rs"
Cohesion: 0.16
Nodes (34): a_zero_speed_profile_comment_gives_one_line(), an_override_changes_only_that_client(), an_override_for_a_missing_mac_has_no_effect(), an_override_in_per_plan_moves_to_its_plan_node(), an_override_on_a_skipped_lease_gives_no_circuit(), by_name(), cfg(), cfg_text() (+26 more)

### Community 5 - "view.rs"
Cohesion: 0.09
Nodes (27): a_broken_config_file_shows_the_error(), a_rejected_apply_says_the_new_file_is_on_disk(), a_skipped_lease_cannot_get_an_override(), an_edit_of_a_discovered_server_makes_its_entry(), an_override_without_a_lease_shows_no_active_lease(), DFILE, discovered_servers_and_leases_are_listed(), Disk (+19 more)

### Community 6 - "devices.rs"
Cohesion: 0.06
Nodes (44): Data, FLAG_DUP_IP, FLAG_NO_PARENT, FLAG_RATE, Versions, Column, .ALL, Down (+36 more)

### Community 7 - "cycle.rs"
Cohesion: 0.13
Nodes (28): a_row_that_stays_keeps_its_group_node(), AAA, an_override_gives_the_same_rate_in_v2_pair_v3_pair_and_v3_network(), dhcp_cfg(), first_forty(), flat_promotion_has_no_client_node_for_a_static_row(), guard_cfg(), guard_holds_a_mass_removal_and_keeps_other_changes() (+20 more)

### Community 8 - "File Safety"
Cohesion: 0.12
Nodes (13): backup(), backup_keeps_the_last_n(), backup_keeps_the_new_folder_and_other_folders(), HDR, Live, load_state_seeds_on_first_run_and_reads_after(), lock(), read_live() (+5 more)

### Community 9 - "SettingsView"
Cohesion: 0.11
Nodes (4): path_text(), SettingsView, show(), Shown

### Community 10 - "Config"
Cohesion: 0.07
Nodes (35): a_bad_or_duplicate_mac_is_an_error(), a_circuit_comment_must_not_be_the_static_comment(), a_client_rate_below_the_floor_is_an_error(), BackstopCap, BackstopCaps, ClientTemplate, deep_merge(), defaults_table() (+27 more)

### Community 11 - "hashmap"
Cohesion: 0.31
Nodes (13): check(), collect_nodes(), duplicate_circuit_id_stops(), min_above_max_or_bad_number_stops(), missing_parent_stops_except_in_no_parent(), multi_device_circuit_passes_but_shared_device_id_stops(), net(), parent_node_id_must_match_the_parent_node() (+5 more)

### Community 12 - "draft.rs"
Cohesion: 0.08
Nodes (32): a_bad_router_name_is_rejected(), a_change_keeps_comments_and_key_order(), add_and_remove_a_dhcp_server(), add_and_remove_a_router(), changed_counts_the_keys(), diff_lines_show_old_and_new(), Draft, FILE (+24 more)

### Community 13 - ".on_key"
Cohesion: 0.22
Nodes (31): a_bad_number_shows_the_key_error(), a_changed_file_shows_the_disk_diff_and_can_restart(), a_failed_preview_blocks_apply(), a_failed_preview_shows_that_apply_is_blocked(), a_secret_for_a_removed_router_is_dropped(), a_typed_password_is_never_in_the_buffer(), add_a_router_asks_for_a_masked_password(), apply_asks_for_a_confirmation() (+23 more)

### Community 14 - "State"
Cohesion: 0.18
Nodes (11): Entry, GuardHold, Source, Dhcp, Hotspot, Ppp, Unknown, State (+3 more)

### Community 15 - "meta.rs"
Cohesion: 0.07
Nodes (41): DEFAULTS_TOML, all(), Base, Float, List, Text, build(), CHOICES (+33 more)

### Community 16 - "Configuration reference"
Cohesion: 0.07
Nodes (31): config/defaults.toml embedded defaults, Plan 2b deferred minor items, RouterOS listen change detection (future), Configuration reference, Comment speed format (Name|25M/5M), Map from config.json, DHCP server discovery, Exclusions ([exclude], [routers.exclude]) (+23 more)

### Community 17 - "apply.rs"
Cohesion: 0.08
Nodes (36): config_digest(), config_digest_is_stable_and_16_hex(), Sent, a_changed_file_on_disk_stops_the_apply(), a_toml_write_error_rolls_back_the_secrets(), apply(), apply_writes_a_backup_and_keeps_n(), ApplyError (+28 more)

### Community 18 - "runner::Runner::cycle (one full cycle)"
Cohesion: 0.13
Nodes (17): Applier (hash, atomic write, reload), Circuit Model, Collector (RouterOS API reads), Config Loader, Proposed Crate Layout, libreqos_network.json, lqosd (LibreQoS daemon), MikroTik RouterOS API (+9 more)

### Community 19 - "preview.rs"
Cohesion: 0.16
Nodes (31): cycle_inputs(), Inputs, a_failed_preview_is_failed_not_zero(), a_new_router_effect_is_not_known(), a_rate_change_gives_the_expected_circuit_changes(), a_source_turned_on_is_not_known(), ALICE, an_old_snapshot_gives_the_stale_warning() (+23 more)

### Community 20 - "MikroTik Session 5"
Cohesion: 0.16
Nodes (14): lqosync3 runtime design (Plan 2), Atomic write (temp file, sync_all, rename, fsync folder; only on byte change), Files: lock, state.json, status.json, candidate, applied, First-run seed (seed_state, seed_nodes from live files), Instance lock <state>/lqosync3.lock (File::try_lock), mikrotik-tokio 0.1.0 16-reply burst cancel bug, Plan 2 scope (in / not in), Process: run loop, SIGUSR1, SIGHUP, SIGTERM/SIGINT, once, check, diff, defaults (+6 more)

### Community 21 - "lqosync3 core design spec"
Cohesion: 0.12
Nodes (42): 2026-10-31 Plan 1: lqosync3 core library session, lqosync3 core crate (config, speed, model, state, sources, topology, cycle, validate, writer), Plan 1 review fixes (commit 529fa36), Session 2026-10-31 project setup and v3 simulation, Session: updatecsv.py dissection, ADR-0003 Router failed vs offline, ADR-0004 TOML policy, Python defaults, ADR-0005 Topology follows v3 (+34 more)

### Community 22 - "scenarios.rs"
Cohesion: 0.27
Nodes (11): assemble(), CycleInput, CycleOutput, node_rate(), render_network_json(), cycle(), read(), report_json() (+3 more)

### Community 23 - "RouterCfg"
Cohesion: 0.22
Nodes (19): RouterCfg, RosRow, RouterSnapshot, collect_dhcp(), collect_hotspot(), collect_ppp(), collect_router(), Collected (+11 more)

### Community 24 - "Review Focus"
Cohesion: 0.17
Nodes (11): Global Constraints, lqosync3 TUI Implementation Plan (Plan 4b: the Settings editor), Review Focus, Review rulings (fact-check of 2026-11-01), Task 1: Key metadata, Task 2: The draft (edit model), Task 3: Validate and secrets, Task 4: Preview (+3 more)

### Community 26 - "Handoff"
Cohesion: 0.17
Nodes (6): 2026-10-31 First live shadow run, FSEHub-Distro router (10.200.1.2:8728), Shadow run results (62 rows, 167 ms, 26 text-only updates), Production state (Python live, lqosync3 shadow, LibreQoS 2.2), Differences from updatecsv.py, Task 1: Crate scaffold and config

### Community 28 - "run"
Cohesion: 0.20
Nodes (24): cfg(), duplicate_ip_existing_vs_existing_keeps_collected(), duplicate_ip_new_vs_existing_keeps_existing(), empty_online_router_removes_its_rows(), failed_and_unreachable_router_keep_rows_and_nodes(), flat_promotion_parent_is_own_client_node(), get(), grace_also_applies_to_a_router_not_in_the_config() (+16 more)

### Community 29 - "Config"
Cohesion: 0.13
Nodes (11): Config, defaults(), DhcpSource, LeaseComment, ServerComment, Settings, Exclude, normalize_mac() (+3 more)

### Community 30 - "runner::Runner::cycle (one full cycle)"
Cohesion: 0.09
Nodes (23): network.json, Router state (online/failed/unreachable/offline), routerstate::classify, Core design spec (2026-10-31-lqosync3-core-design.md), cycle::assemble (pure core), files::backup (keep_backups), files::lock (lqosync3.lock), files::load_state (state.json or seed) (+15 more)

### Community 31 - "dhcp_server_gives_override_or_"
Cohesion: 0.21
Nodes (8): BandwidthDefaults, DhcpCfg, DhcpClient, DhcpMode, PerPlan, PerSite, DhcpServer, HotspotCfg

### Community 32 - "routeros.rs"
Cohesion: 0.19
Nodes (14): a_dead_kept_session_reconnects_without_a_retry(), a_login_error_is_not_retried(), Fake, gives_last_error_after_all_retries(), ppp_router(), Trap, reads_only_enabled_tables(), retry_reconnects_and_succeeds() (+6 more)

### Community 33 - "lqosync3 LibreQoS v3 targets (Plan 3)"
Cohesion: 0.22
Nodes (9): CLI, Config changes, lqosync3 LibreQoS v3 targets (Plan 3), Offline check result (2026-10-31), Out of scope, Purpose, Target selection (ADR-0002), `v3_network` (+1 more)

### Community 34 - "Review Focus"
Cohesion: 0.25
Nodes (7): Global Constraints, lqosync3 LibreQoS v3 Implementation Plan (Plan 3), Review Focus, Task 1: Detector (`src/detect.rs`), Task 2: `v3_network` writer (`src/writer/v3network.rs`), Task 3: Runner, CLI, and check, Task 4: Offline check with the v3 parser

### Community 35 - "RouterResult"
Cohesion: 0.31
Nodes (9): dhcp_snap(), ppp(), ppp_owned(), RouterResult, Failed, Offline, Online, Unreachable (+1 more)

### Community 36 - "lqosync3 core design spec"
Cohesion: 0.13
Nodes (16): Six Network Modes, Topology Builder, LibreQoS v3 topology.compile_mode, Plan 1 review focus (edge cases), Task 3: Model, state file, and seed, Task 5: Topology (build_nodes, to_network_json), Task 6: Cycle assembly and validation, Cycle assembly rules (+8 more)

### Community 38 - "data.rs"
Cohesion: 0.07
Nodes (41): exit_with(), main(), notify(), now(), print_summary(), run(), SECRET_OWNER_UID, a_bad_network_json_keeps_the_last_data_and_shows_an_error() (+33 more)

### Community 39 - "journal.rs"
Cohesion: 0.09
Nodes (19): buffer_keeps_the_last_n_lines(), journal_start_error_is_shown(), journal_stderr_is_shown(), journal_tab_renders(), JournalHandle, JournalMsg, Error, Line (+11 more)

### Community 40 - "diff.rs"
Cohesion: 0.17
Nodes (14): diff_config(), diff_dirs(), diff_dirs_needs_candidate_files(), diff_ignores_column_order_row_order_and_number_text(), diff_pair(), diff_reports_added_removed_changed(), diff_stops_for_v3_network(), diff_stops_in_apply_mode() (+6 more)

### Community 41 - "speed.rs"
Cohesion: 0.10
Nodes (23): RateLimitOrder, DownloadUpload, RouterosRxTx, UnitlessUnit, Bps, Mbps, plan_label(), circuit_rates() (+15 more)

### Community 42 - "dashboard.rs"
Cohesion: 0.08
Nodes (25): narrow_terminal_puts_the_side_panel_under_the_main_panel(), split_main(), dashboard_tab_renders(), DashboardView, data(), digest_check_shows_not_loaded(), Kind, Reload (+17 more)

### Community 43 - "app.rs"
Cohesion: 0.16
Nodes (23): a_click_on_a_tab_title_selects_it(), a_key_press_clears_the_message_so_errors_show(), a_too_large_copy_is_not_sent(), action_result_comes_from_status(), an_open_field_takes_q_and_digits(), app(), app_dialogs_show_their_keys_and_only_y_n_esc_act(), draw() (+15 more)

### Community 45 - "v3network.rs"
Cohesion: 0.27
Nodes (15): a_bad_ip_value_skips_the_circuit(), a_duplicate_node_id_is_an_error(), a_node_type_that_v3_does_not_know_is_an_error(), a_node_without_id_or_type_gets_a_stable_id_and_site(), a_v2_plan_node_from_the_state_is_written_as_site(), bad_circuits_are_skipped_with_an_alert(), prefix(), render() (+7 more)

### Community 52 - "tui/mod.rs"
Cohesion: 0.09
Nodes (8): fresh_pid(), install_panic_hook(), panic_hook_restores_only_on_its_own_thread(), restore_terminal(), RESTORED, run(), TerminalGuard, unix_now()

### Community 53 - "lqosync3 terminal UI (Plan 4)"
Cohesion: 0.11
Nodes (18): 10. Phases, 11. Tests, 12. Left to the builder, 1. Intent, 2. Goals, non-goals, anti-goals, 3. Constraints, 4. Approaches, 5. Changes to the service (Plan 4a) (+10 more)

### Community 54 - "LibreQoS v3 ingress document"
Cohesion: 0.21
Nodes (13): LibreQoS v3 ingress document, Last-good topology retention, liblqos_python.so binding, libreqos_3.0.7 beta package, libreqos_network.json schema 2, libreqos-v3-preflight, lqosd (v3 control plane), lqosd inotify file watcher (+5 more)

### Community 55 - "Operations guide"
Cohesion: 0.08
Nodes (32): Orphan branch lqosync3, RouterOS listen command (untested), 14/15 CSV columns (v2_pair/v3_pair), libreqos_network.json schema_version 2, Plan phases (Plan 1/2/2b done; Plan 3 v3, Plan 4 TUI, Plan 5 installer), Runtime modes (shadow, apply), install.sh one-line installer (target contract), Installation (docs) (+24 more)

### Community 56 - "Review Focus"
Cohesion: 0.15
Nodes (12): Global Constraints, lqosync3 TUI Implementation Plan (Plan 4a: service changes and read-only screens), Review Focus, Review rulings (fact-check of 2026-10-31), Task 1: Service data for the TUI, Task 2: TUI frame, Task 3: Loader worker, Task 4: ShapedDevices tab (+4 more)

### Community 57 - ".draw"
Cohesion: 0.20
Nodes (6): center(), dialog(), Modal, ConfirmSync, Discard, modal_keys()

### Community 58 - "mikrotik.rs"
Cohesion: 0.15
Nodes (5): map_err(), MikrotikConnector, MikrotikSession, tls_client_config(), Setup

### Community 59 - "Action"
Cohesion: 0.18
Nodes (10): Action, ApplySettings, Copy, None, Preview, Quit, Reload, SelectMode (+2 more)

### Community 60 - "ADR-0009 DHCP discovery and exclusions"
Cohesion: 0.10
Nodes (16): ADR-0009 DHCP discovery and exclusions, Speed Rules, Task 2: Rate format and speed rules (SpeedParser), Task 4: Circuit collection (collect_router), Circuit rules for each source, DhcpCfg::server(name) defaults, 1. Intent, 2. Goals, non-goals, anti-goals (+8 more)

### Community 62 - "cli.rs"
Cohesion: 0.06
Nodes (49): Args, check(), check_finds_plain_api_and_no_source_router(), check_reads_the_telegram_token_when_enabled(), check_reports_a_bad_secret_and_a_missing_lqos_dir(), check_shows_the_target_or_the_detection_error(), check_warns_when_the_file_is_not_loaded(), check_with() (+41 more)

### Community 63 - "state.rs"
Cohesion: 0.19
Nodes (14): CircuitNames, load_state(), HDR, read_csv(), read_csv_accepts_bom_crlf_and_missing_columns(), read_csv_skips_blank_names_and_keeps_repeated_names(), seed_infers_source_and_keeps_ids(), seed_nodes() (+6 more)

### Community 64 - "RouterAccess"
Cohesion: 0.50
Nodes (11): RouterAccess, RouterMemo, acc(), classify(), first_unreachable_sets_since(), login_tls_trap_timeout_are_failed_and_keep_memo(), memo(), offline_after_zero_is_offline_at_once() (+3 more)

### Community 65 - "status.rs"
Cohesion: 0.19
Nodes (10): Change, changed_columns(), Report, circuit_line(), updated_line(), CircuitInfo, ReportSummary, status_json_has_the_spec_fields() (+2 more)

### Community 66 - "detect.rs"
Cohesion: 0.18
Nodes (18): Libreqos, Target, Auto, V2Pair, V3Network, V3Pair, Detected, Env (+10 more)

### Community 67 - "ADR-0008 Terminal UI"
Cohesion: 0.16
Nodes (11): offline_action (default remove), offline_after (default 300 s), Speed source order TOML lists, ADR-0008 Terminal UI, TUI config editor (toml_edit), SIGUSR1 sync now / SIGHUP reload, status.json state file, updatecsv.py defects to fix (+3 more)

### Community 68 - "ReadError"
Cohesion: 0.15
Nodes (8): FakeSession, read_snapshot(), ReadError, Login, Protocol, Timeout, Tls, Unreachable

### Community 69 - "PppSource"
Cohesion: 0.29
Nodes (7): PppSource, ActiveComment, Default, ProfileComment, ProfileName, ProfileRateLimit, SecretComment

### Community 70 - "Tab"
Cohesion: 0.29
Nodes (7): Tab, .ALL, Dashboard, Devices, Journal, Network, Settings

### Community 71 - "LogLevel"
Cohesion: 0.40
Nodes (5): LogLevel, Debug, Error, Info, Warn

### Community 72 - ".after_change"
Cohesion: 0.18
Nodes (8): KeyPath, concrete(), Extra, Client, Lease, Note, Group, key_path()

### Community 73 - "CsvRow"
Cohesion: 0.24
Nodes (8): Columns, V2, V3, CsvRow, csv_header_sort_and_quoting(), field(), render_csv(), row()

### Community 74 - "Input"
Cohesion: 0.20
Nodes (9): Input, AddRouter, AddServer, Client, Dropdown, Field, List, None (+1 more)

### Community 75 - "Review Focus"
Cohesion: 0.22
Nodes (8): Global Constraints, lqosync3 DHCP Client Overrides Implementation Plan (Plan 4c), Review Focus, Review rulings (fact-check of 2026-11-01), Task 1: Config, Task 2: The shared lease list and the cycle, Task 3: The DHCP view of the Settings tab, Task 4: Docs, review, graph

### Community 76 - "NetworkMode"
Cohesion: 0.22
Nodes (9): Network, NetworkMode, FlatGlobalRoot, FlatPromotion, FlatRouterRoot, GroupedRouterRoot, NoParent, RootPromotion (+1 more)

### Community 77 - "LeaseResult"
Cohesion: 0.22
Nodes (9): LeaseResult, Shaped, Skipped, RateSource, LeaseComment, Override, ServerComment, Settings (+1 more)

### Community 78 - "VModal"
Cohesion: 0.50
Nodes (4): VModal, ConfirmApply, RemoveRouter, RemoveServer

### Community 79 - "Stage"
Cohesion: 0.50
Nodes (4): Stage, Edit, Result, Verifying

## Ambiguous Edges - Review These
- `Stable node id (lq3- FNV-1a 64)` → `Scenario tests (replace golden parity tests)`  [AMBIGUOUS]
  brain/20-decisions/ADR-0007-fresh-design-not-a-port.md · relation: conceptually_related_to
- `RouterOS listen change detection (future)` → `Collector (RouterOS API reads)`  [AMBIGUOUS]
  docs/ARCHITECTURE.md · relation: conceptually_related_to

## Knowledge Gaps
- **169 isolated node(s):** `lqosync3`, `Check`, `Once`, `Run`, `Diff` (+164 more)
  These have ≤1 connection - possible missing edges or undocumented components. (Counts symbols only; 440 node(s) total have ≤1 connection when file, concept and rationale nodes are included.)
- **11 thin communities (<3 nodes) omitted from report** — run `graphify query` to explore isolated nodes.

## Suggested Questions
_Questions this graph is uniquely positioned to answer:_

- **Why does `Task 3: Transport (post_json, send, token, chat_id)` connect `Telegram Notifier` to `Configuration reference`?**
  _High betweenness centrality (0.176) - this node is a cross-community bridge._
- **Are the 2 inferred relationships involving `Config` (e.g. with `diff_config()` and `fresh_pid()`) actually correct?**
  _`Config` has 2 INFERRED edges - model-reasoned connections that need verification._
- **What connects `lqosync3`, `Check`, `Once` to the rest of the system?**
  _169 weakly-connected nodes found - possible documentation gaps or missing edges._
- **Should `mock_router.rs` be split into smaller, more focused modules?**
  _Cohesion score 0.13949579831932774 - nodes in this community are weakly interconnected._
- **Why does `Config` connect `Config` to `mock_router.rs`, `Telegram Notifier`, `Router State`, `Topology`, `sources.rs`, `cycle.rs`, `Config`, `draft.rs`, `meta.rs`, `preview.rs`, `scenarios.rs`, `RouterCfg`, `run`, `dhcp_server_gives_override_or_`, `routeros.rs`, `data.rs`, `diff.rs`, `tui/mod.rs`, `cli.rs`, `state.rs`, `RouterAccess`, `detect.rs`, `NetworkMode`?**
  _High betweenness centrality (0.109) - this node is a cross-community bridge._

### Low-confidence Hints
_AMBIGUOUS edges — the extractor was unsure. Verify before acting on these._

- **What is the exact relationship between `Stable node id (lq3- FNV-1a 64)` and `Scenario tests (replace golden parity tests)`?**
  _Edge tagged AMBIGUOUS (relation: conceptually_related_to) - confidence is low._
- **What is the exact relationship between `RouterOS listen change detection (future)` and `Collector (RouterOS API reads)`?**
  _Edge tagged AMBIGUOUS (relation: conceptually_related_to) - confidence is low._