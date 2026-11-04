# Rust backend options — 2026-11-04 review

The [video supplied by the owner](https://youtu.be/fjFauqkFm3g) compares Axum, Actix Web, Rocket, and Loco. Its title and description were available; its auto-generated transcript did not return content during this review. The comparison below is checked against the frameworks' own documentation. These are **server/API** frameworks. They do not by themselves provide the browser-side Rust/Wasm UI described in [Rust frontend options](RUST_FRONTEND_OPTIONS.md).

| Backend | Relevant strengths | LQoSync4 fit and cost |
|---|---|---|
| [Axum](https://docs.rs/axum/latest/axum/) | Tokio/Tower ecosystem; typed extractors; built-in WebSocket feature. | Strong default for a separate Rust API and live status stream. It is also an officially supported Leptos server integration. Keep collectors and reconciliation outside request handlers. |
| [Actix Web](https://actix.rs/docs/) | Mature async HTTP server and Rust service framework. | Viable alternative if its API and operations model benchmark better on representative workloads. Switching solely for advertised throughput is not justified. |
| [Rocket](https://rocket.rs/) | Ergonomic route attributes, typed request guards, responders, configuration and async handlers. | Viable API server with a separate Rust/Wasm frontend. Useful if its guards and route ergonomics improve clarity. Verify the exact streaming, WebSocket, middleware and deployment needs in a small spike before choosing it. |
| [Loco](https://loco.rs/docs/) | Rails-inspired, batteries-included framework on Axum with models, auth, jobs, and migrations. | Useful if most built-in application conventions are needed. Adds a framework layer to a port that already has an existing core and state model. |

## Provisional recommendation

Keep **Axum** as the first backend spike because it pairs directly with the provisional Leptos option and exposes WebSocket support. Test Rocket alongside it only if request guards, routing ergonomics, or deployment needs materially favor Rocket. The backend choice must not change the LQoSync3 collector and LibreQoS output semantics. Compare p50/p95 API latency, idle and peak memory, live-update behavior, error handling, and implementation clarity on the same fixture. Framework choice alone does not establish end-to-end performance.

Sources: [Axum WebSocket API](https://docs.rs/axum/latest/axum/extract/ws/), [Leptos server integrations](https://book.leptos.dev/ssr/index.html), [Actix Web](https://actix.rs/docs/whatis), [Rocket overview](https://rocket.rs/guide/v0.5/overview/), [Rocket API](https://docs.rs/rocket/latest/rocket/), [Loco docs](https://loco.rs/docs/).
