---
type: session
created: 2026-10-29
tags: [lqosync-v3, core, routing, tls, integrity, performance]
status: complete
---

# Actual core policy review

The user authorized read-only validation of the actual core before any router writer.
The scope includes performance/integrity review, commit, and push.
Checkout: `/opt/LQoSync3.0`. Branch: `lqosyncv3`. Base: `7141844`.
No router configuration, LibreQoS files, packages, services, or existing processes changed.

The user chose private TOML credentials and completed the file locally.
The initial verified TLS read rejected a private CA chain before authentication.
The user independently confirmed the CA fingerprint through WinBox.
Only the app-local core CA path changed after that confirmation.
The verified core read completed 11 tables with zero table errors on RouterOS 7.25.
Raw inventory, credentials, CA files, and reports remain in ignored application data.
Do not copy private identifiers or credentials into this vault or Git.

The actual core has two observed custom-table paths. No WAN/list/table bindings are configured in this app.
Observed paths remain separate from configured WAN roots and assignments.
The review reports five `complex-prerouting`, four `conditional-matcher`, and one `mapping-required` findings.
PCC and FastTrack entries exist but are disabled. Active counts are zero.
Active connection-mark policy remains unsupported. These results do not authorize a writer or prove subscriber paths.

The shared indexed analyzer now returns multiple scoped findings.
Global findings affect every WAN. The planner still fails on any route finding.
Inventory freshness and provenance remain separate from structural status.
Saved review never enters live inventory or publishes a generation.
The Core review page runs analysis outside the async worker and rejects superseded live inputs.

RouterOS returned the FIB switch as a present field with an empty value.
A regression reproduced the false conflict before the specific FIB fix.
Only this presence-only switch accepts an empty value as enabled. Other boolean flags remain strict.
Unknown filter actions now produce Unsupported. Their regression failed before the fix and passed after it.

Independent review found a silent 512-path cap in observed diagnostics.
The 513-path regression failed before the fix and passed after it.
The report and UI now show omitted counts and a global `observed-path-limit` finding.
The finding cap also adds an explicit global result. Neither limit can produce a false pass.
Queued-worker tests verify changed settings and new read epochs reject old results.
A prior web assertion was updated for the new dual-source credential error text.

The sanitized actual-policy fixture preserves route finding-code counts and the FIB representation.
It removes private names, addresses, comments, and unrelated data.
It does not preserve address containment, subscriber membership, or complete packet behavior.
The exporter refuses a projection whose finding histogram changes.

Verification: 95 functional tests passed, with five manual benchmarks ignored by default.
The new manual benchmark passed separately: 1,000 WANs, 100,000 routes, and 50,000 list members in 28.1 ms (debug).
This measurement excludes collection and disk I/O. Reversed input order produces the same result.
Formatting, strict Clippy, and diff checks passed.
HTTP smoke passed four routes with isolated empty data. No generation was published.
The temporary smoke process and data were removed. Existing processes were not restarted.
A final verified core read repeated 11 complete tables and the same ten findings.
Changed modules: credentials, routeros, model, orchestrator, web, core_routes, core_analysis, core_diagnostics, core_review_web, policy_fixture, and core-validate.
Tests, fixture provenance, example credentials, guides, and the plan accompany the implementation.
See `v3/CORE_REVIEW.md`, [[2026-10-29-core-list-planner]], [[../Home]], and [[../HANDOFF]].

Next: explicitly map WAN/list/table relationships and validate supported routing behavior in an isolated router environment.
Connection-mark policies need a separate design and tests before any production writer.
Router writes, coordinated shaping apply, authentication, and generation retention remain future work.
