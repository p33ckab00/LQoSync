---
type: decision
created: 2026-10-30
tags: [lqosync-v3, topology, routing]
status: accepted
---

# ADR 0002: Separate topology from routing WAN identity

Updated: 2026-10-30T12:32:44+08:00 (Asia/Shanghai).

## Context

Operators need flat roots, independent promoted roots, and arbitrary groups.
Service-specific layers must remain optional. A root can span several WANs, and several roots can share one WAN.
Exported ancestry cannot identify a subscriber's routing WAN in these arrangements.

## Decision

Keep inventory selections, exported nodes, routing WANs, and assignment policies separate.
Store each subscriber's selection, policy, exported parent, and routing WAN in assignment evidence.
Use the same captured assignments for CSV/JSON output, core membership planning, capacity checks, and validation.
Constrain minimum rates by both exported ancestors and the shared routing WAN budget.

Keep legacy settings readable until explicit validated conversion. Save a private settings backup before the first conversion write.
Retain explicit destination replacement history. Reject source edits that would make unresolved legacy evidence impossible to adopt.
Such source edits can proceed after a valid dynamic generation has replaced the legacy evidence.
Do not rewrite committed output or fabricate a reconnect to permit configuration edits.

Continuous sessions keep compatible assignments. New sessions and proven reconnects can balance only among selected eligible pairs.
Manual pins retain priority. Freshness, ownership checks, protected manual entries, and publication gates remain authoritative.

## Consequences

Topology moves preserve routing identity. Core plans no longer infer dynamic routing from exported parents.
Root-only and mixed trees share the same output format. No mandatory DHCP, PPP, or Hotspot node is added.
A rollback to an older binary needs matching pre-conversion settings and evidence, from a private data backup.
The automatic settings backup alone does not back up candidate files or the generation pointer.
Candidate, committed, and applied state remain distinct. This decision does not authorize router writes or production shaping.

See [[adr-0001-session-candidate-state]] and [[10-sessions/2026-10-30-dynamic-topology]].
