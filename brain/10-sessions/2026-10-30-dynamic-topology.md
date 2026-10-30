---
type: session
created: 2026-10-30
tags: [lqosync-v3, topology, routing, console]
status: complete
---

# Dynamic topology and modal configuration

Updated: 2026-10-30T12:37:50+08:00 (Asia/Shanghai).
Branch: `lqosyncv3`. Checkout: `/opt/LQoSync3.0`. Baseline: `ac4c922`.
Core commit: `7c48e06`. Controls and final corrections: `9070d1b`.
Both code commits reached GitHub. The remote branch hash matched `9070d1b41e626ed6ebcf835d975b6f40ef88d71f`.

## Result

Inventory selections, exported parents, and routing WANs now have separate identities.
Operators can configure root-only trees, multiple roots, nested groups, mixed-service destinations, eligible pairs, defaults, metrics, and pins.
Add, remove, promote, and demote preserve stable IDs. Removal requires explicit replacements for dependent destinations and children.
Independent promoted roots keep their routing WAN association. Shared roots and WANs use both ancestor and WAN minimum budgets.
Session evidence keeps continuous clients stable. New sessions and proven reconnects can balance only among selected eligible pairs.
Core address-list planning uses the same captured routing assignments as output and validation.
Legacy settings remain readable until explicit validated conversion. A private matching settings backup precedes the first conversion save.
Replacement history resolves old destinations. Source edits that would orphan unresolved legacy evidence are rejected before save.
A matching private data backup is required for rollback to an older binary; the settings backup does not include candidate files or the generation pointer.

Configuration forms now use cog controls and native dialogs. Dialogs preserve drafts, contain focus, close mobile navigation, and retain failed submissions.
Action labels remain visible. Pin titles identify the subscriber. Recognized field errors have adjacent text and accessibility associations.
A missing resolved WAN pair makes the legacy pin control read-only until fresh inventory resolves it.
Removal dialogs show separate committed and stored-candidate subscriber counts, including inactive assignment records.
Policy summaries are indexed and bounded. Assignment HTML renders outside the application mutex and reuses captured evidence for impact counts.

## Verification

Final isolated checks: `cargo fmt --check`, `cargo test --locked`, strict all-target Clippy, locked binary build, and JavaScript syntax all passed.
The normal suite passed 196 tests, zero failures; 12 manual benchmarks were ignored.
The new 50k selector workload passed separately, including assignment retention, reconnects, shared WAN budgets, persistence, and HTTP rendering.
Empty-evidence policy page: 1.433 seconds, 31,658 bytes. Page with stored candidate: 5.651 seconds, 31,646 bytes.
Collection: 1.167 seconds. Assignment: 1.563 seconds. Retention including collection: 3.901 seconds.
Allocation/compile: 1.554 seconds. Reconnect: 2.273 seconds. Candidate load: 3.985 seconds.
Durable save varied from 1.520 to 7.515 seconds across runs with different host I/O load. These are synthetic debug measurements.
Packaged browser checks passed on 12 routes at 1440px, 390px, and 320px, with a 720px zoom-equivalent dialog check.
They covered layers, focus, Tab/Escape, drafts, invalid save/retry, rejected tokens, field associations, promotion, demotion, replacements, policy edits, and root creation/removal.
Main was fast-forwarded to the exact verified code. Its `v3/` tree matched the isolated worktree.

One fresh whole-change review found five Important and two Minor issues, with no Critical finding.
The correction pass addressed legacy adoption, exact pin pairs, stale forms, rendering cost, impact counts, action labels, and field associations.
Regressions reproduced the relevant failures before correction. Browser checks also found named form controls shadowing DOM properties; attribute access fixed that defect.
An existing five-second mock-router test timed out once under build/browser load. It passed alone in 0.84 seconds without a timeout change.
The final full run passed with limited build concurrency. No unresolved material review finding remains.

## Operational state

The synthetic preview stopped after verification. The existing live console process remains on the earlier activated `ac4c922` build.
No live restart, router write, production shaping, dependency change, or manual-entry bypass occurred.
Private settings, credentials, inventory, and customer data remain out of Git and these notes.
The Graphify map predates this change; current source confirmed the relationships.
Next: activate the new build in a separately authorized restart, then select the operator's actual roots, sources, and WAN pairs through the console.
Keep protected manual membership and all publication gates intact.

See [[20-decisions/adr-0002-independent-topology-routing]], [[20-decisions/adr-0001-session-candidate-state]], and [[2026-10-30-dynamic-topology-design]].
