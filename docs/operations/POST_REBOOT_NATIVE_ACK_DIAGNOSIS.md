# Post-reboot native ACK diagnosis

Reviewed: 2026-11-16T10:18:38+08:00 (Asia/Shanghai).
Scope: read-only diagnosis of `import_confirmed_ack_unproven` and verifier review.

## Verified condition

The operator ran the bounded native probe after the 2026-11-16 host reboot.
Three samples show stable artifacts and stable native reports.
The current pair files match the imported topology.
The runtime is ready, and its generation matches the generated transaction.
Both native and generated transactions use schema 6.

The generated world has 28 leaf operations and 28 mapping operations.
The native applied world has 27 of each, with a different generation.
The current-policy probe fails with `native generation still pending`.
The native report contains a content key but no desired-world content key.
An absent desired key alone is not the verifier's failure condition.
This is an observed policy difference, separate from the deferred 64-versus-1000 capacity mismatch.
Do not interpret a count difference as proof that exactly one named subscriber lacks shaping.
The operation identities and policies can differ in more ways than their net count.

## Upstream hold

The bridge startup journal reports a structural rebuild from the transaction and a persisted boot world with 27 slots.
The lqosd journal repeatedly reports `NON_SUFFIX_STRUCTURAL_REMOVE` afterward.
Its warning states that the removal set is not a contiguous suffix of the bridge slot table.
The broker emits the removable tail and fences the remainder.
The public broker source documents this guard and its structural identity fence.
This explains an observed upstream update hold consistent with the old applied generation.
The full reason that the installed rebuild or recovery path has not completed remains unproven.

The public source is a beta19 projection. It is not proof that every internal branch matches the installed binary.
Do not remove the verifier's generation comparison or accept the shorter policy.
Do not force subscriber retention or rewrite the managed files to avoid the upstream remove operation.
A verified broker/bridge recovery requires separate upstream work or an operator-reviewed recovery procedure.
No recovery operation occurred in this task.

The startup journal also records a Node Manager address-bind failure and a temporary DNS failure.
These are separate boot observations. Their current state and causal role in the ACK hold were not established.

## Code review

Performance: the existing monitor retains its two-second interval with missed ticks skipped.
It caches prepared input normalization and imported-file identity.
A pending native generation returns before the heavier artifact and producer validation stages.
This task adds no production loop, source request, allocation path, or dependency.

Integrity: the verifier correctly rejects the different generation and incomplete native world.
It retains input hashes, stable native reports, producer path validation, exact operation comparison, and receipt identity checks.
The capacity mismatch remains independent evidence. No guard was relaxed.

Reliability: the receipt stays unproven until sufficient native evidence exists.
The new regression models a 28-entry generated world and a 27-entry boot world.
It rejects the old generation, then rejects incomplete policy even after generation strings match.
Only a complete matching native policy can produce an accepted receipt in the fixture.
No demonstrated LQoSync3 production defect requires a runtime patch for this incident.
The operational blocker remains upstream, so this is not a successful native recovery claim.

## Verification and evidence

Focused evidence tests and the new regression passed.
Full suite: 705 tests passed, zero failed, nine ignored across 19 suites.
The focused regression passed after an assertion-style correction.
Final Clippy with warnings denied, formatting, and diff checks passed.
No package installation, production file write, config change, restart, or Telegram send occurred.
All test data uses synthetic circuit names and documentation addresses.

Evidence: `/home/pi/.local/state/lqosync3/post-reboot-ack-task2/`.
`live-metadata.json` holds the operator-run native comparisons.
`journal-findings.json` contains event counts and timestamps without subscriber identifiers.
The native socket requires root access, but the current user can read the service journals.
Some journal messages use byte arrays. Decode those arrays before matching diagnostic markers.

Sources:

- [Public broker structural-removal guard](https://github.com/LibreQoE/LibreQoS/blob/9ebe29bc9d49013aff9b0544de3c6f6fc6ba2f1d/src/rust/lqosd/src/bridge_broker/mod.rs).
- [Public topology custody verification](https://github.com/LibreQoE/LibreQoS/blob/9ebe29bc9d49013aff9b0544de3c6f6fc6ba2f1d/src/rust/lqos_topology/src/publish.rs).
