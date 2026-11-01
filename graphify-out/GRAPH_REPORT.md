# Graph Report - LQoSync3.0  (2026-11-01)

## Corpus Check
- 91 files · ~126,984 words
- Verdict: corpus is large enough that graph structure adds value.
- Unclassified: 4 file(s) not represented in the graph (top: (none) 2, .toml 1, .service 1)

## Summary
- 2032 nodes · 5226 edges · 79 communities (69 shown, 10 thin omitted)
- Extraction: 96% EXTRACTED · 3% INFERRED · 0% AMBIGUOUS · INFERRED: 182 edges (avg confidence: 0.87)
- Token cost: 0 input · 0 output

## Graph Freshness
- Built from commit: `e1ea30a0`
- Run `git rev-parse HEAD` and compare to check if the graph is stale.
- Run `graphify update .` after code changes (no API cost).

## Community Hubs (Navigation)
- mock_router.rs
- Telegram Notifier
- runner.rs
- Topology
- sources.rs
- view.rs
- devices.rs
- cycle.rs
- files.rs
- SettingsView
- Config
- LQoSync3 TUI Cosmetics Implementation Plan
- draft.rs
- .on_key
- Palette
- meta.rs
- Configuration reference
- apply.rs
- runner::Runner::cycle (one full cycle)
- preview.rs
- MikroTik Session 5
- lqosync3 core design spec
- .connect
- RouterCfg
- Review Focus
- theme.rs
- Configuration reference
- Theme
- Config
- runner::Runner::cycle (one full cycle)
- dhcp_server_gives_override_or_
- lqosync3 LibreQoS v3 targets (Plan 3)
- Review Focus
- main.rs
- lqosync3 core design spec
- data.rs
- journal.rs
- a_rejected_apply_says_the_new_file_is_on_disk
- src/validate.rs
- dashboard.rs
- app.rs
- State
- check.sh
- Telegram Notifier 4
- CLI and Checks 4
- lqosync3
- tui/mod.rs
- lqosync3 terminal UI (Plan 4)
- LibreQoS v3 ingress document
- Operations guide
- Review Focus
- Status
- ReadError
- ADR-0009 DHCP discovery and exclusions
- TUI cosmetics design handoff
- cli.rs
- scenarios.rs
- RouterAccess
- TUI cosmetics implementation
- detect.rs
- ADR-0008 Terminal UI
- PppSource
- Role
- State
- TUI btop++ refinement
- .draw
- Input
- Review Focus
- LeaseResult
- BoxKind
- Stage
- ADR-0009 DHCP discovery and exclusions
- RouterResult
- LogLevel
- SAction

## God Nodes (most connected - your core abstractions)
1. `Config` - 89 edges
2. `SettingsView` - 71 edges
3. `CsvRow` - 48 edges
4. `Palette` - 40 edges
5. `key()` - 36 edges
6. `App` - 33 edges
7. `run()` - 32 edges
8. `Draft` - 32 edges
9. `lqosync3 core design spec` - 32 edges
10. `runner()` - 31 edges

## Surprising Connections (you probably didn't know these)
- `v3 offline simulation method` --semantically_similar_to--> `/run/lqos/bus 0777 safety caution`  [INFERRED] [semantically similar]
  brain/30-knowledge/v3-offline-simulation-method.md → docs/LIBREQOS-V3-INGRESS.md
- `v3 offline simulation method` --semantically_similar_to--> `Offline v3 simulation`  [INFERRED] [semantically similar]
  brain/30-knowledge/v3-offline-simulation-method.md → docs/LIBREQOS-V3-INGRESS.md
- `Task 2: Events and message` --references--> `split()`  [EXTRACTED]
  docs/superpowers/plans/2026-10-31-lqosync3-telegram.md → src/notify.rs
- `Session 2026-10-31 project setup and v3 simulation` --references--> `libreqos-v3-preflight`  [INFERRED]
  brain/10-sessions/2026-10-31-project-setup-and-v3-simulation.md → docs/LIBREQOS-V3-INGRESS.md
