# Graph Report - LQoSync3.0  (2026-10-31)

## Corpus Check
- 15 files · ~49,747 words
- Verdict: corpus is large enough that graph structure adds value.

## Summary
- 950 nodes · 2274 edges · 53 communities (45 shown, 8 thin omitted)
- Extraction: 95% EXTRACTED · 5% INFERRED · 0% AMBIGUOUS · INFERRED: 121 edges (avg confidence: 0.89)
- Token cost: 118,530 input · 0 output

## Community Hubs (Navigation)
- MikroTik Session
- Circuit Collection
- Topology
- CLI and Checks
- Speed Rules
- CLI and Checks 2
- Mock Router Tests
- Topology 2
- File Safety
- Config
- hashmap
- MikroTik Session 2
- Session Notes
- Speed Rules 2
- Status and Diff
- MikroTik Session 3
- Status and Diff 2
- fs
- Router State
- Speed Rules 3
- Router Reads
- Topology 3
- btreemap
- Mock Router Tests 2
- MikroTik Session 4
- Config 2
- Config 3
- MikroTik Session 5
- CertificateDer
- Topology 4
- Status and Diff 3
- Topology 5
- BandwidthDefaults
- Secrets
- LibreQoS v3
- Decisions
- CLI and Checks 3
- Speed Rules 4
- Status and Diff 4
- Topology 6
- Plan 2 Runtime
- Session Notes 2
- Router State 2
- Status and Diff 5
- .default()
- LibreQoS v3 2
- State
- check.sh
- Notifier (Telegram)
- lqosync3

## God Nodes (most connected - your core abstractions)
1. `Config` - 61 edges
2. `lqosync3 core design spec` - 32 edges
3. `run()` - 31 edges
4. `CsvRow` - 26 edges
5. `read_router()` - 26 edges
6. `RouterCfg` - 25 edges
7. `ReadError` - 25 edges
8. `Rate` - 24 edges
9. `build_nodes()` - 21 edges
10. `state()` - 20 edges

## Surprising Connections (you probably didn't know these)
- `v3 offline simulation method` --semantically_similar_to--> `Offline v3 simulation`  [INFERRED] [semantically similar]
  brain/30-knowledge/v3-offline-simulation-method.md → docs/LIBREQOS-V3-INGRESS.md
- `v3 offline simulation method` --semantically_similar_to--> `/run/lqos/bus 0777 safety caution`  [INFERRED] [semantically similar]
  brain/30-knowledge/v3-offline-simulation-method.md → docs/LIBREQOS-V3-INGRESS.md
- `ADR-0002 v3 target selection` --references--> `topology_import.json (internal)`  [INFERRED]
  brain/20-decisions/ADR-0002-v3-target-selection.md → docs/LIBREQOS-V3-INGRESS.md
- `scenarios_match_expected_output()` --calls--> `split_static()`  [INFERRED]
  tests/scenarios.rs → src/state.rs
- `scenarios_match_expected_output()` --calls--> `read_csv()`  [INFERRED]
  tests/scenarios.rs → src/state.rs

## Import Cycles
- 2-file cycle: `src/config.rs -> src/speed.rs -> src/config.rs`

