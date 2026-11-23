---
type: session
created: 2026-11-23
tags: [topology, frontend, studio, f1]
status: complete
related: ["[[2026-11-23-topology-studio-f0]]", "[[2026-11-23-topology-studio-tandem-plan]]"]
---

# Topology Studio F1: router configuration drafts

Timestamp: 2026-11-24T00:03:19+08:00 (Asia/Manila).
Base: `18ec855`. Contract: `f7e3977`. Code commits: `19397ac`, `9e18dd6`.
Branch: `codex/topology-studio-ui`.
Worktree: `/home/pi/LQoSync3.0-worktrees/codex-topology-studio-ui`.

## Result

The bounded modal reuses registration, connection forms, credential fields, and the current router-edit transport.
It reads cached router configuration and capabilities.
Each router keeps typed policy drafts and a local edit epoch.
Secrets remain separate and clear when the secret panel closes.
The explicit connection test saves no router. Registration stays in the canvas and saves the router disabled.

The source form supports PPP/PPPoE, DHCP/IPoE, Hotspot, and WAN.
It uses advertised fields and operators, node groups, saved speed plans, rate order, exact capacity, and explicit write grants.
An old backend keeps the connection editor available.
Preview and source/access persistence remain F2/F3 work.

## Verification

The model and browser entry checks failed before implementation, then passed.
Later browser checks caught tooltip overflow and the wrong tab after registration.
The independent review found a secret that survived Add router cancellation.
A render check found a clipped phone selector. Both edge regressions failed before the corrections, then passed.

Studio model: 7 passed. Workspace: 2030 passed, 0 failed, 13 ignored.
Native/Wasm Clippy, format, and whitespace passed.
The final browser gate passed 22 checks and both builds. It skipped `pulse-check`, the established exclusion.

Logs: `/tmp/lqosync-checks.8ZAiCt/`.
The optional design inspector is unavailable. Playwright supplied the rendered checks.

## Decisions and open work


- Keep source and profile persistence in F3. F1 drafts do not claim saved or applied state.

- Extend the shared forms through optional Studio props. Preserve ordinary Routers behavior.

- Use independent scroll regions for the phone selector and form.

- Defer the minor count issue for `PlanBinding`. F3 must enforce the advertised combined per-policy limit before save.

- Check the B1 rule for partial and complete profile updates before F3. The v1 view omits explicit-profile usernames.

- Keep Enable and Disable on Routers until a later canvas task.

The work changed no live router or production service. It changed no writer. No merge, push, or deploy occurred.
Next action: F2. Keep the frontend branch and worktree.

## Links


- [[2026-11-23-topology-studio-f0]]

- [[2026-11-23-topology-studio-tandem-plan]]

- [Frontend handoff](../../docs/handoffs/2026-11-23-codex-topology-frontend.md)

- [Evidence](../../docs/frontend/evidence/topology-studio-f1/README.md)
