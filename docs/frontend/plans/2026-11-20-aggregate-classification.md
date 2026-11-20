# Consolidate aggregate traffic

Status: implemented. The owner authorized this scope on 2026-11-20.
See [verification evidence](../evidence/aggregate-traffic/README.md).

## Scope

Add Actual, Mapped, and Unmapped to the current aggregate traffic panel.
Preserve the current Dashboard layout, diagnostic ticker, and DPDK health card.
Use Rust Leptos and SVG. Add no chart dependency.

## Design

Keep the current type, glass surfaces, and theme tokens.
Use a rainbow area for Actual, amber dashes for Mapped, and violet dots for Unmapped.
Mirror upload below zero. Use independent direction scales and capacity anchors.
Use one local timestamp inspector with pointer, keyboard, and touch access.
Keep unknown, zero, stale, and gaps distinct.

## Steps

1. Add typed classification series with source state to the web contract.
2. Project the published histories and bound the complete overview payload.
3. Add focused tests for gaps, unavailable sources, and response size.
4. Replace the aggregate chart with mirrored geometry and source controls.
5. Add a shared timestamp inspector and an accessible sample table.
6. Check native logic, Wasm compilation, browser behavior, and existing Dashboard flows.
7. Save evidence, update deployment notes and brain memory, then commit and push.

## Acceptance

All three sources share one time axis without stacking.
Red stays at each configured direction capacity.
Above-capacity samples remain visible.
Source updates preserve selected timestamps and source toggles.
Hover does not issue network requests.
The full broker reply stays below 64 KiB.
Unsupported longer windows remain unavailable.
Subscriber queue screens remain outside this change.
