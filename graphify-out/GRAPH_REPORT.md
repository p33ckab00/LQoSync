# Graph Report - LQoSync3.0  (2026-10-31)

## Corpus Check
- 34 files · ~31,674 words
- Verdict: corpus is large enough that graph structure adds value.

## Summary
- 523 nodes · 1236 edges · 30 communities (23 shown, 7 thin omitted)
- Extraction: 95% EXTRACTED · 5% INFERRED · 0% AMBIGUOUS · INFERRED: 63 edges (avg confidence: 0.87)
- Token cost: 200,244 input · 0 output

## Community Hubs (Navigation)
- Decisions and Session Notes
- Scenario Test Harness
- Cycle Assembly
- Topology and Node IDs
- Speed Parsing
- Router Circuit Collection
- Validation and Network Modes
- Config Loading
- Targets and Applier
- DHCP Settings
- Config Rules and Modes
- Output Files and State Store
- Speed Rules Design
- Runtime Settings
- Collector and Router State
- Pure Core Design
- Cycle and Node Rules
- Node Templates
- DHCP Speed Sources
- LibreQoS Targets
- PPPoE Speed Sources
- Backstop Caps
- Offline Action
- Scenario Tests Task
- State JSON
- v3sim Script
- Telegram Notifier
- Crate Root

## God Nodes (most connected - your core abstractions)
1. `Config` - 41 edges
2. `run()` - 27 edges
3. `CsvRow` - 25 edges
4. `Handoff` - 25 edges
5. `Brain Home (MOC)` - 21 edges
6. `Rate` - 20 edges
7. `build_nodes()` - 18 edges
8. `LibreQoS v3 ingress document` - 17 edges
9. `state()` - 17 edges
10. `LQoSync3 README` - 17 edges

## Surprising Connections (you probably didn't know these)
- `v3 offline simulation method` --semantically_similar_to--> `Offline v3 simulation`  [INFERRED] [semantically similar]
  brain/30-knowledge/v3-offline-simulation-method.md → docs/LIBREQOS-V3-INGRESS.md
- `v3 offline simulation method` --semantically_similar_to--> `/run/lqos/bus 0777 safety caution`  [INFERRED] [semantically similar]
  brain/30-knowledge/v3-offline-simulation-method.md → docs/LIBREQOS-V3-INGRESS.md
- `Session 2026-10-31 project setup and v3 simulation` --references--> `libreqos-v3-preflight`  [INFERRED]
  brain/10-sessions/2026-10-31-project-setup-and-v3-simulation.md → docs/LIBREQOS-V3-INGRESS.md
- `ADR-0002 v3 target selection` --references--> `topology_import.json (internal)`  [INFERRED]
  brain/20-decisions/ADR-0002-v3-target-selection.md → docs/LIBREQOS-V3-INGRESS.md
- `ADR-0002 v3 target selection` --rationale_for--> `Target Detector`  [INFERRED]
  brain/20-decisions/ADR-0002-v3-target-selection.md → docs/ARCHITECTURE.md

## Import Cycles
- 2-file cycle: `src/config.rs -> src/speed.rs -> src/config.rs`

## Hyperedges (group relationships)
- **Router-to-LibreQoS cycle data flow** — docs_architecture_collector, docs_architecture_circuit_model, docs_architecture_validator, docs_architecture_target_detector, docs_architecture_writers, docs_architecture_applier [EXTRACTED 1.00]
- **Subscriber removal safety (router state, presence, grace, guard)** — docs_architecture_router_state_machine, docs_configuration_router_state, docs_configuration_subscriber_presence, docs_configuration_removal_guard, docs_superpowers_specs_2026_10_31_lqosync3_core_design_cycle_rules [INFERRED 0.85]
- **Plan 1 core tasks building assemble()** — docs_superpowers_plans_2026_10_31_lqosync3_core_task1_config, docs_superpowers_plans_2026_10_31_lqosync3_core_task2_speed, docs_superpowers_plans_2026_10_31_lqosync3_core_task3_state, docs_superpowers_plans_2026_10_31_lqosync3_core_task4_sources, docs_superpowers_plans_2026_10_31_lqosync3_core_task5_topology, docs_superpowers_plans_2026_10_31_lqosync3_core_task6_cycle, docs_superpowers_plans_2026_10_31_lqosync3_core_task7_writer, docs_superpowers_plans_2026_10_31_lqosync3_core_task8_scenarios [EXTRACTED 1.00]
- **Router failure vs offline handling** — brain_20_decisions_adr_0003_router_failed_vs_offline_router_state_model, brain_20_decisions_adr_0003_router_failed_vs_offline_offline_after, brain_20_decisions_adr_0003_router_failed_vs_offline_offline_action, brain_30_knowledge_updatecsv_py_dissection_defects_to_fix [EXTRACTED 1.00]
- **LibreQoS v3 node rule compliance** — brain_20_decisions_adr_0005_topology_follows_v3_stable_node_id, brain_20_decisions_adr_0005_topology_follows_v3_parent_node_id, brain_20_decisions_adr_0005_topology_follows_v3_csv_column_count, brain_20_decisions_adr_0005_topology_follows_v3_max_floor_mbps, brain_20_decisions_adr_0005_topology_follows_v3_flat_promotion_parent_fix, brain_20_decisions_adr_0006_v3_guards_and_node_types_no_parent_guard, brain_20_decisions_adr_0006_v3_guards_and_node_types_site_group_nodes, brain_20_decisions_adr_0006_v3_guards_and_node_types_compile_mode_warning [INFERRED 0.85]
- **Output target selection** — readme_target_auto, readme_v2_pair, readme_v3_network, readme_v3_pair, brain_30_knowledge_libreqos_v3_ingress_facts_topology_source [EXTRACTED 1.00]

