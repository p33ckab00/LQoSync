# Graph Report - LQoSync3.0  (2026-10-31)

## Corpus Check
- 15 files · ~54,543 words
- Verdict: corpus is large enough that graph structure adds value.

## Summary
- 993 nodes · 2362 edges · 52 communities (43 shown, 9 thin omitted)
- Extraction: 95% EXTRACTED · 5% INFERRED · 0% AMBIGUOUS · INFERRED: 122 edges (avg confidence: 0.88)
- Token cost: 0 input · 0 output

## Community Hubs (Navigation)
- MikroTik Session
- Telegram Notifier
- Router State
- Topology
- Circuit Collection
- Telegram Notifier 2
- Secrets
- CLI and Checks
- File Safety
- fs
- Config
- hashmap
- Session Notes
- Topology 2
- Topology 3
- MikroTik Session 2
- Speed Rules
- Status and Diff
- MikroTik Session 3
- MikroTik Session 4
- MikroTik Session 5
- Speed Rules 2
- Topology 4
- deserialize
- Config 2
- CertificateDer
- Status and Diff 2
- Speed Rules 3
- Plan 2 Runtime
- Config 3
- LibreQoS v3
- dhcp_server_gives_override_or_
- Router State 2
- CLI and Checks 2
- Status and Diff 3
- Router State 3
- Topology 5
- Decisions
- Status and Diff 4
- Telegram Notifier 3
- LibreQoS v3 2
- LogLevel
- CLI and Checks 3
- OfflineAction
- State
- check.sh
- Telegram Notifier 4
- CLI and Checks 4
- lqosync3

## God Nodes (most connected - your core abstractions)
1. `Config` - 62 edges
2. `lqosync3 core design spec` - 32 edges
3. `run()` - 31 edges
4. `CsvRow` - 26 edges
5. `read_router()` - 26 edges
6. `ReadError` - 25 edges
7. `RouterCfg` - 25 edges
8. `Configuration reference` - 25 edges
9. `build_nodes()` - 21 edges
10. `Rate` - 20 edges

## Surprising Connections (you probably didn't know these)
- `v3 offline simulation method` --semantically_similar_to--> `/run/lqos/bus 0777 safety caution`  [INFERRED] [semantically similar]
  brain/30-knowledge/v3-offline-simulation-method.md → docs/LIBREQOS-V3-INGRESS.md
- `v3 offline simulation method` --semantically_similar_to--> `Offline v3 simulation`  [INFERRED] [semantically similar]
  brain/30-knowledge/v3-offline-simulation-method.md → docs/LIBREQOS-V3-INGRESS.md
- `libreqos_network.json schema_version 2` --references--> `v3_network output target (libreqos_network.json)`  [INFERRED]
  brain/30-knowledge/libreqos-v3-ingress-facts.md → README.md
- `Safety rules (shadow default, no router writes, 0600 secrets)` --semantically_similar_to--> `Operational rules`  [INFERRED] [semantically similar]
  README.md → docs/OPERATIONS.md
- `scenarios_match_expected_output()` --calls--> `read_csv()`  [INFERRED]
  tests/scenarios.rs → src/state.rs

## Import Cycles
- None detected.

