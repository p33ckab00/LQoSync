# LibreQoS mockup generation record

Tool: built-in `image_gen.imagegen`. Background: opaque.
References, in order: `mockups/routers-approved-v1.png`, `mockups/dashboard-dpdk-v2.png`.
Output: `mockups/libreqos-proposal-v1.png`.
State at generation: proposal. The owner subsequently approved the unchanged image.

## Final prompt

```text
Create ONE high fidelity screenshot concept for the LibreQoS screen of LQoSync3's Ratatui terminal console. The owner approved the attached Routers and Dashboard images. Strictly preserve their shared sidebar geometry, topbar, footer, typography, panel borders, selected-row treatment and palette. This is a new screen proposal for approval, not implemented software.
Wide landscape approximately 3:1 aspect ratio, matching the 2172 by 724 pixel reference composition. The interface fills the entire screenshot. No browser, device, perspective, desktop chrome, shadows, gradients, rounded web cards, or proportional fonts. Compact fixed-width monospace text, normal-sized numbers, thin straight rectilinear borders. Highly legible aligned terminal columns.

Palette: canvas #0b0c0e, sidebar #141517, panels #17191b, borders #2b2e32, text #f1f3f5, muted #b2b8bf, healthy #2bd391, warning #ffd077, critical #ff858a, accent #80aaff.
Left sidebar unchanged, width about 13.5 percent:
LQoSync3
TERMINAL CONSOLE
1 Dashboard
OPERATIONS
2 Subscribers
3 Routers
4 LibreQoS -- SELECTED with charcoal row and narrow green left marker
5 Workers
MANAGEMENT
6 Settings
7 Journal
ROADMAP
Profiles   planned
WAN plans  planned
Bottom SERVICE ACTIVE, MODE APPLY, TARGET v3_pair. Healthy ACTIVE green.
Main topbar: LibreQoS at left; DATA 2s ago | 09 Nov 2026 10:15 +08 | [Dark] on right.

Main content vertical layout:
1. Slim page heading row "Shaping input", with terminal text actions "[p Preview]  [s Sync now]" aligned right.
2. Full-width persistent warning strip, immediately below heading:
"Native compatibility: BLOCKED"
"Active target v3_pair remains imported. Native input is not enabled."
Use amber for native BLOCKED and green for imported. Clearly distinguish the blocked OPTIONAL native source from the successful ACTIVE pair import. Never show the active shaper as failed. This is an illustrative compatibility notice, not a missing-parent issue.
3. Two side-by-side compact rectangular cards, Last sync (about two thirds main width) and Remote sync (one third).
Last sync content, carefully aligned:
Target       v3_pair                 Mode      APPLY
Last cycle   09 Nov 2026 10:14:28 +08
Written      10:14:28                 Imported  10:14:29
Input state  IMPORTED                 Circuits  1,024
Rows         1,024                    Nodes     5
Use green IMPORTED. Sample times and counts only.
Remote sync content:
Status       NOT CONFIGURED
Destination  --
Last transfer --
Then dim short note "No remote integration configured."
Use muted/neutral state, not a success or an error. No invented remote health.

4. Large full-width Source preview panel, occupying the lower main content. Title "Source preview" and "read-only" qualifier, right label "Showing 1-6 of 1,024".
At top of this panel simple file tabs "[ShapedDevices.csv]" selected in blue, then "network.json" unselected.
Table columns "Circuit ID" | "Name" | "Parent" | "IPv4" | "Down/Up Mbps".
Exactly six synthetic example rows:
SUB-0001 | SUB-0001 | AP-NORTH | 192.0.2.11 | 100 / 50
SUB-0002 | SUB-0002 | AP-NORTH | 192.0.2.12 | 50 / 25
SUB-0003 | SUB-0003 | AP-EAST  | 192.0.2.13 | 100 / 50
SUB-0004 | SUB-0004 | AP-EAST  | 192.0.2.14 | 75 / 35
SUB-0005 | SUB-0005 | AP-SOUTH | 192.0.2.15 | 150 / 75
SUB-0006 | SUB-0006 | AP-SOUTH | 192.0.2.16 | 50 / 25
Highlight SUB-0002 in the same charcoal blue as the approved reference. Columns aligned and readable.
Bottom of preview panel: "Page 1 / 171   [Prev]  [Next]" and right "Selected: SUB-0002".

5. Thin full-width Data status panel above the terminal footer, same appearance as approved reference:
"Snapshot LIVE | Updated 2s ago | Active input imported"
"Illustrative data. Sync now requires confirmation."
6. Bottom keyboard footer: "Tab Focus   Enter Open   p Preview   s Sync now   q Quit".
Far right clearly legible "DESIGN PREVIEW | SAMPLE DATA".

Use the existing screen height efficiently so the warning, both cards, six-row preview, data status and footer all fit without clipped text. Preserve shared shell dimensions and the restrained operations-console look of both approved screenshots. All numbers and records are synthetic. Do not add a diagram, traffic graph, editable cells, credentials, source-switch control, Force Apply, bridge restart, or real customer information. Output only the single LibreQoS terminal screenshot.
```
