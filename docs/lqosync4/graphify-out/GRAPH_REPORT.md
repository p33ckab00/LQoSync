# Graph Report - lqosync4  (2026-11-05)

## Corpus Check
- Corpus is ~25,228 words - fits in a single context window. You may not need a graph.

## Summary
- 79 nodes · 158 edges · 6 communities
- Extraction: 100% EXTRACTED · 0% INFERRED · 0% AMBIGUOUS
- Token cost: 0 input · 0 output

## Graph Freshness
- Built from commit: `64f18ebf`
- Run `git rev-parse HEAD` and compare to check if the graph is stale.
- Run `graphify update .` after code changes (no API cost).

## Community Hubs (Navigation)
- Load Balancer and related topics
- Route and component contract and related topics
- Backend and frontend function map and related topics
- Visual and responsive UI map and related topics
- LibreQoS v3 data flow and related topics
- LQoSync3 core port and related topics

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
- `LQoSync4 delivery roadmap` --documents--> `Phase 5 health and apply`  [EXTRACTED]
  ROADMAP.md → roadmap/05-health-apply.md
- `LQoSync4 delivery roadmap` --documents--> `LQoSync4 web UI`  [EXTRACTED]
  ROADMAP.md → docs/PRODUCT_SPEC.md
- `Controlled requirements` --conceptually_related_to--> `LQoSync4 web UI`  [EXTRACTED]
  ste/REQUIREMENTS.md → docs/PRODUCT_SPEC.md
- `Agent tandem workflow` --documents--> `Backend and frontend function map`  [EXTRACTED]
  brain/AGENT_WORKFLOW.md → docs/FEATURE_FUNCTION_MAP.md

## Communities (6 total, 0 thin omitted)

### Community 0 - "Load Balancer and related topics"
Cohesion: 0.16
Nodes (19): Implementation phases, Performance and reliability improvements, App classification, ASN and organization, Assignments, Audit history, CDN identification, Destination IP intelligence (+11 more)

### Community 1 - "Route and component contract and related topics"
Cohesion: 0.24
Nodes (17): IPoE subscribers, LibreQoS sync, PPPoE users, Bandwidth profiles, Routers, LQoSync4 web UI, AppShell, Route and component contract (+9 more)

### Community 2 - "Backend and frontend function map and related topics"
Cohesion: 0.18
Nodes (13): Architecture workflows, Backend and frontend function map, Actix Web, Axum, Loco, Rocket, Rust backend, Dioxus (+5 more)

### Community 3 - "Visual and responsive UI map and related topics"
Cohesion: 0.19
Nodes (13): Allocations, Dashboard, AllocationCards, AssignmentTable, CSS starter, Desktop layout, HealthGrid, Mobile layout (+5 more)

### Community 4 - "LibreQoS v3 data flow and related topics"
Cohesion: 0.27
Nodes (10): E01 source and output baseline, E02 first web contract and G0 proposal, E03 synthetic full-cycle performance baseline, Missing Parent Node web alert, Phase 0 evidence and contracts, Phase 1 core port, Phase 2 web foundation, LibreQoS v3 data flow (+2 more)

### Community 5 - "LQoSync3 core port and related topics"
Cohesion: 0.29
Nodes (7): Decision log, LQoSync3 core port, Failover, Upstream health, MikroTik core, Shadow mode, Phase 5 health and apply

## Knowledge Gaps
- **15 isolated node(s):** `Actix Web`, `Axum`, `Loco`, `Rocket`, `Dioxus` (+10 more)
  These have ≤1 connection - possible missing edges. (Counts symbols only; 15 node(s) total have ≤1 connection when file, concept and rationale nodes are included.)

## Suggested Questions
_Questions this graph is uniquely positioned to answer:_

- **Why does `Route and component contract` connect `Route and component contract and related topics` to `Backend and frontend function map and related topics`, `Visual and responsive UI map and related topics`, `LibreQoS v3 data flow and related topics`?**
  _High betweenness centrality (0.227) - this node is a cross-community bridge._
- **What connects `Actix Web`, `Axum`, `Loco` to the rest of the system?**
  _15 weakly-connected nodes found - possible documentation gaps or missing edges._
- **Why does `LQoSync4 web UI` connect `Route and component contract and related topics` to `Load Balancer and related topics`, `Backend and frontend function map and related topics`, `Visual and responsive UI map and related topics`?**
  _High betweenness centrality (0.225) - this node is a cross-community bridge._
- **Why does `Backend and frontend function map` connect `Backend and frontend function map and related topics` to `Load Balancer and related topics`, `Route and component contract and related topics`, `LibreQoS v3 data flow and related topics`, `LQoSync3 core port and related topics`?**
  _High betweenness centrality (0.184) - this node is a cross-community bridge._