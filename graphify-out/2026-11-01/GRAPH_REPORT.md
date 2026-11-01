# Graph Report - LQoSync3.0  (2026-11-01)

## Corpus Check
- 93 files · ~128,646 words
- Verdict: corpus is large enough that graph structure adds value.
- Unclassified: 4 file(s) not represented in the graph (top: (none) 2, .toml 1, .service 1)

## Summary
- 2055 nodes · 5255 edges · 94 communities (82 shown, 12 thin omitted)
- Extraction: 96% EXTRACTED · 3% INFERRED · 0% AMBIGUOUS · INFERRED: 182 edges (avg confidence: 0.87)
- Token cost: 0 input · 0 output

## Graph Freshness
- Built from commit: `7ad5e5f7`
- Run `git rev-parse HEAD` and compare to check if the graph is stale.
- Run `graphify update .` after code changes (no API cost).

## Community Hubs (Navigation)
- mock_router.rs
- Telegram Notifier
- runner.rs
- Topology
- sources.rs
- view.rs
- network.rs
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
- speed.rs
- RouterCfg
- Review Focus
- theme.rs
- 2026-10-31 First live shadow run
- Theme
- Config
- devices.rs
- dhcp_server_gives_override_or_
- journal.rs
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
- proc.rs
- check.sh
- Telegram Notifier 4
- CLI and Checks 4
- lqosync3
- tui/mod.rs
- lqosync3 terminal UI (Plan 4)
- LibreQoS v3 ingress document
- Operations guide
- Review Focus
- diff.rs
- ReadError
- v3network.rs
- ADR-0009 DHCP discovery and exclusions
- TUI cosmetics design handoff
- cli.rs
- scenarios.rs
- RouterAccess
- TUI cosmetics implementation
- detect.rs
- ADR-0008 Terminal UI
- mikrotik.rs
- PppSource
- Role
- State
- TUI btop++ refinement
- .draw
- Input
- Review Focus
- AnyCertificate
- LeaseResult
- BoxKind
- Stage
- src/secrets.rs
- status.rs
- settings/validate.rs
- Data
- RouterResult
- Level
- Column
- Command
- LogLevel
- super
- btop TUI, review fixes, and a new LibreQoS installation
- ADR-0010 — A full LibreQoS run before the first full run
- VModal

## God Nodes (most connected - your core abstractions)
1. `Config` - 89 edges
2. `SettingsView` - 71 edges
3. `CsvRow` - 48 edges
4. `Palette` - 39 edges
5. `key()` - 36 edges
6. `App` - 33 edges
7. `run()` - 32 edges
8. `Draft` - 32 edges
9. `Handoff` - 32 edges
10. `lqosync3 core design spec` - 32 edges

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

## Communities (94 total, 12 thin omitted)

### Community 0 - "mock_router.rs"
Cohesion: 0.17
Nodes (25): read_router(), Answer, Empty, Raw, Rows, Silent, Trap, closed_port_is_unreachable() (+17 more)

### Community 1 - "Telegram Notifier"
Cohesion: 0.09
Nodes (39): Task 3: Transport (post_json, send, token, chat_id), Telegram, a_config_error_is_an_error_event(), a_long_multi_line_error_splits_into_valid_html_parts(), a_very_long_error_is_cut(), an_empty_token_is_an_error(), an_error_is_sent_once_then_cleared(), an_error_with_other_numbers_is_not_a_new_event() (+31 more)

### Community 2 - "runner.rs"
Cohesion: 0.08
Nodes (59): target_name(), Runtime, RuntimeMode, Apply, Shadow, RosRow, Connector, RouterSession (+51 more)

### Community 3 - "Topology"
Cohesion: 0.10
Nodes (48): NodeTemplate, StoredNode, GroupKey, DhcpPlan, DhcpSite, Hotspot, Ppp, PppPlan (+40 more)

### Community 4 - "sources.rs"
Cohesion: 0.16
Nodes (34): a_zero_speed_profile_comment_gives_one_line(), an_override_changes_only_that_client(), an_override_for_a_missing_mac_has_no_effect(), an_override_in_per_plan_moves_to_its_plan_node(), an_override_on_a_skipped_lease_gives_no_circuit(), by_name(), cfg(), cfg_text() (+26 more)

