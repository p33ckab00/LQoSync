---
type: session
created: 2026-11-17
tags: [topology, exclusions, sources, parity, integrity, reliability, ste]
status: current-source-exclusions-complete
related: ["[[2026-11-17-topology-legacy-preflight]]", "[[HANDOFF]]", "[[ADR-0009-dhcp-discovery-and-exclusions]]"]
---

# Current-source exclusion filters

Time: 2026-11-17T15:12:42+08:00 (Asia/Shanghai).
Agent: Codex `/root`. Worktree: `/home/pi/LQoSync3.0`. Branch: `lqosync3`.

The owner authorized the preflight commit and push, then the next task.
Checkpoint: `5528f9e`, pushed to `origin/lqosync3`.
These exclusion changes remain local and uncommitted.

## Goal

Apply legacy exclusions to fresh manager observations without stale endpoint reuse or weaker source validation.

## Changes

- Shared the matcher in `src/sources.rs`. Added direct lookup for parsed IPv4 addresses.

- Added configured normalization and `src/topology_manager/exclusion_tests.rs`.

- Extended coordinator, preflight, prepared publication, configured ownership adoption, runner, and their tests.

- Updated implementation design, discussion, task plan, Home, and handoff. No dependency or durable schema changed.

The matcher joins global and router-specific lists.
IP rules apply to all selected subscriber protocols. PPP profiles and DHCP server names use case-insensitive wildcards.
IPv4/CIDR filters do not exclude IPv6 sessions. Unrelated protocol rules have no effect.

Every active row still requires valid current account/profile/server joins, endpoints, and authentication evidence.
Duplicate IPs still block the batch, even for excluded rows.
Filters act before the normalizer groups accounts. Excluded sessions contribute no active comment or service membership.
Profile/server inventory remains visible. Excluded accounts do not need identity enrollment.

The coordinator shares one immutable global exclusion snapshot. It does not copy the whole config for each router.
The matcher checks each inventory name once. Current parsed IPv4 values avoid another text parse.
The legacy collector preserves its current matcher behavior.

A prepared pair carries an optional transient digest of its effective filter policy.
Configured publication and configured legacy adoption require the current digest.
A changed or removed filter list invalidates the pair. Missing proof cannot pass under active filters.
The digest contains no subscriber endpoints and does not extend the source deadline.

Complete exclusions remove subscriber rows and automatic child groups from prepared output.
Configured WAN roots remain. Stored identity and policy intent survive.
A later included session uses fresh IP/MAC data and its independent reference to recover the same Subscriber ID.

## Verification

- Full suite: 1,108 passed, zero failed, and 12 ignored. Added 12 active tests.

- All 373 topology tests pass. The subset also has one optional ignored test.

- Seven normalization tests cover merged lists, wildcards, unrelated protocols, IPv6, partial sessions, row validation, duplicate IPs, expiry, and legacy parity.

- Coordinator tests cover removal, fresh reconnect, enrollment bypass for excluded accounts, policy changes, and configured alias adoption.

- The runner rejects a filtered pair after its policy changes, before compilation or writer ownership.

- The optional installed-library public sync test passes. It publishes an excluded batch with no subscriber rows, then active/disconnected/reconnected batches.

- All native test writes target temporary directories. No live DPDK apply occurred.

- Format, Clippy with warnings denied, and whitespace checks pass. Author review only.

Logs: `.superpowers/sdd/2026-11-17-topology-exclusions/`.
Proof: `full.log`, `clippy.log`, `native-sync.log`, `topology-final.log`, `publication.log`, and red regression logs.
The parity fixture uses current bound IP/MAC values for both collectors. It does not read stored endpoint data.

## Limits and next action

DHCP MAC-address speed overrides remain blocked until explicit independent identity mappings exist.
Dynamic bounds and native group minimums remain separate migration tasks.
Static-input authority, shared branches, enrollment, native planner bounds, and live acceptance remain activation gates.
The live service loop remains unchanged. The filter digest is not general activation authority.

No live source read, router write, service action, deployment, benchmark, or automatic mode switch occurred.
Unrelated operational edits remain intact. The graph remains the earlier baseline.

Next: map DHCP overrides to durable subscriber identities explicitly, with offline/reconnect and conflict tests.
Do not infer continuity from IP, MAC, hostname, or old session rows.

Plan: `docs/superpowers/plans/2026-11-17-topology-exclusions.md`.
Design: `docs/design/TOPOLOGY_MANAGER_IMPLEMENTATION.md`.
