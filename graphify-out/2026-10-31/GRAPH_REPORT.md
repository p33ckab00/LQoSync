# Graph Report - LQoSync3.0  (2026-10-31)

## Corpus Check
- 58 files · ~55,339 words
- Verdict: corpus is large enough that graph structure adds value.
- Unclassified: 4 file(s) not represented in the graph (top: (none) 2, .toml 1, .service 1)

## Summary
- 977 nodes · 2441 edges · 42 communities (33 shown, 9 thin omitted)
- Extraction: 94% EXTRACTED · 6% INFERRED · 0% AMBIGUOUS · INFERRED: 135 edges (avg confidence: 0.88)
- Token cost: 0 input · 0 output

## Graph Freshness
- Built from commit: `c80596cc`
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
- Topology 3
- mikrotik.rs
- Configuration reference
- Status and Diff
- runner::Runner::cycle (one full cycle)
- read_router
- MikroTik Session 5
- Speed Rules 2
- Topology 4
- PppSource
- CertificateDer
- Status and Diff 2
- Speed Rules 3
- Config 3
- dhcp_server_gives_override_or_
- Router State 2
- lqosync3 core design spec
- LibreQoS v3 2
- LogLevel
- OfflineAction
- State
- check.sh
- Telegram Notifier 4
- CLI and Checks 4
- lqosync3

## God Nodes (most connected - your core abstractions)
1. `Config` - 62 edges
2. `CsvRow` - 33 edges
3. `lqosync3 core design spec` - 32 edges
4. `run()` - 31 edges
5. `Configuration reference` - 31 edges
6. `Handoff` - 28 edges
7. `read_router()` - 26 edges
8. `RouterCfg` - 25 edges
9. `ReadError` - 25 edges
10. `Rate` - 24 edges

## Surprising Connections (you probably didn't know these)
- `libreqos_network.json schema_version 2` --references--> `v3_network output target (libreqos_network.json)`  [INFERRED]
  brain/30-knowledge/libreqos-v3-ingress-facts.md → README.md
- `Safety rules (shadow default, no router writes, 0600 secrets)` --semantically_similar_to--> `Operational rules`  [INFERRED] [semantically similar]
  README.md → docs/OPERATIONS.md
- `v3 offline simulation method` --semantically_similar_to--> `Offline v3 simulation`  [INFERRED] [semantically similar]
  brain/30-knowledge/v3-offline-simulation-method.md → docs/LIBREQOS-V3-INGRESS.md
- `v3 offline simulation method` --semantically_similar_to--> `/run/lqos/bus 0777 safety caution`  [INFERRED] [semantically similar]
  brain/30-knowledge/v3-offline-simulation-method.md → docs/LIBREQOS-V3-INGRESS.md
- `Task 2: Events and message` --references--> `split()`  [EXTRACTED]
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

## Communities (42 total, 9 thin omitted)

### Community 0 - "mock_router.rs"
Cohesion: 0.14
Nodes (24): Answer, Empty, Raw, Rows, Silent, Trap, closed_port_is_unreachable(), empty_table_is_ok() (+16 more)

### Community 1 - "Telegram Notifier"
Cohesion: 0.07
Nodes (47): Task 2: Events and message, Task 3: Transport (post_json, send, token, chat_id), Telegram, Change, Report, a_config_error_is_an_error_event(), a_long_multi_line_error_splits_into_valid_html_parts(), a_very_long_error_is_cut() (+39 more)

### Community 2 - "Router State"
Cohesion: 0.09
Nodes (34): exit_with(), main(), notify(), now(), print_summary(), run(), SECRET_OWNER_UID, a_config_reload_error_shows_in_status_until_a_good_reload() (+26 more)

### Community 3 - "Topology"
Cohesion: 0.12
Nodes (45): NodeTemplate, StoredNode, GroupKey, Hotspot, Ppp, a_kept_child_keeps_its_operator_children(), backstop_cap_limits_group(), build_nodes() (+37 more)

