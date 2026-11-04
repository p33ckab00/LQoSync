# LQoSync4 handoff

## Owner decisions

- Port LQoSync3 without its TUI. Build the web UI and its required backend.
- Follow the documented functional, visual, placement, and styling requirements for LQoSync4.
- MikroTik core owns routing and session behavior after failover. A public-IP change or interrupted active session is acceptable.
- The local `lqosync4` branch is already checked out. Do not create another clone for planning.
- Future roadmap: destination IP, ASN, organization, CDN, and possible app tracking.
- Use Rust for backend and frontend application logic. Use Leptos for the WebAssembly browser UI; measure its Dashboard and table slice.

## State

Planning only. The local `/opt/lqosync4` checkout tracks the public `lqosync4` branch; no LQoSync4 application code, deployment, or network changes have been made. The LQoSync3 core reads RouterOS and writes LibreQoS input; router writes are new scope for LQoSync4.

AI coding agents enter through root [AGENTS.md](../../../AGENTS.md), then use the [tandem workflow](AGENT_WORKFLOW.md), [engineering rules](AGENTIC_ENGINEERING_RULES.md), and [completion gate](PRODUCTION_READY_GATE.md). Graphify is a planning map; verify implementation claims in source.

## Next work

Use [the feature and function map](../docs/FEATURE_FUNCTION_MAP.md), [route and component contract](../docs/ROUTES_AND_COMPONENTS.md), [visual/responsive map](../docs/UI_VISUAL_SYSTEM.md), [CSS starter](../ui-reference.css), [workflows](../docs/ARCHITECTURE_WORKFLOWS.md), [STE answers](../ste/WH_AND_HOW.md), and [improvement list](../docs/IMPROVEMENTS.md) as the current design map. Resolve the listed owner decisions before implementing router writes. Then execute [the plan](../docs/IMPLEMENTATION_PLAN.md) starting in shadow mode.
