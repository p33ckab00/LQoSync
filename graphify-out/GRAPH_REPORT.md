# Graph Report - LQoSync3.0  (2026-10-31)

## Corpus Check
- 24 files · ~48,186 words
- Verdict: corpus is large enough that graph structure adds value.

## Summary
- 917 nodes · 2253 edges · 48 communities (41 shown, 7 thin omitted)
- Extraction: 94% EXTRACTED · 6% INFERRED · 0% AMBIGUOUS · INFERRED: 133 edges (avg confidence: 0.88)
- Token cost: 91,748 input · 0 output

## Community Hubs (Navigation)
- Cycle Assembly
- Runner Cycle
- Topology
- Circuit Collection
- CLI and Checks
- Secrets
- Mock Router Tests
- CLI and Checks 2
- Config
- hashmap
- Topology 2
- Topology 3
- File Safety
- MikroTik Session
- Router Reads
- Topology 4
- Speed Rules
- Router Reads 2
- MikroTik Session 2
- Speed Rules 2
- Scenario Tests
- CircuitNames
- AtomicUsize
- Config 2
- Status and Diff
- Router State
- LibreQoS v3
- CertificateDer
- Status and Diff 2
- MikroTik Session 3
- MikroTik Session 4
- Status and Diff 3
- LibreQoS v3 2
- CLI and Checks 3
- Status and Diff 4
- MikroTik Session 5
- .default()
- LibreQoS v3 3
- Router Reads 3
- LogLevel
- CLI and Checks 4
- OfflineAction
- check.sh
- Notifier (Telegram)
- lqosync3

## God Nodes (most connected - your core abstractions)
1. `Config` - 63 edges
2. `lqosync3 Runtime Implementation Plan (Plan 2)` - 38 edges
3. `Configuration` - 37 edges
4. `lqosync3 core design spec` - 34 edges
5. `run()` - 31 edges
6. `Handoff` - 29 edges
7. `RouterCfg` - 25 edges
8. `read_router()` - 25 edges
9. `ReadError` - 24 edges
10. `Home (vault map of content)` - 23 edges

## Surprising Connections (you probably didn't know these)
- `v3 offline simulation method` --semantically_similar_to--> `Offline v3 simulation`  [INFERRED] [semantically similar]
  brain/30-knowledge/v3-offline-simulation-method.md → docs/LIBREQOS-V3-INGRESS.md
- `v3 offline simulation method` --semantically_similar_to--> `/run/lqos/bus 0777 safety caution`  [INFERRED] [semantically similar]
  brain/30-knowledge/v3-offline-simulation-method.md → docs/LIBREQOS-V3-INGRESS.md
- `Operations` --conceptually_related_to--> `Project phase: Plan 1 core and Plan 2 runtime done`  [AMBIGUOUS]
  docs/OPERATIONS.md → brain/HANDOFF.md
- `Scenario tests (replace golden parity tests)` --conceptually_related_to--> `Stable node id (lq3- FNV-1a 64)`  [AMBIGUOUS]
  brain/20-decisions/ADR-0007-fresh-design-not-a-port.md → brain/20-decisions/ADR-0005-topology-follows-v3.md
- `Scenario tests (synthetic fixtures)` --conceptually_related_to--> `ADR-0007 Fresh design, not a port`  [INFERRED]
  docs/superpowers/specs/2026-10-31-lqosync3-core-design.md → brain/20-decisions/ADR-0007-fresh-design-not-a-port.md

## Import Cycles
- 1-file cycle: `src/state.rs -> src/state.rs`

