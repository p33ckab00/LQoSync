---
type: session
created: 2026-10-30
tags: [lqosync-v3, core, ownership, audit]
status: complete
related: ["[[2026-10-30-dynamic-topology-activation]]"]
---

# Core membership audit and resolution preview

Updated: 2026-10-30T13:37:59+08:00 (Asia/Shanghai).
Branch: `lqosyncv3`. Checkout: `/opt/LQoSync3.0`. Active application code: `9070d1b`.
The user authorized the read-only core audit, resolution preparation, and validation. Production router writes remained separate.

## Findings

Fresh verified reads covered two routers and 26 selected subscribers.
The bound WAN lists contain eight protected broad entries: four prefixes and four ranges.
Five entries overlap selected clients: two prefixes and three ranges. Three bound entries do not overlap; five rows outside the bindings remain untouched.
All conflicting rows are static, enabled, permanent, and labeled without this application's ownership marker.
The planner correctly blocks automatic adoption or removal of these protected rows.

Existing core-list membership is 9/17; candidate membership is 13/13. Fourteen selected clients differ.
These counts describe configured membership and local assignments; they do not measure actual packet paths.
Directly applying the existing candidate would change those clients' WAN membership during continuous sessions.
The recommended bootstrap preserves observed current membership, then balances new sessions and reconnects.
This alternative remains a preview; the live candidate and its settings were not changed by the audit.

## Concrete preview and proof

A private continuity-preserving proposal changes five original broad addresses, adds three residual entries, and adds 26 owned host entries.
Original row IDs and comments remain on the modified residual rows. Three unaffected bound rows and all five outside rows remain untouched.
Set subtraction and cardinality checks prove that every non-selected address keeps its previous list membership.
The owned hosts use the same observed WANs, producing zero modeled WAN membership moves.
A local candidate bootstrap would align 14 logical assignments through guarded APIs, retain session evidence, and clear temporary pins afterward.
No direct state-file edit or fabricated reconnect is permitted.

The existing planner and all ten output checks passed against the explicitly simulated inventory and a separate private simulation directory.
Checks cover inventory, speeds, assignments, minimums, capacity, snapshot consistency, CSV, network JSON, core membership, and core-plan JSON.
The first 13/13 simulation also passed the model checks but would move 14 memberships; it is not the recommended continuity baseline.
Neither simulation is a live Pass. No output was staged in the actual application directory.
Private input snapshots, exact diffs, rollback preconditions, and the reusable audit source remain under `/home/pi/.local/state/lqosync3/`.
No credentials, customer identities, addresses, raw rows, or comments are copied into Git or these notes.

## Operational result and next action

Fresh live checks still pass inventory, speeds, assignments, minimums, and capacity. Core membership remains Conflict; no committed generation exists.
Actual settings and address-list rows matched the captured values after the audit. The live console remained healthy.
One on-demand read was superseded by a scheduled read; the stale result was discarded by the existing guard.
No production service, router entry, LibreQoS output, package, or application code changed.
The active legacy updatecsv service was inspected without execution or modification. Its source reads RouterOS and can update LibreQoS files.

The v3 RouterOS client is still read-only. A guarded production migration/rollback executor and long-running writer remain unimplemented.
Next: design and verify that executor, including observed-WAN bootstrap, fresh-input checks, exact row scope, returned-ID rollback, and competing-writer review.
Then obtain explicit authorization for the concrete protected-entry migration. Actual live validation must Pass before actual publication.
Do not weaken ownership checks, claim broad rows, or promote simulation output as a way to remove the current conflict.
See [[2026-10-30-dynamic-topology-activation]] and [[20-decisions/adr-0002-independent-topology-routing]].
