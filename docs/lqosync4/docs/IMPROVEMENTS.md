# Improvements to resolve before implementation

| Priority | Improvement | Why it matters | Proof |
|---|---|---|---|
| P0 | Define one stable subscriber ID across PPPoE reconnects, DHCP renewal, and IP changes. | An IP is not a stable account identity. Wrong identity can move the wrong subscriber. | Reconnect and address-change fixtures keep one ID. |
| P0 | Define a narrow MikroTik write contract and object ownership. | The LQoSync3 code currently reads routers only. A broad writer can overwrite operator rules. | Readback, idempotency, conflict and rollback tests against a test router. |
| P0 | Separate desired, observed, and applied WAN state. | A successful API request does not prove that the router changed. | UI shows each state and failed readback. |
| P0 | Define health `unknown`, `suspect`, `down`, and `recovering`, plus thresholds. | One failed probe or stale monitor result should not cause mass movement. | Flap and stale-monitor simulations. |
| P1 | Version previews and require a fresh version at apply. | A stale tab could apply an old plan after health changed. | Stale plan is rejected and re-previewed. |
| P1 | Use bounded queues, paged APIs, and time-bucket traffic. | Large tables and fast samples can overload the server or browser. | 1,000 and 10,000 subscriber workload measurements. |
| P1 | Define schedule timezone and overlap priority. | An override can run at the wrong time or win unexpectedly. | Midnight, daylight-saving, overlap, and restart fixtures. |
| P1 | Publish data age and collector error with every operational view. | A green badge on stale data is misleading. | Unreachable-router test shows stale/unknown. |
| P2 | Plan IPv6 and multi-WAN/CGNAT identity separately from IP intelligence. | Destination and subscriber IPs have different meanings. | IPv4/IPv6 sample validation. |
| P2 | Specify telemetry retention and confidence before app labels. | Shared CDN IPs can misidentify an app; detailed traffic data needs access limits. | Accuracy sample and retention test. |

No numerical performance guarantee is set yet. Measure baseline on the target hardware, then set service budgets and a regression threshold.

## Web UI performance and readability review

The [visual map](UI_VISUAL_SYSTEM.md) gives the screenshot-to-component layout and concrete CSS starter. These are the priority changes before the web slice is accepted:

| Priority | Risk in the proposed screens | Design response | Proof |
|---|---|---|---|
| P0 | A dashboard request fans out into live router, monitor, and LibreQoS calls. | Serve one versioned summary snapshot. Run collectors independently. Fetch charts by bucket and time window. | Simulate one slow router; dashboard still paints from the last validated snapshot and labels its age. |
| P0 | Large PPPoE/assignment tables and raw CSV preview mount all rows. | Filter, sort, and page on the server; render only visible records; provide explicit CSV download. | Bound DOM count, API payload size, memory, and scroll time with 1,000 and 10,000 subscribers. |
| P0 | A mobile table hides profile or WAN write controls offscreen. | Use stacked record cards or a scroll panel with a reachable action column. | At 320 px width, every permitted action is reachable without page-level horizontal scroll. |
| P1 | Small gray screenshot labels and color-only badges hide state. | Set 14 px default text, stronger muted contrast, state words, keyboard focus, and 44 px touch targets. | Review 320, 390, 768, 1024, and 1440 px; test 200% zoom and keyboard-only use. |
| P1 | Nine or more live WAN graphs rerender on every sample. | Send bounded time buckets; update only changed cards; pause offscreen graphs; keep RX/TX text. | Measure frame time, memory, and network bytes with visible and background tabs. |
| P1 | Repeated same data appears in dashboard, health, and load-balancer views. | Share one snapshot/version key, cache per route, and invalidate only affected read models. | One source update changes the related view without a full page reload or duplicate router read. |
| P1 | A donut center sums unrelated WAN utilization percentages. | Show per-WAN utilization; compute a capacity-weighted aggregate only from matching sample windows. | Mixed 50/500/1000 Mbps fixture gives a valid displayed number. |
| P2 | The CSS starter grows into route-global overrides. | Keep shell, page-grid, and component styles in their owning layers; reuse tokens and primitive classes. | Compare Dashboard, Profiles, LibreQoS, and Load Balancer at each breakpoint for regressions. |

Capture baseline WebAssembly transfer size, first interaction time, API p50/p95, server/client peak memory, DOM row count, chart update cost, and bytes per refresh. Set numeric budgets only after the first Leptos slice runs on representative hardware.