- `ADR-0002 v3 target selection` --references--> `topology_import.json (internal)`  [INFERRED]
  brain/20-decisions/ADR-0002-v3-target-selection.md → docs/LIBREQOS-V3-INGRESS.md

## Import Cycles
- 2-file cycle: `src/tui/app.rs -> src/tui/devices.rs -> src/tui/app.rs`
- 2-file cycle: `src/tui/app.rs -> src/tui/network.rs -> src/tui/app.rs`
- 2-file cycle: `src/config.rs -> src/speed.rs -> src/config.rs`

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

## Communities (79 total, 10 thin omitted)

### Community 0 - "mock_router.rs"
Cohesion: 0.17
Nodes (25): read_router(), Answer, Empty, Raw, Rows, Silent, Trap, closed_port_is_unreachable() (+17 more)

### Community 1 - "Telegram Notifier"
Cohesion: 0.06
Nodes (40): Task 3: Transport (post_json, send, token, chat_id), Telegram, AnyCertificate, a_config_error_is_an_error_event(), a_long_multi_line_error_splits_into_valid_html_parts(), a_very_long_error_is_cut(), an_empty_token_is_an_error(), an_error_is_sent_once_then_cleared() (+32 more)

### Community 2 - "runner.rs"
Cohesion: 0.25
Nodes (32): a_config_reload_error_shows_in_status_until_a_good_reload(), a_fresh_libreqos_directory_gets_both_files(), a_held_lock_stops_the_cycle_without_touching_status(), a_live_shaper_without_its_csv_gets_no_full_run(), a_panicking_router_task_fails_only_that_router(), a_shipped_csv_without_a_full_run_still_gets_a_full_run(), apply_backs_up_writes_and_reloads(), assemble_error_writes_nothing() (+24 more)

### Community 3 - "Topology"
Cohesion: 0.06
Nodes (61): NodeTemplate, UnitlessUnit, Bps, Mbps, StoredNode, GroupKey, Hotspot, Ppp (+53 more)

### Community 4 - "sources.rs"
Cohesion: 0.16
Nodes (34): a_zero_speed_profile_comment_gives_one_line(), an_override_changes_only_that_client(), an_override_for_a_missing_mac_has_no_effect(), an_override_in_per_plan_moves_to_its_plan_node(), an_override_on_a_skipped_lease_gives_no_circuit(), by_name(), cfg(), cfg_text() (+26 more)

### Community 5 - "view.rs"
Cohesion: 0.10
Nodes (26): Preview, Ran, Skipped, a_broken_config_file_shows_the_error(), a_skipped_lease_cannot_get_an_override(), an_edit_of_a_discovered_server_makes_its_entry(), an_override_without_a_lease_shows_no_active_lease(), DFILE (+18 more)

### Community 6 - "devices.rs"
Cohesion: 0.06
Nodes (52): narrow_terminal_puts_the_side_panel_under_the_main_panel(), split_main(), Data, circuits_count_distinct_ids(), Column, .ALL, Down, Ipv4 (+44 more)

### Community 7 - "cycle.rs"
Cohesion: 0.11
Nodes (50): a_row_that_stays_keeps_its_group_node(), AAA, an_override_gives_the_same_rate_in_v2_pair_v3_pair_and_v3_network(), cfg(), dhcp_cfg(), duplicate_ip_existing_vs_existing_keeps_collected(), duplicate_ip_new_vs_existing_keeps_existing(), empty_online_router_removes_its_rows() (+42 more)

### Community 8 - "files.rs"
Cohesion: 0.09
Nodes (27): CircuitNames, backup(), backup_keeps_the_last_n(), backup_keeps_the_new_folder_and_other_folders(), finish_secret(), HDR, Live, load_state() (+19 more)

### Community 9 - "SettingsView"
Cohesion: 0.09
Nodes (8): path_text(), concrete(), SettingsView, Shown, VModal, ConfirmApply, RemoveRouter, RemoveServer

