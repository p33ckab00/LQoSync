# Traffic and Queue Dynamics Implementation Plan

> **For agentic workers:** Use superpowers:executing-plans to implement this plan task by task.

**Goal:** Enhance the whole-bridge DPDK Traffic panel with Queue Dynamics design and linked time inspection.

**Architecture:** Keep Rust/Leptos and the current SVG chart. Add typed optional telemetry after source verification.
Use a separate bounded request for queue history. Keep pointer inspection local.

**Tech Stack:** Rust, Leptos/Wasm, current CSS tokens, native SVG. No new application dependency.

**Spec:** [Traffic and queue dynamics](../TRAFFIC_QUEUE_DYNAMICS.md).

## Global constraints

- Measure DPDK traffic activity only. Queue Dynamics supplies the visual and interaction reference.
- Do not add subscriber mode, circuit selectors, circuit watchers, plan ceilings, or subscriber RTT metrics.
- Preserve current Dashboard behavior, Recent/1h/6h/24h ranges, and the DPDK card.
- Preserve the 60-KiB overview target and 64-KiB IPC limit.
- Keep unknown, zero, stale, missing, unavailable, and reset states distinct.
- Coordinate shared file ownership before the production port. Preserve all unrelated work.
- Use Light, Dark, and System settings. Do not add a second production theme store.
- Do not edit Topology, PPPoE, IPoE, Routers, or production services for this task.

## Review focus

Check bridge source generations, counter resets, mismatched cadences, expired pinned timestamps, and WAN capacity changes.
Reject subscriber-scoped fields from the DPDK chart contract.

## Task 1: Research and executable design reference

- [x] Inspect the installed LibreQoS source and pinned upstream references.
- [x] Compare the supplied v4 SVG against current traffic code and theme settings.
- [x] Document DPDK-only scope, source gaps, interactions, and component boundaries.
- [x] Implement a portable prototype with synthetic data and native controls.
- [x] Export and inspect desktop Light/Dark and narrow SVG samples.
- [x] Run the prototype browser check and record evidence.

## Task 2: Add a bounded queue-history contract

Files: `crates/web-contract/src/lib.rs`, `crates/web-contract/src/dpdk.rs`, and `crates/web-gateway/src/{history,broker,http}.rs`.
The `/api/v1/traffic` route uses the history and broker modules. Keep the current overview intact.

- [ ] Write failing projection tests for scope, units, timestamps, source age, and optional values.
- [ ] Define a queue point with `at`, `generation`, scope, backlog bytes, drop delta, mark delta, and interval milliseconds.
- [ ] Attach independent quality and capability fields. Reject non-finite rates and invalid timestamps.
- [ ] Keep at most 60 raw one-second points for the optional recent queue view.
- [ ] Read the current worker cache. Do not start a second bridge collector.
- [ ] Keep counter resets and source changes out of delta calculations.
- [ ] Test the response size against the current broker envelope limit.
- [ ] Run contract, projection, broker, and gateway checks.

## Task 3: Integrate aggregate detail lanes

Files: `crates/web-ui/src/traffic_chart.rs`, `traffic_plot.rs`, `crates/web-ui/style.css`, and `tools/frontend/aggregate-check.cjs`.

- [ ] Write failing tests for shared timestamp lookup and unavailable secondary lanes.
- [ ] Reuse current rate paths, capacity anchors, theme tokens, and source controls.
- [ ] Add optional Unmapped detail and bridge backlog lanes with explicit units.
- [ ] Exclude subscriber RTT fields and subscriber controls from the DPDK chart.
- [ ] Keep selection across updates. Show unavailable values when a pinned point expires.
- [ ] Add persistent inspector values and an accessible table.
- [ ] Check hover, tap, keyboard, focus, themes, narrow widths, stale data, and gaps in the browser fixture.
- [ ] Run UI tests, release Wasm, Clippy, formatting, and affected browser suites.

## Completion gate

The prototype is the completed deliverable for Task 1.
Tasks 2–3 describe the DPDK production port. They are not complete merely because the mock renders.
Publish or install production artifacts only through the project's normal deployment workflow.
