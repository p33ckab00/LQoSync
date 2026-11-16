---
type: session
created: 2026-11-16
tags: [telemetry, compatibility, implementation]
status: complete-local
related: ["[[2026-11-16-dataplane-update-intelligence-review]]", "[[HANDOFF]]"]
---

# Explicit native slot mapping

Completed: 2026-11-16T10:04:21+08:00 (Asia/Shanghai). Agent: Codex `/root`.
Branch/worktree: `lqosync3`, `/home/pi/LQoSync3.0`.
The owner authorized Task 1 implementation and performance, integrity, and reliability review.
The approved sequence includes a local commit. Push and deployment remain outside this task.

## Changes

The shared applied-state reader decodes optional nested-CBOR `slot_numbers`.
It pairs explicit IDs with names in a BTreeMap.
Only an absent field permits the legacy one-based mapping.
Malformed fields, duplicate or zero IDs, mismatched lengths, and trailing data cause rejection.
Subscriber metrics use the ID lookup. Circuit queue requests use the selected circuit's explicit ID.
Generation and before/after mapping checks remain in place.
The source probe follows the same mapping and reports `applied_slot_matches`.

## Review and checks

Sparse-host, malformed-field, and queue-ID regressions failed before the fix and passed afterward.
Fixtures cover sparse IDs, ID swaps without a generation change, legacy reports, wire reordering, large IDs, and size bounds.
A maximum-size test accepts 65536 sparse mappings and rejects a larger name list.
Host lookup takes O(log n). Map construction takes O(n log n), with O(n) storage.
No new source requests, polling, render work, dependencies, cache schema, or configuration keys were added.
No unresolved blocking finding remained after the code review.

Fresh full checks: 704 passed, zero failed, nine ignored across 19 suites.
Clippy with warnings denied, formatting, whitespace checks, and affected probe builds passed.
The earlier live metadata confirms the installed field format. The changed reader has no live-socket verification yet.
No production install, configuration change, restart, or shaping write occurred.
Evidence: `/home/pi/.local/state/lqosync3/explicit-slots-task1/`.

## Next task

Diagnose the post-reboot native policy proof with read-only evidence.
Do not claim that this mapping change fixes `import_confirmed_ack_unproven` or the deferred capacity mismatch.
Plan: `docs/superpowers/plans/2026-11-16-explicit-slot-mapping.md`.
Operations: `docs/operations/EXPLICIT_SLOT_MAPPING.md`.
Preserve unrelated profile/WAN notes and earlier local release/research documentation.