### Community 5 - "view.rs"
Cohesion: 0.10
Nodes (25): a_broken_config_file_shows_the_error(), a_skipped_lease_cannot_get_an_override(), an_edit_of_a_discovered_server_makes_its_entry(), an_override_without_a_lease_shows_no_active_lease(), DFILE, discovered_servers_and_leases_are_listed(), dsnap(), dview() (+17 more)

### Community 6 - "network.rs"
Cohesion: 0.12
Nodes (22): a_deep_node_keeps_its_name_on_a_narrow_screen(), a_narrow_screen_keeps_a_short_rate(), children(), collapsed_tree_shows_only_roots(), expand_all_and_collapse_all(), key(), names(), narrow() (+14 more)

### Community 7 - "cycle.rs"
Cohesion: 0.11
Nodes (52): a_row_that_stays_keeps_its_group_node(), AAA, an_override_gives_the_same_rate_in_v2_pair_v3_pair_and_v3_network(), assemble(), cfg(), dhcp_cfg(), duplicate_ip_existing_vs_existing_keeps_collected(), duplicate_ip_new_vs_existing_keeps_existing() (+44 more)

### Community 8 - "files.rs"
Cohesion: 0.09
Nodes (27): CircuitNames, backup(), backup_keeps_the_last_n(), backup_keeps_the_new_folder_and_other_folders(), finish_secret(), HDR, Live, load_state() (+19 more)

### Community 9 - "SettingsView"
Cohesion: 0.10
Nodes (5): path_text(), concrete(), SettingsView, show(), Shown

### Community 10 - "Config"
Cohesion: 0.07
Nodes (35): a_bad_or_duplicate_mac_is_an_error(), a_circuit_comment_must_not_be_the_static_comment(), a_client_rate_below_the_floor_is_an_error(), BackstopCap, BackstopCaps, ClientTemplate, defaults_table(), dhcp_client_mac_forms_load() (+27 more)

### Community 11 - "LQoSync3 TUI Cosmetics Implementation Plan"
Cohesion: 0.06
Nodes (30): File Map and Shared Interfaces, Global Constraints, Implementation Record, LQoSync3 TUI Cosmetics Implementation Plan, Review Focus, Spec Coverage and Plan Review, Task 1: Theme Configuration and Shared Palette, Task 2: Shell, Key Chips, and Minimum Terminal Size (+22 more)

### Community 12 - "draft.rs"
Cohesion: 0.08
Nodes (32): a_bad_router_name_is_rejected(), a_change_keeps_comments_and_key_order(), add_and_remove_a_dhcp_server(), add_and_remove_a_router(), changed_counts_the_keys(), diff_lines_show_old_and_new(), Draft, FILE (+24 more)

### Community 13 - ".on_key"
Cohesion: 0.20
Nodes (34): a_bad_number_shows_the_key_error(), a_changed_file_shows_the_disk_diff_and_can_restart(), a_failed_preview_blocks_apply(), a_failed_preview_shows_that_apply_is_blocked(), a_secret_for_a_removed_router_is_dropped(), a_typed_password_is_never_in_the_buffer(), add_a_router_asks_for_a_masked_password(), apply_asks_for_a_confirmation() (+26 more)

### Community 14 - "Palette"
Cohesion: 0.21
Nodes (5): a_box_title_has_the_btop_brackets_and_hotkey(), a_panel_has_the_box_colour_and_marks_focus_in_the_title(), hex(), keys_preserve_text_and_highlight_brackets(), Palette

### Community 15 - "meta.rs"
Cohesion: 0.06
Nodes (42): DEFAULTS_TOML, all(), Base, Float, List, Text, build(), CHOICES (+34 more)

### Community 16 - "Configuration reference"
Cohesion: 0.05
Nodes (36): config/defaults.toml embedded defaults, Plan 2b deferred minor items, RouterOS listen change detection (future), Configuration reference, Comment speed format (Name|25M/5M), Map from config.json, DHCP server discovery, Exclusions ([exclude], [routers.exclude]) (+28 more)