## Hyperedges (group relationships)
- **Router-to-LibreQoS cycle data flow** — docs_architecture_collector, docs_architecture_circuit_model, docs_architecture_validator, docs_architecture_target_detector, docs_architecture_writers, docs_architecture_applier [EXTRACTED 1.00]
- **Plan 1 core tasks building assemble()** — docs_superpowers_plans_2026_10_31_lqosync3_core_task1_config, docs_superpowers_plans_2026_10_31_lqosync3_core_task2_speed, docs_superpowers_plans_2026_10_31_lqosync3_core_task3_state, docs_superpowers_plans_2026_10_31_lqosync3_core_task4_sources, docs_superpowers_plans_2026_10_31_lqosync3_core_task5_topology, docs_superpowers_plans_2026_10_31_lqosync3_core_task6_cycle, docs_superpowers_plans_2026_10_31_lqosync3_core_task7_writer, docs_superpowers_plans_2026_10_31_lqosync3_core_task8_scenarios [EXTRACTED 1.00]
- **Output target selection** — readme_target_auto, readme_v2_pair, readme_v3_network, readme_v3_pair, brain_30_knowledge_libreqos_v3_ingress_facts_topology_source [EXTRACTED 1.00]
- **Removal guard hold-and-release flow** — docs_configuration_removal_guard, docs_superpowers_specs_2026_10_31_lqosync3_core_design_cycle_assembly_rules, docs_superpowers_specs_2026_10_31_lqosync3_core_design_guard_fingerprint, docs_superpowers_specs_2026_10_31_lqosync3_core_design_state_file [EXTRACTED 1.00]
- **Router failure vs offline handling** — brain_20_decisions_adr_0003_router_failed_vs_offline_router_state_model, brain_20_decisions_adr_0003_router_failed_vs_offline_offline_after, brain_20_decisions_adr_0003_router_failed_vs_offline_offline_action, brain_30_knowledge_updatecsv_py_dissection_defects_to_fix [EXTRACTED 1.00]
- **Crash-safe file writes** — docs_superpowers_specs_2026_10_31_lqosync3_runtime_design_instance_lock, docs_superpowers_specs_2026_10_31_lqosync3_runtime_design_write_order_state_first, docs_superpowers_specs_2026_10_31_lqosync3_runtime_design_atomic_write, docs_superpowers_specs_2026_10_31_lqosync3_runtime_design_first_run_seed [INFERRED 0.85]
- **LibreQoS v3 node rule compliance** — brain_20_decisions_adr_0005_topology_follows_v3_stable_node_id, brain_20_decisions_adr_0005_topology_follows_v3_parent_node_id, brain_20_decisions_adr_0005_topology_follows_v3_csv_column_count, brain_20_decisions_adr_0005_topology_follows_v3_max_floor_mbps, brain_20_decisions_adr_0005_topology_follows_v3_flat_promotion_parent_fix, brain_20_decisions_adr_0006_v3_guards_and_node_types_no_parent_guard, brain_20_decisions_adr_0006_v3_guards_and_node_types_site_group_nodes, brain_20_decisions_adr_0006_v3_guards_and_node_types_compile_mode_warning [INFERRED 0.85]
- **DHCP discovery and subscriber exclusion** — brain_20_decisions_adr_0009_dhcp_discovery_and_exclusions_dhcp_server_discovery, brain_20_decisions_adr_0009_dhcp_discovery_and_exclusions_exclusions, docs_configuration_exclusions, docs_superpowers_specs_2026_10_31_lqosync3_core_design_circuit_rules [INFERRED 0.95]
- **Router failed/unreachable/offline state model** — brain_handoff_adr_0003_router_failed_vs_offline, docs_configuration_router_state, docs_configuration_router_access, docs_superpowers_plans_2026_10_31_lqosync3_runtime_classify, docs_superpowers_plans_2026_10_31_lqosync3_runtime_read_router, docs_superpowers_plans_2026_10_31_lqosync3_runtime_read_error [INFERRED 0.85]
- **Shadow run, diff, and cutover to apply** — docs_configuration_runtime_modes, docs_operations_lqosync3_once, docs_operations_lqosync3_diff, docs_operations_cutover_procedure, brain_handoff_manual_shadow_run, brain_handoff_lqosync_updatecsv_service [INFERRED 0.85]
- **Safe cycle write path (lock, atomic write, backup, reload)** — docs_superpowers_plans_2026_10_31_lqosync3_runtime_runner_cycle, docs_superpowers_plans_2026_10_31_lqosync3_runtime_files_lock, docs_superpowers_plans_2026_10_31_lqosync3_runtime_write_if_changed, docs_superpowers_plans_2026_10_31_lqosync3_runtime_files_backup, docs_superpowers_plans_2026_10_31_lqosync3_runtime_run_reload, docs_operations_tool_backups [INFERRED 0.85]

## Communities (53 total, 8 thin omitted)

### Community 0 - "MikroTik Session"
Cohesion: 0.07
Nodes (36): exit_with(), main(), now(), print_summary(), run(), SECRET_OWNER_UID, a_config_reload_error_shows_in_status_until_a_good_reload(), a_held_lock_stops_the_cycle_without_touching_status() (+28 more)