### Community 10 - "Config"
Cohesion: 0.07
Nodes (33): a_bad_or_duplicate_mac_is_an_error(), a_circuit_comment_must_not_be_the_static_comment(), a_client_rate_below_the_floor_is_an_error(), BackstopCap, BackstopCaps, ClientTemplate, defaults_table(), dhcp_client_mac_forms_load() (+25 more)

### Community 11 - "LQoSync3 TUI Cosmetics Implementation Plan"
Cohesion: 0.06
Nodes (30): File Map and Shared Interfaces, Global Constraints, Implementation Record, LQoSync3 TUI Cosmetics Implementation Plan, Review Focus, Spec Coverage and Plan Review, Task 1: Theme Configuration and Shared Palette, Task 2: Shell, Key Chips, and Minimum Terminal Size (+22 more)

### Community 12 - "draft.rs"
Cohesion: 0.08
Nodes (33): a_bad_router_name_is_rejected(), a_change_keeps_comments_and_key_order(), add_and_remove_a_dhcp_server(), add_and_remove_a_router(), changed_counts_the_keys(), diff_lines_show_old_and_new(), Draft, FILE (+25 more)

### Community 13 - ".on_key"
Cohesion: 0.19
Nodes (33): a_bad_number_shows_the_key_error(), a_changed_file_shows_the_disk_diff_and_can_restart(), a_failed_preview_blocks_apply(), a_failed_preview_shows_that_apply_is_blocked(), a_secret_for_a_removed_router_is_dropped(), a_typed_password_is_never_in_the_buffer(), add_a_router_asks_for_a_masked_password(), apply_asks_for_a_confirmation() (+25 more)

### Community 15 - "meta.rs"
Cohesion: 0.06
Nodes (42): DEFAULTS_TOML, all(), Base, Float, List, Text, build(), CHOICES (+34 more)

### Community 16 - "Configuration reference"
Cohesion: 0.09
Nodes (25): Plan 2b deferred minor items, Plan phases (Plan 1/2/2b done; Plan 3 v3, Plan 4 TUI, Plan 5 installer), Telegram settings, Operations guide, Applied backups (keep_backups, 12-digit folders), Command reference (check, once, diff, run, defaults), Cutover from the Python service, lqosync3 detect (Plan 3) (+17 more)

### Community 17 - "apply.rs"
Cohesion: 0.07
Nodes (39): config_digest(), config_digest_is_stable_and_16_hex(), a_changed_file_on_disk_stops_the_apply(), a_toml_write_error_rolls_back_the_secrets(), apply(), apply_writes_a_backup_and_keeps_n(), ApplyError, Changed (+31 more)

### Community 18 - "runner::Runner::cycle (one full cycle)"
Cohesion: 0.13
Nodes (18): Applier (hash, atomic write, reload), Circuit Model, Collector (RouterOS API reads), Config Loader, Proposed Crate Layout, libreqos_network.json, lqosd (LibreQoS daemon), MikroTik RouterOS API (+10 more)

### Community 19 - "preview.rs"
Cohesion: 0.17
Nodes (29): cycle_inputs(), Inputs, a_failed_preview_is_failed_not_zero(), a_new_router_effect_is_not_known(), a_rate_change_gives_the_expected_circuit_changes(), a_router_that_was_not_online_gives_a_warning(), a_source_turned_on_is_not_known(), ALICE (+21 more)

### Community 20 - "MikroTik Session 5"
Cohesion: 0.16
Nodes (14): lqosync3 runtime design (Plan 2), Atomic write (temp file, sync_all, rename, fsync folder; only on byte change), Files: lock, state.json, status.json, candidate, applied, First-run seed (seed_state, seed_nodes from live files), Instance lock <state>/lqosync3.lock (File::try_lock), mikrotik-tokio 0.1.0 16-reply burst cancel bug, Plan 2 scope (in / not in), Process: run loop, SIGUSR1, SIGHUP, SIGTERM/SIGINT, once, check, diff, defaults (+6 more)

### Community 21 - "lqosync3 core design spec"
Cohesion: 0.19
Nodes (19): Session 2026-10-31 project setup and v3 simulation, ADR-0005 Topology follows v3, max_floor_mbps (0.2) / safe_min_mbps (0.1), Parent Node ID column, ADR-0006 v3 guards and node types, compile_mode not full warning, LibreQoS v3 ingress facts, Tool must reject duplicate IP and node name (+11 more)

