# Graph Report - LQoSync3.0  (2026-11-01)

## Corpus Check
- 101 files · ~132,491 words
- Verdict: corpus is large enough that graph structure adds value.
- Unclassified: 4 file(s) not represented in the graph (top: (none) 2, .toml 1, .service 1)

## Summary
- 2127 nodes · 5333 edges · 100 communities (85 shown, 15 thin omitted)
- Extraction: 97% EXTRACTED · 3% INFERRED · 0% AMBIGUOUS · INFERRED: 182 edges (avg confidence: 0.87)
- Token cost: 0 input · 0 output

## Graph Freshness
- Built from commit: `d127f90e`
- Run `git rev-parse HEAD` and compare to check if the graph is stale.
- Run `graphify update .` after code changes (no API cost).

## Community Hubs (Navigation)
- mock_router.rs
- notify.rs
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
- speed.rs
- RouterCfg
- Review Focus
- theme.rs
- 2026-10-31 First live shadow run
- ci.yml
- Writers (one per target)
- dhcp_server_gives_override_or_
- steps
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
- lib.rs
- tui/mod.rs
- lqosync3 terminal UI (Plan 4)
- LibreQoS v3 ingress document
- Operations guide
- Review Focus
- tg
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
- .cycle_inner
- PppSource
- Role
- Versions, releases, and upgrades
- TUI btop++ refinement
- .draw
- Input
- Review Focus
- AnyCertificate
- LeaseResult
- BoxKind
- Stage
- src/secrets.rs
- Status
- settings/validate.rs
- Telegram
- RouterResult
- Level
- LibreQoS v3 upgrade investigation
- Command
- LogLevel
- super
- btop TUI, review fixes, and a new LibreQoS installation
- ADR-0010 — A full LibreQoS run before the first full run
- VModal
- .connect
- Wait
- s
- RuntimeMode
- build.rs
- install.sh
- Host storage cleanup

## God Nodes (most connected - your core abstractions)
1. `Config` - 89 edges
2. `SettingsView` - 71 edges
3. `CsvRow` - 48 edges
4. `Palette` - 39 edges
5. `key()` - 36 edges
6. `App` - 33 edges
7. `Handoff` - 33 edges
8. `run()` - 32 edges
9. `Draft` - 32 edges
10. `lqosync3 core design spec` - 32 edges

## Surprising Connections (you probably didn't know these)
- `v3 offline simulation method` --semantically_similar_to--> `/run/lqos/bus 0777 safety caution`  [INFERRED] [semantically similar]
  brain/30-knowledge/v3-offline-simulation-method.md → docs/LIBREQOS-V3-INGRESS.md
- `v3 offline simulation method` --semantically_similar_to--> `Offline v3 simulation`  [INFERRED] [semantically similar]
  brain/30-knowledge/v3-offline-simulation-method.md → docs/LIBREQOS-V3-INGRESS.md
- `Task 2: Events and message` --references--> `split()`  [EXTRACTED]
  docs/superpowers/plans/2026-10-31-lqosync3-telegram.md → src/notify.rs
- `Task 3: Transport (post_json, send, token, chat_id)` --references--> `post_json()`  [EXTRACTED]
  docs/superpowers/plans/2026-10-31-lqosync3-telegram.md → src/notify.rs
- `Session 2026-10-31 project setup and v3 simulation` --references--> `libreqos-v3-preflight`  [INFERRED]
  brain/10-sessions/2026-10-31-project-setup-and-v3-simulation.md → docs/LIBREQOS-V3-INGRESS.md

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

## Communities (100 total, 15 thin omitted)

### Community 0 - "mock_router.rs"
Cohesion: 0.15
Nodes (25): read_router(), Answer, Empty, Raw, Rows, Silent, Trap, closed_port_is_unreachable() (+17 more)

### Community 1 - "notify.rs"
Cohesion: 0.11
Nodes (20): a_long_multi_line_error_splits_into_valid_html_parts(), body_complete(), cap(), circuit_line(), code_block(), dechunk(), is_chunked(), open_server() (+12 more)

