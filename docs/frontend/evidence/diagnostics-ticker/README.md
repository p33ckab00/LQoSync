# Diagnostics banner evidence

This is the earlier bottom-panel checkpoint. See [current top ticker evidence](../diagnostics-ticker-top/README.md).

Date: 2026-11-20 (Asia/Shanghai). Branch: `lqosync3`. Previous commit: `45dad14`.
See the [design and atomic plan](../../DIAGNOSTICS_TICKER.md).

These screenshots show the implemented Rust application with the isolated browser fixture.
All accounts, component states, capacities, and traffic values are synthetic.

- [Dark banner](dark.png)
- [Light banner](light.png)
- [Mobile static view with reduced motion](mobile-static.png)

Check image integrity with `sha256sum -c SHA256SUMS` in this directory.

## Checks

- Movement continues during live updates. The animation track and component nodes remain stable.

- Keyboard Pause/Resume and hover/focus pause work. The pause state survives live refresh.

- The duplicate loop has `aria-hidden="true"`. The visible status list has semantic list items.

- Reduced motion disables the animation, hides the duplicate, and wraps the original list.

- Repeated component names show their own states. Long labels wrap without overflow in the static view.

- Static details remain available. A failed stream shows unknown health and stale evidence.

- Light/Dark themes and 768/320px layouts passed. No page overflow or browser script error occurred.

- Existing sign-in, theme, session, Dashboard, stale recovery, and sign-out checks passed.

- Wasm release, Clippy with warnings denied, Rust 1.89 compatibility, formatting, and whitespace checks passed.

Browser scripts: `tools/frontend/ticker-check.cjs` and `tools/frontend/browser-check.cjs`.
The core test suite was not repeated for this UI-only update.
The prepared assets still need installation through the web installer.