## Hyperedges (group relationships)
- **Router-to-LibreQoS cycle data flow** — docs_architecture_collector, docs_architecture_circuit_model, docs_architecture_validator, docs_architecture_target_detector, docs_architecture_writers, docs_architecture_applier [EXTRACTED 1.00]
- **Plan 1 core tasks building assemble()** — docs_superpowers_plans_2026_10_31_lqosync3_core_task1_config, docs_superpowers_plans_2026_10_31_lqosync3_core_task2_speed, docs_superpowers_plans_2026_10_31_lqosync3_core_task3_state, docs_superpowers_plans_2026_10_31_lqosync3_core_task4_sources, docs_superpowers_plans_2026_10_31_lqosync3_core_task5_topology, docs_superpowers_plans_2026_10_31_lqosync3_core_task6_cycle, docs_superpowers_plans_2026_10_31_lqosync3_core_task7_writer, docs_superpowers_plans_2026_10_31_lqosync3_core_task8_scenarios [EXTRACTED 1.00]
- **Output target selection** — readme_target_auto, readme_v2_pair, readme_v3_network, readme_v3_pair, brain_30_knowledge_libreqos_v3_ingress_facts_topology_source [EXTRACTED 1.00]
- **Removal guard hold-and-release flow** — docs_configuration_removal_guard, docs_superpowers_specs_2026_10_31_lqosync3_core_design_cycle_assembly_rules, docs_superpowers_specs_2026_10_31_lqosync3_core_design_guard_fingerprint, docs_superpowers_specs_2026_10_31_lqosync3_core_design_state_file [EXTRACTED 1.00]
- **Router failure vs offline handling** — brain_20_decisions_adr_0003_router_failed_vs_offline_router_state_model, brain_20_decisions_adr_0003_router_failed_vs_offline_offline_after, brain_20_decisions_adr_0003_router_failed_vs_offline_offline_action, brain_30_knowledge_updatecsv_py_dissection_defects_to_fix [EXTRACTED 1.00]
- **LibreQoS v3 node rule compliance** — brain_20_decisions_adr_0005_topology_follows_v3_stable_node_id, brain_20_decisions_adr_0005_topology_follows_v3_parent_node_id, brain_20_decisions_adr_0005_topology_follows_v3_csv_column_count, brain_20_decisions_adr_0005_topology_follows_v3_max_floor_mbps, brain_20_decisions_adr_0005_topology_follows_v3_flat_promotion_parent_fix, brain_20_decisions_adr_0006_v3_guards_and_node_types_no_parent_guard, brain_20_decisions_adr_0006_v3_guards_and_node_types_site_group_nodes, brain_20_decisions_adr_0006_v3_guards_and_node_types_compile_mode_warning [INFERRED 0.85]
- **DHCP discovery and subscriber exclusion** — brain_20_decisions_adr_0009_dhcp_discovery_and_exclusions_dhcp_server_discovery, brain_20_decisions_adr_0009_dhcp_discovery_and_exclusions_exclusions, docs_configuration_exclusions, docs_superpowers_specs_2026_10_31_lqosync3_core_design_circuit_rules [INFERRED 0.95]
- **Runner cycle flow: lock, read routers, classify, write, reload, status** — docs_superpowers_plans_2026_10_31_lqosync3_runtime_runner_cycle, docs_superpowers_plans_2026_10_31_lqosync3_runtime_files_lock, docs_superpowers_plans_2026_10_31_lqosync3_runtime_load_state, docs_superpowers_plans_2026_10_31_lqosync3_runtime_read_snapshot_read_router, docs_superpowers_plans_2026_10_31_lqosync3_runtime_routerstate_classify, docs_superpowers_plans_2026_10_31_lqosync3_runtime_write_if_changed, docs_superpowers_plans_2026_10_31_lqosync3_runtime_files_backup_keep_backups, docs_superpowers_plans_2026_10_31_lqosync3_runtime_run_reload, docs_superpowers_plans_2026_10_31_lqosync3_runtime_status_module [EXTRACTED 1.00]
- **Crash-safe file writes** — docs_superpowers_specs_2026_10_31_lqosync3_runtime_design_instance_lock, docs_superpowers_specs_2026_10_31_lqosync3_runtime_design_write_order_state_first, docs_superpowers_specs_2026_10_31_lqosync3_runtime_design_atomic_write, docs_superpowers_plans_2026_10_31_lqosync3_runtime_files_backup_keep_backups, docs_superpowers_specs_2026_10_31_lqosync3_runtime_design_first_run_seed [INFERRED 0.85]
- **Shadow-to-apply cutover workflow** — docs_configuration_runtime_modes, brain_handoff_manual_shadow_run, docs_operations_cutover_from_python, docs_operations_rollback_to_python, docs_operations_applied_backups [INFERRED 0.85]

## Communities (48 total, 7 thin omitted)

### Community 0 - "Cycle Assembly"
Cohesion: 0.06
Nodes (79): a_row_that_stays_keeps_its_group_node(), assemble(), cfg(), changed_columns(), CycleInput, CycleOutput, duplicate_ip_existing_vs_existing_keeps_collected(), duplicate_ip_new_vs_existing_keeps_existing() (+71 more)

