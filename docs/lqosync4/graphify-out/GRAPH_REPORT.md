# Graph Report - lqosync4  (2026-11-05)

## Corpus Check
- Corpus is ~24,140 words - fits in a single context window. You may not need a graph.

## Summary
- 78 nodes · 154 edges · 7 communities
- Extraction: 100% EXTRACTED · 0% INFERRED · 0% AMBIGUOUS
- Token cost: 0 input · 0 output

## Graph Freshness
- Built from commit: `731c3f86`
- Run `git rev-parse HEAD` and compare to check if the graph is stale.
- Run `graphify update .` after code changes (no API cost).

## Community Hubs (Navigation)
- Route and component contract and related topics
- Backend and frontend function map and related topics
- Implementation phases and related topics
- Decision log and related topics
- LQoSync4 delivery roadmap and related topics
- Visual and responsive UI map and related topics
- Load Balancer and related topics

## God Nodes (most connected - your core abstractions)
1. `Route and component contract` - 16 edges
2. `LQoSync4 web UI` - 15 edges
3. `Visual and responsive UI map` - 12 edges
4. `Load Balancer` - 11 edges
5. `AppShell` - 11 edges
6. `LQoSync4 delivery roadmap` - 11 edges
7. `Backend and frontend function map` - 10 edges
8. `Implementation phases` - 9 edges
9. `Rust backend` - 8 edges
10. `Rust frontend` - 7 edges

## Surprising Connections (you probably didn't know these)
- `Phase 3 profiles` --documents--> `Bandwidth profiles`  [EXTRACTED]
  roadmap/03-profiles.md → docs/PRODUCT_SPEC.md
- `Agentic engineering rules` --documents--> `Performance and reliability improvements`  [EXTRACTED]
  brain/AGENTIC_ENGINEERING_RULES.md → docs/IMPROVEMENTS.md
- `Phase 1 core port` --documents--> `LQoSync3 core port`  [EXTRACTED]
  roadmap/01-core-port.md → docs/PRODUCT_SPEC.md
- `LQoSync4 delivery roadmap` --documents--> `Phase 6 telemetry and intelligence`  [EXTRACTED]
  ROADMAP.md → roadmap/06-telemetry.md
- `LQoSync4 delivery roadmap` --documents--> `LQoSync4 web UI`  [EXTRACTED]
  ROADMAP.md → docs/PRODUCT_SPEC.md

## Communities (7 total, 0 thin omitted)

### Community 0 - "Route and component contract and related topics"
Cohesion: 0.24
Nodes (17): IPoE subscribers, LibreQoS sync, PPPoE users, Bandwidth profiles, Routers, LQoSync4 web UI, AppShell, Route and component contract (+9 more)

### Community 1 - "Backend and frontend function map and related topics"
Cohesion: 0.18
Nodes (13): Architecture workflows, Backend and frontend function map, Actix Web, Axum, Loco, Rocket, Rust backend, Dioxus (+5 more)

### Community 2 - "Implementation phases and related topics"
Cohesion: 0.24
Nodes (11): Implementation phases, Performance and reliability improvements, App classification, ASN and organization, CDN identification, LQoSync3 core port, Destination IP intelligence, Performance data flow (+3 more)

### Community 3 - "Decision log and related topics"
Cohesion: 0.27
Nodes (10): Decision log, MikroTik core, E01 source and output baseline, E02 first web contract and G0 proposal, Missing Parent Node web alert, Phase 0 evidence and contracts, Phase 2 web foundation, LibreQoS v3 data flow (+2 more)

### Community 4 - "LQoSync4 delivery roadmap and related topics"
Cohesion: 0.22
Nodes (10): Failover, Agent tandem workflow, Production completion gate, LQoSync4 delivery roadmap, Agentic engineering rules, Phase 1 core port, Phase 3 profiles, Phase 4 assignment planner (+2 more)

### Community 5 - "Visual and responsive UI map and related topics"
Cohesion: 0.25
Nodes (9): AllocationCards, CSS starter, Desktop layout, HealthGrid, Mobile layout, ProfileCards, /, SummaryCards (+1 more)

### Community 6 - "Load Balancer and related topics"
Cohesion: 0.36
Nodes (8): Allocations, Assignments, Audit history, Dashboard, Upstream health, Load Balancer, AssignmentTable, /load-balancer

## Knowledge Gaps
- **15 isolated node(s):** `Actix Web`, `Axum`, `Loco`, `Rocket`, `Dioxus` (+10 more)
  These have ≤1 connection - possible missing edges. (Counts symbols only; 15 node(s) total have ≤1 connection when file, concept and rationale nodes are included.)

## Suggested Questions
_Questions this graph is uniquely positioned to answer:_

- **Why does `Route and component contract` connect `Route and component contract and related topics` to `Backend and frontend function map and related topics`, `Decision log and related topics`, `Visual and responsive UI map and related topics`, `Load Balancer and related topics`?**
  _High betweenness centrality (0.232) - this node is a cross-community bridge._
- **What connects `Actix Web`, `Axum`, `Loco` to the rest of the system?**
  _15 weakly-connected nodes found - possible documentation gaps or missing edges._
- **Why does `LQoSync4 web UI` connect `Route and component contract and related topics` to `Backend and frontend function map and related topics`, `Implementation phases and related topics`, `LQoSync4 delivery roadmap and related topics`, `Load Balancer and related topics`?**
  _High betweenness centrality (0.230) - this node is a cross-community bridge._
- **Why does `Backend and frontend function map` connect `Backend and frontend function map and related topics` to `Route and component contract and related topics`, `Implementation phases and related topics`, `Decision log and related topics`, `LQoSync4 delivery roadmap and related topics`?**
  _High betweenness centrality (0.190) - this node is a cross-community bridge._