## Hyperedges (group relationships)
- **Router-to-LibreQoS cycle data flow** — docs_architecture_collector, docs_architecture_circuit_model, docs_architecture_validator, docs_architecture_target_detector, docs_architecture_writers, docs_architecture_applier [EXTRACTED 1.00]
- **Plan 1 core tasks building assemble()** — docs_superpowers_plans_2026_10_31_lqosync3_core_task1_config, docs_superpowers_plans_2026_10_31_lqosync3_core_task2_speed, docs_superpowers_plans_2026_10_31_lqosync3_core_task3_state, docs_superpowers_plans_2026_10_31_lqosync3_core_task4_sources, docs_superpowers_plans_2026_10_31_lqosync3_core_task5_topology, docs_superpowers_plans_2026_10_31_lqosync3_core_task6_cycle, docs_superpowers_plans_2026_10_31_lqosync3_core_task7_writer, docs_superpowers_plans_2026_10_31_lqosync3_core_task8_scenarios [EXTRACTED 1.00]
- **Removal guard hold-and-release flow** — docs_configuration_removal_guard, docs_superpowers_specs_2026_10_31_lqosync3_core_design_cycle_assembly_rules, docs_superpowers_specs_2026_10_31_lqosync3_core_design_guard_fingerprint, docs_superpowers_specs_2026_10_31_lqosync3_core_design_state_file [EXTRACTED 1.00]
- **Router failure vs offline handling** — brain_20_decisions_adr_0003_router_failed_vs_offline_router_state_model, brain_20_decisions_adr_0003_router_failed_vs_offline_offline_after, brain_20_decisions_adr_0003_router_failed_vs_offline_offline_action, brain_30_knowledge_updatecsv_py_dissection_defects_to_fix [EXTRACTED 1.00]
- **Crash-safe file writes** — docs_superpowers_specs_2026_10_31_lqosync3_runtime_design_instance_lock, docs_superpowers_specs_2026_10_31_lqosync3_runtime_design_write_order_state_first, docs_superpowers_specs_2026_10_31_lqosync3_runtime_design_atomic_write, docs_superpowers_specs_2026_10_31_lqosync3_runtime_design_first_run_seed [INFERRED 0.85]
- **Router failed/unreachable/offline state model** — brain_handoff_adr_0003_router_failed_vs_offline, docs_configuration_router_state, docs_superpowers_plans_2026_10_31_lqosync3_runtime_classify, docs_superpowers_plans_2026_10_31_lqosync3_runtime_read_router, docs_superpowers_plans_2026_10_31_lqosync3_runtime_read_error [INFERRED 0.85]
- **Safe cycle write path (lock, atomic write, backup, reload)** — docs_superpowers_plans_2026_10_31_lqosync3_runtime_runner_cycle, docs_superpowers_plans_2026_10_31_lqosync3_runtime_files_lock, docs_superpowers_plans_2026_10_31_lqosync3_runtime_write_if_changed, docs_superpowers_plans_2026_10_31_lqosync3_runtime_files_backup, docs_superpowers_plans_2026_10_31_lqosync3_runtime_run_reload [INFERRED 0.85]
- **LibreQoS v3 node rule compliance** — brain_20_decisions_adr_0005_topology_follows_v3_stable_node_id, brain_20_decisions_adr_0005_topology_follows_v3_parent_node_id, brain_20_decisions_adr_0005_topology_follows_v3_csv_column_count, brain_20_decisions_adr_0005_topology_follows_v3_max_floor_mbps, brain_20_decisions_adr_0005_topology_follows_v3_flat_promotion_parent_fix, brain_20_decisions_adr_0006_v3_guards_and_node_types_no_parent_guard, brain_20_decisions_adr_0006_v3_guards_and_node_types_site_group_nodes, brain_20_decisions_adr_0006_v3_guards_and_node_types_compile_mode_warning [INFERRED 0.85]
- **DHCP discovery and subscriber exclusion** — brain_20_decisions_adr_0009_dhcp_discovery_and_exclusions_dhcp_server_discovery, brain_20_decisions_adr_0009_dhcp_discovery_and_exclusions_exclusions, docs_configuration_exclusions, docs_superpowers_specs_2026_10_31_lqosync3_core_design_circuit_rules [INFERRED 0.95]
- **Plan 2b Telegram notifier design-to-ops chain** — docs_superpowers_specs_2026_10_31_lqosync3_telegram_design, docs_superpowers_plans_2026_10_31_lqosync3_telegram, docs_configuration_telegram_settings, docs_operations_turn_on_telegram, brain_handoff_plan2b_deferred_items [INFERRED 0.85]
- **Shadow-to-apply cutover flow** — docs_configuration_runtime_modes, docs_operations_command_reference, docs_operations_cutover_procedure, docs_operations_rollback_procedure, brain_10_sessions_2026_10_31_first_live_shadow_run [INFERRED 0.85]
- **LibreQoS output targets selected by auto** — readme_v2_pair, readme_v3_pair, readme_v3_network, readme_auto_target_selection, docs_operations_libreqos_upgrade_2_to_3 [EXTRACTED 1.00]