### Community 17 - "apply.rs"
Cohesion: 0.08
Nodes (39): config_digest(), config_digest_is_stable_and_16_hex(), a_changed_file_on_disk_stops_the_apply(), a_toml_write_error_rolls_back_the_secrets(), apply(), apply_writes_a_backup_and_keeps_n(), ApplyError, Changed (+31 more)

### Community 18 - "runner::Runner::cycle (one full cycle)"
Cohesion: 0.06
Nodes (36): Applier (hash, atomic write, reload), Circuit Model, Collector (RouterOS API reads), Config Loader, Proposed Crate Layout, libreqos_network.json, lqosd (LibreQoS daemon), network.json (+28 more)

### Community 19 - "preview.rs"
Cohesion: 0.16
Nodes (32): cycle_inputs(), Inputs, a_failed_preview_is_failed_not_zero(), a_new_router_effect_is_not_known(), a_rate_change_gives_the_expected_circuit_changes(), a_router_that_was_not_online_gives_a_warning(), a_source_turned_on_is_not_known(), ALICE (+24 more)

### Community 20 - "MikroTik Session 5"
Cohesion: 0.16
Nodes (14): lqosync3 runtime design (Plan 2), Atomic write (temp file, sync_all, rename, fsync folder; only on byte change), Files: lock, state.json, status.json, candidate, applied, First-run seed (seed_state, seed_nodes from live files), Instance lock <state>/lqosync3.lock (File::try_lock), mikrotik-tokio 0.1.0 16-reply burst cancel bug, Plan 2 scope (in / not in), Process: run loop, SIGUSR1, SIGHUP, SIGTERM/SIGINT, once, check, diff, defaults (+6 more)

### Community 21 - "lqosync3 core design spec"
Cohesion: 0.16
Nodes (23): 2026-10-31 Plan 1: lqosync3 core library session, Plan 1 review fixes (commit 529fa36), Session 2026-10-31 project setup and v3 simulation, Session: updatecsv.py dissection, ADR-0005 Topology follows v3, max_floor_mbps (0.2) / safe_min_mbps (0.1), Parent Node ID column, ADR-0006 v3 guards and node types (+15 more)

### Community 22 - "speed.rs"
Cohesion: 0.10
Nodes (23): RateLimitOrder, DownloadUpload, RouterosRxTx, UnitlessUnit, Bps, Mbps, plan_label(), circuit_rates() (+15 more)

### Community 23 - "RouterCfg"
Cohesion: 0.23
Nodes (18): RouterCfg, RouterSnapshot, collect_dhcp(), collect_hotspot(), collect_ppp(), collect_router(), Collected, comment_rate() (+10 more)

### Community 24 - "Review Focus"
Cohesion: 0.17
Nodes (11): Global Constraints, lqosync3 TUI Implementation Plan (Plan 4b: the Settings editor), Review Focus, Review rulings (fact-check of 2026-11-01), Task 1: Key metadata, Task 2: The draft (edit model), Task 3: Validate and secrets, Task 4: Preview (+3 more)

### Community 25 - "theme.rs"
Cohesion: 0.14
Nodes (11): btop_gradient_ends(), Grad, Cpu, Free, Mem, Used, lerp(), light_gradients_are_readable_on_the_light_canvas() (+3 more)

### Community 26 - "2026-10-31 First live shadow run"
Cohesion: 0.17
Nodes (6): 2026-10-31 First live shadow run, FSEHub-Distro router (10.200.1.2:8728), Shadow run results (62 rows, 167 ms, 26 text-only updates), Production state (Python live, lqosync3 shadow, LibreQoS 2.2), Differences from updatecsv.py, Task 1: Crate scaffold and config

### Community 28 - "Theme"
Cohesion: 0.40
Nodes (5): Theme, Dark, Light, Mono, Tui

### Community 29 - "Config"
Cohesion: 0.13
Nodes (11): Config, deep_merge(), defaults(), DhcpSource, LeaseComment, ServerComment, Settings, Exclude (+3 more)

