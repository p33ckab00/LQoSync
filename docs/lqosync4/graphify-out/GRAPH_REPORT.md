# Graph Report - lqosync4  (2026-11-04)

## Corpus Check
- Corpus is ~15,341 words - fits in a single context window. You may not need a graph.

## Summary
- 67 nodes · 124 edges · 6 communities
- Extraction: 100% EXTRACTED · 0% INFERRED · 0% AMBIGUOUS
- Token cost: 0 input · 0 output

## Graph Freshness
- Built from commit: `ed88e618`
- Run `git rev-parse HEAD` and compare to check if the graph is stale.
- Run `graphify update .` after code changes (no API cost).

## Community Hubs (Navigation)
- LQoSync4 web UI and related topics
- Backend and frontend function map and related topics
- Load Balancer and related topics
- Implementation phases and related topics
- Visual and responsive UI map and related topics
- LibreQoS sync and related topics

## God Nodes (most connected - your core abstractions)
1. `LQoSync4 web UI` - 14 edges
2. `Route and component contract` - 13 edges
3. `Load Balancer` - 11 edges
4. `AppShell` - 11 edges
5. `Visual and responsive UI map` - 11 edges
6. `Backend and frontend function map` - 9 edges
7. `Rust backend` - 8 edges
8. `Implementation phases` - 8 edges
9. `Rust frontend` - 7 edges
10. `/load-balancer` - 7 edges

## Surprising Connections (you probably didn't know these)
- `Controlled requirements` --conceptually_related_to--> `LQoSync4 web UI`  [EXTRACTED]
  ste/REQUIREMENTS.md → docs/PRODUCT_SPEC.md
- `Agent tandem workflow` --documents--> `Backend and frontend function map`  [EXTRACTED]
  brain/AGENT_WORKFLOW.md → docs/FEATURE_FUNCTION_MAP.md
- `STE WH and How answers` --conceptually_related_to--> `Backend and frontend function map`  [EXTRACTED]
  ste/WH_AND_HOW.md → docs/FEATURE_FUNCTION_MAP.md
- `Decision log` --conceptually_related_to--> `LQoSync3 core port`  [EXTRACTED]
  brain/DECISIONS.md → docs/PRODUCT_SPEC.md
- `Agentic engineering rules` --documents--> `Performance and reliability improvements`  [EXTRACTED]
  brain/AGENTIC_ENGINEERING_RULES.md → docs/IMPROVEMENTS.md

## Communities (6 total, 0 thin omitted)

### Community 0 - "LQoSync4 web UI and related topics"
Cohesion: 0.28
Nodes (15): IPoE subscribers, PPPoE users, Bandwidth profiles, Routers, LQoSync4 web UI, AppShell, Route and component contract, /ipoe/profiles (+7 more)

### Community 1 - "Backend and frontend function map and related topics"
Cohesion: 0.18
Nodes (13): Architecture workflows, Backend and frontend function map, Actix Web, Axum, Loco, Rocket, Rust backend, Dioxus (+5 more)

### Community 2 - "Load Balancer and related topics"
Cohesion: 0.23
Nodes (12): Decision log, Allocations, Assignments, Audit history, Dashboard, Failover, Upstream health, Load Balancer (+4 more)

### Community 3 - "Implementation phases and related topics"
Cohesion: 0.23
Nodes (12): Implementation phases, Performance and reliability improvements, App classification, ASN and organization, CDN identification, LQoSync3 core port, Destination IP intelligence, Performance data flow (+4 more)

### Community 4 - "Visual and responsive UI map and related topics"
Cohesion: 0.20
Nodes (11): AllocationCards, AssignmentTable, CSS starter, Desktop layout, HealthGrid, Mobile layout, ProfileCards, / (+3 more)

### Community 5 - "LibreQoS sync and related topics"
Cohesion: 0.50
Nodes (4): LibreQoS sync, LibreQoS v3 data flow, v3 native JSON input, v3 CSV + network JSON input

## Knowledge Gaps
- **16 isolated node(s):** `Actix Web`, `Axum`, `Loco`, `Rocket`, `Dioxus` (+11 more)
  These have ≤1 connection - possible missing edges. (Counts symbols only; 16 node(s) total have ≤1 connection when file, concept and rationale nodes are included.)

## Suggested Questions
_Questions this graph is uniquely positioned to answer:_

- **Why does `LQoSync4 web UI` connect `LQoSync4 web UI and related topics` to `Backend and frontend function map and related topics`, `Load Balancer and related topics`, `Implementation phases and related topics`, `LibreQoS sync and related topics`?**
  _High betweenness centrality (0.253) - this node is a cross-community bridge._
- **What connects `Actix Web`, `Axum`, `Loco` to the rest of the system?**
  _16 weakly-connected nodes found - possible documentation gaps or missing edges._
- **Why does `Backend and frontend function map` connect `Backend and frontend function map and related topics` to `LQoSync4 web UI and related topics`, `Implementation phases and related topics`, `LibreQoS sync and related topics`?**
  _High betweenness centrality (0.231) - this node is a cross-community bridge._
- **Why does `Route and component contract` connect `LQoSync4 web UI and related topics` to `Backend and frontend function map and related topics`, `Load Balancer and related topics`, `Visual and responsive UI map and related topics`?**
  _High betweenness centrality (0.228) - this node is a cross-community bridge._