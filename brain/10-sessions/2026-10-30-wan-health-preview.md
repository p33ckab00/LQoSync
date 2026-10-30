---
type: session
created: 2026-10-30
tags: [lqosync-v3, wan-health, preview]
status: implemented-preview
related: ["[[2026-10-30-continuous-core-balancing-design]]"]
---

# WAN health and read-only preview

Branch: `codex/wan-health`. Worktree: `/tmp/lqosync-wan-health`. Base: `7a9f198`.
The operator authorized the health and preview phase. The continuous writer remains a later phase.
Added optional validated health settings, a pure state machine, a conservative Netwatch provider, and an explicit preview CLI.
Existing settings without the health section preserve disabled health configuration and existing serialization.
The first sample establishes a baseline. Only advancing counters for both selected probes count as a new round.
Repeated reads cannot refresh freshness. Settings changes, restarts, missing evidence, and stale progress reset confidence.
Unknown cannot authorize failover. The CLI does not save settings, inventory, assignments, or health state.

The provider requires distinct external IPv4 ICMP targets with dedicated active static main-table routes through the mapped WAN.
It blocks unsupported output routing, scripts, explicit source bindings, ambiguous routes, and unavailable counters.
An absent or inactive probe route remains Unknown in this first provider.
The CLI reads each selected core once per round. It performs sequential bounded client operations and does not run subscriber planning.
The console does not yet display or consume health state. No live configuration, router read/write, or restart occurred during implementation.

Focused verification: 11 tests passed. RED failures proved the missing module/CLI and policy-reset regression before correction.
A full check found one existing Settings fixture that needed the new optional field. The fixture now includes None.
Fresh read-only review found no material code defect. Actual deployed Netwatch counter semantics still require isolated RouterOS proof.
Updated: 2026-10-30T18:54:37+08:00 (Asia/Shanghai).
Final verification: 243 tests passed, zero failures, 12 manual tests ignored.
Format, strict Clippy, diff, documentation links, Bash syntax, and TOML example checks passed.

Next: implement reconciliation decisions and health-aware destination eligibility under the existing plan.
Keep live automatic mode disabled until provider acceptance, guarded writer completion, and production authorization.