### Community 22 - ".connect"
Cohesion: 0.38
Nodes (3): Fake, FakeConnector, FakeSession

### Community 23 - "RouterCfg"
Cohesion: 0.13
Nodes (27): normalize_mac(), parse_ipv4_net(), PppoeCfg, RouterCfg, RouterSnapshot, read_snapshot(), collect_dhcp(), collect_hotspot() (+19 more)

### Community 24 - "Review Focus"
Cohesion: 0.17
Nodes (11): Global Constraints, lqosync3 TUI Implementation Plan (Plan 4b: the Settings editor), Review Focus, Review rulings (fact-check of 2026-11-01), Task 1: Key metadata, Task 2: The draft (edit model), Task 3: Validate and secrets, Task 4: Preview (+3 more)

### Community 25 - "theme.rs"
Cohesion: 0.12
Nodes (16): a_box_title_has_the_btop_brackets_and_hotkey(), a_panel_has_the_box_colour_and_marks_focus_in_the_title(), area_graph_fills_from_the_bottom(), btop_gradient_ends(), Grad, Cpu, Free, Mem (+8 more)

### Community 26 - "Configuration reference"
Cohesion: 0.06
Nodes (28): 2026-10-31 First live shadow run, FSEHub-Distro router (10.200.1.2:8728), Shadow run results (62 rows, 167 ms, 26 text-only updates), config/defaults.toml embedded defaults, Production state (Python live, lqosync3 shadow, LibreQoS 2.2), RouterOS listen change detection (future), Configuration reference, Comment speed format (Name|25M/5M) (+20 more)

### Community 28 - "Theme"
Cohesion: 0.40
Nodes (5): Theme, Dark, Light, Mono, Tui

### Community 29 - "Config"
Cohesion: 0.20
Nodes (5): Config, deep_merge(), defaults(), Exclude, Safety

### Community 30 - "runner::Runner::cycle (one full cycle)"
Cohesion: 0.08
Nodes (27): network.json, Router state (online/failed/unreachable/offline), lqosync3.service systemd unit, Signals (SIGHUP reload, USR1 cycle now), routerstate::classify, Core design spec (2026-10-31-lqosync3-core-design.md), cycle::assemble (pure core), files::backup (keep_backups) (+19 more)

### Community 31 - "dhcp_server_gives_override_or_"
Cohesion: 0.21
Nodes (8): BandwidthDefaults, DhcpCfg, DhcpClient, DhcpMode, PerPlan, PerSite, DhcpServer, HotspotCfg

### Community 33 - "lqosync3 LibreQoS v3 targets (Plan 3)"
Cohesion: 0.22
Nodes (9): CLI, Config changes, lqosync3 LibreQoS v3 targets (Plan 3), Offline check result (2026-10-31), Out of scope, Purpose, Target selection (ADR-0002), `v3_network` (+1 more)

### Community 34 - "Review Focus"
Cohesion: 0.25
Nodes (7): Global Constraints, lqosync3 LibreQoS v3 Implementation Plan (Plan 3), Review Focus, Task 1: Detector (`src/detect.rs`), Task 2: `v3_network` writer (`src/writer/v3network.rs`), Task 3: Runner, CLI, and check, Task 4: Offline check with the v3 parser

### Community 35 - "main.rs"
Cohesion: 0.13
Nodes (12): Runtime, RuntimeMode, Apply, Shadow, exit_with(), main(), notify(), now() (+4 more)

### Community 36 - "lqosync3 core design spec"
Cohesion: 0.12
Nodes (28): 2026-10-31 Plan 1: lqosync3 core library session, lqosync3 core crate (config, speed, model, state, sources, topology, cycle, validate, writer), Plan 1 review fixes (commit 529fa36), Session: updatecsv.py dissection, Architecture (LQoSync3), Six Network Modes, Topology Builder, LibreQoS v3 topology.compile_mode (+20 more)

