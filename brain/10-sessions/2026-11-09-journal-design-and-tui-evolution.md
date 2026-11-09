---
type: session
created: 2026-11-09
tags: [session, tui, journal, design]
status: proposal-review
related: ["[[2026-11-09-tui-shell-implementation]]", "[[2026-11-09-tui-approved-mockups]]", "[[HANDOFF]]"]
---

# Journal proposal and TUI transition review

Recorded: 2026-11-09T13:46:29+08:00 (Asia/Shanghai).
The owner requested updated visual documentation, screenshot comparison, a Journal design, and a transition assessment.
Branch: `lqosync3`. Worktree: `/home/pi/LQoSync3.0`.

## Result

Inspected all seven canonical views across the six approved screens.
Created `docs/design/tui/mockups/journal-proposal-v1.png` with built-in imagegen.
References: approved Workers and Dashboard images. The exact prompt is in `JOURNAL_PROMPT.md` beside the visual contract.
The Journal proposal has three cards, filters, a log table, entry details, selection controls, and stream status.
The proposal matches the shared visual system. Owner approval remains pending.
Updated the visual contract and added `REVIEW_AND_EVOLUTION.md` with per-screen gaps and acceptance steps.
Corrected old claims that no runtime redesign exists. The shell is implemented, but approved bodies remain pending.

## Feasibility

The current Ratatui layout, event loop, background loader, and editor support the requested terminal evolution.
Preserve the approved screenshots. Reuse Settings gates and current Journal workflows.
Add source freshness, DPDK history, focus traversal, forms, and complete mouse/scrollbar controls.
Journal details need verified field parsing. Stream labels need lifecycle and receive-time evidence.
Browser delivery would be a separate task with its own API, authentication, and deployment plan.

## Checks and next action

All fifteen PNG hashes and dimensions pass. All fourteen prior image hashes remain unchanged.
Local design links and whitespace pass. The STE heuristic flags technical terms and grammar candidates; a manual prose review followed.
Application tests were not rerun for this image-and-documentation task.
The existing 516-test result belongs to the prior shared-shell slice.
Next: owner review of the Journal proposal; proceed with approved screen bodies under the documented gates.
No application code, production configuration, service, commit, or push changed in this task.
Preserve all earlier uncommitted work.