## Communities (52 total, 9 thin omitted)

### Community 0 - "MikroTik Session"
Cohesion: 0.05
Nodes (57): RouterAccess, RouterCfg, map_err(), MikrotikConnector, MikrotikSession, tls_client_config(), RosRow, a_dead_kept_session_reconnects_without_a_retry() (+49 more)

### Community 1 - "Telegram Notifier"
Cohesion: 0.06
Nodes (50): Plan 2b deferred minor items, Telegram settings, Turn on Telegram procedure, Plan 2b: lqosync3 Telegram implementation, Task 1: Config keys, Task 2: Events and message, Task 3: Transport (post_json, send, token, chat_id), Task 4: Wire into run, once, check (+42 more)

### Community 2 - "Router State"
Cohesion: 0.10
Nodes (31): Runtime, RuntimeMode, Apply, Shadow, a_config_reload_error_shows_in_status_until_a_good_reload(), a_held_lock_stops_the_cycle_without_touching_status(), a_panicking_router_task_fails_only_that_router(), apply_backs_up_writes_and_reloads() (+23 more)

### Community 3 - "Topology"
Cohesion: 0.11
Nodes (48): StoredNode, GroupKey, DhcpPlan, DhcpSite, Hotspot, Ppp, PppPlan, Rate (+40 more)

### Community 4 - "Circuit Collection"
Cohesion: 0.12
Nodes (40): parse_ipv4_net(), a_zero_speed_profile_comment_gives_one_line(), by_name(), cfg(), cfg_text(), circuit_templates_can_use_router(), collect_dhcp(), collect_hotspot() (+32 more)

### Community 5 - "Telegram Notifier 2"
Cohesion: 0.07
Nodes (36): Args, check(), check_finds_plain_api_and_no_source_router(), check_reads_the_telegram_token_when_enabled(), check_rejects_v3_target_for_now(), check_reports_a_bad_secret_and_a_missing_lqos_dir(), CheckLine, Command (+28 more)

### Community 6 - "Secrets"
Cohesion: 0.08
Nodes (27): PppSource, ActiveComment, Default, ProfileComment, ProfileName, ProfileRateLimit, SecretComment, RateLimitOrder (+19 more)

### Community 7 - "CLI and Checks"
Cohesion: 0.16
Nodes (36): a_row_that_stays_keeps_its_group_node(), cfg(), duplicate_ip_existing_vs_existing_keeps_collected(), duplicate_ip_new_vs_existing_keeps_existing(), empty_online_router_removes_its_rows(), failed_and_unreachable_router_keep_rows_and_nodes(), flat_promotion_has_no_client_node_for_a_static_row(), flat_promotion_parent_is_own_client_node() (+28 more)

### Community 8 - "File Safety"
Cohesion: 0.11
Nodes (14): backup(), backup_keeps_the_last_n(), backup_keeps_the_new_folder_and_other_folders(), HDR, Live, load_state(), load_state_seeds_on_first_run_and_reads_after(), lock() (+6 more)

### Community 9 - "fs"
Cohesion: 0.12
Nodes (24): assemble(), changed_columns(), CycleInput, CycleOutput, names(), ppp(), ppp_owned(), Report (+16 more)

### Community 10 - "Config"
Cohesion: 0.11
Nodes (18): a_circuit_comment_must_not_be_the_static_comment(), DEFAULTS_TOML, exclusions_parse_and_bad_ip_is_rejected(), default_from_file!, MIN, minimal_config_gets_documented_defaults(), node_type_defaults_follow_target(), node_types_must_not_be_empty() (+10 more)

### Community 11 - "hashmap"
Cohesion: 0.15
Nodes (21): Network, NetworkMode, FlatGlobalRoot, FlatPromotion, FlatRouterRoot, GroupedRouterRoot, NoParent, RootPromotion (+13 more)

