# Router reference layout

Status: implemented. The browser checks passed. Production installation remains pending. Reference: `../images/02-routers.png`.

## Audit and intended result

Preserve the current application shell, routes, filters, and router actions.
Use the reference's table-and-details composition inside the existing Rust frontend.

| Element | Reference | Current gap | Change |
|---|---|---|---|
| Inventory | Table on the left | Inventory fills the page | Add a selected-router details column |
| Selection | Blue row fill | No local selection | Add an Inspect action and selected-row state |
| Details | Name, state, endpoint, credential, collection, actions | A separate route owns details | Add a cached summary beside the table; retain full detail routes |
| Type | Strong title and readable facts | Existing local Plex and semantic tokens | Preserve the type family and use a clear panel title |
| Spacing | Approximately 24px gutters and 450px detail panel at 1536px | No split workspace | Use 24px gap and a bounded 360–420px detail column |
| Narrow layout | Desktop reference only | Some columns disappear | Stack details and preserve all table columns in a local scroll region |
| Add and Sync | Text actions in the sample | Owner has requested icons and a modal | Preserve the approved icon controls and registration modal |

The production contract has no collection duration, interval, or detailed collection message.
Show only available evidence. Never replace missing fields with sample values.
Credential status means configured or missing. No password value appears.

## Plan

1. Add local selection to the cached inventory. Use stable router IDs.
2. Add a keyboard-accessible Inspect action and visible selected-row state.
3. Render a named details region with endpoint, roles, collection, and safe credential status.
4. Link Connection, Sources, Activity, and the full router page to existing routes.
5. Close with Escape or the close control. Restore focus to the selected Inspect action.
6. Keep the table and detail panel within the available width. Stack them on smaller screens.
7. Verify selection, focus, filters, themes, narrow widths, and zero router I/O during inspection.
8. Run the existing router browser checks, UI tests, Wasm build, and Clippy.

## Guidance

Apply Vercel frontend-design to the industrial console composition.
Apply Google Modern Web Guidance `responsive-table` to table semantics and local scroll.
Use the existing framework, font, icons, and semantic colors.

## Acceptance

The selected router appears beside the inventory on desktop.
The selected row remains visible. The close control restores keyboard focus.
Inspection reads the current inventory only and sends no mutation request.
Full detail links, Add modal, Sync, and filters retain their behavior.
At 320px, the page has no horizontal overflow. The table can scroll locally.
Screenshot evidence must use the isolated synthetic browser fixture.

Evidence: [screenshots and completed checks](../evidence/router-reference/README.md).