### Community 2 - "runner.rs"
Cohesion: 0.25
Nodes (32): a_config_reload_error_shows_in_status_until_a_good_reload(), a_fresh_libreqos_directory_gets_both_files(), a_held_lock_stops_the_cycle_without_touching_status(), a_live_shaper_without_its_csv_gets_no_full_run(), a_panicking_router_task_fails_only_that_router(), a_shipped_csv_without_a_full_run_still_gets_a_full_run(), apply_backs_up_writes_and_reloads(), assemble_error_writes_nothing() (+24 more)

### Community 3 - "Topology"
Cohesion: 0.12
Nodes (44): StoredNode, GroupKey, Hotspot, Ppp, a_kept_child_keeps_its_operator_children(), backstop_cap_limits_group(), build_nodes(), carried_nodes_of_failed_router_stay() (+36 more)

### Community 4 - "sources.rs"
Cohesion: 0.16
Nodes (34): a_zero_speed_profile_comment_gives_one_line(), an_override_changes_only_that_client(), an_override_for_a_missing_mac_has_no_effect(), an_override_in_per_plan_moves_to_its_plan_node(), an_override_on_a_skipped_lease_gives_no_circuit(), by_name(), cfg(), cfg_text() (+26 more)

### Community 5 - "view.rs"
Cohesion: 0.10
Nodes (22): a_skipped_lease_cannot_get_an_override(), an_edit_of_a_discovered_server_makes_its_entry(), an_override_without_a_lease_shows_no_active_lease(), DFILE, discovered_servers_and_leases_are_listed(), dsnap(), dview(), extra_item() (+14 more)

### Community 6 - "devices.rs"
Cohesion: 0.06
Nodes (52): narrow_terminal_puts_the_side_panel_under_the_main_panel(), split_main(), Data, circuits_count_distinct_ids(), Column, .ALL, Down, Ipv4 (+44 more)

### Community 7 - "cycle.rs"
Cohesion: 0.11
Nodes (50): a_row_that_stays_keeps_its_group_node(), AAA, an_override_gives_the_same_rate_in_v2_pair_v3_pair_and_v3_network(), cfg(), dhcp_cfg(), duplicate_ip_existing_vs_existing_keeps_collected(), duplicate_ip_new_vs_existing_keeps_existing(), empty_online_router_removes_its_rows() (+42 more)

### Community 8 - "files.rs"
Cohesion: 0.07
Nodes (36): CircuitNames, Entry, backup(), backup_keeps_the_last_n(), backup_keeps_the_new_folder_and_other_folders(), finish_secret(), HDR, Live (+28 more)

### Community 9 - "SettingsView"
Cohesion: 0.11
Nodes (5): path_text(), concrete(), SettingsView, show(), Shown

### Community 10 - "Config"
Cohesion: 0.06
Nodes (40): a_bad_or_duplicate_mac_is_an_error(), a_circuit_comment_must_not_be_the_static_comment(), a_client_rate_below_the_floor_is_an_error(), BackstopCap, BackstopCaps, ClientTemplate, Config, deep_merge() (+32 more)

### Community 11 - "LQoSync3 TUI Cosmetics Implementation Plan"
Cohesion: 0.06
Nodes (30): File Map and Shared Interfaces, Global Constraints, Implementation Record, LQoSync3 TUI Cosmetics Implementation Plan, Review Focus, Spec Coverage and Plan Review, Task 1: Theme Configuration and Shared Palette, Task 2: Shell, Key Chips, and Minimum Terminal Size (+22 more)

### Community 12 - "draft.rs"
Cohesion: 0.06
Nodes (47): diff_config(), diff_dirs(), diff_dirs_needs_candidate_files(), diff_ignores_column_order_row_order_and_number_text(), diff_pair(), diff_reports_added_removed_changed(), diff_stops_for_v3_network(), diff_stops_in_apply_mode() (+39 more)

