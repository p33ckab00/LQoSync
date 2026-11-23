# Topology Studio v2 design evidence

Date: 2026-11-23. Source baseline: `2b54a9a` on `lqosync3`.

These screenshots show the corrected local design viewer with synthetic records. They do not show the production application.
All toolbar and action controls use icons inside the canvas viewport. The external viewer header and controls are removed.
The viewer makes no HTTP requests or backend mutations.

## Browser checks

- Three scenes and five source/access panels opened correctly.
- Light and Dark overrides worked. System followed browser changes while the page stayed open.
- The information dialog opened the workflow and returned focus when closed.
- Preview and Load saved topology retained the source selection.
- Standalone SVG followed the System preference.
- No page errors, remote HTTP requests, artboard text clipping, or horizontal page overflow occurred.
- Visible controls and dialogs stayed inside the canvas at 1600, 1440, 768, and 390 CSS pixels.
- Pan, zoom, and fit worked. The accepted topology card geometry and text stayed unchanged.
- The narrow screenshot checks the reference viewer only. It does not prove a responsive application configuration form.

The SVG values are design illustrations. The viewer does not edit policies, test routers, or publish live outputs.
The application backend and its acceptance tests did not change or run for this design task.
The heuristic prose checker reported style findings. No certified language compliance is claimed.

## Files

- `canvas-dark.png` and `canvas-light.png`: the saved-count Sankey with both palettes.
- `router-ppp-dark.png`: active/account/profile query and rate precedence.
- `router-dhcp-light.png`: lease/server grouping and local plan metadata.
- `preview-light.png`: candidate overlay, table completeness, rates, and warnings.
- `viewer-mobile.png`: the local viewer at 390 CSS pixels.
