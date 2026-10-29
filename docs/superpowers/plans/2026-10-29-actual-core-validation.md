# Actual Core Policy Validation Plan

**Goal:** Read actual MikroTik core policy, report all relevant WAN findings, and test sanitized policy fixtures before a writer exists.
**Architecture:** Reuse RouterOS print inventory, add private app-local TOML credentials, and share one indexed route analyzer between planning and diagnostics.
Expose structural pass/conflict/unsupported results separately from inventory health and saved-history status.
**Scope:** `/opt/LQoSync3.0`, branch `lqosyncv3`, base `7141844`.

## Constraints

- No router writes, LibreQoS changes, package changes, or existing process restarts.
- Credentials stay in ignored `v3/data/credentials.toml`, mode 0600. Do not log, render, or commit passwords.
- Actual router reads use existing configured TLS verification and the read-only print allowlist.
- Incomplete or saved inventory cannot authorize publication. Last valid generations remain selected.
- Diagnostics and the planner share the same conservative route checks. Do not relax unsupported policies into passes.
- Each WAN receives all scoped findings. Global core findings affect every WAN without duplicating large finding lists.
- Observed paths remain separate from configured WAN mappings. Do not create shaping nodes or change policy automatically.
- Raw actual inventory and local reports stay under application data. Public fixtures remove private names, addresses, comments, and credentials.
- Findings are bounded. Truncation must remain visible and cannot produce a pass.
- Run expensive work outside async workers and application locks. Index routes and rules once per core.

## Work

- [x] Test and add TOML credential loading. Preserve existing environment references and redact all credential errors.
- [x] Collect actual core inventory with configured read-only access. Record collection limits honestly.
- [x] Add typed, indexed, per-WAN multi-finding route diagnostics and retain planner failure gates.
- [x] Add live/saved diagnostics to the UI, with explicit freshness and provenance labels.
- [x] Derive sanitized representative fixtures from observed policy and check replay fidelity.
- [x] Test stale/partial inputs, global/scoped findings, safe rendering, credential isolation, and planner preservation.
- [x] Measure a large multi-WAN policy workload and run independent performance/integrity review.
- [x] Run full tests, formatting, strict Clippy, and HTTP checks. Update notes, commit, and push.

## Ledger

Recall: existing planner passes only RouterOS 7 direct-list policies. It reports the first route error.
No saved core inventory existed initially. User selected TOML credentials and confirmed local credential entry is ready.
No graph index exists. The initial checkout was clean and already used the authorized feature branch.


Validation completed: 95 functional tests, strict Clippy, formatting, diff checks, and four-route isolated HTTP smoke passed.
The new benchmark passed separately: 1,000 WANs /100k routes /50k list rows, 28.1 ms debug review, excluding collection and disk I/O.
Verified actual core reads completed 11 tables with zero errors on RouterOS 7.25.
Two observed paths remain unsupported, with no configured app bindings. Disabled PCC/FastTrack entries have zero active counts.
The sanitized projection reproduces five complex-prerouting, four conditional-matcher, and one mapping-required findings.
Independent review found silent observed-path truncation. A failing regression preceded the explicit omitted-count fix.
Queued-worker guards, specific FIB switch compatibility, and unknown filter actions have focused regression coverage.
Git commit and remote-head verification form the final publication action after these checks.