### Community 13 - ".on_key"
Cohesion: 0.20
Nodes (34): a_bad_number_shows_the_key_error(), a_changed_file_shows_the_disk_diff_and_can_restart(), a_failed_preview_blocks_apply(), a_failed_preview_shows_that_apply_is_blocked(), a_secret_for_a_removed_router_is_dropped(), a_typed_password_is_never_in_the_buffer(), add_a_router_asks_for_a_masked_password(), apply_asks_for_a_confirmation() (+26 more)

### Community 15 - "meta.rs"
Cohesion: 0.06
Nodes (42): DEFAULTS_TOML, all(), Base, Float, List, Text, build(), CHOICES (+34 more)

### Community 16 - "Configuration reference"
Cohesion: 0.07
Nodes (29): config/defaults.toml embedded defaults, Plan 2b deferred minor items, RouterOS listen change detection (future), Configuration reference, Comment speed format (Name|25M/5M), Map from config.json, DHCP server discovery, Exclusions ([exclude], [routers.exclude]) (+21 more)

### Community 17 - "apply.rs"
Cohesion: 0.08
Nodes (39): config_digest(), config_digest_is_stable_and_16_hex(), a_changed_file_on_disk_stops_the_apply(), a_toml_write_error_rolls_back_the_secrets(), apply(), apply_writes_a_backup_and_keeps_n(), ApplyError, Changed (+31 more)

### Community 18 - "runner::Runner::cycle (one full cycle)"
Cohesion: 0.09
Nodes (23): network.json, Router state (online/failed/unreachable/offline), routerstate::classify, Core design spec (2026-10-31-lqosync3-core-design.md), cycle::assemble (pure core), files::backup (keep_backups), files::lock (lqosync3.lock), files::load_state (state.json or seed) (+15 more)

### Community 19 - "preview.rs"
Cohesion: 0.17
Nodes (29): cycle_inputs(), a_failed_preview_is_failed_not_zero(), a_new_router_effect_is_not_known(), a_rate_change_gives_the_expected_circuit_changes(), a_router_that_was_not_online_gives_a_warning(), a_source_turned_on_is_not_known(), ALICE, an_old_snapshot_gives_the_stale_warning() (+21 more)

### Community 20 - "MikroTik Session 5"
Cohesion: 0.16
Nodes (14): lqosync3 runtime design (Plan 2), Atomic write (temp file, sync_all, rename, fsync folder; only on byte change), Files: lock, state.json, status.json, candidate, applied, First-run seed (seed_state, seed_nodes from live files), Instance lock <state>/lqosync3.lock (File::try_lock), mikrotik-tokio 0.1.0 16-reply burst cancel bug, Plan 2 scope (in / not in), Process: run loop, SIGUSR1, SIGHUP, SIGTERM/SIGINT, once, check, diff, defaults (+6 more)

### Community 21 - "lqosync3 core design spec"
Cohesion: 0.13
Nodes (30): 2026-10-31 Plan 1: lqosync3 core library session, Plan 1 review fixes (commit 529fa36), Session 2026-10-31 project setup and v3 simulation, Session: updatecsv.py dissection, Preflight update prepared, ADR-0003 Router failed vs offline, ADR-0004 TOML policy, Python defaults, ADR-0005 Topology follows v3 (+22 more)

### Community 22 - "speed.rs"
Cohesion: 0.10
Nodes (20): UnitlessUnit, Bps, Mbps, plan_label(), circuit_rates(), CircuitRates, clamp(), default_rate_and_circuit_rates() (+12 more)

### Community 23 - "RouterCfg"
Cohesion: 0.16
Nodes (23): parse_ipv4_net(), PppoeCfg, RouterCfg, RouterSnapshot, collect_dhcp(), collect_hotspot(), collect_ppp(), collect_router() (+15 more)

### Community 24 - "Review Focus"
Cohesion: 0.17
Nodes (11): Global Constraints, lqosync3 TUI Implementation Plan (Plan 4b: the Settings editor), Review Focus, Review rulings (fact-check of 2026-11-01), Task 1: Key metadata, Task 2: The draft (edit model), Task 3: Validate and secrets, Task 4: Preview (+3 more)

