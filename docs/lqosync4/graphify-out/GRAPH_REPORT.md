# Graph Report - lqosync4  (2026-11-04)

## Corpus Check
- Corpus is ~18,692 words - fits in a single context window. You may not need a graph.

## Summary
- 75 nodes · 141 edges · 6 communities
- Extraction: 100% EXTRACTED · 0% INFERRED · 0% AMBIGUOUS
- Token cost: 0 input · 0 output

## Graph Freshness
- Built from commit: `bae71e25`
- Run `git rev-parse HEAD` and compare to check if the graph is stale.
- Run `graphify update .` after code changes (no API cost).

## Community Hubs (Navigation)
- Load Balancer and related topics
- LQoSync4 delivery roadmap and related topics
- LQoSync4 web UI and related topics
- Backend and frontend function map and related topics
- Visual and responsive UI map and related topics
- LibreQoS sync and related topics

## God Nodes (most connected - your core abstractions)
1. `LQoSync4 web UI` - 14 edges
2. `Route and component contract` - 14 edges
3. `Load Balancer` - 11 edges
4. `AppShell` - 11 edges
5. `Visual and responsive UI map` - 11 edges
6. `LQoSync4 delivery roadmap` - 10 edges
7. `Implementation phases` - 9 edges
8. `Backend and frontend function map` - 9 edges
9. `Rust backend` - 8 edges
10. `LQoSync3 core port` - 7 edges

## Surprising Connections (you probably didn't know these)
- `Phase 3 profiles` --documents--> `Bandwidth profiles`  [EXTRACTED]
  roadmap/03-profiles.md → docs/PRODUCT_SPEC.md
- `LQoSync4 delivery roadmap` --documents--> `Phase 6 telemetry and intelligence`  [EXTRACTED]
  ROADMAP.md → roadmap/06-telemetry.md
- `Phase 2 web foundation` --documents--> `Route and component contract`  [EXTRACTED]
  roadmap/02-web-foundation.md → docs/ROUTES_AND_COMPONENTS.md
- `LQoSync4 delivery roadmap` --documents--> `Implementation phases`  [EXTRACTED]
  ROADMAP.md → docs/IMPLEMENTATION_PLAN.md
- `Agentic engineering rules` --documents--> `Performance and reliability improvements`  [EXTRACTED]
  brain/AGENTIC_ENGINEERING_RULES.md → docs/IMPROVEMENTS.md

## Communities (6 total, 0 thin omitted)

### Community 0 - "Load Balancer and related topics"
Cohesion: 0.19
Nodes (15): Implementation phases, Performance and reliability improvements, App classification, ASN and organization, Audit history, CDN identification, Destination IP intelligence, Load Balancer (+7 more)

### Community 1 - "LQoSync4 delivery roadmap and related topics"
Cohesion: 0.18
Nodes (14): Decision log, Assignments, LQoSync3 core port, Failover, Upstream health, MikroTik core, Shadow mode, LQoSync4 delivery roadmap (+6 more)

### Community 2 - "LQoSync4 web UI and related topics"
Cohesion: 0.30
Nodes (14): IPoE subscribers, PPPoE users, Bandwidth profiles, Routers, LQoSync4 web UI, AppShell, Route and component contract, /ipoe/profiles (+6 more)

### Community 3 - "Backend and frontend function map and related topics"
Cohesion: 0.18
Nodes (13): Architecture workflows, Backend and frontend function map, Actix Web, Axum, Loco, Rocket, Rust backend, Dioxus (+5 more)

### Community 4 - "Visual and responsive UI map and related topics"
Cohesion: 0.19
Nodes (13): Allocations, Dashboard, AllocationCards, AssignmentTable, CSS starter, Desktop layout, HealthGrid, Mobile layout (+5 more)

### Community 5 - "LibreQoS sync and related topics"
Cohesion: 0.33
Nodes (6): LibreQoS sync, /libreqos, SourcePreview, LibreQoS v3 data flow, v3 native JSON input, v3 CSV + network JSON input

## Knowledge Gaps
- **16 isolated node(s):** `App classification`, `ASN and organization`, `CDN identification`, `Shadow mode`, `Actix Web` (+11 more)
  These have ≤1 connection - possible missing edges. (Counts symbols only; 16 node(s) total have ≤1 connection when file, concept and rationale nodes are included.)

## Suggested Questions
_Questions this graph is uniquely positioned to answer:_

- **Why does `Route and component contract` connect `LQoSync4 web UI and related topics` to `LQoSync4 delivery roadmap and related topics`, `Backend and frontend function map and related topics`, `Visual and responsive UI map and related topics`, `LibreQoS sync and related topics`?**
  _High betweenness centrality (0.231) - this node is a cross-community bridge._
- **What connects `App classification`, `ASN and organization`, `CDN identification` to the rest of the system?**
  _16 weakly-connected nodes found - possible documentation gaps or missing edges._
- **Why does `LQoSync4 web UI` connect `LQoSync4 web UI and related topics` to `Load Balancer and related topics`, `Backend and frontend function map and related topics`, `Visual and responsive UI map and related topics`, `LibreQoS sync and related topics`?**
  _High betweenness centrality (0.215) - this node is a cross-community bridge._
- **Why does `Backend and frontend function map` connect `Backend and frontend function map and related topics` to `Load Balancer and related topics`, `LQoSync4 delivery roadmap and related topics`, `LQoSync4 web UI and related topics`, `LibreQoS sync and related topics`?**
  _High betweenness centrality (0.201) - this node is a cross-community bridge._