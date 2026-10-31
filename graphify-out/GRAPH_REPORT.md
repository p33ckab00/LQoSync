# Graph Report - LQoSync3.0  (2026-10-31)

## Corpus Check
- 41 files · ~35,786 words
- Verdict: corpus is large enough that graph structure adds value.
- Unclassified: 3 file(s) not represented in the graph (top: (none) 2, .toml 1)

## Summary
- 575 nodes · 1447 edges · 33 communities (27 shown, 6 thin omitted)
- Extraction: 95% EXTRACTED · 5% INFERRED · 0% AMBIGUOUS · INFERRED: 76 edges (avg confidence: 0.88)
- Token cost: 0 input · 0 output

## Graph Freshness
- Built from commit: `ef98520e`
- Run `git rev-parse HEAD` and compare to check if the graph is stale.
- Run `graphify update .` after code changes (no API cost).

## Community Hubs (Navigation)
- Cycle Assembly
- Topology and Node Templates
- Router Circuit Collection
- Speed and DHCP Sources
- Architecture Parts
- DHCP Discovery and Exclusions
- Plan 1 Session and Review
- Early Sessions and ADRs
- Router State Decisions
- Validation
- Scenario Test Harness
- v3 Node Rules
- config.rs
- v3 Ingress Evidence
- State File and CSV Read
- CSV Writer and Columns
- DHCP Settings
- Model Types
- DhcpSource
- Runtime Settings
- Circuit rules for each source
- Terminal UI
- Embedded Defaults Merge
- LibreQoS Targets
- Circuit Sources
- Offline Action
- .from_toml_str
- State JSON
- v3sim Script
- Telegram Notifier
- Crate Root

## God Nodes (most connected - your core abstractions)
1. `Config` - 48 edges
2. `Configuration reference` - 35 edges
3. `lqosync3 core design spec` - 32 edges
4. `run()` - 31 edges
5. `CsvRow` - 26 edges
6. `Handoff (current project state)` - 23 edges
7. `Home (vault map of content)` - 23 edges
8. `Rate` - 22 edges
9. `build_nodes()` - 21 edges
10. `ADR-0005 Topology follows v3` - 21 edges

## Surprising Connections (you probably didn't know these)
- `v3 offline simulation method` --semantically_similar_to--> `Offline v3 simulation`  [INFERRED] [semantically similar]
  brain/30-knowledge/v3-offline-simulation-method.md → docs/LIBREQOS-V3-INGRESS.md
- `v3 offline simulation method` --semantically_similar_to--> `/run/lqos/bus 0777 safety caution`  [INFERRED] [semantically similar]
  brain/30-knowledge/v3-offline-simulation-method.md → docs/LIBREQOS-V3-INGRESS.md
- `Session 2026-10-31 project setup and v3 simulation` --references--> `libreqos-v3-preflight`  [INFERRED]
  brain/10-sessions/2026-10-31-project-setup-and-v3-simulation.md → docs/LIBREQOS-V3-INGRESS.md
- `ADR-0002 v3 target selection` --references--> `topology_import.json (internal)`  [INFERRED]
  brain/20-decisions/ADR-0002-v3-target-selection.md → docs/LIBREQOS-V3-INGRESS.md
- `cycle()` --calls--> `assemble()`  [INFERRED]
  tests/scenarios.rs → src/cycle.rs

## Import Cycles
- 2-file cycle: `src/config.rs -> src/speed.rs -> src/config.rs`

