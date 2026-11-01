# Graph Report - LQoSync3.0  (2026-11-01)

## Corpus Check
- 83 files · ~99,352 words
- Verdict: corpus is large enough that graph structure adds value.
- Unclassified: 4 file(s) not represented in the graph (top: (none) 2, .toml 1, .service 1)

## Summary
- 1760 nodes · 4523 edges · 74 communities (61 shown, 13 thin omitted)
- Extraction: 96% EXTRACTED · 4% INFERRED · 0% AMBIGUOUS · INFERRED: 174 edges (avg confidence: 0.87)
- Token cost: 0 input · 0 output

## Graph Freshness
- Built from commit: `3146d31f`
- Run `git rev-parse HEAD` and compare to check if the graph is stale.
- Run `graphify update .` after code changes (no API cost).

## Community Hubs (Navigation)
- mock_router.rs
- Telegram Notifier
- Router State
- Topology
- sources.rs
- cli.rs
- devices.rs
- cycle.rs
- File Safety
- view.rs
- Config
- hashmap
- draft.rs
- ADR-0005 Topology follows v3
- Source
- meta.rs
- Configuration reference
- apply.rs
- runner::Runner::cycle (one full cycle)
- preview.rs
- MikroTik Session 5
- Speed Rules 2
- scenarios.rs
- RouterCfg
- Review Focus
- CertificateDer
- Handoff
- Speed
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
- App
- mikrotik.rs
- Action
- app.rs
- osc52.rs
- Level
- src/secrets.rs
- RouterAccess
- status.rs
- settings/validate.rs
- Sent
- Command
- PppSource
- Tab
- LogLevel
- OfflineAction
- Env

## God Nodes (most connected - your core abstractions)
1. `Config` - 84 edges
2. `SettingsView` - 58 edges
3. `CsvRow` - 48 edges
4. `lqosync3 core design spec` - 32 edges
5. `run()` - 31 edges
6. `App` - 31 edges
7. `Configuration reference` - 31 edges
8. `Draft` - 29 edges
9. `Handoff` - 28 edges
10. `runner()` - 27 edges

## Surprising Connections (you probably didn't know these)
- `v3 offline simulation method` --semantically_similar_to--> `/run/lqos/bus 0777 safety caution`  [INFERRED] [semantically similar]
  brain/30-knowledge/v3-offline-simulation-method.md → docs/LIBREQOS-V3-INGRESS.md
- `v3 offline simulation method` --semantically_similar_to--> `Offline v3 simulation`  [INFERRED] [semantically similar]
  brain/30-knowledge/v3-offline-simulation-method.md → docs/LIBREQOS-V3-INGRESS.md
- `libreqos_network.json schema_version 2` --references--> `v3_network output target (libreqos_network.json)`  [INFERRED]
  brain/30-knowledge/libreqos-v3-ingress-facts.md → README.md
- `Safety rules (shadow default, no router writes, 0600 secrets)` --semantically_similar_to--> `Operational rules`  [INFERRED] [semantically similar]
  README.md → docs/OPERATIONS.md
- `Task 2: Events and message` --references--> `split()`  [EXTRACTED]
  docs/superpowers/plans/2026-10-31-lqosync3-telegram.md → src/notify.rs

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

## Communities (74 total, 13 thin omitted)

### Community 0 - "mock_router.rs"
Cohesion: 0.15
Nodes (25): read_router(), Answer, Empty, Raw, Rows, Silent, Trap, closed_port_is_unreachable() (+17 more)

### Community 1 - "Telegram Notifier"
Cohesion: 0.10
Nodes (37): Task 2: Events and message, Task 3: Transport (post_json, send, token, chat_id), Telegram, a_config_error_is_an_error_event(), a_long_multi_line_error_splits_into_valid_html_parts(), a_very_long_error_is_cut(), an_empty_token_is_an_error(), an_error_is_sent_once_then_cleared() (+29 more)