### Community 30 - "devices.rs"
Cohesion: 0.15
Nodes (16): circuits_count_distinct_ids(), compact_detail_fields_are_actually_visible(), compact_detail_has_ipv6_and_comment_in_seven_lines(), data(), devices_tab_renders_flags_as_words(), draw_builds_rows_only_for_the_visible_window(), enter_opens_the_parent_node(), key() (+8 more)

### Community 31 - "dhcp_server_gives_override_or_"
Cohesion: 0.21
Nodes (8): BandwidthDefaults, DhcpCfg, DhcpClient, DhcpMode, PerPlan, PerSite, DhcpServer, HotspotCfg

### Community 32 - "journal.rs"
Cohesion: 0.14
Nodes (9): journal_start_error_is_shown(), journal_stderr_is_shown(), JournalHandle, JournalMsg, Error, Line, short_iso_metadata_is_secondary_and_copy_is_raw(), spawn_journal() (+1 more)

### Community 33 - "lqosync3 LibreQoS v3 targets (Plan 3)"
Cohesion: 0.22
Nodes (9): CLI, Config changes, lqosync3 LibreQoS v3 targets (Plan 3), Offline check result (2026-10-31), Out of scope, Purpose, Target selection (ADR-0002), `v3_network` (+1 more)

### Community 34 - "Review Focus"
Cohesion: 0.25
Nodes (7): Global Constraints, lqosync3 LibreQoS v3 Implementation Plan (Plan 3), Review Focus, Task 1: Detector (`src/detect.rs`), Task 2: `v3_network` writer (`src/writer/v3network.rs`), Task 3: Runner, CLI, and check, Task 4: Offline check with the v3 parser

### Community 35 - "main.rs"
Cohesion: 0.21
Nodes (7): exit_with(), main(), notify(), now(), print_summary(), run(), SECRET_OWNER_UID

### Community 36 - "lqosync3 core design spec"
Cohesion: 0.08
Nodes (31): lqosync3 core crate (config, speed, model, state, sources, topology, cycle, validate, writer), ADR-0009 DHCP discovery and exclusions, Six Network Modes, Speed Rules, Topology Builder, LibreQoS v3 topology.compile_mode, lqosync3 Core Implementation Plan (Plan 1), Plan 1 review focus (edge cases) (+23 more)

### Community 38 - "data.rs"
Cohesion: 0.12
Nodes (24): a_bad_network_json_keeps_the_last_data_and_shows_an_error(), a_missing_csv_is_an_error(), collect_names(), Env, env(), FLAG_DUP_IP, FLAG_NO_PARENT, FLAG_RATE (+16 more)

### Community 39 - "journal.rs"
Cohesion: 0.14
Nodes (12): buffer_keeps_the_last_n_lines(), journal_tab_renders(), JournalView, key(), level_filter(), level_of(), mouse_drag_selects_lines(), search_filters_the_lines() (+4 more)

### Community 40 - "a_rejected_apply_says_the_new_file_is_on_disk"
Cohesion: 0.24
Nodes (5): line_diff(), a_rejected_apply_says_the_new_file_is_on_disk(), Disk, disk(), the_header_says_loaded_while_the_first_cycle_runs()

### Community 41 - "src/validate.rs"
Cohesion: 0.16
Nodes (22): Network, NetworkMode, FlatGlobalRoot, FlatPromotion, FlatRouterRoot, GroupedRouterRoot, NoParent, RootPromotion (+14 more)

### Community 42 - "dashboard.rs"
Cohesion: 0.11
Nodes (16): CPU_SCALE, data(), digest_check_shows_not_loaded(), export_dashboard_frames(), Kind, Reload, SyncNow, lines() (+8 more)

### Community 43 - "app.rs"
Cohesion: 0.05
Nodes (66): a_click_on_a_tab_title_selects_it(), a_key_press_clears_the_message_so_errors_show(), a_long_hint_wraps_without_a_lost_key(), a_too_large_copy_is_not_sent(), a_wide_header_is_one_line(), Action, ApplySettings, Copy (+58 more)

