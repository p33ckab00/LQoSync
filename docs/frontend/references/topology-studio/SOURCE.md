# Supplied topology design — extracted source

Source: [original ZIP](../../images/LQoSync_Complete_Design_and_SVGs.zip).

This extraction preserves the supplied text. Tables use Markdown. Embedded figures remain in the DOCX.
This source records proposals. It does not establish implementation or deployment status.

LQoSync Topology Manager

Consolidated Product Design & Technical Specification

Configuration-Driven MikroTik Discovery · Infinite Canvas · Sankey Network Tree · Read/Write Policies · Validation · Context-Aware Guide

Version: Design synthesis | 23 November 2026

Scope: Full design discussion from Sankey topology brainstorming through the information (i) guide. All diagrams and the available original SVG are included.

Figure 1 — Original topology studio visual mockup (also included as editable SVG).

## Executive vision

LQoSync (LibreQoS Synchronization System) is the configuration and orchestration layer. It reads MikroTik and LQoSync policy data, constructs an effective shaping topology, and generates network.json and ShapedDevices.csv. LibreQoS—not LQoSync—performs traffic shaping. The desired UX is a configuration-driven, zoomable canvas that visualizes WANs, routers, subscriber groups, circuits, and Sankey-style flow relationships.

## Core decisions

The router is the entry point for source discovery and configuration; source policies determine what to read, filter, group, and generate.

Router settings, source-policy settings, and node-capacity settings are separate scopes.

Read-only preview is available before committing changes; a saved configuration can be loaded into the canvas.

Auto-discovered routers and eligible groups appear on the canvas, with retained stable IDs and settings.

WAN detection yields candidates with evidence, not unquestionable classifications; the operator can pin WAN sources.

An information (i) icon opens optional, context-aware guidance rather than enforcing a mandatory wizard.

## 1. Current repository baseline

Repository: https://github.com/p33ckab00/LQoSync.git | branch: lqosync3. The earlier review in this conversation inspected the branch around 23 November 2026; these are code observations, not a fresh production deployment audit.

| Area | Observed current implementation | Design extension |
| --- | --- | --- |
| Frontend | Rust/Leptos/Wasm topology canvas with cards, connections, modals | Keep framework; extend policy editors and preview layers |
| Canvas dock | Inventory, Settings, Allocation, Preview, Excluded | Add contextual info guide and saved-view load affordance |
| Router registration | Connection → Sources → Review; new router saved disabled | Test Connection and explicit enable; finer-grained policies |
| WAN discovery | /interface plus active default-route observations | Evidence-ranked candidate classification and persistent pinning |
| Live grouping | Active PPP profiles, DHCP server groups, Hotspot groups | Configurable field-based grouping and source queries |
| Node settings | Capacity, factors, allocation, exclusion | Explicit speed resolver and node capacity formulas |
| Canvas flow width | Subscriber count-based strokes | Selectable subscriber count / capacity / throughput Sankey |
| Publishing | Existing topology compiler and writer ownership | Reuse single validated publishing path |

## 2. Canvas interaction model

A floating dock provides Inventory, Settings (cog), Allocation, Preview, Load Configuration, Excluded, and Information (i). A router card has its own cog for per-router settings. Generated groups have their own cog for capacity, subscriber rules, and parent mapping. All settings appear in modals or a spacious drawer while the infinite canvas remains visible.

Use visual layers: physical/logical inventory, effective shaping tree, and traffic/metrics overlay. The candidate preview uses dashed outlines and status markers; Sankey thickness is reserved for the selected metric. At zoom-out, aggregate thousands of circuits into groups and preserve a stable layout.

Figure 2 — Three-layer Sankey canvas model.

## 3. Router registration and credentials

Add router: name, host/IP, RouterOS API/SSL port, TLS verification, read credential, and Test Connection.

Save registration without implicitly enabling discovery or granting write actions; enable explicitly after connection and permission checks.

Credential strategies: separate read/write accounts (recommended) or one shared account with explicitly authorized operations.

Each source policy references an access profile. Store secrets server-side; UI receives only opaque references and masked state.

Read-only discovery never invokes RouterOS writes; any write policy must be explicitly enabled, scoped, audited, and permission-checked.

## 4. Router source policy engine

Unified flow: Router → Source → Read tables/joins → Filter/query → Grouping/node generation → Identity resolution → Speed resolution → Parent mapping → Candidate export.

| Source | Example reads | Example grouping | Notes |
| --- | --- | --- | --- |
| PPP / PPPoE | /ppp/secret + /ppp/active + /ppp/profile | PPP profile, service, comment token | Support active-only, configured-only, joined mode; RADIUS-only active users |
| DHCP / IPoE | /ip/dhcp-server/lease + /ip/dhcp-server + interfaces/VLAN | DHCP server, VLAN, pool, network | Dynamic lease metadata stored in LQoSync; IP changes should not create new identity |
| Hotspot | /ip/hotspot/active + user + user/profile | Hotspot server, user profile, comment | Optional MAC inclusion; resolve duplicate MACs and active session state |
| WAN | /interface + /ip/route and optional PPP/DHCP client context | Pinned WAN interface | Do not assume all physical ports are WAN |

