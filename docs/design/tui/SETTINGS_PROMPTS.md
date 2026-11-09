# Settings mockup generation record

Tool: built-in `image_gen.imagegen`. Background: opaque.
State at generation: proposals. The owner subsequently approved both final views without an image edit.
These images do not establish runtime functionality.
The first Add Router candidate had a clipped theme label. The final candidate corrects that header.

## Initial Add Router

References, in order: `mockups/routers-approved-v1.png`, `mockups/workers-proposal-v1.png`.
Output: `mockups/settings-add-router-initial-v0.png`.

```text
Create ONE screenshot mockup of the LQoSync3 terminal Settings screen, Add Router view. Match the attached approved Routers and Workers screenshots exactly in visual language: same 3:1 wide composition, fixed-width monospace text, full-screen terminal, 13.5-percent left sidebar, slim topbar, thin square borders, charcoal panels, blue focus, green healthy states and amber draft states. No browser chrome, gradients, shadows, rounded web cards, devices or oversized text. This is a design preview, not an implemented live form.
Palette canvas #0b0c0e, sidebar #141517, panels #17191b, borders #2b2e32, text #f1f3f5, muted #b2b8bf, green #2bd391, amber #ffd077, red #ff858a, blue #80aaff.

Keep the shared sidebar unchanged:
LQoSync3 / TERMINAL CONSOLE
1 Dashboard
OPERATIONS
2 Subscribers
3 Routers
4 LibreQoS
5 Workers
MANAGEMENT
6 Settings -- selected charcoal row and slim green marker
7 Journal
ROADMAP
Profiles   planned
WAN plans  planned
Bottom SERVICE ACTIVE, MODE APPLY, TARGET v3_pair.
Topbar Settings at left; CONFIG LOADED | 09 Nov 2026 10:15 +08 | [Dark] at right.

Settings is an editor, not a monitoring dashboard. Under topbar a slim header:
Configuration   /etc/lqosync3/lqosync3.toml
Second slim amber/dim line: Config draft: 0 changes | Add Router form: UNSAVED.
Main area has three rectangular regions from left to right, within the main content:
A narrow Settings groups list (about 13% main width), a Routers list (about 25%), and a large Add Router form (remaining 62%). Use compact consistent monospace text to fit the controls.

Settings groups vertically: Runtime, Router access, Safety, LibreQoS, Network, Nodes, Circuits, Speed, Bandwidth defaults, Backstop caps, Telegram, Interface, Exclusions, Routers. Routers selected with blue/charcoal fill. Long group labels can use a wider panel or wrap neatly without overlap.

Routers list title Routers with count 4. Four rows EDGE-01, EDGE-02, EDGE-03, EDGE-04. Below a separated blue-focused [+ New router] row. Quiet note below "Edit / remove via draft". Do not show an existing router selected while the form is adding a new one.

Form title "Add router — not saved".
Use real-looking terminal text boxes, choice controls and checkboxes:
Name          [EDGE-05                       ]
Address       [192.0.2.105                   ]
API port      [8729]    TLS [x]   Verify certificate [x]
Username      [lqos-reader                   ]
Password      [********                      ]
Password file /etc/lqosync3/secrets/EDGE-05.password
Source reads  [x] PPPoE   [x] DHCP   [ ] Hotspot
Below "[Advanced options]" and a muted note "Router defaults are preserved."
The name text box is visibly focused with blue border or caret. Never show an actual password.
Then one clear primary terminal action [Add to draft] and secondary [Cancel form].
A dim explanatory line "Adds a draft only. No file or router change."

Full-width below the three regions, a compact Review and apply panel:
"Validation: NOT RUN | Preview: NOT RUN | Draft changes: 0"
"[v Validate & preview] (disabled: add to draft first)"
"[A Save & apply] (disabled: current preview required)    [D Discard draft]"
Make disabled actions muted. No fake green validation success. No active Apply button before the draft and preview exist.
A short line "Apply flow: review -> confirm -> backup -> save -> reload -> verify".
No Save action that bypasses preview and explicit confirmation. No Connect/Test connection button and no router operational changes.

Bottom main content compact status line "Write access available | Form not saved | No files changed".
Footer "Tab Next field   Enter Edit/choose   Space Toggle   Esc Cancel   q Quit".
Far right DESIGN PREVIEW | SAMPLE DATA.
Keep the entire form, groups, review panel and footer legible within the same wide image height as the reference. It should clearly demonstrate a usable Add Router workflow, with typed controls, masked secret, sources and staged actions. Do not add unsupported router role or router enabled fields. Produce only the Settings Add Router terminal screenshot.
```

## Edit Values and Review Changes

References, in order: `mockups/settings-add-router-initial-v0.png`, `mockups/workers-proposal-v1.png`.
Output: `mockups/settings-values-proposal-v1.png`.

