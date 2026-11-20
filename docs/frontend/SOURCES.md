# Sources and provenance

Research date: 2026-11-20 (Asia/Shanghai). Code baseline: `9a465e7` on `lqosync3`.
This directory contains documentation and copied images. It does not contain an LQoSync4 checkout.

## LQoSync4 blueprint

Reviewed revision: `b5284bd9fc4c3e8d7a35b8c6f24e2e4faf8709e8`.

- [Roadmap](https://github.com/p33ckab00/LQoSync/blob/b5284bd9fc4c3e8d7a35b8c6f24e2e4faf8709e8/docs/lqosync4/ROADMAP.md).

- [Product specification](https://github.com/p33ckab00/LQoSync/blob/b5284bd9fc4c3e8d7a35b8c6f24e2e4faf8709e8/docs/lqosync4/docs/PRODUCT_SPEC.md).

- [Routes and components](https://github.com/p33ckab00/LQoSync/blob/b5284bd9fc4c3e8d7a35b8c6f24e2e4faf8709e8/docs/lqosync4/docs/ROUTES_AND_COMPONENTS.md).

- [Visual system](https://github.com/p33ckab00/LQoSync/blob/b5284bd9fc4c3e8d7a35b8c6f24e2e4faf8709e8/docs/lqosync4/docs/UI_VISUAL_SYSTEM.md).

- [Rust frontend decision](https://github.com/p33ckab00/LQoSync/blob/b5284bd9fc4c3e8d7a35b8c6f24e2e4faf8709e8/docs/lqosync4/docs/RUST_FRONTEND_OPTIONS.md).

- [Feature map](https://github.com/p33ckab00/LQoSync/blob/b5284bd9fc4c3e8d7a35b8c6f24e2e4faf8709e8/docs/lqosync4/docs/FEATURE_FUNCTION_MAP.md).

- [CSS reference](https://github.com/p33ckab00/LQoSync/blob/b5284bd9fc4c3e8d7a35b8c6f24e2e4faf8709e8/docs/lqosync4/ui-reference.css).

The blueprint selected Rust and Leptos CSR. Its broader product removes the TUI and adds router-changing operations.
This plan preserves the LQoSync3 backend and TUI under the current request.

## Requested design skills

- [Vercel frontend-design](https://github.com/vercel-labs/open-agents/blob/main/.agents/skills/frontend-design/SKILL.md): aesthetic direction and composition.

- [Vercel Agent Skills](https://github.com/vercel-labs/agent-skills): skill catalog.

- [Vercel web-design-guidelines](https://github.com/vercel-labs/agent-skills/blob/main/skills/web-design-guidelines/SKILL.md): review workflow.

- [Vercel interface rules](https://github.com/vercel-labs/web-interface-guidelines/blob/main/command.md): accessibility, focus, forms, navigation, and performance checks.

- [Google Modern Web Guidance](https://github.com/GoogleChrome/modern-web-guidance/blob/64a6d708f5a43f2cd3fde5d4d7db3905ba1bdf65/skills/modern-web-guidance/SKILL.md): platform patterns and browser support.

Read Google guides `responsive-table`, `navigation-drawer`, and `dialog` through its CLI.
They informed design proposals. This research does not establish implementation compliance.
The React/Next.js skills do not change the selected Rust framework.

Material Design 3 and Google Stitch are not the requested Google skill.

## Framework sources

- [Leptos CSR](https://book.leptos.dev/csr_wrapping_up.html).

- [Leptos rendering modes](https://docs.rs/leptos/latest/leptos/).

- [Axum documentation](https://docs.rs/axum/latest/axum/).

F00 must check stable dependency compatibility against Rust 1.89 and Wasm.
A documentation build on a newer compiler does not establish minimum compiler compatibility.

## Local decisions

Read [Topology layout](../design/tui/TOPOLOGY_MANAGER_LAYOUT.md) for automatic publication and schedule scope.
Read [service API design](../superpowers/specs/2026-11-18-topology-service-api-design.md) for socket ownership and transport.
The [backend map](BACKEND_MAP.md) links current source that confirms reuse claims.

Current files take precedence over older status summaries.

## Images

The built-in imagegen tool generated all 11 PNG files for this conversation.
The tool generated each image independently from text prompts based on the blueprint.
They are illustrative concepts, not captured browser screens.

Original files remain in `/home/pi/.codex/generated_images/01a1f7d8-1525-7e03-91da-3fda9b62560c/`.
Repository copies use stable names under [images](images/).
The [hash manifest](images/SHA256SUMS) records exact SHA-256 hashes.

The [screen catalog](SCREENS.md) describes every image.
The [prompt record](IMAGE_PROMPTS.md) preserves the generation inputs.
Do not use picture data as a production fixture or acceptance oracle.


## Sign-in and theme additions

Three sign-in images extend the initial set to 14 PNG files.
The original sign-in was generated from text. The dark and light glass variants edit that original image.
The owner accepted its composition and requested the glass treatment, superscript version, and three theme modes.
See [THEMING.md](THEMING.md) for the visual references and required behavior.
