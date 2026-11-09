# Workers mockup generation record

Tool: built-in `image_gen.imagegen`. Background: opaque.
References, in order: `mockups/routers-approved-v1.png`, `mockups/libreqos-approved-v1.png`.
Output: `mockups/workers-proposal-v1.png`.
State at generation: proposal. The owner subsequently approved the unchanged image.

## Final prompt

```text
Create ONE screenshot concept for the Workers screen of the LQoSync3 Ratatui terminal console. Match the attached APPROVED Routers and LibreQoS reference images: same full-screen wide approximately 3:1 terminal composition, compact monospace text, sidebar width about 13.5 percent, topbar and footer height, rectangular panels, fine straight borders, subdued charcoal fills, blue focused controls and full-row selection. This is a design preview with synthetic data, not a screenshot of running software.
No browser chrome, device, perspective, proportional fonts, oversized metric numbers, rounded web cards, gradients, shadows or decorative graphics. Use fixed-width terminal characters and block bars only. Palette: canvas #0b0c0e, sidebar #141517, panel #17191b, border #2b2e32, text #f1f3f5, muted #b2b8bf, healthy #2bd391, warning #ffd077, critical #ff858a, accent #80aaff.

Keep the approved sidebar verbatim:
LQoSync3
TERMINAL CONSOLE
1 Dashboard
OPERATIONS
2 Subscribers
3 Routers
4 LibreQoS
5 Workers -- selected with charcoal row and slim green left marker
MANAGEMENT
6 Settings
7 Journal
ROADMAP
Profiles   planned
WAN plans  planned
Sidebar bottom SERVICE ACTIVE in green, MODE APPLY, TARGET v3_pair.
Topbar Workers at left; DATA 2s ago | 09 Nov 2026 10:15 +08 | [Dark] at right.

Main content layout:
Three equal summary cards: Workers / 4 / reported by bridge; Busiest worker / 43.1% / worker 0; Telemetry / LIVE / snapshot age 2s. Normal-sized monospace values.
Below cards a compact toolbar: [Worker ID ...] search with blue focus border; Sample [All v]; [Clear] at right.
Main row below: worker table left about 68 percent main width and Worker details panel right about 32 percent. Match approved Routers table/detail geometry, but leave room below for a full-width pressure panel.
Table title Worker utilization; dim qualifier "Busy excludes empty polling"; right label Showing 1-4 of 4.
Columns Worker | Busy | Idle | Mean backlog | Sample.
Rows:
0 | 43.1% | 56.9% | 8 packets | LIVE
1 | 38.6% | 61.4% | 6 packets | LIVE
2 | 24.8% | 75.2% | 3 packets | LIVE
3 | 17.5% | 82.5% | 1 packet  | LIVE
Highlight worker 0 with the exact blue/charcoal selected-row fill of the references. LIVE in green. Keep table rows compact and aligned, do not invent CPU core IDs or packet-rate columns.
Bottom table label Workers 1-4 / 4 at left; Selected: worker 0 at right.

Worker details panel:
Worker      0
Busy        43.1%  [a green terminal block bar filled to about 43 percent]
Idle        56.9%  [a second bar filled to about 57 percent]
Backlog     8 packets
Sample age  2s
Source      Analytics snapshot
Separator.
"Read-only inspection"
"[Enter] Expand details"
"[c] Copy details"
Separator.
Dim note "Busy is forwarding utilization," and "not OS CPU usage."
Use typography consistent with reference; no line overlap or clipping.

Under both panels, full-width Buffer/ring pressure panel:
Heading "Buffer/ring pressure" at left and green "CLEAR" aligned right.
Two compact rows of text measurements, evenly spaced:
Pool free 84.0% | Ring used 12.5% | Rings >75% 0
New RX buffer events 0 | New pool drops 0 | Sample age 2s
A dim note "New events are increments since the previous valid sample."
No charts in this panel and no sum across worker percentages.

Below, full-width thin Data status panel:
"Snapshot LIVE | Updated 2s ago | Source: LibreQoS DPDK analytics"
"Illustrative data. Worker and queue configuration is unchanged."
Bottom keyboard footer:
"Up/Down Select   Tab Focus   Enter Details   c Copy   r Refresh   q Quit"
Right footer "DESIGN PREVIEW | SAMPLE DATA".
Use the screen height efficiently: summary cards, toolbar, table/details, pressure panel and data panel must all fit. The four-worker set is purely illustrative. Do not add worker-count settings, CPU pinning controls, topology changes, bridge restart actions, packet shaping edits, load-balancer controls, historical graphs, or real customer information. Output only the single Workers screenshot, following the approved terminal design faithfully.
```