### Community 4 - "Circuit Collection"
Cohesion: 0.11
Nodes (45): parse_ipv4_net(), PppoeCfg, RouterCfg, RosRow, RouterSnapshot, read_snapshot(), a_zero_speed_profile_comment_gives_one_line(), by_name() (+37 more)

### Community 5 - "Telegram Notifier 2"
Cohesion: 0.08
Nodes (36): Args, check(), check_finds_plain_api_and_no_source_router(), check_reads_the_telegram_token_when_enabled(), check_rejects_v3_target_for_now(), check_reports_a_bad_secret_and_a_missing_lqos_dir(), CheckLine, Command (+28 more)

### Community 6 - "Secrets"
Cohesion: 0.09
Nodes (23): RateLimitOrder, DownloadUpload, RouterosRxTx, UnitlessUnit, Bps, Mbps, comment_rate(), plan_label() (+15 more)

### Community 7 - "cycle.rs"
Cohesion: 0.06
Nodes (84): a_row_that_stays_keeps_its_group_node(), assemble(), cfg(), changed_columns(), CycleInput, CycleOutput, duplicate_ip_existing_vs_existing_keeps_collected(), duplicate_ip_new_vs_existing_keeps_existing() (+76 more)

### Community 8 - "File Safety"
Cohesion: 0.09
Nodes (27): CircuitNames, backup(), backup_keeps_the_last_n(), backup_keeps_the_new_folder_and_other_folders(), HDR, Live, load_state(), load_state_seeds_on_first_run_and_reads_after() (+19 more)

### Community 9 - "routeros.rs"
Cohesion: 0.14
Nodes (17): a_login_error_is_not_retried(), Connector, Fake, FakeSession, gives_last_error_after_all_retries(), ppp_router(), Login, Trap (+9 more)

### Community 10 - "Config"
Cohesion: 0.11
Nodes (19): a_circuit_comment_must_not_be_the_static_comment(), ClientTemplate, DEFAULTS_TOML, exclusions_parse_and_bad_ip_is_rejected(), default_from_file!, MIN, minimal_config_gets_documented_defaults(), node_type_defaults_follow_target() (+11 more)

### Community 11 - "hashmap"
Cohesion: 0.16
Nodes (20): NetworkMode, FlatGlobalRoot, FlatPromotion, FlatRouterRoot, GroupedRouterRoot, NoParent, RootPromotion, check() (+12 more)

### Community 12 - "Session Notes"
Cohesion: 0.09
Nodes (27): Orphan branch lqosync3, RouterOS listen command (untested), 14/15 CSV columns (v2_pair/v3_pair), Plan phases (Plan 1/2/2b done; Plan 3 v3, Plan 4 TUI, Plan 5 installer), Runtime modes (shadow, apply), install.sh one-line installer (target contract), Installation (docs), Installer options (--mode, --version, --from-source, --import-json, --no-start) (+19 more)

### Community 13 - "Topology 2"
Cohesion: 0.16
Nodes (19): Session 2026-10-31 project setup and v3 simulation, LibreQoS v3 ingress facts, libreqos_network.json schema_version 2, topology_import.json (internal, do not write), [topology] source setting, v3 offline simulation method, LibreQoS v3 ingress document, Last-good topology retention (+11 more)

### Community 14 - "Topology 3"
Cohesion: 0.13
Nodes (18): RouterOS listen change detection (future), Six Network Modes, Topology Builder, LibreQoS v3 topology.compile_mode, LibreQoS v3 checks (no_parent, compile_mode), Node types per target (v2_type, v3_type), Subscriber presence and subscriber_grace, Troubleshooting table (+10 more)

### Community 15 - "mikrotik.rs"
Cohesion: 0.15
Nodes (5): map_err(), MikrotikConnector, MikrotikSession, tls_client_config(), Setup

