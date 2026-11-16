---
type: session
created: 2026-11-16
tags: [topology, catalog, schema, identity, reliability, performance, ste]
status: catalog-boundary-complete
related: ["[[2026-11-16-topology-registry]]", "[[2026-11-16-topology-foundation]]", "[[HANDOFF]]"]
---

# Topology catalog and hierarchy persistence

The owner directs the next slice after the subscriber registry.
Worktree: `/home/pi/LQoSync3.0`. Branch: `lqosync3`. Base: `b94ed7c` plus prior uncommitted foundation and registry work.
Unrelated local edits remain intact.

## Changes

Added `catalog.rs` for router roles, source origins, local profiles, hierarchy inputs, and configured targets.
Added `catalog_tests.rs` and `tests/topology_catalog.rs`.
Extended the registry document, store loader, and snapshot gate for schema 2 and catalog references.
Reused the pure layout validator and checked rate arithmetic.
Numeric policy structs now reject unknown nested fields.

The catalog keeps definition IDs separate from instance IDs and labels.
Repeated source instances keep separate parents and IDs. They share definition factors and profile policy references.
WAN bases, factor choices, group formulas, backstop inputs, and percentage modes remain explicit stored inputs.
The synthetic root has a label and no invented capacity node.

A local profile explicitly selects Local or SourceRead rate authority.
There is no automatic plan overwrite from discovery.
Selected sources require the corresponding router read role.
Unused discoveries remain catalog definitions. A new WAN does not expand an existing target list.

## One checked revision

The catalog and subscriber registry share `topology-manager/registry.json`, one writer, and one expected revision.
A candidate must keep valid source, profile, assignment, and pin references.
It must also respect configured potential target sets and explicit profile restrictions.
Actual current profile membership and active alias eligibility remain resolver checks.

Catalog tables and target lists use canonical order before persistence.
An order-only edit keeps the revision, bytes, and inode.
Node rename and parent move preserve the instance ID and pin.
No move changes packet routing.

## Schema and recovery

Current schema is 2. A strict schema-1 reader constructs schema 2 with catalog None in memory.
It preserves all IDs, counters, references, pins, and the original revision.
Open leaves the schema-1 bytes unchanged. A completed revision-checked commit persists the upgrade at revision + 1.

None means unconfigured. It is not an active default topology.

The loader decodes the original stream again after its schema header.
It does not collapse duplicate fields through a generic JSON map.
Unknown versions, unknown fields, false downgrades, and corrupt files are errors.
A configured catalog cannot become None through an ordinary edit.
Existing write uncertainty, missing-file, private-mode, and lock protections remain in place.

Before production migration, keep a verified private backup and prove rollback with fresh collection.
A configured schema-2 catalog has no lossless schema-1 representation.
No backup, downgrade, production migration, or live recovery action occurred in this slice.

## Review corrections

A store test used array position 2 after canonical sorting. It now finds the source by stable ID.
The expected atomic-edit behavior stays unchanged.
Clippy found fixture initialization outside its struct literal. The corrected fixture needs no lint suppression.

Two final regressions failed before their guards.
PPP uses PlanSum and Hotspot uses CountTimesUnit. DHCP keeps an explicit site/per-plan formula choice.
An existing source ID cannot change its authoritative origin. Shared factors remain editable because they are policy inputs.

The snapshot gate now carries its required scopes and must match the configured catalog.
The validated registry index carries catalog scopes, so reconciliation does not rebuild the catalog for each subscriber.
Source authority and wall-clock freshness still require verified adapters.

## Performance and checks

Catalog validation uses indexes and the iterative graph validator.
Target checks use sets instead of source scans for each subscriber.
Canonical persistence sorts ID tables once per candidate edit.
Schema detection adds a second parse of the saved document. It preserves strict decoding without a lossy intermediate map.

Synthetic proof covers 10,000 nodes and 10,000 subscriber bindings.
This is not a production benchmark or a maximum-throughput claim.

The slice adds 23 checks.
The counts are 15 catalog, 5 schema/reference/snapshot, 1 source-origin transition, and 2 public integration checks.
The first catalog and schema checks failed before implementation.
The final formula and origin regressions failed before their fixes.

Full `cargo test`: 793 passed, zero failed, 9 prior ignored tests.
Formatting, Clippy with denied warnings, whitespace, and no-live-caller checks passed.

No dependency, prior fixture, live configuration, service, router, or dataplane changed.
No commit, push, or deployment occurred.
STE guidance applies to technical text. The checker does not certify compliance.

## Next work

Resolve current source membership, effective profiles, eligibility, placement, rates, and actionable conflicts.
The resolver must use configured WAN bases for percentages, even when WAN shaping factors lower the effective limit.
It must count one circuit across multiple endpoints and preserve valid assignments and pins.
Source/instance ID creation, verified source adapters, full migration, policy history, publication, and TUI remain unfinished.
Native export precision and apply evidence still require proof before activation.
Scheduler overlap, overnight-day, and DST rules remain open.