### Community 38 - "data.rs"
Cohesion: 0.06
Nodes (53): a_bad_network_json_keeps_the_last_data_and_shows_an_error(), a_missing_csv_is_an_error(), collect_names(), Env, env(), FLAG_DUP_IP, FLAG_NO_PARENT, FLAG_RATE (+45 more)

### Community 39 - "journal.rs"
Cohesion: 0.10
Nodes (21): buffer_keeps_the_last_n_lines(), journal_start_error_is_shown(), journal_stderr_is_shown(), journal_tab_renders(), JournalHandle, JournalMsg, Error, Line (+13 more)

### Community 40 - "a_rejected_apply_says_the_new_file_is_on_disk"
Cohesion: 0.27
Nodes (4): a_rejected_apply_says_the_new_file_is_on_disk(), Disk, disk(), the_header_says_loaded_while_the_first_cycle_runs()

### Community 41 - "src/validate.rs"
Cohesion: 0.16
Nodes (22): Network, NetworkMode, FlatGlobalRoot, FlatPromotion, FlatRouterRoot, GroupedRouterRoot, NoParent, RootPromotion (+14 more)

### Community 42 - "dashboard.rs"
Cohesion: 0.09
Nodes (23): CPU_SCALE, data(), digest_check_shows_not_loaded(), export_dashboard_frames(), Kind, Reload, SyncNow, lines() (+15 more)

### Community 43 - "app.rs"
Cohesion: 0.06
Nodes (57): a_click_on_a_tab_title_selects_it(), a_key_press_clears_the_message_so_errors_show(), a_long_hint_wraps_without_a_lost_key(), a_too_large_copy_is_not_sent(), a_wide_header_is_one_line(), Action, ApplySettings, Copy (+49 more)

### Community 52 - "tui/mod.rs"
Cohesion: 0.09
Nodes (9): action_timeout(), fresh_pid(), install_panic_hook(), panic_hook_restores_only_on_its_own_thread(), restore_terminal(), RESTORED, run(), TerminalGuard (+1 more)

### Community 53 - "lqosync3 terminal UI (Plan 4)"
Cohesion: 0.11
Nodes (18): 10. Phases, 11. Tests, 12. Left to the builder, 1. Intent, 2. Goals, non-goals, anti-goals, 3. Constraints, 4. Approaches, 5. Changes to the service (Plan 4a) (+10 more)

### Community 54 - "LibreQoS v3 ingress document"
Cohesion: 0.21
Nodes (13): LibreQoS v3 ingress document, Last-good topology retention, liblqos_python.so binding, libreqos_3.0.7 beta package, libreqos_network.json schema 2, libreqos-v3-preflight, lqosd (v3 control plane), lqosd inotify file watcher (+5 more)

### Community 55 - "Operations guide"
Cohesion: 0.13
Nodes (12): Orphan branch lqosync3, RouterOS listen command (untested), 14/15 CSV columns (v2_pair/v3_pair), libreqos_network.json schema_version 2, Differences from updatecsv.py, LQoSync3 README, LQoSync3 sync tool, mikrotik-rs RouterOS API library (+4 more)

### Community 56 - "Review Focus"
Cohesion: 0.15
Nodes (12): Global Constraints, lqosync3 TUI Implementation Plan (Plan 4a: service changes and read-only screens), Review Focus, Review rulings (fact-check of 2026-10-31), Task 1: Service data for the TUI, Task 2: TUI frame, Task 3: Loader worker, Task 4: ShapedDevices tab (+4 more)

### Community 57 - "Status"
Cohesion: 0.11
Nodes (15): target_name(), random_id(), random_ids_have_the_length_and_charset(), Runner, Runner<C>, state_name(), write_snapshot(), RouterStatus (+7 more)

### Community 58 - "ReadError"
Cohesion: 0.07
Nodes (30): map_err(), MikrotikConnector, MikrotikSession, tls_client_config(), RosRow, a_dead_kept_session_reconnects_without_a_retry(), a_login_error_is_not_retried(), Connector (+22 more)

