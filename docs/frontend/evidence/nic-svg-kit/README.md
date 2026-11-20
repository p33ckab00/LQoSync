# Supplied NIC SVG artwork

Date: 2026-11-21 (Asia/Shanghai). Branch: `lqosync3`.

The owner supplied [nic-ports.svg](../../images/nic-ports.svg) and [nic-svg-kit.zip](../../images/nic-svg-kit.zip).
The SVG in the archive matches the standalone file byte for byte.
The source files remain unchanged.

The UI uses the supplied RJ45, SFP, and QSFP connector geometry and metallic gradients.
The build embeds one static fragment from `crates/web-ui/src/nic_art.svg`.
Each port selects its matching symbol with SVG `use`.
Symbol and gradient IDs have the `lqosync-nic-` prefix to avoid collisions.

The fragment contains only definitions and connector symbols.
It excludes the preview background, demo instances, demo styles, and unused glow filter.
The current telemetry controls link colors and finite RX/TX pulses.
The current generic connector remains the fallback for unknown hardware.
The live diagnostics panel and one-second update target remain unchanged.
Vercel frontend-design and Google modern-web-guidance supplied the icon integration guidance.

## Pictures

The pictures use synthetic loopback data and Reduced Motion mode.

- [Dark summary](dark.png)
- [Light summary](light.png)
- [Mobile summary, 320px](mobile.png)
- [Wide summary, 1920px](wide.png)

Run `sha256sum -c SHA256SUMS` in this directory.

## Source hashes

- SVG: `1a29846af6402c3ddf0609a6cb9ef81febf1ff33db0e525535e791fa8e3d7cc1`
- ZIP: `8b7736dd89d87403dfecd5d78f8fbaaf1ef664f50b5212c05e934a972b8674f7`

## Validation and install

The physical-port check verifies unique IDs, symbol selection, connector families, link/activity behavior, themes, keyboard access, and responsive widths.
The diagnostics browser check verifies that the hover/tap panel still works.
The session note records final build and check results.

This update needs only the web artifacts.
Run `sudo tools/frontend/install.sh` from `/home/pi/LQoSync3.0`, then refresh the browser.
The earlier core update remains necessary for the one-second NIC sampling and hardware detection.
Production services do not change during implementation checks.