### Community 45 - "proc.rs"
Cohesion: 0.16
Nodes (16): systemctl_show(), count(), boot_time(), clk_tck(), cpu_percent(), parse_rss_kb(), parse_stat(), parse_stat_handles_spaces_and_parens_in_comm() (+8 more)

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
Cohesion: 0.08
Nodes (32): Orphan branch lqosync3, RouterOS listen command (untested), 14/15 CSV columns (v2_pair/v3_pair), libreqos_network.json schema_version 2, Plan phases (Plan 1/2/2b done; Plan 3 v3, Plan 4 TUI, Plan 5 installer), Runtime modes (shadow, apply), install.sh one-line installer (target contract), Installation (docs) (+24 more)

### Community 56 - "Review Focus"
Cohesion: 0.15
Nodes (12): Global Constraints, lqosync3 TUI Implementation Plan (Plan 4a: service changes and read-only screens), Review Focus, Review rulings (fact-check of 2026-10-31), Task 1: Service data for the TUI, Task 2: TUI frame, Task 3: Loader worker, Task 4: ShapedDevices tab (+4 more)

### Community 57 - "diff.rs"
Cohesion: 0.18
Nodes (15): changed_columns(), diff_config(), diff_dirs(), diff_dirs_needs_candidate_files(), diff_ignores_column_order_row_order_and_number_text(), diff_pair(), diff_reports_added_removed_changed(), diff_stops_for_v3_network() (+7 more)

### Community 58 - "ReadError"
Cohesion: 0.10
Nodes (22): a_dead_kept_session_reconnects_without_a_retry(), a_login_error_is_not_retried(), Fake, FakeSession, gives_last_error_after_all_retries(), ppp_router(), read_snapshot(), ReadError (+14 more)

### Community 59 - "v3network.rs"
Cohesion: 0.27
Nodes (15): a_bad_ip_value_skips_the_circuit(), a_duplicate_node_id_is_an_error(), a_node_type_that_v3_does_not_know_is_an_error(), a_node_without_id_or_type_gets_a_stable_id_and_site(), a_v2_plan_node_from_the_state_is_written_as_site(), bad_circuits_are_skipped_with_an_alert(), prefix(), render() (+7 more)

### Community 60 - "ADR-0009 DHCP discovery and exclusions"
Cohesion: 0.20
Nodes (10): 1. Intent, 2. Goals, non-goals, anti-goals, 3. Approaches, 4. Config, 5. Cycle (`sources.rs`), 6. TUI (Settings tab), 7. Output (LibreQoS 2.2 and 3.x), 8. Tests (+2 more)

### Community 61 - "TUI cosmetics design handoff"
Cohesion: 0.29
Nodes (7): Checks, Evidence before takeover, Goal, Implementation plan and review, Next action, Result, TUI cosmetics design handoff

### Community 62 - "cli.rs"
Cohesion: 0.18
Nodes (19): Args, check(), check_finds_plain_api_and_no_source_router(), check_reads_the_telegram_token_when_enabled(), check_reports_a_bad_secret_and_a_missing_lqos_dir(), check_shows_the_target_or_the_detection_error(), check_warns_when_the_libreqos_state_directory_is_missing(), DEFAULT_CONFIG (+11 more)

### Community 63 - "scenarios.rs"
Cohesion: 0.13
Nodes (21): CycleInput, CycleOutput, names(), node_rate(), Columns, V2, V3, CsvRow (+13 more)

### Community 64 - "RouterAccess"
Cohesion: 0.44
Nodes (12): RouterAccess, RouterMemo, acc(), classify(), first_unreachable_sets_since(), login_tls_trap_timeout_are_failed_and_keep_memo(), memo(), name() (+4 more)

### Community 65 - "TUI cosmetics implementation"
Cohesion: 0.29
Nodes (7): Changes, Checks, Goal, Limits and next action, Review and fixes, Revision, TUI cosmetics implementation

