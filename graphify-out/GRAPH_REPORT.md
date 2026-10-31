# Graph Report - LQoSync3.0  (2026-10-31)

## Corpus Check
- 63 files · ~59,511 words
- Verdict: corpus is large enough that graph structure adds value.
- Unclassified: 4 file(s) not represented in the graph (top: (none) 2, .toml 1, .service 1)

## Summary
- 1038 nodes · 2590 edges · 48 communities (38 shown, 10 thin omitted)
- Extraction: 94% EXTRACTED · 5% INFERRED · 0% AMBIGUOUS · INFERRED: 141 edges (avg confidence: 0.88)
- Token cost: 0 input · 0 output

## Graph Freshness
- Built from commit: `85859334`
- Run `git rev-parse HEAD` and compare to check if the graph is stale.
- Run `graphify update .` after code changes (no API cost).

## Community Hubs (Navigation)
- mock_router.rs
- Telegram Notifier
- Router State
- Topology
- Circuit Collection
- Telegram Notifier 2
- Secrets
- cycle.rs
- File Safety
- routeros.rs
- Config
- hashmap
- Session Notes
- Topology 2
- CsvRow
- mikrotik.rs
- Configuration reference
- v3network.rs
- runner::Runner::cycle (one full cycle)
- read_router
- MikroTik Session 5
- Speed Rules 2
- scenarios.rs
- PppSource
- main.rs
- CertificateDer
- Configuration reference
- Speed Rules 3
- Speed
- .merge
- lqosync3 Runtime Implementation Plan (Plan 2)
- dhcp_server_gives_override_or_
- Router State 2
- lqosync3 LibreQoS v3 targets (Plan 3)
- Review Focus
- RouterResult
- lqosync3 core design spec
- detect.rs
- LogLevel
- State
- check.sh
- Telegram Notifier 4
- CLI and Checks 4
- lqosync3

## God Nodes (most connected - your core abstractions)
1. `Config` - 62 edges
2. `CsvRow` - 37 edges
3. `lqosync3 core design spec` - 32 edges
4. `run()` - 31 edges
5. `Configuration reference` - 31 edges
6. `Handoff` - 28 edges
7. `read_router()` - 26 edges
8. `RouterCfg` - 25 edges
9. `ReadError` - 25 edges
10. `Rate` - 24 edges

## Surprising Connections (you probably didn't know these)
- `Task 2: Events and message` --references--> `split()`  [EXTRACTED]
  docs/superpowers/plans/2026-10-31-lqosync3-telegram.md → src/notify.rs
- `Safety rules (shadow default, no router writes, 0600 secrets)` --semantically_similar_to--> `Operational rules`  [INFERRED] [semantically similar]
  README.md → docs/OPERATIONS.md
- `v3 offline simulation method` --semantically_similar_to--> `Offline v3 simulation`  [INFERRED] [semantically similar]
  brain/30-knowledge/v3-offline-simulation-method.md → docs/LIBREQOS-V3-INGRESS.md
- `v3 offline simulation method` --semantically_similar_to--> `/run/lqos/bus 0777 safety caution`  [INFERRED] [semantically similar]
  brain/30-knowledge/v3-offline-simulation-method.md → docs/LIBREQOS-V3-INGRESS.md
- `Task 2: Events and message` --references--> `Notifier`  [EXTRACTED]
  docs/superpowers/plans/2026-10-31-lqosync3-telegram.md → src/notify.rs

## Import Cycles
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

## Communities (48 total, 10 thin omitted)

### Community 0 - "mock_router.rs"
Cohesion: 0.16
Nodes (25): read_router(), Answer, Empty, Raw, Rows, Silent, Trap, closed_port_is_unreachable() (+17 more)

### Community 1 - "Telegram Notifier"
Cohesion: 0.08
Nodes (43): Task 3: Transport (post_json, send, token, chat_id), Telegram, a_config_error_is_an_error_event(), a_long_multi_line_error_splits_into_valid_html_parts(), a_very_long_error_is_cut(), an_empty_token_is_an_error(), an_error_is_sent_once_then_cleared(), body_complete() (+35 more)

### Community 2 - "Router State"
Cohesion: 0.14
Nodes (31): target_name(), Connector, a_config_reload_error_shows_in_status_until_a_good_reload(), a_held_lock_stops_the_cycle_without_touching_status(), a_panicking_router_task_fails_only_that_router(), apply_backs_up_writes_and_reloads(), assemble_error_writes_nothing(), auto_on_v2_gives_the_v2_pair() (+23 more)