## Communities (30 total, 7 thin omitted)

### Community 0 - "Decisions and Session Notes"
Cohesion: 0.06
Nodes (72): Session 2026-10-31 project setup and v3 simulation, Session: updatecsv.py dissection, Orphan branch lqosync3, RouterOS listen command (untested), ADR-0003 Router failed vs offline, offline_action (default remove), offline_after (default 300 s), ADR-0004 TOML policy, Python defaults (+64 more)

### Community 1 - "Scenario Test Harness"
Cohesion: 0.06
Nodes (37): Entry, Columns, V2, V3, CsvRow, GuardHold, RouterMemo, Source (+29 more)

### Community 2 - "Cycle Assembly"
Cohesion: 0.10
Nodes (54): a_row_that_stays_keeps_its_group_node(), assemble(), cfg(), changed_columns(), CycleInput, CycleOutput, duplicate_ip_existing_vs_existing_keeps_collected(), duplicate_ip_new_vs_existing_keeps_existing() (+46 more)

### Community 3 - "Topology and Node IDs"
Cohesion: 0.14
Nodes (36): StoredNode, GroupKey, Hotspot, Ppp, backstop_cap_limits_group(), build_nodes(), carried_nodes_of_failed_router_stay(), cfg() (+28 more)

### Community 4 - "Speed Parsing"
Cohesion: 0.11
Nodes (22): RateLimitOrder, DownloadUpload, RouterosRxTx, Speed, UnitlessUnit, Bps, Mbps, circuit_rates() (+14 more)

### Community 5 - "Router Circuit Collection"
Cohesion: 0.19
Nodes (30): Config, RouterCfg, RosRow, RouterSnapshot, by_name(), cfg(), collect_dhcp(), collect_hotspot() (+22 more)

### Community 6 - "Validation and Network Modes"
Cohesion: 0.12
Nodes (20): Network, NetworkMode, FlatGlobalRoot, FlatPromotion, FlatRouterRoot, GroupedRouterRoot, NoParent, RootPromotion (+12 more)

### Community 7 - "Config Loading"
Cohesion: 0.11
Nodes (10): CircuitNames, MIN, minimal_config_gets_documented_defaults(), node_type_defaults_follow_target(), per_site_rejects_lease_comment_and_defaults_follow_mode(), PppoeCfg, Safety, Telegram (+2 more)

### Community 8 - "Targets and Applier"
Cohesion: 0.13
Nodes (20): Applier (hash, atomic write, reload), libreqos_network.json, lqosd (LibreQoS daemon), Target Detector, Target v3_network, Target v3_pair, Map from config.json, lqosync3 import-json command (planned) (+12 more)

### Community 9 - "DHCP Settings"
Cohesion: 0.18
Nodes (7): BandwidthDefaults, DhcpCfg, DhcpMode, PerPlan, PerSite, DhcpServer, HotspotCfg

### Community 10 - "Config Rules and Modes"
Cohesion: 0.20
Nodes (12): Config Loader, Six Network Modes, Topology Builder, LibreQoS v3 topology.compile_mode, DHCP speed sources (per_site / per_plan), lqosync3.toml config file, Secret files (*_file, mode 0600 root), LibreQoS v3 checks (no_parent, compile_mode) (+4 more)

### Community 11 - "Output Files and State Store"
Cohesion: 0.21
Nodes (12): Proposed Crate Layout, network.json, ShapedDevices.csv, State Store (state.json), Target v2_pair, v2 vs v3 writer differences, Validator, Writers (one per target) (+4 more)

### Community 12 - "Speed Rules Design"
Cohesion: 0.27
Nodes (9): Speed Rules, Comment speed format (Name|25M/5M), PPPoE speed source chain (speed.pppoe_sources), Rate direction and units (rate_limit_order, unitless_unit), updatecsv.py (legacy Python service), Task 2: Rate format and speed rules (SpeedParser), Task 4: Circuit collection (collect_router), Circuit rules per source (PPPoE, Hotspot, DHCP) (+1 more)