### Community 66 - "detect.rs"
Cohesion: 0.18
Nodes (18): Libreqos, Target, Auto, V2Pair, V3Network, V3Pair, Detected, Env (+10 more)

### Community 67 - "ADR-0008 Terminal UI"
Cohesion: 0.14
Nodes (20): ADR-0003 Router failed vs offline, offline_action (default remove), offline_after (default 300 s), ADR-0004 TOML policy, Python defaults, Speed source order TOML lists, ADR-0007 Fresh design, not a port, Scenario tests (replace golden parity tests), Semantic compare with live files before cutover (+12 more)

### Community 68 - "mikrotik.rs"
Cohesion: 0.15
Nodes (5): map_err(), MikrotikConnector, MikrotikSession, tls_client_config(), Setup

### Community 69 - "PppSource"
Cohesion: 0.29
Nodes (7): PppSource, ActiveComment, Default, ProfileComment, ProfileName, ProfileRateLimit, SecretComment

### Community 70 - "Role"
Cohesion: 0.29
Nodes (7): Role, Accent, Base, Error, Ok, Secondary, Warning

### Community 71 - "State"
Cohesion: 0.16
Nodes (11): Entry, GuardHold, Source, Dhcp, Hotspot, Ppp, Unknown, State (+3 more)

### Community 72 - "TUI btop++ refinement"
Cohesion: 0.29
Nodes (6): Direction, Implementation and proof, Reason, Second correction: btop++ Default theme, Third change: one process chart, TUI btop++ refinement

### Community 73 - ".draw"
Cohesion: 0.19
Nodes (13): a_very_tall_area_does_not_overflow(), card_areas(), dashboard_card_geometry(), dashboard_tab_renders(), DashboardView, draw_routers(), io_and_net_rates_come_from_counter_differences(), no_process_data_gives_no_samples() (+5 more)

### Community 74 - "Input"
Cohesion: 0.11
Nodes (19): KeyPath, counts_refresh_after_draft_actions(), Extra, Client, Lease, Note, Group, group_change_counts() (+11 more)

### Community 75 - "Review Focus"
Cohesion: 0.22
Nodes (8): Global Constraints, lqosync3 DHCP Client Overrides Implementation Plan (Plan 4c), Review Focus, Review rulings (fact-check of 2026-11-01), Task 1: Config, Task 2: The shared lease list and the cycle, Task 3: The DHCP view of the Settings tab, Task 4: Docs, review, graph

### Community 77 - "LeaseResult"
Cohesion: 0.22
Nodes (9): LeaseResult, Shaped, Skipped, RateSource, LeaseComment, Override, ServerComment, Settings (+1 more)

### Community 78 - "BoxKind"
Cohesion: 0.40
Nodes (5): BoxKind, Cpu, Mem, Net, Proc

### Community 79 - "Stage"
Cohesion: 0.50
Nodes (4): Stage, Edit, Result, Verifying

### Community 80 - "src/secrets.rs"
Cohesion: 0.22
Nodes (7): error_never_contains_the_secret(), file(), missing_file_names_the_path(), read_secret(), reads_and_trims_a_good_secret(), rejects_group_or_other_bits(), rejects_wrong_owner()

### Community 81 - "status.rs"
Cohesion: 0.26
Nodes (9): Change, Report, circuit_line(), updated_line(), CircuitInfo, ReportSummary, status_json_has_the_spec_fields(), summary_takes_values_from_rows_and_the_old_state() (+1 more)

### Community 82 - "settings/validate.rs"
Cohesion: 0.31
Nodes (7): Env, env(), parse(), validate(), validate_accepts_a_typed_password_file(), validate_names_the_key(), validate_returns_warnings()

### Community 83 - "Data"
Cohesion: 0.47
Nodes (4): Data, DevicesView, flag_words(), source_of()

### Community 84 - "RouterResult"
Cohesion: 0.36
Nodes (8): dhcp_snap(), ppp(), ppp_owned(), RouterResult, Failed, Offline, Online, Unreachable

### Community 85 - "Level"
Cohesion: 0.31
Nodes (9): check_warns_when_the_file_is_not_loaded(), check_with(), CheckLine, digest_line(), Level, Error, Ok, Warn (+1 more)

