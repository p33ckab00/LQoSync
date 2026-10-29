---
type: session
created: 2026-10-30
tags: [lqosync-v3, topology, assignment, sessions, performance]
status: complete
related: ["[[HANDOFF]]", "[[20-decisions/adr-0001-session-candidate-state]]", "[[2026-10-30-noc-console]]"]
---

# Flexible topology and reconnect assignment

The user authorized commit/push of the NOC console, implementation of the behavior changes, review, and a final push after successful checks.
The NOC console and prior project documentation reached remote commit `df120ed` on branch `lqosyncv3`.
Checkout: `/opt/LQoSync3.0`. This behavior change uses `df120ed` as its source baseline.

## Behavior

Independent selectors support profiles, servers, individual accounts, and DHCP MAC identities.
Individual selectors override group selectors. Exclusions omit records without deleting RouterOS configuration.
The operator can bind an existing destination or create a branch from discovered inventory.
Named groups support hierarchy and categorization without fixed service-only branches.

Automatic pools default to observed client count. Continuous sessions keep compatible assignments, and pins take priority.
Healthy absence, changed session handles, and reset uptime allow reconnect reassignment.
Explicit `planned_load` mode preserves the earlier weighted assignment policy.
Old settings without selectors and old committed assignment files remain readable.

Candidate observations survive healthy empty scans and blocked publication. Committed output remains unchanged until all gates pass.
Live Validation, candidate pin controls, core planning, and output publication use the same assignment planner.
Assignments labels candidate and committed views separately. Topology labels saved observations as candidate data.
See [[20-decisions/adr-0001-session-candidate-state]] and `v3/FLEXIBLE_TOPOLOGY.md`.

## Review corrections

Independent review found exclusions that incorrectly required vanished records and DHCP absence that failed before session planning.
Corrections allow harmless exclusions and missing dynamic leases while preserving health, duplicate, and present-record validation.
Follow-up regressions protect active excluded records and malformed nonexcluded DHCP identities.

The review also found committed-only pin controls and whole-second timestamps used as unique collection epochs.
Candidate controls now validate fresh ownership, tokens, and snapshot guards before saving.
Read tickets and inventory identity establish ordering. Same-second observations remain valid.
The final source review reported no material issue.

## Verification

Final `cargo test --locked`: 177 passed, zero failed, 11 manual benchmarks ignored.
`cargo fmt --check`, strict all-target Clippy, JavaScript syntax, and diff whitespace checks passed.
Four relevant ignored workloads ran separately and passed with 50,000 clients or selectors.
Browser checks passed on all 12 console routes at widths 1440px, 390px, and 320px.
The browser also saved and restored a custom group's parent and checked source controls and metric selection.
Only temporary synthetic application data supported those browser changes.

Debug measurements: 50k individual selectors collected in 1.606 seconds.
The earlier shared 50k/100-profile workload took 665 ms for normalization and 1.291 seconds through compilation.
The earlier shared 50k/8-WAN workload took 559 ms for new assignments and 232 ms for retained assignments.
These results are close to the prior recorded debug baseline. They are single-run measurements, not a latency guarantee.

The new 50k session workload passed count distribution, continuity, persistence, reload, and output compilation.
In a concurrent run, collection took 683 ms, assignment 1.022 seconds, and retention 1.186 seconds.
Candidate save took 7.890 seconds and load took 2.854 seconds. Durable I/O varied under concurrent checks.
A standalone repeat took 678 ms for collection, 1.003 seconds for assignment, and 1.221 seconds for retention.
Its candidate save took 1.059 seconds and load took 2.731 seconds.
The publication path now reuses validated file bytes and avoids a second parse while preserving a revision check.

## Limits and next action

DHCP observations represent bound leases, not physical device connectivity.
Polling cannot establish a reconnect without absence, a changed handle, or reset uptime.
New settings with selector or metric fields require a matching settings backup for rollback to an older binary.
The Graphify map predates this change. Confirm new relationships in the source before relying on that map.

No private credentials or production inventory were read. No live settings, router configuration, LibreQoS output, or package changed.
The live console remains on its prior process. No live service restart or shaping apply occurred.
Next: activate the verified console only with task authorization, then review protected manual membership before any future router writer.
See [[HANDOFF]].
