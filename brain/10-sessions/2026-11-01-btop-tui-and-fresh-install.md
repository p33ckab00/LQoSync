---
type: session
created: 2026-11-01
tags: [session, tui, libreqos, docs]
status: done
related: ["[[HANDOFF]]", "[[ADR-0008-terminal-ui]]", "[[ADR-0010-first-full-libreqos-run]]", "[[2026-11-01-tui-cosmetics-implementation]]"]
---

# btop TUI, review fixes, and a new LibreQoS installation

Timestamp: 2026-11-01T08:26:04+00:00.

## Goal

The operator found that the first cosmetics result was not like btop++. Claude Code took over from Codex, reviewed the cosmetics changes, and improved the TUI.
The operator then asked for clear installation and TLS guides, and for support of a new LibreQoS installation.

## Changes

- Dark theme: the colors of the btop++ Default theme. Light keeps darker scales with a contrast test.
- Each panel is a btop box: a border color for each kind, and a `┐name┌` title. The focused title has the selection colors.
- One header line when it fits. A tab strip with superscript hotkeys.
- Dashboard: filled braille area graphs and `■` meters with the btop color scales.
- New key `tui.graph_samples` (10 to 3600, default 120).
- Review fixes: Light scales, compact device detail with IPv6, deep Network nodes, short rate form, footer hints with no lost key, and a u16 overflow.
- Reload: a full LibreQoS run before the first full run. Refer to [[ADR-0010-first-full-libreqos-run]].
- `lqosync3 check`: a warning for a missing `libreqos.state_directory`.
- Docs: "Prepare each router" and the value table in INSTALLATION, "New LibreQoS installation", and "Router TLS with your own CA" in OPERATIONS.
- `telegram.notify_on_preflight_warning` has no effect. The old specs read it as a v3 key, which was wrong.

## Decisions of the operator

- No btop net box for the host, and no preflight card on the Dashboard.
- No preflight list in Telegram messages.
- Approved: one process chart with CPU, RSS, I/O, and network lines. I/O and network come from systemd accounting of the service.

## Checks

- Each step: `cargo test` (437 library tests), clippy with `-D warnings`, and fmt.
- Sonnet reviews: the cosmetics range, the review fixes, and the full-run change (FAIL, then PASS after the fix).
- The operator installed the release build. The live binary has the same SHA-256 as the build.

## Limits and next action

- A human must still examine the TUI in the operator terminal.
- The router TLS and user commands were not tested on a real RouterOS router.
- The full-run path was not tested on a new LibreQoS. The tests use a fake `LibreQoS.py`.
- Next action: build the approved process chart.