### Community 2 - "Router State"
Cohesion: 0.09
Nodes (51): target_name(), Runtime, RuntimeMode, Apply, Shadow, a_config_reload_error_shows_in_status_until_a_good_reload(), a_held_lock_stops_the_cycle_without_touching_status(), a_panicking_router_task_fails_only_that_router() (+43 more)

### Community 3 - "Topology"
Cohesion: 0.10
Nodes (48): NodeTemplate, StoredNode, GroupKey, DhcpPlan, DhcpSite, Hotspot, Ppp, PppPlan (+40 more)

### Community 4 - "sources.rs"
Cohesion: 0.21
Nodes (24): a_zero_speed_profile_comment_gives_one_line(), by_name(), cfg(), cfg_text(), circuit_templates_can_use_router(), dhcp_per_plan_uses_lease_comment(), dhcp_per_site_filters_and_names(), dhcp_server_comment_beats_settings() (+16 more)

### Community 5 - "cli.rs"
Cohesion: 0.21
Nodes (12): Args, check_warns_when_the_file_is_not_loaded(), config_digest_is_stable_and_16_hex(), DEFAULT_CONFIG, parse(), parse_commands_and_config_path(), read_config(), reload() (+4 more)

### Community 6 - "devices.rs"
Cohesion: 0.05
Nodes (44): Data, FLAG_DUP_IP, FLAG_NO_PARENT, FLAG_RATE, Versions, Column, .ALL, Down (+36 more)

### Community 7 - "cycle.rs"
Cohesion: 0.12
Nodes (47): a_row_that_stays_keeps_its_group_node(), cfg(), duplicate_ip_existing_vs_existing_keeps_collected(), duplicate_ip_new_vs_existing_keeps_existing(), empty_online_router_removes_its_rows(), failed_and_unreachable_router_keep_rows_and_nodes(), first_forty(), flat_promotion_has_no_client_node_for_a_static_row() (+39 more)

### Community 8 - "File Safety"
Cohesion: 0.09
Nodes (26): CircuitNames, backup(), backup_keeps_the_last_n(), backup_keeps_the_new_folder_and_other_folders(), HDR, Live, load_state(), load_state_seeds_on_first_run_and_reads_after() (+18 more)

### Community 9 - "view.rs"
Cohesion: 0.05
Nodes (69): KeyPath, line_diff(), path_text(), a_bad_number_shows_the_key_error(), a_broken_config_file_shows_the_error(), a_changed_file_shows_the_disk_diff_and_can_restart(), a_failed_preview_blocks_apply(), a_failed_preview_shows_that_apply_is_blocked() (+61 more)

### Community 10 - "Config"
Cohesion: 0.10
Nodes (22): a_circuit_comment_must_not_be_the_static_comment(), ClientTemplate, defaults_table(), dhcp_server_gives_override_or_default(), exclusions_parse_and_bad_ip_is_rejected(), default_from_file!, MIN, minimal_config_gets_documented_defaults() (+14 more)

### Community 11 - "hashmap"
Cohesion: 0.16
Nodes (22): Network, NetworkMode, FlatGlobalRoot, FlatPromotion, FlatRouterRoot, GroupedRouterRoot, NoParent, RootPromotion (+14 more)

### Community 12 - "draft.rs"
Cohesion: 0.08
Nodes (29): a_bad_router_name_is_rejected(), a_change_keeps_comments_and_key_order(), add_and_remove_a_dhcp_server(), add_and_remove_a_router(), changed_counts_the_keys(), diff_lines_show_old_and_new(), Draft, FILE (+21 more)

### Community 13 - "ADR-0005 Topology follows v3"
Cohesion: 0.22
Nodes (13): ADR-0005 Topology follows v3, 14/15 CSV columns (v2_pair/v3_pair), max_floor_mbps (0.2) / safe_min_mbps (0.1), Parent Node ID column, ADR-0006 v3 guards and node types, compile_mode not full warning, Tool must reject duplicate IP and node name, How LibreQoS v3 handles nodes (+5 more)