### Community 16 - "Configuration reference"
Cohesion: 0.07
Nodes (26): config/defaults.toml embedded defaults, Plan 2b deferred minor items, Configuration reference, Comment speed format (Name|25M/5M), Map from config.json, DHCP server discovery, Exclusions ([exclude], [routers.exclude]), lqosync3 import-json (planned) (+18 more)

### Community 17 - "Status and Diff"
Cohesion: 0.13
Nodes (16): diff_config(), diff_dirs(), diff_dirs_needs_candidate_files(), diff_ignores_column_order_row_order_and_number_text(), diff_pair(), diff_reports_added_removed_changed(), diff_stops_in_apply_mode(), group() (+8 more)

### Community 18 - "runner::Runner::cycle (one full cycle)"
Cohesion: 0.05
Nodes (43): Applier (hash, atomic write, reload), Circuit Model, Collector (RouterOS API reads), Config Loader, Proposed Crate Layout, libreqos_network.json, lqosd (LibreQoS daemon), network.json (+35 more)

### Community 19 - "read_router"
Cohesion: 0.19
Nodes (7): a_dead_kept_session_reconnects_without_a_retry(), read_router(), ReadError, Protocol, Timeout, Tls, Unreachable

### Community 20 - "MikroTik Session 5"
Cohesion: 0.16
Nodes (14): lqosync3 runtime design (Plan 2), Atomic write (temp file, sync_all, rename, fsync folder; only on byte change), Files: lock, state.json, status.json, candidate, applied, First-run seed (seed_state, seed_nodes from live files), Instance lock <state>/lqosync3.lock (File::try_lock), mikrotik-tokio 0.1.0 16-reply burst cancel bug, Plan 2 scope (in / not in), Process: run loop, SIGUSR1, SIGHUP, SIGTERM/SIGINT, once, check, diff, defaults (+6 more)

### Community 21 - "Speed Rules 2"
Cohesion: 0.16
Nodes (11): offline_action (default remove), offline_after (default 300 s), Speed source order TOML lists, ADR-0008 Terminal UI, TUI config editor (toml_edit), SIGUSR1 sync now / SIGHUP reload, status.json state file, updatecsv.py defects to fix (+3 more)

### Community 22 - "Topology 4"
Cohesion: 0.24
Nodes (10): max_floor_mbps (0.2) / safe_min_mbps (0.1), ADR-0006 v3 guards and node types, compile_mode not full warning, Tool must reject duplicate IP and node name, How LibreQoS v3 handles nodes, v3 bridge boot gate, compile_mode (flat/ap_only/ap_site/full), v3 name-derived legacy node id (+2 more)

### Community 23 - "PppSource"
Cohesion: 0.17
Nodes (12): DhcpSource, LeaseComment, ServerComment, Settings, PppSource, ActiveComment, Default, ProfileComment (+4 more)

### Community 26 - "Status and Diff 2"
Cohesion: 0.21
Nodes (7): 2026-10-31 First live shadow run, FSEHub-Distro router (10.200.1.2:8728), Shadow run results (62 rows, 167 ms, 26 text-only updates), Handoff, Production state (Python live, lqosync3 shadow, LibreQoS 2.2), Differences from updatecsv.py, Task 1: Crate scaffold and config

### Community 27 - "Speed Rules 3"
Cohesion: 0.25
Nodes (6): ADR-0009 DHCP discovery and exclusions, Speed Rules, Task 2: Rate format and speed rules (SpeedParser), Task 4: Circuit collection (collect_router), Circuit rules for each source, DhcpCfg::server(name) defaults

### Community 29 - "Config 3"
Cohesion: 0.19
Nodes (9): BackstopCap, BackstopCaps, Config, deep_merge(), defaults(), defaults_table(), Exclude, Network (+1 more)

### Community 31 - "dhcp_server_gives_override_or_"
Cohesion: 0.21
Nodes (8): BandwidthDefaults, dhcp_server_gives_override_or_default(), DhcpCfg, DhcpMode, PerPlan, PerSite, DhcpServer, HotspotCfg

