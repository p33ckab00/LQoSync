---
type: session
created: 2026-11-09
tags: [session, tui, implementation]
status: first-slice-complete
related: ["[[2026-11-09-tui-approved-mockups]]", "[[HANDOFF]]"]
---

# TUI shared-shell implementation

Recorded: 2026-11-09T13:29:01+08:00 (Asia/Shanghai).
The owner authorized starting the approved TUI redesign, then requested continuation.
Branch: `lqosync3`. Worktree: `/home/pi/LQoSync3.0`.

## Result

- Add `src/tui/shell.rs`: wide sidebar, seven primary routes, top bar, cached status, and local theme control.
- Add `src/tui/operations.rs`: temporary cached Routers and LibreQoS status views with bounded scrolling.
- Update `src/tui/app.rs` and `src/tui/mod.rs`: Dashboard default, shared key/click navigation, input and modal guards, resize target invalidation.
- Keep existing Settings action gates and parent-node navigation. Escape returns from Network to Subscribers.
- Keep the compact header/body layout below 140 columns or 28 rows. Minimum size remains 80 by 24.
- Update Operations, the visual contract, and the first-slice plan.
- Preserve all earlier uncommitted mockups and documentation.

## Evidence

The default full suite passes: 516 tests, zero failures, six ignored manual tests.
Clippy passes for all targets with warnings denied. The debug build passes.
Formatting and whitespace checks pass. All fourteen approved or historical image hashes remain unchanged.
The existing frame exporter passes and writes 63 synthetic-data JSON frames across seven routes, three themes, and three sizes.
An isolated pseudo-terminal check exits with code zero after navigation, theme switching, resize, and quit.
Mouse capture and the alternate screen are restored. No panic occurs.
Evidence directory: `/home/pi/.local/state/lqosync3/tui-shell-review-20261109`.
Test and lint logs: `/tmp/lqosync3-shell-full.log` and `/tmp/lqosync3-shell-clippy.log`.

## Remaining work

Implement each approved screen body. Temporary status bodies do not establish screenshot parity.
Add DPDK counter provenance, valid deltas, bounded history, and restart/gap handling.
Use collection evidence for subscriber online state.
Complete mouse forms, panel wheel routing, and scrollbar track/drag interaction.
Resolve Journal design and review compact layouts. Compare actual terminal captures with the approved screenshots.
Keep Settings draft validation, preview, backup, conflict detection, explicit apply, and result verification.
No commit, push, installation, production configuration write, or service restart occurred in this slice.
