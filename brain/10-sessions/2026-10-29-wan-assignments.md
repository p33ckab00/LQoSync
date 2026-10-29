---
type: session
created: 2026-10-29
tags: [lqosync-v3, wan, assignments, integrity, performance]
status: complete
---

# WAN mapping and subscriber assignments

The user authorized code, performance/integrity review, commit, and push.
Worktree: `/opt/LQoSync3.0`. Branch: `lqosyncv3`. Base: `a156ec7`.
The work changes only the LQoSync application and project notes.
No production credentials, router writes, LibreQoS writes, package changes, or service restarts occurred.

The WANs page maps fresh discovered interfaces to explicit WAN roots.
The existing root download/upload values remain the WAN capacities.
One router/interface pair can map to one root only.
The user selects the actual WAN. Discovery alone does not prove the traffic path.
Mapped core routers join collection with the existing four-router limit and pause behavior.
Failed, stale, duplicate, down, disabled, or unknown interface metadata blocks normalized publication.
The last valid generation remains selected. No failover or mass migration occurs.

The Assignments page adds source destination pools, assignment modes, and subscriber pins.
Compatible targets have the same subscriber kind and cannot own another source binding.
The original source remains the collector binding. The target becomes the exported subscriber parent.
Pins override previous assignments. Valid previous assignments override balance mode.
Optional balancing selects destinations for new/unassigned subscribers only.
The score uses projected plan load divided by directional WAN and explicit destination capacities.
This is not measured traffic, admission control, or actual router steering.

The algorithm counts retained/pinned/default loads before it distributes new balanced subscribers.
Sorted subscriber IDs and target IDs make results deterministic.
Subscriber rates, minima, IDs, and IPs remain unchanged.
Inactive ownership survives nonempty scans, restart, and capacity changes without contributing plan load.
Policy removal prunes inactive records that no longer have an eligible source or destination.

Each normalized generation now commits four files before its pointer switches:
`subscribers.toml`, `assignments.toml`, `ShapedDevices.csv`, and `network.json`.
The loader checks active subscriber ownership and parent consistency.
Missing or corrupt declared assignment state blocks publication without a silent reset.
Older normalized generations derive initial source ownership from their subscriber parents.
Settings and runtime state remain inside the application directory.
Settings and generation comparisons reject stale UI edits and calculations.

Independent review found two defects: offline assignment loss and search loss during pagination.
Regression tests failed before the fixes and passed after them.
Follow-up review found no remaining material integrity or performance issue.
Final suite: 68 functional tests passed. Three manual benchmarks are ignored by default.
The new assignment benchmark passed separately: 50,000 subscribers / eight WANs.
Debug results: 525 ms for new assignments and 229 ms for retained assignments.
The benchmark excludes collection and disk publication.
Formatting, strict Clippy, and diff checks passed.
HTTP route tests cover token rejection, invalid-edit preservation, automatic publication, pins, and encoded search pagination.
Mock RouterOS tests cover mapped core collection and WAN failure retention.

See `v3/WAN_ASSIGNMENTS.md`, [[../Home]], and [[../HANDOFF]].
Next: validate representative isolated routers and design core list ownership with route verification.
Core address-list writes and production LibreQoS apply remain pending.
The UI source-pool view currently displays the first 200 bound sources.
Authentication, generation retention, RADIUS, IPv6, and multi-device support remain future work.
