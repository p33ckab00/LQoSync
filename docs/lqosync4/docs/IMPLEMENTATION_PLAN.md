# LQoSync4 implementation plan — draft

The [delivery roadmap](../ROADMAP.md) is the agent-ready execution plan. Its phase files define task IDs, layer owners, dependencies, implementation instructions, and acceptance evidence. This file remains the short phase summary; use the roadmap for task handoffs.

The local `/opt/lqosync4` checkout tracks the new `lqosync4` branch. Do not deploy or perform live RouterOS writes during planning. Confirm the core-router contract before implementation.

**Stack constraint:** Rust backend and Leptos browser frontend compiled to WebAssembly. CSS and generated wasm/browser glue are allowed; no hand-written JavaScript or TypeScript application logic. Use a stable Leptos release and measure the Dashboard and table slice. Choose the backend framework after the API spike in [Rust backend options](RUST_BACKEND_OPTIONS.md). Share typed DTOs in a small Rust crate; keep RouterOS and database types server-only.

| Phase | Work | Exit gate |
|---|---|---|
| 0. Baseline | Record LQoSync3 commit, tests, config/state formats, and MikroTik core write contract. Confirm remaining owner features. Measure current cycle time and output size. | Behavior matrix, sample read-only RouterOS data, and baseline measurements. |
| 1. Core port | Remove TUI command/module/dependencies. Preserve sync, shadow/apply, validation, atomic writes, and LibreQoS v2/v3 targets. | Golden output parity, config migration, fresh install and rollback tests. |
| 2. Web foundation | Build a Leptos Dashboard and paged 1,000-row table as the measured UI slice. Use the [visual map](UI_VISUAL_SYSTEM.md) and [CSS starter](../ui-reference.css) for the responsive shell and text/card layout. Add Rust API, auth, roles, audit, migrations; read-only dashboard, routers, subscribers, LibreQoS preview. Serve latest snapshots without blocking on RouterOS. | Real data on all read screens, stale/error states visible, no secret exposure; desktop/mobile/200% zoom review; measured browser bundle and table performance. |
| 3. Profiles | IPoE/PPPoE profiles, baseline speed, scheduled overrides, effective-speed preview and sync. | Midnight, overlap, timezone, restart and idempotency tests. |
| 4. Assignment | Upstream model, capacity, stable percentage/count/manual engine, dry-run, Allocations/Assignments/Discovery/History views. Page and filter large assignment sets on the server. | Deterministic decisions, manual precedence, no unexplained movement; benchmark 1,000 and 10,000 subscribers. |
| 5. Health and apply | Health states and hold-down; narrow RouterOS writer with readback, retry, audit and rollback. | Outage/recovery and partial-failure simulation; reviewed shadow output before live apply. |
| 6. Telemetry | Traffic utilization history, then destination IP/ASN/organization/CDN/app roadmap. | Freshness, load, privacy and classification accuracy measured. |

Each phase delivers a working slice with relevant tests. Implementation does not imply production deployment.

## Performance verification

Capture p50/p95 dashboard and table API latency, browser first load/interaction time, WebAssembly transfer size, server peak memory, collector cycle duration, and RouterOS read/write counts. Benchmark on representative hardware before setting numeric service targets. Test a slow or unreachable router and a burst of subscriber changes; the UI must remain responsive and queued work must stay bounded. Keep a reproducible benchmark command and fixture with the implementation.
