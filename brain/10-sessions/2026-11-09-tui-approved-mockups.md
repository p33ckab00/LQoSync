---
type: session
created: 2026-11-09
tags: [session, tui, design, mockups]
status: design-review
related: ["[[2026-11-09-worker-utilization]]", "[[2026-11-09-buffer-pressure]]", "[[HANDOFF]]"]
---

# Approved TUI mockups

The owner approved the Dashboard, Subscribers, Routers, LibreQoS, and Workers concept screenshots.
The owner selected `LibreQoS DPDK Traffic` for the Dashboard chart title and source.
The latest request approves both Settings views and requires documentation to follow the reviewed screenshots.

## Decisions and artifacts

The approved screenshots define the visual baseline for future Ratatui implementation.
Preserve the shell, panel order, proportions, palette, selection, and text hierarchy.
Use the revised Dashboard image for the owner-selected chart title.
Preserve the approved Subscribers image without an edit.
The approved Routers image is an unchanged copy of the reviewed proposal.
The approved LibreQoS image is an unchanged copy of its reviewed proposal.
The approved Workers image is an unchanged copy of its reviewed proposal.
The owner approved both final Settings images. Their approved copies match the reviewed proposals byte for byte.

The visual contract and review are in `docs/design/tui/README.md`.
The PNG references are in `docs/design/tui/mockups/`.
The original Dashboard remains historical evidence. The revised image supersedes its chart title.
These files contain synthetic records and values. They do not show a running implementation.

## Review and limits

Visual review checked the updated Dashboard against its approved reference.
The panel composition remains the same. The graph title now reads `LibreQoS DPDK Traffic`.
Image generation does not guarantee pixel identity outside the edited text.
Terminal font metrics also affect raster pixels. Compare real terminal captures before accepting implementation.

The documents distinguish design approval from runtime implementation and data availability.
Traffic rates require valid counter differences and bounded history. Missing evidence must show unknown values or gaps.
Router and subscriber labels require collection evidence and freshness. A shaping row alone does not prove online state.
The graph predates these mockups. Use this note and the visual contract for current design decisions.

The asset checks cover all stored PNG files at 2172 by 724 pixels. The hash manifest records each asset.
The stored original Dashboard and Subscribers files match their approved source files byte for byte.
Local contract links and `git diff --check` pass. `docs/design/tui/SHA256SUMS` records the asset hashes.

No application code, configuration, shaping input, or service changed.
No application test or deployment ran for this documentation task.
## Routers approval and LibreQoS proposal

The owner approved the exact Routers screenshot. Visual review checked its shell, cards, filters, table, details, and footer.
The approved copy matches the proposal byte for byte. The visual contract records the matching labels and panel order.
The new LibreQoS concept retains the approved shell and adds sync/import state, compatibility notice, remote status, and source preview.
Its sample native notice distinguishes a blocked optional native source from an imported active pair source.
Remote sync shows `NOT CONFIGURED`. The source preview is read-only, and Sync now requires confirmation.
The image and its exact generation prompt are stored under `docs/design/tui/`.
## LibreQoS approval and Workers proposal

The owner approved the exact LibreQoS screenshot. Its approved copy matches the reviewed proposal byte for byte.
Visual review checked the actions, native notice, sync cards, file tabs, preview columns, selection, page controls, and footer.
The visual contract records these requirements without a redesign.
The Workers proposal uses the same shell, summary cards, worker table, selected details, pressure panel, and snapshot status.
Worker counts and measurements are illustrative. Busy excludes empty polling and differs from operating system CPU usage.
The screen is read-only. It does not change worker, queue, or CPU configuration.
The image and exact prompt are stored under `docs/design/tui/`.
## Workers approval and functional Settings proposals

The owner approved the exact Workers screenshot. Its approved copy matches the reviewed proposal byte for byte.
Visual review checked the cards, filters, worker table, selection, details, pressure, snapshot status, and footer.
The visual contract records those requirements without a redesign.

The Settings proposals show Add Router and Edit Values / Review Changes.
The owner requires real functionality, especially router creation and typed configuration edits.
Source review found existing add-router drafts, pending secrets, metadata, validation, preview, backup, apply, and reload verification.
The new layout must reuse those paths. A screenshot does not demonstrate functional acceptance.
The contract now records the workflow, action gates, and required end-to-end verification.

The Add Router image keeps Preview and Save & apply disabled until the draft and current preview exist.
The values image shows two unsaved runtime edits with a current preview and an enabled confirmation action.
Neither image claims a completed live apply. The final Add Router candidate fixes a clipped top-bar theme label.
The original candidate remains historical generation evidence, not the current proposed baseline.

## Settings approval and screenshot review

The owner approved Add Router and Edit Values / Review Changes.
Both canonical approved copies match the reviewed final screenshots byte for byte.
The initial Add Router candidate remains historical evidence only. No image regeneration or edit occurred during approval.

Review compared the group list, router list, form fields, TLS/source toggles, masked password, draft actions, and apply gates.
It also compared loaded/draft/unit columns, changed-row colors, focus, field help, diff, preview status, and confirmation controls.
The visual contract records the exact panel order, labels, status strips, and footers for both views.
All supported configuration keys remain accessible beyond the sample runtime keys in the image.

Checks: fourteen valid PNGs at 2172 by 724 pixels, twelve unchanged prior hashes, matching approved copies, and valid local design links.
Whitespace checks pass. No application tests apply to this documentation-only task.
Next: future implementation must follow these screenshots and prove the documented functional flows through the real TUI.

## Mouse interaction requirement

The owner requires all actionable controls to support mouse clicks across every screen.
Scrollable panels must support the mouse wheel. Visible scrollbars must support track clicks and thumb drags.
This includes Settings, Add Router, typed values, choices, toggles, confirmation dialogs, and action hints.
The visual contract records per-screen behavior, panel routing, horizontal overflow, focus, and drag lifecycle.
Mouse input must preserve the same validation, read-only, preview, confirmation, and service-result rules as keyboard input.

Current code already enables mouse capture and handles some events. Full redesigned interaction remains unimplemented.
Future acceptance requires event-level tests and real-terminal checks over SSH, including independent panels and resize.
Approved screenshot files remain unchanged. This request adds behavior requirements without a new visual baseline.
