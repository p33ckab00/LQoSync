# Topology Studio F0 evidence

These images show the real Leptos/Wasm UI on the synthetic browser fixture at loopback port `19581`.
Base: `a0e6035`. Code commits: `4b5b424`, `2918086`, `1faa0c0`. Branch: `codex/topology-studio-ui`.
No production account, router, or output supplied these records.

## Behavior

- Icon tools and separate zoom and appearance docks stay inside the canvas.
- The information icon opens the optional workflow. Editor help keeps the parent dialog and its WAN draft open.
- Native dialogs stay inside the canvas at widths 1440, 768, and 390, also after resize.
- The 667×375 landscape view has usable world space and a scrollable tool dock.
- The Reload target remains clear at 375×667.
- A real keyboard layout save keeps its feedback visible in landscape.
- Initial read messages stay below navigation. A failed first read ends the pending state.
- Theme icons share the application preference. System follows browser changes.
- Invalid drafts, selection, viewport position, card data, flows, and current service requests remain intact.
- Read-only reasons and cache errors remain separate and readable.

## Images

| Files | Content |
| --- | --- |
| `studio-canvas-{light,dark,system}.png` | The saved topology and fixed docks |
| `studio-help-{1440,768,390}.png` | Bounded workflow help |
| `studio-landscape-canvas.png` | Short landscape canvas with the last tool in view |
| `studio-edge-667-landscape.png` | Card settings inside the short canvas |
| `studio-edge-375-phone.png` | Phone navigation and Reload |
| `studio-edge-1440-cold.png` | Failed first read without a false pending state |
| `studio-edge-1440-WAN.png` | The incomplete WAN draft after nested help |
| `studio-edge-375-standalone.png` | Standalone appearance controls on the phone |

`SHA256SUMS` covers each PNG.

## Checks

Workspace tests: 2023 passed, 0 failed, 13 ignored.
Workspace and Wasm Clippy, format, and whitespace checks passed.
The full browser gate passed 20 checks. It skipped `pulse-check`, the established exclusion.
The fixture and Wasm builds passed. Logs: `/tmp/lqosync-checks.4eogJE/`.
The independent review found five important issues. Each failed the new edge check before the fix, then passed.
The minor keyboard-tooltip issue on Reload and the editor header Close remains deferred. Both controls have accessible names.
The optional design inspector is unavailable. Playwright supplied the rendered checks and screenshots.

The new source-policy editor remains F1–F3 work. F0 uses current service interfaces only.