## Hyperedges (group relationships)
- **Layered settings merge (embedded defaults, router_defaults, user TOML)** — docs_configuration_defaults_toml, docs_configuration_router_defaults, docs_configuration_lqosync3_toml, docs_superpowers_specs_2026_10_31_lqosync3_core_design_config_from_toml_str [EXTRACTED 1.00]
- **Router-to-LibreQoS cycle data flow** — docs_architecture_collector, docs_architecture_circuit_model, docs_architecture_validator, docs_architecture_target_detector, docs_architecture_writers, docs_architecture_applier [EXTRACTED 1.00]
- **Plan 1 core tasks building assemble()** — docs_superpowers_plans_2026_10_31_lqosync3_core_task1_config, docs_superpowers_plans_2026_10_31_lqosync3_core_task2_speed, docs_superpowers_plans_2026_10_31_lqosync3_core_task3_state, docs_superpowers_plans_2026_10_31_lqosync3_core_task4_sources, docs_superpowers_plans_2026_10_31_lqosync3_core_task5_topology, docs_superpowers_plans_2026_10_31_lqosync3_core_task6_cycle, docs_superpowers_plans_2026_10_31_lqosync3_core_task7_writer, docs_superpowers_plans_2026_10_31_lqosync3_core_task8_scenarios [EXTRACTED 1.00]
- **Output target selection** — readme_target_auto, readme_v2_pair, readme_v3_network, readme_v3_pair, brain_30_knowledge_libreqos_v3_ingress_facts_topology_source [EXTRACTED 1.00]
- **Removal guard hold-and-release flow** — docs_configuration_removal_guard, docs_superpowers_specs_2026_10_31_lqosync3_core_design_cycle_assembly_rules, docs_superpowers_specs_2026_10_31_lqosync3_core_design_guard_fingerprint, docs_superpowers_specs_2026_10_31_lqosync3_core_design_state_file [EXTRACTED 1.00]
- **Router failure vs offline handling** — brain_20_decisions_adr_0003_router_failed_vs_offline_router_state_model, brain_20_decisions_adr_0003_router_failed_vs_offline_offline_after, brain_20_decisions_adr_0003_router_failed_vs_offline_offline_action, brain_30_knowledge_updatecsv_py_dissection_defects_to_fix [EXTRACTED 1.00]
- **LibreQoS v3 node rule compliance** — brain_20_decisions_adr_0005_topology_follows_v3_stable_node_id, brain_20_decisions_adr_0005_topology_follows_v3_parent_node_id, brain_20_decisions_adr_0005_topology_follows_v3_csv_column_count, brain_20_decisions_adr_0005_topology_follows_v3_max_floor_mbps, brain_20_decisions_adr_0005_topology_follows_v3_flat_promotion_parent_fix, brain_20_decisions_adr_0006_v3_guards_and_node_types_no_parent_guard, brain_20_decisions_adr_0006_v3_guards_and_node_types_site_group_nodes, brain_20_decisions_adr_0006_v3_guards_and_node_types_compile_mode_warning [INFERRED 0.85]
- **DHCP discovery and subscriber exclusion** — brain_20_decisions_adr_0009_dhcp_discovery_and_exclusions_dhcp_server_discovery, brain_20_decisions_adr_0009_dhcp_discovery_and_exclusions_exclusions, docs_configuration_routers_dhcp, docs_configuration_exclusions, docs_superpowers_specs_2026_10_31_lqosync3_core_design_circuit_rules [INFERRED 0.95]

## Communities (33 total, 6 thin omitted)

### Community 0 - "Cycle Assembly"
Cohesion: 0.09
Nodes (64): a_row_that_stays_keeps_its_group_node(), assemble(), cfg(), changed_columns(), CycleInput, CycleOutput, duplicate_ip_existing_vs_existing_keeps_collected(), duplicate_ip_new_vs_existing_keeps_existing() (+56 more)

### Community 1 - "Topology and Node Templates"
Cohesion: 0.10
Nodes (48): NodeTemplate, StoredNode, GroupKey, DhcpPlan, DhcpSite, Hotspot, Ppp, PppPlan (+40 more)

### Community 2 - "Router Circuit Collection"
Cohesion: 0.13
Nodes (41): Config, Exclude, parse_ipv4_net(), RouterCfg, RosRow, RouterSnapshot, by_name(), cfg() (+33 more)

### Community 3 - "Speed and DHCP Sources"
Cohesion: 0.09
Nodes (29): PppSource, ActiveComment, Default, ProfileComment, ProfileName, ProfileRateLimit, SecretComment, RateLimitOrder (+21 more)

### Community 4 - "Architecture Parts"
Cohesion: 0.07
Nodes (36): lqosync-updatecsv.service (Python production service), Applier (hash, atomic write, reload), Circuit Model, Collector (RouterOS API reads), Config Loader, Proposed Crate Layout, libreqos_network.json, lqosd (LibreQoS daemon) (+28 more)

### Community 5 - "DHCP Discovery and Exclusions"
Cohesion: 0.17
Nodes (17): RouterOS listen change detection (future), Configuration reference, Comment speed format, Map from config.json (import-json), DHCP speed_sources (per_site / per_plan), Differences from updatecsv.py, Exclusions config ([exclude], [routers.exclude]), lqosync3 check (+9 more)

