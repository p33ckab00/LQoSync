---
type: session
created: 2026-10-30
tags: [lqosync-v3, skills, workflow]
status: complete
---

# Shared skills for LQoSync v3

The user authorized use of the shared skills already available to TI.
This task adds project instructions for Claude Code and Codex.
Branch: `lqosyncv3`. Checkout: `/opt/LQoSync3.0`. Base: `32eb255`.

## Changes

- Added root `AGENTS.md` with shared skill paths, project context, operational limits, and verification commands.
- Added root `CLAUDE.md` as the Claude Code entry point to the same instructions.
- Linked this note from [[Home]] and updated [[HANDOFF]].

The workflow uses the existing shared skills. It does not need another plugin installation.
LQoSync retains its own vault and product rules. TI UI rules do not apply to LQoSync.

## Verification

Verified shared skill files, project references, and note links. Checked the diff for whitespace errors.
This change affects documentation only. Application tests did not run for this task.
No runtime settings, router configuration, LibreQoS files, packages, or services changed.

## Next action

Use the shared workflow for subsequent LQoSync tasks.
The prior project task remains the review of protected manual membership and a per-host migration plan.
That review must preserve ownership checks. See [[2026-10-29-scoped-routing-minimums]].