### Community 3 - "Topology"
Cohesion: 0.10
Nodes (48): NodeTemplate, StoredNode, GroupKey, DhcpPlan, DhcpSite, Hotspot, Ppp, PppPlan (+40 more)

### Community 4 - "Circuit Collection"
Cohesion: 0.12
Nodes (44): Config, Exclude, parse_ipv4_net(), RouterCfg, RosRow, RouterSnapshot, a_zero_speed_profile_comment_gives_one_line(), by_name() (+36 more)

### Community 5 - "Telegram Notifier 2"
Cohesion: 0.08
Nodes (37): Args, check(), check_finds_plain_api_and_no_source_router(), check_reads_the_telegram_token_when_enabled(), check_reports_a_bad_secret_and_a_missing_lqos_dir(), check_shows_the_target_or_the_detection_error(), CheckLine, Command (+29 more)

### Community 6 - "Secrets"
Cohesion: 0.11
Nodes (18): UnitlessUnit, Bps, Mbps, circuit_rates(), CircuitRates, clamp(), default_rate_and_circuit_rates(), finite() (+10 more)

### Community 7 - "cycle.rs"
Cohesion: 0.12
Nodes (46): a_row_that_stays_keeps_its_group_node(), cfg(), duplicate_ip_existing_vs_existing_keeps_collected(), duplicate_ip_new_vs_existing_keeps_existing(), empty_online_router_removes_its_rows(), failed_and_unreachable_router_keep_rows_and_nodes(), first_forty(), flat_promotion_has_no_client_node_for_a_static_row() (+38 more)

### Community 8 - "File Safety"
Cohesion: 0.09
Nodes (27): CircuitNames, backup(), backup_keeps_the_last_n(), backup_keeps_the_new_folder_and_other_folders(), HDR, Live, load_state(), load_state_seeds_on_first_run_and_reads_after() (+19 more)

### Community 9 - "routeros.rs"
Cohesion: 0.16
Nodes (14): a_dead_kept_session_reconnects_without_a_retry(), a_login_error_is_not_retried(), Fake, gives_last_error_after_all_retries(), ppp_router(), Trap, reads_only_enabled_tables(), retry_reconnects_and_succeeds() (+6 more)

### Community 10 - "Config"
Cohesion: 0.09
Nodes (23): a_circuit_comment_must_not_be_the_static_comment(), BackstopCap, BackstopCaps, ClientTemplate, DEFAULTS_TOML, exclusions_parse_and_bad_ip_is_rejected(), default_from_file!, MIN (+15 more)

### Community 11 - "hashmap"
Cohesion: 0.15
Nodes (21): Network, NetworkMode, FlatGlobalRoot, FlatPromotion, FlatRouterRoot, GroupedRouterRoot, NoParent, RootPromotion (+13 more)

### Community 12 - "Session Notes"
Cohesion: 0.08
Nodes (28): Orphan branch lqosync3, RouterOS listen command (untested), 14/15 CSV columns (v2_pair/v3_pair), libreqos_network.json schema_version 2, Plan phases (Plan 1/2/2b done; Plan 3 v3, Plan 4 TUI, Plan 5 installer), Runtime modes (shadow, apply), install.sh one-line installer (target contract), Installation (docs) (+20 more)

### Community 13 - "Topology 2"
Cohesion: 0.12
Nodes (32): Session 2026-10-31 project setup and v3 simulation, ADR-0005 Topology follows v3, max_floor_mbps (0.2) / safe_min_mbps (0.1), Parent Node ID column, ADR-0006 v3 guards and node types, compile_mode not full warning, LibreQoS v3 ingress facts, Tool must reject duplicate IP and node name (+24 more)

### Community 14 - "CsvRow"
Cohesion: 0.09
Nodes (20): Change, changed_columns(), Entry, Report, Columns, V2, V3, CsvRow (+12 more)

### Community 15 - "mikrotik.rs"
Cohesion: 0.15
Nodes (6): map_err(), MikrotikConnector, MikrotikSession, tls_client_config(), RouterSession, Setup

### Community 16 - "Configuration reference"
Cohesion: 0.12
Nodes (17): Plan 2b deferred minor items, Telegram settings, Command reference (check, once, diff, run, defaults), status.json cycle status, Turn on Telegram procedure, cli::parse / cli::Command, status::Status / RouterStatus, Plan 2b: lqosync3 Telegram implementation (+9 more)