### Community 60 - "ADR-0009 DHCP discovery and exclusions"
Cohesion: 0.20
Nodes (10): 1. Intent, 2. Goals, non-goals, anti-goals, 3. Approaches, 4. Config, 5. Cycle (`sources.rs`), 6. TUI (Settings tab), 7. Output (LibreQoS 2.2 and 3.x), 8. Tests (+2 more)

### Community 61 - "TUI cosmetics design handoff"
Cohesion: 0.29
Nodes (7): Checks, Evidence before takeover, Goal, Implementation plan and review, Next action, Result, TUI cosmetics design handoff

### Community 62 - "cli.rs"
Cohesion: 0.05
Nodes (51): Args, check(), check_finds_plain_api_and_no_source_router(), check_reads_the_telegram_token_when_enabled(), check_reports_a_bad_secret_and_a_missing_lqos_dir(), check_shows_the_target_or_the_detection_error(), check_warns_when_the_file_is_not_loaded(), check_warns_when_the_libreqos_state_directory_is_missing() (+43 more)

### Community 63 - "scenarios.rs"
Cohesion: 0.12
Nodes (23): assemble(), CycleInput, CycleOutput, names(), node_rate(), Columns, V2, V3 (+15 more)

### Community 64 - "RouterAccess"
Cohesion: 0.50
Nodes (11): RouterAccess, RouterMemo, acc(), classify(), first_unreachable_sets_since(), login_tls_trap_timeout_are_failed_and_keep_memo(), memo(), offline_after_zero_is_offline_at_once() (+3 more)

### Community 65 - "TUI cosmetics implementation"
Cohesion: 0.29
Nodes (7): Changes, Checks, Goal, Limits and next action, Review and fixes, Revision, TUI cosmetics implementation

### Community 66 - "detect.rs"
Cohesion: 0.09
Nodes (33): Libreqos, Target, Auto, V2Pair, V3Network, V3Pair, Detected, Env (+25 more)

### Community 67 - "ADR-0008 Terminal UI"
Cohesion: 0.13
Nodes (20): ADR-0003 Router failed vs offline, offline_action (default remove), offline_after (default 300 s), ADR-0004 TOML policy, Python defaults, Speed source order TOML lists, ADR-0007 Fresh design, not a port, Scenario tests (replace golden parity tests), Semantic compare with live files before cutover (+12 more)

### Community 69 - "PppSource"
Cohesion: 0.13
Nodes (15): DhcpSource, LeaseComment, ServerComment, Settings, PppSource, ActiveComment, Default, ProfileComment (+7 more)

### Community 70 - "Role"
Cohesion: 0.29
Nodes (7): Role, Accent, Base, Error, Ok, Secondary, Warning

### Community 71 - "State"
Cohesion: 0.10
Nodes (21): Change, changed_columns(), Entry, Report, GuardHold, Source, Dhcp, Hotspot (+13 more)

### Community 72 - "TUI btop++ refinement"
Cohesion: 0.33
Nodes (5): Direction, Implementation and proof, Reason, Second correction: btop++ Default theme, TUI btop++ refinement

### Community 73 - ".draw"
Cohesion: 0.22
Nodes (12): a_very_tall_area_does_not_overflow(), card_areas(), dashboard_card_geometry(), dashboard_tab_renders(), DashboardView, draw_history(), draw_routers(), fill_ratio() (+4 more)

### Community 74 - "Input"
Cohesion: 0.11
Nodes (19): KeyPath, counts_refresh_after_draft_actions(), Extra, Client, Lease, Note, Group, group_change_counts() (+11 more)

### Community 75 - "Review Focus"
Cohesion: 0.22
Nodes (8): Global Constraints, lqosync3 DHCP Client Overrides Implementation Plan (Plan 4c), Review Focus, Review rulings (fact-check of 2026-11-01), Task 1: Config, Task 2: The shared lease list and the cycle, Task 3: The DHCP view of the Settings tab, Task 4: Docs, review, graph

### Community 77 - "LeaseResult"
Cohesion: 0.20
Nodes (10): LeaseResult, Shaped, Skipped, LeaseView, RateSource, LeaseComment, Override, ServerComment (+2 more)

