# Topology Sankey canvas — evidence

Date: 2026-11-24. Branch: `claude/topology-sankey`. Spec:
`docs/superpowers/specs/2026-11-24-topology-sankey-canvas-design.md`.

All screenshots come from the synthetic loopback fixture (`browser_fixture`, port 19581). No
production account, router, or shaping write was used.

## Screenshots

| File | Width | Theme | State |
|---|---|---|---|
| `sankey-1440-dark.png` | 1440 px | dark | live sample |
| `sankey-1440-light.png` | 1440 px | light | live sample |
| `sankey-1440-dark-popover.png` | 1440 px | dark | live sample, WAN North popover open |
| `sankey-1440-dark-stale.png` | 1440 px | dark | stale sample: subscriber counts |
| `sankey-768-light.png` | 768 px | light | live sample |
| `sankey-390-dark.png` | 390 px | dark | live sample |

`SHA256SUMS` holds the checksum of each file.

## Gate results

Command: `tools/frontend/check-all.sh` with the fixture environment.

- Rust stage: fmt, whitespace, Clippy (native and Wasm), and tests: PASS.
- Browser stage: every check PASS. `pulse-check` is skipped (known failure from before the web
  work).
- In one full run, five checks failed with `ERR_CONNECTION_REFUSED` because the fixture stopped.
  A second run of these five checks passed (`/tmp/lqosync-checks.DPwlGk`).

New and changed checks:

- `topology-canvas-check`: ribbons, WAN streams, columns, no card overlap, popover, stream
  details, Escape, Fit view, minimap, toolbar chips, Partial and stale states, legend.
- `shell-rail-check` (new): rail default, link names, focus tooltip, saved choice, phone width.
- `studio-shell-check`, `studio-shell-edge-check`, `topology-initialization-check`,
  `topology-check`: changed for the removed drag and the Sankey tab name.

## Comparison with the owner reference

Reference: `docs/frontend/prototypes/topology-studio-v2/04-owner-sankey-reference.png`.

| Item | Reference | This build |
|---|---|---|
| Columns | Left to right | Left to right. The graph sets the columns (longest path). |
| Ribbons | Filled, width by value | Filled, width by live download (exact sum of the hosts). |
| Color | Cyan to green gradient, amber bypass | One solid color for each WAN stream. The page policy blocks SVG gradient references. No backup flag exists, so no amber. |
| Cards | Dark card, colored accent bar on the left | Dark card. WAN cards have the color of their stream on the left. |
| Card numbers | Gbps, CPU, drops, queues | Mbps, capacity, subscribers. No CPU, drops, or queues: no data source. |
| Hover | "Source → Target" panel | Click popover: stream per WAN, Mbps down and up. |

## Guides used

- Vercel frontend-design (`~/.agents/skills/frontend-design/SKILL.md`): one bold element (the
  ribbons), quiet cards, no decorative KPI tiles.
- Google Modern Web Guidance: non-modal dialog for the popover (focus stays on the card, Escape
  closes it), visible focus, reduced motion, no horizontal scroll at 390 px.

## Known limits

- The host sample has at most 512 rows. A larger network shows the "Partial" chip.
- While the routing writer is off, the routing WAN of a host can differ from the path that the
  routers use. The canvas uses the WAN node in the tree path first.
- The popover is wider than the gap between two columns. It can cover the name of the next card
  until it closes.
- A display-only card cannot be the parent of a WAN node: the tree rule "a WAN node stays a root"
  refuses it. The owner decides if this rule changes.
