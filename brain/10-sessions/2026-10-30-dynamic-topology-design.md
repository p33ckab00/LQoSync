---
type: session
created: 2026-10-30
tags: [lqosync-v3, topology, design]
status: implemented
related: ["[[HANDOFF]]", "[[2026-10-30-flexible-topology]]"]
---

# Dynamic topology design

Updated: 2026-10-30T08:24:53+08:00 (Asia/Shanghai).
Branch: `lqosyncv3`. Checkout: `/opt/LQoSync3.0`. Source baseline: `ac4c922`.
The user clarified that service nodes must be optional. Operators must control node selection and hierarchy.
The user explicitly confirmed independent top-level roots with preserved WAN/core associations.

Read `/opt/libreqos/src/updatecsv.py` as the reference, without execution or modification.
Its topology helpers include flat, grouped, and promoted arrangements.
Current v3 restrictions couple exported ancestry to routing WAN identity and reserve destinations by source.
The proposed design separates source identity, exported parent, and routing WAN.
It covers shared destinations, flat roots, custom branches, explicit replacements on removal, and preserved reconnect behavior.

The user added modal configuration forms and cog controls throughout the console.
The design now specifies native dialogs, correct layer order, contained focus, background scroll control, and mobile access.
It also covers failed saves, preserved form values, unsaved edits, and checks for overlays and keyboard access.
The earlier independent review predates this modal addition. The addition received a local review.

Draft: `docs/superpowers/specs/2026-10-30-dynamic-topology-design.md`.
An independent builder review reported no important unanswered question.
Diff checks passed. The prose checker produced heuristic flags for technical terms and noun clusters. Manual review followed.
No application code, live settings, router records, or LibreQoS output changed.
No application tests ran for this design task.

Pending: user review of the draft before implementation, as required by the invoked brainstorming skill.
After approval, write the implementation plan and test the schema transition and assignment constraints before code changes.
Keep the earlier console activation notes and runtime state intact.
See [[2026-10-30-console-activation]] and [[20-decisions/adr-0001-session-candidate-state]].

## Implementation plan

The user asked for the next step after the modal design discussion.
Prepared `docs/superpowers/plans/2026-10-30-dynamic-topology.md` with six tasks and current module references.
It specifies reversible migration, independent assignments, shared capacity constraints, hierarchy operations, modal editors, and final review.
The recommended execution method is native work in the current session with one fresh review at the end.
The plan handoff awaits user review and the execution method. No product edits or live changes occurred.

## Implementation completed

The user authorized execution. The approved feature was implemented, reviewed, verified, and pushed.
See [[2026-10-30-dynamic-topology]] for current behavior, checks, and operational limits. Earlier pending statements record the planning handoff.
