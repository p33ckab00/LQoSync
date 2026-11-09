# Mockup generation record

Tool: built-in `image_gen.imagegen`.
Mode: reference-image generation. Background: opaque.
The original approved Dashboard and Subscribers assets are unchanged copies of the earlier generated images.
The following records preserve the prompts for the updated Dashboard, Routers, LibreQoS, Workers, and Settings assets.

## Dashboard v2

Reference: `mockups/dashboard-approved-v1.png`.
Output: `mockups/dashboard-dpdk-v2.png`.

```text
Edit this approved LQoSync3 Dashboard terminal mockup with ONE CHANGE ONLY: replace the graph panel title "Traffic overview" with the exact title "LibreQoS DPDK Traffic". Preserve the approved composition and all other text, sidebar order, dimensions, spacing, monospace terminal font, panel borders, palette, chart, legends, time-window controls, cards, dataplane health, WAN planned table, alert panel and footer. The new title must fit at the left of the existing chart header, with [5m] 15m 1h immediately after it and the Down/Up legend retained at the right without overlap. Do not redesign, reflow other panels, add a new widget, or change any sample numbers. This remains a synthetic design preview; retain "DESIGN PREVIEW | SAMPLE DATA". Output the complete full-screen screenshot at the same wide aspect ratio as the reference. No surrounding device or browser frame. All screenshot elements other than this label edit must remain visually identical to the approved reference.
```

## Routers v1

References, in order: `mockups/subscribers-approved-v1.png`, `mockups/dashboard-dpdk-v2.png`.
Output: `mockups/routers-proposal-v1.png`.

```text
Create ONE new LQoSync3 Routers terminal UI screenshot concept, matching the OWNER-APPROVED Subscribers screenshot (first reference) and Dashboard screenshot (second reference). This is the next screen for owner review, not implemented software. Preserve the exact overall visual language and the same approximately 3:1 wide aspect ratio: full screen terminal, compact monospace typography, near-black canvas, charcoal sidebar and panels, fine thin straight borders, bright white labels and muted metadata, blue keyboard focus, green healthy states. No browser chrome, no desktop device, no perspective, no shadows, no proportional fonts, no rounded web cards or oversized numbers.

Keep the same shared shell and same left sidebar occupying about 13.5 percent width. Brand LQoSync3 / TERMINAL CONSOLE. Same navigation:
1 Dashboard
OPERATIONS
2 Subscribers
3 Routers — SELECTED with charcoal fill and narrow green marker
4 LibreQoS
5 Workers
MANAGEMENT
6 Settings
7 Journal
ROADMAP
Profiles   planned
WAN plans  planned
Bottom SERVICE ACTIVE (green), MODE APPLY, TARGET v3_pair.
Topbar title Routers. On right DATA 2s ago | 09 Nov 2026 10:15 +08 | [Dark].
Use reference palette #0b0c0e canvas #141517 sidebar #17191b panels #2b2e32 borders #f1f3f5 text #b2b8bf muted #2bd391 green #ffd077 amber #ff858a red #80aaff blue.

Main region follows Subscribers screen composition precisely:
Three equal boxed cards across the top: Configured / 4 ; Reachable / 4 ; Failed reads / 0. Normal monospace numerals, not oversized. Green for reachable value. Muted qualifiers "enabled routers", "latest collection", "latest collection".
Search/filter strip underneath: [Search router / endpoint ...] with visible blue focus, State [All v], [Clear]. Keep aligned compact controls, no other filters needed.

Main row below: wide router table about 68 percent of the main content on left; selected router details about 32 percent on right, same panel heights as Subscribers. Table header Router list and quiet Showing 1-4 of 4 on right.
Columns Router | Endpoint | State | PPP / IPoE | Last read | Result.
Synthetic rows only:
EDGE-01 | 192.0.2.101 | REACHABLE | 180 / 80 | 2s ago | OK
EDGE-02 | 192.0.2.102 | REACHABLE | 176 / 78 | 2s ago | OK
EDGE-03 | 192.0.2.103 | REACHABLE | 178 / 76 | 2s ago | OK
EDGE-04 | 192.0.2.104 | REACHABLE | 178 / 78 | 2s ago | OK
Highlight EDGE-02 with the same restrained blue/charcoal full-row selection as Subscribers reference. REACHABLE and OK green, rest white/muted.
Keep selected label Selected: EDGE-02 and pager Page 1 / 1 at the bottom of table panel. Leave calm empty space under the four rows so panel geometry remains faithful to the approved reference.

Right panel title Router details.
Key/value rows:
Name        EDGE-02
Endpoint    192.0.2.102
State       REACHABLE
Last read   2s ago
Read time   184 ms
Divider and group Enabled sources:
PPPoE       ON
DHCP / IPoE ON
Hotspot     OFF
Divider and group Collected records:
PPPoE       176
IPoE        78
Total       254
Divider and group Navigation:
[Enter] Expand details
[s] View subscribers
[e] Open Settings
[c] Copy endpoint
Use concise well-aligned readable rows that fit without clipping, at same font size as other text. No credentials, passwords, role or configuration mutations, Add/Delete/Test/Apply buttons, WAN assignments, routing actions, or invented device health measurements.

Bottom of main content, a thin full-width Data status panel matching Subscribers reference:
Snapshot LIVE | Updated 2s ago | Source: latest router collection
Router counts and endpoints are illustrative.
Bottom footer:
Up/Down Select   / Search   Tab Focus   Enter Details   Esc Back   q Quit
Right label must clearly read DESIGN PREVIEW | SAMPLE DATA.
Every word, metric and state shown is sample data in a proposed screenshot, not actual production data. Preserve the practical compact SSH console look, aligned column spacing, shell proportions and graphic restraint of the approved references. Output only this single Routers screenshot.
```

## Routers approval

The owner approved Routers v1 after review.
`mockups/routers-approved-v1.png` is an unchanged copy of `mockups/routers-proposal-v1.png`.
No image edit or regeneration occurred for approval.

## LibreQoS v1

The [LibreQoS generation record](LIBREQOS_PROMPT.md) contains the references and final prompt.
The owner approved LibreQoS v1 after review.
`mockups/libreqos-approved-v1.png` is an unchanged copy of `mockups/libreqos-proposal-v1.png`.
No image edit or regeneration occurred for approval.

## Workers v1

The [Workers generation record](WORKERS_PROMPT.md) contains the references and final prompt.
The owner approved Workers v1 after review.
`mockups/workers-approved-v1.png` is an unchanged copy of `mockups/workers-proposal-v1.png`.
No image edit or regeneration occurred for approval.

## Settings views

The [Settings generation record](SETTINGS_PROMPTS.md) contains the final prompts and references for both approved views.
It also retains the initial Add Router candidate and its header correction.
The owner approved both final Settings views.
`settings-add-router-approved-v1.png` matches `settings-add-router-proposal-v1.png` byte for byte.
`settings-values-approved-v1.png` matches `settings-values-proposal-v1.png` byte for byte.
Use the approved copies as canonical references. No image edit or regeneration occurred for approval.
The initial Add Router candidate remains historical evidence only.
Runtime functionality requires separate implementation and verification.
