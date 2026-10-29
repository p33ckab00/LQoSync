# Connection-mark validation implementation plan

**Goal:** Review supported connection-mark paths and run a read-only subscriber/output consistency check.
**Architecture:** Add one indexed connection-mark analyzer to the shared route validator. Reuse normalization, assignment, compilation, core planning, and immutable generation loading for end-to-end review.
**Tech stack:** Existing Rust, Axum, Maud, serde, and RouterOS print client. No new dependency.
**Scope:** `/opt/LQoSync3.0`, branch `lqosyncv3`, base `3b99e33`.

## Design and constraints

The actual core uses WAN ingress connection marks for router replies in `output`.
Subscriber forwarding uses direct source-list marks with interface-list and destination-list conditions.
Do not invent a subscriber connection-mark chain or silently bypass existing conditional findings.
The user chose validator completion first, with actual WAN/source/core configuration through the UI later.
Missing topology and bindings must appear as setup blockers, never as a pass.

Support two narrow connection-mark forms:

- Forward: one positive source-list classifier for new unmarked connections, followed by one same-list prerouting routing consumer.
- Router reply: one new/unmarked WAN-interface classifier and one output consumer targeting the same WAN table.

Require unique mark producers and consumers, known flags, exact bindings, correct chains, classifier passthrough, and terminal routing marks.
Forward classifiers must precede their consumers. Unsupported matchers and ambiguous producers remain blocked.
Router reply paths do not count as subscriber list routing rules.
Return an explicit direction, rule references, mark, table, interface, and link status for each bounded trace.
Keep global blockers, inventory freshness, saved/live provenance, and planner gates.
Do not relax active PCC, FastTrack, early bypass rules, arbitrary jumps, or unknown conditions.
No packet-path simulation or production-readiness claim is part of this validator.

End-to-end review uses current settings and captured inventories to normalize subscribers, assign branches/WANs, compile CSV/JSON, and build the core list plan.
Read a committed generation through one pointer decision. Validate its snapshot, assignments, core plan, and artifact contents against the candidate.
Report missing generations, stale data, speed/parent mismatches, changed output, and incomplete core policy explicitly.
A GET review never publishes or changes active inventory. Saved inputs remain historical.
Run expensive review outside the application lock. Reject results superseded by settings, reads, or generation changes.
All local reports and raw policy remain in ignored app data. Public fixtures use aliases and preserve only required semantics.
No router writes, LibreQoS writes, package changes, production apply, or unrelated process changes.

## Work

- [x] Add failing tests for safe forward/reply paths and adversarial connection-mark changes.
- [x] Add indexed trace analysis and integrate it without weakening shared planner failure gates.
- [x] Preserve `no-mark` and known connection-state semantics in sanitized fixtures. Check actual replay fidelity.
- [x] Add failing end-to-end tests, one-pointer generation reads, and read-only Validation UI/CLI reports.
- [x] Test UI provenance, stale-result guards, missing setup, corrupt output, and generation retention.
- [x] Measure representative large connection-mark and subscriber workloads. Run independent integrity/performance review.
- [x] Complete full tests, formatting, strict Clippy, build, HTTP/live reads, and notes. Prepare the verified change for authorized commit and push.

## Review focus

- Duplicate or hidden mark producers, output-only marks, mismatched tables/interfaces, and reversed forward rule order cannot produce a subscriber pass.
- Conditional or malformed classifier fields cannot be discarded by a trace join.
- Trace/finding caps remain visible and never authorize incomplete policy.
- A pointer switch cannot combine generation files. Historical inventory cannot become live validation evidence.
- Corrupt or outdated CSV/JSON, missing topology, and changed speeds or parents fail end-to-end consistency.

## Ledger

Current branch is clean at `3b99e33`. Project notes and latest handoff were read.
A fresh verified core read returned 11 tables with zero errors on RouterOS 7.25.
Current app configuration has two routers, zero nodes, zero core bindings, and zero manual DHCP rules.
The user confirmed validator-first scope; no topology or capacity values will be guessed.
Official mangle and PCC documentation confirms separate prerouting and output routing stages.


The narrow mark-link and read-only generation checks passed focused tests.
The actual replay contains two router-reply links and no invented subscriber-forward link.
Actual overall findings remain three complex-prerouting, four conditional-matcher, and one mapping-required.
The 500-marked-WAN /100k-route benchmark passed at 28.9 ms debug, excluding collection and disk I/O.
Independent review found a reply `!local` false pass and a successful CLI exit on failed end-to-end checks.
Failing regressions reproduced both. Role-specific destination guards and strict end-to-end exit checks fixed them.
Final verification passed: 111 functional tests, formatting, strict Clippy, binary build, and isolated HTTP smoke.
The latest console passed live UI checks. Fresh distro/core reads returned 14/11 tables with zero errors.
Actual settings and generation remained unchanged. End-to-end setup is intentionally deferred to the UI.
Authorized Git publication is the final action.
