---
type: session
created: 2026-11-23
tags: [topology, frontend, design, review]
status: documented-proposal
related: ["[[ADR-0014-canvas-auto-apply-and-writer-retirement]]", "[[2026-11-22-topology-canvas-integration-map]]"]
---

# Topology Studio package review

## Goal

Extract the supplied design package. Assess the proposals against current LQoSync source and improve the implementation guide.

## Changes

- Extracted the DOCX and five SVGs from `docs/frontend/images/LQoSync_Complete_Design_and_SVGs.zip`.
- Added faithful source text and checksums under `docs/frontend/references/topology-studio/`.
- Added `docs/frontend/TOPOLOGY_STUDIO_REVIEW.md` and a link in the frontend index.

## Findings

- Keep optional preview and automatic valid edits under [[ADR-0014-canvas-auto-apply-and-writer-retirement]].
- The supplied discovery figure has an approval step that conflicts with the accepted design.
- Separate canvas view restoration from a policy revision restore.
- `src/topology_manager/wan.rs` discards route-read errors with `unwrap_or_default()`.
- The current disabled-router contract conflicts with a connection test before enable.
- Reuse current inventory work, identities, policy modules, routing balance, and the authoritative publisher.

## Evidence and limits

The source checkpoint was `c89d5d7` on `lqosync3`, plus current uncommitted topology changes.
The project graph supplied leads. Direct source reads confirmed the listed findings.
The review inspected all five embedded figures and the complete document text.

The new guide contains proposed phases and acceptance scenarios. It does not authorize a new product decision.
Application code and runtime state did not change for this review.
Archive checks, asset checksums, SVG XML checks, local links, and whitespace checks passed.
Hashes confirmed that all 12 pre-existing application files kept their bytes.
The heuristic prose checker reported style findings. No application tests ran for this documentation task.

## Next action

Reconcile the inventory work with its current owner. Implement optional contextual help as the first bounded slice.
Review route-read status before new WAN automation. Keep policy and telemetry extensions in separate slices.

## Links

- [[2026-11-22-topology-canvas-integration-map]]
- [[ADR-0014-canvas-auto-apply-and-writer-retirement]]
- [Review and implementation guide](../../docs/frontend/TOPOLOGY_STUDIO_REVIEW.md)

## Owner clarification: design and themes

The owner requires the supplied Topology Studio design, with dynamic application themes.
Keep its composition, cards, flow bands, floating tools, and configuration panel.
Use the current Light, Dark, and System control with semantic CSS tokens.
Theme changes must preserve drafts, selection, viewport, and open panels. They must not change topology policy.
This clarification updates the implementation guide. It does not change application code or the original SVG.

## Owner clarification: workflow in the information panel

Show the configuration workflow only after the operator activates the information icon.
Place the steps in the help panel. Keep the main canvas free of the permanent workflow strip from the mockup.
Closing help preserves the canvas state. The help panel follows the application theme.
The implementation guide records this requirement. Application code and original source assets remain unchanged.

## Detailed source-policy design and SVG reference

The owner supplied the full router-source workflow and requested an SVG reference.
Local and remote `lqosync3` matched `2b54a9a` during this follow-up.
Added `docs/frontend/ROUTER_SOURCE_CONFIGURATION.md` and three editable SVG scenes under `docs/frontend/prototypes/topology-studio-v2/`.
The local viewer switches source panels, canvas/preview modes, and Light/Dark/System themes.
Information opens the workflow. No permanent workflow strip appears on the canvas.
Load saved topology displays current saved configuration and preserves the unsaved draft separately. It does not publish or restore historical policy.
The requested test before save needs a new explicit draft connection test, separate from registered disabled-router behavior.
PPP grouping uses active/account/profile joins. Dynamic DHCP metadata stays in LQoSync.
Preview uses frozen bounded evidence and must not mutate registry, grace, routing state, or live outputs.
The examples use synthetic data. These artifacts do not add application features or router operations.

Browser checks passed for the three scenes, five policy/access panels, themes, help focus, and preview/load selection retention.
The viewer made no remote HTTP requests and had no page errors or artboard text clipping.
Three SVG files passed XML parsing. Link and whitespace checks passed.
Screenshots and checksums are in `docs/frontend/evidence/topology-studio-v2/`.
The narrow screenshot checks the reference viewer only. Application form responsiveness still requires implementation checks.

## Owner correction: all controls inside the infinite canvas

The owner accepts the topology cards. All toolbars and action buttons must use icons and stay inside the infinite canvas.
Corrected the v2 SVGs and local viewer. Removed the external header, action tabs, source dropdown, and footer controls.
Added floating icon docks for configuration, add router, preview, load, information, zoom/fit, theme, and SVG export.
Configuration and help panels stay within the canvas bounds. The tool docks stay fixed while the topology pans or zooms.
Kept the topology card geometry, text, and Sankey placement. Added accessible names and icon tooltips.
Browser checks passed at 1600, 1440, 768, and 390 CSS pixels for visible control and dialog containment.
Theme, source-panel, help focus, pan/zoom/fit, and preview/load checks passed with no page errors or HTTP requests.
The corrected artifacts remain a design viewer. Application code and live state did not change.