### Community 1 - "Circuit Collection"
Cohesion: 0.11
Nodes (42): parse_ipv4_net(), a_zero_speed_profile_comment_gives_one_line(), by_name(), cfg(), cfg_text(), circuit_templates_can_use_router(), collect_dhcp(), collect_hotspot() (+34 more)

### Community 2 - "Topology"
Cohesion: 0.12
Nodes (45): NodeTemplate, StoredNode, GroupKey, Hotspot, Ppp, a_kept_child_keeps_its_operator_children(), backstop_cap_limits_group(), build_nodes() (+37 more)

### Community 3 - "CLI and Checks"
Cohesion: 0.08
Nodes (35): Args, check(), check_finds_plain_api_and_no_source_router(), check_rejects_v3_target_for_now(), check_reports_a_bad_secret_and_a_missing_lqos_dir(), CheckLine, Command, Check (+27 more)

### Community 4 - "Speed Rules"
Cohesion: 0.10
Nodes (21): RateLimitOrder, DownloadUpload, RouterosRxTx, UnitlessUnit, Bps, Mbps, circuit_rates(), CircuitRates (+13 more)

### Community 5 - "CLI and Checks 2"
Cohesion: 0.16
Nodes (36): a_row_that_stays_keeps_its_group_node(), cfg(), duplicate_ip_existing_vs_existing_keeps_collected(), duplicate_ip_new_vs_existing_keeps_existing(), empty_online_router_removes_its_rows(), failed_and_unreachable_router_keep_rows_and_nodes(), flat_promotion_has_no_client_node_for_a_static_row(), flat_promotion_parent_is_own_client_node() (+28 more)

### Community 6 - "Mock Router Tests"
Cohesion: 0.15
Nodes (25): read_router(), Answer, Empty, Raw, Rows, Silent, Trap, closed_port_is_unreachable() (+17 more)

### Community 7 - "Topology 2"
Cohesion: 0.12
Nodes (27): 2026-10-31 Plan 1: lqosync3 core library session, lqosync3 core crate (config, speed, model, state, sources, topology, cycle, validate, writer), Plan 1 review fixes (commit 529fa36), ADR-0007 Fresh design, not a port, Scenario tests (replace golden parity tests), Six Network Modes, Topology Builder, LibreQoS v3 topology.compile_mode (+19 more)

### Community 8 - "File Safety"
Cohesion: 0.11
Nodes (14): backup(), backup_keeps_the_last_n(), backup_keeps_the_new_folder_and_other_folders(), HDR, Live, load_state(), load_state_seeds_on_first_run_and_reads_after(), lock() (+6 more)

### Community 9 - "Config"
Cohesion: 0.11
Nodes (17): a_circuit_comment_must_not_be_the_static_comment(), DEFAULTS_TOML, exclusions_parse_and_bad_ip_is_rejected(), default_from_file!, MIN, minimal_config_gets_documented_defaults(), node_type_defaults_follow_target(), node_types_must_not_be_empty() (+9 more)

### Community 10 - "hashmap"
Cohesion: 0.16
Nodes (20): NetworkMode, FlatGlobalRoot, FlatPromotion, FlatRouterRoot, GroupedRouterRoot, NoParent, RootPromotion, check() (+12 more)

### Community 11 - "MikroTik Session 2"
Cohesion: 0.11
Nodes (19): Applier (hash, atomic write, reload), Circuit Model, Collector (RouterOS API reads), Config Loader, Proposed Crate Layout, libreqos_network.json, lqosd (LibreQoS daemon), MikroTik RouterOS API (+11 more)

### Community 12 - "Session Notes"
Cohesion: 0.16
Nodes (14): a_dead_kept_session_reconnects_without_a_retry(), a_login_error_is_not_retried(), Fake, FakeSession, gives_last_error_after_all_retries(), ppp_router(), Trap, reads_only_enabled_tables() (+6 more)

### Community 13 - "Speed Rules 2"
Cohesion: 0.17
Nodes (15): ADR-0003 Router failed vs offline, offline_action (default remove), offline_after (default 300 s), ADR-0004 TOML policy, Python defaults, Speed source order TOML lists, Semantic compare with live files before cutover, updatecsv.py dissection, updatecsv.py defects to fix (+7 more)

