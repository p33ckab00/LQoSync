---
type: session
created: 2026-11-17
tags: [topology, ownership, migration, source-policy, integrity, reliability, ste]
status: isolated-legacy-adoption-complete
related: ["[[2026-11-17-topology-migration-admission]]", "[[HANDOFF]]"]
---

# Durable legacy ownership and source priorities

## Goal

The owner authorized the next implementation task after the GitHub checkpoint.
Agent: Codex `/root`. Timestamp: 2026-11-17T07:16:59+08:00, Asia/Shanghai.
Worktree: `/home/pi/LQoSync3.0`. Branch: `lqosync3`. Base commit: `728c2c2`.

## Changes

Added `ownership.rs` and `legacy_rate.rs` with their tests.
Extended the model, store, publication, migration helper, source policy, reader, normalizer, and related tests.
Updated the implementation design, discussion, plan, Home, and this handoff.

Schema 4 stores explicit circuit/device/node aliases and protected row/root IDs.
Strict schema-1/2/3 adapters do not rewrite the primary on open. The next changed commit saves the exact old bytes.
The store retains 16 recent schema-4 backups and the backups before migration.
Ownership cannot disappear or transfer through an ordinary edit. Labels can change.
Forward policy recovery retains current export ownership and its labels.

`adopt_legacy` requires explicit snapshot mappings and complete managed-policy parity.
It binds the revision, generation, fingerprints, and original source deadline.
It produces a checked candidate for the existing store. It cannot enroll accounts or infer subscriber identity.
Device aliases refer to verified identity references. No saved endpoint or session supplies a row.
An offline subscriber emits no rows. A returned subscriber keeps its alias with current IP/MAC values.
Several sessions cannot claim one aliased device ID.

`prepare_with_static` accepts current authoritative input for independent static roots.
Protected row/root IDs must match exactly. Each protected row requires an explicit parent inside the static forest.
Missing input, unclassified data, duplicate IDs, and overlaps with managed nodes block the pair.
The future runtime owner must verify the source and revision of static input. Previous managed output is not authoritative.
Shared static branches remain unsupported until capacity ownership exists.

Legacy speed conversion preserves PPP, DHCP-server, and Hotspot base priorities.
It preserves source order, direction, units, profile-name rules, and explicit default rates.
Dynamic comments and rates come only from current evidence. Exact integer-bps conversion rejects precision loss.
Different effective speeds across active sessions block acceptance. Another enabled source-rate authority cannot coexist in the same scope.
The source layer shares profile/service indexes for each router. Ownership reservations use indexed membership checks.

## Results

Added 19 tests. Full suite: 1,050 passed, zero failed, 9 ignored. All 328 topology tests pass.
Format, Clippy with denied warnings, and whitespace checks pass.
The final fixture-only Clippy correction passed all 10 ownership tests.
The end-to-end proof covers adoption, durable store restart, fresh preparation, and compiled admission with static rows.
Behavioral red tests cover missing aliases, missing static input, absent source conversion, conflicting sessions, and missing static parents.

The first broad run exposed a repeated index copy per selection. Review removed it before the final checks.
The corrected focused topology run completed in 2.37 seconds before the last two tests.
This result is a synthetic check, not a production throughput benchmark.
No independent reviewer, live native test, deployment, service action, or router write occurred.
No dependency or legacy runtime/configuration parser changed. The manager still has no runtime caller.
No commit or push occurred in this task. Unrelated operational edits remain intact.
The project graph remains the earlier baseline. The recall query supplied leads; current code supplied the implementation facts.

## Next action

Bind the controlled compiler and configuration identity, then connect one runtime lock/writer owner.
Preserve source deadlines, pending receipts, and correlated post-write proof.
Complete shadow parity for legacy exclusions, address-based overrides, minimum/factor policies, and any shared static branches before activation.
Independent identity enrollment, full native planner bounds, TUI, schedules, corrupt-store reconstruction, and live DPDK acceptance remain gates.

Plan: `docs/superpowers/plans/2026-11-17-topology-legacy-adoption.md`.
Logs: `.superpowers/sdd/2026-11-17-topology-legacy-adoption/`.
