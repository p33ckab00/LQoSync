---
type: decision
created: 2026-10-30
tags: [lqosync-v3, assignment, sessions]
status: accepted
related: ["[[10-sessions/2026-10-30-flexible-topology]]"]
---

# Keep session candidates separate from committed output

## Context

The user requires count assignment on a new session or reconnect, with stable assignments during continuous sessions.
A healthy empty scan or a protected core conflict can prevent a generation commit.
Generation-only evidence therefore loses valid absence events. Memory-only evidence does not survive a restart.

## Decision

Store validated session evidence and proposed assignments in private `state/online-assignments.toml`.
Use the same assignment planner for local publication, core plans, and live read-only previews.
Keep committed assignment files and their generation pointer authoritative for committed output.

Save candidate evidence after source validation and the locked checks for settings, read tickets, inventory identity, and generation.
Later capacity or core admission failures keep the last valid generation.
Corrupt candidate data blocks collection. Do not replace it through an unchecked fallback.
Compare file bytes against the validated revision before saving. This avoids a second TOML parse and rejects intervening changes.

Read tickets establish collection ordering. Whole-second timestamps establish freshness, not unique read identity.
Allow distinct successful observations in one second. Reject a timestamp decrease.
Keep one application process per data directory.

## Consequences

Healthy absence survives a blocked or empty publication and a process restart.
Continuous session metadata does not cause generation churn.
Candidate clients can receive validated pins before their first generation commit.
Candidate state adds local file I/O. Large-input checks must include its parse and durable-write cost.
DHCP bound leases prove lease presence. Polling cannot detect a reconnect with no observable evidence.
Protected manual RouterOS members remain outside application ownership.
