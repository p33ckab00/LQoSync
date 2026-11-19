---
type: knowledge
created: 2026-11-17
tags: [topology, status, architecture, implementation]
status: current
related: ["[[2026-11-17-topology-enrollment]]", "[[2026-11-17-topology-migration-staging]]"]
---

# Topology Manager feature status

Status checkpoint: `79ba5a2` on branch `lqosync3`. The commit is pushed to `origin/lqosync3`.
The feature has backend engines and controlled Runner APIs. The complete operator TUI and production activation are unfinished.
An implemented backend does not mean the feature is available in the TUI or active in production.

## Implemented backend

| Area | Current status | Evidence |
|---|---|---|
| Catalog and hierarchy | Implemented policy model, stable IDs, hierarchy validation, and layout operations | `src/topology_manager/catalog.rs`, `layout.rs`, `ids.rs` |
| Placement and balancing | Implemented placement, capacity checks, stable assignments, and explicit scoped rebalance | `src/topology_manager/resolver.rs`, `aggregate.rs` |
| Current subscriber data | Implemented PPP, DHCP, and Hotspot reads, normalization, observation gates, and configured orchestration | `src/topology_manager/reader.rs`, `normalize.rs`, `observation.rs`, `coordinator.rs` |
| Subscriber identity | Implemented durable identity and authorized source-reference reconciliation | `src/topology_manager/identity.rs`, `model.rs`, `store.rs` |
| Rates and capacities | Implemented rate rules, factors, subscriber bounds, node minimums, and hierarchy budgets | `src/topology_manager/rates.rs`, `catalog.rs`, `legacy_bounds.rs`, `source_rate.rs` |
| Source compatibility | Implemented source-policy preflight, exclusions, and explicit DHCP override bindings | `src/topology_manager/preflight.rs`, `source_policy.rs`, `dhcp_overrides.rs` |
| Static branches | Implemented authoritative source binding, protected ownership, shared attachments, and shared budgets | `src/topology_manager/static_input.rs`, `ownership.rs`, `shared_static.rs` |
| Legacy migration | Implemented explicit mappings, parity checks, aliases, and exact shared snapshot adoption | `src/topology_manager/migration.rs`, `parity.rs`, `legacy_static.rs` |
| Durable recovery | Implemented private backups, revision checks, staged installation, cancellation before installation, and forward recovery | `src/topology_manager/store.rs`, `staging.rs` |

Dynamic subscriber endpoints come from fresh source reads. Saved policy does not supply an old session's IP or MAC.
Placement in a WAN is topology intent. Router routing changes remain a separate task.

## Connected Runner APIs

| API or boundary | Current status |
|---|---|
| `Runner::prepare_topology` | Fresh configured preview. No registry commit, shaping write, reload, or writer claim |
| `Runner::enroll_topology` | Controlled first adoption. Checks the current live pair and manifest, then stages and completes policy installation |
| `Runner::lock` | Automatic recovery of a pending migration before receipt initialization and normal service work |
| `Runner::sync_topology` | Explicit collection, authorization, compilation, and publication API. The service loop does not select it automatically |
| Compiler and publication adapters | Implemented candidate preservation checks, one writer owner, write receipts, and apply-evidence integration |

These APIs are in `src/runner.rs`. Compiler and publication adapters are in `src/topology_manager/compiler.rs` and `publication.rs`.
Critical conflicts block the affected operation. Unknown files are not replaced with a guessed repair.
Enrollment and recovery do not claim topology runtime ownership or activate the manager service loop.

## Unfinished feature work

| Area | Current status |
|---|---|
| Enrollment interface and manifest editor | Done in Phase 1 on 2026-11-18: `lqosync3 topology` CLI, PPP-name and DHCP-MAC identity rules, and a runbook. Production enrollment is an owner action |
| Topology Manager TUI | Planned. No complete tab for discovered-node selection, hierarchy editing, promotion/demotion, or topology preview |
| Settings TUI migration | Pending. Node capacity, factor, and subscriber policy controls still need the approved topology interface |
| Speed scheduler | Planned. Subscriber policies and WAN/group capacities remain the approved scope |
| Scheduler profiles and timezone | Planned day/time rules and Mbps/percentage selectors. Configurable timezone defaults to `Asia/Manila` |
| Native planner admission | Pending proof of native implicit commitments and planner transformations |
| Live DPDK acceptance | Pending production acceptance proof. Scratch compilation does not establish live acceptance |
| Manager service activation | Gated. The existing service loop retains legacy behavior until a separate activation action |
| Additional migration paths | Existing-source replacement, active-owner migration, and direct schema-2 circuit/device-ID migration remain separate work |

## Phase 1 enrollment CLI (2026-11-18)

- Commands: `lqosync3 topology policy propose | check | policy install | propose | enroll`. See [the runbook](../../docs/operations/TOPOLOGY_ENROLLMENT.md).
- Identity rules: `PppSecretName` and `DhcpClientMac`, opt-in for each source scope. No router marker is necessary.
- Parity accepts a whole-Mbps node export when the legacy value equals the calculated ceiling, and ignores the CSV `Comment` column.
- Enrollment needs `routers.dhcp.require_bound = true`. Scopes with leases that are not in the legacy files, such as `ipoevlan10`, enter after Phase 4 activation.
- Verification: 1,235 tests passed, 0 failed, 13 ignored. Format, Clippy with warnings denied, and whitespace checks passed. The MSRV 1.89 build runs in CI.
- Next: Phase 2, the Topology Manager TUI tab and the Settings migration.

