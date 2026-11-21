# Empty topology and shared source placements

Date: 2026-11-21 (Asia/Manila).

The screenshots use the isolated loopback browser fixture and intercepted synthetic snapshots.
They contain no live account, router credential, or production registry.

- `empty-topology.png`: no registry, no default nodes, and an explicit Start empty topology action.
- `add-parent.png`: a subscriber source can add another parent without moving its original placement.
- `shared-source-two-parents.png`: the same source has placements under WAN North and WAN South.

`tools/frontend/topology-initialization-check.cjs` verifies the empty state, cancel, review, confirm,
draft edit gates, blank parent refusal, duplicate parent exclusion, and the unchanged original parent.
Browser interception proves presentation and request shape. Core API tests prove durable initialization,
receipt retries, inventory-only discovery, selected source attachment, and no live output changes.

Core validation: 1,580 tests passed, 10 ignored, no failures.
The UI model has 40 passing tests. Gateway and contract checks have 62 passing tests and one ignored test.
Native and Wasm Clippy passed with warnings denied. The optimized Wasm build passed.
The existing topology, topology workflow, and subscriber workflow browser suites also passed.
These cover both themes, narrow layouts, discovery, reviewed changes, selection, and stale response guards.

No production registry, service, router, enrollment, runtime owner, or shaping file changed.
Existing saved policy remains visible. Initialization cannot reset it.
