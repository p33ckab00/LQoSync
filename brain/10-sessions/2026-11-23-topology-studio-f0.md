---
type: session
created: 2026-11-23
tags: [topology, frontend, canvas, f0]
status: complete
related: ["[[2026-11-23-topology-studio-tandem-plan]]", "[[ADR-0014-canvas-auto-apply-and-writer-retirement]]"]
---

# Topology Studio F0: canvas shell

Timestamp: 2026-11-23T23:08:19+08:00 (Asia/Manila).

## Scope and result

F0 is complete from G0 base `a0e6035`. Contract: `f7e3977`.
Branch: `codex/topology-studio-ui`.
Worktree: `/home/pi/LQoSync3.0-worktrees/codex-topology-studio-ui`.
Code commits: `4b5b424`, `2918086`, `1faa0c0`.

The real Leptos UI has fixed icon docks, optional information help, and bounded dialogs.
Appearance has a separate dock and shares the application preference.
The short landscape view has a scrollable tool dock and usable world space.
Help inside an editor keeps the WAN draft and returns focus.
A failed first read ends the pending state.
A keyboard layout save keeps its feedback visible in landscape.

The changes preserve card IDs, saved positions, flow data, counts, and current service requests.
The source-policy editor remains F1–F3 work. Backend work stays with Claude Code.
No production router, account, output, service, or writer owner changed.
No merge, push, or deploy occurred.

## Verification

Baseline UI tests: 72 passed, 0 failed.
The first shell check failed because the information button did not exist.
Later checks exposed tablet layout, overlay conflicts, and cache notices.
The independent review found five important issues. The edge check reproduced each issue before the fix.
All focused checks and the final full browser gate passed after the corrections.

Workspace tests: 2023 passed, 0 failed, 13 ignored.
Workspace and Wasm Clippy passed with `-D warnings`.
Format and whitespace checks passed.
The browser gate passed 20 checks. It skipped `pulse-check`, the established exclusion.
The fixture and Wasm builds passed.

Browser logs: `/tmp/lqosync-checks.4eogJE/`.
The optional design inspector is unavailable. Playwright supplied the rendered checks and screenshots.

## Rulings and open items

- Share one theme signal and one cookie update path through `app.rs`.
- Preserve the current select-then-Enter card behavior.
- Extract the F0 brief manually. The task-start script cannot parse the alphanumeric headings in the plan.
- Defer the minor keyboard-tooltip issue on Reload and the editor header Close. Both controls have accessible names.
- Keep stale responses, route failures, save conflicts, durable evaluation, and pagination in their assigned later tasks.

Next action: start F1 from the shared B0 contract and fixture. Integration and I0 remain separate.
Keep the branch and worktree for the next frontend slice.

## Links

- [[2026-11-23-topology-studio-tandem-plan]]
- [[ADR-0014-canvas-auto-apply-and-writer-retirement]]
- [Frontend handoff](../../docs/handoffs/2026-11-23-codex-topology-frontend.md)
- [Screenshots and checksums](../../docs/frontend/evidence/topology-studio-f0/README.md)