### Community 25 - "theme.rs"
Cohesion: 0.10
Nodes (19): Theme, Dark, Light, Mono, Tui, a_box_title_has_the_btop_brackets_and_hotkey(), a_panel_has_the_box_colour_and_marks_focus_in_the_title(), btop_gradient_ends() (+11 more)

### Community 26 - "2026-10-31 First live shadow run"
Cohesion: 0.17
Nodes (6): 2026-10-31 First live shadow run, FSEHub-Distro router (10.200.1.2:8728), Shadow run results (62 rows, 167 ms, 26 text-only updates), Production state (Python live, lqosync3 shadow, LibreQoS 2.2), Differences from updatecsv.py, Task 1: Crate scaffold and config

### Community 28 - "ci.yml"
Cohesion: 0.11
Nodes (19): jobs, msrv, test, runs-on, steps, name, on, pull_request (+11 more)

### Community 30 - "Writers (one per target)"
Cohesion: 0.13
Nodes (17): Applier (hash, atomic write, reload), Circuit Model, Collector (RouterOS API reads), Config Loader, Proposed Crate Layout, libreqos_network.json, lqosd (LibreQoS daemon), MikroTik RouterOS API (+9 more)

### Community 31 - "dhcp_server_gives_override_or_"
Cohesion: 0.21
Nodes (8): BandwidthDefaults, DhcpCfg, DhcpClient, DhcpMode, PerPlan, PerSite, DhcpServer, HotspotCfg

### Community 32 - "steps"
Cohesion: 0.11
Nodes (17): GH_TOKEN, jobs, release, name, on, push, permissions, contents (+9 more)

### Community 33 - "lqosync3 LibreQoS v3 targets (Plan 3)"
Cohesion: 0.22
Nodes (9): CLI, Config changes, lqosync3 LibreQoS v3 targets (Plan 3), Offline check result (2026-10-31), Out of scope, Purpose, Target selection (ADR-0002), `v3_network` (+1 more)

### Community 34 - "Review Focus"
Cohesion: 0.25
Nodes (7): Global Constraints, lqosync3 LibreQoS v3 Implementation Plan (Plan 3), Review Focus, Task 1: Detector (`src/detect.rs`), Task 2: `v3_network` writer (`src/writer/v3network.rs`), Task 3: Runner, CLI, and check, Task 4: Offline check with the v3 parser

### Community 35 - "main.rs"
Cohesion: 0.18
Nodes (10): Task 2: Events and message, exit_with(), main(), notify(), now(), print_summary(), run(), SECRET_OWNER_UID (+2 more)

### Community 36 - "lqosync3 core design spec"
Cohesion: 0.08
Nodes (32): lqosync3 core crate (config, speed, model, state, sources, topology, cycle, validate, writer), ADR-0009 DHCP discovery and exclusions, Six Network Modes, Speed Rules, Topology Builder, LibreQoS v3 topology.compile_mode, lqosync3 Core Implementation Plan (Plan 1), Plan 1 review focus (edge cases) (+24 more)

### Community 38 - "data.rs"
Cohesion: 0.14
Nodes (24): a_bad_network_json_keeps_the_last_data_and_shows_an_error(), a_missing_csv_is_an_error(), collect_names(), Env, env(), FLAG_DUP_IP, FLAG_NO_PARENT, FLAG_RATE (+16 more)

### Community 39 - "journal.rs"
Cohesion: 0.08
Nodes (21): buffer_keeps_the_last_n_lines(), journal_start_error_is_shown(), journal_stderr_is_shown(), journal_tab_renders(), JournalHandle, JournalMsg, Error, Line (+13 more)

### Community 40 - "a_rejected_apply_says_the_new_file_is_on_disk"
Cohesion: 0.15
Nodes (7): a_broken_config_file_shows_the_error(), a_rejected_apply_says_the_new_file_is_on_disk(), Disk, disk(), no_snapshot_shows_only_the_toml_servers(), removed_children_count_in_surviving_owner(), the_header_says_loaded_while_the_first_cycle_runs()

