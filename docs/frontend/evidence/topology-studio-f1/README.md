# Topology Studio F1 evidence

Timestamp: 2026-11-24T00:03:19+08:00 (Asia/Manila).
Base: `18ec855`. Code commits: `19397ac`, `9e18dd6`. Branch: `codex/topology-studio-ui`.
The real Leptos/Wasm UI uses the synthetic loopback fixture. No production router or credential supplied these records.

## Result

The router modal reads cached inventory, capabilities, and source configuration.
It reuses registration, connection editing, credential fields, validation, and the current router-edit transport.
Registration stays inside the canvas and saves the router disabled.
The explicit draft test handles an accepted queued operation, clears its secret, and invalidates changed connection results.

Source icons select separate PPP/PPPoE, DHCP/IPoE, Hotspot, and WAN drafts.
The form uses advertised query fields, operators, grouping fields, saved speed plans, exact capacity strings, and ordered rate sources.
Access-profile metadata stays separate from secret input. Closing its panel clears the secret.
The phone selector and form have independent scroll regions.

Preview and source/access persistence remain F2/F3 work. Their controls remain unavailable with visible reasons.
The existing connection save path still works. An old backend keeps the connection editor available.

## Images

| Files | Content |
| --- | --- |
| `studio-policy-{1440,768,390}.png` | Source rail and typed policy draft |
| `studio-router-{1440,768,390}.png` | Access-profile drafts |
| `studio-policy-{dark,light}.png` | System theme changes in the open modal |
| `studio-router-edge-phone-selector.png` | Reachable router choices and an independently scrollable form |
| `studio-router-edge-secret-panel-close.png` | Secret cleared after Add router cancellation |

`SHA256SUMS` covers each PNG.

## Verification

The seven Studio model tests passed. Workspace tests: 2030 passed, 0 failed, 13 ignored.
Native and Wasm Clippy passed with `-D warnings`. Format and whitespace checks passed.
The final browser gate passed 22 checks and both builds. It skipped `pulse-check`, the established exclusion.
Logs: `/tmp/lqosync-checks.8ZAiCt/`.
The browser uses a queued response override for draft-test polling. The B0 fixture itself completes jobs immediately.

The independent review found one important secret-lifecycle defect. A render check found a clipped phone selector.
Both regression cases failed before the corrections, then passed.
A minor binding-limit issue remains: each row uses 128, without the advertised combined per-policy count. F3 must enforce that count before save.
The optional design inspector is unavailable. Playwright supplied the rendered checks and screenshots.