### Community 14 - "Source"
Cohesion: 0.33
Nodes (6): Entry, Source, Dhcp, Hotspot, Ppp, Unknown

### Community 15 - "meta.rs"
Cohesion: 0.07
Nodes (41): DEFAULTS_TOML, all(), Base, Float, List, Text, build(), CHOICES (+33 more)

### Community 16 - "Configuration reference"
Cohesion: 0.06
Nodes (30): Plan 2b deferred minor items, RouterOS listen change detection (future), Configuration reference, Comment speed format (Name|25M/5M), Map from config.json, DHCP server discovery, Exclusions ([exclude], [routers.exclude]), lqosync3 import-json (planned) (+22 more)

### Community 17 - "apply.rs"
Cohesion: 0.08
Nodes (34): config_digest(), a_changed_file_on_disk_stops_the_apply(), a_toml_write_error_rolls_back_the_secrets(), apply(), apply_writes_a_backup_and_keeps_n(), ApplyError, Changed, Io (+26 more)

### Community 18 - "runner::Runner::cycle (one full cycle)"
Cohesion: 0.13
Nodes (18): Applier (hash, atomic write, reload), Circuit Model, Collector (RouterOS API reads), Config Loader, Proposed Crate Layout, libreqos_network.json, lqosd (LibreQoS daemon), MikroTik RouterOS API (+10 more)

### Community 19 - "preview.rs"
Cohesion: 0.09
Nodes (46): Libreqos, Target, Auto, V2Pair, V3Network, V3Pair, Detected, Env (+38 more)

### Community 20 - "MikroTik Session 5"
Cohesion: 0.16
Nodes (14): lqosync3 runtime design (Plan 2), Atomic write (temp file, sync_all, rename, fsync folder; only on byte change), Files: lock, state.json, status.json, candidate, applied, First-run seed (seed_state, seed_nodes from live files), Instance lock <state>/lqosync3.lock (File::try_lock), mikrotik-tokio 0.1.0 16-reply burst cancel bug, Plan 2 scope (in / not in), Process: run loop, SIGUSR1, SIGHUP, SIGTERM/SIGINT, once, check, diff, defaults (+6 more)

### Community 21 - "Speed Rules 2"
Cohesion: 0.11
Nodes (31): 2026-10-31 Plan 1: lqosync3 core library session, lqosync3 core crate (config, speed, model, state, sources, topology, cycle, validate, writer), Plan 1 review fixes (commit 529fa36), Session: updatecsv.py dissection, ADR-0003 Router failed vs offline, offline_action (default remove), offline_after (default 300 s), ADR-0004 TOML policy, Python defaults (+23 more)

### Community 22 - "scenarios.rs"
Cohesion: 0.10
Nodes (28): assemble(), changed_columns(), CycleInput, CycleOutput, names(), Columns, V2, V3 (+20 more)

### Community 23 - "RouterCfg"
Cohesion: 0.19
Nodes (18): parse_ipv4_net(), RouterCfg, RouterSnapshot, read_snapshot(), collect_dhcp(), collect_hotspot(), collect_ppp(), collect_router() (+10 more)

### Community 24 - "Review Focus"
Cohesion: 0.17
Nodes (11): Global Constraints, lqosync3 TUI Implementation Plan (Plan 4b: the Settings editor), Review Focus, Review rulings (fact-check of 2026-11-01), Task 1: Key metadata, Task 2: The draft (edit model), Task 3: Validate and secrets, Task 4: Preview (+3 more)

### Community 26 - "Handoff"
Cohesion: 0.17
Nodes (8): 2026-10-31 First live shadow run, FSEHub-Distro router (10.200.1.2:8728), Shadow run results (62 rows, 167 ms, 26 text-only updates), Handoff, config/defaults.toml embedded defaults, Production state (Python live, lqosync3 shadow, LibreQoS 2.2), Differences from updatecsv.py, Task 1: Crate scaffold and config

