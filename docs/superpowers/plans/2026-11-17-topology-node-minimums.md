# Plan for Explicit Node Minimums

Execute inline with test-driven development and verification-before-completion.

**Goal:** Preserve explicit native node minimums and reject impossible guarantees before publication.

**Architecture:** Add a per-direction `NodeMinimum` policy to catalog nodes. Keep it separate from capacity floors. Fold direct subscriber minimums and explicit child guarantees from leaves to roots. Export whole-Mbps minimum fields through the existing compatibility writer.

**Spec:** `docs/design/TOPOLOGY_MANAGER_IMPLEMENTATION.md`.

## Constraints


- Base checkpoint: `e21f11d`, pushed to `origin/lqosync3`.

- Preserve unrelated operational edits. Do not deploy, change routers, or activate manager mode.

- Preserve absent per-direction minimums and existing native defaults.

- Require positive whole-Mbps explicit node minimums without silent rounding.

- Count each child guarantee once. Reject guarantees above a ceiling or explicit parent guarantee.

- Keep implicit native planner commitments and live acceptance as separate activation checks.

## Task 1: Policy and aggregate guarantees


- [x] Add regression tests for explicit minimums, sibling excess, nested guarantees, and direct subscribers.

- [x] Add `NodeMinimum` with optional down/up bps values and checked conversion from legacy node templates.

- [x] Default an absent catalog field to empty and omit empty policies during serialization.

- [x] Fold guarantees iteratively with checked sums. Preserve empty automatic group removal.

- [x] Reject a declared minimum below descendant obligations or above the effective ceiling.

## Task 2: Export and migration checks


- [x] Add tests for per-direction JSON minimums, quantized ceilings, and compiler preservation.

- [x] Export explicit integer minimum fields without changing subscriber rates or WAN budgets.

- [x] Compare every source instance against the configured legacy template, including dormant instances.

- [x] Keep capacity floors and unsupported source structures as explicit conflicts.

- [x] Prove round-trip storage and reject changed or dropped minimum fields in compiled artifacts.

## Task 3: Verification and capture


- [x] Run focused and full tests, format, Clippy, and whitespace checks.

- [x] Run the installed-library scratch compiler with an explicit node minimum.

- [x] Review numerical limits, tree traversal, and every constructor.

- [x] Update design, discussion, session, Home, and handoff.

- [x] Leave this implementation slice local for review. No live dataplane apply.


## Results

Full suite: 1,137 passed, zero failed, and 12 ignored. Added ten active tests.
All 402 topology tests pass.
The installed-library scratch compiler preserves an explicit node minimum.

Format, Clippy with warnings denied, and whitespace checks pass.
Initial checks found missing integration constructors and incorrect test bounds. The corrected full suite passes.
The native schema-2 test checks node conversion only. Generated manager device IDs exceed that writer's 64-character limit.
The manager currently publishes the legacy/manual pair. Direct schema-2 circuit publication remains outside this adapter.

Implicit native planner commitments and live DPDK acceptance remain separate gates.
The author reviewed the changes. No independent review or production benchmark occurred.

Proof: `.superpowers/sdd/2026-11-17-topology-node-minimums/`.
Final logs: `full-complete.log`, `clippy-complete.log`, and `native-minimum.log`.
The graph remains the earlier baseline. Current files take precedence.
