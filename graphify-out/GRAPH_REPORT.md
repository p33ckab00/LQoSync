# Graph Report - LQoSync3.0  (2026-11-01)

## Corpus Check
- 85 files · ~108,885 words
- Verdict: corpus is large enough that graph structure adds value.
- Unclassified: 4 file(s) not represented in the graph (top: (none) 2, .toml 1, .service 1)

## Summary
- 1869 nodes · 4818 edges · 73 communities (64 shown, 9 thin omitted)
- Extraction: 96% EXTRACTED · 4% INFERRED · 0% AMBIGUOUS · INFERRED: 179 edges (avg confidence: 0.87)
- Token cost: 0 input · 0 output

## Graph Freshness
- Built from commit: `36e0b3ef`
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
- Status
- draft.rs
- .on_key
- ADR-0009 DHCP discovery and exclusions
- meta.rs
- Configuration reference
- apply.rs
- runner::Runner::cycle (one full cycle)
- preview.rs
- MikroTik Session 5
- lqosync3 core design spec
- RuntimeMode
- RouterCfg
- Review Focus
- Preview
- Configuration reference
- runner::Runner::cycle (one full cycle)
- dhcp_server_gives_override_or_
- routeros.rs
- lqosync3 LibreQoS v3 targets (Plan 3)
- Review Focus
- RouterResult
- lqosync3 core design spec
- data.rs
- journal.rs
- a_rejected_apply_says_the_new_file_is_on_disk
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
- cli.rs
- CsvRow
- RouterAccess
- status.rs
- detect.rs
- ADR-0008 Terminal UI
- PppSource
- LogLevel
- CsvRow
- Input
- Review Focus
- LeaseResult
- VModal
- Stage

## God Nodes (most connected - your core abstractions)
1. `Config` - 89 edges
2. `SettingsView` - 71 edges
3. `CsvRow` - 48 edges
4. `key()` - 35 edges
5. `run()` - 32 edges
6. `Draft` - 32 edges
7. `lqosync3 core design spec` - 32 edges
8. `App` - 31 edges
9. `Configuration reference` - 31 edges
10. `Handoff` - 28 edges

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

## Communities (73 total, 9 thin omitted)

### Community 0 - "mock_router.rs"
Cohesion: 0.17
Nodes (25): read_router(), Answer, Empty, Raw, Rows, Silent, Trap, closed_port_is_unreachable() (+17 more)

### Community 1 - "Telegram Notifier"
Cohesion: 0.06
Nodes (41): Task 3: Transport (post_json, send, token, chat_id), Telegram, AnyCertificate, a_config_error_is_an_error_event(), a_long_multi_line_error_splits_into_valid_html_parts(), a_very_long_error_is_cut(), an_empty_token_is_an_error(), an_error_is_sent_once_then_cleared() (+33 more)

### Community 2 - "runner.rs"
Cohesion: 0.25
Nodes (27): a_config_reload_error_shows_in_status_until_a_good_reload(), a_held_lock_stops_the_cycle_without_touching_status(), a_panicking_router_task_fails_only_that_router(), apply_backs_up_writes_and_reloads(), assemble_error_writes_nothing(), auto_on_v2_gives_the_v2_pair(), cfg(), Env (+19 more)

### Community 3 - "Topology"
Cohesion: 0.11
Nodes (45): NodeTemplate, StoredNode, GroupKey, DhcpPlan, DhcpSite, Hotspot, Ppp, PppPlan (+37 more)

### Community 4 - "sources.rs"
Cohesion: 0.16
Nodes (34): a_zero_speed_profile_comment_gives_one_line(), an_override_changes_only_that_client(), an_override_for_a_missing_mac_has_no_effect(), an_override_in_per_plan_moves_to_its_plan_node(), an_override_on_a_skipped_lease_gives_no_circuit(), by_name(), cfg(), cfg_text() (+26 more)

### Community 5 - "view.rs"
Cohesion: 0.12
Nodes (24): KeyPath, a_broken_config_file_shows_the_error(), a_skipped_lease_cannot_get_an_override(), an_edit_of_a_discovered_server_makes_its_entry(), an_override_without_a_lease_shows_no_active_lease(), DFILE, discovered_servers_and_leases_are_listed(), dsnap() (+16 more)

### Community 6 - "devices.rs"
Cohesion: 0.06
Nodes (40): Data, Column, .ALL, Down, Ipv4, Name, Parent, Source (+32 more)