### Community 86 - "Column"
Cohesion: 0.22
Nodes (8): Column, .ALL, Down, Ipv4, Name, Parent, Source, Up

### Community 87 - "Command"
Cohesion: 0.25
Nodes (8): Command, Check, Defaults, Detect, Diff, Once, Run, Tui

### Community 88 - "LogLevel"
Cohesion: 0.40
Nodes (5): LogLevel, Debug, Error, Info, Warn

### Community 90 - "btop TUI, review fixes, and a new LibreQoS installation"
Cohesion: 0.33
Nodes (6): btop TUI, review fixes, and a new LibreQoS installation, Changes, Checks, Decisions of the operator, Goal, Limits and next action

### Community 91 - "ADR-0010 — A full LibreQoS run before the first full run"
Cohesion: 0.40
Nodes (5): ADR-0010 — A full LibreQoS run before the first full run, Consequences, Context, Decision, Rejected

### Community 92 - "VModal"
Cohesion: 0.50
Nodes (4): VModal, ConfirmApply, RemoveRouter, RemoveServer

## Ambiguous Edges - Review These
- `Collector (RouterOS API reads)` → `RouterOS listen change detection (future)`  [AMBIGUOUS]
  docs/ARCHITECTURE.md · relation: conceptually_related_to
- `Stable node id (lq3- FNV-1a 64)` → `Scenario tests (replace golden parity tests)`  [AMBIGUOUS]
  brain/20-decisions/ADR-0007-fresh-design-not-a-port.md · relation: conceptually_related_to

## Knowledge Gaps
- **187 isolated node(s):** `lqosync3`, `Check`, `Once`, `Run`, `Diff` (+182 more)
  These have ≤1 connection - possible missing edges or undocumented components. (Counts symbols only; 522 node(s) total have ≤1 connection when file, concept and rationale nodes are included.)
- **12 thin communities (<3 nodes) omitted from report** — run `graphify query` to explore isolated nodes.

## Suggested Questions
_Questions this graph is uniquely positioned to answer:_

- **Why does `Task 3: Transport (post_json, send, token, chat_id)` connect `Telegram Notifier` to `Configuration reference`?**
  _High betweenness centrality (0.194) - this node is a cross-community bridge._
- **Are the 3 inferred relationships involving `Config` (e.g. with `diff_config()` and `action_timeout()`) actually correct?**
  _`Config` has 3 INFERRED edges - model-reasoned connections that need verification._
- **What connects `lqosync3`, `Check`, `Once` to the rest of the system?**
  _187 weakly-connected nodes found - possible documentation gaps or missing edges._
- **Should `Telegram Notifier` be split into smaller, more focused modules?**
  _Cohesion score 0.09049773755656108 - nodes in this community are weakly interconnected._
- **Why does `Config` connect `Config` to `mock_router.rs`, `Telegram Notifier`, `runner.rs`, `Topology`, `sources.rs`, `cycle.rs`, `files.rs`, `Config`, `draft.rs`, `meta.rs`, `preview.rs`, `RouterCfg`, `Theme`, `dhcp_server_gives_override_or_`, `main.rs`, `data.rs`, `src/validate.rs`, `tui/mod.rs`, `diff.rs`, `ReadError`, `cli.rs`, `scenarios.rs`, `RouterAccess`, `detect.rs`, `settings/validate.rs`, `Level`?**
  _High betweenness centrality (0.141) - this node is a cross-community bridge._

### Low-confidence Hints
_AMBIGUOUS edges — the extractor was unsure. Verify before acting on these._

- **What is the exact relationship between `Collector (RouterOS API reads)` and `RouterOS listen change detection (future)`?**
  _Edge tagged AMBIGUOUS (relation: conceptually_related_to) - confidence is low._
- **What is the exact relationship between `Stable node id (lq3- FNV-1a 64)` and `Scenario tests (replace golden parity tests)`?**
  _Edge tagged AMBIGUOUS (relation: conceptually_related_to) - confidence is low._