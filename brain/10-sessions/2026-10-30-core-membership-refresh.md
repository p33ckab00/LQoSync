---
type: session
created: 2026-10-30
tags: [lqosync-v3, core, audit, preview]
status: complete
related: ["[[2026-10-30-core-membership-audit]]"]
---

# Fresh core membership audit

Updated: 2026-10-30T14:17:38+08:00 (Asia/Shanghai).
Branch: `lqosyncv3`. Checkout: `/opt/LQoSync3.0`. Last agent: Codex.
The operator accepted the updated UI and authorized a fresh audit and migration preview.
The console now has a different process from the earlier activation note. No restart occurred during this audit.

Fresh verified read-only inputs cover two routers and 27 selected clients, one more than the earlier audit.
Observed core membership is 9/18. The current logical candidate is 13/14.
Fourteen candidate routes differ from observed membership. Preserve observed WANs before reconnect balancing begins.

The exact private proposal modifies five protected broad rows, adds two residual rows, and adds 27 owned host rows.
Three other bound rows and five rows outside the bindings remain unchanged.
The proposal retains original row IDs and comments. It includes expected values, bootstrap routes, and guarded rollback requirements.
All ten output checks passed in the separate simulation. Actual core membership remains Conflict and the committed generation remains unavailable.
A second fresh read found unchanged address-list rows, normalized subscribers, observed routes, and settings.
An independent interval-union check proves identical IPv4 membership for every router and list after the modeled operations.
The modeled rollback restores all original rows exactly. A real router rollback was not executed or tested.
Private evidence stays under `/home/pi/.local/state/lqosync3/`. No customer data or exact router rows enter Git.

The application client remains read-only. The production writer and guarded migration/rollback executor remain unimplemented.
The legacy updatecsv service remains active. Its source reads routers and can write LibreQoS output.
No service, setting, router row, or production shaping output changed during this audit.
No application code changed. No Rust test suite was required for these documentation changes.
Documentation paths and the Git diff were checked.

Next: design and test the guarded executor, including fresh-input guards, competing-writer checks, returned row IDs, and rollback recovery.
Then obtain explicit authorization for the concrete protected-row changes before execution.
Do not use the simulation Pass to unlock actual publication. Require a fresh actual Pass after any authorized migration.
