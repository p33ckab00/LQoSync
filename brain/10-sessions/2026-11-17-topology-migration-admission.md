---
type: session
created: 2026-11-17
tags: [topology, migration, admission, precision, integrity, reliability, ste]
status: mapping-compilation-gates-complete
related: ["[[2026-11-17-topology-publication]]", "[[HANDOFF]]"]
---

# Explicit migration mappings and compilation admission

The owner directs continuation with explicit legacy mappings and native admission checks.
Agent: Codex `/root`. Timestamp: 2026-11-17T06:28:03+08:00, Asia/Shanghai.
Worktree: `/home/pi/LQoSync3.0`. Branch: `lqosync3`. Base: `489acc7` plus prior local slices.
All new work remains local and uncommitted. Unrelated operational changes remain intact.

## Changes

Added migration.rs/migration_tests.rs and admission.rs/admission_tests.rs under `src/topology_manager/`.
Extended parity issues, publication and its tests, and the public topology integration test.
Added the diagnostic `examples/topology_compilation_check.rs` and strengthened `tools/v3sim/check.sh`.
Extended `import_evidence` with borrowed network/device comparison. Existing matches semantics remain unchanged.
Updated design, discussion, plan, precision report, Home, and handoff.

Migration manifests bind explicit row/node mappings to a revision, observation generation, and baseline/candidate fingerprints.
They remap declared IDs/labels for comparison without changing rates, endpoints, hierarchy, or extra metadata.
Unmapped and protected rows remain explicit issues, even when their IDs coincide with current managed rows.
Duplicate or many-to-one ownership, stale evidence, and overlapping managed/protected ownership fail.
Strict manifests contain no address/session fields and do not transfer registry identity or policy.
They apply to one snapshot. Durable alias/static ownership adoption remains unfinished.

Only AdmittedPair exposes the public manager publisher.
The checker consumes a fresh PreparedPair and checks canonical plus compiled shaping artifacts against it.
It checks schema/source, ingress identity, nodes, ancestry, attachments, intrinsic rates, complete device policy, and current endpoints.
Unrequested movement, allowed parents, anchors, circuit-match extensions, or another candidate cannot pass.
The existing post-write Prepared/Verified path still establishes import/native application separately.

Pair input has a 16 MiB budget. Combined compiler artifacts have a 64 MiB budget.
The diagnostic readers enforce combined bounds before parse. A counter checks serialized artifact size without another full copy.
Borrowed import comparison avoids temporary cloned artifact trees. The mapping pass clones the baseline network once and then moves values.
These code checks do not establish production throughput.

## Installed compiler evidence

Package: `3.0.7.202611162151`. The precision report records the binding hash.
The existing harness uses scratch paths, blocked sockets, and its write audit. No live apply occurred.
Manager-style IDs compile successfully. The old prefix-only harness check incorrectly rejected them.
A requested 100.5 Mbps node compiles to an intrinsic 101 Mbps, despite compiler success.
The compatibility tree alone does not expose that increase. The new admission check rejects it.

The adapter now applies the approved ceiling-safe export rule to whole-Mbps node limits.
It keeps the calculated ceiling and exposes the exported ceiling through node_export_adjustments.
It rejects a ceiling below 1 Mbps, current subscriber minimum commitments, or a configured automatic group floor.
Subscriber min/max speeds remain unchanged. The final 100 Mbps node/50.5 Mbps subscriber scratch check passes.

Report: `docs/reports/2026-11-17-native-node-export-precision.md`.

## Checks and review

The initial mapping run failed four checks and passed one. The initial admission run failed one and passed three.
Additional behavioral red runs cover coincident-ID ownership, quantization, minimum commitments, unrequested movement, and duplicate offline node IDs.
Initial mapping code needed a borrow/type correction. Clippy identified one nested condition.

This slice adds 19 checks. Full suite: 1,031 passed, zero failed, 9 ignored. All 309 topology checks pass.
Format, Clippy with denied warnings, whitespace, and shell syntax checks pass.
The public restart proof passes compiled admission before temporary publication and the existing Prepared builder.
The final installed-binding scratch check passes. The unquantized fractional sample correctly fails admission.

Author review covered mapping authority, protected data, precision, byte bounds, deadlines, and publisher access.
The final review regression rejects duplicate node IDs in offline diagnostics.
No independent reviewer participated. No code finding remains open in this slice.
The STE checker is heuristic. Manual review does not establish certified compliance.
No dependency, registry schema, legacy config/source parser, router reader, cycle, deployment, service, or router changed.
No commit or push occurred. The project graph remains the earlier baseline.

Plan: `docs/superpowers/plans/2026-11-17-topology-migration-admission.md`.
Logs: `.superpowers/sdd/2026-11-17-topology-migration-admission/`.

## Next action

Implement durable legacy alias/label/static ownership adoption and source-priority conversion with parity proof.
Then connect the controlled compiler and runtime lock/writer owner with current compiler/configuration identity checks.
Complete full shaping-planner admission, enrollment, TUI, schedules, and live DPDK acceptance before activation.
Full corrupt-store identity reconstruction also remains a gate. Do not activate or deploy manager mode yet.
