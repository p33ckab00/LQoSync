# Authoritative Static Input Implementation Plan

**Goal:** Bind independent operator-owned static rows and roots to one current source document.

**Architecture:** Read `<state_dir>/topology-manager/static-input.json` only when protected ownership exists.
The private document contains schema version 1, a positive revision, rows, and the compatibility network.
One atomic document prevents mixed CSV/network source generations. Never derive this document from generated output.

Use existing ownership, size, collision, endpoint, and compiler checks. Keep dynamic endpoints on the fresh-read path.
Bind the exact document bytes and revision through compilation and publication. Record the binding in the write receipt.
Reject a lower revision or changed bytes at the same revision against the current receipt.

**Skills:** writing-plans, test-driven-development, verification-before-completion, ponytail, brain, and ASD-STE100.
Execute this plan inline. Do not deploy or activate a service.

## Tasks

1. Add failing tests for current source loading, ownership preservation, changed/deleted sources, and unsafe files.

2. Add `static_input.rs`. Bound reads to the existing pair budget. Stop row decoding above 100,000 entries. Require a private regular file and private source directory.

3. Load the source before router reads in `coordinator.rs`. Merge it through existing `prepare_with_static`.

4. Bind the source in `PreparedPair`. Check its configured path and exact bytes before admission and publication.

5. Add an optional source binding to `TopologyContext`. Preserve old receipt decoding. Check revision continuity before registry commit.

6. Test missing sources before connection, changed sources after preparation, and protected ownership parity.

7. Run topology tests, the full test suite, format, Clippy, and whitespace checks. Run the installed compiler scratch test when available.

8. Update design and project memory with results and remaining activation gates.

## Acceptance and limits

Protected static input must match every selected static identity and independent root. Missing input blocks publication.
The collector can reuse unchanged input because it is explicit static policy. Dynamic subscriber endpoints never come from this document.

Every static policy edit must increase its revision. Replace the complete document atomically with mode 0600.
Source checks detect changes at admission and publication boundaries. They do not lock an external editor across output writes.

Shared managed/static branches, automatic enrollment, schedules, TUI, service activation, and native planner proof remain outside this slice.


## Result

The implementation and verification steps are complete locally. Commit `fa964c4` contains the prior node minimum slice. The push succeeded.
The new static-input slice remains uncommitted. No deployment or service activation occurred.

The full suite passed: 1,145 tests, zero failures, and 12 ignored tests. All 409 active topology tests passed.
Format, Clippy with warnings denied, and whitespace checks passed. The installed-library scratch compiler passed with a static root and node minimum.
Logs: `.superpowers/sdd/2026-11-17-topology-static-input/`. Final proof uses `full-verified.log`, `clippy-verified.log`, and `native-static.log`.

Author review found that compact empty rows could allocate excessive memory before ownership checks.
The decoder now stops above 100,000 rows. A focused test covers that case within the source byte limit.

The receipt regression test first failed because equal output reused the old source revision. It passes after the binding comparison fix.
No independent review or production benchmark occurred. The remaining scope limits still apply.