### Community 28 - "Speed"
Cohesion: 0.25
Nodes (8): DhcpSource, LeaseComment, ServerComment, Settings, RateLimitOrder, DownloadUpload, RouterosRxTx, Speed

### Community 29 - "Config"
Cohesion: 0.14
Nodes (8): BackstopCap, BackstopCaps, Config, deep_merge(), defaults(), Exclude, Safety, Tui

### Community 30 - "runner::Runner::cycle (one full cycle)"
Cohesion: 0.08
Nodes (25): network.json, lqosync3.service systemd unit, Signals (SIGHUP reload, USR1 cycle now), Core design spec (2026-10-31-lqosync3-core-design.md), cycle::assemble (pure core), files::backup (keep_backups), files::lock (lqosync3.lock), files::load_state (state.json or seed) (+17 more)

### Community 31 - "dhcp_server_gives_override_or_"
Cohesion: 0.23
Nodes (7): BandwidthDefaults, DhcpCfg, DhcpMode, PerPlan, PerSite, DhcpServer, HotspotCfg

### Community 32 - "routeros.rs"
Cohesion: 0.10
Nodes (24): RosRow, a_dead_kept_session_reconnects_without_a_retry(), a_login_error_is_not_retried(), Connector, Fake, FakeSession, gives_last_error_after_all_retries(), ppp_router() (+16 more)

### Community 33 - "lqosync3 LibreQoS v3 targets (Plan 3)"
Cohesion: 0.22
Nodes (9): CLI, Config changes, lqosync3 LibreQoS v3 targets (Plan 3), Offline check result (2026-10-31), Out of scope, Purpose, Target selection (ADR-0002), `v3_network` (+1 more)

### Community 34 - "Review Focus"
Cohesion: 0.25
Nodes (7): Global Constraints, lqosync3 LibreQoS v3 Implementation Plan (Plan 3), Review Focus, Task 1: Detector (`src/detect.rs`), Task 2: `v3_network` writer (`src/writer/v3network.rs`), Task 3: Runner, CLI, and check, Task 4: Offline check with the v3 parser

### Community 35 - "RouterResult"
Cohesion: 0.28
Nodes (9): ppp(), ppp_owned(), RouterResult, Failed, Offline, Online, Unreachable, name() (+1 more)

### Community 36 - "lqosync3 core design spec"
Cohesion: 0.11
Nodes (25): ADR-0009 DHCP discovery and exclusions, Six Network Modes, Speed Rules, Topology Builder, LibreQoS v3 topology.compile_mode, Plan 1 review focus (edge cases), Task 2: Rate format and speed rules (SpeedParser), Task 3: Model, state file, and seed (+17 more)

### Community 38 - "data.rs"
Cohesion: 0.07
Nodes (40): exit_with(), main(), notify(), now(), print_summary(), run(), SECRET_OWNER_UID, a_bad_network_json_keeps_the_last_data_and_shows_an_error() (+32 more)

### Community 39 - "journal.rs"
Cohesion: 0.10
Nodes (19): buffer_keeps_the_last_n_lines(), journal_start_error_is_shown(), journal_stderr_is_shown(), journal_tab_renders(), JournalHandle, JournalMsg, Error, Line (+11 more)

### Community 40 - "diff.rs"
Cohesion: 0.21
Nodes (14): diff_config(), diff_dirs(), diff_dirs_needs_candidate_files(), diff_ignores_column_order_row_order_and_number_text(), diff_pair(), diff_reports_added_removed_changed(), diff_stops_for_v3_network(), diff_stops_in_apply_mode() (+6 more)

### Community 41 - "speed.rs"
Cohesion: 0.11
Nodes (19): UnitlessUnit, Bps, Mbps, circuit_rates(), CircuitRates, clamp(), default_rate_and_circuit_rates(), finite() (+11 more)

