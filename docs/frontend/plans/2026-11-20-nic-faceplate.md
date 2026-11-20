# Physical NIC faceplate

Status: implemented. Date: 2026-11-20 (Asia/Shanghai).
See [validation evidence](../evidence/nic-faceplate/README.md).

Use the owner's compact faceplate reference inside the current NIC section.
Keep the current font, themes, and Dashboard layout.
Use adjacent SVG ports without individual card borders.
Place role, speed/port number, port artwork, RX/TX LEDs, and PCI identity in that order.

1. Add optional hardware identity to the existing collector cache.
2. Resolve PCI and netdev identity, then read explicit LibreQoS bridge roles.
3. Detect module families from DPDK EEPROM or a bounded kernel query.
4. Confirm native RJ45 only from explicit media evidence. Preserve unknown and inferred states.
5. Add typed packet/byte deltas and sample timing to the browser contract.
6. Render keyed port components with independent link and activity states.
7. Pulse activity LEDs once per fresh distinct sample. Stop pulses on stale/down/reset states.
8. Check core/projection logic, browser states, themes, mobile layout, and reduced motion.
9. Save evidence and deployment instructions, then commit and push.

The link state stays green for up, red for down, and gray for unknown/stale.
RX uses blue. TX uses amber. Idle activity LEDs remain off.
A 180-ms pulse indicates traffic in the completed telemetry interval.
Do not simulate packets or repeat pulses on duplicate overview frames.
Reduced motion uses static activity indications with text.
Do not infer roles from port order or media from negotiated speed alone.
Do not expose module serial numbers, credentials, or complete LibreQoS configuration.