### Community 41 - "src/validate.rs"
Cohesion: 0.16
Nodes (22): Network, NetworkMode, FlatGlobalRoot, FlatPromotion, FlatRouterRoot, GroupedRouterRoot, NoParent, RootPromotion (+14 more)

### Community 42 - "dashboard.rs"
Cohesion: 0.09
Nodes (17): CPU_SCALE, data(), digest_check_shows_not_loaded(), export_dashboard_frames(), Kind, Reload, SyncNow, lines() (+9 more)

### Community 43 - "app.rs"
Cohesion: 0.06
Nodes (57): a_click_on_a_tab_title_selects_it(), a_key_press_clears_the_message_so_errors_show(), a_long_hint_wraps_without_a_lost_key(), a_too_large_copy_is_not_sent(), a_wide_header_is_one_line(), Action, ApplySettings, Copy (+49 more)

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

### Community 57 - "tg"
Cohesion: 0.23
Nodes (17): a_config_error_is_an_error_event(), a_very_long_error_is_cut(), an_empty_token_is_an_error(), an_error_is_sent_once_then_cleared(), an_error_with_other_numbers_is_not_a_new_event(), disabled_gives_no_message(), files_changed_lists_the_circuits(), first_cycle_sends_only_a_router_that_is_not_online() (+9 more)

### Community 58 - "ReadError"
Cohesion: 0.07
Nodes (31): map_err(), MikrotikConnector, MikrotikSession, tls_client_config(), RosRow, a_dead_kept_session_reconnects_without_a_retry(), a_login_error_is_not_retried(), Connector (+23 more)

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
Cohesion: 0.19
Nodes (14): check(), check_finds_plain_api_and_no_source_router(), check_reads_the_telegram_token_when_enabled(), check_reports_a_bad_secret_and_a_missing_lqos_dir(), check_shows_the_target_or_the_detection_error(), check_warns_when_the_libreqos_state_directory_is_missing(), DEFAULT_CONFIG, Env (+6 more)

### Community 63 - "scenarios.rs"
Cohesion: 0.09
Nodes (32): assemble(), Change, changed_columns(), CycleInput, CycleOutput, names(), node_rate(), Report (+24 more)

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
Nodes (14): offline_action (default remove), offline_after (default 300 s), Speed source order TOML lists, TUI config editor (toml_edit), SIGUSR1 sync now / SIGHUP reload, status.json state file, updatecsv.py dissection, updatecsv.py defects to fix (+6 more)

### Community 68 - ".cycle_inner"
Cohesion: 0.15
Nodes (7): target_name(), random_id(), random_ids_have_the_length_and_charset(), Runner, Runner<C>, state_name(), write_snapshot()

### Community 69 - "PppSource"
Cohesion: 0.13
Nodes (15): DhcpSource, LeaseComment, ServerComment, Settings, PppSource, ActiveComment, Default, ProfileComment (+7 more)

### Community 70 - "Role"
Cohesion: 0.29
Nodes (7): Role, Accent, Base, Error, Ok, Secondary, Warning

### Community 71 - "Versions, releases, and upgrades"
Cohesion: 0.22
Nodes (8): Continuous integration, Find the source of a version, Install or upgrade on a shaper, Make a release, Roll back to an earlier release, Rules that keep the number correct, Version number, Versions, releases, and upgrades

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
Cohesion: 0.24
Nodes (7): error_never_contains_the_secret(), file(), missing_file_names_the_path(), read_secret(), reads_and_trims_a_good_secret(), rejects_group_or_other_bits(), rejects_wrong_owner()

### Community 81 - "Status"
Cohesion: 0.22
Nodes (8): RouterStatus, Status, Trigger, Once, Reload, Start, SyncNow, Timer

### Community 82 - "settings/validate.rs"
Cohesion: 0.31
Nodes (7): Env, env(), parse(), validate(), validate_accepts_a_typed_password_file(), validate_names_the_key(), validate_returns_warnings()

### Community 83 - "Telegram"
Cohesion: 0.43
Nodes (7): Task 3: Transport (post_json, send, token, chat_id), Telegram, chat_id(), send(), send_all(), tls_config(), token()

