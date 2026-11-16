# Durable identity authorization implementation plan

> **For agentic workers:** Use superpowers:executing-plans inline. Do not delegate unless the owner requests delegation.

**Goal:** Authorize independent current subscriber references, then reconcile verified returns with the existing registry.

**Architecture:** Consume fresh normalized routers and explicit source selections. An opt-in operator-managed comment contract supplies an independent reference. Reuse the observation gate and registry reconciliation. Return typed conflicts without guessed ownership.

**Tech Stack:** Existing Rust, tokio, anyhow, and registry types. Add no dependency or schema.

**Spec:** `docs/design/TOPOLOGY_MANAGER_IMPLEMENTATION.md`, discussion detail 3, and the current normalization plan.

## Global Constraints

- Preserve unrelated work and the uncommitted normalization slice. Do not commit, push, deploy, or write routers.
- Names, addresses, MACs, client IDs, and RouterOS locators alone do not authorize continuity.
- Reference authority has no implicit default. Ordinary verified returns require no manual approval after explicit enrollment.
- The operator-managed reference contract is optional. Its marker is `[lqosync-id:REFERENCE]` in current administrator-managed comments.
- The operator assigns the reference independently and never reuses it for another subscriber within its source scope.
- A current explicit reference is an operator assertion, not proof that RouterOS row IDs or account names are immutable.
- Reject missing, malformed, duplicate, and ambiguous references for selected active accounts.
- Equal references across routers or protocols do not merge circuits without existing explicit registry aliases.
- Use fresh current endpoints only. Offline records keep durable intent and contribute no active endpoints.
- Keep source profile observations current and numeric rates absent until exact source-rate integration exists.
- Preserve expiry and advancing collection generations. No current-read provenance becomes durable endpoint storage.

## Review Focus

- Reused account names without the independent reference must not receive saved policy.
- A current reference that changed must become a distinct identity. Never transfer the prior pin.
- Unknown source reads must not become complete empty reads. Check which protocols the normalizer actually read.
- Overlapping source selections and partial multi-service membership must not copy endpoints to the wrong scope.
- Current alias observations must respect registry ownership. Duplicate live claims within one scope remain critical.
- Stale revisions, mixed generations, expired evidence, and missing routers must fail before acceptance.

## Task 1: Authorize and reconcile current references

**Files:** add `src/topology_manager/identity.rs` and `identity_tests.rs`. Extend normalized provenance and module exports.

- [x] Add failing tests for enrollment, verified return with new endpoints, rename, account reuse, changed reference, and offline removal.
- [x] Add tests for DHCP endpoint-only evidence, missing/invalid/duplicate references, unconfigured authority, and MAC/trial guests.
- [x] Add tests for router/protocol separation, explicit aliases, scope selection, partial membership, and unread protocols.
- [x] Add tests for revision, generation, deadline, replay, and large input order independence.
- [x] Keep completed protocol provenance and expiry in normalized metadata.
- [x] Implement strict reference parsing under an explicit source authority contract.
- [x] Check selected membership with indexed joins. Withhold all accepted reads on critical identity conflicts.
- [x] Reuse `ObservationGate` and registry reconciliation. Return the original deadline with the reconciled world.

## Task 2: Check and capture

- [x] Run focused/full tests, format, Clippy with denied warnings, and whitespace checks.
- [x] Review the trusted operator contract, source selection, duplicate ownership, freshness, and failure paths.
- [x] Capture results, API limits, remaining integration work, and the next action in the design and brain notes.

## Execution Rulings

Worktree: `/home/pi/LQoSync3.0`, branch `lqosync3`, base `489acc7` plus local normalization.
The owner approves identity authorization and registry/pipeline reconciliation.
The deployment has no RADIUS service. This contract requires none and does not introduce router writes.

The marker format is a concrete implementation of the approved operator-reference path.
It is not an automatic interpretation of ordinary comments. Callers must select the operator-managed authority contract explicitly.
Reference values use 1–128 ASCII letters/digits with internal dot, underscore, or hyphen. Address-shaped values are invalid.
The registry account reference uses the namespace `operator-comment/v1/` to avoid collision with other authority types.

Source selections have an explicit stable scope and an optional list of current service references.
An empty list includes that router/protocol. Do not guess or split mixed service membership without endpoint-level evidence.
The returned world carries the original deadline. Later placement/publication must keep and recheck it.
Exact source-rate parsing, UI enrollment, migration, and live activation remain separate tasks.

Stop when this identity path and the existing reconciliation pass the checks. Do not add CRM, remote enrollment, or a second registry.

## Local results

Initial red run: 15 failed and 2 passed against the authorization stub.
All 22 focused identity checks pass. The public persistence/placement integration check also passes.
The trial-session regression failed before the normalization fix and passes after it.
Full suite: 950 passed, zero failed, 9 ignored. Format and Clippy with denied warnings passed.
Whitespace and stub checks passed. No dependency, schema, legacy path, service, configuration, or router write changed.

The public test covers offline removal, registry restart, verified rename/return with a new endpoint, saved pin, and unchanged 50 Mbps subscriber maximum.
The test uses an explicit local rate profile. It does not claim exact source-rate parsing or native acceptance.
A reversed 10,000-account/10,000-service input yields the same IDs and current output. It is not a production benchmark.

The scope remains local and uncommitted. Next: source-scope mapping and exact source-rate integration, then migration/recovery and publication/UI integration.
Authority selection and enrollment remain API contracts until durable configuration and UI integration exist.
