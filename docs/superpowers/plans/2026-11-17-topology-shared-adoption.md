# Shared Legacy Adoption Implementation Plan

> Execute this plan inline with the executing-plans workflow. Do not spawn agents or deploy services.

**Goal:** Extract explicitly classified shared legacy static branches and prove unchanged output before ownership adoption.

**Architecture:** Add `legacy_static.rs` as a pure proposal boundary. Reuse existing managed mapping, alias, source-policy, and parity checks.
Detach only selected static roots. Verify each actual legacy parent against its explicit managed parent mapping.
Rebuild the aliased full snapshot, merge the extracted static source, and run shared capacity and full preservation checks.

Return the candidate registry and a bounded static source draft. Do not write files or select a runtime owner.

**Tech stack:** Rust, existing serde JSON models, current registry store, current controlled compiler, and fake source tests.
**Skills:** migration, writing-plans, test-driven-development, verification-before-completion, ponytail, brain, and ASD-STE100.

## Steps

- [x] Add failing tests for nested static extraction and aliased parent preservation.

- [x] Add a bounded forest partition that rejects missing, overlapping, or incorrectly attached selected roots.

- [x] Reuse managed adoption. Expose the current tree-remap and snapshot parity helpers without changing their behavior.

- [x] Rebuild the complete aliased snapshot. Check ownership collisions, shared budgets, output bounds, and semantic equality.

- [x] Return a schema-1 static source draft with a positive revision and existing source size limits.

- [x] Keep configured adoption behind current source-filter and legacy-policy preflight checks.

- [x] Test stale snapshots, endpoint/rate drift, incomplete classification, wrong parents, budget conflicts, restart, and fresh reconnects.

- [x] Run focused and full tests, format, Clippy, whitespace, and installed-library scratch compilation where applicable.

- [x] Record results and remaining activation gates in the design and project brain.

## Forward and rollback paths

Adoption returns a proposal only. Existing readers and adoption helpers retain their contracts.
The caller must establish the returned static source as explicit operator-owned policy before normal configured publication.
The source contains only protected static rows and selected subtrees. Dynamic endpoints remain absent from registry policy and this draft.

No automatic registry commit, source replacement, output write, or mode change occurs in this slice.
The caller can discard a rejected or unused proposal without rollback writes.
If source installation and registry commit happen separately, a missing or mismatched source blocks configured publication.
Existing verified backups and forward registry recovery remain the rollback mechanism after a later committed edit.

## Scope limits

The helper does not infer ownership from matching IP/MAC addresses or rewrite a mismatched topology to make it fit.
Native implicit commitments, planner/live proof, publication activation, enrollment UI, schedules, and TUI remain separate work.


## Result

The proposal implementation and its checks are complete locally. The changes remain uncommitted.
Base checkpoint: `6e5debd`. No deployment or activation occurred.

The full suite passed: 1,167 tests, zero failures, and 13 ignored tests. All 431 active topology tests passed.
This slice adds ten active tests and one optional installed-library test.
Format, Clippy with warnings denied, and whitespace checks passed. The optional installed-library scratch compiler test also passed.

The source serializer borrows the proposal payload. Tree extraction copies each selected subtree once and retains other fields without repeated ancestor clones.
Existing parity and tree-remap functions now serve both adoption paths. Their prior tests still pass.
Author review only. No production performance benchmark occurred.

Logs: `.superpowers/sdd/2026-11-17-topology-shared-adoption/`.
Proof: `full.log`, `clippy.log`, `format.log`, and `native-adoption.log`.
Next: controlled migration staging/recovery. This API does not prove later source installation or perform a mode switch.
