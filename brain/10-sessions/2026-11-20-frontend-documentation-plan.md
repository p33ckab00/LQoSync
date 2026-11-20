---
type: session
created: 2026-11-20
tags: [frontend, design, planning, rust]
status: documented
related: ["[[2026-11-20-frontend-blueprint-research]]", "[[2026-11-18-topology-service-api]]"]
---

# Frontend documentation and atomic plan

Checkpoint: 2026-11-20T16:11:44+08:00 (Asia/Shanghai).
Agent: Codex `/root`. Worktree: `/home/pi/LQoSync3.0`. Branch: `lqosync3`.
Source baseline: `9a465e7`.

## Owner request

Save every generated picture under `docs/frontend` with documentation.
Write the feature design and atomic implementation plan in STE.
Keep Rust application logic, connect the frontend to the current backend, and record missing functions.
Address performance, reliability, and integrity. Commit and push to `origin/lqosync3`.

## Deliverables

The [frontend index](../../docs/frontend/README.md) links the complete package.
It contains 11 original PNG copies, hashes, prompts, screen descriptions, feature design, contracts, backend gaps, and quality requirements.
The implementation plan has 19 task cards with dependencies, interfaces, files, tests, and stop conditions.
The package preserves the TUI and uses Leptos CSR with Rust/Wasm.

The proposed integration uses an unprivileged Rust HTTP gateway and a narrow local broker.
The existing service remains the policy and publication owner.
Local plan workflows replace the mockups' router-profile sync behavior.
Routing writes, schedules, remote control, and destination intelligence retain explicit domain gates.

## Checks and limits

All image hashes and relative document links pass.
The task order satisfies the declared dependencies. Whitespace checks pass.
The STE heuristic and manual review cover the documents. No certified compliance is claimed.
No application code, runtime state, or dependency changed. No Rust or browser tests were necessary for this documentation-only change.
The graph remains the earlier index. Current source and the backend map take precedence.
The [validation report](../../docs/frontend/VALIDATION.md) records the scope of proof.

## Primary frontend clarification

The owner selected the web UI as the primary frontend and operator workflow surface.
The TUI stays as is. Web delivery does not require a TUI process, redesign, or migration.
The Rust service retains execution authority. CLI-only account administration remains unchanged.
Updated the feature design, implementation constraints, backend map, and independence checks.

## Next action

Follow-up owner direction: manage all usernames and passwords through root CLI only.
Allow multiple superadmins. The browser supplies sign-in/sign-out, with no signup or account CRUD routes.
Updated `docs/frontend/CLI_ACCOUNT_POLICY.md` and the connected design, contract, and task documents.
The Users mockup remains a withdrawn concept. The actual navigation excludes Users.

Execute F00 when implementation starts. Record compatible dependencies and a measured baseline before feature code.
This request authorizes documentation and its GitHub publication, not production deployment.


## 2026-11-20T16:47:12+08:00 — Sign-in approval and theme direction

The owner accepted the original sign-in layout and logo mark.
Requested changes: restrained glass effect, superscript v3, and application-wide Light/Dark/System modes.
Generated dark and light variants from the original. Saved all three sign-in images under `docs/frontend/images/`.
The image set now has 14 PNG files with matching SHA-256 hashes.
Updated THEMING, design, screen catalog, prompts, quality requirements, sources, and F05 theme tasks.
System is the default and follows the client environment. Manual overrides remain explicit.
Checks: 14 hashes, relative links, STE review, and whitespace. No frontend implementation or browser proof occurred.


## 2026-11-20T16:53:01+08:00 — Feature-local settings and shared theme selector

Owner direction: show Light / Dark / System on sign-in and the main screens.
Each feature owns its configuration/settings screen. PPPoE settings stay in PPPoE, with the same rule for other features.
Global Settings contains application-wide fields only.
Added `docs/frontend/SCREEN_SETTINGS.md` with screen ownership, scoped API requirements, task mapping, and acceptance checks.
Updated design, contracts, mockup corrections, theme direction, quality checks, and F05/F15/F18.
Scoped forms share the backend transaction and digest conflict boundary. No duplicate configuration authority was introduced.
Checks: document links, 19-task dependency order, 14 image hashes, STE review, and whitespace.
No implementation, application tests, or runtime changes occurred. Account CRUD remains CLI-only. The TUI stays as is.


## 2026-11-20T16:58:02+08:00 — Company branding in global Settings

The owner requested company name/logo configuration beside LQoSync branding, with commit and push.
Added `docs/frontend/COMPANY_BRANDING.md` and F19. The plan now contains 20 atomic task cards.
Company identity supplements the accepted LQoSync logo and superscript v3 on sign-in and the shared main header.
Branding stays in gateway-owned presentation storage, separate from core service config.
Updated global field ownership, safe public branding DTOs, upload bounds, preview/save, asset recovery, and theme checks.
Checks: links, 20-task dependency order, 14 image hashes, STE review, and whitespace.
No image, application code, live configuration, or runtime behavior changed.
