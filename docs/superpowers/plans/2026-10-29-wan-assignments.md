# WAN Mapping and Subscriber Assignment

**Goal:** Add explicit WAN interface mapping and stable subscriber assignments to the Rust UI and local publisher.
**Architecture:** Reuse WAN node router/source fields and manual root capacities. Add per-source destination pools and subscriber pins in local settings.
Persist assignment ownership with the normalized snapshot and output pair under the same generation pointer.

## Rules

- No router writes, routing changes, LibreQoS export/apply, package changes, or service restarts.
- A WAN interface is an explicit operator choice from fresh discovered interfaces. Do not infer actual subscriber paths.
- One router/interface maps to one WAN root. Capacities remain the root's configured download/upload values.
- Destination pools include the original source node and compatible same-kind unbound branches under mapped WANs.
- Pins take precedence. Optional balancing applies to new/unassigned subscribers only; valid prior assignments remain stable.
- Balance by projected plan-rate load relative to directional WAN capacity and explicit target-node capacity.
- This is planned-load distribution, not live traffic balancing or a hard admission quota. Keep subscriber rates unchanged.
- Process retained/pinned loads before new subscribers. Sort new IDs and tie-break target IDs for deterministic output.
- Reject invalid pools/pins, incompatible source targets, duplicate subscriber IDs, corrupt assignment state and unhealthy mapped WAN metadata.
- Interface unavailability retains the last valid generation; no automatic WAN failover or mass subscriber migration.
- Persist source ownership and target assignment before the authoritative pointer switches. Older generations without assignment metadata remain readable.
- Poll mapped WAN routers with source routers. Keep selected-read concurrency and pause behavior.
- Run CPU-heavy work and snapshot reads outside the application lock. Bound and paginate assignment UI.

## Work

- [x] Write failing tests for mappings, compatibility, pins, stable assignment, restart, deterministic capacity distribution and rate integrity.
- [x] Implement settings validation, WAN health validation and assignment selection.
- [x] Persist assignment metadata atomically with normalized generation files, with old-generation compatibility.
- [x] Connect assignment processing and mapped WAN collection to the orchestrator with stale-result guards.
- [x] Add WAN mapping/capacity forms, pool controls and paginated subscriber assignment controls.
- [x] Verify HTTP token checks, invalid-edit preservation, assignment corruption, and automatic publication.
- [x] Measure a synthetic 50k-subscriber assignment workload and perform independent performance/integrity review.
- [x] Run checks and update project notes. Prepare the authorized commit and push to `lqosyncv3`.

## Verification

68 functional tests passed. Formatting, strict Clippy, and diff checks passed.
Three manual benchmarks are ignored by default. The new assignment benchmark passed separately.
50,000 subscribers across eight WANs: new assignments 525 ms, retained assignments 229 ms, debug build.
Measurements exclude collection and disk publication.
Independent review identified offline ownership loss and search pagination defects.
Regression tests failed before the fixes and passed after them. Follow-up review found no remaining material issue.
The application and test data remain separate from production LibreQoS and router configuration.
