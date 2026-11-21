# Topology workflow repair evidence

Date: 2026-11-21 (Asia/Manila).
All screenshots and browser requests use the isolated synthetic fixture.
No production account, router configuration, service, or shaping file changed for these checks.

## Changes and proof

| Area | Result |
| --- | --- |
| Hierarchy | Nested lists, disclosure, ancestor search, readable parent/source names, contextual child/instance actions |
| Responsive layout | Inventory / tree / details at desktop width, collapsible inventory and stacked details on narrow screens |
| Source views | IP/MAC display and search, local-plan filter, reset, reload, selected details, shared reviewed actions |
| Request integrity | Late replies cannot overwrite a newer filter. Snapshot refresh preserves selection and open drafts |
| Source freshness | Stale rows and endpoints remain visibly historical |
| WAN discovery | Every enabled router has a result, including failure or zero interfaces. Repeated reads use new replies |
| Automatic sync | The regular Runner cycle invokes the controlled topology compiler for the active topology writer |
| Activation | Explicit preview and `activate --yes`, with enrollment, lease, source, static-file, compiler, and revision checks |
| Failure behavior | The previous shaping pair and source timestamps remain intact. No legacy fallback |
| Diagnostics | Native/bridge diagnostics and topology circuit counts remain available |

## Checks

- The full core library suite passed: 1,573 tests, no failures, 10 ignored.
- The web contract, gateway, and UI suites passed: 100 tests, no failures, one ignored scale test.
- The ignored web scale test passed when run explicitly.
- The installed-binding native test passed in temporary directories with live writes excluded.
- That test proves dry-run activation, explicit activation, stable identity, automatic reparent export, and failure retention.
- The router-count regression failed before the projection fix and passed after it.
- Native Clippy for all affected packages/targets passed with `-D warnings`.
- Wasm Clippy passed with `-D warnings`.
- Formatting, JavaScript syntax, and whitespace checks passed.
- The optimized Wasm, core binary, gateway, broker, and web-admin builds passed.
- The release CLI advertises activation and rejects invalid activation flags before configuration or router access.
- All three affected browser suites passed.
- The source and Builder workflow checks also passed with task-only staged CSS. Subscriber columns stay accessible without pending Routers styles.

The optional UI/UX inspector is not installed. Playwright supplies the browser checks.
The source uses the current Leptos stack and CSS tokens. It adds no application dependency.
The browser checks cover 320, 390, 768, 1024, and 1440 px layouts across the applicable suites.
Desktop and narrow screenshots received visual review.

## Reproduce

Use the existing Playwright module, Chromium binary, and test-only fixture password.
Start a fresh `web-gateway` `browser_fixture` for each browser suite on loopback port 19581.
Never substitute production credentials or a production URL.

```sh
cargo test --locked -p lqosync3 --lib
cargo test --locked -p web-contract -p web-gateway -p web-ui
cargo test --locked -p web-gateway --test topology release_scale_reads_stay_bounded -- --ignored --exact
cargo clippy --locked --all-targets -p lqosync3 -p web-contract -p web-gateway -p web-ui -- -D warnings
cargo clippy --locked -p web-ui --target wasm32-unknown-unknown -- -D warnings
cargo fmt --all --check
node tools/frontend/topology-check.cjs
node tools/frontend/topology-workflow-check.cjs
node tools/frontend/subscriber-workflow-check.cjs
```

The native binding test requires explicit selection of the installed LibreQoS binding.
It copies the binding and example configuration to private temporary directories.

```sh
LQOSYNC3_RUNTIME_BINDING=/opt/libreqos/src cargo test --locked -p lqosync3 --lib runner::tests::installed_binding_runs_configured_source_sync_without_live_writes -- --ignored --exact
```

## Screenshots

- [Builder, desktop Dark](builder-dark-1440.png).
- [Builder, desktop Light](builder-light-1440.png).
- [Builder, narrow Dark](builder-dark-390.png).
- [PPPoE with selected subscriber](pppoe-workflow-dark.png).
- [IPoE/DHCP, desktop Light](ipoe-workflow-light-1440.png).
- [IPoE/DHCP, narrow Light](ipoe-workflow-light-390.png).

## Design guidance

[Vercel frontend-design](https://github.com/vercel-labs/open-agents/blob/main/.agents/skills/frontend-design/SKILL.md) guided compact hierarchy and reuse of the current console.
[Vercel interface rules](https://github.com/vercel-labs/web-interface-guidelines/blob/main/command.md) guided semantic controls, explicit labels, focus, and request guards.

[Google Modern Web Guidance](https://github.com/GoogleChrome/modern-web-guidance), skill version `2026_11_10-47b4c80e`, supplied these guides:

- `css-layout`: “DOM order still drives sequential focus”. The grid keeps source order and visual order aligned.
- `responsive-table`: “Use standard `<table>` elements”. The source tables keep their headers and keyboard-scroll regions.
- `accordion`: “Do not add arrow key navigation”. Disclosure uses normal buttons without an incomplete ARIA tree keyboard model.

## Deployment boundary

These results prove source and fixture behavior. They do not prove that the running installation uses the repaired binaries.
Install the repaired core, web services, and browser assets through the normal deployment workflow.
Then follow [the explicit activation procedure](../../TOPOLOGY_WORKFLOW_REPAIR.md).
Inspect files/import and native verification separately after activation.
