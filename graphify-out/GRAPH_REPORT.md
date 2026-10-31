# Graph Report - LQoSync3.0  (2026-10-31)

## Corpus Check
- 54 files · ~49,747 words
- Verdict: corpus is large enough that graph structure adds value.
- Unclassified: 4 file(s) not represented in the graph (top: (none) 2, .toml 1, .service 1)

## Summary
- 906 nodes · 2316 edges · 50 communities (40 shown, 10 thin omitted)
- Extraction: 94% EXTRACTED · 6% INFERRED · 0% AMBIGUOUS · INFERRED: 136 edges (avg confidence: 0.88)
- Token cost: 0 input · 0 output

## Graph Freshness
- Built from commit: `b6b34b65`
- Run `git rev-parse HEAD` and compare to check if the graph is stale.
- Run `graphify update .` after code changes (no API cost).

## Community Hubs (Navigation)
- Cycle Assembly
- Runner Cycle
- Topology
- Circuit Collection
- cli.rs
- Secrets
- mock_router.rs
- CLI and Checks 2
- Config
- hashmap
- Topology 2
- Topology 3
- File Safety
- MikroTik Session
- secrets.rs
- Topology 4
- Speed Rules
- Router Reads 2
- mikrotik.rs
- Speed Rules 2
- scenarios.rs
- CircuitNames
- Command
- Config 2
- Status and Diff
- Router State
- DhcpServer
- CertificateDer
- Status and Diff 2
- MikroTik Session 3
- MikroTik Session 4
- Status and Diff 3
- LibreQoS v3 2
- BackstopCap
- Status and Diff 4
- MikroTik Session 5
- .default()
- LibreQoS v3 3
- Router Reads 3
- LogLevel
- CLI and Checks 4
- config/defaults.toml embedded defaults
- check.sh
- Notifier (Telegram)
- lqosync3
- model.rs
- State

## God Nodes (most connected - your core abstractions)
1. `Config` - 61 edges
2. `lqosync3 Runtime Implementation Plan (Plan 2)` - 38 edges
3. `Configuration` - 37 edges
4. `lqosync3 core design spec` - 34 edges
5. `Handoff` - 34 edges
6. `run()` - 31 edges
7. `CsvRow` - 30 edges
8. `read_router()` - 26 edges
9. `RouterCfg` - 25 edges
10. `ReadError` - 25 edges

## Surprising Connections (you probably didn't know these)
- `v3 offline simulation method` --semantically_similar_to--> `Offline v3 simulation`  [INFERRED] [semantically similar]
  brain/30-knowledge/v3-offline-simulation-method.md → docs/LIBREQOS-V3-INGRESS.md
- `v3 offline simulation method` --semantically_similar_to--> `/run/lqos/bus 0777 safety caution`  [INFERRED] [semantically similar]
  brain/30-knowledge/v3-offline-simulation-method.md → docs/LIBREQOS-V3-INGRESS.md
- `Session 2026-10-31 project setup and v3 simulation` --references--> `libreqos-v3-preflight`  [INFERRED]
  brain/10-sessions/2026-10-31-project-setup-and-v3-simulation.md → docs/LIBREQOS-V3-INGRESS.md
- `ADR-0002 v3 target selection` --references--> `topology_import.json (internal)`  [INFERRED]
  brain/20-decisions/ADR-0002-v3-target-selection.md → docs/LIBREQOS-V3-INGRESS.md
- `Operations` --conceptually_related_to--> `Project phase: Plan 1 core and Plan 2 runtime done`  [AMBIGUOUS]
  docs/OPERATIONS.md → brain/HANDOFF.md

## Import Cycles
- 2-file cycle: `src/config.rs -> src/speed.rs -> src/config.rs`

