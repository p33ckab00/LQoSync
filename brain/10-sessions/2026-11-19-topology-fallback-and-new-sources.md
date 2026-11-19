---
type: session
created: 2026-11-19
tags: [topology, fallback, sources, tui, api]
status: done
related: ["[[topology-manager-status]]", "[[2026-11-19-settings-managed-fields]]"]
---

# Topology fallback group and new sources (Phase 2c-2)

A new router plan after enrollment held publication (Decision 10). The owner wants shaping to continue through a fallback group, and the operator adds the new source from the Builder.

## Results

- Plan: [2026-11-19-topology-fallback-and-new-sources](../../docs/superpowers/plans/2026-11-19-topology-fallback-and-new-sources.md). Six tasks on branch `topology-fallback`.
- Schema: optional `catalog.fallback`. Source policy: unbound PPP profiles with a fallback. Resolver: fallback placement and re-placement of ineligible unpinned assignments.
- API: `topology.sources.candidates`, edits `fallback.set`, `fallback.clear`, `source.add`, `node.create` with `node_kind: "group"`.
- TUI: `F`, `i`, the strip count, and the Assignments fallback text.
- No live service, router, or LibreQoS file changed.

Related: [[topology-manager-status]].
