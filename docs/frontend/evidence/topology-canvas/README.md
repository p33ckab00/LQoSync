# Canvas settings design reference

Date: 2026-11-21 (Asia/Manila).
Status: browser mock and design evidence. Production implementation remains pending.

Open [the mock](mockup.html) in a browser.
The right-side icon dock opens Inventory, Topology settings, Subscriber allocation, and optional Preview modals.
Node buttons open configuration modals. All topology records and rates are synthetic.
Fields change local example state only. This mock has no service, network request, router access, or persistent policy write.
It does not implement canvas pan, zoom, source discovery, drag placement, or the production auto-apply engine.

## Evidence

- [Dark canvas](canvas-dark.png)
- [Light canvas](canvas-light.png)
- [Settings modal](settings-dark.png)
- [Site location modal](node-location.png)
- [375px settings modal](settings-375.png)
- [768px settings modal](settings-768.png)

## Checks

Run `node docs/frontend/evidence/topology-canvas/check.cjs` from the project root.
Override `PLAYWRIGHT_MODULE` and `CHROMIUM_PATH` when the local tools have different paths.

The browser check passed for keyboard modal access, Escape, focus return, and local field feedback.
It also passed for full decimal input, optional preview, Light/Dark capture, and horizontal overflow at 375px and 768px.
No browser script errors or HTTP requests occurred.
Manual inspection covered the dark canvas, settings modal, and narrow settings capture.
These checks do not establish production accessibility, service integration, or LibreQoS runtime acceptance.

## Design sources

[Vercel frontend-design](https://github.com/vercel-labs/open-agents/blob/main/.agents/skills/frontend-design/SKILL.md) informed the industrial console direction and restrained canvas composition.
[Vercel Web Interface Guidelines](https://github.com/vercel-labs/web-interface-guidelines/blob/main/command.md) informed accessible icon controls, focus, dialogs, paste, and narrow layouts.
The mock keeps the current application typography and theme direction.
It does not introduce React or replace the Rust/Leptos production stack.

See the [finalized canvas design](../../TOPOLOGY_CANVAS_DESIGN.md) for the required production workflow.