### Community 14 - "Status and Diff"
Cohesion: 0.14
Nodes (13): Handoff 2026-10-31 (documentation phase), libreqos-v3-ingress-facts (vault note), Plan 2b: Telegram, Plan 3: LibreQoS v3 detection and writers, v3-offline-simulation-method (vault note), tools/v3sim/check.sh (v3 3.0.7 compiler check), Differences from updatecsv.py, rate_limit_order and unitless_unit (+5 more)

### Community 15 - "MikroTik Session 3"
Cohesion: 0.16
Nodes (9): RouterCfg, map_err(), MikrotikConnector, MikrotikSession, tls_client_config(), RosRow, Connector, RouterSession (+1 more)

### Community 16 - "Status and Diff 2"
Cohesion: 0.17
Nodes (13): diff_config(), diff_dirs(), diff_dirs_needs_candidate_files(), diff_ignores_column_order_row_order_and_number_text(), diff_pair(), diff_reports_added_removed_changed(), diff_stops_in_apply_mode(), group() (+5 more)

### Community 17 - "fs"
Cohesion: 0.17
Nodes (15): assemble(), changed_columns(), CycleInput, CycleOutput, names(), Report, fmt_rate(), fnv1a64() (+7 more)

### Community 18 - "Router State"
Cohesion: 0.24
Nodes (19): ppp(), ppp_owned(), RouterResult, Failed, Offline, Online, Unreachable, RouterMemo (+11 more)

### Community 19 - "Speed Rules 3"
Cohesion: 0.14
Nodes (14): [backstop_caps] per-source caps, [bandwidth_defaults] rate floors and factors, Comment speed format (Name|25M/5M, 25M/5M, 25M), config/defaults.toml (embedded defaults), DHCP server discovery (/ip/dhcp-server), DHCP speed_sources (server_comment, lease_comment, settings), Exclusions ([exclude] and [routers.exclude]), /etc/lqosync3/lqosync3.toml (+6 more)

### Community 20 - "Router Reads"
Cohesion: 0.14
Nodes (8): read_snapshot(), ReadError, Login, Protocol, Timeout, Tls, Unreachable, row()

### Community 21 - "Topology 3"
Cohesion: 0.22
Nodes (13): ADR-0005 Topology follows v3, 14/15 CSV columns (v2_pair/v3_pair), max_floor_mbps (0.2) / safe_min_mbps (0.1), Parent Node ID column, ADR-0006 v3 guards and node types, compile_mode not full warning, Tool must reject duplicate IP and node name, How LibreQoS v3 handles nodes (+5 more)

### Community 22 - "btreemap"
Cohesion: 0.15
Nodes (11): Entry, GuardHold, Source, Dhcp, Hotspot, Ppp, Unknown, State (+3 more)

### Community 23 - "Mock Router Tests 2"
Cohesion: 0.17
Nodes (15): network.json, lqosync3.service systemd unit, lqosync3 run (service loop), Signals (SIGHUP reload, SIGUSR1 cycle now), <state_dir>/status.json, cycle::assemble (pure core), files::lock (lqosync3.lock), files::load_state (state.json or seed) (+7 more)

### Community 24 - "MikroTik Session 4"
Cohesion: 0.19
Nodes (12): lqosync3 runtime design (Plan 2), Atomic write (temp file, sync_all, rename, fsync folder; only on byte change), Files: lock, state.json, status.json, candidate, applied, First-run seed (seed_state, seed_nodes from live files), Instance lock <state>/lqosync3.lock (File::try_lock), mikrotik-tokio 0.1.0 16-reply burst cancel bug, Plan 2 scope (in / not in), Process: run loop, SIGUSR1, SIGHUP, SIGTERM/SIGINT, once, check, diff, defaults (+4 more)

### Community 25 - "Config 2"
Cohesion: 0.17
Nodes (15): lqosync3 (Rust replacement for updatecsv.py), lqosync-updatecsv.service (live Python service), Plan 5: installer, updatecsv.py (Python predecessor), Map from config.json, lqosync3 import-json (planned), Runtime modes (shadow | apply), install.sh one-line installer (target contract) (+7 more)

