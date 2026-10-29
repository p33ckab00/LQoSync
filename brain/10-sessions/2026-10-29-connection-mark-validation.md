---
type: session
created: 2026-10-29
tags: [lqosync-v3, core, routing, validation, integrity, performance]
status: complete
---

# Connection-mark and output validation

The user authorized read-only connection-mark validation and subscriber/output checks.
The user chose validator completion first. Actual topology, capacities, and source/core mappings will be configured through the UI later.
Checkout: `/opt/LQoSync3.0`. Branch: `lqosyncv3`. Base: `3b99e33`.
Commit/push authorization continues. No router configuration or LibreQoS changes are authorized.

Actual core inspection corrected the proposed chain model.
Two active connection marks classify WAN ingress for router replies in the output chain.
Subscriber forwarding uses direct source-list routing with interface-list and destination-list conditions.
The validator keeps these paths separate. It does not invent a subscriber connection-mark chain.

The new indexed analyzer supports two narrow forms.
A forward classifier marks new unmarked connections from a positive source list.
A later prerouting consumer uses the same source list and mark for its configured WAN table.
A reply classifier marks new unmarked WAN ingress and links to output routing through the same WAN table/interface.
Reply paths do not count as subscriber list-routing rules.
Unknown conditions, mark ambiguity, wrong chains, invalid flags, reversed forward order, implicit mark clearing, and unmodeled output actions remain blocked.
Global policy findings, default route checks, freshness, and planner failure gates remain active.
Connection-path caps produce explicit global findings. Mark-link status is separate from overall WAN status.

Core review now displays direction, mark, source list, table/interface, rule references, and mark-link status.
The new Validation page normalizes captured inputs, assigns subscribers, compiles shaping output, and builds the core list plan without publication.
It reads one committed generation and compares snapshot, assignments, CSV, JSON, and core plan with current inputs.
Each artifact has a 64 MiB review limit. Stale settings, reads, inventory identities, errors, and generation changes invalidate UI work.
Saved evidence remains historical and cannot establish live success or enter active inventory.
The CLI supports `--end-to-end`, saves a private local report, and requires Pass for exit code zero.
Unknown CLI flags fail before settings or router access.

The actual fixture preserves reserved `no-mark` and known connection-state tokens.
Projection checks both finding counts and connection-path directions/statuses.
A fresh verified core read completed 11 tables with zero errors.
Both actual router-reply links passed their narrow mark-link checks.
Overall findings remain: three complex-prerouting, four conditional-matcher, and one mapping-required.
Active PCC and FastTrack counts remain zero. Existing disabled entries remain disabled.
Actual end-to-end review reports missing generation, source/topology setup, and core mappings. Expected exit code is nonzero.
No actual subscriber/WAN assignment or packet path has been proven.

Tests first reproduced unsupported valid paths, hidden output/mark-clear hazards, and the missing validation route.
Independent review found two defects: reply classifiers accepted `!local`, and failed end-to-end CLI results returned zero.
Both regressions failed before their fixes, then passed.
Reply classifiers now require unrestricted or local destination type. End-to-end CLI requires a passing outcome after report creation.
No additional material issue was reported by the review.

The synthetic benchmark uses 500 marked WANs and 100,000 routes.
Final debug result: 28.8 ms, excluding collection and disk I/O.

Verification: 111 functional tests passed; seven manual benchmarks are ignored by default.
The connection-mark benchmark passed separately. Formatting, strict Clippy, binary build, and diff checks passed.
Four isolated HTTP routes passed. The latest verified binary now serves the LQoSync console.
Fresh UI collection completed 14 distro tables and 11 core tables with zero errors.
Validation and Core review render the new results. Settings, confirmed CA paths, and the generation pointer remained unchanged.
Actual end-to-end validation remains Unavailable because the user deferred topology and mappings to the UI.
Only the LQoSync web process was restarted. No router configuration or LibreQoS file, package, service, or shaping change occurred.
Changed modules: core_connections, core_analysis, core_routes, core_diagnostics, core_review_web, validation, validation_web, store, core-validate, web, lib, and policy_fixture.
Tests, fixture provenance, guides, the plan, and project notes accompany the code.
See `v3/VALIDATION.md`, [[2026-10-29-actual-core-validation]], [[2026-10-29-dhcp-server-speeds]], [[../Home]], and [[../HANDOFF]].

Next: configure actual WAN/source/core bindings in the UI and review remaining conditional/bypass policies.
Router writes, coordinated shaping apply, full packet-path validation, authentication, and retention remain future work.
