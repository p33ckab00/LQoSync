# LQoSync4 handoff

## Owner decisions

- Port LQoSync3 without its TUI. Build the web UI and the backend required by all supplied screenshots.
- Use screenshots as functional and visual references, including placement and styling. NetCore source code is not available.
- MikroTik core owns routing and session behavior after failover. A public-IP change or interrupted active session is acceptable.
- Do not clone the future LQoSync4 branch yet.
- Future roadmap: destination IP, ASN, organization, CDN, and possible app tracking.

## State

Planning only. No LQoSync4 application code, checkout, deployment, or network changes. The public LQoSync3 branch was inspected read-only. Its current tool reads RouterOS and writes LibreQoS input; router writes are new scope for LQoSync4.

## Next work

Confirm remaining features, target branch/path, hosting, authentication, telemetry source, and the exact MikroTik core write contract. Then execute [the plan](../docs/IMPLEMENTATION_PLAN.md) starting in shadow mode.