### 4.1 Policy builder controls

Policy name, enabled state, priority, access profile, source type and read/write operation permissions.

Read mode: active/configured/both, query path and joins, filter field/operator/value, AND/OR rules, optional safe regex.

Group-by field and node naming template; allow match field to differ from grouping field.

Stable subscriber identity strategy, deduplication rules, missing-record grace, and exclusions.

Speed plan resolution precedence, defaults, conflict behavior, parent topology assignment and preview diagnostics.

### 4.2 Illustrative rule examples

| Match condition | Generated behavior |
| --- | --- |
| PPP profile starts PLAN- | Group by PPP profile, map plan rate per subscriber |
| PPP comment contains SITE: | Extract site token and group by site |
| DHCP lease server = dhcp-vlan3001 | Map to IPoE-VLAN3001 node |
| DHCP MAC + VLAN matches subscriber registry | Keep one household circuit with dynamic IP update |
| Hotspot user profile = guest | Group under Guest Hotspot node |

## 5. WAN discovery and pinning

Evidence may include an active 0.0.0.0/0 route, resolved outgoing interface, active PPPoE client, DHCP client gateway, or upstream VLAN. Interface names are only weak hints. Display candidate confidence, evidence, route table, interface status, and last observation; offer Pin as WAN, Ignore, and manual selection.

Distinguish ‘Pin WAN source’ (classifies an interface) from ‘Pin subscriber to WAN’ (subscriber allocation preference). A WAN pin does not itself change MikroTik routing. WAN capacity must be set and validated before use as a shaping parent.

Figure 3 — Configuration workflow, including WAN discovery and load.

## 6. Read-only discovery preview

A Preview action opens an immutable, read-only candidate topology over the canvas. It must not mutate MikroTik, active topology, or LibreQoS outputs. Cache per-router/source snapshots with collection timestamps, completeness flags, and revision identifiers; edit rules against cached data without refetching on every keystroke.

## 6.1 Preview table

| Field | Meaning |
| --- | --- |
| Source record | PPP account/session, DHCP lease, Hotspot session or WAN interface |
| Matched rule | Policy ID, filter evaluation and rule precedence |
| Identity | Resolved stable circuit/subscriber ID |
| Generated node | Group key, display label and proposed parent |
| Speed source | Explicit override, mapping, effective session, profile or fallback |
| Status | Valid, warning, blocker, excluded or stale |
| Provenance | Router, endpoint, snapshot ID and field references |

## 6.2 Validation gates

Blockers: cycles, invalid parent, duplicate effective circuit ownership, unsupported export schema, unresolved mandatory speed, conflicting IP ownership within the same routing context.

Warnings: stale data, fallback plan use, offline PPP account, uncertain WAN evidence, unconfirmed destructive change.

Informational: dynamic DHCP lease resolved through LQoSync metadata, inherited settings, unchanged groups.

Only complete successful source reads may authorize absence-based removal; failed or partial reads cannot remove subscribers.

Preview active/proposed/diff modes. Any structural revision mismatch requires revalidation before publish.

Figure 4 — Snapshot-to-validation pipeline.

## 7. Save, Load Configuration, and publish semantics

Save Source Configuration persists policies. Preview shows candidate state. Load Configuration restores a saved effective canvas view and policy revision. This is not equivalent to publishing network.json or ShapedDevices.csv.

The inspected canvas design currently auto-saves/applies valid ordinary edits. Preserve that behavior unless explicitly changed through a migration plan; do not silently introduce a mandatory preview approval gate. Publication should continue through the existing validated writer, with audit records, staging, hashes, rollback, and safe reconciliation.

## 8. Auto-generated router and group cards

Router cards may be auto-added when active subscriber sources exist. PPP groups may follow active PPP profiles; DHCP groups follow shaped leases/server; Hotspot groups follow active sessions. Preserve inactive groups’ settings, parent relationships, WAN connections, and layout during grace periods. WAN-only core routers can be represented by their WAN cards, according to the existing design.

## 8.1 Exclude router from network tree

The router card may remain visible on the canvas while its router node is omitted from network.json. Child source groups can be promoted to the router’s effective parent (e.g., WAN). This is different from disabling router discovery or excluding a source from shaping. The UI must explain these distinct toggles.

## 8.2 Node configuration

