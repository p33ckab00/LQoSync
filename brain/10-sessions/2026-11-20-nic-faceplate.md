---
type: session
created: 2026-11-20
tags: [frontend, dpdk, nic, hardware]
status: implemented
related: ["[[2026-11-20-dpdk-dashboard]]", "[[2026-11-20-aggregate-traffic]]"]
---

# Physical NIC faceplate

The owner supplied a compact physical-port reference.
The NIC section now uses adjacent SVG ports without individual card borders.
Each tile shows role, speed/port number, connector, RX/TX LEDs, and PCI address.
Vercel frontend-design and Google modern-web-guidance supplied interface guidance.

The core collector adds optional hardware identity to the current NIC cache.
DPDK module identifiers supply confirmed connector information.
Bounded kernel queries provide optional module or inferred copper-media evidence.
The detector does not infer SFP+ from a 10G negotiated speed.
Unknown hardware keeps a generic connector.

WAN/LAN comes from explicit LibreQoS bridge interface configuration and verified device identity.
Multiple netdevs on one PCI device do not establish a unique interface match.
Basic identity for all ports precedes optional driver queries.
The web services continue to read only the published cache.
Module serials and full configuration do not enter the browser contract.

Link state is independent from activity.
Green means up, red means down, and gray means unknown/stale.
Fresh distinct NIC samples trigger separate 180-ms RX/TX pulses.
Duplicate, idle, down, reset, and stale frames do not replay pulses.
The tracker waits for the server clock anchor before it consumes a new sample.
Reduced Motion shows static activity indications.

The source cadence remains ten seconds, with five-second full web projections.
LEDs show completed-interval activity, not individual packets.
The initial sample establishes a baseline.
Longer raw counters remain in diagnostics.

Native core/worker/projection/UI checks and browser regressions passed.
See [the feature contract](../../docs/frontend/NIC_FACEPLATE.md).
See [pictures and validation](../../docs/frontend/evidence/nic-faceplate/README.md).

The release core and web artifacts are prepared.
The operator must install both components for full hardware identity.
Production files/services, NIC bindings, TUI presentation, and CLI accounts did not change.
A privileged live module probe was unavailable without a sudo password.
Fixture checks do not establish the host's actual connector type.
