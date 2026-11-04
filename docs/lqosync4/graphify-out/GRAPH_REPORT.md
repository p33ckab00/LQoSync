# Graph Report - lqosync4  (2026-11-04)

## Corpus Check
- Corpus is ~10,808 words - fits in a single context window. You may not need a graph.

## Summary
- 64 nodes · 117 edges · 8 communities
- Extraction: 100% EXTRACTED · 0% INFERRED · 0% AMBIGUOUS
- Token cost: 0 input · 0 output

## Graph Freshness
- Built from commit: `6c4d7cf0`
- Run `git rev-parse HEAD` and compare to check if the graph is stale.
- Run `graphify update .` after code changes (no API cost).

## Community Hubs (Navigation)
- LQoSync4 web UI and related topics
- Backend and frontend function map and related topics
- Visual and responsive UI map and related topics
- Load Balancer and related topics
- LibreQoS sync and related topics
- Destination IP intelligence and related topics
- Implementation phases and related topics
- Performance data flow and related topics

## God Nodes (most connected - your core abstractions)
1. `LQoSync4 web UI` - 14 edges
2. `Route and component contract` - 13 edges
3. `Load Balancer` - 11 edges
4. `AppShell` - 11 edges
5. `Visual and responsive UI map` - 11 edges
6. `Backend and frontend function map` - 8 edges
7. `Rust backend` - 8 edges
8. `Rust frontend` - 7 edges
9. `Implementation phases` - 7 edges
10. `/load-balancer` - 7 edges

## Surprising Connections (you probably didn't know these)
- `Controlled requirements` --conceptually_related_to--> `Load Balancer`  [EXTRACTED]
  ste/REQUIREMENTS.md → docs/PRODUCT_SPEC.md
- `Decision log` --conceptually_related_to--> `LQoSync3 core port`  [EXTRACTED]
  brain/DECISIONS.md → docs/PRODUCT_SPEC.md
- `Controlled requirements` --conceptually_related_to--> `LQoSync4 web UI`  [EXTRACTED]
  ste/REQUIREMENTS.md → docs/PRODUCT_SPEC.md
- `STE WH and How answers` --conceptually_related_to--> `Backend and frontend function map`  [EXTRACTED]
  ste/WH_AND_HOW.md → docs/FEATURE_FUNCTION_MAP.md
- `Visual and responsive UI map` --documents--> `CSS starter`  [EXTRACTED]
  docs/UI_VISUAL_SYSTEM.md → ui-reference.css

## Communities (8 total, 0 thin omitted)

### Community 0 - "LQoSync4 web UI and related topics"
Cohesion: 0.27
Nodes (15): IPoE subscribers, PPPoE users, Bandwidth profiles, Routers, LQoSync4 web UI, AppShell, Route and component contract, /ipoe/profiles (+7 more)

### Community 1 - "Backend and frontend function map and related topics"
Cohesion: 0.18
Nodes (13): Architecture workflows, Backend and frontend function map, Actix Web, Axum, Loco, Rocket, Rust backend, Dioxus (+5 more)

### Community 2 - "Visual and responsive UI map and related topics"
Cohesion: 0.19
Nodes (13): Allocations, Dashboard, AllocationCards, AssignmentTable, CSS starter, Desktop layout, HealthGrid, Mobile layout (+5 more)

### Community 3 - "Load Balancer and related topics"
Cohesion: 0.33
Nodes (7): Decision log, Assignments, Audit history, Failover, Upstream health, Load Balancer, MikroTik core

### Community 4 - "LibreQoS sync and related topics"
Cohesion: 0.33
Nodes (6): LibreQoS sync, /libreqos, SourcePreview, LibreQoS v3 data flow, v3 native JSON input, v3 CSV + network JSON input

### Community 5 - "Destination IP intelligence and related topics"
Cohesion: 0.50
Nodes (4): App classification, ASN and organization, CDN identification, Destination IP intelligence

### Community 6 - "Implementation phases and related topics"
Cohesion: 0.67
Nodes (3): Implementation phases, LQoSync3 core port, Shadow mode

### Community 7 - "Performance data flow and related topics"
Cohesion: 1.00
Nodes (3): Performance and reliability improvements, Performance data flow, Web UI performance and readability

## Knowledge Gaps
- **16 isolated node(s):** `Actix Web`, `Axum`, `Loco`, `Rocket`, `Dioxus` (+11 more)
  These have ≤1 connection - possible missing edges. (Counts symbols only; 16 node(s) total have ≤1 connection when file, concept and rationale nodes are included.)

## Suggested Questions
_Questions this graph is uniquely positioned to answer:_

- **Why does `LQoSync4 web UI` connect `LQoSync4 web UI and related topics` to `Backend and frontend function map and related topics`, `Visual and responsive UI map and related topics`, `Load Balancer and related topics`, `LibreQoS sync and related topics`, `Implementation phases and related topics`, `Performance data flow and related topics`?**
  _High betweenness centrality (0.265) - this node is a cross-community bridge._
- **What connects `Actix Web`, `Axum`, `Loco` to the rest of the system?**
  _16 weakly-connected nodes found - possible documentation gaps or missing edges._
- **Why does `Route and component contract` connect `LQoSync4 web UI and related topics` to `Backend and frontend function map and related topics`, `Visual and responsive UI map and related topics`, `LibreQoS sync and related topics`?**
  _High betweenness centrality (0.230) - this node is a cross-community bridge._
- **Why does `Backend and frontend function map` connect `Backend and frontend function map and related topics` to `LQoSync4 web UI and related topics`, `LibreQoS sync and related topics`, `Implementation phases and related topics`?**
  _High betweenness centrality (0.214) - this node is a cross-community bridge._