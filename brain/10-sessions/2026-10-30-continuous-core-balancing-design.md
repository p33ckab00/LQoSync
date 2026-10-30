---
type: session
created: 2026-10-30
tags: [lqosync-v3, balancing, failover, design]
status: planned
related: ["[[2026-10-30-core-production-adapter]]"]
---

# Continuous core balancing design

Updated: 2026-10-30T18:39:42+08:00 (Asia/Shanghai). Last agent: Codex.
Branch: `lqosyncv3`. Checkout: `/opt/LQoSync3.0`.
The operator accepted active-client evacuation on confirmed WAN failure, with possible connection interruption.
Normal balancing retains continuous sessions and reassesses arrivals and reconnects.
Keep WAN failure policy configurable. Pin overrides default off as an engineering proposal.
Automatic mode remains disabled until a separate production rollout authorization.

Added the continuous balancing design and implementation plan under `docs/superpowers/`.
The plan reuses session scoring, ownership validation, exact row guards, and journal recovery.
Proposed health input uses operator-configured ICMP Netwatch probes with verified per-WAN attribution.
Probe completion freshness and same-core list changes require isolated RouterOS verification.
Unknown health does not authorize evacuation. Protected residual overlap blocks automatic moves.
Do not weaken the maintenance engine's membership-preservation invariant to implement the new writer.
Connection marks can preserve old routing after a list move. Actual traffic failover needs separate evidence.

Checks: document links and diff reviewed. No implementation tests ran for this feature.
No live router writes, service changes, or restart occurred in this design task.
The prior migration review is historical and must not be applied without fresh evidence and authorization.
Next: execute the plan in an isolated worktree, starting with health configuration and pure state transitions.
The existing uncommitted `v3/OPERATIONS.md` quick restart guide must be preserved.
