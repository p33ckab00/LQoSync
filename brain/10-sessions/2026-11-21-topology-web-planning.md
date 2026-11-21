---
type: session
created: 2026-11-21
tags: [topology, frontend, design, planning]
status: ready-for-review
related: ["[[topology-manager-status]]", "[[2026-11-21-router-toolbar-modal]]"]
---

# Topology Manager web design

Checkpoint: 2026-11-21T12:56:35+08:00 (Asia/Shanghai).
Goal: port the Topology Manager workflow into a documented web design, implementation plan, and screenshot references.
Last agent: Codex `/root`. Branch: `lqosync3`. Worktree: `/home/pi/LQoSync3.0`.
Source baseline: `76563ee`. The topology/API/Runner sources were unchanged at cross-check `e17b3b8`.
The other worker has active web source edits. Preserve them and all unrelated notes.

## Deliverables

- `docs/frontend/TOPOLOGY_DESIGN.md`: 22 features, four views, responsive layouts, and 15 quality gates.

- `docs/frontend/TOPOLOGY_CONNECTIONS.md`: source evidence, current operations, new typed routes, coherent views, receipts, and failure rules.

- `docs/frontend/TOPOLOGY_IMPLEMENTATION_PLAN.md`: 12 ordered tasks with a read-only release cut before mutation delivery.

- `docs/frontend/evidence/topology-manager/`: editable static HTML/CSS, 13 screenshots, capture results, and a review report.

The plan preserves the service's single policy/publication owner and the independent TUI.
It requires coherent bounded reads, durable core receipts, proposal leases, and explicit unknown outcomes before web writes.
Schedules, activation/enrollment actions, router routing writes, and unsupported policy editors remain outside the initial web scope.
The mockups contain synthetic data only. This task does not implement production topology routes.

## Checks

Thirteen static browser captures passed font, title, text-size, contrast-token, and horizontal-overflow checks.
Document links, feature/gate/task counts, current operation names, proposal lifetime, and proposed-API boundaries passed checks.
Manual source and visual review corrected count labels, sample arithmetic, and control contrast.
The STE checker is heuristic. Technical term flags received manual review, without a certified compliance claim.
No runtime acceptance, live publication, production service, or router test occurred.

## Publication handoff

Updated: 2026-11-21T13:01:27+08:00 (Asia/Shanghai).
The owner first requested a wait until Claude Code finished.
The owner later authorized commit/push after the no-collision work-split note passes review.
The shared HANDOFF entry at 2026-11-21T12:50:00+08:00 assigns Journal/Workers to Claude Code and Topology to Codex.
The current HEAD is `e17b3b8`. The other worker's pending source files remain outside this task.
The Topology documents, mockups, and this session note occupy separate new paths.

Publication uses explicit paths, an inspected index, no concurrent Git writer, and a normal fast-forward push.
Shared Home, HANDOFF, frontend indexes, and source files remain untouched to preserve the other worker's ownership.
The static capture, link, coverage, source-operation, and whitespace checks precede the commit.
Next action after publication: review the design, then authorize implementation separately.

## PPPoE and IPoE delivery extension

Updated: 2026-11-21T13:20:36+08:00 (Asia/Shanghai).
The owner requested PPPoE/IPoE views in the same Topology Manager delivery.
Added `docs/frontend/TOPOLOGY_SUBSCRIBER_VIEWS.md` and extended the design, connection map, and T00–T11 plan.
The feature map now has 26 features. T03 includes both source views and shared plan membership.
T08 reuses shared actions and cross-source impact review.

Added four HTML mockups and 12 screenshots. All 25 static captures passed.
Both protocols show the same verified subscriber ID and local plan.
Unlinked lease/account observations stay read-only. Old endpoint evidence stays historical.
Vercel frontend-design and Google responsive-table/tabs guidance informed the layouts.
STE review remains heuristic. Production implementation and runtime acceptance remain future tasks.
Shared source files, Home, and HANDOFF remain under the existing work split.
Next action: use the combined plan and screenshots for implementation.

## Subscriber documentation publication

Updated: 2026-11-21T14:08:55+08:00 (Asia/Shanghai).
The owner authorized commit and push. Parent checkpoint: `89ff93f`.
The Journal/Workers implementation is committed. The current work split permits isolated Topology documentation publication.
Checks: 25 static captures, local Markdown/HTML links, 26 unique feature IDs, and whitespace passed.
This documentation change needs no production build or runtime deployment. Runtime release checks remain in T11.
Publish only the Topology plans, subscriber specification, static evidence, and this session note.
Preserve unrelated untracked notes and assets. Next action: implement the combined delivery plan.
