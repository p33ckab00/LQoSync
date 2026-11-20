# Service diagnostics banner

Status: implemented. Runtime installation is pending.

## Design

Show synchronization and component health as a stock-exchange-style banner.
Keep it below the upstream table, in the Service diagnostics panel.
Use the existing Plex font, tabular numbers, theme colors, and panel borders.
Separate entries with thin rules. Do not invent market prices, trends, or movement arrows.

Show evidence freshness, service health, publication phase, runtime mode, output target, and cycle duration.
Then show each published component and its health state.
Keep the current source semantics. A confirmed import does not prove applied shaping.
Use text and status colors together. Runtime context values do not show a warning.

## Atomic plan and result

1. Reuse the existing overview signal. Add no source query or backend operation.

2. Add a stable Rust banner component. Update values without replacement of its animation track.

3. Use a CSS transform for a continuous right-to-left loop.

4. Add a visible keyboard Pause/Resume control. Pause motion on hover and focus.

5. Use a static wrapped list when the browser requests reduced motion.

6. Keep the full static diagnostics under View diagnostics.

7. Check live changes, stale evidence, repeated names, long labels, themes, and mobile widths.

8. Save browser evidence, build, document, commit, and push.

The pause control stops motion. Source values continue to update.
Each loop has two identical groups. The second group is hidden from assistive technology.
No `aria-live` region repeatedly announces the banner.
Component keys include position and name, so repeated source labels remain separate.

The track uses the compositor-friendly `transform` property.
It adds no JavaScript animation loop, timer, package, or extra network request.
Only `prefers-reduced-motion: no-preference` enables movement.
The reduced-motion view wraps all entries and hides the duplicate group.

## Checks and deployment

Wasm release build, Wasm Clippy with warnings denied, and Rust 1.89 check passed.
The focused browser check covers motion, stable nodes, keyboard controls, hover, reduced motion, and stale states.
It also checks repeated component names, long labels, Light/Dark themes, and 768/320px widths.
The existing sign-in and Dashboard browser smoke check passed.

See [browser pictures and evidence](evidence/diagnostics-ticker/README.md).
The optional design-taste inspector is unavailable. Playwright supplies browser checks.
Core, gateway, and TUI code did not change in this update.

Install the prepared web build from the repository:

```bash
cd /home/pi/LQoSync3.0
sudo tools/frontend/install.sh
```

## Skills

Vercel frontend-design keeps the existing product design and limits motion to the requested banner.
Google modern-web-guidance `motion` says: "Provide a pause mechanism".
The implementation uses a visible keyboard control, transform animation, and a static reduced-motion view.