### Community 17 - "v3network.rs"
Cohesion: 0.12
Nodes (30): diff_config(), diff_dirs(), diff_dirs_needs_candidate_files(), diff_ignores_column_order_row_order_and_number_text(), diff_pair(), diff_reports_added_removed_changed(), diff_stops_for_v3_network(), diff_stops_in_apply_mode() (+22 more)

### Community 18 - "runner::Runner::cycle (one full cycle)"
Cohesion: 0.06
Nodes (39): Applier (hash, atomic write, reload), Circuit Model, Collector (RouterOS API reads), Config Loader, Proposed Crate Layout, libreqos_network.json, lqosd (LibreQoS daemon), network.json (+31 more)

### Community 19 - "read_router"
Cohesion: 0.15
Nodes (8): FakeSession, read_snapshot(), ReadError, Login, Protocol, Timeout, Tls, Unreachable

### Community 20 - "MikroTik Session 5"
Cohesion: 0.16
Nodes (14): lqosync3 runtime design (Plan 2), Atomic write (temp file, sync_all, rename, fsync folder; only on byte change), Files: lock, state.json, status.json, candidate, applied, First-run seed (seed_state, seed_nodes from live files), Instance lock <state>/lqosync3.lock (File::try_lock), mikrotik-tokio 0.1.0 16-reply burst cancel bug, Plan 2 scope (in / not in), Process: run loop, SIGUSR1, SIGHUP, SIGTERM/SIGINT, once, check, diff, defaults (+6 more)

### Community 21 - "Speed Rules 2"
Cohesion: 0.14
Nodes (20): ADR-0003 Router failed vs offline, offline_action (default remove), offline_after (default 300 s), ADR-0004 TOML policy, Python defaults, Speed source order TOML lists, ADR-0007 Fresh design, not a port, Scenario tests (replace golden parity tests), Semantic compare with live files before cutover (+12 more)

### Community 22 - "scenarios.rs"
Cohesion: 0.20
Nodes (14): assemble(), CycleInput, CycleOutput, names(), State, fmt_rate(), fnv1a64(), render_network_json() (+6 more)

### Community 23 - "PppSource"
Cohesion: 0.29
Nodes (7): PppSource, ActiveComment, Default, ProfileComment, ProfileName, ProfileRateLimit, SecretComment

### Community 24 - "main.rs"
Cohesion: 0.18
Nodes (8): exit_with(), main(), notify(), now(), print_summary(), run(), SECRET_OWNER_UID, CycleSummary

### Community 26 - "Configuration reference"
Cohesion: 0.08
Nodes (20): 2026-10-31 First live shadow run, FSEHub-Distro router (10.200.1.2:8728), Shadow run results (62 rows, 167 ms, 26 text-only updates), config/defaults.toml embedded defaults, Production state (Python live, lqosync3 shadow, LibreQoS 2.2), RouterOS listen change detection (future), Configuration reference, Comment speed format (Name|25M/5M) (+12 more)

### Community 27 - "Speed Rules 3"
Cohesion: 0.21
Nodes (8): ADR-0009 DHCP discovery and exclusions, Speed Rules, Task 2: Rate format and speed rules (SpeedParser), Task 4: Circuit collection (collect_router), Task 7: Pair writer (render_csv, render_network_json), Circuit rules for each source, DhcpCfg::server(name) defaults, Output format (round3, fmt_rate, 14/15 CSV columns)

### Community 28 - "Speed"
Cohesion: 0.18
Nodes (8): DhcpSource, LeaseComment, ServerComment, Settings, RateLimitOrder, DownloadUpload, RouterosRxTx, Speed

### Community 29 - ".merge"
Cohesion: 0.47
Nodes (3): deep_merge(), defaults(), defaults_table()

### Community 30 - "lqosync3 Runtime Implementation Plan (Plan 2)"
Cohesion: 0.25
Nodes (5): Router state (online/failed/unreachable/offline), routerstate::classify, Core design spec (2026-10-31-lqosync3-core-design.md), lqosync3 Runtime Implementation Plan (Plan 2), Runtime design spec (2026-10-31-lqosync3-runtime-design.md)

### Community 31 - "dhcp_server_gives_override_or_"
Cohesion: 0.21
Nodes (8): BandwidthDefaults, dhcp_server_gives_override_or_default(), DhcpCfg, DhcpMode, PerPlan, PerSite, DhcpServer, HotspotCfg

