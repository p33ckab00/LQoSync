---
type: session
created: 2026-11-20
tags: [frontend, dpdk, research, queues]
status: proposal
related: ["[[2026-11-20-dpdk-dashboard]]"]
---

# Mirrored traffic and queue research

The owner requested research and a design proposal for one traffic comparison panel.
The panel would show Actual, Mapped, and Unmapped traffic with mirrored directions and gradients.
The proposal also includes compact DPDK figures and linked chart inspection.

The research checked installed LibreQoS source maps and a pinned upstream revision.
Queue Dynamics uses aligned throughput/RTT plots and a shared cursor.
Queue Stats links six charts over raw one-second samples for three minutes.
Queue Tree follows a circuit's live ancestor path.

Actual means delivered traffic. Mapped and Unmapped mean enqueued traffic.
Do not stack or add these series.
The current aggregate chart already uses Actual traffic.

Core histories exist, but the browser lacks typed classification history.
The broker response budget prevents an unchecked addition of full histories.
Circuit queue views need an authenticated contract with applied mapping checks.
Live ancestor metrics need verification before a live Queue Tree port.

The proposed chart retains Rust Leptos/Wasm and the current visual design.
Vercel frontend-design and Google modern-web-guidance supplied interface guidance.
The proposal includes atomic steps, backend gaps, and validation requirements.
No application code, dependency, production service, or TUI changed.

See [research and proposal](../../docs/frontend/MIRRORED_TRAFFIC_RESEARCH.md).
The design remains a proposal, not an accepted implementation decision.
