---
type: session
created: 2026-10-29
tags: [lqosync-v3, routing, minimums, validation, performance]
status: complete
---

# Scoped routing and minimum allocation

The user authorized support review for seven routing findings and automatic proportional minimum adjustment.
Keep configured floors, capped by subscriber maxima. Fail when floors cannot fit. Keep router maxima unchanged.
Branch/worktree: `lqosyncv3`, `/opt/LQoSync3.0`. Base: `1a51b9e`.
No router or LibreQoS changes are authorized. Commit and push authorization continues.

The new scoped analyzer reads interface-list definitions and members.
It supports static ingress lists with paired earlier destination-list bypasses and exact OSPF/UDP control exclusions.
It rejects composition, unknown flags, mutable destinations, full IPv4 exclusion unions, unsupported conditions, and wrong order.
The Core review UI states the scope limits. It does not claim subscriber ingress or complete packet-path proof.

Minimum allocation follows WAN assignment. It reserves floors and scales remaining headroom under every ancestor, separately per direction.
It uses deterministic contiguous subtree ranges. Failed allocation leaves caller records unchanged.
Publication and Validation use the same effective minimums. The compiler also checks aggregate admission.
Source-managed topology edits validate an allocated temporary copy before normal refresh/publication.

Tests reproduced the original unsupported valid scope and aggregate minimum admission gap.
Additional regressions reproduced an empty scope hidden by an exclusion union and an exact-floor rounding failure.
Independent review found active list writers, a topology-edit caller gap, and quadratic scope lookups.
Failing regressions reproduced the writer and topology failures. The fixes reject writers and allocate a temporary snapshot copy.
The scope analyzer now indexes definitions, members, interfaces, and addresses. A candidate limit precedes expensive checks.
A regression requires the explicit candidate-budget finding even for rejected candidates.
A final numerical regression removes an absolute tolerance bypass for very small positive capacities.
All review regressions passed after their fixes. Scope lists and UI output remain explicitly bounded.

Fresh independent core capture completed 12 tables with zero errors.
The actual sanitized fixture preserves two forwarding scopes and two router-reply mark links.
With explicit bindings, the actual route analyzer has zero findings.
The initial candidate had 25 subscribers. A later fresh read has 26, with 12/14 WAN coverage.
The new candidate has approximately 320 Mbps minimum totals in each direction on each WAN.
All 26 subscriber maximums remain unchanged. The base fraction and configured floor remain unchanged.
Core planning still blocks on protected manual address-list entries.
All 26 candidates overlap existing protected members. Seventeen proposed WANs differ from current membership evidence.
Do not adopt, remove, overwrite, or bypass these manual entries.

The full suite exposed two old pagination fixtures that compiled overloaded synthetic minima directly.
Those fixtures now use the allocation step. Large pipeline workloads also use the complete allocation sequence.
Final verification: 127 functional tests passed, zero failed, nine manual benchmarks ignored by default.
Formatting, strict Clippy, binary build, and diff checks passed.
Independent review reported no additional material issue beyond the three corrected defects.

Final debug benchmarks: 50,000-subscriber allocation, 288.4 ms. Indexed 5,001-member scope review, 33.1 ms.
The pre-index scope result was 2.515 seconds. Measurements exclude network collection and disk publication.
The 50k/8-WAN assignment workload passed: new 536.0 ms, retained 230.6 ms.
The 50k/100-profile workload passed: normalization 643.4 ms, including allocation and compilation 1.271 seconds.
The initial 1,000-WAN /100k-route review also passed at 32.0 ms debug.

The latest verified binary now serves the local LQoSync console.
Fresh verified UI reads completed 14 distro and 12 core tables without errors.
Eight UI routes passed. Live validation passes inventory, speeds, assignment, minimum allocation, and shaping capacity.
Core review reports both forwarding scopes with no route findings.
The overall end-to-end result remains Unavailable without a committed generation. Core membership is Conflict due to protected manual overlap.
No generation was published. Candidate assignments remain previews rather than committed sticky state or actual router routes.
The earlier live verification expected a fixed roster of 25; a new active client correctly changed that count to 26.
The final check uses matching current candidate/UI counts instead of a fixed client count.

The refreshed CSV/JSON pair has 26 unique identities/IPs, valid parents, and directional ancestor caps.
The installed v3 preflight validated its topology and at least 26 circuits, pre-install exit 0.
Landlock denied filesystem mutations in the diagnostic child. This does not establish packet-path or runtime readiness.
Router settings and confirmed CA paths stayed unchanged. Only the LQoSync process was restarted.
LibreQoS remains version `2.2.202608181454`, held. No router configuration or LibreQoS files/packages/services/shaping changed.
Private reports and candidate files remain under ignored application data with restrictive permissions.

Changed files include core_scopes, minimums, core_analysis, core_diagnostics, core_review_web, core_plan, routeros, model, orchestrator, validation, web, speed_web, and policy_fixture.
New and updated tests, guides, sanitized fixture, implementation plan, and vault notes accompany the code.
See `docs/superpowers/plans/2026-10-29-scoped-routing-minimums.md`, `v3/SCOPED_ROUTING.md`, and `v3/SPEED_RULES.md`.
Next: review protected manual membership and the intended per-host migration without changing router rules or bypassing ownership.
Commit and push of the verified code and sanitized notes are authorized. Private configuration and subscriber data must stay outside Git.
