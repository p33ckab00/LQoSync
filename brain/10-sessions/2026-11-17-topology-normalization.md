---
type: session
created: 2026-11-17
tags: [topology, normalization, source, identity, reliability, ste]
status: current-normalization-complete
related: ["[[2026-11-16-topology-source-boundary]]", "[[HANDOFF]]"]
---

# Current subscriber normalization

The owner requests continuation after the topology checkpoint push.
Worktree: `/home/pi/LQoSync3.0`. Branch: `lqosync3`. Base: `489acc7`.

Agent: Codex `/root`. Timestamp: 2026-11-17T00:02:43+08:00, Asia/Shanghai.
The plan began on November 16. Checks and capture finished after midnight on November 17.

Before this slice, Codex pushed commit `489acc7c1e48760935e0131fce35f09adecde6c9` and verified the GitHub branch.
This slice remains local and uncommitted. No deployment, router write, service restart, or configuration change occurred.
Unrelated operational edits remain intact.

## Changes

Added `src/topology_manager/normalize.rs` and `normalize_tests.rs`.
Updated module exports and internal reader metadata access. No dependency or legacy path changed.
Updated the design, discussion, source plan, normalization plan, Home, and handoff notes.

The normalizer retains router ID, generation, and the original read deadline.
It checks freshness before/after normalization and on account/profile access.
It keeps typed current accounts and profiles outside the durable document.
The router result and account rows have no Debug, Clone, or serde implementation.

PPP active sessions join current secrets and profiles by exact names within one router/protocol.
Several sessions keep all current endpoints and one current account/profile reference.
Missing, disabled, or ambiguous required accounts/profiles produce typed errors.
A non-MAC caller ID does not supply a guessed MAC.

DHCP includes only bound leases and requires fresh active endpoint fields plus an unambiguous enabled server.
Known non-bound states contribute no endpoints, including retained static leases.
Unknown states, invalid endpoints, and unresolved servers produce typed errors.
Hostname, MAC, client ID, comment, and row locators remain current evidence without permanent identity authority.
Repeated hostname/MAC values do not merge leases.

Hotspot checks documented methods and local user/profile references where applicable.
MAC/trial sessions remain endpoint-only evidence. A trial without a local user retains no invented profile.
Mixed named/MAC sessions for one account stay endpoint-only in either reply order.
Profile and direct lease rates remain raw current input. Missing rates have no invented default.

IP ownership checks cover these protocol reads and report the active row index.
Indexed joins avoid repeated full-table scans. Output sorting is deterministic.
No normalization error includes customer names, addresses, MACs, or raw rates.

## Evidence

Initial red run: 18 failed against the stub. Initial green run: all 18 passed.
The conflict-location regression failed before its fix. The direct-rate regression also failed before its fix.
The focused suite now has 24 passed and zero failed.

The full suite has 927 passed, zero failed, and 9 ignored.
Format and Clippy with denied warnings passed. Whitespace and stub checks passed.

A reversed 10,000-account input produces equivalent normalized output.
This synthetic check does not establish production throughput.
The STE checker is heuristic. Manual review does not establish certified compliance.

Plan: `docs/superpowers/plans/2026-11-16-topology-normalization.md`.
Logs: `.superpowers/sdd/2026-11-16-topology-normalization/`.
Design: `docs/design/TOPOLOGY_MANAGER_IMPLEMENTATION.md`.

## Limits and next action

Local-account evidence does not prove permanent account continuity after rename, replacement, or name reuse.
DHCP hostname/MAC/client IDs and RouterOS locators do not establish that continuity by themselves.
A wildcard server does not justify guessed service membership. Unknown methods/states remain conflicts.
These checks cover current IPv4 lease/Hotspot tables and the addresses returned by PPP active reads.
They do not discover every IPv6 prefix or prove routing.

No normalizer path creates `SourceRead::Complete`, transfers saved intent, publishes shaping data, or writes the registry.
Current data may expire between normalization and its eventual use. The authorization/acceptance stage must check expiry again.
Source-scope mapping and exact source-rate parsing remain integration work.
Next: durable identity authorization and lifecycle/conflict rules against the existing registry.
Then complete migration/recovery, export/native integration, TUI, and schedules.