### Community 6 - "Plan 1 Session and Review"
Cohesion: 0.17
Nodes (20): lqosync3 core crate (config, speed, model, state, sources, topology, cycle, validate, writer), Plan 2 runtime (next action), Node types (v2_type / v3_type), Router state (online / failed / unreachable / offline), lqosync3 Core Implementation Plan (Plan 1), Plan 1 review focus (edge cases), Task 3: Model, state file, and seed, Task 5: Topology (build_nodes, to_network_json) (+12 more)

### Community 7 - "Early Sessions and ADRs"
Cohesion: 0.19
Nodes (20): Session 2026-10-31 project setup and v3 simulation, Session: updatecsv.py dissection, Orphan branch lqosync3, RouterOS listen command (untested), LibreQoS v3 ingress facts, libreqos_network.json schema_version 2, topology_import.json (internal, do not write), [topology] source setting (+12 more)

### Community 8 - "Router State Decisions"
Cohesion: 0.16
Nodes (15): ADR-0003 Router failed vs offline, offline_action (default remove), offline_after (default 300 s), ADR-0004 TOML policy, Python defaults, Speed source order TOML lists, Semantic compare with live files before cutover, updatecsv.py dissection, updatecsv.py defects to fix (+7 more)

### Community 9 - "Validation"
Cohesion: 0.14
Nodes (21): Network, NetworkMode, FlatGlobalRoot, FlatPromotion, FlatRouterRoot, GroupedRouterRoot, NoParent, RootPromotion (+13 more)

### Community 10 - "Scenario Test Harness"
Cohesion: 0.15
Nodes (20): 2026-10-31 Plan 1: lqosync3 core library session, Plan 1 review fixes (commit 529fa36), ADR-0005 Topology follows v3, 14/15 CSV columns (v2_pair/v3_pair), max_floor_mbps (0.2) / safe_min_mbps (0.1), Parent Node ID column, ADR-0006 v3 guards and node types, compile_mode not full warning (+12 more)

### Community 11 - "v3 Node Rules"
Cohesion: 0.15
Nodes (7): render_network_json(), cycle(), read(), report_json(), results(), scenarios_match_expected_output(), Snapshot

### Community 12 - "config.rs"
Cohesion: 0.10
Nodes (10): BackstopCap, BackstopCaps, ClientTemplate, DEFAULTS_TOML, default_from_file!, MIN, Nodes, PppoeCfg (+2 more)

### Community 13 - "v3 Ingress Evidence"
Cohesion: 0.21
Nodes (13): LibreQoS v3 ingress document, Last-good topology retention, liblqos_python.so binding, libreqos_3.0.7 beta package, libreqos_network.json schema 2, libreqos-v3-preflight, lqosd (v3 control plane), lqosd inotify file watcher (+5 more)

### Community 14 - "State File and CSV Read"
Cohesion: 0.18
Nodes (13): CircuitNames, HDR, read_csv(), read_csv_accepts_bom_crlf_and_missing_columns(), read_csv_skips_blank_names_and_keeps_repeated_names(), seed_infers_source_and_keeps_ids(), seed_nodes(), seed_nodes_reads_the_tree_with_parents_and_ids() (+5 more)

### Community 15 - "CSV Writer and Columns"
Cohesion: 0.19
Nodes (7): Columns, V2, V3, csv_header_sort_and_quoting(), field(), render_csv(), row()

### Community 16 - "DHCP Settings"
Cohesion: 0.21
Nodes (8): BandwidthDefaults, dhcp_server_gives_override_or_default(), DhcpCfg, DhcpMode, PerPlan, PerSite, DhcpServer, HotspotCfg

### Community 17 - "Model Types"
Cohesion: 0.28
Nodes (5): GuardHold, RouterMemo, State, V2_COLUMNS, V3_COLUMNS

### Community 18 - "DhcpSource"
Cohesion: 0.25
Nodes (4): DhcpSource, LeaseComment, ServerComment, Settings

### Community 19 - "Runtime Settings"
Cohesion: 0.22
Nodes (9): LogLevel, Debug, Error, Info, Warn, Runtime, RuntimeMode, Apply (+1 more)

