---
type: session
created: 2026-11-09
tags: [session, libreqos-v3, node-minimums]
status: deployment-pending
related: ["[[2026-11-09-libreqos-beta19-integration-review]]", "[[HANDOFF]]"]
---

# Optional node minimums

## Scope

The user authorized priority task 1, code review, checks, commit, push, and an LQoSync3 restart.
Add optional root and group minimums. Keep the live configuration and topology source unchanged.

## Changes

- Add optional whole-Mbps minimums to root and group configuration, Settings metadata, state, and pair output.
- Reject zero, negative, fractional, and invalid rates. Reject generated minimums above their ceilings before live writes.
- Include minimum-only changes in node diffs. Carry explicit minimums into the native writer without bypass of its readiness guard.
- Preserve manual minimums with old state. Honor manual changes and removals in preserve mode.
- Keep old state readable. Omit absent minimums from state and network output.

## Review

Review covered config input, generated nodes, carried nodes, state, preserve mode, validation, diffs, and both writers.
The review found a preserve-mode defect with old state. Regression tests reproduced it, then passed after the fix.
Manual changes and removals also have regression tests.
The change adds bounded work per node within current passes. It adds no network calls, background tasks, or dependencies.
It adds no work to the packet path. This review did not benchmark live throughput.

## Checks

- `cargo fmt --check`: pass.
- `cargo clippy --all-targets --locked -- -D warnings`: pass.
- `cargo test --locked`: 489 pass, zero failures, six manual tests ignored.
- Installed pair compiler check: one more manual test passed with `3.0.7.202611082352`.
  The generated pair retained root minimums 20/10 and group minimums 8/4 in canonical output.
  The check used temporary directories and injected `EACCES` for every socket connection.
- Live minimum policy and final runtime parent clamps were not exercised. No new minimum is enabled on the shaper.

## Deployment

The current service uses `/usr/local/bin/lqosync3`, version `3.0.136+ge8d6cbd`.
The live target remains `v3_pair`. The native probe remains blocked.
Host permissions prevent automatic deployment: `sudo -n true` requires a password.
`systemctl --no-ask-password restart lqosync3.service` also requires interactive authentication and did not restart the service.
After commit and push, build the committed revision and prepare a checked deployment script with a binary backup.
The operator must execute the root deployment step. Then confirm the new process version and a successful cycle.
The graph was not rebuilt. Current source and this note supersede its old node field descriptions.