## Hyperedges (group relationships)
- **Router-to-LibreQoS cycle data flow** — docs_architecture_collector, docs_architecture_circuit_model, docs_architecture_validator, docs_architecture_target_detector, docs_architecture_writers, docs_architecture_applier [EXTRACTED 1.00]
- **Plan 1 core tasks building assemble()** — docs_superpowers_plans_2026_10_31_lqosync3_core_task1_config, docs_superpowers_plans_2026_10_31_lqosync3_core_task2_speed, docs_superpowers_plans_2026_10_31_lqosync3_core_task3_state, docs_superpowers_plans_2026_10_31_lqosync3_core_task4_sources, docs_superpowers_plans_2026_10_31_lqosync3_core_task5_topology, docs_superpowers_plans_2026_10_31_lqosync3_core_task6_cycle, docs_superpowers_plans_2026_10_31_lqosync3_core_task7_writer, docs_superpowers_plans_2026_10_31_lqosync3_core_task8_scenarios [EXTRACTED 1.00]
- **Output target selection** — readme_target_auto, readme_v2_pair, readme_v3_network, readme_v3_pair, brain_30_knowledge_libreqos_v3_ingress_facts_topology_source [EXTRACTED 1.00]
- **Removal guard hold-and-release flow** — docs_configuration_removal_guard, docs_superpowers_specs_2026_10_31_lqosync3_core_design_cycle_assembly_rules, docs_superpowers_specs_2026_10_31_lqosync3_core_design_guard_fingerprint, docs_superpowers_specs_2026_10_31_lqosync3_core_design_state_file [EXTRACTED 1.00]
- **Router failure vs offline handling** — brain_20_decisions_adr_0003_router_failed_vs_offline_router_state_model, brain_20_decisions_adr_0003_router_failed_vs_offline_offline_after, brain_20_decisions_adr_0003_router_failed_vs_offline_offline_action, brain_30_knowledge_updatecsv_py_dissection_defects_to_fix [EXTRACTED 1.00]
- **Runner cycle flow: lock, read routers, classify, write, reload, status** — docs_superpowers_plans_2026_10_31_lqosync3_runtime_runner_cycle, docs_superpowers_plans_2026_10_31_lqosync3_runtime_files_lock, docs_superpowers_plans_2026_10_31_lqosync3_runtime_load_state, docs_superpowers_plans_2026_10_31_lqosync3_runtime_read_snapshot_read_router, docs_superpowers_plans_2026_10_31_lqosync3_runtime_routerstate_classify, docs_superpowers_plans_2026_10_31_lqosync3_runtime_write_if_changed, docs_superpowers_plans_2026_10_31_lqosync3_runtime_files_backup_keep_backups, docs_superpowers_plans_2026_10_31_lqosync3_runtime_run_reload, docs_superpowers_plans_2026_10_31_lqosync3_runtime_status_module [EXTRACTED 1.00]
- **Crash-safe file writes** — docs_superpowers_specs_2026_10_31_lqosync3_runtime_design_instance_lock, docs_superpowers_specs_2026_10_31_lqosync3_runtime_design_write_order_state_first, docs_superpowers_specs_2026_10_31_lqosync3_runtime_design_atomic_write, docs_superpowers_plans_2026_10_31_lqosync3_runtime_files_backup_keep_backups, docs_superpowers_specs_2026_10_31_lqosync3_runtime_design_first_run_seed [INFERRED 0.85]
- **Shadow-to-apply cutover workflow** — docs_configuration_runtime_modes, brain_handoff_manual_shadow_run, docs_operations_cutover_from_python, docs_operations_rollback_to_python, docs_operations_applied_backups [INFERRED 0.85]
- **LibreQoS v3 node rule compliance** — brain_20_decisions_adr_0005_topology_follows_v3_stable_node_id, brain_20_decisions_adr_0005_topology_follows_v3_parent_node_id, brain_20_decisions_adr_0005_topology_follows_v3_csv_column_count, brain_20_decisions_adr_0005_topology_follows_v3_max_floor_mbps, brain_20_decisions_adr_0005_topology_follows_v3_flat_promotion_parent_fix, brain_20_decisions_adr_0006_v3_guards_and_node_types_no_parent_guard, brain_20_decisions_adr_0006_v3_guards_and_node_types_site_group_nodes, brain_20_decisions_adr_0006_v3_guards_and_node_types_compile_mode_warning [INFERRED 0.85]
- **DHCP discovery and subscriber exclusion** — brain_20_decisions_adr_0009_dhcp_discovery_and_exclusions_dhcp_server_discovery, brain_20_decisions_adr_0009_dhcp_discovery_and_exclusions_exclusions, docs_configuration_exclusions, docs_superpowers_specs_2026_10_31_lqosync3_core_design_circuit_rules [INFERRED 0.95]

