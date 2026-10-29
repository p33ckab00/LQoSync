---
type: session
created: 2026-10-29
tags: [lqosync-v3, core, address-lists, routing, integrity, performance]
status: complete
---

# Core address-list planner and route checks

The user authorized implementation, performance/integrity review, commit, and push.
Worktree: `/opt/LQoSync3.0`. Branch: `lqosyncv3`. Base: `7c7b68c`.
All changes remain inside the LQoSync project.
No router writes, production credentials, LibreQoS changes, package changes, or service restarts occurred.

The Core lists UI binds each WAN to its mapped core/both router, a positive list name, and a custom FIB table.
One WAN has one binding. Lists and tables are unique per core.
Every active subscriber WAN needs a binding after the planner is enabled.
Form revisions prevent old forms from overwriting newer settings or generations.

The planner uses the same normalized subscribers and assignments as the local shaping generation.
It creates one desired IPv4 `/32` host per active subscriber.
Exact application/subscriber/WAN markers establish the local ownership scope.
Only static, enabled, permanent owned hosts in current configured lists can appear as removals.
Manual and other-owner rows remain protected. Retired lists remain untouched.
Manual host/CIDR/range overlaps, DNS entries, malformed markers, duplicate ownership, and dynamic owned rows block the plan.

Core/both inventory now reads routing tables, routing rules, address lists, mangle, and filter tables.
Full match fields preserve unknown conditions for conservative rejection.
The initial audit supports RouterOS 7 and direct prerouting list-to-table marks with `passthrough=false`.
It checks FIB state and one resolved active default through the mapped WAN.
Complex matchers, jumps, routing rules, FastTrack, ECMP, and ambiguous defaults require review.
These checks do not prove the packet path. NAT, VRFs, destination exceptions, and connection state need isolated validation.

The publisher commits `core-plan.json` as a fifth immutable generation file before the pointer switches.
The loader validates active subscriber/IP/parent consistency, ownership, binding scope, and inventory provenance.
Older generations remain readable. Missing or corrupt declared plans block managed publication.
Failed, partial, stale, or invalid inputs keep the entire last valid generation.
Source and core timestamps both form part of the plan.

Independent review found one material gap: an older distro read could expire while a fresh core read remained valid.
The regression failed before the fix and passed after it.
The publisher now checks all source/core timestamps after artifact writes, immediately before the pointer switch.
The review found no other concrete ownership, route-audit, or scaling issue.

Verification: 81 functional tests passed. Four manual benchmarks are ignored by default.
Formatting, strict Clippy, build, and diff checks passed.
An actual HTTP smoke checked four routes with isolated empty application data.
The temporary smoke process terminated and its temporary data was removed.
Existing development and production processes were not restarted.

The new benchmark passed separately: 50,000 subscribers and 60,000 existing list members.
Final debug results: plan build 876 ms, integrity check 211 ms, compact JSON serialization 791 ms.
The output was 16,219,265 bytes. Network collection and disk publication are excluded.
The workload checks 25k adds, 25k keeps, 10k owned removals, and 25k protected rows.
Indexes avoid a full subscriber scan per list member. Route checks run once per core.

Changed modules: `core_plan`, `core_routes`, `core_web`, `model`, `routeros`, `store`, `orchestrator`, `web`, and `lib`.
Tests, guides, the plan, and these notes accompany the code.
See `v3/CORE_PLANNER.md`, [[../Home]], and [[../HANDOFF]].
Next: validate representative isolated router policies, then design coordinated execution with ownership rechecks and actual-result verification.
Router writes, production shaping apply, authentication, generation retention, and broader route-policy support remain future work.
