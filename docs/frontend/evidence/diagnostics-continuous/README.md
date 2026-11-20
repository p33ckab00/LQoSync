# Continuous ticker evidence

Date: 2026-11-20 (Asia/Shanghai). Branch: `lqosync3`. Previous commit: `02eaebb`.
See [design and queue rules](../../DIAGNOSTICS_TICKER.md).

## Regression

The previous implementation jumped 83 pixels when an activity entry changed the track width and duration.
The same insertion test now passes with a movement allowance of 12 pixels over two browser frames.
The test seeks the animation to 40 percent before insertion.

A second test checks the position of the next item through a handoff.
Fourteen further handoffs check that the track covers the viewport without a blank tail.
New entries append to the tail without changing the current pixel distance.
Visible text finishes its pass before it can change or expire.

## Checks

- Release Wasm, Clippy with warnings denied, and Rust 1.89 checks passed.
- The focused browser check passed for insertion continuity, handoffs, coverage, and stable nodes.
- Permanent fields, activity suppression, expiry, stale evidence, and static diagnostics checks passed.
- Light/Dark, reduced motion, long labels, hidden copies, and 768/320px checks passed.
- Existing sign-in, session, theme, failure/recovery, and sign-out smoke checks passed.
- Formatting and whitespace checks passed.

The queue holds at most 18 moving items.
CSS performs the motion. Rust measures once per handoff.
The change adds no network request, backend operation, or dependency.
The existing native activity logic, core, and TUI did not change.

## Pictures

These screenshots contain synthetic fixture data.

- [Dark](dark.png)
- [Light](light.png)
- [Mobile with reduced motion](mobile-static.png)

Run `sha256sum -c SHA256SUMS` in this directory to check the images.
The screenshots show appearance. The browser regression checks confirm continuity.

Install the prepared assets with `sudo tools/frontend/install.sh`.
Production assets and services were not changed.