### Community 32 - "Router State 2"
Cohesion: 0.50
Nodes (11): RouterAccess, RouterMemo, acc(), classify(), first_unreachable_sets_since(), login_tls_trap_timeout_are_failed_and_keep_memo(), memo(), offline_after_zero_is_offline_at_once() (+3 more)

### Community 36 - "lqosync3 core design spec"
Cohesion: 0.15
Nodes (28): 2026-10-31 Plan 1: lqosync3 core library session, lqosync3 core crate (config, speed, model, state, sources, topology, cycle, validate, writer), Plan 1 review fixes (commit 529fa36), Session: updatecsv.py dissection, ADR-0003 Router failed vs offline, ADR-0004 TOML policy, Python defaults, ADR-0005 Topology follows v3, Parent Node ID column (+20 more)

### Community 40 - "LibreQoS v3 2"
Cohesion: 0.33
Nodes (6): Libreqos, Target, Auto, V2Pair, V3Network, V3Pair

### Community 41 - "LogLevel"
Cohesion: 0.22
Nodes (9): LogLevel, Debug, Error, Info, Warn, Runtime, RuntimeMode, Apply (+1 more)

### Community 43 - "OfflineAction"
Cohesion: 0.67
Nodes (3): OfflineAction, Keep, Remove

## Ambiguous Edges - Review These
- `RouterOS listen change detection (future)` → `Collector (RouterOS API reads)`  [AMBIGUOUS]
  docs/ARCHITECTURE.md · relation: conceptually_related_to
- `Scenario tests (replace golden parity tests)` → `Stable node id (lq3- FNV-1a 64)`  [AMBIGUOUS]
  brain/20-decisions/ADR-0007-fresh-design-not-a-port.md · relation: conceptually_related_to

## Knowledge Gaps
- **100 isolated node(s):** `lqosync3`, `Check`, `Once`, `Run`, `Diff` (+95 more)
  These have ≤1 connection - possible missing edges or undocumented components. (Counts symbols only; 210 node(s) total have ≤1 connection when file, concept and rationale nodes are included.)
- **9 thin communities (<3 nodes) omitted from report** — run `graphify query` to explore isolated nodes.

## Suggested Questions
_Questions this graph is uniquely positioned to answer:_

- **Why does `Task 3: Transport (post_json, send, token, chat_id)` connect `Telegram Notifier` to `Configuration reference`?**
  _High betweenness centrality (0.259) - this node is a cross-community bridge._
- **What connects `lqosync3`, `Check`, `Once` to the rest of the system?**
  _100 weakly-connected nodes found - possible documentation gaps or missing edges._
- **Should `mock_router.rs` be split into smaller, more focused modules?**
  _Cohesion score 0.1431451612903226 - nodes in this community are weakly interconnected._
- **Why does `Handoff` connect `Status and Diff 2` to `lqosync3 core design spec`, `Session Notes`, `Topology 2`, `Topology 3`, `Configuration reference`, `runner::Runner::cycle (one full cycle)`, `Speed Rules 2`, `Topology 4`?**
  _High betweenness centrality (0.192) - this node is a cross-community bridge._
- **Should `Telegram Notifier` be split into smaller, more focused modules?**
  _Cohesion score 0.0744047619047619 - nodes in this community are weakly interconnected._

### Low-confidence Hints
_AMBIGUOUS edges — the extractor was unsure. Verify before acting on these._

- **What is the exact relationship between `RouterOS listen change detection (future)` and `Collector (RouterOS API reads)`?**
  _Edge tagged AMBIGUOUS (relation: conceptually_related_to) - confidence is low._
- **What is the exact relationship between `Scenario tests (replace golden parity tests)` and `Stable node id (lq3- FNV-1a 64)`?**
  _Edge tagged AMBIGUOUS (relation: conceptually_related_to) - confidence is low._