### Community 1 - "Runner Cycle"
Cohesion: 0.09
Nodes (34): Runtime, RuntimeMode, Apply, Shadow, a_held_lock_stops_the_cycle_without_touching_status(), a_panicking_router_task_fails_only_that_router(), apply_backs_up_writes_and_reloads(), assemble_error_writes_nothing() (+26 more)

### Community 2 - "Topology"
Cohesion: 0.11
Nodes (47): StoredNode, GroupKey, DhcpPlan, DhcpSite, Hotspot, Ppp, PppPlan, a_kept_child_keeps_its_operator_children() (+39 more)

### Community 3 - "Circuit Collection"
Cohesion: 0.12
Nodes (38): Config, Exclude, RouterCfg, by_name(), cfg(), cfg_text(), circuit_templates_can_use_router(), collect_dhcp() (+30 more)

### Community 4 - "CLI and Checks"
Cohesion: 0.08
Nodes (35): Args, check(), check_finds_plain_api_and_no_source_router(), check_rejects_v3_target_for_now(), check_reports_a_bad_secret_and_a_missing_lqos_dir(), CheckLine, Command, Check (+27 more)

### Community 5 - "Secrets"
Cohesion: 0.09
Nodes (29): PppSource, ActiveComment, Default, ProfileComment, ProfileName, ProfileRateLimit, SecretComment, RateLimitOrder (+21 more)

### Community 6 - "Mock Router Tests"
Cohesion: 0.15
Nodes (25): read_router(), Answer, Empty, Raw, Rows, Silent, Trap, closed_port_is_unreachable() (+17 more)

### Community 7 - "CLI and Checks 2"
Cohesion: 0.12
Nodes (21): Session: updatecsv.py dissection, ADR-0003 Router failed vs offline, offline_action (default remove), offline_after (default 300 s), Handoff, Deferred minor items from defaults review, Naming: branch, crate, binary, service lqosync3, PPPoE speed order decision (+13 more)

### Community 8 - "Config"
Cohesion: 0.10
Nodes (18): a_circuit_comment_must_not_be_the_static_comment(), DEFAULTS_TOML, exclusions_parse_and_bad_ip_is_rejected(), default_from_file!, MIN, minimal_config_gets_documented_defaults(), node_type_defaults_follow_target(), parse_ipv4_net() (+10 more)

### Community 9 - "hashmap"
Cohesion: 0.15
Nodes (21): Network, NetworkMode, FlatGlobalRoot, FlatPromotion, FlatRouterRoot, GroupedRouterRoot, NoParent, RootPromotion (+13 more)

### Community 10 - "Topology 2"
Cohesion: 0.14
Nodes (20): ADR-0005 Topology follows v3, 14/15 CSV columns (v2_pair/v3_pair), max_floor_mbps (0.2) / safe_min_mbps (0.1), Parent Node ID column, ADR-0006 v3 guards and node types, compile_mode not full warning, Tool must reject duplicate IP and node name, How LibreQoS v3 handles nodes (+12 more)

### Community 11 - "Topology 3"
Cohesion: 0.17
Nodes (19): Session 2026-10-31 project setup and v3 simulation, LibreQoS v3 ingress facts, topology_import.json (internal, do not write), v3 offline simulation method, Home (vault map of content), libreqos.target (auto, v2_pair, v3_pair, v3_network), LibreQoS v3 ingress document, Last-good topology retention (+11 more)

### Community 12 - "File Safety"
Cohesion: 0.14
Nodes (13): backup(), backup_keeps_the_last_n(), HDR, Live, load_state(), load_state_seeds_on_first_run_and_reads_after(), lock(), read_live() (+5 more)

### Community 13 - "MikroTik Session"
Cohesion: 0.12
Nodes (19): Applier (hash, atomic write, reload), Circuit Model, Collector (RouterOS API reads), Config Loader, Proposed Crate Layout, libreqos_network.json, lqosd (LibreQoS daemon), network.json (+11 more)

### Community 14 - "Router Reads"
Cohesion: 0.13
Nodes (10): FakeSession, read_snapshot(), ReadError, Login, Protocol, Timeout, Tls, Unreachable (+2 more)