## Phase 2a service API (2026-11-18)

- The service listens on a root-only Unix socket, `/var/lib/lqosync3/api.sock`, with JSON-lines requests.
- Operations: `status`, `registry.get`, `policy.commit`. Command: `lqosync3 api status`.
- Edits do not change live shaping before Phase 4 activation.
- Next: Phase 2b, the Topology TUI with per-subscriber plans and pins, through this API.

## Phase 2b-1 Topology TUI read-only views (2026-11-18)

- `lqosync3 tui` has a Topology route (page 4) with Builder, Assignments, Policies, and Changes views.
- The preview is on demand: on route open when older than 300 s, or with `r`. It reads routers in the service and never publishes.
- Next: Phase 2b-2, per-subscriber plan, pin, and scoped rebalance edits.

## Phase 2b-2 subscriber plans (2026-11-18)

- The operator creates LQoSync speed plans and attaches one to a DHCP/IPoE client, or to a PPP subscriber as an override.
- API: `plan.create`, `plan.update`, `plan.delete`, `subscriber.plan`. TUI: plan picker in Assignments, Plans section and form in Policies.
- Edits need an enrolled registry. They do not change live shaping before Phase 4 activation.
- Next: Phase 2b-3, Builder node edits. Then pin and scoped rebalance, then 2b-4 profiles and speed defaults.

## Phase 2b-3a WAN discovery (2026-11-19)

- Production has two WANs: `pppoe-out` interfaces on a MikroTik core. PBR on the core uses per-IP address-lists from an owner script.
- LQoSync will decide the WAN of each subscriber by load balancing. Writing the core address-lists is a later feature.
- API: `wan.discover` reads `/interface` of each configured router and lists WAN candidates. TUI: `w` in the Builder.
- The legacy cycle skips routers without a subscriber protocol. The owner reviews this again when the Topology Manager is complete.
- Next: Phase 2b-3b, Builder edits (WAN node from a candidate, intermediate nodes, instances under several parents, move, remove, capacity). Then 2b-3c pin and scoped rebalance.

## Phase 2b-3b Builder edits (2026-11-19)

- The operator builds WAN, intermediate, and group instance nodes, moves and removes them, and edits names and capacities.
- API: `topology.edit.propose`, `topology.edit.confirm`, `topology.edit.cancel`, `topology.tree`. One kept proposal, 120 s.
- Each edit that changes the destinations of a source rebalances that source pool in the same revision: each group splits in the capacity ratio of its WANs. Group minimums split in the same ratio.
- Next: Phase 2b-3c, explicit pin and scoped rebalance from the TUI. Then 2b-4 profiles and speed defaults.

## Phase 2b-3c explicit rebalance and pins (2026-11-19)

- `rebalance` (one group or all) and `subscriber.pin` (pin by WAN, unpin) are edit kinds with the propose and confirm flow.
- Pins need an online subscriber. Offline pins can be removed by `subscriber_id`. A rebalance clears offline, unpinned assignments in the group.
- The owner plans application pinning with DPI later. It needs core routing and is not a subscriber pin.
- Next: Phase 2b-4, profiles and speed defaults. Then 2c Settings migration.

## Phase 2b-4 speed policy and group factors (2026-11-19)

- One global Subscriber Speed Policy for every subscriber. Plans have no minimum of their own. The edit replaces the legacy rounded rules of the imported profiles.
- The group factor (PPP `0.215`, DHCP `0.5` in production) is editable for each source.
- A group node capacity is plan sum times factor, limited by the WAN ceiling.
- Phase 4 must treat the registry as the authority, not the legacy `config.toml` values.
- Next: Phase 2c, Settings migration.

## Phase 2c-1 Settings managed fields (2026-11-19)

- `status` reports `writer` (`legacy`, `topology`, or `unknown`) from the runtime owner file.
- Settings notes the `nodes.`, `bandwidth_defaults.`, and `backstop_caps.` fields after enrollment and locks them once the Topology Manager writes shaping.
- Next: Phase 2c-2, new subscriber sources from the Builder.

## Phase 2c-2 fallback group and new sources (2026-11-19)

- An operator-set fallback group catches subscribers whose source the registry does not know, so shaping continues. Speed: router rate-limit, else the fallback default plan.
- An unpinned subscriber whose saved assignment is no longer eligible is placed again. Pins stay strict.
- `source.add` adds a new router source from a fresh enrollment draft; its fallback subscribers move to the new group.
- Rollback: clear the fallback before a rollback to an older build (`deny_unknown_fields`).
- Next: Phase 3, the planner and the DPDK proof.

## Recorded verification

At checkpoint `79ba5a2`, the recorded full suite has 1,193 passed tests, zero failures, and 13 ignored tests.
It includes 445 active topology tests and 12 new enrollment/recovery tests.
Format, Clippy with warnings denied, whitespace, and isolated native configured-source checks passed.
This graph refresh does not rerun production acceptance or provide a production performance benchmark.

## Sources

- [Implementation design](../../docs/design/TOPOLOGY_MANAGER_IMPLEMENTATION.md)
- [Approved discussion](../../docs/design/TOPOLOGY_MANAGER_DISCUSSION.md)
- [Planned TUI layout](../../docs/design/tui/TOPOLOGY_MANAGER_LAYOUT.md)
- [[2026-11-17-topology-enrollment]]
- [[2026-11-17-topology-migration-staging]]
- [[2026-11-17-topology-shared-adoption]]
