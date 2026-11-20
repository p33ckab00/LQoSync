# Rolling diagnostics strip

Historical checkpoint. See the [continuous ticker](../diagnostics-continuous/README.md).

Date: 2026-11-20 (Asia/Shanghai). Branch: `lqosync3`. Previous commit: `8bfdd26`.
See [design and scope](../../DIAGNOSTICS_TICKER.md).

The strip appears above the Dashboard heading.
It has no card, heading, subtitle, or pause control.
The five permanent fields and temporary activity entries remain.
Movement continues from right to left, including during hover.
Reduced motion shows one static list that wraps within the page.
Full static diagnostics remain below the main panels.

## Checks

- Release Wasm build, Clippy with warnings denied, and Rust 1.89 checks passed.
- Browser checks passed for compact height, absent controls, continuous motion, and stable nodes during updates.
- Five permanent fields, conditional activity, duplicate suppression, expiry, and stale status checks passed.
- Light/Dark, reduced motion, long labels, hidden duplicate, and 768/320px checks passed.
- Existing sign-in, session, theme, failure/recovery, and sign-out checks passed.
- Formatting and whitespace checks passed.

The existing native activity logic did not change.
Core, broker, gateway, and TUI code did not change.
The core test suite was not repeated.

## Pictures

These screenshots show synthetic browser fixture data.
They contain no production credentials or subscriber records.

- [Dark](dark.png)
- [Light](light.png)
- [Mobile with reduced motion](mobile-static.png)

Run `sha256sum -c SHA256SUMS` in this directory to check the images.
Install the prepared assets with `sudo tools/frontend/install.sh`.
Production assets and services were not changed.