### Community 26 - "Config 3"
Cohesion: 0.21
Nodes (13): CircuitNames, HDR, read_csv(), read_csv_accepts_bom_crlf_and_missing_columns(), read_csv_skips_blank_names_and_keeps_repeated_names(), seed_infers_source_and_keeps_ids(), seed_nodes(), seed_nodes_reads_the_tree_with_parents_and_ids() (+5 more)

### Community 27 - "MikroTik Session 5"
Cohesion: 0.15
Nodes (14): mikrotik-rs =0.9.0 (mikrotik-tokio 0.3.0), Configuration (docs), Config loader validation rules, Secret files (*_file keys, mode 0600, owner root), Manual installation, mikrotik::MikrotikConnector / MikrotikSession, routeros::ReadError, routeros::read_router (connect, retries) (+6 more)

### Community 29 - "Topology 4"
Cohesion: 0.31
Nodes (11): Session 2026-10-31 project setup and v3 simulation, Session: updatecsv.py dissection, Orphan branch lqosync3, RouterOS listen command (untested), LibreQoS v3 ingress facts, topology_import.json (internal, do not write), v3 offline simulation method, Home (vault map of content) (+3 more)

### Community 30 - "Status and Diff 3"
Cohesion: 0.21
Nodes (12): Next action: manual shadow run on production shaper, LibreQoS v3 checks (no_parent, compile_mode), [network] mode (grouped_router_root, flat_promotion, no_parent, ...), Node types (v2_type, v3_type), Checks after installation, Command reference, lqosync3 check, lqosync3 defaults (+4 more)

### Community 31 - "Topology 5"
Cohesion: 0.23
Nodes (12): LibreQoS v3 ingress document, Last-good topology retention, liblqos_python.so binding, libreqos_3.0.7 beta package, libreqos_network.json schema 2, lqosd (v3 control plane), lqosd inotify file watcher, Offline v3 simulation (+4 more)

### Community 32 - "BandwidthDefaults"
Cohesion: 0.21
Nodes (8): BandwidthDefaults, dhcp_server_gives_override_or_default(), DhcpCfg, DhcpMode, PerPlan, PerSite, DhcpServer, HotspotCfg

### Community 33 - "Secrets"
Cohesion: 0.17
Nodes (12): DhcpSource, LeaseComment, ServerComment, Settings, PppSource, ActiveComment, Default, ProfileComment (+4 more)

### Community 34 - "LibreQoS v3"
Cohesion: 0.24
Nodes (8): Columns, V2, V3, CsvRow, csv_header_sort_and_quoting(), field(), render_csv(), row()

### Community 35 - "Decisions"
Cohesion: 0.20
Nodes (11): libreqos.target (auto | v2_pair | v3_pair | v3_network), libreqos-v3-preflight, LibreQoS.py --updateonly (v2 reload), LibreQoS upgrade from 2.2 to 3.x, lqosd (LibreQoS v3 daemon), Operations (docs), Rollback to the Python service, Backups the tool keeps (applied/, keep_backups) (+3 more)

### Community 36 - "CLI and Checks 3"
Cohesion: 0.20
Nodes (9): BackstopCap, BackstopCaps, ClientTemplate, Config, Exclude, Network, Nodes, Safety (+1 more)

### Community 37 - "Speed Rules 4"
Cohesion: 0.25
Nodes (7): ADR-0009 DHCP discovery and exclusions, Speed Rules, Task 2: Rate format and speed rules (SpeedParser), Task 4: Circuit collection (collect_router), Circuit rules for each source, DhcpCfg::server(name) defaults, Output format (round3, fmt_rate, 14/15 CSV columns)

### Community 38 - "Status and Diff 4"
Cohesion: 0.31
Nodes (10): first_forty(), guard_cfg(), guard_holds_a_mass_removal_and_keeps_other_changes(), guard_off_removes_at_once(), guard_releases_on_force(), guard_releases_on_same_fingerprint_next_cycle(), guard_restarts_on_a_different_fingerprint(), hundred() (+2 more)

### Community 39 - "Topology 6"
Cohesion: 0.36
Nodes (6): libreqos_network.json schema_version 2, [topology] source setting, LQoSync3 README, Output target v2_pair, Output target v3_network, Output target v3_pair

