# Graph Report - lqosync4  (2026-11-04)

## Corpus Check
- Corpus is ~1,611 words - fits in a single context window. You may not need a graph.

## Summary
- 23 nodes · 31 edges · 4 communities
- Extraction: 100% EXTRACTED · 0% INFERRED · 0% AMBIGUOUS
- Token cost: 0 input · 0 output

## Community Hubs (Navigation)
- Operations and Health
- Core and Routing
- Roadmap and Delivery
- Web and Subscribers

## God Nodes (most connected - your core abstractions)
1. `LQoSync4 web UI` - 9 edges
2. `Load Balancer` - 7 edges
3. `LQoSync3 core port` - 4 edges
4. `Destination IP intelligence` - 4 edges
5. `Implementation phases` - 4 edges
6. `Dashboard` - 3 edges
7. `Bandwidth profiles` - 3 edges
8. `Upstream health` - 3 edges
9. `MikroTik core` - 3 edges
10. `IPoE subscribers` - 2 edges

## Surprising Connections (you probably didn't know these)
- `Controlled requirements` --conceptually_related_to--> `LQoSync4 web UI`  [EXTRACTED]
  ste/REQUIREMENTS.md → docs/PRODUCT_SPEC.md
- `Controlled requirements` --conceptually_related_to--> `Load Balancer`  [EXTRACTED]
  ste/REQUIREMENTS.md → docs/PRODUCT_SPEC.md
- `Decision log` --conceptually_related_to--> `LQoSync3 core port`  [EXTRACTED]
  brain/DECISIONS.md → docs/PRODUCT_SPEC.md
- `Decision log` --conceptually_related_to--> `MikroTik core`  [EXTRACTED]
  brain/DECISIONS.md → docs/PRODUCT_SPEC.md
- `Implementation phases` --conceptually_related_to--> `LQoSync3 core port`  [EXTRACTED]
  docs/IMPLEMENTATION_PLAN.md → docs/PRODUCT_SPEC.md

## Communities (4 total, 0 thin omitted)

### Community 0 - "Operations and Health"
Cohesion: 0.33
Nodes (7): Allocations, Audit history, Dashboard, Failover, Upstream health, Load Balancer, Controlled requirements

### Community 1 - "Core and Routing"
Cohesion: 0.33
Nodes (6): Decision log, Assignments, LQoSync3 core port, LibreQoS sync, MikroTik core, Shadow mode

### Community 2 - "Roadmap and Delivery"
Cohesion: 0.40
Nodes (5): Implementation phases, App classification, ASN and organization, CDN identification, Destination IP intelligence

### Community 3 - "Web and Subscribers"
Cohesion: 0.60
Nodes (5): IPoE subscribers, PPPoE users, Bandwidth profiles, Routers, LQoSync4 web UI

## Knowledge Gaps
- **6 isolated node(s):** `Routers`, `Shadow mode`, `Audit history`, `ASN and organization`, `CDN identification` (+1 more)
  These have ≤1 connection - possible missing edges. (Counts symbols only; 6 node(s) total have ≤1 connection when file, concept and rationale nodes are included.)

## Suggested Questions
_Questions this graph is uniquely positioned to answer:_

- **Why does `LQoSync4 web UI` connect `Web and Subscribers` to `Operations and Health`, `Core and Routing`, `Roadmap and Delivery`?**
  _High betweenness centrality (0.431) - this node is a cross-community bridge._
- **What connects `Routers`, `Shadow mode`, `Audit history` to the rest of the system?**
  _6 weakly-connected nodes found - possible documentation gaps or missing edges._
- **Why does `Implementation phases` connect `Roadmap and Delivery` to `Operations and Health`, `Core and Routing`, `Web and Subscribers`?**
  _High betweenness centrality (0.403) - this node is a cross-community bridge._
- **Why does `Load Balancer` connect `Operations and Health` to `Core and Routing`, `Roadmap and Delivery`, `Web and Subscribers`?**
  _High betweenness centrality (0.359) - this node is a cross-community bridge._