### Community 12 - "Session Notes"
Cohesion: 0.13
Nodes (17): Orphan branch lqosync3, RouterOS listen command (untested), Plan phases (Plan 1/2/2b done; Plan 3 v3, Plan 4 TUI, Plan 5 installer), Operations guide, Applied backups (keep_backups, 12-digit folders), Cutover from the Python service, lqosync3 detect (Plan 3), LibreQoS upgrade from 2.2 to 3.x (+9 more)

### Community 13 - "Topology 2"
Cohesion: 0.15
Nodes (20): Session 2026-10-31 project setup and v3 simulation, LibreQoS v3 ingress facts, Tool must reject duplicate IP and node name, libreqos_network.json schema_version 2, topology_import.json (internal, do not write), [topology] source setting, v3 offline simulation method, LibreQoS v3 ingress document (+12 more)

### Community 14 - "Topology 3"
Cohesion: 0.17
Nodes (18): Six Network Modes, Topology Builder, LibreQoS v3 topology.compile_mode, Plan 1 review focus (edge cases), Task 3: Model, state file, and seed, Task 5: Topology (build_nodes, to_network_json), Task 6: Cycle assembly and validation, lqosync3 core design spec (+10 more)

### Community 15 - "MikroTik Session 2"
Cohesion: 0.15
Nodes (7): exit_with(), main(), notify(), now(), print_summary(), run(), SECRET_OWNER_UID

### Community 16 - "Speed Rules"
Cohesion: 0.12
Nodes (15): config/defaults.toml embedded defaults, Home (vault MOC), RouterOS listen change detection (future), Configuration reference, Comment speed format (Name|25M/5M), Map from config.json, DHCP server discovery, Exclusions ([exclude], [routers.exclude]) (+7 more)

### Community 17 - "Status and Diff"
Cohesion: 0.17
Nodes (13): diff_config(), diff_dirs(), diff_dirs_needs_candidate_files(), diff_ignores_column_order_row_order_and_number_text(), diff_pair(), diff_reports_added_removed_changed(), diff_stops_in_apply_mode(), group() (+5 more)

### Community 18 - "MikroTik Session 3"
Cohesion: 0.13
Nodes (17): Applier (hash, atomic write, reload), Circuit Model, Collector (RouterOS API reads), Config Loader, Proposed Crate Layout, libreqos_network.json, lqosd (LibreQoS daemon), MikroTik RouterOS API (+9 more)

### Community 19 - "MikroTik Session 4"
Cohesion: 0.12
Nodes (18): network.json, cycle::assemble (pure core), files::backup (keep_backups), files::lock (lqosync3.lock), files::load_state (state.json or seed), mikrotik::MikrotikConnector / MikrotikSession, tests/mock_router.rs (mock RouterOS server), routeros::ReadError (+10 more)

### Community 20 - "MikroTik Session 5"
Cohesion: 0.16
Nodes (14): lqosync3 runtime design (Plan 2), Atomic write (temp file, sync_all, rename, fsync folder; only on byte change), Files: lock, state.json, status.json, candidate, applied, First-run seed (seed_state, seed_nodes from live files), Instance lock <state>/lqosync3.lock (File::try_lock), mikrotik-tokio 0.1.0 16-reply burst cancel bug, Plan 2 scope (in / not in), Process: run loop, SIGUSR1, SIGHUP, SIGTERM/SIGINT, once, check, diff, defaults (+6 more)

### Community 21 - "Speed Rules 2"
Cohesion: 0.20
Nodes (13): ADR-0004 TOML policy, Python defaults, Speed source order TOML lists, ADR-0007 Fresh design, not a port, Scenario tests (replace golden parity tests), Semantic compare with live files before cutover, updatecsv.py dissection, updatecsv.py defects to fix, Live production values (2026-10-31) (+5 more)

### Community 22 - "Topology 4"
Cohesion: 0.24
Nodes (12): ADR-0005 Topology follows v3, 14/15 CSV columns (v2_pair/v3_pair), max_floor_mbps (0.2) / safe_min_mbps (0.1), Parent Node ID column, ADR-0006 v3 guards and node types, compile_mode not full warning, How LibreQoS v3 handles nodes, v3 bridge boot gate (+4 more)