### Community 7 - "cycle.rs"
Cohesion: 0.10
Nodes (54): a_row_that_stays_keeps_its_group_node(), AAA, an_override_gives_the_same_rate_in_v2_pair_v3_pair_and_v3_network(), assemble(), cfg(), changed_columns(), dhcp_cfg(), duplicate_ip_existing_vs_existing_keeps_collected() (+46 more)

### Community 8 - "files.rs"
Cohesion: 0.07
Nodes (35): Network, NetworkMode, FlatGlobalRoot, FlatPromotion, FlatRouterRoot, GroupedRouterRoot, NoParent, RootPromotion (+27 more)

### Community 9 - "SettingsView"
Cohesion: 0.10
Nodes (7): path_text(), concrete(), Note, key_path(), SettingsView, show(), Shown

### Community 10 - "Config"
Cohesion: 0.06
Nodes (41): a_bad_or_duplicate_mac_is_an_error(), a_circuit_comment_must_not_be_the_static_comment(), a_client_rate_below_the_floor_is_an_error(), BackstopCap, BackstopCaps, ClientTemplate, Config, deep_merge() (+33 more)

### Community 11 - "Status"
Cohesion: 0.12
Nodes (15): target_name(), random_id(), random_ids_have_the_length_and_charset(), Runner, Runner<C>, state_name(), write_snapshot(), RouterStatus (+7 more)

### Community 12 - "draft.rs"
Cohesion: 0.08
Nodes (32): a_bad_router_name_is_rejected(), a_change_keeps_comments_and_key_order(), add_and_remove_a_dhcp_server(), add_and_remove_a_router(), changed_counts_the_keys(), diff_lines_show_old_and_new(), Draft, FILE (+24 more)

### Community 13 - ".on_key"
Cohesion: 0.20
Nodes (32): a_bad_number_shows_the_key_error(), a_changed_file_shows_the_disk_diff_and_can_restart(), a_failed_preview_blocks_apply(), a_failed_preview_shows_that_apply_is_blocked(), a_secret_for_a_removed_router_is_dropped(), a_typed_password_is_never_in_the_buffer(), add_a_router_asks_for_a_masked_password(), apply_asks_for_a_confirmation() (+24 more)

### Community 14 - "ADR-0009 DHCP discovery and exclusions"
Cohesion: 0.25
Nodes (6): ADR-0009 DHCP discovery and exclusions, Speed Rules, Task 2: Rate format and speed rules (SpeedParser), Task 4: Circuit collection (collect_router), Circuit rules for each source, DhcpCfg::server(name) defaults

### Community 15 - "meta.rs"
Cohesion: 0.07
Nodes (41): DEFAULTS_TOML, all(), Base, Float, List, Text, build(), CHOICES (+33 more)

### Community 16 - "Configuration reference"
Cohesion: 0.12
Nodes (17): Plan 2b deferred minor items, Telegram settings, Command reference (check, once, diff, run, defaults), status.json cycle status, Turn on Telegram procedure, cli::parse / cli::Command, status::Status / RouterStatus, Plan 2b: lqosync3 Telegram implementation (+9 more)

### Community 17 - "apply.rs"
Cohesion: 0.08
Nodes (39): config_digest(), config_digest_is_stable_and_16_hex(), a_changed_file_on_disk_stops_the_apply(), a_toml_write_error_rolls_back_the_secrets(), apply(), apply_writes_a_backup_and_keeps_n(), ApplyError, Changed (+31 more)

### Community 18 - "runner::Runner::cycle (one full cycle)"
Cohesion: 0.12
Nodes (19): Applier (hash, atomic write, reload), Circuit Model, Collector (RouterOS API reads), Config Loader, Proposed Crate Layout, libreqos_network.json, lqosd (LibreQoS daemon), MikroTik RouterOS API (+11 more)

### Community 19 - "preview.rs"
Cohesion: 0.17
Nodes (29): cycle_inputs(), Inputs, a_failed_preview_is_failed_not_zero(), a_new_router_effect_is_not_known(), a_rate_change_gives_the_expected_circuit_changes(), a_router_that_was_not_online_gives_a_warning(), a_source_turned_on_is_not_known(), ALICE (+21 more)