### Community 15 - "Topology 4"
Cohesion: 0.17
Nodes (18): 2026-10-31 Plan 1: lqosync3 core library session, lqosync3 core crate (config, speed, model, state, sources, topology, cycle, validate, writer), Plan 1 review fixes (commit 529fa36), Project phase: Plan 1 core and Plan 2 runtime done, lqosync3 Core Implementation Plan (Plan 1), Plan 1 review focus (edge cases), Task 3: Model, state file, and seed, Task 6: Cycle assembly and validation (+10 more)

### Community 16 - "Speed Rules"
Cohesion: 0.14
Nodes (14): ADR-0009 DHCP discovery and exclusions, Removal guard on (>20% or >50 rows held), RouterOS listen change detection (future), Speed Rules, DHCP server discovery (/ip/dhcp-server, discover), Exclusions ([exclude], [routers.exclude]), Subscriber presence and subscriber_grace, Task 2: Rate format and speed rules (SpeedParser) (+6 more)

### Community 17 - "Router Reads 2"
Cohesion: 0.17
Nodes (18): Deferred minor items from Plan 2 review, Manual shadow run on production shaper (next action), lqosync3 Runtime Implementation Plan (Plan 2), files::lock (try_lock, 'another lqosync3 is running'), files::load_state (state.json or seed), Task 9 Step 5 manual shadow read (operator approval), CycleSummary::next_sleep (error_retry / active / idle), Plan 2b: Telegram (+10 more)

### Community 18 - "MikroTik Session 2"
Cohesion: 0.15
Nodes (6): map_err(), MikrotikConnector, MikrotikSession, tls_client_config(), Connector, Setup

### Community 19 - "Speed Rules 2"
Cohesion: 0.19
Nodes (15): ADR-0004 TOML policy, Python defaults, Speed source order TOML lists, ADR-0007 Fresh design, not a port, Scenario tests (replace golden parity tests), Semantic compare with live files before cutover, updatecsv.py dissection, updatecsv.py defects to fix, Live production values (2026-10-31) (+7 more)

### Community 20 - "Scenario Tests"
Cohesion: 0.19
Nodes (7): State, cycle(), read(), report_json(), results(), scenarios_match_expected_output(), Snapshot

### Community 21 - "CircuitNames"
Cohesion: 0.19
Nodes (13): CircuitNames, HDR, read_csv(), read_csv_accepts_bom_crlf_and_missing_columns(), read_csv_skips_blank_names_and_keeps_repeated_names(), seed_infers_source_and_keeps_ids(), seed_nodes(), seed_nodes_reads_the_tree_with_parents_and_ids() (+5 more)

### Community 22 - "AtomicUsize"
Cohesion: 0.19
Nodes (11): Fake, gives_last_error_after_all_retries(), ppp_router(), Trap, reads_only_enabled_tables(), retry_reconnects_and_succeeds(), Script, session_is_kept_between_reads() (+3 more)

### Community 23 - "Config 2"
Cohesion: 0.15
Nodes (15): Repository branch lqosync3 (no shared history), Production state: Python lqosync-updatecsv.service and LibreQoS 2.2 unchanged, Map from config.json and planned import-json, Runtime modes (shadow, apply), Installation, One-line installer install.sh (target contract), Installer options (--mode, --version, --from-source, --import-json, --no-start), Manual installation (+7 more)

### Community 24 - "Status and Diff"
Cohesion: 0.17
Nodes (8): diff(), exit_with(), main(), now(), print_summary(), read_or_empty(), run(), SECRET_OWNER_UID

### Community 25 - "Router State"
Cohesion: 0.37
Nodes (11): RouterAccess, acc(), classify(), first_unreachable_sets_since(), login_tls_trap_timeout_are_failed_and_keep_memo(), memo(), name(), offline_after_zero_is_offline_at_once() (+3 more)

### Community 26 - "LibreQoS v3"
Cohesion: 0.16
Nodes (10): dhcp_server_gives_override_or_default(), DhcpCfg, DhcpMode, PerPlan, PerSite, DhcpServer, DhcpSource, LeaseComment (+2 more)

### Community 28 - "Status and Diff 2"
Cohesion: 0.24
Nodes (9): diff_ignores_column_order_row_order_and_number_text(), diff_pair(), diff_reports_added_removed_changed(), group(), header(), NEW14, nodes(), OLD13 (+1 more)

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
Cohesion: 0.36
Nodes (4): csv_header_sort_and_quoting(), field(), render_csv(), row()

### Community 33 - "CLI and Checks 3"
Cohesion: 0.24
Nodes (4): BackstopCap, BackstopCaps, BandwidthDefaults, HotspotCfg

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

