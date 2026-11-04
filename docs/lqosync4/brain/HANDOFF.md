# LQoSync4 handoff

## Owner decisions

- Port LQoSync3 without its TUI. Build the web UI and the backend required by all supplied screenshots.
- Use screenshots as functional and visual references, including placement and styling. NetCore source code is not available.
- MikroTik core owns routing and session behavior after failover. A public-IP change or interrupted active session is acceptable.
- Do not clone the future LQoSync4 branch yet.
- Future roadmap: destination IP, ASN, organization, CDN, and possible app tracking.
- Use Rust for backend and frontend application logic. Use Leptos for the WebAssembly browser UI; measure its Dashboard and table slice.

## State

Planning only. The local `/opt/lqosync4` checkout tracks the public `lqosync4` branch; no LQoSync4 application code, deployment, or network changes have been made. The LQoSync3 core reads RouterOS and writes LibreQoS input; router writes are new scope for LQoSync4.

## Next work

Confirm remaining features, hosting, authentication, telemetry source, backend framework, and the exact MikroTik core write contract. Then execute [the plan](../docs/IMPLEMENTATION_PLAN.md) starting in shadow mode.
