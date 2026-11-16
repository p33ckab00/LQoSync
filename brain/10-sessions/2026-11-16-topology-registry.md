---
type: session
created: 2026-11-16
tags: [topology, registry, identity, reliability, performance, ste]
status: registry-boundary-complete
related: ["[[2026-11-16-topology-foundation]]", "[[2026-11-16-topology-product-review]]", "[[HANDOFF]]"]
---

# Topology subscriber registry

The owner directs the next implementation slice after the pure foundation.
Worktree: `/home/pi/LQoSync3.0`. Branch: `lqosync3`. Base: `b94ed7c`.
Prior foundation work and unrelated local changes remain intact.

## Changes

- Added `model.rs` under `src/topology_manager/` for durable identities and subscriber policy intent.
- Added `observation.rs` for complete current snapshots and identity reconciliation.
- Added `store.rs` for private, revision-checked durable edits.
- Added `tests/topology_registry.rs` for restart, absence, and reconnect through the public API.
- Updated module exports, both implementation plans, the design, and the discussion state.

The registry contains scoped account references, internal IDs, profile intent, assignment intent, and pins.
It has no fields for IP, MAC, or session observations. Strict JSON decoding rejects unknown fields.
Ordinary edits keep all previous records and verified identity ownership.
They may add explicit aliases and change policy intent.
The complete policy catalog must check opaque profile and instance references before publication.

## Current observations

A complete snapshot requires one successful read for each required source scope.
Missing, partial, failed, duplicate, or unexpected reads cannot produce that snapshot.
The gate rejects reused collection generations, including previously failed reads.
Reconciliation consumes a snapshot. The type has no Clone or serde implementation.
This is an in-process evidence boundary.
It does not certify a RouterOS adapter or source freshness by wall time.


Current endpoints come only from that snapshot.
Complete empty reads remove active entries while durable identity and policy remain.
A reconnect with changed addresses keeps the same verified identity and pin.
A different current account at the same old address receives its own ID and default intent.
Several sessions or explicit aliases share one circuit without dropping their endpoints.
Current packet-visible IP collisions are errors even across different router IDs.

## Durability and recovery

The store holds the existing daemon instance lock.
It uses `files::write_secret` for a private temporary file, file sync, rename, and directory sync.
Expected revision and saved-document equality reject stale or external edits.
Unchanged intent keeps its revision, bytes, and inode.
A write error can follow rename. The store then blocks document access and commits until reopen.


Review found a missing-file ambiguity after restart. A new private directory now receives a durable empty document at revision 0.
A missing file in an existing directory requires recovery. Corrupt or future-schema files never become empty registries.
Review also found that a FIFO can block before metadata checks. A failing timeout regression preceded the `O_NONBLOCK` fix.
Reopen confirms the private directory after a possible earlier sync error.
No local file format can recover an entirely lost state directory without a backup.


## Performance review

Identity validation and lookup use indexes. They do not scan prior CSV rows for every subscriber.
New ID allocation sorts only new identities. Active output uses deterministic ordering by ID and session key.
Each ordinary edit checks the full saved registry before replacement.
Space is linear in durable identities and current endpoints.
Sort cost depends on the new identities and active output.

A synthetic check covers 10,000 subscribers, two sessions each, and changed current addresses.
It keeps the same registry intent after reconnect. This is not a production benchmark.

## Checks

Model: 7 passed. Observations: 13 passed. Store: 14 passed. Public integration: 1 passed.
Model, observation, and store behavior tests failed at stubs before implementation.
The two recovery regressions failed before their fixes. The scale and integration checks cover composed behavior.
Full `cargo test`: 770 passed, zero failed, 9 prior ignored tests.

Formatting, Clippy with denied warnings, and whitespace checks passed.
No prior fixture, Cargo dependency, or live caller changed.

No commit, push, deployment, service action, production write, or router change occurred.
STE guidance applies to technical prose and code text. Its checker does not certify compliance.

## Next work

Build the complete policy/source catalog and persist the approved hierarchy and capacity inputs.
Keep configured manual capacity inputs and factors separate from resolved values.
Then implement verified source adapters, explicit migration, placements, balancing, publication, and TUI integration.
Before activation, prove catalog references, source authority, collection age, and native export/apply behavior.
Scheduler targets remain subscriber policies and WAN/group capacities. Overlap, overnight-day, and DST rules remain open.
