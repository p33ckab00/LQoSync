# LQoSync4 handoff

## Owner decisions

- Port LQoSync3 without its TUI. Build the web UI and its required backend.
- Use the LQoSync4 web application as the product baseline. Reuse and enhance the existing LQoSync3 synchronization core within it.
- Follow the documented functional, visual, placement, and styling requirements for LQoSync4.
- MikroTik core owns routing and session behavior after failover. A public-IP change or interrupted active session is acceptable.
- The local `lqosync4` branch is already checked out. Do not create another clone for planning.
- Future roadmap: destination IP, ASN, organization, CDN, and possible app tracking.
- Use Rust for backend and frontend application logic. Use Leptos for the WebAssembly browser UI; measure its Dashboard and table slice.

## State

The LQoSync4 web application is still in planning; no HTTP API, database, auth service, Leptos browser app, deployment, or network change has been made. The local `/opt/lqosync4` checkout tracks the public `lqosync4` branch. The ported LQoSync3 core reads RouterOS and writes LibreQoS input; a native apply guard is committed on this branch. Router writes are new scope for LQoSync4.

AI coding agents enter through root [AGENTS.md](../../../AGENTS.md), then use the [tandem workflow](AGENT_WORKFLOW.md), [engineering rules](AGENTIC_ENGINEERING_RULES.md), and [completion gate](PRODUCTION_READY_GATE.md). Graphify is a planning map; verify implementation claims in source.

## Next work

Use the [delivery roadmap](../ROADMAP.md) as the agent task index. It links [the feature and function map](../docs/FEATURE_FUNCTION_MAP.md), [route and component contract](../docs/ROUTES_AND_COMPONENTS.md), [visual/responsive map](../docs/UI_VISUAL_SYSTEM.md), [CSS starter](../ui-reference.css), [workflows](../docs/ARCHITECTURE_WORKFLOWS.md), [STE answers](../ste/WH_AND_HOW.md), and [improvement list](../docs/IMPROVEMENTS.md). Resolve the listed owner decisions before implementing router writes. Start with baseline evidence and shadow mode.

E01 now has a [source and fixture baseline](../roadmap/E01_BASELINE.md) pinned to `f358e9b5b4b64ba59919caa7b54d808b6345f7db`. Reuse the existing synthetic scenarios for Phase 1 parity checks. The native v3 fixture records a skipped static circuit; native apply now rejects any skipped circuit and reports the alert. An operator must set a valid Parent Node for that static circuit before apply.

E03 now has a [synthetic full-cycle baseline](../roadmap/E03_BASELINE.md) and repeatable Rust harness. It measures 1k/10k subscribers in shadow mode across all three output targets and records the v3 pair interruption and native acceptance limits. These are local, zero-network measurements, not a production capacity budget. C01 can use them while porting the core without TUI; C02 must measure representative hardware and topology before setting performance limits. Preserve zero unchanged shaping writes and the native apply gate.

E02 has a [first-slice web contract and G0 recommendation](../roadmap/E02_WEB_CONTRACT.md): four read endpoints, durable snapshot/diagnostic versions, error and stale states, role proposal, and W01/W02 ownership. The proposed Axum/SQLite/local-session stack is **not owner-approved**. D-12 stays open; obtain the owner's framework, store, auth/MFA, and operator apply decisions before W01 persistence. No API, auth service, database, or Leptos app has been implemented.

The [missing-parent web alert contract](../docs/MISSING_PARENT_ALERT.md) assigns the typed DTO and last-known state to W01, then native diagnostic production and the Dashboard and `/libreqos` views to W06. It specifies circuit/row counts, stale and blocked states, placement, remediation, and browser checks. This is a planned UI/API requirement; the current branch has no Leptos app or HTTP API.
