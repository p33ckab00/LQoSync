---
type: session
created: 2026-11-18
tags: [topology, enrollment, cli, identity]
status: done
related: ["[[topology-manager-status]]", "[[2026-11-17-topology-enrollment]]"]
---

# Topology Manager enrollment CLI (Phase 1)

The owner chose a CLI enrollment path, name and MAC identity rules, and an exact snapshot enrollment.
The owner stops the service in a maintenance window for the one-time enrollment.

## Owner decisions

- Identity: PPP secret name and DHCP client MAC. No `[lqosync-id:]` router marker.
- Enrollment adopts the current legacy rows only. `ipoevlan10` clients enter after Phase 4 activation.
- Suggested mappings need operator confirmation. Static rows are protected. Offline rows are dropped explicitly.
- G1: a whole-Mbps node export matches a legacy fractional value that equals the calculated ceiling.
- G2: per-site DHCP uses a plan sum when the router has no DHCP client overrides.

## Results

- Branch `topology-enrollment-cli`: Tasks 1 to 8 of [the plan](../../docs/superpowers/plans/2026-11-18-topology-enrollment-cli.md).
- 1,235 tests passed, 0 failed, 13 ignored. Format, Clippy, and whitespace checks passed.
- The production runbook needs `require_bound = true` and a temporary `ipoevlan10` exclusion.
- No live service, router, or LibreQoS file changed.

Related: [[topology-manager-status]].
