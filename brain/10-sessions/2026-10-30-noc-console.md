---
type: session
created: 2026-10-30
tags: [lqosync-v3, ui, topology, noc]
status: complete
related: ["[[HANDOFF]]", "[[2026-10-29-scoped-routing-minimums]]"]
---

# Compact NOC console

The user supplied an HTML reference and asked to follow its visual design.
Branch: `lqosyncv3`. Checkout: `/opt/LQoSync3.0`. Source baseline: `32eb255`.
Earlier shared-skill and Graphify documentation changes remain in the worktree.

## Changes

The console uses the reference CSS, compact sidebar, grouped navigation, top bar, tables, and responsive layout.
Overview uses actual local state. Routers has a table, search, and a selected-router inspector.
Topology has discovered inventory, the configured hierarchy, and a selected-node inspector.

Current save/delete forms keep their handlers, tokens, and validation gates.
Creation forms remain collapsed until selected. The browser stores display density locally.
The small script comes from the same origin. Inline script permission remains disabled.

Synthetic reference data and preview-only actions do not replace application records or controls.

Changed files: `v3/src/{noc_web.rs,console.js,style.css,web.rs,lib.rs}`, `v3/tests/web.rs`, and `v3/README.md`.
The design and plan are under `docs/superpowers/` with the date `2026-10-30`.
This note, Home, and HANDOFF record the milestone.

## Product requirements recalled

The operator must eventually select inventory, build a hierarchy, and categorize sources with manual or automatic controls.
Inventory includes WAN candidates, PPP profiles/secrets, DHCP servers/leases, and Hotspot clients.
Two WAN roots with PPP branches are examples, not a fixed topology.
Removing a topology item must not delete RouterOS records.


For automatic balancing, a new online session or reconnect must become eligible for assignment by online client count.
Keep an assignment stable during its online session. Manual pins keep priority.
Use the same intended assignment for topology, shaping output, and the core address-list plan.
A failed inventory read must not count as a disconnect.

This policy and the broader topology behavior remain future work. This change updates the visual console only.
The current implementation still retains valid assignments and uses planned load relative to capacity.
Protected manual core entries still require migration review before any future writer.

## Verification

The new shell and inspector tests failed before implementation and passed after implementation.
The inventory test checks escaped names, omitted extra fields, read-health labels, and the absence of publication.
Independent review found invisible keyboard targets in the closed mobile sidebar. A browser check reproduced the defect.

The fix hides the closed sidebar from keyboard navigation. The open menu selects a navigation link.
The menu restores focus after closure when necessary. Follow-up review found no material issue.

Final checks: 130 Rust tests passed, zero failed. Nine manual benchmarks remain ignored.
Format, strict Clippy, and the binary build passed.
Browser checks passed on 12 routes at widths 1440px, 390px, and 320px.
Checks cover overflow, script policy, density persistence, router search, inspectors, form disclosure, and mobile keyboard navigation.
Reference CSS comparison and JavaScript syntax checks passed.
The documentation checker produced heuristic findings. Manual review kept technical terms and corrected paragraph length and word choices.

## Limits and next action

The live console remains on its prior process. No live service restart occurred.
Tests use temporary application data. Private credentials and production inventory were not read.
No router, LibreQoS, package, or shaping change occurred.

The Graphify map predates these source changes and this note. Update it before relying on graph coverage of the new UI.
Next: review the new appearance, then complete the flexible topology and reconnect policy design before protected membership migration.
See [[HANDOFF]].
