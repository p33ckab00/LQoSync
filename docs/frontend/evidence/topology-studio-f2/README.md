# Topology Studio F2 evidence

Timestamp: 2026-11-24T06:51:39+08:00 (Asia/Manila).

Base: `f58b924`. Code commits: `93d5a44`, `d546293`. Test correction: `b5325fc`. Branch: `codex/topology-studio-ui`.
The real Leptos/Wasm UI uses the B0 synthetic loopback fixture.
No production router or credential supplied these records.

## Result

Refresh discovery explicitly collects the source types in the current draft.
It uses saved access configuration. Preview evaluates that draft against one snapshot.

The canvas shows candidate cards, diff markers, and subscriber-count flow widths.
Capacity labels do not claim live throughput. Saved cards remain visible and inert during preview.
Load saved topology sends only a cached GET and preserves the draft.

The details modal shows complete issue counts, paged issues and records, and source provenance.
It labels partial tables, truncation, unknown rates, and unresolved identity.
Old responses cannot replace a newer draft. Expiry preserves that draft.
Read recovery queries the original operation before any resubmission with the original request key.

The current tab keeps non-secret drafts through reauthentication by the same actor.
Explicit logout and a different actor clear that memory. The application does not write source drafts or access secrets to browser storage.
Profile and policy persistence remain F3 work. Their save control remains unavailable.

## Images

| Files | Content |
| --- | --- |
| `studio-preview-canvas.png` | Read-only candidate cards and count-based flows over the saved topology |
| `studio-preview-details-{1440,768,390}.png` | Paged records and provenance within the canvas |
| `studio-preview-details-dark.png` | The same modal with the System dark theme |

`SHA256SUMS` covers each PNG. The rendered checks cover keyboard controls and responsive bounds.

## Verification

All 14 Studio model tests passed. Workspace tests: 2037 passed, 0 failed, 13 ignored.
Workspace native/Wasm Clippy passed with `-D warnings` before the final modal correction.
The final correction also passed native/Wasm Clippy for `web-ui` and the full browser gate.
Format and whitespace checks passed.

The final browser gate passed 24 checks and both builds.

An earlier run read the old inventory layout immediately after resize.
The check now waits for the expected hidden header before its assertion.
The gate skipped `pulse-check`, the established exclusion. Logs: `/tmp/lqosync-checks.T2xkuC/`.

The browser covers normal, partial-read, expired-candidate, and lost-reply scenarios.
It also delays old responses, forces reauthentication, and masks accepted operation replies.
The card-entry regression failed with one modal left open, then passed after the correction.

The independent reviewer could not run because of a usage limit.
A separate author review found and corrected the card-modal defect. This review is weaker than an independent review.

Minor deferred issue: an older snapshot selection can hide the locally collected choice if cached metadata omits it.
The optional design inspector is unavailable. Playwright supplied the rendered checks and screenshots.

B3/G3 integration remains unverified. These checks use B0 fixtures only.