### Community 23 - "deserialize"
Cohesion: 0.16
Nodes (11): Entry, GuardHold, Source, Dhcp, Hotspot, Ppp, Unknown, State (+3 more)

### Community 24 - "Config 2"
Cohesion: 0.21
Nodes (13): CircuitNames, HDR, read_csv(), read_csv_accepts_bom_crlf_and_missing_columns(), read_csv_skips_blank_names_and_keeps_repeated_names(), seed_infers_source_and_keeps_ids(), seed_nodes(), seed_nodes_reads_the_tree_with_parents_and_ids() (+5 more)

### Community 26 - "Status and Diff 2"
Cohesion: 0.21
Nodes (7): 2026-10-31 First live shadow run, FSEHub-Distro router (10.200.1.2:8728), Shadow run results (62 rows, 167 ms, 26 text-only updates), Handoff, Production state (Python live, lqosync3 shadow, LibreQoS 2.2), Differences from updatecsv.py, Task 1: Crate scaffold and config

### Community 27 - "Speed Rules 3"
Cohesion: 0.21
Nodes (8): ADR-0009 DHCP discovery and exclusions, Speed Rules, Task 2: Rate format and speed rules (SpeedParser), Task 4: Circuit collection (collect_router), Task 7: Pair writer (render_csv, render_network_json), Circuit rules for each source, DhcpCfg::server(name) defaults, Output format (round3, fmt_rate, 14/15 CSV columns)

### Community 28 - "Plan 2 Runtime"
Cohesion: 0.18
Nodes (13): Runtime modes (shadow, apply), install.sh one-line installer (target contract), Installation (docs), Installer options (--mode, --version, --from-source, --import-json, --no-start), What the installer does (10 steps), Manual installation, Checks after installation, Removal procedure (+5 more)

### Community 29 - "Config 3"
Cohesion: 0.23
Nodes (6): Config, deep_merge(), defaults(), defaults_table(), Exclude, Safety

### Community 30 - "LibreQoS v3"
Cohesion: 0.24
Nodes (8): Columns, V2, V3, CsvRow, csv_header_sort_and_quoting(), field(), render_csv(), row()

### Community 31 - "dhcp_server_gives_override_or_"
Cohesion: 0.22
Nodes (10): dhcp_server_gives_override_or_default(), DhcpCfg, DhcpMode, PerPlan, PerSite, DhcpServer, DhcpSource, LeaseComment (+2 more)

### Community 32 - "Router State 2"
Cohesion: 0.55
Nodes (10): RouterMemo, acc(), classify(), first_unreachable_sets_since(), login_tls_trap_timeout_are_failed_and_keep_memo(), memo(), offline_after_zero_is_offline_at_once(), offline_at_offline_after() (+2 more)

### Community 33 - "CLI and Checks 2"
Cohesion: 0.24
Nodes (4): BackstopCap, BackstopCaps, BandwidthDefaults, HotspotCfg

### Community 34 - "Status and Diff 3"
Cohesion: 0.31
Nodes (10): first_forty(), guard_cfg(), guard_holds_a_mass_removal_and_keeps_other_changes(), guard_off_removes_at_once(), guard_releases_on_force(), guard_releases_on_same_fingerprint_next_cycle(), guard_restarts_on_a_different_fingerprint(), hundred() (+2 more)

### Community 35 - "Router State 3"
Cohesion: 0.25
Nodes (5): Router state (online/failed/unreachable/offline), routerstate::classify, Core design spec (2026-10-31-lqosync3-core-design.md), lqosync3 Runtime Implementation Plan (Plan 2), Runtime design spec (2026-10-31-lqosync3-runtime-design.md)

### Community 36 - "Topology 5"
Cohesion: 0.36
Nodes (8): 2026-10-31 Plan 1: lqosync3 core library session, lqosync3 core crate (config, speed, model, state, sources, topology, cycle, validate, writer), Plan 1 review fixes (commit 529fa36), Session: updatecsv.py dissection, lqosync3 Core Implementation Plan (Plan 1), Task 8: Scenario tests and v3 check, Scenario tests (synthetic fixtures), tools/v3sim/check.sh (v3 compiler check)

