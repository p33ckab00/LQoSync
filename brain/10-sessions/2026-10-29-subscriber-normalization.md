---
type: session
created: 2026-10-29
tags: [lqosync-v3, subscribers, sources, integrity, performance]
status: complete
---

# Subscriber normalization and source selection

The user authorized implementation, performance/integrity review, commit, and push.
Worktree: `/opt/LQoSync3.0`, branch `lqosyncv3`, base `7623e31`.
No production LibreQoS or router changes were made.

The Sources UI binds discovered PPP profiles, DHCP servers, and Hotspot profiles to existing shaping nodes.
Node router/source fields hold these exact bindings. Duplicate bindings are rejected.
Active DHCP server/MAC/address fields take precedence over static lease fields.
Unknown mappings are blocked, rather than assumed to be unselected.
Account and source names remain exact, including spaces.

UUID v5 IDs exclude IP, profile, hostname, and node.
DHCP uses canonical MAC identity. PPP and Hotspot use exact account identity, scoped by router and service.
Same-router DHCP/Hotspot overlap uses Hotspot only when IP and MAC match.
Other IP or identity collisions block publication. Multiple IPs per identity remain unsupported.

Healthy selected inventories trigger normalization automatically after source, policy, or inventory changes.
Scheduled rounds read only selected routers, with at most four concurrent reads.
Pause stops queued reads and future publication. Epochs reject superseded read results.
Required tables, selected metadata, fresh timestamps, rates, and identity checks gate publication.
A whole empty scan is blocked. Failed/partial scans retain the last valid snapshot and generation.
Unused computed branches are omitted from nonempty output and return when subscribers return.

Each managed generation contains the snapshot, CSV, and network JSON before one pointer changes.
The committed generation is authoritative. The state snapshot is a compatibility mirror.
Mirror failures are visible but do not hide the committed snapshot.
Corrupt committed snapshots produce errors instead of silent fallback.
Clearing all bindings retains managed state and cannot republish it as legacy offline data.

Topology aggregation visits subscriber ancestors once. Profiles and source tables use indexes.
Heavy work and snapshot reads run in blocking workers outside the application lock.
The serialized commit uses a blocking worker while it holds the application guard.

Final checks: 59 functional tests passed, strict Clippy, formatting, and diff checks passed.
Two manual performance tests are excluded from the standard suite; the new benchmark ran separately and passed.
50,000 subscribers / 100 sources: normalization 639.2 ms; normalization plus compilation 921.7 ms, debug build.
This excludes router network time and disk publication.
Tests include source HTTP token checks, automatic mock-router publication, pause, stale work, mirror failure, and retained generations.
Independent review findings were fixed and rechecked; no material finding remains.

See [[../HANDOFF]] and `v3/SUBSCRIBERS.md` for operation and limits.
Next: isolated representative router validation, WAN assignment, effective RADIUS/IPv6/multi-device support, authentication, and retention.
Production export/apply remains outside this slice.