### Community 20 - "MikroTik Session 5"
Cohesion: 0.16
Nodes (14): lqosync3 runtime design (Plan 2), Atomic write (temp file, sync_all, rename, fsync folder; only on byte change), Files: lock, state.json, status.json, candidate, applied, First-run seed (seed_state, seed_nodes from live files), Instance lock <state>/lqosync3.lock (File::try_lock), mikrotik-tokio 0.1.0 16-reply burst cancel bug, Plan 2 scope (in / not in), Process: run loop, SIGUSR1, SIGHUP, SIGTERM/SIGINT, once, check, diff, defaults (+6 more)

### Community 21 - "lqosync3 core design spec"
Cohesion: 0.12
Nodes (40): 2026-10-31 Plan 1: lqosync3 core library session, lqosync3 core crate (config, speed, model, state, sources, topology, cycle, validate, writer), Plan 1 review fixes (commit 529fa36), Session 2026-10-31 project setup and v3 simulation, Session: updatecsv.py dissection, ADR-0003 Router failed vs offline, ADR-0004 TOML policy, Python defaults, ADR-0005 Topology follows v3 (+32 more)

### Community 22 - "RuntimeMode"
Cohesion: 0.33
Nodes (5): Runtime, RuntimeMode, Apply, Shadow, CycleSummary

### Community 23 - "RouterCfg"
Cohesion: 0.23
Nodes (18): RouterCfg, RouterSnapshot, collect_dhcp(), collect_hotspot(), collect_ppp(), collect_router(), Collected, comment_rate() (+10 more)

### Community 24 - "Review Focus"
Cohesion: 0.17
Nodes (11): Global Constraints, lqosync3 TUI Implementation Plan (Plan 4b: the Settings editor), Review Focus, Review rulings (fact-check of 2026-11-01), Task 1: Key metadata, Task 2: The draft (edit model), Task 3: Validate and secrets, Task 4: Preview (+3 more)

### Community 25 - "Preview"
Cohesion: 0.33
Nodes (5): Preview, Ran, Skipped, PreviewDone, ran_with()

### Community 26 - "Configuration reference"
Cohesion: 0.08
Nodes (18): 2026-10-31 First live shadow run, FSEHub-Distro router (10.200.1.2:8728), Shadow run results (62 rows, 167 ms, 26 text-only updates), config/defaults.toml embedded defaults, Production state (Python live, lqosync3 shadow, LibreQoS 2.2), Configuration reference, Comment speed format (Name|25M/5M), Map from config.json (+10 more)

### Community 30 - "runner::Runner::cycle (one full cycle)"
Cohesion: 0.07
Nodes (30): network.json, Router state (online/failed/unreachable/offline), lqosync3.service systemd unit, Signals (SIGHUP reload, USR1 cycle now), routerstate::classify, Core design spec (2026-10-31-lqosync3-core-design.md), cycle::assemble (pure core), files::backup (keep_backups) (+22 more)

### Community 31 - "dhcp_server_gives_override_or_"
Cohesion: 0.21
Nodes (8): BandwidthDefaults, DhcpCfg, DhcpClient, DhcpMode, PerPlan, PerSite, DhcpServer, HotspotCfg

### Community 32 - "routeros.rs"
Cohesion: 0.08
Nodes (29): RosRow, a_dead_kept_session_reconnects_without_a_retry(), a_login_error_is_not_retried(), Connector, Fake, FakeSession, gives_last_error_after_all_retries(), ppp_router() (+21 more)

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
Cohesion: 0.17
Nodes (15): RouterOS listen change detection (future), Six Network Modes, Topology Builder, LibreQoS v3 topology.compile_mode, Subscriber presence and subscriber_grace, Plan 1 review focus (edge cases), Task 3: Model, state file, and seed, Task 5: Topology (build_nodes, to_network_json) (+7 more)

### Community 38 - "data.rs"
Cohesion: 0.06
Nodes (45): exit_with(), main(), notify(), now(), print_summary(), run(), SECRET_OWNER_UID, a_bad_network_json_keeps_the_last_data_and_shows_an_error() (+37 more)

### Community 39 - "journal.rs"
Cohesion: 0.09
Nodes (19): buffer_keeps_the_last_n_lines(), journal_start_error_is_shown(), journal_stderr_is_shown(), journal_tab_renders(), JournalHandle, JournalMsg, Error, Line (+11 more)

### Community 40 - "a_rejected_apply_says_the_new_file_is_on_disk"
Cohesion: 0.31
Nodes (4): a_rejected_apply_says_the_new_file_is_on_disk(), Disk, disk(), the_header_says_loaded_while_the_first_cycle_runs()

