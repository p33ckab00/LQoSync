# Current subscriber normalization implementation plan

> **For agentic workers:** Use superpowers:executing-plans inline. Do not delegate unless the owner requests delegation.

**Goal:** Convert complete current RouterOS evidence into typed current accounts, profiles, and endpoints without invented identity continuity.

**Architecture:** Reuse the bounded-age reader and current endpoint type. Normalize local accounts and current sessions with indexed joins. Keep account evidence separate from permanent identity authorization.

**Tech Stack:** Current Rust, tokio, anyhow, and RouterSession. Add no dependency.

**Spec:** `docs/design/TOPOLOGY_MANAGER_IMPLEMENTATION.md` and discussion detail 3.

## Global Constraints

- Preserve unrelated work. Do not deploy, restart services, write routers, or change live configuration.
- No RADIUS dependency. No IP, MAC, hostname, or row locator becomes a permanent subscriber ID.
- Current rows alone determine presence and endpoints. Never restore an old configured address.
- PPP/Hotspot local account joins must be unambiguous and use the current inventory.
- MAC/trial Hotspot presence remains endpoint-only evidence. It does not prove stable account continuity.
- Current profiles keep raw rate input. This task does not invent a default or interpret absent rates.
- Keep the reader deadline across normalization. Reject expired input and expired output access.
- Keep normalized data outside the durable policy document. Do not claim `SourceRead::Complete` without identity authorization.

## Review Focus

- Duplicate account/profile names and missing current references must produce typed conflicts.
- Active sessions for disabled or missing local accounts must not silently disappear.
- Unbound static leases must not contribute endpoints. Bound leases require current active endpoint fields.
- Multiple sessions for one account must form one current account with all current endpoints.
- IP conflicts must remain visible across protocols. MAC reuse does not merge accounts.
- Replies in another order must produce equivalent normalized output with linear indexed joins and final sorting.

## Task 1: Add typed normalization with regression checks

**Files:** add `src/topology_manager/normalize.rs` and `normalize_tests.rs`. Extend reader metadata access and module exports.

- [x] Write failing tests for PPP account/profile joins, multi-session accounts, disabled accounts, and offline removal.
- [x] Write failing tests for bound-only DHCP, current active fields, duplicate hostname/MAC, and server validation.
- [x] Write failing tests for Hotspot local authentication, MAC/trial evidence, unknown methods, and duplicate accounts.
- [x] Test malformed endpoints, cross-protocol IP conflicts, deterministic ordering, expiry, and a large account set.
- [x] Implement typed current evidence and indexed joins. Keep error text free of customer names and endpoint values.
- [x] Keep endpoint-only evidence distinct from local account evidence. Neither establishes durable continuity by itself.

## Task 2: Check and capture

- [x] Run focused/full tests, format, Clippy with denied warnings, and whitespace checks.
- [x] Review source freshness, endpoint ownership, joins, and privacy.
- [x] Update design, discussion, session, Home, and handoff notes with exact scope and limits.

## Execution Rulings

Worktree: `/home/pi/LQoSync3.0`, branch `lqosync3`, base `489acc7`.
The owner requests continuation after the verified topology checkpoint push.

The source clarification identifies local RouterOS tables. It does not supply an immutable account lifecycle reference.
This task completes current protocol normalization. Durable identity authorization remains a distinct integration gate.
The next identity stage must reject ambiguous reuse and must not infer continuity from these normalized endpoint attributes.

Stop after current normalization passes these checks and you record its limits. Do not add a new registry schema or collector.

## Local results

The initial red run had 18 failures. All 24 focused checks now pass.
Review regressions proved the active-row conflict location and direct rate input before their fixes.

Full suite: 927 passed, zero failed, 9 ignored. Format and Clippy with denied warnings passed.
Whitespace and stub checks passed. No dependency, live caller, source acceptance, or durable registry write changed.

This task finishes current normalization only. Durable identity authorization remains the next integration gate.
The reversed 10,000-account check is synthetic evidence, not a production benchmark.