### Community 20 - "Circuit rules for each source"
Cohesion: 0.23
Nodes (8): ADR-0009 DHCP discovery and exclusions, Speed Rules, Name templates ([nodes], [circuits]), Task 2: Rate format and speed rules (SpeedParser), Task 4: Circuit collection (collect_router), Circuit rules for each source, DhcpCfg::server(name) defaults, Output format (round3, fmt_rate, 14/15 CSV columns)

### Community 21 - "Terminal UI"
Cohesion: 0.53
Nodes (4): ADR-0008 Terminal UI, TUI config editor (toml_edit), SIGUSR1 sync now / SIGHUP reload, status.json state file

### Community 22 - "Embedded Defaults Merge"
Cohesion: 0.47
Nodes (3): deep_merge(), defaults(), defaults_table()

### Community 23 - "LibreQoS Targets"
Cohesion: 0.33
Nodes (6): Libreqos, Target, Auto, V2Pair, V3Network, V3Pair

### Community 24 - "Circuit Sources"
Cohesion: 0.40
Nodes (5): Source, Dhcp, Hotspot, Ppp, Unknown

### Community 25 - "Offline Action"
Cohesion: 0.50
Nodes (4): OfflineAction, Keep, Remove, RouterAccess

### Community 26 - ".from_toml_str"
Cohesion: 0.20
Nodes (9): a_circuit_comment_must_not_be_the_static_comment(), exclusions_parse_and_bad_ip_is_rejected(), minimal_config_gets_documented_defaults(), node_type_defaults_follow_target(), per_site_rejects_lease_comment_and_defaults_follow_mode(), router_defaults_apply_to_each_router_and_a_router_overrides_them(), unknown_key_is_rejected_with_its_name(), unknown_template_fields_are_rejected() (+1 more)

## Ambiguous Edges - Review These
- `Stable node id (lq3- FNV-1a 64)` → `Scenario tests (replace golden parity tests)`  [AMBIGUOUS]
  brain/20-decisions/ADR-0007-fresh-design-not-a-port.md · relation: conceptually_related_to
- `Collector (RouterOS API reads)` → `RouterOS listen change detection (future)`  [AMBIGUOUS]
  docs/ARCHITECTURE.md · relation: conceptually_related_to

## Knowledge Gaps
- **61 isolated node(s):** `lqosync3`, `Shadow`, `Apply`, `Error`, `Warn` (+56 more)
  These have ≤1 connection - possible missing edges or undocumented components. (Counts symbols only; 118 node(s) total have ≤1 connection when file, concept and rationale nodes are included.)
- **6 thin communities (<3 nodes) omitted from report** — run `graphify query` to explore isolated nodes.

## Suggested Questions
_Questions this graph is uniquely positioned to answer:_

- **Why does `Config` connect `Router Circuit Collection` to `Cycle Assembly`, `Topology and Node Templates`, `Speed and DHCP Sources`, `Validation`, `v3 Node Rules`, `config.rs`, `State File and CSV Read`, `DHCP Settings`, `DhcpSource`, `Runtime Settings`, `Embedded Defaults Merge`, `LibreQoS Targets`, `Offline Action`, `.from_toml_str`?**
  _High betweenness centrality (0.091) - this node is a cross-community bridge._
- **What connects `lqosync3`, `Shadow`, `Apply` to the rest of the system?**
  _61 weakly-connected nodes found - possible documentation gaps or missing edges._
- **Should `Cycle Assembly` be split into smaller, more focused modules?**
  _Cohesion score 0.08819538670284939 - nodes in this community are weakly interconnected._
- **Why does `Rate` connect `Speed and DHCP Sources` to `Cycle Assembly`, `Topology and Node Templates`, `Router Circuit Collection`, `config.rs`, `DHCP Settings`?**
  _High betweenness centrality (0.033) - this node is a cross-community bridge._
- **Should `Topology and Node Templates` be split into smaller, more focused modules?**
  _Cohesion score 0.10482180293501048 - nodes in this community are weakly interconnected._

### Low-confidence Hints
_AMBIGUOUS edges — the extractor was unsure. Verify before acting on these._

- **What is the exact relationship between `Stable node id (lq3- FNV-1a 64)` and `Scenario tests (replace golden parity tests)`?**
  _Edge tagged AMBIGUOUS (relation: conceptually_related_to) - confidence is low._
- **What is the exact relationship between `Collector (RouterOS API reads)` and `RouterOS listen change detection (future)`?**
  _Edge tagged AMBIGUOUS (relation: conceptually_related_to) - confidence is low._