### Community 41 - "OfflineAction"
Cohesion: 0.67
Nodes (3): OfflineAction, Keep, Remove

## Ambiguous Edges - Review These
- `Scenario tests (replace golden parity tests)` → `Stable node id (lq3- FNV-1a 64)`  [AMBIGUOUS]
  brain/20-decisions/ADR-0007-fresh-design-not-a-port.md · relation: conceptually_related_to
- `Collector (RouterOS API reads)` → `RouterOS listen change detection (future)`  [AMBIGUOUS]
  docs/ARCHITECTURE.md · relation: conceptually_related_to
- `Project phase: Plan 1 core and Plan 2 runtime done` → `Operations`  [AMBIGUOUS]
  docs/OPERATIONS.md · relation: conceptually_related_to
- `Deferred minor items from Plan 2 review` → `Write order: state.json first, then LibreQoS pair`  [AMBIGUOUS]
  brain/HANDOFF.md · relation: conceptually_related_to
- `Requirements (Ubuntu 24.04, LibreQoS 2.0-2.2 / 3.x, Rust 1.88+)` → `rust-version 1.89 (std::fs::File::try_lock)`  [AMBIGUOUS]
  docs/INSTALLATION.md · relation: conceptually_related_to
- `lqosync3 once --dry-run` → `cli::Command (check, once --force, run, diff, defaults)`  [AMBIGUOUS]
  docs/OPERATIONS.md · relation: conceptually_related_to
- `Router state classification (RouterResult, unreachable_since)` → `read_snapshot / read_router (timeouts, retries, session reuse)`  [AMBIGUOUS]
  docs/superpowers/plans/2026-10-31-lqosync3-runtime.md · relation: conceptually_related_to

## Knowledge Gaps
- **80 isolated node(s):** `Failed`, `Offline`, `Unreachable`, `V2`, `V3` (+75 more)
  These have ≤1 connection - possible missing edges or undocumented components. (Counts symbols only; 177 node(s) total have ≤1 connection when file, concept and rationale nodes are included.)
- **7 thin communities (<3 nodes) omitted from report** — run `graphify query` to explore isolated nodes.

## Suggested Questions
_Questions this graph is uniquely positioned to answer:_

- **Why does `Config` connect `Circuit Collection` to `Cycle Assembly`, `CLI and Checks 3`, `Runner Cycle`, `Topology`, `CLI and Checks`, `.default()`, `LibreQoS v3 3`, `Secrets`, `Config`, `hashmap`, `CLI and Checks 4`, `Mock Router Tests`, `Scenario Tests`, `CircuitNames`, `AtomicUsize`, `Status and Diff`, `Router State`, `LibreQoS v3`?**
  _High betweenness centrality (0.151) - this node is a cross-community bridge._
- **Are the 2 inferred relationships involving `lqosync3 Runtime Implementation Plan (Plan 2)` (e.g. with `Project phase: Plan 1 core and Plan 2 runtime done` and `lqosync3 Core Implementation Plan (Plan 1)`) actually correct?**
  _`lqosync3 Runtime Implementation Plan (Plan 2)` has 2 INFERRED edges - model-reasoned connections that need verification._
- **What connects `Failed`, `Offline`, `Unreachable` to the rest of the system?**
  _80 weakly-connected nodes found - possible documentation gaps or missing edges._
- **Should `Cycle Assembly` be split into smaller, more focused modules?**
  _Cohesion score 0.0603448275862069 - nodes in this community are weakly interconnected._
- **Why does `RouterCfg` connect `Circuit Collection` to `CLI and Checks 3`, `Runner Cycle`, `Mock Router Tests`, `Config`, `Router Reads`, `MikroTik Session 2`, `AtomicUsize`, `LibreQoS v3`?**
  _High betweenness centrality (0.039) - this node is a cross-community bridge._

### Low-confidence Hints
_AMBIGUOUS edges — the extractor was unsure. Verify before acting on these._

- **What is the exact relationship between `Scenario tests (replace golden parity tests)` and `Stable node id (lq3- FNV-1a 64)`?**
  _Edge tagged AMBIGUOUS (relation: conceptually_related_to) - confidence is low._
- **What is the exact relationship between `Collector (RouterOS API reads)` and `RouterOS listen change detection (future)`?**
  _Edge tagged AMBIGUOUS (relation: conceptually_related_to) - confidence is low._