### Community 32 - "Router State 2"
Cohesion: 0.34
Nodes (14): OfflineAction, Keep, Remove, RouterAccess, RouterMemo, acc(), classify(), first_unreachable_sets_since() (+6 more)

### Community 33 - "lqosync3 LibreQoS v3 targets (Plan 3)"
Cohesion: 0.22
Nodes (9): CLI, Config changes, lqosync3 LibreQoS v3 targets (Plan 3), Offline check result (2026-10-31), Out of scope, Purpose, Target selection (ADR-0002), `v3_network` (+1 more)

### Community 34 - "Review Focus"
Cohesion: 0.25
Nodes (7): Global Constraints, lqosync3 LibreQoS v3 Implementation Plan (Plan 3), Review Focus, Task 1: Detector (`src/detect.rs`), Task 2: `v3_network` writer (`src/writer/v3network.rs`), Task 3: Runner, CLI, and check, Task 4: Offline check with the v3 parser

### Community 35 - "RouterResult"
Cohesion: 0.32
Nodes (8): ppp(), ppp_owned(), RouterResult, Failed, Offline, Online, Unreachable, name()

### Community 36 - "lqosync3 core design spec"
Cohesion: 0.12
Nodes (28): 2026-10-31 Plan 1: lqosync3 core library session, lqosync3 core crate (config, speed, model, state, sources, topology, cycle, validate, writer), Plan 1 review fixes (commit 529fa36), Session: updatecsv.py dissection, Architecture (LQoSync3), Six Network Modes, Topology Builder, LibreQoS v3 topology.compile_mode (+20 more)

### Community 40 - "detect.rs"
Cohesion: 0.18
Nodes (18): Libreqos, Target, Auto, V2Pair, V3Network, V3Pair, Detected, Env (+10 more)

### Community 41 - "LogLevel"
Cohesion: 0.20
Nodes (9): LogLevel, Debug, Error, Info, Warn, Runtime, RuntimeMode, Apply (+1 more)

## Ambiguous Edges - Review These
- `RouterOS listen change detection (future)` → `Collector (RouterOS API reads)`  [AMBIGUOUS]
  docs/ARCHITECTURE.md · relation: conceptually_related_to
- `Scenario tests (replace golden parity tests)` → `Stable node id (lq3- FNV-1a 64)`  [AMBIGUOUS]
  brain/20-decisions/ADR-0007-fresh-design-not-a-port.md · relation: conceptually_related_to

## Knowledge Gaps
- **101 isolated node(s):** `lqosync3`, `Check`, `Once`, `Run`, `Diff` (+96 more)
  These have ≤1 connection - possible missing edges or undocumented components. (Counts symbols only; 232 node(s) total have ≤1 connection when file, concept and rationale nodes are included.)
- **10 thin communities (<3 nodes) omitted from report** — run `graphify query` to explore isolated nodes.

## Suggested Questions
_Questions this graph is uniquely positioned to answer:_

- **Why does `Task 3: Transport (post_json, send, token, chat_id)` connect `Telegram Notifier` to `Configuration reference`?**
  _High betweenness centrality (0.238) - this node is a cross-community bridge._
- **What connects `lqosync3`, `Check`, `Once` to the rest of the system?**
  _101 weakly-connected nodes found - possible documentation gaps or missing edges._
- **Should `Telegram Notifier` be split into smaller, more focused modules?**
  _Cohesion score 0.08106473079249849 - nodes in this community are weakly interconnected._
- **Why does `Handoff` connect `Topology 2` to `lqosync3 core design spec`, `Session Notes`, `Configuration reference`, `Speed Rules 2`, `Configuration reference`, `lqosync3 Runtime Implementation Plan (Plan 2)`?**
  _High betweenness centrality (0.166) - this node is a cross-community bridge._
- **Should `Router State` be split into smaller, more focused modules?**
  _Cohesion score 0.14153561517113783 - nodes in this community are weakly interconnected._

### Low-confidence Hints
_AMBIGUOUS edges — the extractor was unsure. Verify before acting on these._

- **What is the exact relationship between `RouterOS listen change detection (future)` and `Collector (RouterOS API reads)`?**
  _Edge tagged AMBIGUOUS (relation: conceptually_related_to) - confidence is low._
- **What is the exact relationship between `Scenario tests (replace golden parity tests)` and `Stable node id (lq3- FNV-1a 64)`?**
  _Edge tagged AMBIGUOUS (relation: conceptually_related_to) - confidence is low._