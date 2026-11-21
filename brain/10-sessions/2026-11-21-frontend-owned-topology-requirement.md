---
type: session
created: 2026-11-21
tags: [topology, frontend, requirements]
status: verified-not-deployed
related: ["[[2026-11-21-topology-workflow-repair]]"]
---

# Frontend controls the topology from an empty state

The owner clarified that a new topology must start empty.
The frontend must control initial hierarchy creation as well as later changes.
Router and source discovery belongs in Inventory. Discovery must not create or attach hierarchy nodes automatically.
Legacy import must be a separate explicit choice, not the default initialization path.
The backend validates and persists frontend-approved policy, then orchestrates the generated network and subscriber hierarchy.

The frontend now proposes `topology.initialize` through the existing review and confirmation flow.
Confirmation creates a marked draft with configured routers and zero topology nodes or selected sources.
Structural edits work before enrollment only for that explicit draft. Subscriber rules and publication retain their admission gates.
GET, preview, cancellation, and discovery do not initialize or import hierarchy nodes.
Initialization refuses any existing installed policy, saved records, or active/unknown topology writer.

The Builder exposes Add parent for subscriber groups. It creates another instance of the shared source under a different parent.
The original instance keeps its parent. Duplicate source/parent instances are refused.
Each actual export node has one parent. Each subscriber retains one effective placement and one circuit.

Existing saved policies stay visible and have an explicit label. No saved registry was reset.
The reported live registry could not be inspected without privileged access; synthetic WAN names are not proof of its content.
Verification passed: 1,580 core tests, 102 web tests, native/Wasm Clippy, optimized Wasm, and four browser suites.
Evidence: `docs/frontend/evidence/topology-initialization/README.md`. No production service, router, shaping pair, enrollment, or writer ownership changed.