### Community 41 - "speed.rs"
Cohesion: 0.10
Nodes (22): UnitlessUnit, Bps, Mbps, plan_label(), circuit_rates(), CircuitRates, clamp(), default_rate_and_circuit_rates() (+14 more)

### Community 42 - "dashboard.rs"
Cohesion: 0.07
Nodes (26): narrow_terminal_puts_the_side_panel_under_the_main_panel(), split_main(), dashboard_tab_renders(), DashboardView, data(), digest_check_shows_not_loaded(), Kind, Reload (+18 more)

### Community 43 - "app.rs"
Cohesion: 0.19
Nodes (23): a_click_on_a_tab_title_selects_it(), a_key_press_clears_the_message_so_errors_show(), a_too_large_copy_is_not_sent(), action_result_comes_from_status(), an_open_field_takes_q_and_digits(), app(), app_dialogs_show_their_keys_and_only_y_n_esc_act(), draw() (+15 more)

### Community 45 - "v3network.rs"
Cohesion: 0.11
Nodes (30): diff_config(), diff_dirs(), diff_dirs_needs_candidate_files(), diff_ignores_column_order_row_order_and_number_text(), diff_pair(), diff_reports_added_removed_changed(), diff_stops_for_v3_network(), diff_stops_in_apply_mode() (+22 more)

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
Nodes (28): Orphan branch lqosync3, RouterOS listen command (untested), 14/15 CSV columns (v2_pair/v3_pair), libreqos_network.json schema_version 2, Plan phases (Plan 1/2/2b done; Plan 3 v3, Plan 4 TUI, Plan 5 installer), Runtime modes (shadow, apply), install.sh one-line installer (target contract), Installation (docs) (+20 more)

### Community 56 - "Review Focus"
Cohesion: 0.15
Nodes (12): Global Constraints, lqosync3 TUI Implementation Plan (Plan 4a: service changes and read-only screens), Review Focus, Review rulings (fact-check of 2026-10-31), Task 1: Service data for the TUI, Task 2: TUI frame, Task 3: Loader worker, Task 4: ShapedDevices tab (+4 more)

### Community 57 - ".draw"
Cohesion: 0.11
Nodes (14): App, center(), dialog(), Modal, ConfirmSync, Discard, modal_keys(), Tab (+6 more)

### Community 58 - "mikrotik.rs"
Cohesion: 0.17
Nodes (5): map_err(), MikrotikConnector, MikrotikSession, tls_client_config(), Setup

### Community 59 - "Action"
Cohesion: 0.19
Nodes (9): Action, ApplySettings, Copy, None, Preview, Quit, Reload, SelectMode (+1 more)

### Community 60 - "ADR-0009 DHCP discovery and exclusions"
Cohesion: 0.17
Nodes (10): 1. Intent, 2. Goals, non-goals, anti-goals, 3. Approaches, 4. Config, 5. Cycle (`sources.rs`), 6. TUI (Settings tab), 7. Output (LibreQoS 2.2 and 3.x), 8. Tests (+2 more)

### Community 62 - "cli.rs"
Cohesion: 0.05
Nodes (50): Args, check(), check_finds_plain_api_and_no_source_router(), check_reads_the_telegram_token_when_enabled(), check_reports_a_bad_secret_and_a_missing_lqos_dir(), check_shows_the_target_or_the_detection_error(), check_warns_when_the_file_is_not_loaded(), check_with() (+42 more)

### Community 63 - "CsvRow"
Cohesion: 0.08
Nodes (39): CircuitNames, CycleInput, CycleOutput, Entry, names(), node_rate(), Live, load_state() (+31 more)

### Community 64 - "RouterAccess"
Cohesion: 0.50
Nodes (11): RouterAccess, RouterMemo, acc(), classify(), first_unreachable_sets_since(), login_tls_trap_timeout_are_failed_and_keep_memo(), memo(), offline_after_zero_is_offline_at_once() (+3 more)

### Community 65 - "status.rs"
Cohesion: 0.29
Nodes (8): Change, Report, updated_line(), CircuitInfo, ReportSummary, status_json_has_the_spec_fields(), summary_takes_values_from_rows_and_the_old_state(), UpdatedInfo