### Community 78 - "BoxKind"
Cohesion: 0.40
Nodes (5): BoxKind, Cpu, Mem, Net, Proc

### Community 79 - "Stage"
Cohesion: 0.50
Nodes (4): Stage, Edit, Result, Verifying

### Community 80 - "ADR-0009 DHCP discovery and exclusions"
Cohesion: 0.23
Nodes (7): ADR-0009 DHCP discovery and exclusions, Speed Rules, Task 2: Rate format and speed rules (SpeedParser), Task 4: Circuit collection (collect_router), Circuit rules for each source, DhcpCfg::server(name) defaults, Output format (round3, fmt_rate, 14/15 CSV columns)

### Community 84 - "RouterResult"
Cohesion: 0.31
Nodes (9): dhcp_snap(), ppp(), ppp_owned(), RouterResult, Failed, Offline, Online, Unreachable (+1 more)

### Community 88 - "LogLevel"
Cohesion: 0.40
Nodes (5): LogLevel, Debug, Error, Info, Warn

### Community 89 - "SAction"
Cohesion: 0.50
Nodes (4): SAction, Apply, None, Preview

## Ambiguous Edges - Review These
- `Collector (RouterOS API reads)` → `RouterOS listen change detection (future)`  [AMBIGUOUS]
  docs/ARCHITECTURE.md · relation: conceptually_related_to
- `Stable node id (lq3- FNV-1a 64)` → `Scenario tests (replace golden parity tests)`  [AMBIGUOUS]
  brain/20-decisions/ADR-0007-fresh-design-not-a-port.md · relation: conceptually_related_to

## Knowledge Gaps
- **187 isolated node(s):** `lqosync3`, `Check`, `Once`, `Run`, `Diff` (+182 more)
  These have ≤1 connection - possible missing edges or undocumented components. (Counts symbols only; 504 node(s) total have ≤1 connection when file, concept and rationale nodes are included.)
- **10 thin communities (<3 nodes) omitted from report** — run `graphify query` to explore isolated nodes.

## Suggested Questions
_Questions this graph is uniquely positioned to answer:_

- **Why does `Task 3: Transport (post_json, send, token, chat_id)` connect `Telegram Notifier` to `Configuration reference`?**
  _High betweenness centrality (0.183) - this node is a cross-community bridge._
- **Are the 3 inferred relationships involving `Config` (e.g. with `diff_config()` and `action_timeout()`) actually correct?**
  _`Config` has 3 INFERRED edges - model-reasoned connections that need verification._
- **What connects `lqosync3`, `Check`, `Once` to the rest of the system?**
  _187 weakly-connected nodes found - possible documentation gaps or missing edges._
- **Should `Telegram Notifier` be split into smaller, more focused modules?**
  _Cohesion score 0.06433566433566433 - nodes in this community are weakly interconnected._
- **Why does `Config` connect `Config` to `mock_router.rs`, `Telegram Notifier`, `runner.rs`, `Topology`, `sources.rs`, `cycle.rs`, `files.rs`, `Config`, `draft.rs`, `meta.rs`, `preview.rs`, `RouterCfg`, `Theme`, `dhcp_server_gives_override_or_`, `main.rs`, `data.rs`, `src/validate.rs`, `tui/mod.rs`, `Status`, `ReadError`, `cli.rs`, `scenarios.rs`, `RouterAccess`, `detect.rs`, `PppSource`?**
  _High betweenness centrality (0.141) - this node is a cross-community bridge._

### Low-confidence Hints
_AMBIGUOUS edges — the extractor was unsure. Verify before acting on these._

- **What is the exact relationship between `Collector (RouterOS API reads)` and `RouterOS listen change detection (future)`?**
  _Edge tagged AMBIGUOUS (relation: conceptually_related_to) - confidence is low._
- **What is the exact relationship between `Stable node id (lq3- FNV-1a 64)` and `Scenario tests (replace golden parity tests)`?**
  _Edge tagged AMBIGUOUS (relation: conceptually_related_to) - confidence is low._