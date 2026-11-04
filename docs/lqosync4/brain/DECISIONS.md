# Decision log

| ID | Decision | Basis | State |
|---|---|---|---|
| D-01 | Port LQoSync3 core and remove TUI. | Owner | Accepted |
| D-02 | Build the complete web UI and required backend. | Owner | Accepted |
| D-03 | MikroTik core owns routing and session behavior. | Owner | Accepted |
| D-04 | Separate LibreQoS shaping state from WAN assignment state. | Proposal | Validate |
| D-05 | Start router-changing behavior in shadow/dry-run mode. | Proposal | Validate |
| D-06 | Show evidence and confidence for IP-to-app classification. | Proposal | Roadmap |
| D-07 | Use Rust for backend and browser-side UI; compile the UI to WebAssembly. | Owner | Accepted |
| D-08 | Use Leptos for the Rust/Wasm frontend; start with CSR and measure the Dashboard and table slice. | Owner | Accepted |
| D-09 | Select the Rust backend framework after an API and live-update spike; Rocket is a candidate. | Proposal | Open |
| D-10 | Use agent-tandem coordination, Graphify source checks, shared brain handoffs, and the production completion gate for AI coding work. | Owner | Accepted |
| D-11 | Use the LQoSync4 web application as the product baseline; reuse and enhance LQoSync3 code as its TUI-free synchronization core. | Owner | Accepted |

## Open decisions

Backend HTTP framework; persistent store; auth; RouterOS write mechanism; upstream health source and thresholds; manual-pin behavior on outage; failback policy; destination-traffic source and retention.
