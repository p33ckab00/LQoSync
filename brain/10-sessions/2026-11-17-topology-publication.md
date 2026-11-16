---
type: session
created: 2026-11-17
tags: [topology, publication, parity, receipts, integrity, reliability, ste]
status: publication-boundary-complete
related: ["[[2026-11-17-topology-settings-recovery]]", "[[HANDOFF]]"]
---

# Isolated topology publication and legacy parity

The owner directs continuation with the publication boundary and legacy parity checks.
Agent: Codex `/root`. Timestamp: 2026-11-17T05:38:14+08:00, Asia/Shanghai.
Worktree: `/home/pi/LQoSync3.0`. Branch: `lqosync3`. Base: `489acc7` plus prior local slices.
The work remains local and uncommitted. Unrelated operational changes remain intact.

## Changes

Added publication.rs/publication_tests.rs and parity.rs/parity_tests.rs under `src/topology_manager/`.
Extended write_receipt metadata and `apply_evidence/verify` context correlation.
Updated the runner, outbox fixture, receipt tests, public topology test, and two examples for the optional metadata field.
Updated module exports, design, discussion, plan, Home, and this handoff.

Pair preparation consumes authorized reconciliation and reuses placement, aggregate checks, validation, renderers, and import normalization.
CSV rates preserve integer bps through decimal text and the f32 import conversion or produce a conflict.
Node export uses effective ceilings and stable IDs. Distinct keys combine labels with IDs.
The synthetic root remains policy metadata. Empty automatic groups disappear while configured WANs remain.
Current sessions produce distinct device rows under the same subscriber circuit ID.
No offline registry entry supplies endpoints. Export display names use IDs until explicit label migration exists.
The adapter checks the 63-level JSON import depth limit before recursive work.

Publication commits the registry candidate before file side effects and checks the source deadline at each boundary.
The current tracker writes network first, then CSV. It keeps partial outcomes and does not claim pair atomicity.
Optional receipt context stores the policy revision, observation generation, and registry digest.
Prepared/Verified evidence binds that context in addition to all previous import/native checks.
Matching unchanged content keeps its receipt identity and original attempt context.
Files without a matching receipt receive an attempt with unchanged outcomes, without replacement of their bytes.

The shared tracker now stops both file writes if durable intent storage fails.
Optional event-history faults still report degraded notification state without a false rollback of shaping.
These shared correctness changes intentionally affect the legacy tracker too.
The runner only gains an optional None initializer. It does not call the manager adapter.

The parity report exposes added, missing, changed, or duplicate rows and changed network content.
It compares exact numeric rates and normalizes JSON numeric spelling.
An unmanaged baseline row remains visible as missing. Endpoint matches do not establish identity mappings.
The report does not enroll accounts or convert legacy source-priority rules.

## Checks

The preparation red run had three failures and two passes. The parity red run had two failures and one pass.
Publication, context reuse, failed durable intent, and missing receipt each failed a behavioral check before their fix.
A parity fixture first needed test-only visibility. A fractional-rate fixture first hit a valid aggregate minimum conflict.
Explicit unit factors now isolate that precision check. Full compilation also found two example constructors that needed the optional metadata field.

This slice adds 19 checks. Full suite: 1,012 passed, zero failed, 9 ignored.
All 290 topology checks pass. Format, Clippy with denied warnings, and whitespace checks pass.
The public restart test reaches temporary pair files and the existing Prepared builder with saved Local/SourceRead settings and fresh endpoints.
It preserves the saved pin and 50/10 Mbps subscriber maximum.
The WAN clipping test preserves subscriber maxima while child ceilings stay within WAN limits.
Native proof tests use fixture artifacts. They do not prove live admission or live DPDK application.

Author review covered rates, depth, references, endpoints, deadlines, revision fences, receipts, and parity.
No independent reviewer participated. No code finding remains open in this slice.
The STE checker is heuristic. Manual review does not establish certified compliance.
No dependency, legacy config/source parser, router reader, cycle, deployment, service, or router changed.
No commit or push occurred. The graph remains at its earlier baseline.

Plan: `docs/superpowers/plans/2026-11-17-topology-publication.md`.
Logs: `.superpowers/sdd/2026-11-17-topology-publication/`.
Design: `docs/design/TOPOLOGY_MANAGER_IMPLEMENTATION.md`.

## Next action

Add explicit legacy identity/label/static ownership conversion and source-policy parity proof.
Complete exact native rate/size admission and runtime owner wiring before manager-mode activation.
Keep one writer and the existing read-only native verifier. Do not introduce another ACK loop.
Then connect enrollment, TUI, and schedules after the remaining schedule boundary decisions.
Full corrupt-store identity reconstruction also remains an activation gate.
Do not deploy this isolated path before its gates pass.