| Setting | Expected behavior |
| --- | --- |
| Subscriber plan rate | Per-circuit upload/download rates, normalized from RouterOS or explicit plan mapping |
| Node capacity | Fixed aggregate rate or explicit computed policy |
| Concurrency factor | Optional aggregate-demand multiplier, not a universal guaranteed formula |
| Minimum rate | Configurable floor with validation |
| WAN allocation | Count, Capacity or Quality; preserve a single effective circuit identity |
| Multi-WAN budget | Avoid double-counting shared physical capacity |
| Parent assignment | Valid acyclic shaping tree with effective parent per circuit |

Illustrative calculation: 21 subscribers × 15 Mbps × 0.5 concurrency = 157.5 Mbps aggregate target. This is a sample planning formula only. Upload and download must be calculated independently, and profile rate-limit is not automatically the aggregate node ceiling.

## 9. Sankey visual specification

Metric selector: subscriber count, provisioned capacity, measured download, measured upload, utilization.

Scale edge widths by the selected metric with explicit units and legend. Never portray an estimated capacity as observed throughput.

Use separate edge types or overlays for observed physical link, shaping parent, and live flow.

Use collapsible groups, virtualization, and stable node positions for large deployments.

Show stale telemetry and no-data states rather than reusing outdated measurements without marking them.

Highlight proposed additions, removals, and modifications using borders, line patterns and icons rather than altering traffic-width semantics.

## 10. Context-aware information guide

Place an (i) icon in the canvas floating toolbar. It opens an optional help drawer/panel without changing the topology or forcing a wizard. The guide has two modes: Getting Started (full sequence) and About This Setting (help for the currently selected field, router, source policy, WAN or node).

Figure 5 — Context-aware guide behavior.

| Guide topic | Help content |
| --- | --- |
| Add Router | Connection, test, save, explicit enable |
| Access profiles | Read vs write privileges, shared credential risks |
| Source policies | How filters, grouping and speed resolution interact |
| WAN classification | Why an interface was suggested and how to pin it |
| Preview | What is read-only and what changes only after saving |
| Node capacity | Per-subscriber plan vs aggregate node budget |
| Validation | Blocker/warning descriptions and corrective steps |

## 11. Proposed backend contracts

| Object | Responsibility |
| --- | --- |
| RouterAccessProfile | Secret reference, transport, permission scope |
| RouterSourcePolicy | Source type, query, filters, grouping, speed, write authorization |
| DiscoverySnapshot | Immutable read results with timestamps and completeness |
| WanCandidate | Interface, route evidence, confidence and pinned state |
| CandidateTopology | Proposed graph with provenance and export assignments |
| ValidationReport | Rule ID, severity, entity, explanation, suggested fix |
| SavedCanvasView | Positions, zoom, collapsed groups, selected overlays |
| PublishRecord | Revision, output hashes, writer result, rollback reference |

## 12. Recommended implementation order

Phase A: information guide and navigation affordances; minimal backend risk.

Phase B: router configuration drawer, access profiles and Test Connection.

Phase C: versioned source policy schema, rule evaluator and snapshot preview.

Phase D: WAN candidate evidence, manual pinning and policy-derived canvas nodes.

Phase E: node capacity configuration and Sankey metric modes.

Phase F: load saved view, diff/validation integration and production migration tests.

## 13. Safety and acceptance criteria

Read-only preview never writes to RouterOS or LibreQoS files.

Existing topology remains intact after a failed, partial or stale router read.

Router exclusion and source exclusion have distinct visible consequences.

Every generated circuit has a stable identity, speed provenance and valid effective parent.

Publishing retains one authoritative writer; no parallel output owner is introduced.

All user-visible guides are optional and context-aware; canvas remains navigable.

Preview, saved policy, effective graph, and live telemetry are distinguishable.

## Appendix A — Included visual design files

The following SVG source files are supplied alongside this document. Their PNG renderings appear in the body, including the original studio mockup available from the earlier conversation.

lqosync_topology_studio.svg — Original topology studio mockup: WAN/router/source-group Sankey canvas and configuration.

lqosync_configuration_workflow.svg — Router-to-canvas configuration workflow.

lqosync_discovery_pipeline.svg — Read-only discovery, candidate generation and validation.

lqosync_sankey_layers.svg — Physical, shaping and telemetry layers.

lqosync_information_guide.svg — Information (i) guide and context-aware help.

## Appendix B — Source references

GitHub branch: https://github.com/p33ckab00/LQoSync/tree/lqosync3

Code areas inspected in the earlier design review: crates/web-ui/src/topology_canvas.rs; crates/web-ui/src/router_form.rs; crates/web-ui/src/router_form_model.rs; crates/web-contract/src/routers.rs; crates/web-contract/src/topology.rs; src/topology_manager/wan.rs.

Design documents consulted: docs/frontend/TOPOLOGY_CANVAS_INTEGRATION.md; docs/superpowers/specs/2026-11-22-live-source-canvas-design.md.

This document consolidates product requirements and proposals from the conversation. Sample values, rule examples, UI mockups and diagrams are illustrative. It does not claim that all proposed features already exist or that any production changes have been deployed.
