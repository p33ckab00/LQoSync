---
type: session
created: 2026-11-01
tags: [session, tui, design, handoff]
status: done
related: ["[[HANDOFF]]", "[[ADR-0008-terminal-ui]]"]
---

# TUI cosmetics design handoff

Timestamp: 2026-11-01T11:54:22+08:00 (Asia/Shanghai).

## Goal

Continue the design spec after Claude Code reached its usage limit.
The operator selected all cosmetics items A1–A5 and B6–B10, without an ASCII mockup.

## Evidence before takeover

- Branch: `lqosync3`, commit `cf2c959`, worktree `/home/pi/LQoSync3.0`.
- The working tree had no changes. No cosmetics spec existed.
- A Claude Code process remained. Its visible children were MCP servers, without a build or edit subprocess.
- The shared handoff listed cosmetics after Plan 4c and the minor fixes.
- The graph query located the TUI entry points. Source files confirmed the current behavior.

## Result

Created `docs/superpowers/specs/2026-11-01-lqosync3-tui-cosmetics-design.md`.
The spec covers themes, shell, all tabs, narrow layouts, and acceptance checks.
It includes file ownership by component and an implementation order.

Network bars show configured download capacity against the maximum in the loaded tree.
The RSS gauge shows the current value against the peak in the stored history.
These labels avoid a false claim of live traffic or a memory limit.
The new `tui.theme` choice follows the current apply flow and takes effect at TUI restart.
These details are proposals in the design spec. They do not change the accepted architecture decision.

## Checks

- `/home/pi/.cargo/bin/cargo test --lib tui:: --quiet`: 160 passed, 0 failed, 1 ignored.
- The shell did not find `cargo` through PATH. The installed absolute path worked.
- The STE heuristic checker ran. Technical terms and noun sequences still produce advisory findings.
- Manual review covered all ten request items and the 80×24 layout budget.
- Source code, production files, service state, and dependency versions did not change.

## Next action

Read the spec and plan, then implement in component order when authorized.
Run the full gates and real-terminal checks stated in the spec during implementation.
The existing graph does not include this new spec or session note.

## Implementation plan and review

Timestamp: 2026-11-01T12:03:25+08:00 (Asia/Shanghai).
The operator requested a plan, a review against the spec, then commit and push if the review passes.

Created `docs/superpowers/plans/2026-11-01-lqosync3-tui-cosmetics.md`.
The plan has eight tasks with concrete interfaces, tests, focused gates, and commit boundaries.
Its coverage table maps all ten acceptance items and all numbered spec sections.

Inline review result: PASS for plan coverage.
The review made the Network bar helper contract explicit for zero and non-finite values.
It also defined the global-count fallback when a removed router has no surviving group.
All five review focus cases have tests in their owning tasks.

Fresh baseline: `/home/pi/.cargo/bin/cargo test --lib tui:: --quiet` gave 160 passed, 0 failed, and 1 ignored.
The remote fetch succeeded. Local and remote branches matched before the documentation commit.
Only documentation changes enter this commit. Implementation and visual acceptance remain open.
The graph still describes the earlier code and document baseline.