### Community 84 - "RouterResult"
Cohesion: 0.31
Nodes (9): dhcp_snap(), ppp(), ppp_owned(), RouterResult, Failed, Offline, Online, Unreachable (+1 more)

### Community 85 - "Level"
Cohesion: 0.31
Nodes (9): check_warns_when_the_file_is_not_loaded(), check_with(), CheckLine, digest_line(), Level, Error, Ok, Warn (+1 more)

### Community 86 - "LibreQoS v3 upgrade investigation"
Cohesion: 0.29
Nodes (7): Evidence, LibreQoS v3 upgrade investigation, Local facts, Package simulation and cleanup, Remaining upgrade checks, Upstream snapshot, Warning mechanisms

### Community 87 - "Command"
Cohesion: 0.22
Nodes (9): Command, Check, Defaults, Detect, Diff, Once, Run, Tui (+1 more)

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

### Community 93 - ".connect"
Cohesion: 0.38
Nodes (3): Fake, FakeConnector, FakeSession

### Community 94 - "Wait"
Cohesion: 0.33
Nodes (6): Wait, Done, Pending, Restarted, wait_result(), TimedOut

### Community 95 - "s"
Cohesion: 0.50
Nodes (5): Args, parse(), parse_commands_and_config_path(), s(), tui_command_parses()

### Community 96 - "RuntimeMode"
Cohesion: 0.40
Nodes (4): Runtime, RuntimeMode, Apply, Shadow

### Community 99 - "Host storage cleanup"
Cohesion: 0.67
Nodes (3): Checks and retained files, Host storage cleanup, Removed files

## Ambiguous Edges - Review These
- `RouterOS listen change detection (future)` → `Collector (RouterOS API reads)`  [AMBIGUOUS]
  docs/ARCHITECTURE.md · relation: conceptually_related_to
- `Stable node id (lq3- FNV-1a 64)` → `Scenario tests (replace golden parity tests)`  [AMBIGUOUS]
  brain/20-decisions/ADR-0007-fresh-design-not-a-port.md · relation: conceptually_related_to

## Knowledge Gaps
- **188 isolated node(s):** `lqosync3`, `Check`, `Once`, `Run`, `Diff` (+183 more)
  These have ≤1 connection - possible missing edges or undocumented components. (Counts symbols only; 558 node(s) total have ≤1 connection when file, concept and rationale nodes are included.)
- **15 thin communities (<3 nodes) omitted from report** — run `graphify query` to explore isolated nodes.

## Suggested Questions
_Questions this graph is uniquely positioned to answer:_

- **Why does `Task 3: Transport (post_json, send, token, chat_id)` connect `Telegram` to `Configuration reference`, `notify.rs`?**
  _High betweenness centrality (0.202) - this node is a cross-community bridge._
- **Are the 3 inferred relationships involving `Config` (e.g. with `diff_config()` and `action_timeout()`) actually correct?**
  _`Config` has 3 INFERRED edges - model-reasoned connections that need verification._
- **What connects `lqosync3`, `Check`, `Once` to the rest of the system?**
  _188 weakly-connected nodes found - possible documentation gaps or missing edges._
- **Should `notify.rs` be split into smaller, more focused modules?**
  _Cohesion score 0.11330049261083744 - nodes in this community are weakly interconnected._
- **Why does `Handoff` connect `lqosync3 core design spec` to `Configuration reference`, `2026-10-31 First live shadow run`, `runner::Runner::cycle (one full cycle)`, `Operations guide`?**
  _High betweenness centrality (0.156) - this node is a cross-community bridge._

### Low-confidence Hints
_AMBIGUOUS edges — the extractor was unsure. Verify before acting on these._

- **What is the exact relationship between `RouterOS listen change detection (future)` and `Collector (RouterOS API reads)`?**
  _Edge tagged AMBIGUOUS (relation: conceptually_related_to) - confidence is low._
- **What is the exact relationship between `Stable node id (lq3- FNV-1a 64)` and `Scenario tests (replace golden parity tests)`?**
  _Edge tagged AMBIGUOUS (relation: conceptually_related_to) - confidence is low._