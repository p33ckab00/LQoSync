# Guarded core migration implementation plan

> **For agentic workers:** Use superpowers:executing-plans to implement these tasks inline.

**Goal:** Build and test the guarded migration and rollback executor described to the operator.

**Architecture:** Add a separate Rust executor library with an injected transport and a durable private journal. Keep the live router client, console routes, publication gates, and services unchanged. A future production adapter must supply fresh configuration/session evidence and exact address-list rows.

**Tech stack:** Existing Rust, serde, UUID revision digests, and filesystem APIs. Add no dependencies.

**Spec:** The operator-approved flow in the conversation and the private migration preview. Private customer evidence must remain outside Git.

## Scope and constraints

This stage implements the execution engine and isolated failure tests. It does not connect writes to the live console.
Protect original row IDs and comments. Add residual and owned host rows before narrowing original broad rows.
Require exact approved-plan identity, fresh stable source/configuration evidence, and an exclusive journal claim before execution.
Reject changes that alter the full address membership of any list. Reject unrelated rows, existing ownership markers, dynamic rows, and expiring rows.
Require transport implementations to exclude competing writers. RouterOS has no compare-and-swap transaction in this engine.
A transport error may mean that a write succeeded. Journal the intent before every request and never retry an unknown add automatically.
Stop safely on drift, unknown outcomes, or journal failures. Rollback restores original rows before removing exact returned IDs.
A simulator Pass must not unlock actual publication. A final fresh actual validation belongs to the production integration stage.

## Tasks

- [x] Add failing tests for exact scope, membership preservation, stale inputs, authorization mismatch, and duplicate execution.
- [x] Implement typed plans and independent interval-union checks. Compile concrete residual and host additions from explicit protected-row selections.
- [x] Add failing tests for partial writes, lost replies, durable journals, exact returned IDs, external edits, and rollback recovery.
- [x] Implement the forward executor and rollback state machine. Use a transport contract and isolated mock router.
- [x] Review connected code and adversarial tests. Run format, locked tests, and strict Clippy.
- [x] Document the engine boundary and production adapter requirements. Capture the result and commit/push after successful verification.

## Review focus

- A write can succeed while its response is lost. Preserve an unknown outcome instead of repeating the request.
- A crash can occur after a write but before its journal result. Preserve the pending intent for manual reconciliation.
- A foreign edit during rollback must block destructive compensation.
- Restore broad entries before removal of additions so rollback does not create a membership gap.
- Competing external writers and configuration/session evidence require real integration checks. The mock does not prove production isolation.

## Execution record

- Ruling: implement the isolated engine with an injected transport. Production transport, authentication, writer fencing, and candidate bootstrap remain a separate integration stage.
- Tests failed before the initial module existed. Adversarial tests also reproduced overlap, row-ID scope, and corrupt journal progress defects before correction.
- Fresh review reproduced expiry during an awaited observation and invalid marker identities. Both regression tests passed after correction.
- The final request guard checks approval and observation age after durable intent persistence. It clears only intents for requests never sent.
- Follow-up review found no remaining actionable issue. No production write or live process change occurred.
