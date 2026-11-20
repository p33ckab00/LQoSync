# Physical NIC faceplate

Date: 2026-11-20 (Asia/Shanghai).
Status: implementation. The owner supplied the layout reference.

## Layout

The NIC ports card belongs to the Dashboard service summary, beside PPPoE and LibreQoS.
It appears before the traffic charts. DPDK diagnostics keep the raw counters.
Wide screens show all six summary cards in one row.

Smaller screens use three or two columns. The NIC card spans both columns below 1201px.
The ports remain adjacent without individual card borders.
Each port uses this order:

1. WAN, LAN, WAN/LAN, or Role unknown.
2. Reported link speed and DPDK port number.
3. Physical connector SVG with a steady link indicator.
4. Independent RX and TX activity LEDs directly below the SVG.
5. PCI address, or the reported device name when PCI identity is unavailable.

RJ45, SFP-family, and QSFP-family ports use the [owner-supplied SVG kit](evidence/nic-svg-kit/README.md).
The artwork retains the supplied metallic gradients. Unknown connectors keep the generic fallback.
SFP/SFP+/SFP28 share their cage geometry. QSFP/QSFP+/QSFP28 share their cage geometry.
The accessible description retains the exact reported type and confidence.
Hover opens the glass diagnostics panel after 180 ms.
Enter, Space, or tap opens a persistent panel. Escape or Close dismisses it.
The panel shows identity and confidence. The port retains an accessible identity label.

The current diagnostics keep raw NIC counters.

## Data ownership

The core collector supplies physical identity in `nic-telemetry.json`.
The web broker projects allowlisted fields from this cache.
The gateway and browser do not query sysfs, run ethtool, or open DPDK sockets.
The browser contract contains typed activity counters and the sample interval.
The TUI continues to use its current presentation.

| Information | Source and fallback |
| --- | --- |
| Port number | DPDK port ID; never treated as WAN/LAN evidence |
| PCI address | Valid PCI BDF from the DPDK device name |
| Linux interface | Netdev mapping under that PCI device in sysfs |
| WAN/LAN | Explicit `bridge.to_internet` / `bridge.to_network` from the configured LibreQoS TOML file |
| Link and speed | Current DPDK link telemetry |
| Connector | DPDK module identifier, then a bounded kernel module query |
| Native copper fallback | Kernel twisted-pair media, shown as inferred RJ45 |
| Activity | Valid packet or byte counter deltas from the existing collector |

The collector reads the configured `libreqos.v3_config` path.
The role can match a verified netdev name or an explicit PCI identifier.
If both configured roles match, the label is WAN/LAN.
If the mapping is absent or ambiguous, the label is Role unknown.
A vfio-bound NIC may have no Linux netdev. Module telemetry remains the preferred source.
Multiple netdevs on one PCI device do not establish a unique interface match.

The optional DPDK module query has a shared maximum budget of 300 ms.
An unavailable or unsupported module endpoint does not invalidate the counter snapshot.
Kernel ethtool queries use a fixed executable and validated interface names.
Each query has a 250-ms timeout and a 16-KiB output limit.
The whole identity enrichment has a two-second limit.
The collector resolves basic identity for all ports before optional kernel module queries.
All queries are read-only. The collector does not bind, reset, or configure a NIC.

## Confidence

- Confirmed: an explicit module identifier supplies the connector family or variant.
- Inferred: kernel media reports twisted pair, but cage identity is not verified.
- Unknown: no supported evidence exists. The UI uses a generic connector.

A reported 10G speed does not prove SFP+.
An SFP module identifier alone does not prove SFP+ or SFP28 cage capability.
The detector does not invent a faster variant from the negotiated speed.
The broker rejects unsupported type and confidence labels.
Module serial numbers, EEPROM contents, credentials, and complete configuration do not enter the browser contract.

## Link and activity

| Condition | Link indicator | RX / TX LEDs |
| --- | --- | --- |
| Fresh link up | Steady green | Off when idle |
| Fresh link down | Steady red | Off |
| Fresh RX traffic | Steady green | RX blue pulse |
| Fresh TX traffic | Steady green | TX amber pulse |
| Fresh duplex traffic | Steady green | Both LEDs pulse |
| Unknown or stale telemetry | Gray | Off |
| Counter reset | Link state remains independent | Activity off |

Each pulse lasts 180 ms and animates opacity.
A pulse occurs once per fresh, distinct completed sample.
The initial sample establishes a baseline. Duplicate overview frames do not replay activity.
The sample tracker waits for the server-time freshness check before it consumes a new timestamp.
This prevents a clock-anchor update from suppressing a valid pulse.
A replacement port gets a new component identity.

The NIC collector targets one counter sample each second.
The broker reads NIC evidence during the same one-second refresh as aggregate and classification traffic.
Hardware identity refreshes every thirty seconds. Failed counter reads retry after ten seconds.
The LEDs therefore report sampled interval activity, not individual packets or subsecond traffic timing.
Do not use an endless animation to imply continuous traffic.
Reduced Motion uses a static color for activity in the latest valid interval.
The tooltip supplies packet totals, byte totals, and the interval.
Stale, down, and reset states suppress activity in both motion modes.

## Validation and deployment

See [hardware evidence](evidence/nic-faceplate/README.md) and [summary placement evidence](evidence/nic-summary/README.md).
See [the original atomic plan](plans/2026-11-20-nic-faceplate.md) and [the live HUD plan](plans/2026-11-21-nic-live-hud.md).
See [live diagnostics](NIC_LIVE_DIAGNOSTICS.md) for the current panel and cadence contract.
The design uses the current Vercel frontend-design and Google modern-web-guidance workflows.

Full hardware detection needs the updated core collector and web packages.
The summary placement update changes only the web UI.
It shares the existing snapshot and server-time clock with DPDK diagnostics.
See [deployment](DEPLOYMENT.md#nic-faceplate-update) for the install sequence.
An older collector still supplies link, speed, PCI-name fallback, and existing activity counters.
Its absent hardware metadata remains unknown until the core update.

Primary references:

- [DPDK telemetry interface](https://doc.dpdk.org/guides/howto/telemetry.html).
- [DPDK module EEPROM test and Identifier example](https://doc.dpdk.org/dts/test_plans/telemetry_test_plan.html).
- [DPDK module information and kernel EEPROM interfaces](https://doc.dpdk.org/guides/prog_guide/ethdev/ethdev.html).

The implementation checks endpoint availability at runtime.
It does not assume that an installed driver supports every documented query.
