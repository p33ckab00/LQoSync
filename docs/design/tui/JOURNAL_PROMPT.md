# Journal mockup generation record

Tool: built-in `image_gen.imagegen`. Background: opaque.
References: `mockups/workers-approved-v1.png` and `mockups/dashboard-dpdk-v2.png`.
Output: `mockups/journal-proposal-v1.png` (2172 by 724 pixels).
State: proposal for owner review. No Journal approval has been recorded.

## Final prompt

```text
Create ONE high fidelity screenshot concept for the Journal screen of the LQoSync3 Ratatui terminal console. Use the attached APPROVED Workers and Dashboard images as style references only. Match the same full-screen 3:1 wide terminal composition, compact fixed-width monospace, left sidebar about 13.5 percent, slim top bar, bottom keyboard footer, fine rectangular borders, dense operational information, no rounded cards. This is a design proposal with synthetic log records, not running software.

No browser chrome, perspective, devices, shadows, gradients, decorative graphics or proportional fonts. Palette canvas #0b0c0e, sidebar #141517, panels #17191b, border #2b2e32, text #f1f3f5, muted #b2b8bf, healthy #2bd391, warning #ffd077, critical #ff858a, accent #80aaff. Normal sized monospace metrics; blue/charcoal selected row and blue focus outline.

Sidebar exactly matching references:
LQoSync3
TERMINAL CONSOLE
1 Dashboard
OPERATIONS
2 Subscribers
3 Routers
4 LibreQoS
5 Workers
MANAGEMENT
6 Settings
7 Journal -- selected with charcoal full row and thin green left marker
ROADMAP
Profiles   planned
WAN plans  planned
Bottom separator, SERVICE ACTIVE green, MODE APPLY, TARGET v3_pair.
Topbar Journal on left; LOG 2s ago | 09 Nov 2026 10:15 +08 | [Dark] right.

Main area top three equal summary cards:
Log source / lqosync3.service / journalctl stream
Retained lines / 1,000 / bounded local buffer
View / PAUSED / selection retained
Next toolbar one row: focused blue-outlined [Search: reload...] about half width, Level [All v], [Follow OFF], [Clear search]. All legible monospace.

Next main panel row: 70 percent width log table on left and 30 percent Entry details on right. Table title Service journal, right Showing 8 of 1,000 retained. Dim subtitle Search: reload | Level: All. Columns Time | Level | Message. Eight synthetic records with compact alignment:
10:14:02 | INFO | config reload requested
10:14:03 | INFO | config reload accepted
10:14:15 | WARN | router Backup read failed; retry queued
10:14:30 | INFO | cycle completed; cached data retained
10:14:42 | ERROR | LibreQoS reload timed out
10:14:44 | INFO | reload retry queued
10:14:58 | INFO | LibreQoS reload completed
10:15:00 | INFO | reload result verified
IMPORTANT keep all eight records consistent with search reload: change the WARN message to "router Backup reload check delayed" and the 10:14:30 message to "reload check uses cached data".
Select 10:14:42 ERROR row in blue/charcoal. WARN yellow, ERROR red, INFO muted white, never rely on color alone. Thin vertical scrollbar at right inside table with visible thumb. Footer inside table "Selected 1 entry | Follow paused". Avoid implying all logs are retained indefinitely.

Entry details panel:
Selected entry
09 Nov 2026 10:14:42 +08
Unit    lqosync3.service
Level   ERROR
Separator
Full message
LibreQoS reload timed out
Separator
[Copy selected]
[Resume follow]
Separator
"Inspection only"
"Actions do not restart services."
Do not include credentials, tokens, export, log deletion or service restart buttons.

Below panel row full-width compact Selection and navigation strip:
[Select text OFF] | Shift+Up/Down extend | [Copy selected] | End: latest
"Terminal selection mode temporarily releases application mouse capture."
Then full-width Data status strip:
Stream CONNECTED | Last received 2s ago | Retained 1,000 | Follow PAUSED
"Retention is bounded; search and level filters apply to retained lines."
Footer keyboard hints: 1-7 Navigate  / Search  l Level  f Follow  y Copy  s Select text  q Quit
Bottom right blue: DESIGN PROPOSAL | SAMPLE DATA
Ensure no overlaps, no clipping, comfortable single-row typography matching references, no text outside panels. Entire image must be a terminal design, not a web browser.
```
