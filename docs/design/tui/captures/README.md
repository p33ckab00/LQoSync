# TUI implementation captures

Date: 2026-11-09. All data is synthetic and isolated from production.
The [visual contract](../README.md) links the unchanged approved concepts.

| Screen | Implementation image | Capture type |
|---|---|---|
| Dashboard | [Dashboard](dashboard-v1.png) | Running Zutty terminal, Xvfb |
| Subscribers | [Subscribers](subscribers-v1.png) | Running terminal |
| Routers | [Routers](routers-v1.png) | Running terminal |
| LibreQoS | [LibreQoS](libreqos-v1.png) | Running terminal |
| Workers | [Workers](workers-v1.png) | Running terminal |
| Settings | [Read-only](settings-readonly-terminal-v1.png) | Running terminal |
| Settings Add Router | [Form](settings-add-router-v1.png) | Actual Ratatui fixture frame, rasterized |
| Settings values | [Values and review](settings-values-review-v1.png) | Actual Ratatui fixture frame, rasterized |
| Journal | [Journal](journal-v1.png) | Running terminal, isolated log provider |

Running-terminal images use 240x40 cells, the 9x18 terminal font, and the dark theme.
Reference concept images are 2172x724 pixels. Actual framebuffer images are 2164x724 pixels.
Fixture renders are 2160x720 pixels. Font and border differences prevent pixel identity.
The panel order, shared roles, and selected-record relationships follow the approved concepts.
Unavailable values retain their evidence-based labels instead of copying sample values.

## Reproduce

Build with `cargo build --locked`.
Run `python3 scripts/dev/capture_tui.py --screen 7 --output /tmp/journal.png` from the repository root.
Choose screen numbers 1 through 7. The script creates temporary config, data, and credentials.
For Journal, a temporary PATH supplies a synthetic log provider. The script never reads the host journal.
Requirements: Xvfb, Zutty, libX11, libXtst, and Pillow. No production service is started or restarted.

For writable Settings, set `LQOSYNC_RENDER_DIR` to a temporary output directory.
Run `cargo test --locked export_settings_fixture_frames -- --ignored`.
Rasterize the exported JSON with `python3 scripts/dev/render_terminal_frame.py FRAME.json OUTPUT.png`.
This runs the real renderer with a controlled writable App fixture. It is not a privileged running-terminal session.
Sudo requires an operator password on this host. No production configuration was applied during capture.

The final 63-frame matrix is stored outside the repository in
`/home/pi/.local/state/lqosync3/tui-complete-review-20261109`.
Use `export_dashboard_frames` with the same environment variable to reproduce it.
