---
type: session
created: 2026-11-20
tags: [frontend, dpdk, traffic]
status: implemented
related: ["[[2026-11-20-mirrored-traffic-research]]", "[[2026-11-20-dpdk-dashboard]]"]
---

# Consolidate aggregate traffic

The owner approved Actual, Mapped, and Unmapped in the current aggregate chart.
The implementation retains Rust Leptos/Wasm and the approved Dashboard layout.
Vercel frontend-design and Google modern-web-guidance supplied the interface guidance.

The original aggregate source remains Actual delivered traffic.
Mapped and Unmapped use enqueue histories from the same published worker cache.
The browser receives typed numeric series with independent source availability.
The chart does not add or stack these rates.

Download appears above zero. Upload appears below zero.
Each direction has its own scale and capacity gradient anchor.
The source controls, selected timestamp, and open table survive live updates.
Pointer, keyboard, and touch select one timestamp for all three sources.
Missing samples remain distinct from zero.

The broker starts with at most 300 points per source.
It trims the oldest samples when the full overview exceeds 60 KiB.
The reply guard enforces the 64-KiB IPC limit.
Cached time indices and SVG geometry keep pointer inspection local.
Cache failures retain last known histories with stale state.

Native projection and UI tests, release builds, Clippy, Rust 1.89, and browser checks passed.
The browser checks cover themes, mobile width, source state, capacity, gaps, and selection stability.
See [feature contract](../../docs/frontend/AGGREGATE_TRAFFIC.md).
See [screenshots and verification](../../docs/frontend/evidence/aggregate-traffic/README.md).

Production services and files did not change.
Install the prepared web artifacts with `sudo tools/frontend/install.sh`.
The TUI, core collectors, and CLI account policy remain unchanged.
Longer ranges and subscriber queue views need separate contracts.