## Communities (50 total, 10 thin omitted)

### Community 0 - "Cycle Assembly"
Cohesion: 0.09
Nodes (62): a_row_that_stays_keeps_its_group_node(), assemble(), cfg(), changed_columns(), CycleInput, CycleOutput, duplicate_ip_existing_vs_existing_keeps_collected(), duplicate_ip_new_vs_existing_keeps_existing() (+54 more)

### Community 1 - "Runner Cycle"
Cohesion: 0.10
Nodes (35): Runtime, RuntimeMode, Apply, Shadow, a_config_reload_error_shows_in_status_until_a_good_reload(), a_held_lock_stops_the_cycle_without_touching_status(), a_panicking_router_task_fails_only_that_router(), apply_backs_up_writes_and_reloads() (+27 more)

### Community 2 - "Topology"
Cohesion: 0.11
Nodes (47): StoredNode, GroupKey, DhcpPlan, DhcpSite, Hotspot, Ppp, PppPlan, a_kept_child_keeps_its_operator_children() (+39 more)

### Community 3 - "Circuit Collection"
Cohesion: 0.10
Nodes (48): Config, DhcpSource, LeaseComment, ServerComment, Settings, Exclude, parse_ipv4_net(), RouterCfg (+40 more)

### Community 4 - "cli.rs"
Cohesion: 0.18
Nodes (18): check(), check_finds_plain_api_and_no_source_router(), check_rejects_v3_target_for_now(), check_reports_a_bad_secret_and_a_missing_lqos_dir(), CheckLine, DEFAULT_CONFIG, Env, env() (+10 more)

### Community 5 - "Secrets"
Cohesion: 0.08
Nodes (29): PppSource, ActiveComment, Default, ProfileComment, ProfileName, ProfileRateLimit, SecretComment, RateLimitOrder (+21 more)

### Community 6 - "mock_router.rs"
Cohesion: 0.05
Nodes (53): MikrotikConnector, MikrotikSession, tls_client_config(), a_dead_kept_session_reconnects_without_a_retry(), a_login_error_is_not_retried(), Connector, Fake, FakeSession (+45 more)

### Community 7 - "CLI and Checks 2"
Cohesion: 0.20
Nodes (17): Session 2026-10-31 project setup and v3 simulation, Session: updatecsv.py dissection, ADR-0003 Router failed vs offline, LibreQoS v3 ingress facts, topology_import.json (internal, do not write), v3 offline simulation method, Handoff, Naming: branch, crate, binary, service lqosync3 (+9 more)

### Community 8 - "Config"
Cohesion: 0.10
Nodes (19): a_circuit_comment_must_not_be_the_static_comment(), DEFAULTS_TOML, exclusions_parse_and_bad_ip_is_rejected(), default_from_file!, MIN, minimal_config_gets_documented_defaults(), node_type_defaults_follow_target(), node_types_must_not_be_empty() (+11 more)

### Community 9 - "hashmap"
Cohesion: 0.15
Nodes (21): Network, NetworkMode, FlatGlobalRoot, FlatPromotion, FlatRouterRoot, GroupedRouterRoot, NoParent, RootPromotion (+13 more)

### Community 10 - "Topology 2"
Cohesion: 0.14
Nodes (20): ADR-0005 Topology follows v3, 14/15 CSV columns (v2_pair/v3_pair), max_floor_mbps (0.2) / safe_min_mbps (0.1), Parent Node ID column, ADR-0006 v3 guards and node types, compile_mode not full warning, Tool must reject duplicate IP and node name, How LibreQoS v3 handles nodes (+12 more)

### Community 11 - "Topology 3"
Cohesion: 0.17
Nodes (14): libreqos.target (auto, v2_pair, v3_pair, v3_network), LibreQoS v3 ingress document, Last-good topology retention, liblqos_python.so binding, libreqos_3.0.7 beta package, libreqos_network.json schema 2, libreqos-v3-preflight, lqosd (v3 control plane) (+6 more)