```text
Create ONE screenshot mockup of LQoSync3 Settings, Edit Values and Review Changes view. Match the attached Settings Add Router image and approved Workers image: same full-screen terminal 3:1 landscape composition, left sidebar, main geometry, compact monospace typography, fine rectangular borders and dark palette. This is a design proposal with sample values, not a running application.
Do not clip any label. Keep all topbar text fully inside the image, with padding at the right. No browser chrome, devices, gradients, shadows, proportional fonts, rounded web cards or oversized metrics.
Palette canvas #0b0c0e, sidebar #141517, panels #17191b, borders #2b2e32, text #f1f3f5, muted #b2b8bf, green #2bd391, amber #ffd077, red #ff858a, blue #80aaff.
Identical sidebar:
LQoSync3 / TERMINAL CONSOLE
1 Dashboard
OPERATIONS
2 Subscribers
3 Routers
4 LibreQoS
5 Workers
MANAGEMENT
6 Settings -- selected charcoal row with narrow green marker
7 Journal
ROADMAP
Profiles   planned
WAN plans  planned
Bottom SERVICE ACTIVE, MODE APPLY, TARGET v3_pair.
Main topbar Settings on left; shorter right text "LOADED | 09 Nov 2026 10:15 +08 | [Dark]" fitting with right padding.

Header row "Configuration  /etc/lqosync3/lqosync3.toml".
Next amber line "Draft: 2 changes | Not saved | Preview current".
Main content three regions: narrow Settings groups list identical to Add Router reference (Runtime selected), large Runtime values editor in middle, and Review changes panel on right. The middle and right panels together use all remaining main width, with enough space for actual key names and typed boxes.
Settings groups list: Runtime (selected), Router access, Safety, LibreQoS, Network, Nodes, Circuits, Speed, Bandwidth defaults, Backstop caps, Telegram, Interface, Exclusions, Routers.

Middle panel title "Runtime values".
Column headings "Key" | "Loaded" | "Draft" | "Unit".
Rows with real schema identifiers:
mode | apply | [apply v] | --
scan_interval_active | 25 | [30] | seconds
scan_interval_idle | 120 | [180] | seconds
error_retry_interval | 30 | [30] | seconds
subscriber_grace | 0 | [0] | seconds
keep_backups | 10 | [10] | backups
log_level | info | [info v] | --
Show only the two changed scan_interval rows in amber. Focus the [30] draft editor in blue. Enum fields use simple terminal dropdown markers, numbers use boxed inputs. All rows must fit without truncating the keys. Keep values aligned.
Below, a separated Field help area:
"scan_interval_active"
"Seconds between scans after a change."
"Changes stay in the draft until Save & apply."
A dim hint "Enter edit | Space toggle | Reset selected value".

Right panel title "Review changes".
Show an actual compact diff with exact text:
- scan_interval_active = 25
+ scan_interval_active = 30
- scan_interval_idle = 120
+ scan_interval_idle = 180
Old lines muted red, new lines green; no actual secrets.
Below a separator:
"Validation  VALID"
"Preview     CURRENT"
"Output      Not needed"
"Reason      Runtime-only changes"
Another note "No shaping-output change in this example."
Do not imply the configuration has already been saved or applied.

Below main panels, full-width Review and apply panel:
"2 draft values changed | Current preview valid | Not saved"
"[v Validate & preview]  [A Save & apply]  [D Discard draft]"
Save & apply is enabled because THIS exact draft already has a current valid preview, but explicit confirmation is still required. Render a clear note "Save & apply opens confirmation. No live change yet."
Another short line "After confirmation: backup -> save -> reload -> verify".
Do not show a green Applied state.

Bottom slim status panel:
"Write access available | Draft differs from loaded config"
"Later edits invalidate this preview. External file changes block save."
Footer "Tab Next field   Enter Edit/choose   v Preview   A Save & apply   Esc Back   q Quit".
Right DESIGN PREVIEW | SAMPLE DATA.
Preserve the approved shell and clear keyboard-driven operational feel. All fields and controls are proposed functional UI requirements, not decorative placeholder buttons. No new unsupported schema key, router operational action, restart button, secret text or production mutation. Output only this single Settings values screenshot.
```

## Final Add Router header correction

References, in order: `mockups/settings-add-router-initial-v0.png`.
Output: `mockups/settings-add-router-proposal-v1.png`.

```text
Edit only the top-right header of this Settings Add Router terminal screenshot. Its theme label is clipped at the right edge. Replace the top-right text with the shorter exact string "LOADED | 09 Nov 2026 10:15 +08 | [Dark]" and place the complete string fully inside the frame with visible right padding, matching the Settings values view. Preserve every other element exactly: sidebar, configuration path, groups, router list, form fields and masked password, source toggles, draft counters, disabled preview/apply controls, footer, colors, panel geometry, font and sample-data label. No other redesign or content change. Keep the same wide 3:1 full-screen terminal composition. Output the complete corrected screenshot.
```