### Community 13 - "Runtime Settings"
Cohesion: 0.20
Nodes (9): LogLevel, Debug, Error, Info, Warn, Runtime, RuntimeMode, Apply (+1 more)

### Community 14 - "Collector and Router State"
Cohesion: 0.29
Nodes (6): Circuit Model, Collector (RouterOS API reads), RouterOS listen change detection (future), MikroTik RouterOS API, Router State settings (offline_after, offline_action), Subscriber Presence and subscriber_grace

### Community 15 - "Pure Core Design"
Cohesion: 0.32
Nodes (6): Plan 1 review focus (edge cases), Task 3: Model, state file, and seed, RouterResult (Online/Failed/Unreachable/Offline), RouterSnapshot (six RouterOS tables), State seed from live ShapedDevices.csv, state.json format (version 1)

### Community 16 - "Cycle and Node Rules"
Cohesion: 0.52
Nodes (6): Task 5: Topology (build_nodes, to_network_json), Task 6: Cycle assembly and validation, Cycle assembly rules (8 ordered rules), Guard hold fingerprint (FNV-1a 64 of removed names), Node rules (group nodes, roots, client), Validation stop rules

### Community 17 - "Node Templates"
Cohesion: 0.38
Nodes (3): ClientTemplate, Nodes, NodeTemplate

### Community 18 - "DHCP Speed Sources"
Cohesion: 0.29
Nodes (4): DhcpSource, LeaseComment, ServerComment, Settings

### Community 19 - "LibreQoS Targets"
Cohesion: 0.29
Nodes (6): Libreqos, Target, Auto, V2Pair, V3Network, V3Pair

### Community 20 - "PPPoE Speed Sources"
Cohesion: 0.29
Nodes (7): PppSource, ActiveComment, Default, ProfileComment, ProfileName, ProfileRateLimit, SecretComment

### Community 22 - "Offline Action"
Cohesion: 0.40
Nodes (4): OfflineAction, Keep, Remove, RouterAccess

### Community 23 - "Scenario Tests Task"
Cohesion: 1.00
Nodes (3): Task 8: Scenario tests and v3 check, Scenario tests (synthetic fixtures), tools/v3sim/check.sh v3 compiler check

## Ambiguous Edges - Review These
- `Collector (RouterOS API reads)` → `RouterOS listen change detection (future)`  [AMBIGUOUS]
  docs/ARCHITECTURE.md · relation: conceptually_related_to
- `Stable node id (lq3- FNV-1a 64)` → `Scenario tests (replace golden parity tests)`  [AMBIGUOUS]
  brain/20-decisions/ADR-0007-fresh-design-not-a-port.md · relation: conceptually_related_to

## Knowledge Gaps
- **56 isolated node(s):** `LibreQoS.py --updateonly no-op in v3`, `[topology] source in lqos.conf`, `lqosync3`, `Shadow`, `Apply` (+51 more)
  These have ≤1 connection - possible missing edges or undocumented components. (Counts symbols only; 114 node(s) total have ≤1 connection when file, concept and rationale nodes are included.)
- **7 thin communities (<3 nodes) omitted from report** — run `graphify query` to explore isolated nodes.

## Suggested Questions
_Questions this graph is uniquely positioned to answer:_

- **Why does `Config` connect `Router Circuit Collection` to `Scenario Test Harness`, `Cycle Assembly`, `Topology and Node IDs`, `Speed Parsing`, `Validation and Network Modes`, `Config Loading`, `DHCP Settings`, `Runtime Settings`, `Node Templates`, `DHCP Speed Sources`, `LibreQoS Targets`, `Backstop Caps`, `Offline Action`?**
  _High betweenness centrality (0.087) - this node is a cross-community bridge._
- **What connects `LibreQoS.py --updateonly no-op in v3`, `[topology] source in lqos.conf`, `lqosync3` to the rest of the system?**
  _56 weakly-connected nodes found - possible documentation gaps or missing edges._
- **Should `Decisions and Session Notes` be split into smaller, more focused modules?**
  _Cohesion score 0.055921855921855924 - nodes in this community are weakly interconnected._
- **Why does `CsvRow` connect `Scenario Test Harness` to `Cycle Assembly`, `Topology and Node IDs`, `Validation and Network Modes`?**
  _High betweenness centrality (0.032) - this node is a cross-community bridge._
- **Should `Scenario Test Harness` be split into smaller, more focused modules?**
  _Cohesion score 0.05552617662612375 - nodes in this community are weakly interconnected._

### Low-confidence Hints
_AMBIGUOUS edges — the extractor was unsure. Verify before acting on these._

- **What is the exact relationship between `Collector (RouterOS API reads)` and `RouterOS listen change detection (future)`?**
  _Edge tagged AMBIGUOUS (relation: conceptually_related_to) - confidence is low._
- **What is the exact relationship between `Stable node id (lq3- FNV-1a 64)` and `Scenario tests (replace golden parity tests)`?**
  _Edge tagged AMBIGUOUS (relation: conceptually_related_to) - confidence is low._