### Community 12 - "File Safety"
Cohesion: 0.14
Nodes (13): backup(), backup_keeps_the_last_n(), backup_keeps_the_new_folder_and_other_folders(), HDR, Live, load_state_seeds_on_first_run_and_reads_after(), lock(), read_live() (+5 more)

### Community 13 - "MikroTik Session"
Cohesion: 0.12
Nodes (19): Applier (hash, atomic write, reload), Circuit Model, Collector (RouterOS API reads), Config Loader, Proposed Crate Layout, libreqos_network.json, lqosd (LibreQoS daemon), network.json (+11 more)

### Community 14 - "secrets.rs"
Cohesion: 0.22
Nodes (7): error_never_contains_the_secret(), file(), missing_file_names_the_path(), read_secret(), reads_and_trims_a_good_secret(), rejects_group_or_other_bits(), rejects_wrong_owner()

### Community 15 - "Topology 4"
Cohesion: 0.17
Nodes (18): 2026-10-31 Plan 1: lqosync3 core library session, lqosync3 core crate (config, speed, model, state, sources, topology, cycle, validate, writer), Plan 1 review fixes (commit 529fa36), Project phase: Plan 1 core and Plan 2 runtime done, lqosync3 Core Implementation Plan (Plan 1), Plan 1 review focus (edge cases), Task 3: Model, state file, and seed, Task 6: Cycle assembly and validation (+10 more)

### Community 16 - "Speed Rules"
Cohesion: 0.14
Nodes (14): ADR-0009 DHCP discovery and exclusions, Removal guard on (>20% or >50 rows held), RouterOS listen change detection (future), Speed Rules, DHCP server discovery (/ip/dhcp-server, discover), Exclusions ([exclude], [routers.exclude]), Subscriber presence and subscriber_grace, Task 2: Rate format and speed rules (SpeedParser) (+6 more)

### Community 17 - "Router Reads 2"
Cohesion: 0.17
Nodes (18): Deferred minor items from Plan 2 review, Manual shadow run on production shaper (next action), lqosync3 Runtime Implementation Plan (Plan 2), files::lock (try_lock, 'another lqosync3 is running'), files::load_state (state.json or seed), Task 9 Step 5 manual shadow read (operator approval), CycleSummary::next_sleep (error_retry / active / idle), Plan 2b: Telegram (+10 more)

### Community 19 - "Speed Rules 2"
Cohesion: 0.16
Nodes (17): offline_action (default remove), offline_after (default 300 s), ADR-0004 TOML policy, Python defaults, Speed source order TOML lists, ADR-0007 Fresh design, not a port, Scenario tests (replace golden parity tests), Semantic compare with live files before cutover, updatecsv.py dissection (+9 more)

### Community 20 - "scenarios.rs"
Cohesion: 0.30
Nodes (8): split_static(), render_network_json(), cycle(), read(), report_json(), results(), scenarios_match_expected_output(), Snapshot

### Community 21 - "CircuitNames"
Cohesion: 0.22
Nodes (13): CircuitNames, load_state(), HDR, read_csv(), read_csv_accepts_bom_crlf_and_missing_columns(), read_csv_skips_blank_names_and_keeps_repeated_names(), seed_infers_source_and_keeps_ids(), seed_nodes() (+5 more)

### Community 22 - "Command"
Cohesion: 0.20
Nodes (10): Args, Command, Check, Defaults, Diff, Once, Run, parse() (+2 more)

### Community 23 - "Config 2"
Cohesion: 0.15
Nodes (15): Repository branch lqosync3 (no shared history), Production state: Python lqosync-updatecsv.service and LibreQoS 2.2 unchanged, Map from config.json and planned import-json, Runtime modes (shadow, apply), Installation, One-line installer install.sh (target contract), Installer options (--mode, --version, --from-source, --import-json, --no-start), Manual installation (+7 more)

### Community 24 - "Status and Diff"
Cohesion: 0.19
Nodes (6): exit_with(), main(), now(), print_summary(), run(), SECRET_OWNER_UID

### Community 25 - "Router State"
Cohesion: 0.19
Nodes (22): OfflineAction, Keep, Remove, RouterAccess, ppp(), ppp_owned(), RouterResult, Failed (+14 more)