### Community 66 - "detect.rs"
Cohesion: 0.18
Nodes (18): Libreqos, Target, Auto, V2Pair, V3Network, V3Pair, Detected, Env (+10 more)

### Community 67 - "ADR-0008 Terminal UI"
Cohesion: 0.16
Nodes (11): offline_action (default remove), offline_after (default 300 s), Speed source order TOML lists, ADR-0008 Terminal UI, TUI config editor (toml_edit), SIGUSR1 sync now / SIGHUP reload, status.json state file, updatecsv.py defects to fix (+3 more)

### Community 69 - "PppSource"
Cohesion: 0.13
Nodes (15): DhcpSource, LeaseComment, ServerComment, Settings, PppSource, ActiveComment, Default, ProfileComment (+7 more)

### Community 71 - "LogLevel"
Cohesion: 0.40
Nodes (5): LogLevel, Debug, Error, Info, Warn

### Community 73 - "CsvRow"
Cohesion: 0.21
Nodes (6): Columns, V2, V3, csv_header_sort_and_quoting(), render_csv(), row()

### Community 74 - "Input"
Cohesion: 0.10
Nodes (14): valid_router_name(), Input, AddRouter, AddServer, Client, Dropdown, Field, List (+6 more)

### Community 75 - "Review Focus"
Cohesion: 0.22
Nodes (8): Global Constraints, lqosync3 DHCP Client Overrides Implementation Plan (Plan 4c), Review Focus, Review rulings (fact-check of 2026-11-01), Task 1: Config, Task 2: The shared lease list and the cycle, Task 3: The DHCP view of the Settings tab, Task 4: Docs, review, graph

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
- `RouterOS listen change detection (future)` → `Collector (RouterOS API reads)`  [AMBIGUOUS]
  docs/ARCHITECTURE.md · relation: conceptually_related_to
- `Scenario tests (replace golden parity tests)` → `Stable node id (lq3- FNV-1a 64)`  [AMBIGUOUS]
  brain/20-decisions/ADR-0007-fresh-design-not-a-port.md · relation: conceptually_related_to

## Knowledge Gaps
- **169 isolated node(s):** `lqosync3`, `Check`, `Once`, `Run`, `Diff` (+164 more)
  These have ≤1 connection - possible missing edges or undocumented components. (Counts symbols only; 442 node(s) total have ≤1 connection when file, concept and rationale nodes are included.)
- **9 thin communities (<3 nodes) omitted from report** — run `graphify query` to explore isolated nodes.

## Suggested Questions
_Questions this graph is uniquely positioned to answer:_

- **Why does `Task 3: Transport (post_json, send, token, chat_id)` connect `Telegram Notifier` to `Configuration reference`?**
  _High betweenness centrality (0.173) - this node is a cross-community bridge._
- **Are the 3 inferred relationships involving `Config` (e.g. with `diff_config()` and `action_timeout()`) actually correct?**
  _`Config` has 3 INFERRED edges - model-reasoned connections that need verification._
- **What connects `lqosync3`, `Check`, `Once` to the rest of the system?**
  _169 weakly-connected nodes found - possible documentation gaps or missing edges._
- **Should `Telegram Notifier` be split into smaller, more focused modules?**
  _Cohesion score 0.06331976481230213 - nodes in this community are weakly interconnected._
- **Why does `Config` connect `Config` to `mock_router.rs`, `Telegram Notifier`, `runner.rs`, `Topology`, `sources.rs`, `cycle.rs`, `files.rs`, `Status`, `draft.rs`, `meta.rs`, `preview.rs`, `RuntimeMode`, `RouterCfg`, `dhcp_server_gives_override_or_`, `routeros.rs`, `data.rs`, `speed.rs`, `v3network.rs`, `tui/mod.rs`, `cli.rs`, `CsvRow`, `RouterAccess`, `detect.rs`, `PppSource`?**
  _High betweenness centrality (0.131) - this node is a cross-community bridge._

### Low-confidence Hints
_AMBIGUOUS edges — the extractor was unsure. Verify before acting on these._

- **What is the exact relationship between `RouterOS listen change detection (future)` and `Collector (RouterOS API reads)`?**
  _Edge tagged AMBIGUOUS (relation: conceptually_related_to) - confidence is low._
- **What is the exact relationship between `Scenario tests (replace golden parity tests)` and `Stable node id (lq3- FNV-1a 64)`?**
  _Edge tagged AMBIGUOUS (relation: conceptually_related_to) - confidence is low._