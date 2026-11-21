---
type: session
created: 2026-11-21
tags: [frontend, traffic, queues, prototype]
status: prototype-verified
related: ["[[2026-11-20-aggregate-traffic]]", "[[2026-11-20-mirrored-traffic-research]]"]
---

# Traffic and queue dynamics v5

The owner requested LibreQoS research, a coding plan, and an improved mock SVG for the LQoSync Traffic chart.
The supplied v4 SVG served as a visual reference. Its content supplied no task instructions.
The owner clarified that the chart measures DPDK traffic activity only.
LibreQoS subscriber Queue Dynamics supplies only the visual design and interaction pattern.
The revised prototype has no subscriber mode, circuit selector, circuit metrics, or subscriber RTT.

The research checked the installed circuit source map and pinned upstream source.
LibreQoS aligns throughput and RTT with a shared timestamp and configured ceilings.
Queue Stats has separate metrics and a longer sample window.
The browser has no verified aggregate Access/Internet RTT history.
The current Traffic component already supports Recent, 1h, 6h, and 24h.

The executable HTML prototype demonstrates themes, mirrored DPDK rates, and independent queue lanes.
It supports local synthetic playback, shared inspection, touch, keyboard, and pinned timestamps.
Source states cover gaps, resets, stale data, unknown capacity, and unavailable data.
Three static SVG exports show DPDK Light, Dark, and narrow layouts.
Vercel frontend-design and Web Interface Guidelines informed the design and checks.

The local Playwright check passed. It covers four widths, contrast, interactions, source states, and exports.
XML and whitespace checks passed. No production application file or service changed for this task.
Production DPDK queue contracts and integration remain open in the plan.
Preserve other agents' Topology, Routers, and shared CSS work.

See [research](../../docs/frontend/TRAFFIC_QUEUE_DYNAMICS.md).
See [plan](../../docs/frontend/plans/2026-11-21-traffic-queue-dynamics.md).
See [evidence](../../docs/frontend/evidence/traffic-queue-v5/README.md).