### Community 40 - "Plan 2 Runtime"
Cohesion: 0.22
Nodes (9): LogLevel, Debug, Error, Info, Warn, Runtime, RuntimeMode, Apply (+1 more)

### Community 41 - "Session Notes 2"
Cohesion: 0.29
Nodes (5): OfflineAction, Keep, Remove, RouterAccess, FakeSession

### Community 42 - "Router State 2"
Cohesion: 0.33
Nodes (5): RouterOS listen change detection (future), [router_access] timeouts, retries, offline_after, Router state (online | failed | unreachable | offline), Subscriber presence and subscriber_grace, routerstate::classify

### Community 43 - "Status and Diff 5"
Cohesion: 0.53
Nodes (4): ADR-0008 Terminal UI, TUI config editor (toml_edit), SIGUSR1 sync now / SIGHUP reload, status.json state file

### Community 44 - ".default()"
Cohesion: 0.47
Nodes (3): deep_merge(), defaults(), defaults_table()

### Community 45 - "LibreQoS v3 2"
Cohesion: 0.33
Nodes (6): Libreqos, Target, Auto, V2Pair, V3Network, V3Pair

## Ambiguous Edges - Review These
- `Stable node id (lq3- FNV-1a 64)` → `Scenario tests (replace golden parity tests)`  [AMBIGUOUS]
  brain/20-decisions/ADR-0007-fresh-design-not-a-port.md · relation: conceptually_related_to
- `Collector (RouterOS API reads)` → `RouterOS listen change detection (future)`  [AMBIGUOUS]
  docs/ARCHITECTURE.md · relation: conceptually_related_to
- `ADR-0001 One model, many writers` → `Operational rules`  [AMBIGUOUS]
  docs/OPERATIONS.md · relation: conceptually_related_to

## Knowledge Gaps
- **83 isolated node(s):** `Dhcp`, `Hotspot`, `Ppp`, `Unknown`, `Check` (+78 more)
  These have ≤1 connection - possible missing edges or undocumented components. (Counts symbols only; 192 node(s) total have ≤1 connection when file, concept and rationale nodes are included.)
- **8 thin communities (<3 nodes) omitted from report** — run `graphify query` to explore isolated nodes.

## Suggested Questions
_Questions this graph is uniquely positioned to answer:_

- **Why does `Config` connect `CLI and Checks 3` to `BandwidthDefaults`, `Secrets`, `MikroTik Session`, `CLI and Checks`, `Circuit Collection`, `CLI and Checks 2`, `Status and Diff 4`, `Topology`, `Plan 2 Runtime`, `Config`, `Session Notes 2`, `.default()`, `LibreQoS v3 2`, `MikroTik Session 3`, `Status and Diff 2`, `fs`, `Config 3`?**
  _High betweenness centrality (0.109) - this node is a cross-community bridge._
- **Are the 10 inferred relationships involving `read_router()` (e.g. with `.cycle_inner()` and `closed_port_is_unreachable()`) actually correct?**
  _`read_router()` has 10 INFERRED edges - model-reasoned connections that need verification._
- **What connects `Dhcp`, `Hotspot`, `Ppp` to the rest of the system?**
  _83 weakly-connected nodes found - possible documentation gaps or missing edges._
- **Should `MikroTik Session` be split into smaller, more focused modules?**
  _Cohesion score 0.06522522522522523 - nodes in this community are weakly interconnected._
- **Why does `RouterCfg` connect `MikroTik Session 3` to `BandwidthDefaults`, `MikroTik Session`, `Circuit Collection`, `CLI and Checks 3`, `Mock Router Tests`, `Config`, `Session Notes 2`, `Session Notes`, `Router Reads`?**
  _High betweenness centrality (0.039) - this node is a cross-community bridge._

### Low-confidence Hints
_AMBIGUOUS edges — the extractor was unsure. Verify before acting on these._

- **What is the exact relationship between `Stable node id (lq3- FNV-1a 64)` and `Scenario tests (replace golden parity tests)`?**
  _Edge tagged AMBIGUOUS (relation: conceptually_related_to) - confidence is low._
- **What is the exact relationship between `Collector (RouterOS API reads)` and `RouterOS listen change detection (future)`?**
  _Edge tagged AMBIGUOUS (relation: conceptually_related_to) - confidence is low._