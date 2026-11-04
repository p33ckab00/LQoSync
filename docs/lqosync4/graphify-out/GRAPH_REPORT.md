# Graph Report - lqosync4  (2026-11-04)

## Corpus Check
- Corpus is ~5,454 words - fits in a single context window. You may not need a graph.

## Summary
- 38 nodes · 60 edges · 6 communities
- Extraction: 100% EXTRACTED · 0% INFERRED · 0% AMBIGUOUS
- Token cost: 0 input · 0 output

## Community Hubs (Navigation)
- Operations and Assignments
- Core and Performance
- Web and Subscribers
- Rust API Frameworks
- Rust UI Frameworks
- IP Intelligence Roadmap

## God Nodes (most connected - your core abstractions)
1. `LQoSync4 web UI` - 13 edges
2. `Load Balancer` - 10 edges
3. `Rust backend` - 8 edges
4. `Rust frontend` - 7 edges
5. `Implementation phases` - 7 edges
6. `LQoSync3 core port` - 6 edges
7. `Backend and frontend function map` - 6 edges
8. `Destination IP intelligence` - 5 edges
9. `Performance data flow` - 5 edges
10. `Architecture workflows` - 4 edges

## Surprising Connections (you probably didn't know these)
- `Decision log` --conceptually_related_to--> `LQoSync3 core port`  [EXTRACTED]
  brain/DECISIONS.md → docs/PRODUCT_SPEC.md
- `Controlled requirements` --conceptually_related_to--> `LQoSync4 web UI`  [EXTRACTED]
  ste/REQUIREMENTS.md → docs/PRODUCT_SPEC.md
- `Controlled requirements` --conceptually_related_to--> `Load Balancer`  [EXTRACTED]
  ste/REQUIREMENTS.md → docs/PRODUCT_SPEC.md
- `STE WH and How answers` --conceptually_related_to--> `Backend and frontend function map`  [EXTRACTED]
  ste/WH_AND_HOW.md → docs/FEATURE_FUNCTION_MAP.md
- `Decision log` --conceptually_related_to--> `MikroTik core`  [EXTRACTED]
  brain/DECISIONS.md → docs/PRODUCT_SPEC.md

## Communities (6 total, 0 thin omitted)

### Community 0 - "Operations and Assignments"
Cohesion: 0.24
Nodes (10): Decision log, Allocations, Assignments, Audit history, Dashboard, Failover, Upstream health, Load Balancer (+2 more)

### Community 1 - "Core and Performance"
Cohesion: 0.24
Nodes (10): Implementation phases, Performance and reliability improvements, App classification, ASN and organization, CDN identification, LQoSync3 core port, Destination IP intelligence, LibreQoS sync (+2 more)

### Community 2 - "Web and Subscribers"
Cohesion: 0.60
Nodes (5): IPoE subscribers, PPPoE users, Bandwidth profiles, Routers, LQoSync4 web UI

### Community 3 - "Rust API Frameworks"
Cohesion: 0.40
Nodes (5): Actix Web, Axum, Loco, Rocket, Rust backend

### Community 4 - "Rust UI Frameworks"
Cohesion: 0.40
Nodes (5): Dioxus, Leptos, Rust frontend, Sycamore, Yew

### Community 5 - "IP Intelligence Roadmap"
Cohesion: 1.00
Nodes (3): Architecture workflows, Backend and frontend function map, STE WH and How answers

## Knowledge Gaps
- **13 isolated node(s):** `Routers`, `Shadow mode`, `Audit history`, `ASN and organization`, `CDN identification` (+8 more)
  These have ≤1 connection - possible missing edges. (Counts symbols only; 13 node(s) total have ≤1 connection when file, concept and rationale nodes are included.)

## Suggested Questions
_Questions this graph is uniquely positioned to answer:_

- **Why does `LQoSync4 web UI` connect `Web and Subscribers` to `Operations and Assignments`, `Core and Performance`, `Rust API Frameworks`, `Rust UI Frameworks`, `IP Intelligence Roadmap`?**
  _High betweenness centrality (0.376) - this node is a cross-community bridge._
- **What connects `Routers`, `Shadow mode`, `Audit history` to the rest of the system?**
  _13 weakly-connected nodes found - possible documentation gaps or missing edges._
- **Why does `Implementation phases` connect `Core and Performance` to `Operations and Assignments`, `Web and Subscribers`, `Rust API Frameworks`, `Rust UI Frameworks`?**
  _High betweenness centrality (0.265) - this node is a cross-community bridge._
- **Why does `Load Balancer` connect `Operations and Assignments` to `Core and Performance`, `Web and Subscribers`, `IP Intelligence Roadmap`?**
  _High betweenness centrality (0.260) - this node is a cross-community bridge._