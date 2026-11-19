# Manager node locations implementation plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development or superpowers:executing-plans.

**Goal:** The manager export writes the router `latitude` and `longitude` on each node from that router, so enrollment parity matches the legacy pair.

**Spec:** `docs/superpowers/specs/2026-11-19-topology-node-locations-design.md`

## Global Constraints

- No registry schema change. No new dependencies. MSRV 1.89.
- Memory: `nice -n 19`, `CARGO_BUILD_JOBS=1`, tests `-- --test-threads=2`. Gate: `cargo fmt --check`, `cargo clippy --all-targets --locked -- -D warnings`, `cargo test --locked -- --test-threads=2`, `git diff --check`.
- Do not stop, restart, or reconfigure `lqosync3.service`; do not write `/etc`, `/var/lib`, `/opt/libreqos`, or routers.
- Fixtures are synthetic. Persisted text follows ASD-STE100.

## Review Focus

1. A pair prepared for the live test, the enrollment, and publication all carry the same locations (one code path).
2. The legacy `add_locations` output and the manager output give equal `numeric_network` values (float formatting).
3. A node without a source never gets a location.

### Task 1: Router locations in the manager export

**Files:** `src/topology_manager/publication.rs` (node export near `exported_node`), the caller chain from `Coordinator::prepare` (which has `cfg`) to `prepare_with_static`, tests in the matching `*_tests.rs` and `parity_tests.rs`.

**Interfaces:** a router-name-to-location map built from `cfg.routers` reaches the export (for example a field on `AuthorizedReconciliation` or a parameter); the exact plumbing is the implementer's call. Existing test callers pass an empty map.

- [ ] Write the failing tests from the spec Verification list.
- [ ] Run them: FAIL.
- [ ] Implement.
- [ ] Run the tests and the full gate: PASS.
- [ ] Commit `fix(topology): write router locations on manager nodes`.
- [ ] Update `docs/design/TOPOLOGY_MANAGER_IMPLEMENTATION.md` (Phase 3 section: the location fix) and `brain/30-knowledge/topology-manager-status.md`; commit `docs(topology): record manager node locations`.