### Community 37 - "Decisions"
Cohesion: 0.53
Nodes (4): ADR-0003 Router failed vs offline, offline_action (default remove), offline_after (default 300 s), Architecture (LQoSync3)

### Community 38 - "Status and Diff 4"
Cohesion: 0.53
Nodes (5): ADR-0008 Terminal UI, TUI config editor (toml_edit), SIGUSR1 sync now / SIGHUP reload, status.json state file, Delivery in five plans (core, runtime, v3, TUI, install)

### Community 39 - "Telegram Notifier 3"
Cohesion: 0.33
Nodes (5): Command reference (check, once, diff, run, defaults), status.json cycle status, cli::parse / cli::Command, status::Status / RouterStatus, Notification events (changes, error, cleared, router state, guard)

### Community 40 - "LibreQoS v3 2"
Cohesion: 0.33
Nodes (6): Libreqos, Target, Auto, V2Pair, V3Network, V3Pair

### Community 41 - "LogLevel"
Cohesion: 0.40
Nodes (5): LogLevel, Debug, Error, Info, Warn

### Community 42 - "CLI and Checks 3"
Cohesion: 0.50
Nodes (3): ClientTemplate, Nodes, NodeTemplate

### Community 43 - "OfflineAction"
Cohesion: 0.67
Nodes (3): OfflineAction, Keep, Remove

## Ambiguous Edges - Review These
- `Collector (RouterOS API reads)` → `RouterOS listen change detection (future)`  [AMBIGUOUS]
  docs/ARCHITECTURE.md · relation: conceptually_related_to
- `Stable node id (lq3- FNV-1a 64)` → `Scenario tests (replace golden parity tests)`  [AMBIGUOUS]
  brain/20-decisions/ADR-0007-fresh-design-not-a-port.md · relation: conceptually_related_to

## Knowledge Gaps
- **100 isolated node(s):** `Failed`, `Offline`, `Unreachable`, `Hotspot`, `Ppp` (+95 more)
  These have ≤1 connection - possible missing edges or undocumented components. (Counts symbols only; 215 node(s) total have ≤1 connection when file, concept and rationale nodes are included.)
- **9 thin communities (<3 nodes) omitted from report** — run `graphify query` to explore isolated nodes.

## Suggested Questions
_Questions this graph is uniquely positioned to answer:_

- **Why does `Task 2: Events and message` connect `Telegram Notifier` to `Telegram Notifier 3`?**
  _High betweenness centrality (0.245) - this node is a cross-community bridge._
- **Are the 10 inferred relationships involving `read_router()` (e.g. with `.cycle_inner()` and `closed_port_is_unreachable()`) actually correct?**
  _`read_router()` has 10 INFERRED edges - model-reasoned connections that need verification._
- **What connects `Failed`, `Offline`, `Unreachable` to the rest of the system?**
  _100 weakly-connected nodes found - possible documentation gaps or missing edges._
- **Should `MikroTik Session` be split into smaller, more focused modules?**
  _Cohesion score 0.05393258426966292 - nodes in this community are weakly interconnected._
- **Why does `Config` connect `Config 3` to `MikroTik Session`, `CLI and Checks 2`, `Router State`, `Telegram Notifier`, `Status and Diff 3`, `Telegram Notifier 2`, `Secrets`, `CLI and Checks`, `LibreQoS v3 2`, `fs`, `Config`, `hashmap`, `CLI and Checks 3`, `Circuit Collection`, `MikroTik Session 2`, `Topology`, `Status and Diff`, `Config 2`?**
  _High betweenness centrality (0.193) - this node is a cross-community bridge._

### Low-confidence Hints
_AMBIGUOUS edges — the extractor was unsure. Verify before acting on these._

- **What is the exact relationship between `Collector (RouterOS API reads)` and `RouterOS listen change detection (future)`?**
  _Edge tagged AMBIGUOUS (relation: conceptually_related_to) - confidence is low._
- **What is the exact relationship between `Stable node id (lq3- FNV-1a 64)` and `Scenario tests (replace golden parity tests)`?**
  _Edge tagged AMBIGUOUS (relation: conceptually_related_to) - confidence is low._