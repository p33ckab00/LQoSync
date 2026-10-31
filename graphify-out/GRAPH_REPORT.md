# Graph Report - LQoSync3.0  (2026-10-31)

## Corpus Check
- Corpus is ~6,567 words - fits in a single context window. You may not need a graph.

## Summary
- 71 nodes · 150 edges · 9 communities (8 shown, 1 thin omitted)
- Extraction: 83% EXTRACTED · 17% INFERRED · 0% AMBIGUOUS · INFERRED: 25 edges (avg confidence: 0.9)
- Token cost: 70,046 input · 0 output

## Community Hubs (Navigation)
- Sync Pipeline Internals
- Operations and Cutover
- LibreQoS v3 Runtime Evidence
- Output Target Selection
- Brain Vault Notes
- Configuration and Secrets
- Speed Rules and Open Decisions
- Installation
- Runtime Mode

## God Nodes (most connected - your core abstractions)
1. `Architecture document` - 19 edges
2. `LibreQoS v3 ingress document` - 15 edges
3. `Brain Home (MOC)` - 12 edges
4. `Configuration document` - 11 edges
5. `Handoff note` - 9 edges
6. `LQoSync UpdateCSV README` - 8 edges
7. `Operations document` - 8 edges
8. `Session 2026-10-31 project setup and v3 simulation` - 8 edges
9. `ADR-0001 One model, many writers` - 8 edges
10. `ADR-0002 v3 target selection` - 8 edges

## Surprising Connections (you probably didn't know these)
- `v3 offline simulation method` --semantically_similar_to--> `Offline v3 simulation`  [INFERRED] [semantically similar]
  brain/30-knowledge/v3-offline-simulation-method.md → docs/LIBREQOS-V3-INGRESS.md
- `v3 offline simulation method` --semantically_similar_to--> `/run/lqos/bus 0777 safety caution`  [INFERRED] [semantically similar]
  brain/30-knowledge/v3-offline-simulation-method.md → docs/LIBREQOS-V3-INGRESS.md
- `Secret files (*_file, mode 0600 root)` --semantically_similar_to--> `Read-only router access rule`  [INFERRED] [semantically similar]
  docs/CONFIGURATION.md → README.md
- `ADR-0002 v3 target selection` --references--> `topology_import.json (internal)`  [INFERRED]
  brain/20-decisions/ADR-0002-v3-target-selection.md → docs/LIBREQOS-V3-INGRESS.md
- `Map from config.json to TOML` --references--> `updatecsv.py (Python predecessor)`  [INFERRED]
  docs/CONFIGURATION.md → README.md

## Hyperedges (group relationships)
- **Per-cycle data pipeline: collect, model, validate, write, apply** — docs_architecture_collector, docs_architecture_circuit_model, docs_architecture_validator, docs_architecture_target_detector, docs_architecture_writers, docs_architecture_applier [EXTRACTED 1.00]
- **LibreQoS output targets selected by auto** — readme_auto_target, readme_v2_pair_target, readme_v3_pair_target, readme_v3_network_target [EXTRACTED 1.00]
- **Shadow-to-apply migration from the Python service** — readme_shadow_mode, docs_configuration_runtime_mode, docs_operations_cutover_procedure, docs_operations_rollback_procedure, docs_operations_applied_backups [INFERRED 0.85]

## Communities (9 total, 1 thin omitted)

### Community 0 - "Sync Pipeline Internals"
Cohesion: 0.19
Nodes (12): Architecture document, Applier, Circuit model, Collector, topology.compile_mode = full, Six network modes, Notifier (Telegram), Target selection flowchart (+4 more)

### Community 1 - "Operations and Cutover"
Cohesion: 0.20
Nodes (10): Map from config.json to TOML, Operations document, Applied version backups (last 10), Command reference (check, detect, once, run), Cutover from the Python service, Rollback to the Python service, Troubleshooting table, LQoSync UpdateCSV (Rust tool) (+2 more)

### Community 2 - "LibreQoS v3 Runtime Evidence"
Cohesion: 0.27
Nodes (10): LibreQoS v3 ingress document, Last-good topology retention, liblqos_python.so binding, libreqos_3.0.7 beta package, libreqos-v3-preflight, lqosd (v3 control plane), lqosd inotify file watcher, Offline v3 simulation (+2 more)

### Community 3 - "Output Target Selection"
Cohesion: 0.44
Nodes (9): Target detector, Writers (one per target), [topology] source in lqos.conf, LibreQoS upgrade 2.2 to 3.x, LQoSync UpdateCSV README, target = auto, v2_pair target, v3_network target (+1 more)

### Community 4 - "Brain Vault Notes"
Cohesion: 0.86
Nodes (5): Session 2026-10-31 project setup and v3 simulation, LibreQoS v3 ingress facts, v3 offline simulation method, Handoff note, Brain Home (MOC)

### Community 5 - "Configuration and Secrets"
Cohesion: 0.40
Nodes (4): Config loader, Configuration document, import-json command (planned), /etc/lqosync/updatecsv.toml

### Community 6 - "Speed Rules and Open Decisions"
Cohesion: 0.40
Nodes (5): Operator decisions pending (speed rules, crate location, repo URL), Proposed crate layout, Speed rules, Backstop caps, Open item: speed rule direction and units

### Community 7 - "Installation"
Cohesion: 0.50
Nodes (5): Installation document, install.sh one-line installer, Checks after installation, Removal procedure, lqosync-updatecsv-rs.service

## Knowledge Gaps
- **5 isolated node(s):** `Notifier (Telegram)`, `Checks after installation`, `Removal procedure`, `LibreQoS.py --updateonly no-op in v3`, `Applied version backups (last 10)`
  These have ≤1 connection - possible missing edges or undocumented components. (Counts symbols only; 6 node(s) total have ≤1 connection when file, concept and rationale nodes are included.)
- **1 thin communities (<3 nodes) omitted from report** — run `graphify query` to explore isolated nodes.

## Suggested Questions
_Questions this graph is uniquely positioned to answer:_

- **Why does `Architecture document` connect `Sync Pipeline Internals` to `Output Target Selection`, `Brain Vault Notes`, `Configuration and Secrets`, `Speed Rules and Open Decisions`?**
  _High betweenness centrality (0.333) - this node is a cross-community bridge._
- **What connects `Notifier (Telegram)`, `Checks after installation`, `Removal procedure` to the rest of the system?**
  _5 weakly-connected nodes found - possible documentation gaps or missing edges._
- **Why does `Brain Home (MOC)` connect `Brain Vault Notes` to `Sync Pipeline Internals`, `Operations and Cutover`, `LibreQoS v3 Runtime Evidence`, `Output Target Selection`, `Configuration and Secrets`, `Installation`?**
  _High betweenness centrality (0.326) - this node is a cross-community bridge._
- **Why does `LibreQoS v3 ingress document` connect `LibreQoS v3 Runtime Evidence` to `Sync Pipeline Internals`, `Output Target Selection`, `Brain Vault Notes`?**
  _High betweenness centrality (0.212) - this node is a cross-community bridge._