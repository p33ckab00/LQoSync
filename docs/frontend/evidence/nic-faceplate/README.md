# Physical NIC faceplate evidence

Date: 2026-11-20 (Asia/Shanghai). Branch: `lqosync3`.
See [the feature contract](../../NIC_FACEPLATE.md).

## Pictures

The pictures use synthetic fixture data and Reduced Motion mode.
The static RX/TX colors show activity in the latest valid interval.
The live pulse behavior has separate browser assertions.
No production subscriber records, module serials, or account credentials appear.

- [Dark](dark.png)
- [Light](light.png)
- [Mobile, 320px](mobile.png)

Run `sha256sum -c SHA256SUMS` in this directory to check the pictures.

## Checks

Fifteen core NIC tests passed.
They cover connector identifiers, explicit roles, optional module failure, serial-field exclusion, cache validity, counters, resets, and bounded transport.
The related worker suite passed 65 tests, with one existing ignored test.
Nine DPDK projection tests and ten overview tests passed in the web suite.
The new projection test checks allowlisted identity, PCI fallback, valid intervals, and reset suppression.
Eight native UI tests passed.
The activity test covers duplicate timestamps, separate RX/TX, stale/down/reset states, and source-clock ordering.

The physical-port browser check passed these cases:

- Adjacent ports without individual card borders.
- WAN/LAN, speed, port number, PCI, connector family, and confidence labels.
- RX/TX LEDs directly below the artwork.
- RX-only and TX-only pulses on distinct samples.
- No replay, idle, down, reset, or stale pulses.
- Stable port DOM across ordinary updates.
- Generic connector and unknown-role fallback.
- SFP, SFP+, SFP28, QSFP, QSFP+, QSFP28, and RJ45 artwork selection.
- Light/Dark, 320px layout, keyboard access, Reduced Motion, and forced colors.

The existing DPDK, aggregate traffic, sign-in/Dashboard, and ticker checks passed.
The checks ran against an isolated loopback fixture with synthetic credentials.
The native core, native web packages, and Rust/Wasm release builds passed.
Native/Wasm Clippy and Rust 1.89 checks passed.
Formatting, whitespace, and local documentation links passed.

Vercel frontend-design and Google modern-web-guidance supplied interface guidance.
The Google motion guide recommends opacity animation and Reduced Motion support.
The optional skill inspector scripts are absent here.
Playwright and the installed Chromium supplied screenshots and browser assertions.
The screenshots also received a visual review.
The STE checker is heuristic and does not establish certified compliance.

## Runtime limits

The release core and web artifacts are prepared.
Production binaries, services, NIC bindings, and routing did not change.
The TUI presentation and CLI account policy did not change.
Follow [the update steps](../../DEPLOYMENT.md#nic-faceplate-update) to install the artifacts.

A read-only host check found PCI-named ports and Linux netdev mappings.
The configured bridge interfaces supply explicit WAN/LAN references.
The current user could not access the privileged DPDK module endpoint without a sudo password.
Therefore, these checks do not establish this host's physical connector type.
The installed core collector must detect it with its normal service permissions.
Unknown evidence remains generic.

The collector uses ten-second NIC samples and the broker's five-second full projection.
A 180-ms LED pulse reports traffic in a completed interval.
It does not provide per-packet or subsecond timing.