### Community 26 - "DhcpServer"
Cohesion: 0.21
Nodes (8): BandwidthDefaults, dhcp_server_gives_override_or_default(), DhcpCfg, DhcpMode, PerPlan, PerSite, DhcpServer, HotspotCfg

### Community 28 - "Status and Diff 2"
Cohesion: 0.21
Nodes (13): diff_config(), diff_dirs(), diff_dirs_needs_candidate_files(), diff_ignores_column_order_row_order_and_number_text(), diff_pair(), diff_reports_added_removed_changed(), diff_stops_in_apply_mode(), group() (+5 more)

### Community 29 - "MikroTik Session 3"
Cohesion: 0.23
Nodes (10): Orphan branch lqosync3, RouterOS listen command (untested), libreqos_network.json schema_version 2, [topology] source setting, LQoSync3 README, LQoSync3 (lqosync3 tool), mikrotik-rs RouterOS API crate, Output target v2_pair (+2 more)

### Community 30 - "MikroTik Session 4"
Cohesion: 0.24
Nodes (11): Router library mikrotik-rs =0.9.0 in this project only, Mock RouterOS server tests (tests/mock_router.rs), lqosync3 runtime design (Plan 2), Atomic write (temp file, sync_all, rename, fsync folder; only on byte change), Files: lock, state.json, status.json, candidate, applied, First-run seed (seed_state, seed_nodes from live files), Instance lock <state>/lqosync3.lock (File::try_lock), mikrotik-tokio 0.1.0 16-reply burst cancel bug (+3 more)

### Community 31 - "Status and Diff 3"
Cohesion: 0.18
Nodes (12): Operations, Backups that the tool keeps (last 10 in /var/lib/lqosync3/applied), Command reference (check, once, once --force, diff, run, defaults), Daily tasks (systemctl status, journalctl), lqosync3 detect (Plan 3), lqosync3 once --dry-run, Signals: SIGHUP reload, SIGUSR1 cycle now, cli::Command (check, once --force, run, diff, defaults) (+4 more)

### Community 32 - "LibreQoS v3 2"
Cohesion: 0.21
Nodes (7): Columns, V2, V3, csv_header_sort_and_quoting(), field(), render_csv(), row()

### Community 34 - "Status and Diff 4"
Cohesion: 0.31
Nodes (6): ADR-0008 Terminal UI, TUI config editor (toml_edit), SIGUSR1 sync now / SIGHUP reload, status.json state file, Plan 4: TUI, Delivery in five plans (core, runtime, v3, TUI, install)

### Community 35 - "MikroTik Session 5"
Cohesion: 0.29
Nodes (6): Troubleshooting table, Connector trait (connect -> Session), MikrotikConnector / MikrotikSession / tls_client_config, secrets::read_secret (mode & 0o077 == 0, owner uid), RouterCfg tls_verify / tls_ca_file, Secrets (password_file read each connect, never logged)

### Community 36 - ".default()"
Cohesion: 0.47
Nodes (3): deep_merge(), defaults(), defaults_table()

### Community 37 - "LibreQoS v3 3"
Cohesion: 0.33
Nodes (6): Libreqos, Target, Auto, V2Pair, V3Network, V3Pair

### Community 38 - "Router Reads 3"
Cohesion: 0.50
Nodes (4): read_snapshot / read_router (timeouts, retries, session reuse), RouterSession trait (print path proplist), TABLES const (6 paths with .proplist), Router paths and .proplist fields table

### Community 39 - "LogLevel"
Cohesion: 0.40
Nodes (5): LogLevel, Debug, Error, Info, Warn

### Community 40 - "CLI and Checks 4"
Cohesion: 0.50
Nodes (3): ClientTemplate, Nodes, NodeTemplate

### Community 41 - "config/defaults.toml embedded defaults"
Cohesion: 0.25
Nodes (7): Deferred minor items from defaults review, config/defaults.toml embedded defaults, /etc/lqosync3/lqosync3.toml config file, [router_defaults] per-router defaults, Task 1: Crate scaffold and config, Config::from_toml_str (defaults merge), TLS (tls_verify, tls_ca_file, webpki-roots)

### Community 48 - "model.rs"
Cohesion: 0.29
Nodes (4): GuardHold, State, V2_COLUMNS, V3_COLUMNS

