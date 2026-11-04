# Rust frontend decision — 2026-11-04 review

## Constraint

The owner requires Rust backend and Rust frontend. Browser-side application logic is compiled to WebAssembly. CSS and browser/wasm glue are still necessary for a web app; the constraint excludes hand-written JavaScript/TypeScript application code. Chart, table, and browser APIs must be checked for Rust/Wasm support before selecting UI dependencies.

This page compares **browser UI** frameworks. Rocket, Axum, Actix Web, and Loco are server/API choices; see [Rust backend options](RUST_BACKEND_OPTIONS.md).

## Candidates

| Option | Fit for LQoSync4 | Main tradeoff |
|---|---|---|
| [Leptos](https://book.leptos.dev/) | Fine-grained reactive Rust UI; CSR, SSR, hydration and Axum integration. Strong fit for live dashboards and independently updating cells. | More framework-specific reactive patterns; SSR adds complexity that a private operations dashboard may not need. |
| [Dioxus](https://dioxuslabs.com/learn/0.7/) | Rust full-stack framework with web, desktop and mobile targets; Axum integration and server functions. Good if desktop/mobile clients are planned. | Cross-platform and full-stack tooling add surface area when the requirement is only a browser UI. |
| [Yew](https://yew.rs/docs/) | Established component-based Rust/Wasm UI; works with Trunk and a separate Rust API. Good conventional SPA choice. | Component update model needs careful profiling for dense, rapidly changing tables. |
| [Sycamore](https://sycamore.dev/) | Fine-grained reactivity, SSR and SPA support. Small, focused candidate. | Evaluate ecosystem and operational tooling against the concrete screens before choosing it. |

## Owner decision: Leptos

Use **Leptos** for the browser UI. Start with client-side rendering (CSR) for this private operations dashboard; SSR and hydration are not in initial scope. Use one shared Rust DTO crate, keep collector and RouterOS writer server-only, and build the dashboard, an allocation card, and a paged 1,000-row subscriber table. The backend framework is a separate decision. Do not infer total application speed from framework marketing benchmarks.

Measure transferred Wasm bytes, cold first interaction, update latency for a changing status card, memory, table scroll/search performance, build time, and the amount of browser API glue. Use the measurements to tune the Leptos implementation. Pin a stable release at implementation time; do not depend on a release candidate by default.

## Sources

- [Leptos API modes and features](https://docs.rs/leptos/latest/leptos/), [fine-grained reactivity](https://book.leptos.dev/reactivity/index.html), [Axum integration](https://book.leptos.dev/ssr/index.html).
- [Dioxus full-stack and Axum integration](https://dioxuslabs.com/learn/0.7/essentials/fullstack/), [platform targets](https://dioxuslabs.com/learn/0.7/guides/platforms/).
- [Yew overview](https://yew.rs/), [Trunk setup](https://yew.rs/docs/next/getting-started/introduction).
- [Sycamore overview](https://sycamore.dev/).