### Community 42 - "dashboard.rs"
Cohesion: 0.10
Nodes (17): dashboard_tab_renders(), DashboardView, data(), digest_check_shows_not_loaded(), Kind, Reload, SyncNow, lines() (+9 more)

### Community 43 - "app.rs"
Cohesion: 0.21
Nodes (17): a_click_on_a_tab_title_selects_it(), a_key_press_clears_the_message_so_errors_show(), an_open_field_takes_q_and_digits(), app(), app_dialogs_show_their_keys_and_only_y_n_esc_act(), draw(), draws_on_a_40x10_terminal_without_a_panic(), enter_opens_the_parent_node_in_the_network_tab() (+9 more)

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
Cohesion: 0.16
Nodes (19): Session 2026-10-31 project setup and v3 simulation, LibreQoS v3 ingress facts, libreqos_network.json schema_version 2, topology_import.json (internal, do not write), [topology] source setting, v3 offline simulation method, LibreQoS v3 ingress document, Last-good topology retention (+11 more)

### Community 55 - "Operations guide"
Cohesion: 0.10
Nodes (26): Orphan branch lqosync3, RouterOS listen command (untested), Plan phases (Plan 1/2/2b done; Plan 3 v3, Plan 4 TUI, Plan 5 installer), Runtime modes (shadow, apply), install.sh one-line installer (target contract), Installation (docs), Installer options (--mode, --version, --from-source, --import-json, --no-start), What the installer does (10 steps) (+18 more)

### Community 56 - "Review Focus"
Cohesion: 0.15
Nodes (12): Global Constraints, lqosync3 TUI Implementation Plan (Plan 4a: service changes and read-only screens), Review Focus, Review rulings (fact-check of 2026-10-31), Task 1: Service data for the TUI, Task 2: TUI frame, Task 3: Loader worker, Task 4: ShapedDevices tab (+4 more)

### Community 58 - "mikrotik.rs"
Cohesion: 0.15
Nodes (5): map_err(), MikrotikConnector, MikrotikSession, tls_client_config(), Setup

### Community 59 - "Action"
Cohesion: 0.18
Nodes (9): Action, ApplySettings, Copy, None, Preview, Quit, Reload, SelectMode (+1 more)

### Community 60 - "app.rs"
Cohesion: 0.17
Nodes (12): a_too_large_copy_is_not_sent(), dialog(), journal_app(), keys_1_to_5_select_the_tabs_and_q_quits(), Modal, ConfirmSync, Discard, modal_keys() (+4 more)

### Community 62 - "Level"
Cohesion: 0.23
Nodes (15): check(), check_finds_plain_api_and_no_source_router(), check_reads_the_telegram_token_when_enabled(), check_reports_a_bad_secret_and_a_missing_lqos_dir(), check_shows_the_target_or_the_detection_error(), check_with(), CheckLine, digest_line() (+7 more)

### Community 63 - "src/secrets.rs"
Cohesion: 0.22
Nodes (7): error_never_contains_the_secret(), file(), missing_file_names_the_path(), read_secret(), reads_and_trims_a_good_secret(), rejects_group_or_other_bits(), rejects_wrong_owner()

### Community 64 - "RouterAccess"
Cohesion: 0.50
Nodes (11): RouterAccess, RouterMemo, acc(), classify(), first_unreachable_sets_since(), login_tls_trap_timeout_are_failed_and_keep_memo(), memo(), offline_after_zero_is_offline_at_once() (+3 more)

### Community 65 - "status.rs"
Cohesion: 0.29
Nodes (8): Change, Report, updated_line(), CircuitInfo, ReportSummary, status_json_has_the_spec_fields(), summary_takes_values_from_rows_and_the_old_state(), UpdatedInfo

### Community 66 - "settings/validate.rs"
Cohesion: 0.31
Nodes (7): Env, env(), parse(), validate(), validate_accepts_a_typed_password_file(), validate_names_the_key(), validate_returns_warnings()