## Ambiguous Edges - Review These
- `read_snapshot / read_router (timeouts, retries, session reuse)` → `Router state classification (RouterResult, unreachable_since)`  [AMBIGUOUS]
  docs/superpowers/plans/2026-10-31-lqosync3-runtime.md · relation: conceptually_related_to
- `lqosync3 once --dry-run` → `cli::Command (check, once --force, run, diff, defaults)`  [AMBIGUOUS]
  docs/OPERATIONS.md · relation: conceptually_related_to
- `Requirements (Ubuntu 24.04, LibreQoS 2.0-2.2 / 3.x, Rust 1.88+)` → `rust-version 1.89 (std::fs::File::try_lock)`  [AMBIGUOUS]
  docs/INSTALLATION.md · relation: conceptually_related_to
- `Stable node id (lq3- FNV-1a 64)` → `Scenario tests (replace golden parity tests)`  [AMBIGUOUS]
  brain/20-decisions/ADR-0007-fresh-design-not-a-port.md · relation: conceptually_related_to
- `Collector (RouterOS API reads)` → `RouterOS listen change detection (future)`  [AMBIGUOUS]
  docs/ARCHITECTURE.md · relation: conceptually_related_to
- `Project phase: Plan 1 core and Plan 2 runtime done` → `Operations`  [AMBIGUOUS]
  docs/OPERATIONS.md · relation: conceptually_related_to
- `Deferred minor items from Plan 2 review` → `Write order: state.json first, then LibreQoS pair`  [AMBIGUOUS]
  brain/HANDOFF.md · relation: conceptually_related_to

## Knowledge Gaps
- **79 isolated node(s):** `lqosync3`, `Check`, `Once`, `Run`, `Diff` (+74 more)
  These have ≤1 connection - possible missing edges or undocumented components. (Counts symbols only; 177 node(s) total have ≤1 connection when file, concept and rationale nodes are included.)
- **10 thin communities (<3 nodes) omitted from report** — run `graphify query` to explore isolated nodes.

## Suggested Questions
_Questions this graph is uniquely positioned to answer:_

- **Why does `Config` connect `Circuit Collection` to `Cycle Assembly`, `BackstopCap`, `Runner Cycle`, `Topology`, `cli.rs`, `.default()`, `LibreQoS v3 3`, `Secrets`, `Config`, `hashmap`, `CLI and Checks 4`, `scenarios.rs`, `CircuitNames`, `Status and Diff`, `Router State`, `DhcpServer`, `Status and Diff 2`?**
  _High betweenness centrality (0.101) - this node is a cross-community bridge._
- **Are the 2 inferred relationships involving `lqosync3 Runtime Implementation Plan (Plan 2)` (e.g. with `Project phase: Plan 1 core and Plan 2 runtime done` and `lqosync3 Core Implementation Plan (Plan 1)`) actually correct?**
  _`lqosync3 Runtime Implementation Plan (Plan 2)` has 2 INFERRED edges - model-reasoned connections that need verification._
- **What connects `lqosync3`, `Check`, `Once` to the rest of the system?**
  _79 weakly-connected nodes found - possible documentation gaps or missing edges._
- **Should `Cycle Assembly` be split into smaller, more focused modules?**
  _Cohesion score 0.09038461538461538 - nodes in this community are weakly interconnected._
- **Why does `RouterCfg` connect `Circuit Collection` to `Runner Cycle`, `mock_router.rs`, `Config`, `mikrotik.rs`, `DhcpServer`?**
  _High betweenness centrality (0.034) - this node is a cross-community bridge._

### Low-confidence Hints
_AMBIGUOUS edges — the extractor was unsure. Verify before acting on these._

- **What is the exact relationship between `read_snapshot / read_router (timeouts, retries, session reuse)` and `Router state classification (RouterResult, unreachable_since)`?**
  _Edge tagged AMBIGUOUS (relation: conceptually_related_to) - confidence is low._
- **What is the exact relationship between `lqosync3 once --dry-run` and `cli::Command (check, once --force, run, diff, defaults)`?**
  _Edge tagged AMBIGUOUS (relation: conceptually_related_to) - confidence is low._