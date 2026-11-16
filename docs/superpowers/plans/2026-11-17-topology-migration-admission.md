# Explicit Migration and Compilation Admission Implementation Plan

> **For agentic workers:** Use superpowers:executing-plans. Execute the tasks in order.

**Goal:** Check explicit legacy mappings and require exact compiled policy preservation before manager publication.

**Architecture:** Add a strict, generation-bound migration manifest and a compiled-candidate checker. Reuse parity, import normalization, rate conversion, and the current publication/receipt owner.

**Tech Stack:** Rust, serde, serde_json, existing hashes and offline compiler harness. No new dependency.

**Spec:** `docs/design/TOPOLOGY_MANAGER_IMPLEMENTATION.md`.

## Global Constraints

- Never infer subscriber identity from IP/MAC, name, or a past session.
- Keep unmapped/static baseline rows visible. Do not reclassify or remove them implicitly.
- Mapping approval applies to exact baseline/candidate digests and one observation generation.
- Keep candidate data in memory. Do not store current addresses or session locators in the registry.
- Preserve subscriber rates, node ceilings, source deadlines, and expected revisions.
- Keep the existing writer and native verifier. Do not activate manager mode or deploy.
- Preserve unrelated local work. No commit, push, router write, or service action.

## Review Focus

- Many-to-one maps and stale manifests must fail before parity can claim equivalence.
- Labels, node parents, extra metadata, and unmanaged rows must remain visible in comparison.
- A compiler success flag alone must not authorize changed rates, identities, parents, or endpoint data.
- Compilation evidence for another pair must not authorize publication.
- Input/artifact byte bounds and source deadlines must apply before durable or shaping side effects.

## Task 1: Explicit mapped parity

**Files:** add `migration.rs` and `migration_tests.rs`. Extend parity and module exports.

**Interfaces:** strict `MigrationManifest`, explicit row/node mappings, exact content fingerprints, and `compare_mapped`.

- [x] Write failing tests for explicit equivalence, changed rates/endpoints, duplicate mappings, stale digests/generation, and unmanaged rows.
- [x] Check all mapping references and bijections. Remap only declared identity/label fields for comparison.
- [x] Preserve all unmapped data in the parity report. Do not mutate baseline, registry, or prepared pair.
- [x] Keep the manifest as evidence for one snapshot. It does not enroll accounts or persist session aliases.

## Task 2: Compiled candidate admission

**Files:** add `admission.rs` and `admission_tests.rs`. Extend publication, integration tests, and the offline harness as needed.

**Interfaces:** compiled-artifact validation bound to a prepared pair, and publication that requires checked compilation evidence.

- [x] Inspect actual isolated compiler output before defining its contract.
- [x] Write failing tests for compiler loss/clamping, extra/missing objects, mismatched pair evidence, deadlines, and byte budgets.
- [x] Compare compiled node/circuit/device semantics with exact intended rates and topology.
- [x] Require checked compilation before registry commit and the current tracker call.
- [x] Run the installed compiler only in scratch directories with socket connections blocked. Do not apply native state.

## Task 3: Check and capture

- [x] Review precision, mapping authority, static ownership, deadline fences, and proof binding.
- [x] Run focused/full tests, format, Clippy, and whitespace checks.
- [x] Update design, session, Home, and handoff with results and remaining runtime activation gates.

## Stop condition

Stop when mapped parity and compiled-candidate admission pass their checks through isolated publication.
Durable export alias migration, source-priority conversion, runtime lock/owner wiring, and live enrollment remain separate activation work.
Native admission checks do not establish a live DPDK apply result. Retain the existing read-only post-write verifier.
Execute inline and use author review. Do not add another service, scheduler, or ACK loop.

## Results

The mapping red run failed four checks and passed one. The initial admission red run failed its positive check and passed three rejection checks.
Additional red runs reproduced coincident-ID ownership, unsafe node rounding, erased minimum commitments, unrequested movement, and duplicate offline node IDs.
Initial mapping code needed a borrow/type correction before the first green run.

This slice adds 19 checks. Full suite: 1,031 passed, zero failed, 9 ignored.
All 309 topology checks pass. Format, Clippy with denied warnings, whitespace, and shell syntax checks pass.
Clippy also identified one nested condition, which now uses a single conditional expression.
The public restart test now passes checked compilation before the existing publisher and Prepared builder.

The installed binding accepts manager-style IDs. The original harness prefix check incorrectly rejected them.
The new harness compares actual requested policy and endpoints.
A scratch 100.5 Mbps node compiled to 101 Mbps. Admission rejected that intrinsic rate.
The adapter now exports whole-Mbps node ceilings downward and exposes calculated/exported values.
A safe 100 Mbps node plus a 50.5 Mbps subscriber passes the final scratch compiler/checker path.
Minimum commitments and configured automatic group floors still block unsafe export quantization.
Report: `docs/reports/2026-11-17-native-node-export-precision.md`.

The current 16 MiB pair and 64 MiB combined artifact budgets bound the input readers/checker.
The borrowed import comparison avoids another full clone of compiled network/device data.
The mapping pass clones the network once and moves values thereafter. No production throughput claim forms part of these checks.

Author review covered mapping bijections, protected ownership, content/revision/generation fences, rates, artifacts, byte bounds, and the sealed publisher.
The final review regression now rejects duplicate requested node IDs in offline diagnostics too.
No independent reviewer participated. No code finding remains open in this slice.

Stop here. Durable alias/label/static ownership adoption, legacy source-priority conversion, controlled compiler/configuration binding, and runtime lock/owner wiring remain unfinished.
Full shaping-planner bounds, enrollment, TUI, schedules, corrupt-store identity reconstruction, and live DPDK acceptance also remain activation gates.
No dependency, registry schema, legacy config/source parser, router reader, cycle, service, deployment, or router changed.
No commit or push occurred. Preserve unrelated local work and all prior slices.
Logs: `.superpowers/sdd/2026-11-17-topology-migration-admission/`.