### Community 67 - "Sent"
Cohesion: 0.20
Nodes (8): action_result_comes_from_status(), Sent, Wait, Done, Pending, Restarted, wait_result(), TimedOut

### Community 68 - "Command"
Cohesion: 0.25
Nodes (8): Command, Check, Defaults, Detect, Diff, Once, Run, Tui

### Community 69 - "PppSource"
Cohesion: 0.29
Nodes (7): PppSource, ActiveComment, Default, ProfileComment, ProfileName, ProfileRateLimit, SecretComment

### Community 70 - "Tab"
Cohesion: 0.29
Nodes (7): Tab, .ALL, Dashboard, Devices, Journal, Network, Settings

### Community 71 - "LogLevel"
Cohesion: 0.40
Nodes (5): LogLevel, Debug, Error, Info, Warn

### Community 72 - "OfflineAction"
Cohesion: 0.67
Nodes (3): OfflineAction, Keep, Remove

## Ambiguous Edges - Review These
- `Stable node id (lq3- FNV-1a 64)` → `Scenario tests (replace golden parity tests)`  [AMBIGUOUS]
  brain/20-decisions/ADR-0007-fresh-design-not-a-port.md · relation: conceptually_related_to
- `Collector (RouterOS API reads)` → `RouterOS listen change detection (future)`  [AMBIGUOUS]
  docs/ARCHITECTURE.md · relation: conceptually_related_to

## Knowledge Gaps
- **161 isolated node(s):** `lqosync3`, `Check`, `Once`, `Run`, `Diff` (+156 more)
  These have ≤1 connection - possible missing edges or undocumented components. (Counts symbols only; 409 node(s) total have ≤1 connection when file, concept and rationale nodes are included.)
- **13 thin communities (<3 nodes) omitted from report** — run `graphify query` to explore isolated nodes.

## Suggested Questions
_Questions this graph is uniquely positioned to answer:_

- **Why does `Task 3: Transport (post_json, send, token, chat_id)` connect `Telegram Notifier` to `Configuration reference`?**
  _High betweenness centrality (0.176) - this node is a cross-community bridge._
- **Are the 2 inferred relationships involving `Config` (e.g. with `diff_config()` and `fresh_pid()`) actually correct?**
  _`Config` has 2 INFERRED edges - model-reasoned connections that need verification._
- **What connects `lqosync3`, `Check`, `Once` to the rest of the system?**
  _161 weakly-connected nodes found - possible documentation gaps or missing edges._
- **Should `Telegram Notifier` be split into smaller, more focused modules?**
  _Cohesion score 0.1036077705827937 - nodes in this community are weakly interconnected._
- **Why does `Config` connect `Config` to `mock_router.rs`, `Telegram Notifier`, `Router State`, `Topology`, `sources.rs`, `cli.rs`, `cycle.rs`, `File Safety`, `Config`, `hashmap`, `draft.rs`, `meta.rs`, `preview.rs`, `scenarios.rs`, `RouterCfg`, `Speed`, `dhcp_server_gives_override_or_`, `routeros.rs`, `RouterResult`, `data.rs`, `diff.rs`, `tui/mod.rs`, `Level`, `RouterAccess`, `settings/validate.rs`?**
  _High betweenness centrality (0.119) - this node is a cross-community bridge._

### Low-confidence Hints
_AMBIGUOUS edges — the extractor was unsure. Verify before acting on these._

- **What is the exact relationship between `Stable node id (lq3- FNV-1a 64)` and `Scenario tests (replace golden parity tests)`?**
  _Edge tagged AMBIGUOUS (relation: conceptually_related_to) - confidence is low._
- **What is the exact relationship between `Collector (RouterOS API reads)` and `RouterOS listen change detection (future)`?**
  _Edge tagged AMBIGUOUS (relation: conceptually_related_to) - confidence is low._