---
type: handoff
created: 2026-10-31
tags: [handoff]
status: active
related: ["[[2026-10-31-project-setup-and-v3-simulation]]"]
---

# Handoff

## 2026-11-21T00:12:08+08:00 (Asia/Shanghai) — NIC summary placement

- Goal: move the NIC SVG faceplate beside PPPoE and LibreQoS in the service summary.
- Last agent: Codex `/root`. Branch: `lqosync3`. Worktree: `/home/pi/LQoSync3.0`. Base HEAD: `fb720af`.
- Changed: Dashboard/DPDK composition, shared snapshot and server clock, NIC heading, responsive CSS, browser checks, evidence, and documentation.
- One faceplate appears before traffic. Raw NIC counters remain in DPDK diagnostics.
- Checks: NIC/DPDK, sign-in/Dashboard, and aggregate browser checks; web release build; Wasm Clippy; Rust 1.89; formatting and whitespace.
- Evidence: `docs/frontend/evidence/nic-summary/README.md`. Synthetic data only.
- Next action: install the web artifacts with `sudo tools/frontend/install.sh`, then refresh the browser.
- Full connector detection still needs the previous core update if absent. This placement change needs only web artifacts.
- Production services did not change. Preserve the unrelated November 16 notes and report.

## 2026-11-20T23:56:24+08:00 (Asia/Shanghai) — Physical NIC faceplate

- Goal: show adjacent hardware-specific NIC SVGs with explicit roles, speed/port number, PCI identity, and independent RX/TX LEDs.
- Last agent: Codex `/root`. Branch: `lqosync3`. Worktree: `/home/pi/LQoSync3.0`. Base HEAD: `f3249cd`.
- Changed: core NIC identity/enrichment and settings handoff, typed web identity/activity projection, keyed faceplate and pulse tracker, fixtures/checks, design/deployment/evidence, and brain notes.
- Sources: DPDK link/counters and optional module identifiers; PCI/netdev sysfs mapping; explicit LibreQoS bridge interface configuration; bounded optional kernel queries.
- Unknown connector/role remains explicit. Speed alone never establishes a connector type. Module serials and full config stay private.
- Activity: one 180-ms RX/TX pulse per distinct fresh completed interval, after an initial baseline. No duplicate/idle/down/reset/stale replay. Reduced Motion stays static.
- Cadence: NIC ten seconds; full web projection five seconds. This is sampled activity, not packet timing.
- Checks: fifteen NIC tests, 65 worker tests with one existing ignored test, web/native UI checks, all browser regressions, release builds, Clippy, Rust 1.89, formatting, and whitespace. Evidence: `docs/frontend/evidence/nic-faceplate/README.md`.
- Next runtime action: follow `docs/frontend/DEPLOYMENT.md#nic-faceplate-update` to install the core and web artifacts together.
- Production files/services and NIC bindings did not change. TUI presentation and CLI accounts remain unchanged.
- A privileged live module probe needed a sudo password, so actual host connector identity remains unverified until the installed collector reads it.
- Preserve unrelated November 16 notes and the dataplane report.

## 2026-11-20T23:15:08+08:00 (Asia/Shanghai) — Aggregate traffic consolidation

- Goal: consolidate Actual/Mapped/Unmapped in the current aggregate chart.
- Last agent: Codex `/root`. Branch: `lqosync3`. Worktree: `/home/pi/LQoSync3.0`. Base HEAD: `6c2cc0d`.
- Changed: web contract/projection, reply size guard, mirrored SVG geometry/component/CSS, stale handling, browser fixtures/checks, design/deployment/evidence, and brain notes.
- Sources: Actual delivered history and classification enqueue histories in `worker-utilization.json`. Do not sum or stack them.
- Behavior: independent mirrored direction scales, capacity-anchored gradients, shared timestamp inspector, persistent source toggles/pins, and accessible samples.
- Bounds: at most 300 points per source before a 60-KiB full overview budget. The broker reply guard enforces 64 KiB.
- Checks: native tests, native/Wasm releases, native/Wasm Clippy, Rust 1.89, formatting, whitespace, and browser evidence. See `docs/frontend/evidence/aggregate-traffic/README.md` for results.
- Next runtime action: operator runs `sudo tools/frontend/install.sh`, then refreshes the browser and compares the same timestamps with the TUI.
- Remaining scope: longer window API and subscriber Queue Dynamics/Stats/Tree contracts.
- No production files, services, collectors, TUI, or CLI account policy changed. Preserve unrelated November 16 notes and the dataplane report.

## 2026-11-20T22:48:47+08:00 (Asia/Shanghai) — Mirrored traffic research

- Goal: research Actual/Mapped/Unmapped comparison, compact DPDK figures, and LibreQoS subscriber queue charts.
- Last agent: Codex `/root`. Branch: `lqosync3`. Worktree: `/home/pi/LQoSync3.0`. Base HEAD: `6c96a71`.
- Changed: `docs/frontend/MIRRORED_TRAFFIC_RESEARCH.md`, the research session note, Home, and this handoff.
- Evidence: installed LibreQoS `3.0.7.202611192341`, embedded source map, pinned upstream sources, and current core/TUI/web contracts.
- Key constraint: Actual is delivered; Mapped/Unmapped are enqueued. Do not sum or stack them.
- Proposal: one mirrored traffic panel, a capacity gradient, shared time inspection, and compact DPDK figures with detail access.
- Gaps: typed browser classification history, bounded time windows, circuit queue web contract, and verified live ancestor metrics.
- Checks: documentation checks only. No application change needs a build or runtime test.
- Next action: review this proposal, then implement its bounded traffic contract before the chart changes.
- Production services, installed LibreQoS, core collectors, TUI, and dependencies were not changed.
- Preserve the unrelated November 16 notes and dataplane report.

## 2026-11-20T22:25:18+08:00 (Asia/Shanghai) — DPDK Dashboard card

- Goal: add DPDK stats/health with the same telemetry sources as the TUI, with planned charts and updates.
- Last agent: Codex `/root`. Branch: `lqosync3`. Worktree: `/home/pi/LQoSync3.0`. Previous HEAD: `ca9065e`.
- Changed: typed DPDK contract, cache projection, overview integration, browser stale handling, Dashboard card/history/CSS, fixtures/tests, frontend plan/deployment/evidence, and brain notes.
- Sources: `worker-utilization.json` and `nic-telemetry.json`. No collector or DPDK socket access from the web services.
- Bounds: all-worker summary; sixteen detailed workers; eight visible bars; eight ports; sixteen xstats per port; sixteen-KiB optional row details; sixty session samples.
- Freshness: workers six seconds; bridge health and NIC fifteen seconds. Browser age uses server time and elapsed receipt time.
- Backend gaps: persisted worker history and a full paged Workers inspector contract. F06/F10 remain partial.
- Checks: eight projection tests, six native UI tests, existing gateway tests, DPDK browser checks, existing smoke and ticker regressions, native/Wasm release, Clippy, Rust 1.89, formatting, and whitespace passed.
- Read-only runtime probe: one worker and two ports; live worker/NIC data and healthy bridge at that check. Approximate debug projection: 24 ms; no production latency claim.
- Plan: `docs/frontend/DPDK_DASHBOARD.md`. Evidence: `docs/frontend/evidence/dpdk-dashboard`.
- Next runtime step: operator runs `sudo tools/frontend/install.sh`, then compares source values with the TUI. Prepared web binaries/assets are ready.
- Production files/services, core collectors, and TUI code were not changed. Preserve unrelated November 16 notes and the dataplane report.

## 2026-11-20T21:42:13+08:00 (Asia/Shanghai) — Ticker startup values

- Goal: correct all-Unknown moving values after sign-in.
- Last agent: Codex `/root`. Branch: `lqosync3`. Worktree: `/home/pi/LQoSync3.0`. Previous HEAD: `b83eb63`.
- Cause: the queue captured ten placeholders before the first response. A browser test reproduced the failure.
- Runtime check: core, broker, and gateway active; status cache has mode, target, and cycle duration. Preserve real degraded/unknown health.
- Changed: Rust initialization/readiness, waiting-state CSS, browser regression, frontend docs/evidence, and brain notes.
- Checks: delayed-response startup, actual moving values, existing continuity/activity checks and smoke, Wasm release, Clippy, Rust 1.89, formatting, and whitespace passed.
- Evidence: `docs/frontend/evidence/diagnostics-warmup`.
- Next runtime step: operator runs `sudo tools/frontend/install.sh`. Production files and services were not changed.
- Preserve unrelated November 16 notes and the dataplane report.

## 2026-11-20T21:36:04+08:00 (Asia/Shanghai) — Continuous ticker updates

- Goal: keep the ticker moving without a reset when activity arrives.
- Last agent: Codex `/root`. Branch: `lqosync3`. Worktree: `/home/pi/LQoSync3.0`. Previous HEAD: `02eaebb`.
- Cause: total width and duration changed during a lap. A browser test reproduced an 83-pixel jump.
- Changed: Rust moving queue, CSS pixel distance, browser continuity checks, frontend documentation/evidence, and brain notes.
- Behavior: append activity at the tail; refresh status text after it exits; remove expired waiting entries offscreen; keep visible text through its pass.
- Bound: ten status items and at most eight activity items. CSS motion uses one Rust measurement per handoff.
- Checks: insertion regression, repeated handoffs/coverage, existing ticker checks and smoke, Wasm release, Clippy, Rust 1.89, formatting, and whitespace passed.
- Evidence: `docs/frontend/evidence/diagnostics-continuous`. Design: `docs/frontend/DIAGNOSTICS_TICKER.md`.
- Next runtime step: operator runs `sudo tools/frontend/install.sh`. Production assets and services were not changed.
- Preserve unrelated November 16 notes and the dataplane report.

## 2026-11-20T21:26:16+08:00 (Asia/Shanghai) — Rolling diagnostics strip

- Goal: show only a rolling banner. Remove the card, heading, subtitle, and pause control.
- Last agent: Codex `/root`. Branch: `lqosync3`. Worktree: `/home/pi/LQoSync3.0`. Previous HEAD: `8bfdd26`.
- Changed: Rust ticker markup, app wrapper, CSS, browser check, design/evidence documents, and brain notes.
- Behavior: continuous movement, no hover pause, five permanent fields, and existing bounded temporary activity. Reduced motion stays static.
- Checks: Wasm release, Clippy, Rust 1.89, formatting, whitespace, focused browser checks, and existing sign-in/Dashboard smoke passed.
- Evidence: `docs/frontend/evidence/diagnostics-strip`. Design: `docs/frontend/DIAGNOSTICS_TICKER.md`.
- Next runtime step: operator runs `sudo tools/frontend/install.sh`. Production assets and services were not changed.
- Preserve unrelated November 16 notes and the dataplane report.

## 2026-11-20T21:16:53+08:00 (Asia/Shanghai) — Top ticker and conditional activity

- Goal: move diagnostics above the Dashboard heading and keep exactly five permanent status fields.
- Last agent: Codex `/root`. Branch: `lqosync3`. Worktree: `/home/pi/LQoSync3.0`. Previous HEAD: `9c2e06b`.
- Changed: app placement, static details wrapper, Rust activity tracker/ticker, CSS, browser checks, docs/screenshots, and brain notes.
- Permanent fields: Service health, Runtime mode, Output target, Publication phase, Last cycle. Preserve raw publication values.
- Activity: baseline first snapshot, suppress duplicates and traffic-only changes, keep eight entries for 120 seconds, expire without another source update.
- Backend gap: observed browser changes only. A full job-progress/activity feed needs an authenticated backend event projection.
- Checks: five native chart/activity tests, release Wasm, Clippy, Rust 1.89, focused browser and existing sign-in/Dashboard smoke passed. Browser verifies placement, permanent fields, activity, duplicate suppression, expiry, motion, themes, and 768/320px layouts.
- Evidence: `docs/frontend/evidence/diagnostics-ticker-top`. Current design: `docs/frontend/DIAGNOSTICS_TICKER.md`.
- Next runtime step: operator runs `sudo tools/frontend/install.sh`. Production files/services were not changed in this session.
- Preserve unrelated November 16 notes and the dataplane report.

## 2026-11-20T21:02:39+08:00 (Asia/Shanghai) — Diagnostics ticker

- Goal: show synchronization and component health as a stock-exchange-style banner.
- Last agent: Codex `/root`. Branch: `lqosync3`. Worktree: `/home/pi/LQoSync3.0`. Previous HEAD: `45dad14`.
- Changed: Rust diagnostics banner, Dashboard wrapper, CSS, focused browser test, frontend docs/screenshots, and brain notes.
- Behavior: stable transform track, live values, visible keyboard Pause/Resume, hover/focus pause, static reduced motion, and full static details. Hide the duplicate from assistive technology.
- Checks: release Wasm, Clippy with warnings denied, Rust 1.89, formatting, whitespace, focused browser checks, and existing sign-in/Dashboard smoke passed. Browser checks include duplicate names, long labels, stale evidence, themes, and 768/320px widths.
- Core, gateway, and TUI code did not change. The core test suite was not repeated.
- Evidence: `docs/frontend/evidence/diagnostics-ticker`. Design/atomic plan: `docs/frontend/DIAGNOSTICS_TICKER.md`.
- Next runtime step: operator runs `sudo tools/frontend/install.sh`. This session does not replace installed files or restart production services.
- Preserve unrelated November 16 notes and the dataplane report.

## 2026-11-20T20:43:13+08:00 (Asia/Shanghai) — Live traffic and WAN capacity

- Goal: adapt LibreQoS live cadence and implement a rainbow chart with configured WAN capacity limits.
- Last agent: Codex `/root`. Branch: `lqosync3`. Worktree: `/home/pi/LQoSync3.0`. Previous HEAD: `61b1dc9`.
- Changed: core status/runner and telemetry cadence/history bound; web contract, projection, broker, authenticated stream, Rust browser lifecycle, chart geometry/styles, fixture/tests, Nginx example, frontend docs/evidence, operations guide, and brain notes.
- Source: LibreQoS revision `15be4ca69d07691813260ed620498fb1f6c9aae6`; one-second ticker and timestamped ring buffer. No upstream code copied.
- Capacity: sum true validated WAN bases separately for download/upload. Do not infer routing, live link health, or applied shaping.
- Stream: authenticated SSE, 16-connection bound, account/session checks per event, no slow-client queue, hidden-page cleanup, and fallback reads.
- TUI screens and controls stay unchanged. One-second telemetry keeps one hour with 3601 points and a 2 MiB public cache bound.
- Checks: 1590 passed, 0 failed, 12 existing ignored tests. Native core/web and Wasm release builds passed. Gateway/Wasm Clippy and Rust 1.89 checks passed. Browser smoke/pulse, themes, 320/768px layouts, forced colors, stream lifecycle, overflow, and unknown capacity passed.
- Performance evidence: full-hour fixture gives 300 browser points and 19,511 bytes. One debug projection took approximately 27 ms. No production latency claim.
- Evidence: `docs/frontend/evidence/live-traffic`. Plan/source details: `docs/frontend/LIVE_TRAFFIC.md`. Session: `brain/10-sessions/2026-11-20-live-traffic-capacity.md`.
- Unfinished runtime step: install the prepared core binary and web services. No production binary or service changed in this session. Passwordless sudo is unavailable. Follow `docs/frontend/DEPLOYMENT.md`, Live traffic update.
- Remaining F06 scope: per-WAN attribution/health, longer browser windows, and broader performance gates.
- Preserve unrelated November 16 notes and the dataplane report.

## 2026-11-20T20:03:19+08:00 (Asia/Shanghai) — Dashboard reference alignment

- Owner direction: use the approved Dashboard picture and frontend documents as acceptance criteria. Always use Vercel and Google frontend skills.
- Last agent: Codex `/root`. Branch: `lqosync3`. Worktree: `/home/pi/LQoSync3.0`. Previous HEAD: `06f0536`.
- Changed: web contracts, cache projection and tests, browser fixture, app shell, new `crates/web-ui/src/dashboard.rs`, CSS, browser checks, alignment documentation, screenshots, and this session's brain notes.
- Implemented: five icon cards with a bound-DHCP IPoE count, documented dark palette, timestamp-based gradient traffic, scale/time labels, unavailable history states, WAN distribution and upstream table states, and service diagnostics below the main panels.
- Reliability: keep details open during polling. Mark retained data stale and health unknown on a request failure. Reject invalid traffic timestamps and check sample age.
- Checks: 11 web tests passed. Native/Wasm release builds, Rust 1.89 checks, gateway/UI Clippy with warnings denied, formatting, browser failure/recovery, reference sections, and 320px checks passed.
- Limits: no safe WAN attribution/capacity/health overview exists. Longer history needs retention. These panels explicitly state unavailable. F06 remains partial.
- Runtime: current installed web services and local Traefik route work. This update is built locally and has not replaced the installed files. Core service and TUI source files are unchanged.
- Next: operator runs `sudo tools/frontend/install.sh`, then reviews the live Dashboard. Read `docs/frontend/DASHBOARD_ALIGNMENT.md` for future work.
- Preserve unrelated untracked November 16 notes and the dataplane operations report.

## 2026-11-20T19:23:23+08:00 (Asia/Shanghai) — Traffic chart gradient

- Owner requested gradient areas that match the Dashboard mockup, with Vercel and Google frontend guidance.
- Last agent: Codex `/root`. Branch: `lqosync3`. Worktree: `/home/pi/LQoSync3.0`. Previous HEAD: `0b4abff`.
- Changed: `crates/web-ui/src/app.rs`, `crates/web-ui/style.css`, frontend index, and `docs/frontend/evidence/traffic-gradient`.
- Added blue and green fades with unique SVG gradient IDs. Keep line geometry, counts, polling, and stale states.
- Leptos static SVG fragments wrap stops in groups. A bound stop color keeps six direct stops and fixes rendering.
- Checks: release Wasm build, Rust 1.89 check, formatting, browser smoke, Light/Dark screenshots, 320px layout, refresh references, and high-contrast mode passed.
- Runtime update: the operator installed the first slice. Both web units are active. The local Traefik hostname route returns HTTP 200 after the indentation correction.
- Unfinished: install the rebuilt gradient assets with `sudo tools/frontend/install.sh`. This session has not changed installed assets. Public HTTPS was not checked.
- Next: operator installs the prepared update, then checks the live Dashboard. Keep unrelated untracked notes outside the commit.

## 2026-11-20T19:00:50+08:00 (Asia/Shanghai) — Web sign-in and Dashboard

- Goal: deliver the first Rust web slice on port `9581` for `lqosync.fsehub.qzz.io`.
- Last agent: Codex `/root`. Branch: `lqosync3`. Worktree: `/home/pi/LQoSync3.0`. Previous HEAD: `df5738d`.
- Changed: workspace manifest/lock, `crates/web-contract`, `crates/web-gateway`, `crates/web-ui`, `tools/frontend`, `deploy/frontend`, frontend documentation, screenshots, and this session's brain notes.
- Runtime: unprivileged gateway, root broker, private account database, root-only account CLI. No web signup or account CRUD.
- Source reads: bounded public cache projection. No core service or TUI source changes. Domain configuration actions remain planned.
- Checks: core tests 1,571 passed with 12 ignored; nine web tests passed. Gateway Clippy, formatting, native/Wasm release builds, and Rust 1.89 checks passed.
- Browser: sign-in, password visibility, themes, System changes, 320px layout, session persistence, and logout passed. See `docs/frontend/evidence/FIRST_SLICE.md` for the final gzip check and screenshots.
- Blocker: passwordless sudo is unavailable. Live web services, the first real account, Traefik merge, and public-route validation remain pending.
- Next: run the reviewed installer as root, create an account interactively, merge the Traefik router/service, and verify the HTTPS route. See `docs/frontend/DEPLOYMENT.md`.
- Preserve unrelated untracked November 16 notes and the dataplane operations report. They are outside this commit.

## 2026-11-20T16:58:02+08:00 (Asia/Shanghai) — Global company branding

- Owner requested company name/logo in global Settings alongside LQoSync branding, then commit/push.
- Added COMPANY_BRANDING.md and F19. Updated design, contracts, settings ownership, theme rules, screen catalog, and quality requirements.
- Keep product logo and superscript v3. Company identity applies to sign-in and the shared main header across themes.
- Presentation-only gateway storage and safe public branding reads. No core config write or service reload.
- Checks: links, 20-task order, 14 image hashes, STE review, and whitespace. Documentation only.
- Branch: `lqosync3`. Worktree: `/home/pi/LQoSync3.0`. Baseline: `aa75133`. Agent: Codex `/root`.
- Next: execute F00 and dependencies; F19 implements company branding. CLI-only accounts, primary web control, and unchanged TUI remain requirements.

## 2026-11-20T16:53:01+08:00 (Asia/Shanghai) — Screen-owned settings

- Owner direction: Light/Dark/System is available on sign-in and the shared main header. Each feature owns its configuration forms.
- Added `docs/frontend/SCREEN_SETTINGS.md`. Updated feature design, contracts, theme rules, screen catalog, quality checks, and F05/F15/F18.
- Global Settings contains application-wide fields only. Typed scoped patches reject foreign fields and share the backend digest/transaction boundary.
- Checks: document links, task order, 14 image hashes, STE/manual review, and whitespace. Documentation only.
- Branch: `lqosync3`. Worktree: `/home/pi/LQoSync3.0`. Agent: Codex `/root`. Baseline: `9e73725`.
- Next: execute F00 and follow the updated task dependencies. CLI-only accounts, primary web control, and unchanged TUI remain requirements.

## 2026-11-20T16:47:12+08:00 (Asia/Shanghai) — Sign-in and theme direction

- Owner accepted original sign-in composition and logo. Requested glass effect, superscript v3, and Light/Dark/System themes across the web frontend.
- Saved original sign-in plus dark/light glass variants. There are now 14 PNG files under `docs/frontend/images/`.
- Added THEMING.md and updated F05, design, quality, catalog, prompt record, sources, and session note. The two glass variants remain review concepts.
- Checks: image hashes, document links, STE review, whitespace. No application or runtime changes.
- Next: retain the accepted composition and implement the F05 theme contract after its dependencies. CLI-only accounts and unchanged TUI remain requirements.

## 2026-11-20T16:31:35+08:00 (Asia/Shanghai) — Primary web frontend clarification

- Owner direction: the web UI is the primary frontend and operator workflow surface. The TUI stays as is.
- Keep backend execution and synchronization independent of browser, gateway, and TUI processes.
- Updated frontend design, plan, backend map, quality requirements, and session note on `lqosync3`.
- Removed mandatory TUI edits from planned web tasks. Web settings writes need proof of safe coexistence with the unchanged TUI.
- Account CRUD remains root CLI-only. No application implementation or runtime change occurred.


## 2026-11-20T16:24:46+08:00 (Asia/Shanghai) — CLI-only frontend accounts

- Owner direction: root CLI manages usernames and passwords. Allow multiple superadmins. Web access has sign-in/sign-out only.
- Agent: Codex `/root`. Branch: `lqosync3`. Worktree: `/home/pi/LQoSync3.0`. Baseline: `4305ed8`.
- Updated `docs/frontend/` and the frontend session note. Added `CLI_ACCOUNT_POLICY.md`; replaced web account tasks with CLI tasks.
- Account storage is root-owned and separate from gateway sessions. Credential changes invalidate existing sessions.
- The Users picture remains a withdrawn concept. No signup, invitations, Users navigation, or account CRUD HTTP routes are planned.
- Checks: relative links, task dependencies, STE review, and whitespace. No application code or runtime state changed.
- Next: implement F00, then the updated F04/F14 account contracts in dependency order. Preserve unrelated operational notes.

## 2026-11-20T16:11:44+08:00 (Asia/Shanghai) — Frontend design and atomic plan

- Goal: document all pictures and plan a Rust frontend connected to LQoSync3. The owner requested commit and push to `lqosync3`.
- Agent: Codex `/root`. Branch: `lqosync3`. Worktree: `/home/pi/LQoSync3.0`. Source baseline: `9a465e7`.
- Files: `docs/frontend/`, root README, the frontend research/session notes, Home, and this handoff.
- Deliverables: 11 PNG copies with hashes, prompt record, screen catalog, feature design, backend gaps, API contracts, quality requirements, and 19 atomic task cards.
- Checks: image hashes, local links, task dependencies, STE heuristic/manual review, and whitespace. See `docs/frontend/VALIDATION.md`.
- Application implementation has not started. No build, browser benchmark, service, router, live shaping input, or deployment changed.
- Preserve unrelated untracked operational notes. The graph remains the older index.
- Next: execute F00 for compatible dependency selection and measured baseline, then the listed task order. Production deployment is outside this request.
- Session: [[2026-11-20-frontend-documentation-plan]]. Documentation entry: `docs/frontend/README.md`.

## 2026-11-20T15:26:39+08:00 (Asia/Shanghai) — Frontend blueprint research

- Goal: adapt LQoSync4 frontend ideas to the current LQoSync3 backend.
- Agent: Codex `/root`. Branch: `lqosync3`. Worktree: `/home/pi/LQoSync3.0`. Baseline: `9a465e7`.
- Read remote planning documents through the GitHub connector. No LQoSync4 branch or directory was created.
- Changed files: this handoff, Home, and [[2026-11-20-frontend-blueprint-research]]. Research recommendations remain proposals.
- Checks: current navigation, TUI data inputs, socket dispatch, peer checks, dependencies, graph leads, and remote roadmap reviewed.
- No application tests or browser checks occurred. No source, service, or production data changed. Preserve existing untracked operational notes.
- Google skill resolved: GoogleChrome `modern-web-guidance`. Read the skill and retrieved table, drawer, and dialog guides.
- Unfinished: agree on navigation, first screens, and frontend stack.
- Next: discuss the proposed shell and Dashboard. The research note records source links, capability gaps, and three approaches.

## 2026-11-17T22:06:29+08:00 (Asia/Shanghai) — Runner enrollment and recovery complete locally

- Agent: Codex `/root`. Worktree: `/home/pi/LQoSync3.0`. Branch: `lqosync3`. Base and pushed checkpoint: `c9890b2`.
- Committed and pushed the preceding migration staging slice. This new controller slice remains local and uncommitted. The index is empty.
- Preserve unrelated operational edits and older entries in this file. The prior commit included only topology hunks from Home and HANDOFF.
- Added `topology_manager/enrollment.rs` and `runner/tests/enrollment.rs`. Extended Runner lock initialization, the pending-state helper, reader visibility, and test fixture reuse.
- Updated design, discussion, plan, session, and Home. No dependency, registry schema, CLI, or TUI changed.
- `Runner::lock()` recovers pending migration before receipt initialization and normal service work, using the same instance lock.
- No journal means no registry creation or policy decode during ordinary startup. The normal path adds a presence check.
- Recovery clears cached registry state and transport sessions before it opens the store. Only success replaces the cached store.
- First-lock failures release the temporary lock. An existing owner keeps its lock and receipt tracker. Failed recovery leaves no stale registry cache.
- Critical conflicts block startup or the current operation and preserve unknown files. The same Runner can retry after explicit repair.
- `prepare_topology(max_age)` now exposes the existing fresh preview API. It keeps target, receipt, collection, and identity gates.
- `enroll_topology` reads the current legacy pair, checks configured adoption, stages policy, and completes the shared recovery boundary.
- The reader requires two bounded regular files and a combined raw budget of 16 MiB. It rejects missing, malformed, oversized, or symlinked inputs.
- The current manifest must still match live endpoints and hierarchy. Changed source filters or incompatible legacy rates fail before installation.
- Enrollment returns the installed registry revision and preserves the live pair. It does not write shaping output, request a reload, or claim a topology writer.
- Recovery can finish after a mode change because it writes policy only. Manager service activation remains gated; legacy behavior stays unchanged until separate activation.
- Full suite: 1,193 passed, zero failed, 13 ignored. All 12 new enrollment tests and 445 active topology tests pass.
- Format, Clippy with warnings denied, whitespace, and installed-library configured-source scratch sync pass. Author review only. No production benchmark occurred.
- Logs: `.superpowers/sdd/2026-11-17-topology-enrollment/`. Proof: `full-final.log`, `clippy.log`, `format.log`, and `native-source.log`.
- No live source read, router write, live shaping output, native apply, service action, deployment, or runtime mode switch occurred.
- Next: enrollment interface and manifest workflow. Native implicit commitments, planner/live proof, schedules, TUI, and service activation remain gates.
- Existing-source replacement, active-owner migration, mixed-version registry edits, uncoordinated external edits, and schema-2 circuit ID migration remain outside this slice.
- Session: [[2026-11-17-topology-enrollment]]. Plan: `docs/superpowers/plans/2026-11-17-topology-enrollment.md`.
- The graph remains the earlier baseline. Current files take precedence.

## 2026-11-17T19:35:49+08:00 (Asia/Shanghai) — Migration staging and recovery complete locally

- Agent: Codex `/root`. Worktree: `/home/pi/LQoSync3.0`. Branch: `lqosync3`. Base and pushed checkpoint: `e0afb4b`.
- The preceding shared adoption slice is committed and pushed. This new slice remains local and uncommitted. The index is empty.
- Preserve unrelated operational edits, the prior session changes, and older entries in this file.
- Added `staging.rs` and `staging_tests.rs`. Extended proposal seals, store validation reuse, backup lock checks, runtime guards, and source read visibility.
- Updated design, discussion, plan, session, and Home. No dependency or registry schema changed.
- `stage` checks the original deadline and registry seal. It confirms a backup and saves a private journal before installation.
- The journal binds the canonical runtime owner directory, base/target registry policy, and exact static source through SHA-256. It has a 48-MiB limit.
- First adoption requires absent source and runtime-owner files. The existing 16-MiB static-source limit remains in force.
- Journal, blocker, and source use the existing private hard-link publisher. Unknown destinations cannot be replaced, including symlinks or files created after an absence check.
- Pending journals block ordinary commits, owner claims, and both runtime publication paths. Unsafe or corrupt journals also block these operations.
- A temporary `migration-pending` owner marker blocks earlier strict runtime readers. It never grants topology ownership.
- `recover` confirms recognized base/target policy and exact source bytes. It reuses ordinary store invariants and commits at most one target revision.
- Interrupted writes retain the store's reopen requirement. Tests reopen after every durable boundary and resume without duplicate revisions.
- Source loss or changes after commit require explicit repair. Unknown source, owner, or registry content is never replaced with a guess.
- `cancel` works only before source installation with the base registry still current. Later recovery moves forward and preserves durable identities.
- Completion removes the temporary marker and journal with directory syncs. Later source revisions remain possible through normal source/receipt checks.
- Dynamic endpoints enter neither registry policy nor the journal's static source. Later publication still requires fresh collection and compiler admission.
- Full suite: 1,181 passed, zero failed, 13 ignored. All 445 active topology tests pass. Added 14 active tests.
- Format, Clippy with warnings denied, whitespace, and installed-library scratch compilation pass. Author review only. No production benchmark occurred.
- Logs: `.superpowers/sdd/2026-11-17-topology-migration-staging/`. Proof: `full-final.log`, `clippy.log`, `format.log`, and `native-adoption.log`.
- No live source read, router write, shaping output, native apply, service action, deployment, or runtime mode switch occurred.
- Next: connect controlled enrollment/recovery before normal runtime work. No TUI or service-start controller calls these APIs yet.
- Existing-source replacement, active-owner migration, mixed-version registry edits, and uncoordinated external edits remain outside this first-adoption API.
- Native implicit commitments, planner/live proof, enrollment UI, schedules, TUI, service activation, and schema-2 circuit ID migration remain gates.
- Session: [[2026-11-17-topology-migration-staging]]. Plan: `docs/superpowers/plans/2026-11-17-topology-migration-staging.md`.
- The graph remains the earlier baseline. Current files take precedence.

## 2026-11-17T18:55:19+08:00 (Asia/Shanghai) — Shared legacy adoption proposals complete locally

- Agent: Codex `/root`. Worktree: `/home/pi/LQoSync3.0`. Branch: `lqosync3`. Base and pushed checkpoint: `6e5debd`.
- Commit `6e5debd` contains shared static attachments and budgets. The push succeeded.
- The new proposal slice remains local and uncommitted. Preserve unrelated operational edits and old notes in this file.
- Added `legacy_static.rs` and `legacy_static_tests.rs`. Reused tree-remap and snapshot parity helpers; extended coordinator tests and module registration.
- Updated design, discussion, plan, Home, and session. No dependencies changed.
- The pure API checks complete snapshot evidence and extracts only selected static roots and protected rows.
- Actual legacy parents must match explicit managed parent mappings. No branch reparenting occurs during adoption.
- Missing/overlapping roots, duplicate attachments, managed-node extraction, static identity collisions, and incomplete row classification fail.
- Existing managed adoption builds aliases. The helper reconstructs the full aliased snapshot and checks preservation, shared budgets, import, and output bounds.
- Configured adoption retains current source-filter proof and legacy policy preflight. Existing independent adoption APIs keep their behavior.
- Returned source drafts use schema 1, positive revisions, 100,000-row limits, and 16-MiB byte limits. Serialization borrows the payload.
- Dynamic subscriber endpoints enter neither registry policy nor the static draft. Restart/disconnect/reconnect tests use fresh IP/MAC with stable legacy aliases.
- Repeated fresh proposals are stable. Expired evidence fails. The helper performs no commit, source installation, output write, owner claim, or service action.
- Full suite: 1,167 passed, zero failed, 13 ignored. All 431 active topology tests pass. Added ten active tests and one optional native test.
- Format, Clippy with warnings denied, whitespace, and installed-library scratch compilation pass. Author review only; no production benchmark.
- Logs: `.superpowers/sdd/2026-11-17-topology-shared-adoption/`. Proof: `full.log`, `clippy.log`, `format.log`, and `native-adoption.log`.
- No live source read, router write, native apply, service action, deployment, or mode switch occurred.
- Next: controlled migration staging/recovery. The proposal does not prove later exact source installation or complete a migration transaction.
- Native implicit commitments, planner/live proof, enrollment UI, schedules, TUI, service activation, and schema-2 circuit ID migration remain gates.
- Session: [[2026-11-17-topology-shared-adoption]]. Plan: `docs/superpowers/plans/2026-11-17-topology-shared-adoption.md`.
- The graph remains the earlier baseline. Current files take precedence.

## 2026-11-17T18:36:55+08:00 (Asia/Shanghai) — Shared static attachments complete locally

- Agent: Codex `/root`. Worktree: `/home/pi/LQoSync3.0`. Branch: `lqosync3`. Base and pushed checkpoint: `f104356`.
- Commit `f104356` contains authoritative static-input binding. The push to `origin/lqosync3` succeeded. Remote tracking equals HEAD.
- The shared branch slice remains local and uncommitted. Preserve unrelated operational edits and older notes in this file.
- Added `shared_static.rs` and `shared_static_tests.rs`. Extended ownership, coordinator/compiler tests, and module registration.
- Updated design, discussion, plan, Home, and session. No dependencies changed.
- Optional `shared_attachments` maps unique protected roots to internal managed WAN/intermediate parent IDs.
- Source documents remain forests. Publication resolves parent aliases and inserts roots without changing static nodes or rows.
- Roots without attachments remain independent. Subscriber groups cannot receive children. Global IDs, labels, and depth checks remain mandatory.
- Shared checks use exported whole-Mbps capacities and check both directions. Child ceilings must fit immediate parents.
- Circuit minimums count once across device rows. Devices must agree on parent and rates. Mixed direct circuits/child groups fail.
- An attached static node without an explicit minimum reserves its full ceiling, including when it has no rows.
- Explicit guarantees must cover descendants. Reverse propagation counts each child obligation once. Shared obligations must fit parent/WAN budgets.
- Subscriber maximums remain unchanged; a test keeps 50 Mbps below a 20/10 Mbps static node.
- Schema 4 remains current. Empty fields preserve old serialization. Older strict readers reject nonempty attachment policy.
- The store canonicalizes completed edits. Attachments survive restart. Reparenting does not remove protected root ownership.
- Full suite: 1,157 passed, zero failed, 12 ignored. All 421 active topology tests pass. Added 12 active tests.
- Format, Clippy with warnings denied, whitespace, and installed-library scratch compilation pass. Author review only; no production benchmark.
- Logs: `.superpowers/sdd/2026-11-17-topology-shared-static/`. Proof: `full-verified.log`, `clippy-verified.log`, `format.log`, and `native-shared.log`.
- No live source read, router write, native apply, service action, deployment, or mode switch occurred.
- Placement weights and saved assignments remain unchanged. A new static reservation can expose an assignment conflict; no automatic rebalance occurs.
- Native implicit managed commitments and planner/live proof remain gates. Source checks do not lock external editors across writes.
- Next: exact shared legacy snapshot extraction and ownership adoption. Existing adoption helpers still classify independent roots only.
- Enrollment UI, schedules, TUI, service activation, and direct schema-2 circuit ID migration remain unfinished.
- Session: [[2026-11-17-topology-shared-static]]. Plan: `docs/superpowers/plans/2026-11-17-topology-shared-static.md`.
- The graph remains the earlier baseline. Current files take precedence.

## 2026-11-17T18:20:31+08:00 (Asia/Shanghai) — Independent static source complete locally

- Agent: Codex `/root`. Worktree: `/home/pi/LQoSync3.0`. Branch: `lqosync3`. Base and pushed checkpoint: `fa964c4`.
- Commit `fa964c4` contains explicit node minimums. The push to `origin/lqosync3` succeeded. Remote tracking equals HEAD.
- The static-input slice remains local and uncommitted. Preserve unrelated operational edits in this worktree and in this file.
- Added `static_input.rs`. Extended coordinator, publication, runner, receipts, compiler tests, apply-evidence tests, and receipt tests.
- Updated design, discussion, plan, Home, and session. No dependencies changed.
- Configured collection reads a separate private `topology-manager/static-input.json` only for declared protected ownership.
- The document contains schema version 1, a positive revision, rows, and the compatibility forest. Generated output never supplies this source.
- Reads require a private regular file and directory owned by the current user. Byte limit: 16 MiB. Decoder limit: 100,000 rows.
- Existing checks require exact protected rows and independent roots. Dynamic subscriber endpoints still require current observations.
- Source path, revision, and digest follow prepared/compiled pairs and receipts. Raw static previews cannot publish.
- Missing source fails before router connection. Drift during compile or before publication fails. Wrong runtime owner paths fail.
- The runner rejects revision conflicts before compile. Publication repeats the check before registry commit.
- Against the current receipt, lower revisions and changed bytes at the same revision fail. Equal output still gets a new receipt for a new binding.
- Old receipts without the optional field load. Older strict readers reject a nonempty new field. Receipt version remains 1.
- Apply evidence includes the entire context. Tests reject reuse after static path, revision, or digest changes.
- Full suite: 1,145 passed, zero failed, 12 ignored. All 409 active topology tests pass. Added eight active tests.
- Format, Clippy with warnings denied, whitespace, and installed-library scratch compilation pass. Author review only; no production benchmark.
- Logs: `.superpowers/sdd/2026-11-17-topology-static-input/`. Final proof: `full-verified.log`, `clippy-verified.log`, and `native-static.log`.
- No live source read, router write, native apply, service action, deployment, or mode switch occurred.
- Boundary checks do not lock an external editor across writes. A late source change can block file intent after a registry commit.
- Revision continuity uses the current receipt, not a permanent separate ledger. Existing recovery rules remain in use.
- Next: shared managed/static branch ownership and capacity budgets. Enrollment UI, schedules, TUI, service activation, and native planner/live proof remain gates.
- Direct schema-2 circuit activation still needs a durable device-ID length migration. Current publication remains legacy/manual.
- Session: [[2026-11-17-topology-static-input]]. Plan: `docs/superpowers/plans/2026-11-17-topology-static-input.md`.
- The graph remains the earlier baseline. Current files take precedence.

## 2026-11-17T17:56:39+08:00 (Asia/Shanghai) — Explicit node minimums complete locally

- Agent: Codex `/root`. Worktree: `/home/pi/LQoSync3.0`. Branch: `lqosync3`. Base: `e21f11d`.
- Checkpoint `e21f11d` commits DHCP overrides and dynamic bounds. The push to `origin/lqosync3` succeeded.
- Current node minimum changes remain local and uncommitted. Preserve unrelated operational edits.
- Extended rates, catalog, aggregate, publication, legacy preflight, and tests. Updated external test constructors.
- Updated design, discussion, plan, Home, and session.
- Explicit optional directions require positive whole Mbps. Empty policies preserve old serialization. Schema 4 remains current.
- Guaranteed budgets propagate iteratively from leaves to roots. Each child obligation counts once. Parent guarantees must cover descendants.
- Sibling sums and each propagated commitment must fit the effective parent/WAN ceilings. Export checks the quantized ceilings again.
- Automatic empty groups remain absent. Fixed empty branches reserve explicit minimums. No rate or guarantee is silently reduced.
- Export writes explicit compatibility minimum keys. Dormant preflight checks every source instance against the legacy node template.
- Compiler admission rejects changed or dropped minimum metadata. The installed-library scratch compiler preserves an explicit minimum.
- Full suite: 1,137 passed, zero failed, 12 ignored. All 402 topology tests pass. Added ten active tests.
- Format, Clippy with warnings denied, and whitespace checks pass. Author review only. No production benchmark occurred.
- No live source read, router write, native apply, service action, deployment, or mode switch occurred.
- These budgets cover explicit guarantees and subscriber minimums. Implicit native default commitments and planner/live proof remain gates.
- The schema-2 node writer test uses no circuits. Generated manager device IDs exceed its 64-character ID limit. Current publication remains legacy/manual.
- Next: authoritative static-input binding and source revisions. Shared branches, enrollment UI, schedules, service activation, and TUI remain unfinished.
- Session: [[2026-11-17-topology-node-minimums]]. Plan: `docs/superpowers/plans/2026-11-17-topology-node-minimums.md`.
- Logs: `.superpowers/sdd/2026-11-17-topology-node-minimums/`. Proof: `full-complete.log`, `clippy-complete.log`, and `native-minimum.log`.
- The graph remains the earlier baseline. Current files take precedence.

## 2026-11-17T17:38:44+08:00 (Asia/Shanghai) — Dynamic subscriber bounds complete locally

- Agent: Codex `/root`. Worktree: `/home/pi/LQoSync3.0`. Branch: `lqosync3`. Base: `7d3c95f`.
- DHCP identity overrides and this dynamic-bound slice remain local and uncommitted. Preserve unrelated operational edits.
- Added `legacy_bounds.rs` and tests. Extended rates, catalog validation, legacy preflight, coordinator tests, and module exports.
- Updated design, discussion, plan, Home, and session.
- The explicit importer produces four profile overrides. The existing revision-checked store persists completed edits.
- `LegacyRoundedBase` preserves legacy three-decimal Mbps rounding and floors. Current exact fixed/fraction rules and new defaults keep their behavior.
- Imported factors require exact six-decimal representation in `(0, 1]`. Configuration floors and exact rate conversions remain mandatory.
- The catalog rejects reversed factors/floors without live samples. Dormant preflight compares all effective controls after inheritance.
- Matching controls clear the dynamic-bound issue. Source priority, group factors/formulas, native minimums, ownership, and snapshot checks remain independent.
- Fresh PPP speeds change from 50/10 Mbps to 23/7 Mbps after disconnect, with minimums of 2.3/0.7 Mbps at 10%.
- The DHCP reconnect test also combines identity-bound 20/7 Mbps with minimums of 2/0.7 Mbps.
- Full suite: 1,127 passed, zero failed, 12 ignored. All 392 topology tests pass. Added nine active tests.
- The installed-library sync test passes in temporary directories. Format, Clippy with warnings denied, and whitespace checks pass.
- Author review only. No production benchmark, dependency change, or schema-version change. Older readers reject the new variant.
- No live source read, router write, service action, native apply, deployment, or mode switch occurred.
- Next: native group minimum commitments. Keep WAN ceilings authoritative and keep capacity floors separate from guaranteed minimums.
- Static-input authority, shared branches, enrollment UI, native planner admission, and live proof remain gates.
- Schedules, service activation, and the Topology Manager TUI remain unfinished.
- Session: [[2026-11-17-topology-dynamic-bounds]]. Plan: `docs/superpowers/plans/2026-11-17-topology-dynamic-bounds.md`.
- Logs: `.superpowers/sdd/2026-11-17-topology-dynamic-bounds/`. Proof: `full-complete.log`, `clippy-complete.log`, and `native-sync.log`.
- The graph remains the earlier baseline. Current source files take precedence.

## 2026-11-17T16:16:18+08:00 (Asia/Shanghai) — DHCP identity overrides complete locally

- Agent: Codex `/root`. Worktree: `/home/pi/LQoSync3.0`. Branch: `lqosync3`. Base: `7d3c95f`.
- The exclusion checkpoint is committed and pushed as `7d3c95f`. Current DHCP changes remain local and uncommitted.
- Added `src/topology_manager/dhcp_overrides.rs`. Extended source policy, model, identity, settings, preflight, publication, runner assertion, and tests.
- Updated design, discussion, plan, Home, and session. Preserve unrelated operational edits.
- Explicit bindings map legacy rate-list positions to enrolled independent account references and exact rates. No endpoint identifies a subscriber.
- Complete coverage spans disjoint selected DHCP scopes per router. Duplicate positions/references, unknown identities, conflicting aliases, changed rates, and local bases block collection.
- Independent authorization precedes override selection. Offline removal and fresh reconnects preserve policy without stored endpoint reuse.
- A second subscriber with the old MAC does not inherit the override. The existing legacy collector remains unchanged.
- Prepared output binds to configured override rates. Changed or removed rates fail the existing pre-compilation/publication/adoption fence.
- Identity/profile indexes and one rate-list read per router avoid repeated scans. No production benchmark occurred.
- Full suite: 1,118 passed, zero failed, 12 ignored. All 383 topology tests pass. Added ten active tests.
- The optional installed-library sync test passes in temporary directories. Format, Clippy with warnings denied, and whitespace checks pass.
- Author review only. No dependency or schema-version change. Older readers reject a nonempty unknown policy field.
- No live source read, router write, service action, native apply, deployment, or live mode switch occurred.
- Next: dynamic subscriber min/max bounds migration. Source-read profiles still block configured legacy adoption on that requirement.
- Native group minimums, static-input authority, shared branches, enrollment UI, native planner bounds, and live proof remain gates.
- Schedules, service activation, and the Topology Manager TUI remain unfinished.
- Session: [[2026-11-17-topology-dhcp-overrides]]. Plan: `docs/superpowers/plans/2026-11-17-topology-dhcp-overrides.md`.
- Logs: `.superpowers/sdd/2026-11-17-topology-dhcp-overrides/`. Proof: `full-complete.log`, `clippy-complete.log`, and `native-sync.log`.
- The graph remains the earlier baseline. Current files take precedence.

## 2026-11-17T15:12:42+08:00 (Asia/Shanghai) — Current-source exclusions complete locally

- Agent: Codex `/root`. Worktree: `/home/pi/LQoSync3.0`. Branch: `lqosync3`. Base: `5528f9e`.
- The preflight checkpoint is committed and pushed as `5528f9e`. Current exclusion changes remain local and uncommitted.
- Shared the matcher in sources. Extended normalization, coordinator, preflight, publication, configured adoption, runner, and tests. Added exclusion tests.
- Updated design, discussion, plan, Home, and session. Preserve unrelated operational edits.
- Valid global/router IPv4, PPP profile, and DHCP server exclusions now apply to fresh sessions. Invalid relevant rules still block before connection.
- Full row validation and duplicate-IP checks precede exclusion. Filtered sessions leave no active comments or service membership.
- Inventory remains available. Excluded accounts do not need identity enrollment. IPv4 rules do not exclude IPv6 sessions.
- Complete exclusions remove rows and automatic child groups. Fresh reconnects preserve independent Subscriber IDs with current endpoints.
- Prepared output carries a transient filter digest. Configured publication/adoption reject missing or stale proof, also after filter removal.
- Shared global lists, indexed name checks, and parsed IPv4 lookup avoid repeated work. No benchmark occurred.
- Full suite: 1,108 passed, zero failed, 12 ignored. All 373 topology tests pass. Added 12 active tests.
- The optional installed-library sync test passes in temporary directories with excluded, active, disconnected, and reconnected batches.
- Format, Clippy with warnings denied, and whitespace checks pass. Author review only.
- No dependency/schema change, live source read, router write, service action, native apply, deployment, or live mode switch occurred.
- Next: explicit durable-identity mappings for DHCP overrides, with offline/reconnect and conflict tests. Do not infer identity from endpoints.
- Dynamic bounds, native group minimums, static-input authority, shared branches, enrollment, native planner bounds, and live proof remain gates.
- Session: [[2026-11-17-topology-exclusions]]. Plan: `docs/superpowers/plans/2026-11-17-topology-exclusions.md`.
- Logs: `.superpowers/sdd/2026-11-17-topology-exclusions/`. Proof: `full.log`, `clippy.log`, `native-sync.log`, `topology-final.log`, and `publication.log`.
- Graph remains the earlier baseline. Current files take precedence.

## 2026-11-17T14:54:12+08:00 (Asia/Shanghai) — Legacy configuration preflight complete locally

- Agent: Codex `/root`. Worktree: `/home/pi/LQoSync3.0`. Branch: `lqosync3`. Base: `e222547`.
- The source coordinator is committed and pushed as `e222547`. Current preflight changes remain local and uncommitted.
- Added preflight and preflight tests. Extended coordinator/tests, module exports, ownership adoption, and the optional native runner test.
- Updated design, discussion, plan, Home, and session. Preserve unrelated operational edits.
- Configured sync blocks unsupported selected-source exclusions, DHCP client overrides, and incompatible DHCP membership before connection.
- Bound-only DHCP requires `require_bound = true` and `shape_unassigned = false`. Without discovery, selected service names must equal the configured server set.
- `adopt_configured_legacy` checks dormant source priorities, local bounds, group formulas/factors, caps, and native minimum declarations before exact snapshot adoption.
- Dynamic bounds and unsupported declarations remain explicit issues. A clear report is not activation authority.
- Factor comparison uses integer products and exact configured conversion at six decimal places. Profile/assignment indexes avoid repeated subscriber scans.
- Full suite: 1,096 passed, zero failed, 12 ignored. All 11 preflight and 14 coordinator tests pass. Added 12 active tests.
- The optional installed-library sync test passes in temporary directories, including exclusion rejection before pair publication.
- Format, Clippy with warnings denied, and whitespace checks pass. Author review only. No benchmark occurred.
- No dependency/schema change, live source read, router write, service action, native apply, deployment, or live mode switch occurred.
- Next: exclusion filters on current evidence with parity tests. Keep MAC overrides blocked until explicit independent identity mappings exist.
- Dynamic bounds, node minimums, static-input authority, shared static branches, enrollment, native planner bounds, and live DPDK proof remain gates.
- Session: [[2026-11-17-topology-legacy-preflight]]. Plan: `docs/superpowers/plans/2026-11-17-topology-legacy-preflight.md`.
- Logs: `.superpowers/sdd/2026-11-17-topology-legacy-preflight/`. Proof: `full.log`, `clippy.log`, `verified.log`, `coordinator.log`, and `native-sync.log`.
- Graph remains the earlier baseline. Current files take precedence.

## 2026-11-17T11:19:15+08:00 (Asia/Shanghai) — Configured source coordinator complete locally

- Agent: Codex `/root`. Worktree: `/home/pi/LQoSync3.0`. Branch: `lqosync3`. Base: `16d2b80`.
- The runtime-owner checkpoint is pushed. Current task changes remain local and uncommitted.
- Added coordinator and coordinator tests under `src/topology_manager/`. Extended reader, module exports, runner, and runner tests.
- Updated implementation design, discussion, task plan, Home, and session. Preserve unrelated operational edits.
- `Runner::sync_topology` connects configured source reads, authorization, compilation, and publication through the shared writer owner.
- Catalog router names require one exact configured connection. Selected protocols require enabled settings and catalog roles.
- Parallel reads share one deadline. Failed or cancelled batches cannot reuse stored subscriber observations.
- A reproduced stale-session defect is fixed. Current connection fields govern reuse. Config reloads close all coordinator sessions.
- Generations advance before reads and beyond the recovered receipt after restart. Current endpoints remain in memory only.
- Full suite: 1,084 passed, zero failed, 12 ignored. All 13 coordinator tests pass. Added 15 active tests.
- The optional installed-library sync test passes in temporary directories. It proves failure fencing, restart, disconnect, and fresh reconnect output.
- Format, Clippy with warnings denied, and whitespace checks pass. Author review only. No benchmark or hard transport memory bound is claimed.
- No dependency or live service loop changed. No live RouterOS read, native apply, routing change, router write, or deployment occurred.
- Next: legacy-policy parity and activation preflight. Cover exclusions, DHCP address overrides, minimum/factor rules, and static-input authority.
- Shared static branches, independent enrollment, native planner bounds, and live DPDK proof remain activation gates.
- TUI, schedules, and complete recovery from a corrupt store remain separate tasks.
- Session: [[2026-11-17-topology-source-coordinator]]. Plan: `docs/superpowers/plans/2026-11-17-topology-source-coordinator.md`.
- Logs: `.superpowers/sdd/2026-11-17-topology-source-coordinator/`. Proof: `full.log`, `clippy.log`, `coordinator-verified.log`, and `native-sync.log`.
- Graph remains the earlier baseline. Current files take precedence.

## 2026-11-17T10:58:55+08:00 (Asia/Shanghai) — Controlled compiler and runtime owner complete locally

- Agent: Codex `/root`. Worktree: `/home/pi/LQoSync3.0`. Branch: `lqosync3`. Base: `b25e0cf53162a4d05e217b2e6fece39fe4efa130`.
- Current task changes remain local and uncommitted. Preserve unrelated operational edits.
- Added compiler, compiler sandbox, compiler tests, and runtime ownership modules under `src/topology_manager/`.
- Extended files, runner, store, admission, publication, receipts, apply verification, and receipt tests.
- Updated design, discussion, plan, Home, and session. No dependency changed.
- `Runner::publish_topology` uses the shared instance lock, registry, and receipt tracker.
- The compiler checks exact source/config bytes, deadlines, private artifacts, and effective config identity.
- Kernel controls deny socket operations and content writes outside the private run workspace.
- A durable marker blocks legacy publication after restart. Old executables require an explicit downgrade migration.
- Automatic scheduler requests reuse receipt identity. Pending receipts retain the existing native proof requirements.
- Full suite: 1,069 passed, zero failed, 11 ignored. All 337 topology tests pass.
- Both optional installed-library tests pass in temporary directories. Format, Clippy, and whitespace checks pass.
- Native config migration requires private workspace writes. Original config and library remain unchanged.
- Author review only. No live activation, deployment, service action, router write, native apply, or benchmark occurred.
- Next: configured source reads and identity authorization in the service loop, then legacy parity and activation preflight.
- Legacy policy parity, native planner bounds, independent enrollment, and live DPDK proof remain activation gates.
- TUI, schedules, and full corrupt-store reconstruction remain separate tasks.
- Session: [[2026-11-17-topology-runtime-owner]]. Plan: `docs/superpowers/plans/2026-11-17-topology-runtime-owner.md`.
- Logs: `.superpowers/sdd/2026-11-17-topology-runtime-owner/`. Final results: `full-verified.log` and `clippy-final.log`.
- The graph remains the earlier baseline. Current files take precedence.


## 2026-11-17T07:16:59+08:00 (Asia/Shanghai) — Durable aliases and base source priorities complete locally

- Agent: Codex `/root`. Worktree: `/home/pi/LQoSync3.0`. Branch: `lqosync3`. Base: `728c2c2`.
- The owner authorized this task after the GitHub checkpoint. No commit or push occurred in this task.
- Added ownership/legacy_rate modules and tests. Extended model/store, publication, migration, source policy, current metadata, and reader fields.
- Updated related tests, design, discussion, plan, Home, and session. Preserve unrelated operational work.
- Schema 4 stores explicit export aliases and static ownership IDs. Strict schema-1/2/3 adapters preserve old primary bytes until a changed commit.
- Exact pre-upgrade backups remain available. Normal history keeps 16 schema-4 backups. Current identities, counters, and export reservations survive recovery.
- Circuit/device/node aliases preserve legacy output IDs and labels. Device aliases require independent identity references, not endpoints or saved sessions.
- adopt_legacy requires explicit mappings and managed-policy parity at the exact revision, generation, fingerprints, and original deadline.
- Managed disconnects emit no rows. Reconnects use current IP/MAC data. Ambiguous sessions cannot claim one aliased device ID.
- Current authoritative static input can preserve declared independent roots. Missing rows/roots, absent parents, unclassified data, and managed-ID collisions block publication.
- Static source/revision verification belongs to the future runtime owner. Shared static branches still require capacity rules.
- Source conversion preserves configured PPP, DHCP-server, and Hotspot base priorities, order, units, direction, name rules, and explicit defaults.
- Dynamic comments/rates remain in memory. Lossy integer-bps conversion, conflicting session speeds, or competing source authorities block acceptance.
- Shared router indexes avoid a reproduced quadratic copy. Ownership checks use indexed membership. No production benchmark claim.
- Added 19 tests. Full suite: 1,050 passed, zero failed, 9 ignored. All 328 topology tests pass. Format, Clippy, and whitespace pass.
- A final fixture-only Clippy cleanup passed all 10 ownership tests. Author review only; no independent reviewer participated.
- No dependency, legacy runtime/config parser, live caller, deployment, native apply, service action, or router write changed.
- Next: controlled compiler/configuration binding and one runtime lock/writer owner. Preserve deadlines, pending receipts, and post-write proof.
- Complete legacy shadow parity for exclusions, address-based overrides, minimum/factor policies, and shared static branches before activation.
- Enrollment, native planner bounds, TUI, schedules, corrupt-store reconstruction, and live DPDK proof remain gates.
- Session: [[2026-11-17-topology-legacy-adoption]]. Plan: `docs/superpowers/plans/2026-11-17-topology-legacy-adoption.md`.
- Logs: `.superpowers/sdd/2026-11-17-topology-legacy-adoption/`. Graph remains the earlier baseline.


## 2026-11-17T06:53:28+08:00 (Asia/Shanghai) — Topology checkpoint for GitHub

- The owner authorized a commit and push before further implementation.
- This checkpoint includes normalization, identity authorization, source policy, settings recovery, publication, explicit mappings, and compiled admission.
- Checks: 1,031 passed, zero failed, 9 ignored. Format, Clippy, whitespace, and shell syntax passed.
- Preserve unrelated operational edits outside this commit. No deployment or live apply occurs.
- Next: durable legacy ownership adoption and source-priority conversion. Runtime integration remains gated.





## 2026-11-17T06:28:03+08:00 (Asia/Shanghai) — Explicit mappings and compiled admission complete locally

- Agent: Codex `/root`. Worktree: `/home/pi/LQoSync3.0`, branch `lqosync3`, base `489acc7` plus prior local slices.
- Owner directs continuation. Added migration/admission modules and tests. Extended parity, publication, publication tests, and public topology integration.
- Added topology_compilation_check example and strengthened tools/v3sim/check.sh. Extended import_evidence with borrowed comparison parts.
- Updated design, discussion, plan, precision report, Home, session, and this handoff. Preserve unrelated operational work and all prior local slices.
- Manifests declare explicit row/node ID/label mappings plus protected row IDs. They bind the revision, observation generation, and baseline/candidate fingerprints.
- No endpoint/name-based identity inference or registry ownership transfer occurs. Rates, endpoints, hierarchy, and metadata remain visible.
- Unmapped/protected baseline rows produce explicit issues even when export IDs match. Duplicate, many-to-one, stale, or conflicting ownership is rejected.
- Manifests are evidence for one snapshot, not durable enrollment or a session alias archive. Durable export aliases and static preservation remain unfinished.
- Only AdmittedPair exposes the public publisher. Fresh prepared pairs require exact canonical/compiled shaping checks before commit/write.
- Checks cover artifact schema/source/ingress, full device policy/endpoints, nodes, parents, intrinsic rates, attachments, and unrequested movement/matching extensions.
- Pair budget: 16 MiB. Combined artifacts: 64 MiB. Readers enforce combined bounds. Borrowed import comparison and one cloned/remapped network avoid extra full artifact copies.
- Installed package 3.0.7.202611162151 accepts manager-style IDs. The old harness lq3-prefix assertion was too narrow and is replaced by exact policy comparison.
- Scratch compilation found node 100.5 Mbps becomes intrinsic 101 Mbps. Success alone is insufficient. Admission correctly rejects this loss.
- The adapter applies approved ceiling-safe whole-Mbps node export: 100.5 calculated -> 100 exported. It exposes both values, keeps subscriber rates, and rejects lost minimum commitments/group floors.
- Final scratch check: node 100 Mbps and subscriber max 50.5 Mbps preserve policy. Socket connections were blocked and the write audit passed. No native world was applied.
- Added 19 checks. Full suite: 1,031 passed, zero failed, 9 ignored. All 309 topology checks pass. Format, Clippy, whitespace, and shell syntax pass.
- The public restart integration now passes compiled admission before temporary pair publication and the existing Prepared builder.
- Author review fixed a reproduced offline duplicate-node-ID collapse. No independent reviewer participated. No code finding remains open in this slice.
- No dependency, schema, legacy config/source parser, router reader, cycle, live caller, deployment, service, or router write changed. No commit or push occurred.
- All new slices remain local/uncommitted. Next: durable legacy alias/label/static ownership adoption and source-priority conversion with parity.
- Then wire the controlled compiler and one runtime lock/writer owner. Bind compiler/configuration identity and preserve fresh deadlines and pending receipt recovery.
- Full shaping-planner admission, enrollment, TUI, schedules, corrupt-store reconstruction, and live DPDK acceptance remain activation gates.
- Session: [[2026-11-17-topology-migration-admission]]. Plan: `docs/superpowers/plans/2026-11-17-topology-migration-admission.md`.
- Precision report: `docs/reports/2026-11-17-native-node-export-precision.md`. Logs: `.superpowers/sdd/2026-11-17-topology-migration-admission/`.

## 2026-11-17T05:38:14+08:00 (Asia/Shanghai) — Isolated publication and parity complete locally

- Agent: Codex `/root`. Worktree: `/home/pi/LQoSync3.0`, branch `lqosync3`, base `489acc7` plus prior local slices.
- Owner directs continuation. Added publication.rs/publication_tests.rs and parity.rs/parity_tests.rs. Extended module exports.
- Extended write_receipt and apply_evidence/verify for optional TopologyContext. Updated runner/outbox fixtures, receipt/public topology tests, and two examples.
- Updated design, discussion, plan, Home, this handoff, and the linked session. Preserve unrelated operational work and prior local slices.
- Preparation consumes authorized reconciliation and reuses placement, aggregate checks, pair validation/renderers, and import normalization.
- Exact decimal CSV rates must recover the same integer bps through f32 import. Precision loss blocks rather than changing subscriber policy.
- Effective node ceilings preserve WAN bounds. Distinct label/ID keys support repeated labels. Synthetic root remains policy metadata. Empty automatic groups disappear.
- Current sessions export distinct device rows under one subscriber circuit. Display names use IDs until explicit label migration exists. No archived endpoints supply rows.
- The JSON import depth bound is 63 node levels. It is checked before recursive serialization/validation.
- Publication commits registry intent, rechecks the original deadline, then uses the current tracker in network-first order. Partial outcomes remain explicit.
- Optional context binds policy revision, observation generation, and registry digest into Prepared/Verified proof correlation. Existing native checks remain intact.
- Matching unchanged content keeps its receipt identity and original context. Matching files without a receipt create a trackable attempt with unchanged outcomes.
- Fixed two reproduced shared tracker defects: durable intent failure no longer permits file writes; unchanged inputs without a matching receipt no longer remain untrackable.
- Optional event-history faults still degrade notification health without blocking shaping. These shared tracker fixes intentionally affect the legacy path.
- Parity reports changed/added/missing/duplicate rows and changed network content. Unmanaged baseline rows remain visible. No IP/MAC-based identity mapping occurs.
- Added 19 checks. Full suite: 1,012 passed, zero failed, 9 ignored. All 290 topology checks pass. Format, Clippy with denied warnings, and whitespace pass.
- Public restart proof reaches temporary pair publication and the existing Prepared builder with saved Local/SourceRead settings and current endpoints.
- Native proof checks use fixtures. No live native admission or DPDK apply test occurred. Author review only; no independent reviewer participated.
- No manager caller was added to the runner. Its only change is optional metadata initialization. No dependencies, config/source parser, router reader, or cycle changed.
- All new slices remain local/uncommitted. No push, deployment, live configuration, service, or router write occurred.
- Next: explicit legacy identity/label/static ownership conversion and source-policy parity. Then exact native rate/size admission and runtime owner wiring.
- Enrollment, TUI, schedules, and full corrupt-store identity reconstruction remain gated. Do not activate manager mode before its gates pass.
- Session: [[2026-11-17-topology-publication]]. Plan: `docs/superpowers/plans/2026-11-17-topology-publication.md`.
- Logs: `.superpowers/sdd/2026-11-17-topology-publication/`.

## 2026-11-17T04:40:31+08:00 (Asia/Shanghai) — Durable settings and forward recovery complete locally

- Agent: Codex `/root`. Worktree: `/home/pi/LQoSync3.0`, branch `lqosync3`, base `489acc7` plus prior local slices.
- Owner directs continuation. Added settings.rs/settings_tests.rs and store_recovery_tests.rs. Extended model/store to schema 3.
- Added strict serialization to source selectors, authority, policy bindings, and exact rate formats. Legacy config enums remain unchanged.
- Added authorize_configured and changed the public restart proof to consume persisted settings from the expected revision.
- Updated design, discussion, plan, Home, this handoff, and the linked session. Preserve unrelated operational changes and prior local slices.
- Settings/catalog validation rejects missing scopes, duplicate/overlapping selectors, and missing selected profile bindings. Unconfigured authority stays a draft and blocks configured acceptance.
- Strict schema-1/2 load preserves IDs, intent, references, counters, and catalog policy without rewriting disk or inventing source authority.
- Changed commits require exact verified private backups before primary replacement. Backup publication uses a synced temporary file and non-replacing hard link.
- History keeps 16 recent schema-3 backups plus backups before migration. Normal forward upgrades create at most one backup per old schema.
- Optional cleanup errors set history_cleanup_pending. Open repeats the check so unresolved errors remain visible after restart.
- restore_policy creates a forward revision with old catalog/source settings and current subscriber records/pins/counter. Incompatible references block recovery.
- Corrupt/missing primary files still block open. Old snapshots alone cannot authorize identity/counter rewind. Full reconstruction requires independently verified complete evidence.
- Added 20 checks. Full suite: 993 passed, zero failed, 9 ignored. Format, Clippy with denied warnings, and whitespace passed.
- The source has 276 topology checks. A reversed 10,000-profile case proves equivalent canonical output, not production throughput.
- Author review only. No independent reviewer participated. No code finding remains open in this slice.
- All new slices remain local/uncommitted. No dependency, legacy runtime/config/parser/reader, live caller, push, deployment, router write, or service action changed.
- Next: publication boundary plus explicit legacy-policy parity migration. Keep source deadlines and prove native bounds/export precision, admission, and correlated DPDK evidence.
- Then connect TUI and schedules. Live enrollment, full corrupt-store reconstruction, and manager-mode activation remain gated.
- Session: [[2026-11-17-topology-settings-recovery]]. Plan: `docs/superpowers/plans/2026-11-17-topology-settings-recovery.md`.
- Logs: `.superpowers/sdd/2026-11-17-topology-settings-recovery/`.

## 2026-11-17T03:18:43+08:00 (Asia/Shanghai) — Source mapping and exact rates complete locally

- Agent: Codex `/root`. Worktree: `/home/pi/LQoSync3.0`, branch `lqosync3`, base `489acc7` plus local normalization/identity.
- Owner directs continuation. Added source_rate/source_policy modules and tests. Extended identity selections, normalized service metadata, Hotspot inventory reads, and public integration proof.
- Updated module exports, reader/normalizer/identity tests, design, discussion, source-policy plan, Home, session, and this handoff. Preserve unrelated work and prior local slices.
- SourceSelection now uses `service_names` and an explicit SourcePolicy. No serialized settings or live callers used the earlier isolated `services` field.
- Fresh DHCP/Hotspot name indexes resolve current locators. Unknown selectors fail instead of asserting an empty source. PPP uses documented transport names.
- Hotspot requires `/ip/hotspot` inventory before/after active reads, with two extra read-only commands. Active server membership/disabled checks prevent incomplete provenance.
- Explicit profile-name bindings provide stable source references. Missing active mappings and invalid enabled rates withhold source acceptance. Selected catalog filters require configured bindings.
- Disabled/RateLimit modes are per mapped profile, with an explicit unprofiled/direct lease mode. Missing rates remain absent.
- Exact integer parser supports sustained pairs/symmetric single values, configured direction/unitless units, decimals, and k/M/G case compatibility. No float, round, clamp, or default speed.
- The public restart/store/placement/capacity test now covers both Local and SourceRead authority. Fresh locator mapping preserves the 50/10 Mbps subscriber max and valid saved pin.
- Missing SourceRead speed blocks the resolver. A negative assertion initially entered aggregate evaluation after that block; the corrected assertion and final full suite pass.
- Added 23 checks. All 256 topology checks pass. Full suite: 973 passed, zero failed, 9 ignored. Format and Clippy with denied warnings passed.
- Synthetic evidence: reversed 10,000-account/10,000-service results remain equivalent; 2,000 decimal inputs match exact bps. No production benchmark claim.
- All new slices remain local/uncommitted. No dependency, durable schema, legacy path, live caller, commit, push, deployment, router write, service, or configuration change occurred.
- Next action: durable source/authority settings and migration/recovery. Then publication/native integration, TUI, and schedules.
- Legacy comment/name/default speed priorities need explicit migration. Native bounds/export precision, retained deadlines, admission, and DPDK proof remain activation gates.
- Session: [[2026-11-17-topology-source-policy]]. Plan: `docs/superpowers/plans/2026-11-17-topology-source-policy.md`.
- Logs: `.superpowers/sdd/2026-11-17-topology-source-policy/`.

## 2026-11-17T02:38:16+08:00 (Asia/Shanghai) — Identity authorization complete locally

- Agent: Codex `/root`. Worktree: `/home/pi/LQoSync3.0`, branch `lqosync3`, base `489acc7` plus local normalization.
- Owner approves identity authorization. Added `identity.rs`/`identity_tests.rs`, normalized provenance/deadline metadata, module export, and a public catalog integration test.
- Updated design, discussion, source/identity plans, Home, session, and this handoff. Preserve unrelated work and prior uncommitted normalization.
- Opt-in operator reference: `[lqosync-id:REFERENCE]` in current administrator-managed account/lease comments. Namespace: `operator-comment/v1/`.
- The contract requires independent references, uniqueness, and non-reassignment within each source scope. No authority default or automatic router enrollment.
- Missing/invalid/duplicate references, ambiguous scopes, and partial multi-service membership withhold complete source acceptance. No endpoint/name/locator fallback.
- Require expected registry revision, exact selected scopes, current router generations, selected protocol provenance, and the original read deadline.
- Reuse `ObservationGate` and registry reconciliation. Return the deadline with the current world. Preserve saved intent only through verified references.
- Cross-router/protocol equal references remain separate unless existing explicit registry aliases link them. Offline entries provide no endpoints.
- Corrected a reproduced integrity defect: trial Hotspot sessions could borrow a matching local user's metadata. Trial sessions now have no such authority/profile.
- Added 23 checks: 22 unit checks plus public restart/offline/rename/return/store/placement/capacity proof with unchanged subscriber max.
- Full suite: 950 passed, zero failed, 9 ignored. Format and Clippy with denied warnings passed. Whitespace and stub scans passed.
- Indexed service matching handles a reversed 10,000-account/10,000-service check with equivalent IDs and output. No production benchmark claim.
- No dependency, schema, legacy path, live caller, commit, push, deployment, router write, service, or configuration changed in this slice.
- Trusted reference assertions can be false. Software cannot detect deliberate or accidental tag reassignment during an unobserved replacement.
- Next action: source-scope mapping and exact source-rate integration. Then durable authority settings/enrollment, migration/recovery, publication, TUI, and schedules.
- Do not invent rates for SourceRead authority. Carry/recheck the deadline through later stages. Native admission/DPDK and manager-mode activation remain gated.
- Session: [[2026-11-17-topology-identity]]. Plan: `docs/superpowers/plans/2026-11-17-topology-identity.md`.
- Logs: `.superpowers/sdd/2026-11-17-topology-identity/`.

## 2026-11-17T00:02:43+08:00 (Asia/Shanghai) — Current protocol normalization complete locally

- Agent: Codex `/root`. Worktree: `/home/pi/LQoSync3.0`, branch `lqosync3`, base `489acc7`.
- Prior topology checkpoint `489acc7c1e48760935e0131fce35f09adecde6c9` was committed, pushed, and verified on GitHub at the owner's request.
- The owner then requested continuation. Added `normalize.rs`/`normalize_tests.rs`, internal reader metadata access, and module export.
- Updated design, discussion, source/normalization plans, Home, session, and this handoff. Preserve unrelated edits.
- Normalize current PPP/Hotspot local account/profile joins and current bound DHCP endpoints. Group simultaneous local-account sessions once.
- MAC/trial and DHCP evidence remain endpoint-only. Names, MACs, IPs, client IDs, and row locators do not establish durable identity continuity.
- Reject ambiguous/disabled/missing required accounts, unknown methods/states, unusable endpoints, unresolved servers, and current IP ownership conflicts.
- Keep raw current rates without guessed defaults. Keep the original read deadline and check it on normalized data access.
- Review regression fixed the IP-conflict row location after session grouping. Mixed named/MAC authority stays endpoint-only independent of order.
- Added 24 checks. Full suite: 927 passed, zero failed, 9 ignored. Format and Clippy with denied warnings passed.
- A reversed 10,000-account check proves output equivalence, not production throughput. No new dependency or legacy path changed.
- This slice is local and uncommitted. No live caller, registry write, source acceptance, deployment, router write, service, or configuration change occurred.
- Next action: durable identity authorization and account lifecycle/conflict rules. Then source-scope/rate integration, migration/recovery, publication, TUI, and schedules.
- Local account evidence is not an immutable account incarnation. Keep critical conflicts when continuity cannot be proved.
- Session: [[2026-11-17-topology-normalization]]. Plan: `docs/superpowers/plans/2026-11-16-topology-normalization.md`.
- Logs: `.superpowers/sdd/2026-11-16-topology-normalization/`. Complete source acceptance remains unfinished.

## 2026-11-16T21:50:35+08:00 (Asia/Shanghai) — Topology implementation commit authorized

- Owner requests commit and push to GitHub. Agent: Codex `/root`.
- Worktree: `/home/pi/LQoSync3.0`. Branch: `lqosync3`.
- Commit scope: isolated core, registry, catalog, placement, aggregates, scoped rebalance, IDs, and read guards.
- Include their tests, plans, design updates, and session notes. Preserve unrelated operational work outside the commit.
- Last checks: 903 passed, zero failed, 9 ignored. Format and Clippy with denied warnings passed.
- Next: protocol normalization and account continuity checks. The manager still has no live caller.
- Git push changes repository history only. No deployment or live configuration change is authorized by this request.

## 2026-11-16T21:47:14+08:00 (Asia/Shanghai) — ID and read boundary complete locally

- Agent: Codex `/root`. Worktree: `/home/pi/LQoSync3.0`, branch `lqosync3`, base `b94ed7c` plus prior uncommitted slices.
- Added `ids.rs`/`ids_tests.rs` and `reader.rs`/`reader_tests.rs`. Updated model validation, exports, design, discussion, plan, and brain notes.
- Catalog ID batches use the shared durable counter and expected revisions. Restart and deletion do not reuse generated IDs.
- Role-based reads check configuration before/after active tables, reply completeness, row locators, age, timeout, and advancing generations.
- No passwords or old configured DHCP endpoint fallbacks. Error/cancellation drops the API session before retry. Successful sessions remain reusable.
- Full suite: 903 passed, zero failed, 9 ignored. Format and Clippy with denied warnings passed. Added 19 checks.
- Synthetic checks cover 10,000 IDs and 10,000 reordered rows. No production benchmark claim.
- Owner identifies PPP secrets, current DHCP hostname/MAC, and Hotspot active rows. No RADIUS service exists or is required for local reads.
- Those tables do not prove ambiguous identity continuity. Keep old pins/policies isolated when account replacement or DHCP changes cannot be verified.
- No protocol normalizer, complete source acceptance, or live caller exists yet. Multiple commands do not form an atomic RouterOS snapshot.
- No commit, push, deployment, dependency, legacy reader, router, service, or configuration changed. Preserve unrelated edits and all prior uncommitted slices.
- Next action: typed protocol normalization and explicit account lifecycle/conflict rules. Do not invent a RADIUS dependency or use raw row locators as permanent identities.
- Then complete migration/recovery, export/native integration, TUI, and schedules. Scheduler overlap/overnight/DST decisions remain open.
- Session: [[2026-11-16-topology-source-boundary]]. Plan: `docs/superpowers/plans/2026-11-16-topology-source-boundary.md`.
- Logs: `.superpowers/sdd/2026-11-16-topology-source-boundary/`. Task 3 normalization remains open by design.

## 2026-11-16T21:21:39+08:00 (Asia/Shanghai) — Scoped rebalance complete locally

- Agent: Codex `/root`. Worktree: `/home/pi/LQoSync3.0`, branch `lqosync3`, base `b94ed7c` plus prior uncommitted slices.
- Owner directs continuation. Added `rebalance.rs`/`rebalance_tests.rs`, resolver child-module declaration, root public alias, and a catalog public integration test.
- The explicit request requires an expected revision and a source-pool or branch ID. It consumes fresh reconciliation and reuses normal resolver validation.
- Normal resolution assigns new circuits first. Proposed moves refer to that baseline and affect current unpinned circuits within selected destinations only.
- Pins, offline intent, and outside-scope assignments stay fixed and contribute demand. Source pools also check current membership.
- One deterministic sweep favors larger plans, then stable IDs. Strict bottleneck checks prevent equal-quality movement from indivisible plans.
- Sparse exact score deltas rank improving moves. The final proposal passes aggregate/minimum/floor checks; conflicts withhold its candidate.
- Mixed bounded/unbounded scopes require compatible explicit capacity policy. No invented weights or implicit scope expansion.
- Added 25 checks. Full suite: 884 passed, zero failed, 9 ignored. Format and Clippy with denied warnings passed.
- Synthetic proof: 10,000 circuits and no moves on repeat. Score oracle checks 768 trials against full recomputation, including shared ancestors. No production benchmark claim.
- API performs no durable writes or publication. Public integration proves revision-checked persistence, pins, restart, and no stored session endpoints.
- No live caller, dependency, original fixture expectation, legacy builder, writer, router, service, configuration, commit, push, or deployment changed. Preserve unrelated edits.
- Next: verified source/identity adapters and stable ID creation, then complete migration/recovery, export/native integration, and TUI.
- One sweep does not prove a global optimum or globally minimum move count. Scheduler overlap/overnight/DST decisions remain open.
- Session: [[2026-11-16-topology-rebalance]]. Plan: `docs/superpowers/plans/2026-11-16-topology-rebalance.md`. Logs under its `.superpowers/sdd/` workspace.

## 2026-11-16T20:58:59+08:00 (Asia/Shanghai) — Aggregate boundary complete locally

- Agent: Codex `/root`. Worktree: `/home/pi/LQoSync3.0`, branch `lqosync3`, base `b94ed7c` plus prior uncommitted slices.
- Owner directs the next task. Added `aggregate.rs`/`aggregate_tests.rs`; updated module export, rate helper visibility, catalog floor input, and catalog/foundation tests.
- The stage consumes fresh resolved placements. It computes direct formulas, applies factors/backstops/floors, and constrains capacities by every ancestor.
- Demand/minimum sums count each circuit once. They support checks and display without recalculating intermediate capacities from subscriber plans.
- Empty automatic branches are omitted; manual/WAN nodes and durable definitions remain. Subscriber maxima stay unchanged.
- Oversubscription is a warning. Impossible minima or floors produce typed node conflicts and withhold the candidate.
- Optional source floor defaults to None and omits its absent field. A configured floor still requires a compatible reader and a verified migration/rollback path.
- Final review fixed acceptance of a stage input after current endpoint evidence was stripped. Conflict reason order is stable per node.
- Added 35 checks. Full suite: 859 passed, zero failed, 9 ignored. Format and Clippy with denied warnings passed.
- Public parity covers legacy PPP factors/backstop/floor. A reversed 20,000-level graph proves recursion safety, not production throughput.
- No live caller, original fixture expectation, dependency, legacy builder, writer, service, router, configuration, commit, push, or deployment changed. Preserve unrelated edits.
- Next: explicit scoped rebalance, then verified adapters/ID creation, complete migration/recovery, export/native integration, and TUI.
- Candidate minimum checks are mathematical feasibility only. Native rate bounds, precision, admission, and DPDK evidence remain required.
- Session: [[2026-11-16-topology-aggregates]]. Plan: `docs/superpowers/plans/2026-11-16-topology-aggregates.md`. Logs under its `.superpowers/sdd/` workspace.

## 2026-11-16T20:41:06+08:00 (Asia/Shanghai) — Placement boundary complete locally

- Agent: Codex `/root`. Worktree: `/home/pi/LQoSync3.0`, branch `lqosync3`, base `b94ed7c` plus prior uncommitted slices.
- Owner asks for status and directs continuation. Added fresh profile/rate evidence, current account records, and a pure placement resolver.
- Changed `observation.rs`, `catalog.rs`, `mod.rs`, the registry integration fixture, design/discussion, plan, and brain notes. Added `resolver.rs` and `resolver_tests.rs`.
- Resolve explicit source/ancestor/profile restrictions, current alias agreement, explicit rate authority, min/max policies, stable assignments, pins, and weighted new allocations.
- A circuit contributes plan demand once. Current endpoints and policy observations stay outside the registry.
- Typed conflicts withhold the candidate. A placement candidate still needs aggregate/minimum/export/native checks before publication.
- Review regressions fixed ratio equality, explicit-instance eligibility, target-set restrictions, ancestor bypass, and implicit SourceRead authority.
- Added 31 checks. Full suite: 824 passed, zero failed, 9 ignored. Format and Clippy with denied warnings passed.
- Synthetic proof: 10,000 circuits, reversed collection order, same placements. No production benchmark claim.
- No live caller, dependency, service, router, configuration, shaping output, commit, push, or deployment changed. Preserve unrelated work.
- Next: aggregate formulas and minimum feasibility, then scoped rebalance, verified adapters/ID creation, migration/recovery, publication, and TUI.
- Scheduler still needs overlap, overnight, and DST decisions. Keep application default Asia/Manila separate from note timestamps.
- Session: [[2026-11-16-topology-resolver]]. Plan: `docs/superpowers/plans/2026-11-16-topology-resolver.md`. Check logs under its `.superpowers/sdd/` workspace.

## 2026-11-16T20:22:16+08:00 (Asia/Shanghai) — Catalog/schema boundary complete locally

- Agent: Codex `/root`. Worktree: `/home/pi/LQoSync3.0`, branch `lqosync3`, base `b94ed7c` plus uncommitted prior slices.
- Owner directs source/policy catalog and hierarchy persistence. Added catalog types/tests and public persistence tests; extended model/store/observation/rate helpers.
- Schema 2 stores catalog plus subscriber intent in one checked revision. Schema 1 loads losslessly without rewriting; a revision-checked commit persists the upgrade.
- Router roles, source origins, local profiles, explicit target sets, root label, hierarchy instances, configured capacities, and factors remain distinct.
- Reference checks reject orphaned plans/assignments/pins. Canonical order avoids replacement on order-only edits. Snapshot scopes must match selected catalog scopes.
- Review guards reject source-origin replacement under the same ID and invalid protocol formulas. DHCP site/per-plan basis remains explicit.
- Tests: 23 new checks. Full suite: 793 passed, zero failed, 9 ignored. Formatting and Clippy with denied warnings passed.
- Synthetic scale covers 10,000 nodes and 10,000 subscriber bindings. No production benchmark claim.
- No live caller, prior fixture, dependency, config, service, router, dataplane, commit, push, or deployment changed. Preserve unrelated edits.
- Next: fresh membership/profile resolution, eligibility, placements, rates, and typed conflicts. Keep configured WAN bases separate from compiled shaping targets.
- ID creation, verified adapters, complete migration/backup/rollback, policy history, publication, and TUI remain gates before activation.
- Session: [[2026-11-16-topology-catalog]]. Plan: `docs/superpowers/plans/2026-11-16-topology-catalog.md`.

## 2026-11-16T19:40:11+08:00 (Asia/Shanghai) — Subscriber registry boundary complete locally

- Agent: Codex `/root`. Worktree: `/home/pi/LQoSync3.0`, branch `lqosync3`, base `b94ed7c`. Prior foundation remains uncommitted.
- Owner directs the next implementation slice. Added registry model, complete-observation reconciliation, and private store under `src/topology_manager/`.
- Added public restart/reconnect integration test. Updated plans, design, discussion, Home, and session.
- IDs/policy persist; current endpoints do not. Incomplete sources cannot produce a complete snapshot. Source adapter authority and wall-clock freshness remain future gates.
- Reuse the instance lock and synced private writer. Revision checks, external-change detection, no-op suppression, and uncertain-write blocking passed.
- Review fixes: initialize a durable empty document only for a new private directory; reject a missing file in an existing directory. Use a nonblocking open to reject FIFOs without waiting.
- Tests: 35 new checks. Full suite: 770 passed, zero failed, 9 ignored. Formatting and Clippy with denied warnings passed.
- Synthetic scale: 10,000 subscribers with two sessions each; changed endpoints keep registry intent. No production benchmark claim.
- No new live caller, prior fixture, dependency, config, service, router, commit, push, or deployment changed. Preserve unrelated notes.
- Next: complete source/policy catalog and hierarchy persistence, then adapters/migration, assignments, publication, and TUI. Store lock ownership must transfer into one daemon owner at integration.
- Session: [[2026-11-16-topology-registry]]. Plan: `docs/superpowers/plans/2026-11-16-topology-registry.md`.

## 2026-11-16T19:14:07+08:00 (Asia/Shanghai) — Topology foundation complete locally

- Agent: Codex `/root`. Worktree: `/home/pi/LQoSync3.0`, branch `lqosync3`, base `b94ed7c`.
- Owner authorizes implementation and requests performance, integrity, and reliability review.
- Added pure rates/layout modules, exported the module, and added compatibility integration tests. Updated plan, design, discussion, and session.
- Review: checked u128-to-u64 arithmetic, independent replacement controls, local/ceiling separation, exact legacy conversions, and iterative graph validation.
- Hierarchy uses expected O(N + E) work plus ID bytes and O(N) space. A reversed 50,000-level tree passes.
- TDD: 31 new tests failed before implementation, then passed. Full suite: 735 passed, zero failed, 9 ignored.
- Formatting, Clippy with denied warnings, and whitespace checks passed. No legacy fixture or dependency changed.
- No production caller, live config, service, router, dataplane, commit, push, or deployment changed in this implementation slice.
- Documentation baseline `b94ed7c` was committed and pushed before implementation. Preserve unrelated local edits.
- Remaining: durable policy/registry, fresh observations, assignments, balancing, publication, and TUI. Export precision/backend bounds remain adapter gates.
- Scheduler scope includes subscriber policies and WAN/group capacities. Overlap/overnight/DST decisions remain open.
- Session: [[2026-11-16-topology-foundation]]. Plan: `docs/superpowers/plans/2026-11-16-topology-foundation.md`.

## 2026-11-16T18:48:21+08:00 (Asia/Shanghai) — Topology documentation/product review complete

- Owner requests review against the current product, performance/reliability/integrity improvements, and STE for implementation text.
- Agent: Codex `/root`. Worktree: `/home/pi/LQoSync3.0`, branch `lqosync3`, HEAD `0ad8b8d`. Preserve unrelated changes.
- Added product review, proposed implementation design, and isolated foundation plan. Updated discussion/layout/research links, Home, and session notes.
- Evidence: current collector/cycle identity and retention differ from fresh-only manager requirements. Reuse writer/receipts/native verification instead of parallel services.
- Performance design: indexed observations, deterministic revision caches, unchanged-content suppression, bounded history, next-boundary schedules, and viewport-only UI.
- Integrity: distinguish one subscriber circuit from multiple endpoints, preserve IDs, validate address namespaces, and do not claim atomic pair writes or freshness of held native mappings.
- Checks: 117 focused tests passed, zero failures. Documentation whitespace, links, and source-reference checks passed. Ran STE heuristic plus manual prose review.
- No performance benchmark or new runtime feature is claimed. No product code, live config, router, service, package, commit, push, or deployment changed.
- Follow-up answer: scheduler targets include subscriber policies and WAN/group capacities. Updated all current design records. Configured true WAN and ancestor ceilings still apply. Overlap/overnight/DST policies remain undecided.
- Next: isolated tasks in `docs/superpowers/plans/2026-11-16-topology-foundation.md`, then separate migration/publication/UI/scheduler slices.
- Session: [[2026-11-16-topology-product-review]].

## 2026-11-16T18:34:01+08:00 (Asia/Shanghai) — Speed schedule concept approved

- Owner approves recurring speed schedules, multiple schedule profiles, and a configurable timezone dropdown defaulting to `Asia/Manila`.
- Updated discussion record, TUI layout, and session. Schedule profiles are distinct from subscriber service/speed profiles.
- Each schedule has weekday selection, start/end, enable state, and Mbps/Percentage settings. Automatic native verification remains required.
- Outstanding: exact targets, midnight/day semantics, overlaps, inheritance, restart and DST edge behavior. Do not invent approved precedence.
- Application timezone default is Asia/Manila. Documentation timestamps remain Asia/Shanghai per user workflow.
- Worktree: `/home/pi/LQoSync3.0`, branch `lqosync3`. No code, live schedules, or production changes.
- Checks: whitespace, scheduler approval markers, and timezone default passed. Preserve unrelated edits.

## 2026-11-16T18:31:24+08:00 (Asia/Shanghai) — Recurring speed scheduler requested

- Owner asks for Sunday-to-Saturday checkboxes, time ranges, and Percentage/Mbps speed settings.
- Added proposed extension to the discussion record and Policies > Speed Schedules placement to the layout record. Updated the session.
- First open question: target subscriber min/max, WAN/group capacities, or both. No target scope or overlap policy is approved.
- Keep fresh-only observations, base-policy restoration, noncompounding percentages, and automatic DPDK verification.
- Worktree: `/home/pi/LQoSync3.0`, branch `lqosync3`. No code, live schedule, or production changes.
- Checks: documentation whitespace and proposed-scope markers passed. Preserve unrelated edits.

## 2026-11-16T18:26:02+08:00 (Asia/Shanghai) — Initial Topology TUI layout approved

- Owner approves the initial Topology sidebar entry, four views, and inventory/tree/details arrangement.
- Owner explicitly expects further interface improvements. Treat this as a baseline, not a frozen final design.
- Updated `docs/design/tui/TOPOLOGY_MANAGER_LAYOUT.md` and the topology session.
- Next UI work: refine wide/compact wireframes and interactions while preserving approved behavior and automatic publication.
- Worktree: `/home/pi/LQoSync3.0`, branch `lqosync3`. No renderer, runtime code, or production changes.
- Checks: documentation whitespace, approval state, and requirements link passed. Preserve unrelated edits.

## 2026-11-16T18:19:05+08:00 (Asia/Shanghai) — Topology TUI layout proposed

- Owner asks about the manager's TUI location, feature placement, and UI terminology.
- Read actual sidebar/routes. Proposed one Topology route after Routers, with Builder, Assignments, Policies, and Changes views.
- Added `docs/design/tui/TOPOLOGY_MANAGER_LAYOUT.md` and updated the topology session. Layout remains unapproved.
- Wide Builder: inventory, tree, details. Compact mode: focused-panel switcher. Automatic publication status remains visible.
- No manual preview/Validate/Apply gate. Preserve keyboard focus and edited fields during refresh.
- Worktree: `/home/pi/LQoSync3.0`, branch `lqosync3`. No renderer, runtime code, or production changes.
- Checks: documentation whitespace and requirements link passed. Preserve unrelated edits.

## 2026-11-16T18:15:31+08:00 (Asia/Shanghai) — Supplemental requirements approvals complete

- Owner approves detail 2b: Subscriber Speed Policy controls in Topology Manager, global defaults, shared-source overrides, and per-control inheritance.
- Marked all eleven behavior decisions and four supplemental groups approved in the discussion record. Updated the session.
- Removed stale pending-default/UI wording. Existing migration values and fresh-only session observations remain required.
- Next: concrete data model, Settings TUI migration boundaries, implementation tasks, and focused verification plan.
- Exact identity adapters, native ACK correlation, interrupted writes, edit completion, and recovery parameters still require design/proof.
- Requirements approval does not claim runtime implementation or authorize a live deployment.
- Worktree: `/home/pi/LQoSync3.0`, branch `lqosync3`. No product code or production changes.
- Checks: whitespace, all decision approval states, absence of pending owner-decision markers, and report link passed. Preserve unrelated edits.

## 2026-11-16T18:13:43+08:00 (Asia/Shanghai) — Subscriber speed defaults approved

- Owner approves new-policy maximum 100 percent and minimum fixed 1 Mbps, separately for download and upload.
- Existing effective values remain preserved during migration. Both min/max unit selectors remain independently switchable.
- Updated detail 2a, supplemental tracker, and session. Added pending detail 2b for UI location and per-control inheritance.
- Proposal: all subscriber policy controls in Topology Manager, global defaults overridden by shared profile/service controls.
- No per-WAN-instance or individual subscriber speed override in the proposal. Node capacity factors remain separate.
- Worktree: `/home/pi/LQoSync3.0`, branch `lqosync3`. No code or production changes.
- Checks: whitespace and approval-state consistency passed. Preserve unrelated edits.

## 2026-11-16T18:10:48+08:00 (Asia/Shanghai) — Publication behavior approved, speed defaults pending

- Owner approves supplemental detail 4: automatic triggers, serialized writes, fresh-data recovery, and correlated native verification.
- Updated discussion record and session. Approval does not substitute for isolated implementation/protocol tests.
- Next: detail 2a, initial subscriber speed defaults. Proposal: max 100 percent and min fixed 1 Mbps, each direction.
- Preserve migrated settings. Mode selectors are already approved; new default values are not.
- Proposed subscriber control location and inheritance still need separate confirmation.
- Worktree: `/home/pi/LQoSync3.0`, branch `lqosync3`. No product code or production changes.
- Checks: whitespace, approval states, and reference link passed. Preserve unrelated edits.

## 2026-11-16T18:07:28+08:00 (Asia/Shanghai) — Identity registry approved, publication detail pending

- Owner approves subscriber identity/policy registry after the CRM clarification, then requests the next detail.
- CRM remains a future extension, not an authorized feature implementation.
- Added supplemental detail 4 proposal: completed valid edits/fresh changed collections trigger serialized publication and native verification.
- Recover from fresh observations; do not replay archived subscriber IP/MAC. Recheck delayed ACK before considering another mutation.
- Detail 4 remains proposed. Subscriber policy defaults remain open; do not silently select them.
- Updated record and session in `/home/pi/LQoSync3.0`, branch `lqosync3`. No product code or production changes.
- Checks: whitespace, supplemental decision states, and report link passed. Preserve unrelated edits.

## 2026-11-16T17:58:56+08:00 (Asia/Shanghai) — Min/max unit selectors approved

- Owner explicitly approves Mbps/Percentage modes for both subscriber minimum and maximum speed, then requests the next detail.
- Updated supplemental detail 2 and session. Do not infer selection of the proposed fixed-1-Mbps default.
- Added detail 3 proposal: persistent internal subscriber ID mapped to verified source account/reference, with fresh-only endpoint observations.
- PPP/Hotspot use authenticated active records; DHCP uses bound plus an independent reliable reference for durable continuity.
- Changed IP/MAC without reliable identity evidence is a critical reconciliation conflict, not an automatic merge. Detail 3 remains proposed.
- Worktree: `/home/pi/LQoSync3.0`, branch `lqosync3`. No code or production changes.
- Checks: documentation whitespace and decision-state consistency passed. Preserve unrelated edits.

## 2026-11-16T17:53:34+08:00 (Asia/Shanghai) — Subscriber speed unit selector requested

- Owner requests switching between percentage-based and Mbps-based speed controls.
- Updated supplemental detail 2 and the session. Minimum modes are confirmed; maximum fixed-Mbps scope needs clarification.
- The proposed fixed-1-Mbps default, UI location, and inheritance details are not approved by this clarification.
- Mode changes must not silently reinterpret the previous numeric value. Show base and resolved speeds.
- Worktree: `/home/pi/LQoSync3.0`, branch `lqosync3`. Checks: whitespace and pending-state consistency passed.
- No runtime code or production changes. Preserve unrelated edits.

## 2026-11-16T17:47:28+08:00 (Asia/Shanghai) — Supplemental topology detail 1 approved

- Owner approves intermediate capacity inheritance, optional explicit limits, and no mixed subscriber/group children in the first version.
- Owner adds explicit capacity based on a percentage of WAN capacity. Distinguish fixed Mbps and WAN-percentage modes.
- Example: 50 percent of configured 310 Mbps is 155 Mbps, further constrained by any lower ancestor ceiling.
- Updated discussion record and session in `/home/pi/LQoSync3.0`, branch `lqosync3`.
- Next: supplemental detail 2, subscriber speed controls/defaults. Proposed UI, inheritance, and fixed-1-Mbps defaults remain unapproved.
- Checks: whitespace, reference link, and percentage arithmetic passed. No runtime code or production changes. Preserve unrelated edits.

## 2026-11-16T17:37:30+08:00 (Asia/Shanghai) — All eleven topology behavior decisions approved

- Agent: Codex `/root`. Worktree: `/home/pi/LQoSync3.0`, branch `lqosync3`.
- Owner explicitly approved corrected decision 11: automatic checks, write/apply, DPDK verification, bounded retries, and fresh-data recovery.
- Preview is optional visibility. Routine operator Validate/Apply gates are superseded. Unresolved critical conflicts require intervention.
- Updated `docs/design/TOPOLOGY_MANAGER_DISCUSSION.md` and the topology session to record all eleven approvals.
- Remaining design details stay open: intermediate/mixed-node capacity, subscriber controls/defaults, source identity/presence, edit completion, and recovery mechanisms.
- This completes the requested decision round. No implementation or live deployment occurred.
- Checks: all eleven decision states, documentation whitespace, and report link passed. Preserve unrelated edits.
- Next: use this record for the implementation design, without inventing open defaults or claiming runtime reliability proof.

## 2026-11-16T17:32:42+08:00 (Asia/Shanghai) — Owner directs automatic topology publication

- Owner supersedes the proposed manual Validate/Apply lifecycle: automatically write, apply, and verify the DPDK dataplane.
- Preview shows changes without gating publication. Intervention is for unresolved critical conflicts.
- Internal checks remain automatic to meet approved reliability and integrity requirements.
- Revised decision 11 and earlier manual-Apply wording in the discussion record. Updated the session.
- Keep configured, published, imported, and verified-applied states separate. Failed collection holds automatically; fresh recovery resumes automatically.
- Detailed edit completion and recovery/rollback triggers remain open. Do not mark them approved by implication.
- Worktree: `/home/pi/LQoSync3.0`, branch `lqosync3`. No code or production changes.
- Checks: documentation whitespace and automatic-lifecycle consistency passed. Preserve unrelated edits.

## 2026-11-16T17:20:53+08:00 (Asia/Shanghai) — Topology decision 10 approved

- Agent: Codex `/root`. Worktree: `/home/pi/LQoSync3.0`, branch `lqosync3`.
- Owner approved visible max-rate oversubscription, blocking invalid commitments/unresolved placements, and explicit fallback only.
- Added decision 11 proposal: separate drafts, saved policy, published files, imported world, and verified native application.
- Rollback must restore policy with fresh subscriber observations, never replay historical IP/MAC rows.
- Serialize writes and recover interrupted attempts. Pair-file renames do not form one atomic transaction.
- Decision 11 remains unapproved. Added residual implementation questions to prevent a false design-complete claim.
- Updated discussion record and session. Checks: whitespace and decision-state consistency passed.
- No product code or production changes. Preserve unrelated edits.

## 2026-11-16T17:17:10+08:00 (Asia/Shanghai) — Topology decision 9 approved

- Agent: Codex `/root`. Worktree: `/home/pi/LQoSync3.0`, branch `lqosync3`.
- Owner approved decision 9 after DHCP bound-status and lease-expiry clarifications, then requested the next decision.
- Approved lifecycle includes confirmed-absence removal, fresh return data, publication hold on failed required reads, and explicit reconciliation.
- Empty manual nodes remain configured. Empty automatic branches leave active export but retain internal identities.
- Added decision 10 proposal: allow visible maximum-rate oversubscription, reject invalid commitments, and hold unresolved required placements.
- Strict publication and explicit fallback details remain unapproved. Publication hold does not block forwarding or prove shaping of new subscribers.
- Updated discussion record and session. Checks: whitespace and decision-state consistency passed.
- No product code or production changes. Preserve unrelated edits.

## 2026-11-16T17:12:59+08:00 (Asia/Shanghai) — DHCP bound-state clarification

- Owner asks about static/dynamic DHCP bound status. Verified official RouterOS DHCP documentation.
- DHCPv4 active-lease criterion: current `status=bound`, with fresh address/MAC attributes. Static configuration alone does not qualify.
- Bound is lease state, not immediate reachability. A device can disconnect before the lease changes or expires.
- Updated discussion record and session. Decision 9 remains pending approval.
- Worktree: `/home/pi/LQoSync3.0`, branch `lqosync3`. No code or production changes.
- Checks: documentation whitespace and decision-state consistency passed. Preserve unrelated edits.

## 2026-11-16T17:01:46+08:00 (Asia/Shanghai) — Immediate-parent rule approved, decision 9 pending

- Agent: Codex `/root`. Worktree: `/home/pi/LQoSync3.0`, branch `lqosync3`.
- Owner approved the corrected immediate-parent calculation and requested the next decision.
- Added decision 9 proposal for fresh collection, confirmed absence, rename/source reconciliation, and empty-node exports.
- Proposal: failed required reads hold the combined export; no stored-IP/MAC fallback or false fresh-publication claim.
- Proposal: retain configured WAN/manual nodes; omit empty automatic branches from export while preserving definitions and instance IDs.
- Decision 9 remains unapproved. Source-specific active-presence rules and remaining capacity/control details still need design.
- Updated discussion record and session. Checks: whitespace and decision-state consistency passed.
- No product code or production changes. Preserve unrelated edits.

## 2026-11-16T16:59:45+08:00 (Asia/Shanghai) — Immediate subscriber parent calculation clarified

- Owner clarifies that subscriber-based auto-compute and group factors apply at the immediate parent of subscribers.
- Corrected decision 8 and removed the example that recalculated every ancestor from descendant plans.
- WAN capacity stays explicitly configured. A direct-subscriber WAN does not switch to automatic calculation.
- Example: 10 times 50 Mbps with group factor 0.8 computes 400 Mbps, capped at the configured 310 Mbps WAN. Subscriber maxima remain 50 Mbps.
- Intermediate aggregates and mixed subscriber/group children still need an explicit policy. Do not invent their output rates.
- Updated discussion record, topology session, and handoff in `/home/pi/LQoSync3.0`, branch `lqosync3`.
- Checks: documentation whitespace and example arithmetic passed. No code or production changes. Preserve unrelated edits.

## 2026-11-16T16:54:09+08:00 (Asia/Shanghai) — Topology decision 8 approved with subscriber speed controls

- Agent: Codex `/root`. Worktree: `/home/pi/LQoSync3.0`, branch `lqosync3`.
- Owner approved nested computation, with strict WAN ceilings for aggregate nodes and unchanged individual subscriber maxima.
- Example: a 310 Mbps WAN caps descendant aggregate nodes at 310 Mbps, while a subscriber plan remains 50 Mbps.
- Owner requests visible subscriber max factors and minimum settings: fixed Mbps or percent of the current read base speed.
- These subscriber controls are separate from WAN/group factors. Minimum examples do not select a default.
- Updated discussion record and session. UI location, inheritance, and initial minimum defaults remain open details.
- Checks: documentation whitespace, approval state, and example arithmetic passed. No code or production changes.
- Next: clarify subscriber-policy details, then decision 9. Preserve unrelated edits.

## 2026-11-16T16:42:49+08:00 (Asia/Shanghai) — Topology decision 7 approved

- Agent: Codex `/root`. Worktree: `/home/pi/LQoSync3.0`, branch `lqosync3`.
- Owner approved custom Auto default, optional Manual mode, default factors of 1.0, and preservation of migrated values.
- Updated discussion record, topology session, and handoff. Read current group-rate formulas in `src/topology.rs`.
- Next: decision 8, raw descendant demand, per-node factors, ancestor ceilings, and computed-only balancing. All new details remain proposed.
- Preserve source-specific plan-sum versus count-times-unit formulas. Do not silently replace them with one generic formula.
- Checks: documentation whitespace and decision-state consistency passed. No runtime code, live configuration, or product tests changed.
- Preserve unrelated edits. Session: [[2026-11-16-topology-manager-adoption]].

## 2026-11-16T16:40:58+08:00 (Asia/Shanghai) — Fresh-data clarification approved, decision 7 pending

- Agent: Codex `/root`. Worktree: `/home/pi/LQoSync3.0`, branch `lqosync3`.
- Owner explicitly approved fresh observations without stored-IP/MAC fallback, then requested the next decision.
- Added decision 7 proposal to the discussion record. Updated the topology session and this handoff.
- Proposal: automatic custom groups by default, optional explicit manual capacity, and factors of 1.0 without configured defaults.
- Preserve migrated values, capacity modes on moves, and approved per-instance overrides. These new defaults remain unapproved.
- Checks: documentation whitespace and decision-state consistency passed. No product code or live configuration changed.
- Next: obtain decision 7. Preserve unrelated edits. Session: [[2026-11-16-topology-manager-adoption]].

## 2026-11-16T16:39:35+08:00 (Asia/Shanghai) — Fresh subscriber data clarification

- Owner prohibits reuse of stored IP/MAC observations and requires fresh subscriber data for active output.
- Never fill current session fields from historical rows or reconcile identity through historical IP/MAC matches.
- Saved operator policies remain separate. Failed reads must not present an old snapshot as fresh.
- Updated the discussion record and topology session. Decision 6 remains approved with this clarification.
- Worktree: `/home/pi/LQoSync3.0`, branch `lqosync3`. No runtime code or production changes.
- Checks: documentation whitespace and required fresh-data statements passed. Preserve unrelated edits.
- Next: decision 7. Source-specific failure/presence handling remains decision 9.

## 2026-11-16T16:35:10+08:00 (Asia/Shanghai) — Topology decision 6 and subscriber lifecycle approved

- Agent: Codex `/root`. Worktree: `/home/pi/LQoSync3.0`, branch `lqosync3`.
- Owner approved pins and exceptions, with explicit subscriber identity integrity across disconnect and reconnect.
- Confirmed absence removes active CSV/network subscriber output. Retain internal identity and valid saved pins separately.
- Return requires fresh IP, MAC, and session data. Reused addresses must not transfer identity or policy.
- Updated the discussion record, topology session, and this handoff. Added lifecycle acceptance requirements.
- Failed or partial reads do not prove absence. Source-specific identity, presence, stale-data, and empty-group details still need design.
- Next: decision 7, custom-node capacity policies. Decision 9 must retain these approved lifecycle requirements.
- Checks: documentation whitespace and decision-state consistency passed. No runtime code or production changes.
- Preserve unrelated edits. Session: [[2026-11-16-topology-manager-adoption]].

## 2026-11-16T16:26:11+08:00 (Asia/Shanghai) — Topology decision 5 approved

- Agent: Codex `/root`. Worktree: `/home/pi/LQoSync3.0`, branch `lqosync3`.
- Owner approved automatic initial assignment, stable current placements, previewed changes, and explicit scoped rebalance.
- First-version continuous background redistribution remains disabled.
- Updated the discussion record, topology session, and this handoff.
- Next: decision 6, manual assignments, instance pins, and explicit eligibility exceptions. These details remain proposed.
- Checks: documentation whitespace and decision-state consistency passed. No product code or live configuration changed.
- Preserve unrelated edits. Session: [[2026-11-16-topology-manager-adoption]].

## 2026-11-16T16:22:31+08:00 (Asia/Shanghai) — Topology decision 4 approved

- Agent: Codex `/root`. Worktree: `/home/pi/LQoSync3.0`, branch `lqosync3`.
- Owner approved capacity-weighted plan allocation for destinations with configured capacities, with download and upload checks.
- Group allocation without independent capacity remains decision 8.
- Updated the discussion record, topology session, and this handoff.
- Next: decision 5, new-subscriber assignment and controlled rebalance of current subscribers. Its defaults remain proposed.
- Checks: documentation whitespace and decision-state consistency passed. No product code or live configuration changed.
- Preserve unrelated edits. Session: [[2026-11-16-topology-manager-adoption]].

## 2026-11-16T16:15:25+08:00 (Asia/Shanghai) — Topology decision 3 approved

- Agent: Codex `/root`. Worktree: `/home/pi/LQoSync3.0`, branch `lqosync3`.
- Owner approved subscriber eligibility after clarification: a shared pool can span all selected WANs, even with one profile.
- Optional explicit filters restrict destinations or exclude subscribers. Pool and WAN target selection remain explicit.
- Updated the discussion record, topology session, and this handoff.
- Next: decision 4, capacity-weighted plan allocation with directional capacity checks. It remains proposed.
- Checks: documentation whitespace and decision-state consistency passed. No product code or live configuration changed.
- Preserve unrelated edits. Session: [[2026-11-16-topology-manager-adoption]].

## 2026-11-16T15:31:38+08:00 (Asia/Shanghai) — Topology decision 2 approved

- Agent: Codex `/root`. Worktree: `/home/pi/LQoSync3.0`, branch `lqosync3`.
- Owner approved shared target sets, explicit placements, overlap deduplication, and explicit inclusion of new WANs.
- Updated the discussion record, topology session, and this handoff.
- Next: discuss decision 3, subscriber eligibility. The proposed source pools and filters remain unapproved.
- Checks: documentation whitespace and decision-state consistency passed. No runtime code or live configuration changed.
- Preserve unrelated edits. Session: [[2026-11-16-topology-manager-adoption]].

## 2026-11-16T15:28:57+08:00 (Asia/Shanghai) — Topology decision 1 approved

- Agent: Codex `/root`. Branch/worktree: `lqosync3`, `/home/pi/LQoSync3.0`, HEAD `0ad8b8d`.
- Owner approved shared source definitions/default factors, separate instances, and optional instance factor overrides.
- Owner explicitly requires reliability and integrity. Added requirements for IDs, references, inheritance, uniqueness, and failure preservation.
- Updated `docs/design/TOPOLOGY_MANAGER_DISCUSSION.md`, the topology session, and this handoff.
- Decision 2 remains proposed: shared child targets versus explicit placement, overlap handling, and deliberate target-set changes.
- Next: obtain the owner's decision on item 2. No runtime behavior is implemented or verified.
- Checks: documentation whitespace and decision-state consistency. No code or live configuration changed. Preserve unrelated edits.
- Session: [[2026-11-16-topology-manager-adoption]].

## 2026-11-16T15:26:14+08:00 (Asia/Shanghai) — Topology requirements recorded

- Agent: Codex `/root`. Branch/worktree: `lqosync3`, `/home/pi/LQoSync3.0`, HEAD `0ad8b8d`.
- Owner requests one decision per discussion round and documentation of all prior requirements.
- Added `docs/design/TOPOLOGY_MANAGER_DISCUSSION.md`. Updated the topology session and this handoff.
- The record separates confirmed direction from proposals and eleven pending decisions.
- Topology assignment precedes routing. Traffic intelligence is a separate packet/IP classification function.
- Next: discuss source identity and topology instances. Do not treat the proposed defaults as approved.
- No product code or live configuration changed. Preserve prior unrelated changes.
- Checks: documentation whitespace and report link. No product tests needed for this documentation update.
- Session: [[2026-11-16-topology-manager-adoption]].

## 2026-11-16T14:11:53+08:00 (Asia/Shanghai) — Topology Manager adoption research

- Agent: Codex `/root`. Goal: research adoption of LibreQoS Topology Manager in LQoSync3.
- Branch/worktree: `lqosync3`, `/home/pi/LQoSync3.0`, HEAD `0ad8b8d`. Preserve prior unrelated edits.
- Added `docs/reports/2026-11-16-topology-manager-adoption.md` and the matching session note. Updated Home and this handoff.
- Read the project graph, local source, installed UI/state, and public source at `9ebe29bc9d49013aff9b0544de3c6f6fc6ba2f1d`.
- Manual import creates fixed nodes. Public native JSON lacks editor metadata. A source switch alone does not enable native moves.
- Proposed paths: LQoSync-owned editor with current pair export, or an ingress extension for the native editor. Neither is implemented.
- Checks passed: whitespace, report reference links, local code/note paths, and static source assertions.
- Reviewed STE heuristic findings manually. Technical names and list-related findings remain. No certified-compliance claim.
- No product test run was needed for these documentation changes. No live configuration, service, router, or package changed.
- Open: select editor ownership and verify the exact installed ingress contract. Prior native rate-loss findings need a current-build retest.
- Next: specify a persistent topology catalog and revision flow, or prototype native ingress in isolation. Keep the convergence investigation separate.
- Session: [[2026-11-16-topology-manager-adoption]]. No commit, push, external message, or deployment occurred.

## 2026-11-16T08:44:08+08:00 (Asia/Shanghai) — RAM-cache deployment authorized; CI fix

- Owner authorized clean release preparation, deployment/restart, and production verification. Agent: Codex `/root`.
- Branch/worktree: `lqosync3`, `/home/pi/LQoSync3.0`. Preserve the unrelated profile/WAN work.
- CI for `ad1665c` failed its same-size atomic-replacement loader regression. Local earlier runs passed because timestamps differed.
- A deterministic same-mtime/same-size test reproduces the missed refresh. The shared TUI stamp now includes device and inode identity.
- Fresh full checks passed 700 tests, zero failures, nine manual tests ignored. The corrective commit is required before release.
- Next: commit/push this fix, verify its CI, build a clean detached release, prepare rollback and RAM-link checks, then deploy.
- Host sudo requires the operator password. Prepare the complete deployment script before requesting the terminal step.
- Current configuration hash differs from the November 12 manifest. Preserve the current configuration and pin its hash in the new artifact.
- No production change has occurred in this deployment task yet. Allocation repair remains deferred.


## 2026-11-16T08:33:17+08:00 (Asia/Shanghai) — RAM-cache commit and push authorized

- Owner requested commit and push only. Agent: Codex `/root`.
- Branch/worktree: `lqosync3`, `/home/pi/LQoSync3.0`. Starting HEAD and origin both point to `d569bf8`.
- Scope: RAM-cache implementation/tests, operations documentation, implementation plan, and completed capacity/Telegram/performance session notes.
- Preserve the unrelated profile/WAN Knowledge note and its Home link outside the commit.
- Fresh pre-push checks passed: 700 tests, zero failures, nine manual tests ignored; formatting and diff checks passed. Prior Clippy, isolated I/O verification, and seven-route terminal smoke passed.
- Commit and push follow this preparation note. Confirm the resulting HEAD against `origin/lqosync3` before reporting success.
- Evidence belongs in `/home/pi/.local/state/lqosync3/ram-cache-task/`. No private native captures enter Git.
- Deployment and restart are not requested in this task. Production performance savings remain unmeasured.
- After push, the next release step is a clean artifact and deployment only when requested.

Session: [[2026-11-12-worker-ram-cache]].

## 2026-11-12T15:19:25+08:00 (Asia/Shanghai) — RAM-backed worker cache complete locally

- Owner requested the next optimization. Agent: Codex `/root`. Branch/worktree: `lqosync3`, `/home/pi/LQoSync3.0`.
- Implemented `src/worker_util/cache.rs`, monitor integration and temporary-file cleanup in `src/worker_util.rs`, plus a TUI loader regression in `src/tui/data.rs`.
- Public cache path/schema remain compatible through an atomic symlink to a per-UID, per-state tmpfs directory. Polling and all chart histories remain unchanged.
- Directory ownership/mode/tmpfs checks, private-directory preservation, rollback, runtime failure fallback, and stale/missing behavior have regression coverage.
- Full checks: 700 passed, zero failed, nine manual tests ignored. Clippy, formatting, debug build, and seven-route TUI smoke passed.
- Manual debug I/O test: 50 publications, 283,359-byte cache, zero RAM-path block writes versus 14,336,000 disk-path bytes. Not a production-wide reduction claim.
- Plan: `docs/superpowers/plans/2026-11-12-worker-ram-cache.md`. Operations and unified-health follow-up documentation updated.
- Evidence: `/home/pi/.local/state/lqosync3/ram-cache-task/`; red/green/review logs are under the existing performance-observation directory.
- No commit, push, production migration, restart, or deployment occurred. Installed service remains `3.0.179+g12e3d57`.
- Next when release is requested: commit only selected feature/docs files, push/check CI, build a clean release, deploy, then verify public link, reader access, freshness, and production I/O.
- Preserve unrelated profile/WAN notes and earlier uncommitted diagnosis/verification documentation. Allocation work remains deferred by the owner.

Session: [[2026-11-12-worker-ram-cache]].

## 2026-11-12T14:56:30+08:00 (Asia/Shanghai) — Production performance observation complete

- Owner approved observation. Agent: Codex `/root`. Branch/worktree: `lqosync3`, `/home/pi/LQoSync3.0`.
- Five minutes, 151 samples, three completed normal sync cycles. Average CPU 0.285% of one core; memory 25.69–28.04 MiB.
- Sync times 157–164 ms, median 159 ms. No cycle errors, stale telemetry, cache read errors, or expired health components.
- MainPID 882084 stayed active. Automatic restarts: zero. Known allocation mismatch remains the only nonhealthy component.
- Service write rate: 199.36 KiB/s in a separate 267-second I/O window. This implies 16.43 GiB/day only if sustained.
- About 401 KB of worker history is republished every two seconds. That pattern is consistent with most writes, but no per-file I/O attribution occurred.
- No memory pressure or OOM increase. I/O pressure was negligible. No sustained CPU/memory bottleneck appeared in this short unchanged-output workload.
- Evidence: `/home/pi/.local/state/lqosync3/performance-observation/`. Metadata only, no subscriber inventories or credentials.
- Updated operations documentation, session note, Home link, and this handoff. Prior capacity/Telegram documentation changes remain local.
- Checks: script syntax, result assertions, service status, and diff whitespace passed. No product-code tests apply.
- No product code/config change, forced output write, restart, deployment, commit, or push occurred.
- Proposed next task: RAM-backed worker telemetry cache, preserving two-second refresh, history, reader compatibility, and durable operation/outbox records. Not started.
- The owner defers the LibreQoS beta allocation issue. Preserve unrelated profile/WAN notes.

Session: [[2026-11-12-production-performance-observation]].

## 2026-11-12T14:46:14+08:00 (Asia/Shanghai) — Live Telegram delivery verified

- Owner deferred the beta LibreQoS allocation issue and requested the next task. Agent: Codex `/root`.
- Branch/worktree: `lqosync3`, `/home/pi/LQoSync3.0`. Installed release remains `3.0.179+g12e3d57`.
- Actual cycle 185 wrote both managed files. Current hashes match the durable receipt.
- Same write ID: WRITTEN/PENDING message 4058, then APPLIED message 4059. One positive API receipt per report, completed, no skip/pause/error.
- Operation remained stable during the root metadata probe. No pending write events or delivery-gap marker exist.
- Shared apply is APPLIED, notification health HEALTHY. Overall remains DEGRADED because the allocation mismatch stays visible.
- Private evidence: `/home/pi/.local/state/lqosync3/telegram-verification/`. `verified.json` records the assertion result and times.
- Checks: probe syntax, stable operation, file hashes, IDs, report order, completion flags, and queue state passed. Diff whitespace check passed.
- Updated operations documentation, this handoff, the session note, and its Home link. No product code changed.
- No forced managed-file update, synthetic Telegram send, configuration change, service restart, deployment, commit, or push occurred.
- API acceptance is verified. Human reading and live retry/failure cases are not part of this result.
- Next proposed task: production performance observation. Allocation work remains deferred pending upstream clarification or a relevant beta update.
- Preserve unrelated profile/WAN work and the earlier capacity diagnosis changes.

Session: [[2026-11-12-telegram-delivery-verification]].

## 2026-11-12T14:31:45+08:00 (Asia/Shanghai) — Capacity trace verified

- Owner ran the root metadata probe. Agent: Codex `/root`. Branch/worktree remains `lqosync3`, `/home/pi/LQoSync3.0`.
- Current native policy verification passes: 64 applied / 1000 desired slots. No managed-file write occurred.
- Running/installed bridge hashes match. Revision: `f1cc998a013e`.
- LKG schema 1 contains 26 leaf operations. The recursive metadata probe finds no explicit capacity field.
- Correct the earlier hypothesis: the LKG does not demonstrably store literal 64. Startup derives a bound from restored placement.
- Installed static code selects a placement-derived power-of-two bound before the separate 1000-slot fallback. A 26-entry placement gives 64.
- Shared capacity initializes once by the observed direct-reference path. Reports and R3 rebuild paths read that value.
- This is the supported mismatch mechanism. Exact branch confirmation by private source or a controlled boot test remains unavailable.
- Evidence: `/home/pi/.local/state/lqosync3/capacity-trace/`. Keep metadata, static excerpts, reference inventory, and startup evidence outside Git.
- Updated diagnosis session and handoff only. Probe syntax and JSON checks passed. Diff whitespace check passed. No product-code tests apply.
- No code/configuration change, LKG edit, restart, deployment, commit, push, or Telegram send occurred.
- Next repair boundary: upstream must reconcile desired/placement capacity at boot and define capacity-only updates. Do not assume ordinary restart repairs this.
- Preserve unrelated profile/WAN work. The real two-stage Telegram delivery observation still awaits an actual managed-file change.

Session: [[2026-11-12-native-capacity-diagnosis]].

## 2026-11-12T14:28:03+08:00 (Asia/Shanghai) — Installed bridge trace; metadata pending

- Owner requested the installed bridge allocation trace. Agent: Codex `/root`.
- Branch/worktree: `lqosync3`, `/home/pi/LQoSync3.0`. No product edits.
- Exact bridge revision: `f1cc998a013e`. ELF build ID: `7d2a946d53a6712aa9fc2315b2cb31b7dbbd4e98`.
- Startup journal confirms LKG autonomous boot with 26 leaves. Placement revision differed, so the bridge reconstructed that section.
- Static inspection identifies power-of-two startup allocation and a separate 1000-slot fallback. Exact boot-object field remains uncertain.
- Prepared `/home/pi/.local/state/lqosync3/capacity-trace/read-only-probe.py`. Syntax check passed.
- Asked the operator to run it with sudo and redirect to `live-metadata.json` in the same directory. Host sudo needs the operator password.
- Next: inspect that result for LKG/desired capacities, native acceptance, and running/installed binary hashes. Do not repeat the request if the result exists.
- Evidence stays outside Git. No restart, configuration change, managed-file write, commit, push, or Telegram send occurred.
- Updated this handoff and the diagnosis session. Preserve unrelated profile/WAN work.

Session: [[2026-11-12-native-capacity-diagnosis]].

## 2026-11-12T08:44:12+08:00 (Asia/Shanghai) — Capacity diagnosis

- Goal: read-only investigation of 64 native / 1000 desired slots. Agent: Codex `/root`.
- Branch/worktree: `lqosync3`, `/home/pi/LQoSync3.0`, HEAD `d569bf8`.
- Public source confirms a 1000-slot emitter floor, policy-only broker convergence, and omitted capacity in projection/incremental delta.
- Leading hypothesis: allocation does not propagate through that incremental path. The origin of 64 and installed bridge resize rules remain unproved.
- Existing root proof verifies policy and reports capacity mismatch. No new root socket sample occurred.
- Package version: `3.0.7.202611102128`. Public source revision is not a confirmed installed build revision.
- No product/config/service changes, restart, deployment, commit, push, or Telegram send.
- New files/changes: diagnosis session note and this handoff. Read-only checks only; no code tests required.
- Next: establish the installed bridge cold-load/resize contract with upstream before a repair plan. Do not assume restart fixes capacity.
- Preserve the unrelated profile/WAN note and Home link. Live two-stage Telegram delivery still awaits an actual managed-file change.

Session: [[2026-11-12-native-capacity-diagnosis]].

## 2026-11-12T08:33:16+08:00 (Asia/Shanghai) — Unified health deployed and verified

- Owner authorized release and ran the root deploy script. Last agent: Codex `/root`.
- Feature commit `12e3d57` is pushed to `lqosync3`; installed release `3.0.179+g12e3d57` runs as PID 882084.
- Installed checksum and configuration hash match the manifest. Service is active/running, with zero automatic restarts.
- GitHub CI, 690 local tests, release smoke, and installed seven-route terminal smoke passed.
- Independent checks confirmed advancing telemetry, collection freshness, shared health, notification worker, and native policy evidence.
- Allocation remains 64 native / 1000 generated slots. Startup wrote no new pair, so shared apply state has no tracked operation.
- No forced shaping-file write or synthetic Telegram message occurred. Live two-stage delivery awaits the next actual managed-file change.
- Backup: `/var/backups/lqosync3/health-12e3d57-146zv46b`. Evidence: `/home/pi/.local/state/lqosync3/health-12e3d57/`.
- No deployment work remains. Preserve unrelated profile/WAN notes and Home link.

Session: [[2026-11-12-unified-health-deployment]].

## 2026-11-12T08:31:24+08:00 (Asia/Shanghai) — Unified health pushed; root deploy pending

- Owner authorized commit, push, and deployment. Feature commit `12e3d579b055fd5c42d6757ad4100954d9e7b0b3` is on `origin/lqosync3`.
- Clean artifact: `3.0.179+g12e3d57`, `/home/pi/.local/state/lqosync3/health-12e3d57/lqosync3`.
- Both CI jobs passed: https://github.com/p33ckab00/LQoSync/actions/runs/43127174714. Release terminal smoke passed seven routes.
- Deployment script checks passed. Binary/config/state backup, atomic install, restart, six telemetry sources, unified health, delivery worker, native policy proof, and binary rollback are prepared.
- Root execution is pending because noninteractive sudo requires a password. Operator was asked to run `/home/pi/.local/state/lqosync3/health-12e3d57/deploy.sh` with sudo.
- Next: inspect DEPLOYED.json; run artifact `smoke.py /usr/local/bin/lqosync3`, then `verify.py`. Do not claim deployment until those checks pass.
- Main worktree retains unrelated Home Knowledge link and untracked profile/WAN note. This handoff update is local pending deployment completion.
- Artifact checkout is detached and clean at the feature commit. Private native captures remain outside Git.


## 2026-11-12T08:25:19+08:00 (Asia/Shanghai) — Unified health release authorized

- Owner requested commit, push to `lqosync3`, and deployment. Last agent: Codex `/root`.
- Local implementation gates passed: 690 tests, Clippy, formatting, debug build, and seven-screen terminal smoke.
- Next: create the feature commit, push, check CI, build a clean release artifact, and prepare backup/restart/rollback verification.
- Host sudo still requires the operator password. Complete the package before requesting the operator's terminal step.
- Keep unrelated profile/WAN note and its Home link outside the feature commit. Private native captures stay outside Git.


## 2026-11-12T08:18:12+08:00 (Asia/Shanghai) — Unified health complete locally

- Goal: report each v3 pair write as WRITTEN/PENDING, then a verified terminal outcome, without blocking collection.
- Last agent: Codex `/root`. Branch/worktree: `lqosync3`, `/home/pi/LQoSync3.0`. Changes remain uncommitted.
- All four plan tasks are implemented. Files include import/apply evidence, health hub, receipts, Telegram delivery/outbox, runner/main/samplers, TUI loaders/health panel, tests, examples, and documentation.
- Final checks: 690 tests passed, zero failed, eight manual tests ignored; all-target Clippy, formatting, debug build, and diff whitespace checks passed.
- Actual terminal smoke passed all seven routes. Dashboard capture uses synthetic data and preserves the approved layout.
- Operator read-only native policy check passed. Allocation evidence differs: 64 native slots versus 1000 generated. Policy APPLIED and allocation DEGRADED remain separate facts.
- No commit, push, deployment, production restart, or live Telegram test send occurred.
- Unfinished release work: commit/push/deploy only when requested; verify the deployed worker and API delivery during that separate task.
- Next action: report local completion. Investigate the native allocation mismatch separately if requested.
- Evidence: `/home/pi/.local/state/lqosync3/unified-health-task1/`. Private native captures must stay outside Git.
- Preserve unrelated profile/WAN Knowledge note and its pre-existing Home link.

Session: [[2026-11-11-unified-health-implementation]].

## 2026-11-11T16:21:30+08:00 (Asia/Shanghai) — Unified health implementation in progress

- Owner approved execution and requested continuation. Work is local and uncommitted on `lqosync3` in `/home/pi/LQoSync3.0`.
- Task 1 import comparator and native observer are implemented. Initial full checkpoint: 644 passed, zero failed, seven ignored; Clippy/fmt passed.
- Operator native probe now succeeds after correcting the envelope/transaction schema distinction. Three stable samples match import and shaping generation; native/generated schemas are 6. Transaction equality remains false and needs encoding/folding analysis.
- APPLIED is gated off. No false runtime acceptance is emitted.
- Task 2 health hub, durable write receipts, runner/status wiring, and existing sampler metadata hooks are in progress. Focused health/receipt tests passed; final full checks are pending.
- Task 3 Telegram worker and Task 4 shared TUI projection are not implemented. Do not deploy this unfinished worktree.
- Next: finish Task 2 review/tests, validate exact native contract, then continue the plan. Session note contains the current file map and risks.
- No production change, commit, push, restart, or live Telegram test send occurred. Preserve unrelated profile notes.

Session: [[2026-11-11-unified-health-implementation]].

## 2026-11-11T15:28:38+08:00 (Asia/Shanghai) — Unified health and Telegram design review

- Owner requested review/research/brainstorming and selected immediate WRITTEN/PENDING reports with APPLIED/FAILED follow-ups.
- The draft covers one logical update for the file pair, per-file outcomes, persisted write IDs, domain health, and asynchronous Telegram delivery.
- Confirmed gaps: premature applied title, missing acceptance-transition trigger, inline send latency, best-effort delivery, and partial-write visibility.
- Import comparison currently omits some policy-critical fields. Native generation correlation must be verified before enabling stronger APPLIED claims.
- Draft: `docs/superpowers/specs/2026-11-11-unified-system-health-telegram-design.md`. Independent health incident/recovery alerts and queue limits remain proposals.
- Reviewed official Telegram, Prometheus, SRE, installed scheduler, and pinned native state contract. Twenty-five focused notifier tests passed.
- No product code, production config, live Telegram message, restart, or deployment changed during this research.
- Performance follow-up (2026-11-11T15:36:15+08:00): draft adds receipt reuse, freshness deadlines, pending-only verification, bounded delivery work, and measurement gates. No implementation or measured speedup.
- Next: review the draft and validate read-only acceptance evidence before implementation planning.
- Preserve unrelated local-profile Knowledge note. `brain/Home.md` now has the new research-session link plus its pre-existing unrelated Knowledge link. Stage them separately.
- Branch/worktree: `lqosync3`, `/home/pi/LQoSync3.0`. Last agent: Codex.

Session: [[2026-11-11-unified-health-telegram-research]].

## 2026-11-11T08:21:05+08:00 (Asia/Shanghai) — NIC runtime recovery confirmed

- Continued verification found a newer LibreQoS bridge process, PID 630703, started at 08:09:05+08:00. This agent did not restart it.
- LQoSync remains active on `3.0.175+gb4e3846`, PID 596019, cycle 213, with no service/configuration error and zero automatic restarts.
- Binary and configuration hashes still match the reviewed release.
- Two samples eleven seconds apart confirmed advancing NIC timestamps and bridge uptime in the new process epoch.
- Bridge reports healthy. Both NIC ports remain UP at 10,000 Mbps, with 78 extended statistics each. Basic error increments were zero in the observed interval.
- Aggregate, health, host, NIC, and collection caches are fresh. The collector recovered its baseline after the bridge process change.
- The original immediate deployment audit compares against the deployment bridge PID/uptime. It is not a later-epoch runtime monitor.
- Current evidence: `/home/pi/.local/state/lqosync3/nic-b4e3846/latest-runtime-verification.json`. Original deployment evidence is preserved.
- No production code, config, permission, or service change occurred during this follow-up.

Session: [[2026-11-11-dpdk-nic-diagnostics]].

## 2026-11-11T01:26:13+08:00 (Asia/Shanghai) — Native NIC diagnostics deployment verified

- Operator deployed `3.0.175+gb4e3846` at 2026-11-11T01:24:51+08:00. Independent checks passed at 2026-11-11T01:25:35+08:00.
- Installed binary hash matches the clean artifact. Service is active/running, PID 596019, with zero automatic restarts.
- Cycle 1 succeeded in `apply` mode with target `v3_pair`. No service or configuration error exists. Configuration hash is unchanged.
- Root deployment matched native NIC peer/port identities, basic deltas, and extended field names against the service cache.
- Independent NIC timestamp advanced after deployment. Both ports report UP at 10,000 Mbps, full duplex, with 78 extended statistics each.
- Basic error, missed-packet, and no-mbuf increments were zero in the observed interval. This is an interval observation, not a long-term health guarantee.
- Existing aggregate, classification, health, host, queue, worker, circuit, and collection checks passed. Bridge reports healthy with no degraded subsystems.
- Three circuit samples matched and advanced, with false gap/reset flags. Installed TUI smoke passed all seven routes with clean exit and no panic.
- Backup: `/var/backups/lqosync3/nic-b4e3846-6koqynjh`. No rollback was required.
- Evidence: `/home/pi/.local/state/lqosync3/nic-b4e3846/` contains `DEPLOYED.json`, `independent-verification.json`, `deployed-nic-source.jsonl`, and `installed-tui-smoke.json`.
- Task complete. Reopen existing TUI sessions to load NIC diagnostics.
- Preserve unrelated `brain/Home.md` and local-profile Knowledge note. Branch/worktree: `lqosync3`, `/home/pi/LQoSync3.0`. Last agent: Codex.

Session: [[2026-11-11-dpdk-nic-diagnostics]].

## 2026-11-11T01:24:25+08:00 (Asia/Shanghai) — NIC diagnostics release pushed; activation pending

- Owner authorized commit, push, and deployment. Code `b4e3846` is on GitHub `lqosync3`.
- CI run `42863911257` passed test and MSRV jobs. Local checks remain current: 632 passed, zero failed, seven ignored.
- Clean detached source build produced `3.0.175+gb4e3846`. All seven release TUI routes passed smoke with clean exit and no panic.
- Artifact: `/home/pi/.local/state/lqosync3/nic-b4e3846/`. It includes source, manifest, CI evidence, binaries, native NIC probe, deploy, smoke, and independent verification scripts.
- Binary SHA256: `4058fdee9fea04b6f457a511f68108ecc06ac2ecbee206c3eb763e43acd41f4b`.
- Deployment adds native NIC endpoint/cache checks to the existing backup, atomic install, restart, cycle, collection, telemetry, and rollback checks.
- The NIC gate verifies peer PID, port IDs/names, fresh baseline increments, and extended field names. Its live-shape replay passed.
- `sudo -n` stopped before mutation because a password is required. Pending operator command: `sudo /home/pi/.local/state/lqosync3/nic-b4e3846/deploy.sh`.
- Production remains `3.0.172+gb6778cb`, PID 566154, active/running, zero automatic restarts. Configuration hash is unchanged.
- Next after operator execution: inspect `DEPLOYED.json`, run artifact `smoke.py /usr/local/bin/lqosync3`, then `verify.py`. The NIC cache timestamp must advance after deployment verification.
- Preserve unrelated `brain/Home.md` and local-profile Knowledge note. Branch/worktree: `lqosync3`, `/home/pi/LQoSync3.0`. Last agent: Codex.

Session: [[2026-11-11-dpdk-nic-diagnostics]].

## 2026-11-11T01:03:51+08:00 (Asia/Shanghai) — Native NIC diagnostics implemented locally

- Owner prioritized NIC diagnostics. Operator probe verified two UP, 10 Gbps full-duplex ports with 78 extended statistics each.
- Native endpoint: `/run/dpdk/rte/dpdk_telemetry.v2`; peer `bridge_rs`, DPDK 25.11.0.
- Added an independent ten-second sampler, bounded public `nic-telemetry.json`, and configurable native socket path.
- Dashboard health shows port summaries. Workers details and Copy show raw extended statistics through existing scrolling.
- Basic deltas rebase on failures, gaps, resets, process/port changes. Extended vendor statistics remain raw because they can include gauges.
- NIC-only settings changes preserve analytics histories and subscriber sampler state. Obsolete reads cannot publish after reload.
- Checks: 632 passed, zero failed, seven ignored, fmt, all-target Clippy, build, whitespace, source replay, and actual terminal capture review.
- Maximum cache: 312,567 bytes. One hundred debug detail formats: 90.7 ms. No new dependency or packet-path processing.
- Main code: `src/nic_stats.rs`, `src/nic_stats/{ipc,tests}.rs`, config/defaults, monitor lifecycle, TUI loader/overview/workers, and capture helper.
- Docs: `docs/design/tui/NIC_DIAGNOSTICS.md`, stats/operations/capture index, and two NIC terminal captures.
- Work is local and uncommitted. Production remains `3.0.172+gb6778cb`. No push, deployment, restart, config, or socket permission change occurred.
- Preserve unrelated `brain/Home.md` and local-profile Knowledge note. Branch/worktree: `lqosync3`, `/home/pi/LQoSync3.0`. Last agent: Codex.

Session: [[2026-11-11-dpdk-nic-diagnostics]].

## 2026-11-11T00:42:44+08:00 (Asia/Shanghai) — Independent directional scale deployment verified

- Operator deployed `3.0.172+gb6778cb`. Independent checks passed at 2026-11-11T00:42:21+08:00.
- Installed binary hash matches the clean artifact. Service is active/running, PID 566154, with zero automatic restarts.
- Cycle 1 succeeded in `apply` mode with target `v3_pair`. No service or configuration error exists. Configuration hash is unchanged.
- All six telemetry sources passed root deployment checks. Independent checks confirm fresh, advancing aggregate, classification, bridge health, and host data.
- Bridge reports healthy with no degraded subsystems. Three circuit samples matched their slot and advanced. Gap/reset flags were false.
- Collection cache is fresh and matches the service PID/cycle. Root deployment checks verified raw snapshot counts.
- Installed TUI smoke passed all seven routes, clean exit, and terminal cleanup, with no panic.
- Backup: `/var/backups/lqosync3/directional-b6778cb-kgw7_dtn`. No rollback was required.
- Evidence: `/home/pi/.local/state/lqosync3/directional-b6778cb/` contains `DEPLOYED.json`, `independent-verification.json`, and `installed-tui-smoke.json`.
- Download and upload now use separate dynamic ceilings. Reopen existing TUI sessions to load the installed renderer.
- Task complete. Preserve unrelated `brain/Home.md` and local-profile Knowledge note.
- Branch/worktree: `lqosync3`, `/home/pi/LQoSync3.0`. Last agent: Codex.

Session: [[2026-11-11-mirrored-dpdk-chart]].

## 2026-11-11T00:40:53+08:00 (Asia/Shanghai) — Directional scale release pushed; activation pending

- Owner authorized commit, push, and deployment. Code commit `b6778cb` is on GitHub `lqosync3`.
- CI run `42856413482` passed test and MSRV jobs. Local checks: 618 passed, zero failed, seven ignored.
- Clean detached source build produced `3.0.172+gb6778cb`. Release TUI smoke passed all seven routes with clean exit and no panic.
- Artifact: `/home/pi/.local/state/lqosync3/directional-b6778cb/`. It includes manifest, binary, circuit probe, CI evidence, smoke, deploy, and independent verification scripts.
- Binary SHA256: `207a64570b3221f623c063efdaa823468246f733515f224660827d3d7163d556`.
- Deployment verifies hashes/config, backup, atomic replacement, restart, successful cycle, collection, six telemetry sources, and rollback.
- `sudo -n` stopped before mutation because a password is required. Operator request: `sudo /home/pi/.local/state/lqosync3/directional-b6778cb/deploy.sh`.
- Production remains `3.0.169+gfd5430a`, PID 544762, active/running, zero automatic restarts.
- Next after operator execution: inspect `DEPLOYED.json`, run `smoke.py /usr/local/bin/lqosync3`, then `verify.py` from the artifact directory.
- Preserve unrelated `brain/Home.md` and local-profile Knowledge note. Branch/worktree: `lqosync3`, `/home/pi/LQoSync3.0`. Last agent: Codex.

Session: [[2026-11-11-mirrored-dpdk-chart]].

## 2026-11-11T00:37:02+08:00 (Asia/Shanghai) — Independent download and upload scales

- Owner clarified that download and upload must follow separate directional peaks. This replaces the equal-bound proposal below.
- The mirror layout remains. Download is above zero; upload is below zero. Each half has its own dynamic rate ceiling.
- Each ceiling follows its highest visible directional sample in the selected window, with headroom and fine decimal rounding.
- Only rendered coordinates are normalized. Cached histories, reported rates, and peaks retain their units and values.
- One canvas remains. The new segment renderer performs no per-segment allocation or extra source read.
- Checks passed: 618 tests, zero failed, seven ignored, fmt, all-target Clippy, build, whitespace, and actual terminal capture review.
- Maximum debug redraws: 1,505.9 ms for 100 frames with three 1,801-point histories; 818.7 ms for three 901-segment histories.
- Updated visual/operations docs and `dashboard-dynamic-ceiling-v1.png` show independent D/U labels and geometry.
- Work remains local and uncommitted. Production remains `3.0.169+gfd5430a`. No push, deployment, restart, or config change occurred.
- Preserve unrelated `brain/Home.md` and the local-profile Knowledge note.
- Branch/worktree: `lqosync3`, `/home/pi/LQoSync3.0`. Last agent: Codex.

Session: [[2026-11-11-mirrored-dpdk-chart]].

## 2026-11-11T00:32:42+08:00 (Asia/Shanghai) — Dynamic mirrored ceiling ready locally

- Owner prioritized dynamic traffic scaling before chart inspection.
- Fine decimal steps replace coarse 1/2/5 steps. Bounds follow the highest visible download/upload peak with headroom.
- Removed the Actual-specific 1 Mbps floor. All source selections use a 1 kbps minimum.
- Both mirror halves share bounds. Old peaks remain until expiry; hidden sources no longer set the ceiling.
- Checks passed: 617 tests, zero failed, seven ignored, fmt, all-target Clippy, build, whitespace, and actual terminal capture review.
- One initial full run hit an unchanged Journal test's channel-disconnect failure. Its isolated rerun and full-suite rerun passed.
- Changed: `src/tui/overview.rs`, `src/tui/traffic_chart.rs`, mirrored visual contract, capture index/image, and these notes.
- Enhancement is local and uncommitted. Production remains `3.0.169+gfd5430a`. No deployment or restart occurred.
- Preserve unrelated `brain/Home.md` and the local-profile Knowledge note. No new telemetry polling or render-path I/O exists.
- Branch/worktree: `lqosync3`, `/home/pi/LQoSync3.0`. Last agent: Codex.

Session: [[2026-11-11-mirrored-dpdk-chart]].

## 2026-11-11T00:20:56+08:00 (Asia/Shanghai) — Mirrored chart deployment verified

- Operator deployed `3.0.169+gfd5430a`. Independent checks passed.
- Installed binary hash matches the clean artifact. Service is active/running, PID 544762, with zero automatic restarts.
- Cycle 1 succeeded in `apply` mode with target `v3_pair`. No service or configuration error exists. Configuration hash is unchanged.
- All six telemetry sources passed the root deployment checks. Independent checks confirm fresh, advancing aggregate, classification, bridge health, and host data.
- Bridge reports healthy with no degraded subsystems. The circuit probe returned three advancing samples with matching slots and fifty RTT bins.
- Circuit gap flags: [False, False, False]. Counter reset flags: [False, False, False].
- Collection cache is fresh and matches the service PID and cycle. The root script verified raw snapshot counts; that snapshot is not user-readable.
- Installed TUI smoke passed all seven routes, clean exit, and terminal cleanup, with no panic.
- Backup: `/var/backups/lqosync3/mirror-fd5430a-5o36lykc`. No rollback was required.
- Evidence: `/home/pi/.local/state/lqosync3/mirror-fd5430a/` contains `DEPLOYED.json`, `independent-verification.json`, and `installed-tui-smoke.json`.
- Task complete. Reopen existing TUI sessions to use the mirrored chart.
- Preserve unrelated local-profile Knowledge notes. Branch/worktree: `lqosync3`, `/home/pi/LQoSync3.0`. Last agent: Codex.

Session: [[2026-11-11-mirrored-dpdk-chart]].

## 2026-11-11T00:18:38+08:00 (Asia/Shanghai) — Mirrored chart pushed; deployment pending

- Owner authorized commit, push, and deployment. Code commit `fd5430a` is on GitHub `lqosync3`.
- GitHub CI run `42852687985` passed the test and MSRV jobs. Local checks remain current: 616 passed, zero failed, seven ignored.
- Clean detached build produced `3.0.169+gfd5430a`. All seven release TUI routes passed the read-only smoke check.
- Artifact: `/home/pi/.local/state/lqosync3/mirror-fd5430a/`. The manifest records source, binary hashes, config hash, and CI result.
- Deployment script checks hashes and config, backs up, installs atomically, restarts, verifies the cycle and six telemetry sources, and restores the old binary on failure.
- `sudo -n` stopped before mutation because a password is required. The operator has a pending request to run `sudo /home/pi/.local/state/lqosync3/mirror-fd5430a/deploy.sh`.
- Production remains `3.0.166+g828061e`, active/running, PID 510745, zero automatic restarts. No configuration change occurred.
- Next: after operator execution, inspect `DEPLOYED.json`, verify installed version/hash, service/cycle identity, fresh advancing telemetry, and installed TUI smoke.
- Preserve unrelated local-profile Knowledge notes. Branch/worktree: `lqosync3`, `/home/pi/LQoSync3.0`. Last agent: Codex.

Session: [[2026-11-11-mirrored-dpdk-chart]].

## 2026-11-11T00:11:15+08:00 (Asia/Shanghai) — Combined mirrored chart implementation

- Owner approved implementation of one combined mirrored chart and requested a performance/reliability review.
- Actual, Mapped, and Unmapped start visible in blue, amber, and violet. Download is above zero; upload is below zero.
- Both halves share a symmetric scale. Stored and displayed upload rates remain positive. Delivered/enqueue classes remain separate overlays.
- Legend clicks and uppercase A/M/U toggle traces. v cycles focus and returns to the prior selection. a restores all sources.
- Source rates, freshness, ages, and counts remain independent. Time gaps and missing samples remain visible.
- A bounded curve cache tracks report identity, selected window, expiry, future points, clock regression, and source removal.
- Replaced expensive per-segment rendering with one canvas after a fragmented-history stress test exposed the cost.
- Checks passed: 616 tests, zero failed, seven ignored, fmt, Clippy, build, whitespace, and actual terminal capture review.
- Three full 1,801-point histories: 1123.6 ms for 100 cached debug redraws. Three 901-segment histories: 797.6 ms.
- The approved panel order, all themes, compact/resized layouts, and mouse scroll controls remain supported.
- Changed files: `src/tui/{overview,traffic_chart,app}.rs`, capture script, visual/operations docs, captures, plan, and these notes.
- Local implementation only. Production remains on verified `3.0.166+g828061e`. No push, deployment, restart, configuration, or router-policy change occurred.
- Preserve unrelated local-profile Knowledge notes. Branch/worktree: `lqosync3`, `/home/pi/LQoSync3.0`. Last agent: Codex.

Session: [[2026-11-11-mirrored-dpdk-chart]].
Visual contract: `docs/design/tui/MIRRORED_TRAFFIC.md`.

## 2026-11-10T23:24:06+08:00 (Asia/Shanghai) — DPDK deployment verified

- Operator deployed the reviewed release at 2026-11-10T23:23:22+08:00. Independent verification passed at 23:24:06+08:00.
- Installed `3.0.166+g828061e` matches the clean artifact checksum. Service active/running, PID 510745, zero automatic restarts.
- Deployment verified the running executable hash. Current service, cycle status, and deployment record retain the same PID/version.
- Successful cycle 1: mode `apply`, target `v3_pair`, no service/configuration error. Configuration checksum is unchanged.
- All six telemetry sources passed deployment checks. Independent checks confirm fresh aggregate, classification, health, and host caches with advancing timestamps/uptime.
- Bridge reports healthy with no degraded subsystems. The three circuit samples matched their slot and advanced, with false gap/reset flags.
- Installed TUI smoke passed all seven routes, clean quit, terminal cleanup, and no panic.
- Source commit `828061e` and documentation commit `830eed7` are on GitHub. Their CI checks passed. Local suite: 613 passed, zero failed, seven ignored.
- Backup: `/var/backups/lqosync3/dpdk-828061e-5hq48cg8`. No rollback was required.
- Evidence: `/home/pi/.local/state/lqosync3/dpdk-828061e/DEPLOYED.json`, `independent-verification.json`, `deployed-circuit-source.jsonl`, and `installed-tui-smoke.json`.
- Task complete. Reopen existing TUI sessions to use the installed UI. A root TUI is still required for direct circuit queue reads on this root-only socket.
- Preserve unrelated local-profile Knowledge notes. Branch/worktree: `lqosync3`, `/home/pi/LQoSync3.0`. Last agent: Codex.

Session: [[2026-11-10-dpdk-performance-and-deployment-review]].

## 2026-11-10T23:19:10+08:00 (Asia/Shanghai) — DPDK release pushed; root activation pending

- Reviewed code commit `828061e` and the six prior DPDK checkpoints are on GitHub `lqosync3`.
- GitHub CI run `42841241848` passed both test and MSRV jobs. Local checks: 613 passed, zero failed, seven ignored.
- Clean detached source build produced `3.0.166+g828061e`. Binary SHA256: `29ae89c2f5eac09b9a2597010460450d439ba8f43e6b0fb440d1b2e8de9a173b`.
- The release TUI passed seven routes, clean quit, and terminal cleanup against the live read-only configuration.
- The smoke harness forces complete redraws because raw terminal diffs can omit unchanged letters. Initial harness failures were not product failures.
- Artifact: `/home/pi/.local/state/lqosync3/dpdk-828061e/`. It contains the manifest, CI result, binary, circuit probe, deployment script, and smoke report.
- Deployment verifies hashes, config, backup, atomic replacement, restart, cycle, collection, six telemetry sources, and rollback.
- `sudo -n` stopped before mutation with `a password is required`. The operator has a pending request to run `sudo /home/pi/.local/state/lqosync3/dpdk-828061e/deploy.sh`.
- Next: after operator execution, inspect `DEPLOYED.json`, process/hash/version, current cycle, fresh caches, probe evidence, and installed TUI smoke.
- Production remains at `3.0.157+g39459a7` until that script succeeds. Do not report deployment complete without evidence.
- Preserve the unrelated local-profile Knowledge notes. Branch/worktree: `lqosync3`, `/home/pi/LQoSync3.0`. Last agent: Codex.

Session: [[2026-11-10-dpdk-performance-and-deployment-review]].

## 2026-11-10T23:07:32+08:00 (Asia/Shanghai) — DPDK performance review and deployment preparation

- Owner authorized review of all six DPDK tasks, improvements, commit/push, and deployment.
- Fixed an aggregate reload race. A read from superseded settings cannot publish its cache after the pending socket read completes.
- Shared adaptive rate units cover charts, classification, subscriber details, and copy. Stored values remain Mbps.
- Chart labels retain source scope, age, current state, window peaks, sample counts, and exact axis bounds. Time gaps break plotted lines.
- Operational alerts include bridge degradation and stale health. Long notices wrap within existing mouse scroll controls.
- Checks passed: 613 tests, zero failed, seven ignored, fmt, Clippy, build, whitespace, and actual terminal capture review.
- A maximum 1,801-point chart took 770.1 ms for 100 debug redraws at 240x40. No extra polling, dependency, or packet-path work exists.
- Next: push the reviewed commit, verify CI, build a clean artifact, and deploy with checksum, backup, health checks, and rollback.
- Non-interactive sudo currently requires a password. Complete the artifact and deployment script before requesting the operator's terminal action.
- Preserve unrelated local-profile Knowledge notes. Branch/worktree: `lqosync3`, `/home/pi/LQoSync3.0`. Last agent: Codex.

Session: [[2026-11-10-dpdk-performance-and-deployment-review]].
Review: `docs/design/tui/DPDK_REVIEW.md`.

## 2026-11-10T21:30:00+08:00 (Asia/Shanghai) — DPDK task 6 implementation

- Task 6 adds manual selected-circuit queues in Subscribers: `d`, mouse, copy, and existing scrolling.
- A cancellable background worker reads state/queue_stats/state for one slot. Stable generation and slot order are required.
- One request and a two-second cooldown bound each TUI. Per-request limits remain two seconds and 2 MiB, with a seven-second job deadline.
- Source age, missing fields, gap/reset, and stale values remain explicit. A persistent title retains state during scrolling.
- Queue memory uses the raw reported unit. Tin sojourn is separate from host RTT. No interval rates or loss percentages are inferred.
- Operator probe: three matching single rows, eight tins per direction, fifty RTT bins, advancing sequence, stable epoch, false gap/reset.
- The current user needs socket access. This host requires a root TUI. No broker, ACL, cache writer, or automatic queue poll exists.
- Checks passed: 606 tests, zero failed, seven ignored, fmt, Clippy, build, whitespace, source probe, and terminal capture review.
- A sixteen-tin-per-direction fixture produced 5,212 bytes. One thousand debug formats took 61.0 ms.
- All six local implementation checkpoints are complete. Publication and deployment await a later operator request.
- Preserve the unrelated local-profile Knowledge link and note. No push, deployment, restart, or production configuration change occurred.
- Branch/worktree: `lqosync3`, `/home/pi/LQoSync3.0`. Last agent: Codex.
- Changed files: `src/circuit_queue*`, `worker_util/ipc`, TUI action/detail paths, probe/fixture examples, capture script, review docs, and these notes.

Session: [[2026-11-10-dpdk-circuit-queues]].

## 2026-11-10T20:49:33+08:00 (Asia/Shanghai) — DPDK task 5 implementation

- Task 5 adds per-IP subscriber delivered rates/bytes, RTT sample medians, observed retransmit counters, and packet age at read.
- Three operator-run samples verified 26/26 applied slot/name/IP matches. Slot 1 names the first circuit. Producer timestamps advanced.
- A bounded sampler reads state/snapshot/state every ten seconds and shares monitor reload/shutdown ownership. Aggregate polling remains separate.
- A separate 1 MiB cache stores at most 512 shaped host summaries. Missing/capped rows remain unknown, with no online/offline inference.
- Canonical ownership rejects duplicate/overlapping IPs, including projected collection-only records. Routed prefixes remain unsupported.
- Subscribers keeps approved composition, persistent selection context, wrapped scrolling, mouse focus, page keys, and copy.
- Checks passed: 596 tests, zero failed, seven ignored, fmt, Clippy, build, whitespace, live probe, and capture review.
- Maximum host cache: 351,809 bytes; one debug publish/read sample 35.1 ms. A 75,000-row ownership index took 634.9 ms on reload.
- No push, deployment, restart, or production configuration change. Next: Task 6 on-demand circuit queue diagnostics.
- Preserve the unrelated local-profile Knowledge link and note.
- Branch/worktree: `lqosync3`, `/home/pi/LQoSync3.0`. Last agent: Codex.

Session: [[2026-11-10-dpdk-subscriber-observations]].

## 2026-11-10T20:13:44+08:00 (Asia/Shanghai) — DPDK task 4 implementation

- Task 4 adds independent bridge health, reported uptime, topology/classification state, and named degraded subsystems.
- Three operator-run samples reported healthy, no degraded subsystems, one worker, and advancing uptime. Empty topology tier stays unknown.
- One monitor runs health every ten seconds alongside the two-second snapshot cadence. Outcomes and freshness remain independent.
- Dashboard/Workers retain approved composition, scroll controls, and copy. The public cache stores bounded aggregate health only.
- Checks passed: 585 tests, zero failed, seven ignored, fmt, Clippy, build, whitespace, source probe, and capture review.
- Maximum cache fixture: 742,194 bytes after fallback; one synthetic debug publish/read sample 155.3 ms.
- No push, deployment, or restart. Next: Task 5 subscriber traffic, RTT, and retransmissions.
- Preserve the unrelated local-profile Knowledge link and note.
- Branch/worktree: `lqosync3`, `/home/pi/LQoSync3.0`. Last agent: Codex.

Session: [[2026-11-10-dpdk-bridge-health]].

## 2026-11-10T20:01:01+08:00 (Asia/Shanghai) — DPDK task 3 implementation

- Task 3 adds reported idle, laps/s, busy laps, cycles/busy lap, and ten cumulative phase counters.
- The operator probe verified three advancing samples. Laps/s equaled busy ppm; retain the producer-reported label.
- Workers retains approved composition, scrolling, copy, mouse focus, page keys, and a persistent worker/state title.
- The 1 MiB fallback preserves basic workers and histories with a reason. Source values remain intact in memory.
- Checks passed: 576 tests, zero failed, seven ignored, fmt, Clippy, build, and capture review.
- Synthetic maximum fallback: 739,546 bytes; one debug publish/read sample 154.6 ms.
- No push, deployment, or restart. Next: Task 4 bridge health, uptime, and degraded subsystems.
- Preserve the unrelated local-profile Knowledge link and note.
- Branch/worktree: `lqosync3`, `/home/pi/LQoSync3.0`. Last agent: Codex.

Session: [[2026-11-10-dpdk-worker-details]].

## 2026-11-10T18:27:59+08:00 (Asia/Shanghai) — DPDK task 2 implementation

- Task 2 adds whole-bridge mapped/unmapped enqueue counters, independent rates/histories, and explicit source selection.
- Actual remains delivered traffic. No subtraction from Actual, capped host sum, WAN attribution, or new poll exists.
- Root read probe verified both counters and producer progress. Evidence: `/home/pi/.local/state/lqosync3/dpdk-mapping-task2/live-source.jsonl`.
- Dashboard buttons/`v`, scrollable details, and copy show correct scope and stale/unknown states.
- Checks passed: 568 tests, zero failed, seven ignored, fmt, Clippy, build, whitespace, and capture review.
- Maximum-cache fixture: 740,161 bytes for 4,096 workers and three full histories, within the 1 MiB read limit.
- No push, deployment, or restart. Task 1 and Task 2 remain local implementation checkpoints.
- Next: Task 3 worker details. Task 4 still includes bridge health, uptime, and degraded subsystems, as clarified to the owner.
- Preserve unrelated capture-index/local-profile edits. Branch/worktree: `lqosync3`, `/home/pi/LQoSync3.0`. Last agent: Codex.

Session: [[2026-11-10-dpdk-mapped-unmapped]].

## 2026-11-10T18:07:51+08:00 (Asia/Shanghai) — DPDK task 1 implementation

- Owner requested six DPDK statistics checkpoints. Task 1 is queue drops, AQM marks, and backlog bytes.
- Code extends the same analytics poll/cache and the Dashboard/Workers details. No new service or request cadence.
- Read-only root probe passed: all fields exist and producer timestamps advance.
- Missing/stale/reset handling and whole-bridge scope are explicit. UI and copy suppress stale increments.
- Synthetic terminal captures: `docs/design/tui/captures/dashboard-queue-v1.png` and `workers-queue-v1.png`.
- Checks passed: 560 tests, zero failed, seven ignored, fmt, Clippy, build, whitespace, and capture review.
- Review/check logs: `/home/pi/.local/state/lqosync3/dpdk-queue-*.log`.
- No push, deployment, or restart. Next checkpoint: mapped/unmapped traffic.
- Preserve unrelated edits in the capture index and local-profile Brain discussion.
- Branch/worktree: `lqosync3`, `/home/pi/LQoSync3.0`. Last agent: Codex.

Session: [[2026-11-10-dpdk-queue-statistics]].

## 2026-11-09T20:32:29+08:00 (Asia/Shanghai) — Live collection deployment complete

- Operator deployed `3.0.157+g39459a7`. Installed checksum matches the artifact; deployment checked the running executable and raw source counts.
- Service active/running, PID `61153`, zero automatic restarts. Independent cycle 2 check passed with no service/config errors.
- Collection and status PID/sequence agree. Latest cycle: PPPoE 21 online / three offline; DHCP five BOUND leases; Hotspot disabled.
- Collection remains fresh under the existing sync-cycle cadence. DPDK telemetry is fresh and traffic history grows.
- Installed TUI read-only smoke passes all seven routes, DPDK panel, subscriber states, quit/cleanup, and no panic.
- Config unchanged. Backup: `/var/backups/lqosync3/tui-39459a7-6rmrzs5d`.
- Evidence: `/home/pi/.local/state/lqosync3/collection-39459a7/DEPLOYED.json` and `installed-tui-smoke.json`.
- Task complete. Next operator action: reopen existing TUI sessions. No implementation or deployment work remains.
- Branch/worktree: `lqosync3`, `/home/pi/LQoSync3.0`. Last agent: Codex.

Session: [[2026-11-09-live-subscriber-collection]].

## 2026-11-09T19:46:45+08:00 (Asia/Shanghai) — Cycle collection fix pushed; activation pending

- Owner requested live PPPoE/Hotspot/DHCP collection and states; chose refresh only with the current sync cycle.
- Code commit `39459a7` pushed. 552 tests pass, zero fail, seven ignored; fmt/Clippy/build/capture review pass. GitHub CI `42611316069` passed.
- Clean artifact: `3.0.157+g39459a7` in `/home/pi/.local/state/lqosync3/collection-39459a7/`.
- New cache `collection.json` comes from cycle reads. Optional local Hotspot inventory is read only for enabled sources; failure cannot block shaping.
- Subscribers projects collected-only accounts separately from shaping. Ambiguous identity, failed reads, old cycle/process, and expiry cannot claim live states.
- Operator command: `sudo /home/pi/.local/state/lqosync3/collection-39459a7/deploy.sh`. It includes rollback/restart and raw-count/runtime verification.
- Sudo requires an operator password. Attempt stopped before mutation; operator execution was requested.
- Next: inspect deployment record, version/PID/hash, successful cycle, collection and DPDK freshness, then smoke the installed UI.
- No production source enablement or scan interval change is authorized by this fix. Existing Hotspot source remains disabled.
- Branch/worktree: `lqosync3`, `/home/pi/LQoSync3.0`. Last agent: Codex.

Session: [[2026-11-09-live-subscriber-collection]].

## 2026-11-09T19:37:05+08:00 (Asia/Shanghai) — Live cycle collection implementation

- Owner requested real PPPoE/Hotspot/DHCP collection counts and states; chose refresh only with existing sync cycles.
- Code: `src/collection.rs`, Runner publishing, optional enabled-source Hotspot inventory, `tui/presence.rs`, loader/App wiring, source/state/filter UI.
- Subscriber projection keeps observed-only accounts outside shaping inputs; no changes to removal, grace, rates, or reload rules.
- Cache is bounded and omits secrets. Failed/stale/older-cycle evidence cannot become live or imply offline.
- New actual fixture captures: `docs/design/tui/captures/*-collection-v2.png`. Concepts remain unchanged.
- Performance evidence and tests are in `/home/pi/.local/state/lqosync3/collection-*.log`; see the session for measured limits.
- Next: finish verification, commit/push and build clean artifact, prepare deployment and runtime collection checks. Sudo needs operator authentication.
- Production Hotspot remains disabled; this task does not enable production sources or change scan intervals.
- Branch/worktree: `lqosync3`, `/home/pi/LQoSync3.0`. Last agent: Codex.

Session: [[2026-11-09-live-subscriber-collection]].

## 2026-11-09T18:45:30+08:00 (Asia/Shanghai) — TUI deployment complete

- Operator ran the prepared sudo deployment. Release `3.0.154+g7259316` is installed and running.
- Verified checksum matches the artifact; deployment verified running executable hash. Independent status agrees on PID `13860` and version.
- Service active/running, zero automatic restarts; successful cycle without service/config errors. Mode `apply`, target `v3_pair`, configuration unchanged.
- Fresh worker telemetry and actual DPDK rates pass; traffic history is growing. No rollback required.
- Installed TUI read-only live-config smoke passes all seven routes, DPDK panel, quit/cleanup, and no panic.
- Evidence: `/home/pi/.local/state/lqosync3/tui-7259316/DEPLOYED.json` and `installed-tui-smoke.json`.
- Backup: `/var/backups/lqosync3/tui-7259316-zv2e3xw_`.
- Owner task is complete: review, fix, tests/CI, push, deploy, restart, and verification. Reopen existing TUI sessions for the new UI.
- Branch/worktree: `lqosync3`, `/home/pi/LQoSync3.0`. Last agent: Codex.

Session: [[2026-11-09-tui-release-review]].

## 2026-11-09T18:42:04+08:00 (Asia/Shanghai) — TUI pushed; root deployment pending

- Code review/fix commit `7259316` and all TUI commits are pushed to `origin/lqosync3`. GitHub CI `42602346450` passed.
- Clean release: `3.0.154+g7259316`; 542 tests pass, zero failures, seven manual ignored; fmt/Clippy/release build pass.
- Artifact and concrete deployment script: `/home/pi/.local/state/lqosync3/tui-7259316/`.
- Run as operator: `sudo /home/pi/.local/state/lqosync3/tui-7259316/deploy.sh`. It includes rollback, restart, and executable/cycle/telemetry checks.
- Non-interactive sudo requires a password. The attempted deployment stopped before changing the binary. Operator execution was requested.
- Last observed service: active/running, PID 1833, `3.0.144+g1d2b4d5`. No deployment or restart occurred in this task yet.
- Next: after operator execution, inspect `DEPLOYED.json`, current process version/hash, successful cycle, and fresh worker/traffic cache. Do not claim deployment from build success.
- Branch/worktree: `lqosync3`, `/home/pi/LQoSync3.0`. Last agent: Codex.

Session: [[2026-11-09-tui-release-review]].

## 2026-11-09T18:38:29+08:00 (Asia/Shanghai) — Reviewed TUI release preparation

- Owner now authorizes push, binary deployment, and restart after a clean performance review. This supersedes local-only instructions below.
- Branch/worktree: `lqosync3`, `/home/pi/LQoSync3.0`. Last agent: Codex.
- Review found and fixed clock-regression ordering during analytics read failure in `src/worker_util/traffic.rs`.
- Regression test failed before the fix. Final suite: 542 pass, zero fail, seven manual ignored; Clippy, formatting, and whitespace pass.
- Synthetic 50,000-row debug benchmark: 2.013–7.703 ms/frame across seven screens/two sizes; unchanged snapshot mean 0.001 ms.
- Evidence: `/home/pi/.local/state/lqosync3/tui-release-review/`. See the linked session for limits.
- Next: commit/push, build a clean release, prepare atomic binary swap/rollback and post-restart checks. Root execution needs sudo authentication.
- Production remains active at `3.0.144+g1d2b4d5`. Do not report deployment until binary/process/status/telemetry checks pass.

Session: [[2026-11-09-tui-release-review]].

## 2026-11-09T18:32:30+08:00 (Asia/Shanghai) — All approved TUI screens complete

- Goal achieved: Dashboard -> Subscribers -> Routers -> LibreQoS -> Workers -> Settings -> Journal implemented, reviewed, and locally checkpointed. Do not push.
- Last agent: Codex. Branch/worktree: `lqosync3`, `/home/pi/LQoSync3.0`.
- Prior commits: `d9e0ffb`, `72b09c6`, `74a12c4`, `7f6f61b`, `62007b1`, `f49b377`. This checkpoint contains Journal and final docs.
- Final code: `src/tui/journal/approved.rs`, Journal lifecycle/retention/search, App input/dialog mouse routing, bounded main-loop queue/drain, Journal header freshness.
- Checks: 541 tests pass, zero fail, seven manual ignored; formatting, Clippy, build, whitespace, 15 concept hashes, 63 frames, capture PNGs/links, and isolated PTY smoke pass.
- Evidence: `docs/design/tui/captures/README.md`; logs and frames in `/home/pi/.local/state/lqosync3/tui-complete-review-20261109`.
- Performance/integrity: visible rows, cached filters/diffs, bounded history/logs, original retained-record copy, explicit unknown/stale states, existing Settings save/secret gates.
- No implementation work remains in the approved seven-screen slice. Deployment remains separate: live DPDK history needs the new monitor; privileged Settings terminal verification needs the operator's sudo access.
- No install, service restart, production configuration write, or push occurred. Next action: report completion; wait for deployment/push instructions.

Session: [[2026-11-09-approved-tui-implementation]].

## 2026-11-09T15:26:26+08:00 (Asia/Shanghai) — Settings checkpoint

- Owner task remains seven screens, per-screen review/test/capture and local commits. Do not push.
- Completed commits: Dashboard `d9e0ffb`, Subscribers `72b09c6`, Routers `74a12c4`, LibreQoS `7f6f61b`, Workers `62007b1`.
- Settings passes checks: full router form, loaded/draft values, review/apply, mouse/list/dialog/scroll controls, and existing save/secret gates.
- New code: `src/tui/settings/view/approved.rs`, `form.rs`; atomic router-config undo helper in Draft.
- Shared fix: clip partially visible mouse targets; account for hidden row offsets in compact previews.
- Checks: 534 tests pass, zero fail, seven manual ignored; all-target Clippy, formatting, whitespace, build, and capture review pass.
- Actual read-only terminal image and two labeled writable Ratatui fixture renders are in `docs/design/tui/captures/settings-*.png`.
- Sudo needs a password; user namespaces are restricted. No privileged running-terminal capture or production change occurred.
- Next: commit Settings locally, then implement approved Journal and global confirmation mouse buttons. Finish full verification and update all design statuses.
- Branch/worktree: `lqosync3`, `/home/pi/LQoSync3.0`. Last agent: Codex.

Session: [[2026-11-09-approved-tui-implementation]].

## 2026-11-09T14:38:28+08:00 (Asia/Shanghai) — Workers checkpoint

- User authorized all seven approved screens with per-screen review/test/capture and local commits. Do not push.
- Done: Dashboard `d9e0ffb`, Subscribers `72b09c6`, Routers `74a12c4`, LibreQoS `7f6f61b`; Workers passes checks.
- Code: `src/tui/workers/approved.rs`, WorkerView state, guarded app input paths.
- Review: cache/filter work outside visible-row draw; selection/copy uses reported worker; timer expiry invalidates Live filter; pressure needs fresh complete diagnostics for CLEAR.
- Checks: 529 tests pass, zero fail, six ignored; all-target Clippy, formatting, whitespace, build, and real terminal screenshot review pass.
- Capture: `docs/design/tui/captures/workers-v1.png` uses isolated synthetic data.
- Next: commit Workers locally, then Settings -> Journal. Keep existing exact-draft preview/apply and secret gates.
- Branch/worktree: `lqosync3`, `/home/pi/LQoSync3.0`. Last agent: Codex. Production unchanged.

Session: [[2026-11-09-approved-tui-implementation]].

## 2026-11-09T14:29:51+08:00 (Asia/Shanghai) — LibreQoS checkpoint

- Owner task remains all approved screens with review, tests, captures, and local commits. Do not push.
- Done: Dashboard `d9e0ffb`, Subscribers `72b09c6`, Routers `74a12c4`; LibreQoS now passes checks.
- LibreQoS source: `src/tui/libreqos.rs`, native input retention in loader, app guarded sync routing, local timestamp formatter.
- Removed the temporary `operations.rs`. New view formats JSON only on input changes and builds visible rows only.
- Capture: `docs/design/tui/captures/libreqos-v1.png`, real Zutty with isolated synthetic data.
- Checks: 527 tests pass, zero fail, six ignored; all-target Clippy, formatting, whitespace, build, and screenshot review pass.
- Next: commit LibreQoS locally, then Workers -> Settings -> Journal. No production changes.
- Branch/worktree: `lqosync3`, `/home/pi/LQoSync3.0`. Last agent: Codex.

Session: [[2026-11-09-approved-tui-implementation]].

## 2026-11-09T14:20:19+08:00 (Asia/Shanghai) — Routers checkpoint

- Goal: complete all approved screens with review/tests/captures and local commits. Do not push.
- Done locally: Dashboard `d9e0ffb`, Subscribers `72b09c6`; Routers now passes review and checks.
- Router code: `src/tui/routers.rs`, app routing, and Subscriber router-filter navigation.
- Checks: 525 tests pass, zero fail, six ignored; all-target Clippy, build, formatting, whitespace, and real terminal capture review pass.
- Capture: `docs/design/tui/captures/routers-v1.png`, isolated synthetic data. Config parse only on config changes; visible rows only.
- Router copy contains only the IPv4/hostname/bracketed IPv6 endpoint. Actions only navigate; they do not modify Settings drafts.
- Next: commit Routers locally, then LibreQoS -> Workers -> Settings -> Journal.
- Branch/worktree: `lqosync3`, `/home/pi/LQoSync3.0`. Last agent: Codex. Production unchanged.

Session: [[2026-11-09-approved-tui-implementation]].

## 2026-11-09T14:13:22+08:00 (Asia/Shanghai) — Subscribers checkpoint

- Goal remains all approved screens, with review/tests/captures and local commits. Do not push.
- Dashboard local commit: `d9e0ffb`. Subscribers body now passes review and verification.
- Code: `src/tui/devices/approved.rs`, device index integration, app input routing, and shared selection style.
- Capture: `docs/design/tui/captures/subscribers-v1.png`; synthetic fixtures in a real terminal.
- Checks: 523 tests pass, zero fail, six ignored; all-target Clippy, formatting, whitespace, and build pass.
- Review: visible rows only; cached filters; unknown online state; guarded input; bounded OSC 52 copy; independent table/detail scrolling.
- Next: commit Subscribers locally, then Routers -> LibreQoS -> Workers -> Settings -> Journal.
- Branch/worktree: `lqosync3`, `/home/pi/LQoSync3.0`. Last agent: Codex. Production unchanged.

Session: [[2026-11-09-approved-tui-implementation]].

## 2026-11-09T14:05:42+08:00 (Asia/Shanghai) — Approved screens implementation: Dashboard

- Owner authorized all seven screen bodies, per-screen review/test/capture, and local commits. Do not push.
- Journal mockup is approved unchanged. Last agent: Codex. Branch/worktree: `lqosync3`, `/home/pi/LQoSync3.0`.
- Dashboard is implemented: cards, actual aggregate DPDK rates, bounded history, health, planned WAN, alerts, refresh, and scroll input.
- New code: `src/tui/overview.rs`, `design.rs`, `src/worker_util/traffic.rs`; integration in app/loader/monitor.
- Actual screenshot and reproducible capture script: `docs/design/tui/captures/dashboard-v1.png`, `scripts/dev/capture_tui.py`.
- Checks: 521 tests pass, zero fail, six ignored; all-target Clippy, formatting, whitespace, debug build, and screenshot review pass.
- The new monitor needs deployment to populate traffic on the live system. Production is unchanged.
- Next action: commit this checkpoint locally, then implement Subscribers and continue Routers -> LibreQoS -> Workers -> Settings -> Journal.
- Keep existing Settings gates, source freshness, bounded data work, and keyboard/mouse parity.

Session: [[2026-11-09-approved-tui-implementation]].

## 2026-11-09T13:46:29+08:00 (Asia/Shanghai) — Journal proposal and transition review

- Goal: Update visual documentation, compare exact mockups, propose Journal, and assess terminal evolution.
- Last agent: Codex. Branch: `lqosync3`. Worktree: `/home/pi/LQoSync3.0`.
- Changed in this task: root README link, visual contract, new `REVIEW_AND_EVOLUTION.md`, `JOURNAL_PROMPT.md`, Journal PNG, hashes, Home, this handoff, and session note.
- Journal state: proposal only. No owner approval has been recorded.
- Review: all seven canonical approved views inspected; new Journal reviewed against the shared shell and its own screenshot.
- Feasibility: current Ratatui supports the requested composition. Reuse current workflows; add missing read models, focus, forms, and complete mouse controls.
- Checks: fifteen PNG hashes/dimensions pass; fourteen prior hashes unchanged; local design links and whitespace pass. Manual prose review followed the STE heuristic.
- Runtime status: shared shell implemented; screen bodies, DPDK chart/history, full mouse, and actual screenshot parity remain pending.
- Journal needs field parsing and stream lifecycle/receive evidence. Preserve bounded retention, paused selection, minimum-level filters, OSC 52 fallback, and terminal selection.
- Next action: review Journal proposal, then implement approved bodies under `docs/design/tui/REVIEW_AND_EVOLUTION.md` acceptance steps.
- No application code, production config, service, commit, or push changed. Preserve earlier uncommitted work.

Session: [[2026-11-09-journal-design-and-tui-evolution]].

## 2026-11-09T13:29:01+08:00 (Asia/Shanghai) — First TUI shared-shell slice

- Goal: Begin the approved web-style TUI with the shared shell. First-slice checks pass.
- Last agent: Codex. Branch: `lqosync3`. Worktree: `/home/pi/LQoSync3.0`.
- Code: `src/tui/app.rs`, `mod.rs`, new `shell.rs`, and new `operations.rs`.
- Docs: Operations, visual-contract implementation status, first-slice plan, Home, session note, and this handoff.
- Preserve all earlier uncommitted design assets and documentation.
- Behavior: Dashboard default; seven key/click routes; wide sidebar; local theme; cached status bodies; modal/input guards; resize target invalidation.
- Checks: 516 tests pass, zero fail, six manual ignored; all-target Clippy with warnings denied, debug build, formatting, whitespace, and fourteen image hashes pass.
- Export: 63 synthetic-data terminal frames. Isolated pseudo-terminal navigation/theme/resize/quit passes with terminal cleanup and no panic.
- Evidence: `/home/pi/.local/state/lqosync3/tui-shell-review-20261109`; full-suite and Clippy logs under `/tmp/lqosync3-shell-*.log`.
- Unfinished: approved screen bodies, DPDK history and source handling, subscriber online evidence, full mouse controls and scrollbar dragging, Journal design, compact visual review, and actual screenshot comparison.
- Next action: implement Dashboard body from its approved DPDK mockup while preserving source freshness and missing-data semantics.
- No commit, push, install, production configuration write, or service restart occurred in this slice.

Session: [[2026-11-09-tui-shell-implementation]].

## 2026-11-09T10:52:22+08:00 (Asia/Shanghai) — Mouse interaction requirements

- Goal: Document mouse clicks across all screens, wheel support, and interactive scrollbars.
- Last agent: Codex. Branch: `lqosync3`. Worktree: `/home/pi/LQoSync3.0`.
- Changed files in this task: visual contract, session note, and this handoff. Preserve earlier uncommitted design work.
- Owner requirement: every enabled action supports clicks; overflowing panels support wheel input; visible scrollbars support track clicks and thumb drags.
- Scope includes Settings, router creation, typed editing, choices, toggles, preview/apply, and dialogs.
- Existing foundation: crossterm capture and partial mouse handling. Full interaction support is a future implementation task.
- Integrity: mouse input uses the same action gates as keyboard input. Dialogs block click-through. Resize invalidates stale targets.
- Appearance: approved screenshots remain unchanged. Interaction requirements supplement the visual baseline.
- Checks: all fourteen image hashes unchanged; local design links and whitespace pass. No application tests apply to this documentation task.
- Next action: use the mouse contract during implementation and prove clicks, wheel, scrollbar drag, and keyboard parity in real terminals.
- No application code, production configuration, shaping input, or service changed.

Session: [[2026-11-09-tui-approved-mockups]].

## 2026-11-09T10:43:59+08:00 (Asia/Shanghai) — Settings approved and documented

- Goal: Record approval of both Settings views and review the visual contract against the exact screenshots.
- Last agent: Codex. Branch: `lqosync3`. Worktree: `/home/pi/LQoSync3.0`.
- Owner approvals: Dashboard, Subscribers, Routers, LibreQoS, Workers, and Settings (Add Router and Edit Values / Review Changes).
- Canonical Settings references: `settings-add-router-approved-v1.png` and `settings-values-approved-v1.png` under `docs/design/tui/mockups/`.
- Review: group/list/form placement, fields, toggles, masks, draft actions, loaded/draft values, units, diff, preview/apply gates, status strips, and footer match.
- Image integrity: both approved Settings assets match the reviewed final proposals byte for byte. No generation or image edit occurred.
- Changes: visual contract, approval metadata in prompt records, canonical image copies, hashes, README, session note, and this handoff.
- Checks: fourteen PNGs decode at 2172 by 724 pixels; all twelve prior hashes unchanged; approved copies and local design links pass; whitespace passes.
- Functional requirements remain: router add/edit/remove, typed values, exact-draft preview, explicit confirmation, backup/save, reload, and service-result verification.
- Unfinished: new-layout implementation, functional end-to-end checks, and real-terminal screenshot comparison. This approval does not establish runtime acceptance.
- Next action: use `docs/design/tui/README.md` and its approved images as the implementation baseline when implementation is requested.
- Preserve previous uncommitted design work. This task changed no application code, production configuration, shaping input, or service.

Session: [[2026-11-09-tui-approved-mockups]].

## 2026-11-09T10:34:18+08:00 (Asia/Shanghai) — Workers approved; functional Settings proposals

- Goal: Record Workers approval, check its screenshot contract, and propose Settings with real router and value workflows.
- Last agent: Codex. Branch: `lqosync3`. Worktree: `/home/pi/LQoSync3.0`.
- Owner approvals: Dashboard, Subscribers, Routers, LibreQoS, and Workers. Preserve the approved visual compositions.
- Workers integrity: `workers-approved-v1.png` matches the reviewed proposal byte for byte.
- Settings: two proposals, Add Router and Edit Values / Review Changes. Current canonical proposal paths are in the visual contract.
- Functional requirement: draft -> validate/preview -> review/confirm -> backup/save -> reload/verify. Do not bypass existing editor controls.
- Source review: add-router drafts, pending secrets, typed metadata, validation, preview, apply, conflicts, and service verification already exist.
- Changes: documentation, prompt records, images, and hashes only. Preserve earlier uncommitted design work.
- Checks: all eight prior image hashes unchanged; approved Workers copy identical; twelve valid PNGs at 2172 by 724; local links and source references pass; whitespace passes.
- Unfinished: Settings visual approval, new-layout implementation, functional end-to-end checks, and real-terminal comparison.
- Next: review the two Settings proposals. A mockup is not evidence of a successful router add or config apply.
- No application code, production configuration, shaping input, or service changed in this task.

Session: [[2026-11-09-tui-approved-mockups]].

## 2026-11-09T10:25:56+08:00 (Asia/Shanghai) — LibreQoS approved; Workers proposal

- Goal: Record LibreQoS approval, compare its contract with the screenshot, and provide the Workers proposal.
- Last agent: Codex. Branch: `lqosync3`. Worktree: `/home/pi/LQoSync3.0`.
- Owner approvals: Dashboard, Subscribers, Routers, and LibreQoS. Preserve the exact approved visual compositions.
- Changed files: visual contract, mockup assets and hashes, prompt records, root README, session note, and this handoff.
- LibreQoS review: actions, notice, sync cards, source tabs, preview columns, selection, page controls, Data status, and footer match.
- Image integrity: `libreqos-approved-v1.png` is identical to the reviewed `libreqos-proposal-v1.png`.
- Workers: proposal only. Summary cards, worker table, selected details, pressure panel, and snapshot status. Read-only.
- Checks: eight PNGs decode at 2172 by 724 pixels. All six prior hashes are unchanged. Local design links and whitespace checks pass. No application tests apply.
- Unfinished: Workers approval, runtime implementation, traffic history, and real-terminal visual comparison.
- Next action: review `docs/design/tui/mockups/workers-proposal-v1.png`. Use the visual contract for future implementation.
- Preserve prior uncommitted design work. No application code, production input, or service changed.

Session: [[2026-11-09-tui-approved-mockups]].

## 2026-11-09T10:20:36+08:00 (Asia/Shanghai) — Routers approved; LibreQoS proposal

- Goal: Record Routers approval, check the contract against its screenshot, and provide the LibreQoS screen proposal.
- Last agent: Codex. Branch: `lqosync3`. Worktree: `/home/pi/LQoSync3.0`.
- Owner approvals: Dashboard, Subscribers, and Routers. Keep the exact approved visual compositions.
- Changed files: visual contract, prompt record, mockup assets and hashes, root README, session note, and this handoff.
- Routers review: cards, filters, table columns, selection, detail groups, Data status, and footer match the approved image.
- Image integrity: `routers-approved-v1.png` matches the reviewed `routers-proposal-v1.png` byte for byte.
- LibreQoS: proposal only. Sync cards, native compatibility notice, remote status, and paged read-only source preview.
- Checks: six PNGs decode at 2172 by 724 pixels. Prior hashes are unchanged. Local design links and whitespace checks pass. No application tests apply.
- Unfinished: LibreQoS approval, runtime implementation, traffic history, and real-terminal visual comparison.
- Next action: review `docs/design/tui/mockups/libreqos-proposal-v1.png`. Use the visual contract for future implementation.
- Existing uncommitted design documentation remains intact. No application code, production input, or service changed.

Session: [[2026-11-09-tui-approved-mockups]].

## 2026-11-09T10:14:38+08:00 (Asia/Shanghai) — Approved TUI mockups

- Goal: Update the Dashboard design and documentation, compare with approved screenshots, and provide a Routers proposal.
- Last agent: Codex. Branch: `lqosync3`. Worktree: `/home/pi/LQoSync3.0`.
- Owner approval: Dashboard and Subscribers composition. Dashboard chart title/source: `LibreQoS DPDK Traffic`.
- Changes: `docs/design/tui/` contract and PNG assets, README link, earlier cosmetics-spec pointer, Home, and session note.
- Review: Dashboard v2 retains the approved composition with the selected title. Subscribers PNG remains the original approved asset.
- Status: design documentation only. Routers remains a proposal. Runtime application code and service state did not change.
- Checks: four valid PNGs, each 2172 by 724 pixels. Original approved assets match byte for byte. Local contract links and whitespace checks pass.
- Artifact hashes: `docs/design/tui/SHA256SUMS`. The generation record contains the final prompts. No application tests apply.
- Unfinished: Router design approval, runtime redesign, DPDK traffic collection/history, and real-terminal visual comparison.
- Next action: review the Routers screenshot. Use `docs/design/tui/README.md` as the visual baseline for future implementation.
- Preserve the earlier buffer-pressure deployment task below. This design task does not resolve its live deployment status.

Session: [[2026-11-09-tui-approved-mockups]].

## 2026-11-09T09:42:49+08:00 (Asia/Shanghai) — Buffer and ring pressure

- Goal: Complete priority task 3, review, checks, commit, push, deployment, and restart.
- Last agent: Codex. Branch: `lqosync3`. Worktree: `/home/pi/LQoSync3.0`.
- Changes: optional snapshot diagnostics, pressure evaluation, counter baselines, transition journal alerts, Workers display, tests, and operations notes.
- Checks: 511 tests pass. Formatting and Clippy pass. No unresolved review finding.
- Performance: same bounded two-second poll. No new dependency or service. No packet-path changes.
- Live state before deployment: active `3.0.142+g8ee064d`, PID 99607, `v3_pair`.
- Code commit: `1d2b4d5`, pushed. CI run `42547035993` passes, including Rust 1.89 MSRV.
- Release: `3.0.144+g1d2b4d5`. Build and artifact hash check pass. Deployment script syntax passes.
- Unfinished: root deployment and live verification.
- Next action: operator runs `sudo /home/pi/.local/state/lqosync3/buffer-pressure-1d2b4d5/deploy.sh`. Sudo needs interactive authentication.
- The script checks the artifact, backs up the binary, restarts, checks a successful cycle and fresh diagnostics, and restores on failure.
- This handoff update changes notes only. The prepared binary remains the verified code commit `1d2b4d5`.

Session: [[2026-11-09-buffer-pressure]].

## 2026-11-09T09:33:41+08:00 (Asia/Shanghai) — Worker utilization verified

- Goal: Complete priority task 2, review, checks, commit, push, deployment, and restart.
- Last agent: Codex. Branch: `lqosync3`. Worktree: `/home/pi/LQoSync3.0`.
- Code: `febea1f` and `8ee064d`, pushed. Changes cover the bounded analytics client, monitor, cache, socket config, Workers tab 6, tests, and docs.
- Checks: 503 tests pass. Clippy and formatting pass. Final GitHub CI test and Rust 1.89 MSRV jobs pass.
- Review: no unresolved finding. One bounded request per two seconds. Only visible table rows are formatted. Independent TUI ticks expire stale metrics.
- Live state: service active, version `3.0.142+g8ee064d`, PID `99607`, `v3_pair`, cycle 2, no cycle error.
- Production telemetry: fresh cache, no error reason, advancing samples. Worker 0 sample: 43.1 percent busy, 56.9 percent idle, mean backlog 8 packets.
- Changed files in this verification: this handoff, the session note, and the implementation plan. No application code changed.
- Unfinished: none for task 2. Live throughput was not benchmarked.
- Next action: user can open Workers with key 6. Further integration tasks require their own scope.

Session: [[2026-11-09-worker-utilization]].

## 2026-11-09T08:52:00+08:00 (Asia/Shanghai) — Optional node minimums

- Goal: Implement priority task 1, review, check, commit, push, and restart LQoSync3.
- Last agent: Codex. Branch: `lqosync3`. Worktree: `/home/pi/LQoSync3.0`.
- Changes: config, node state, pair output, minimum validation, diffs, native translation, Settings metadata, tests, and docs.
- Checks: 489 tests pass, clippy and formatting pass. One additional installed pair compiler test passes.
- Review: fixed preserve-mode loss with old state and manual edits. Added regression tests. No new packet-path work or dependencies.
- Live state: unchanged configuration, `v3_pair`, service version `3.0.136+ge8d6cbd`. No minimums enabled.
- Unfinished: root deployment and post-restart verification. Passwordless sudo is unavailable. The service restart requires interactive authentication.
- Next: commit and push the verified source, build that revision, and prepare the operator deployment command.

Session: [[2026-11-09-node-minimums]].

## 2026-11-03T09:30:00+08:00 (Asia/Manila) — LibreQoS v3 package update 3.0.7.202611021200

| Item | Value |
|------|-------|
| Goal | Update the LibreQoS v3 packages and verify the live shaper |
| Last agent | Claude Code |
| Branch and revision | `lqosync3`, `e8d6cbd`. This task changed notes only. |
| Operator changes | `sudo apt full-upgrade`: `libreqos` and `libreqos-dataplane` to `3.0.7.202611021200`, `libreqos-api` removed (now in `lqosd` at `/api/v3`). `bridge.env` replaced with the maintainer version; old file at `/etc/libreqos/bridge.env.dpkg-old`. |
| Runtime | `lqosd`, bridge, scheduler active. 26 mappings applied, `tripwire=ok`, traffic in both directions. mlx5 `txq_mem_algn=0` override still active. LQoSync `3.0.136+ge8d6cbd`, `apply`, `v3_pair`, no circuit changes. |
| Checks | Native probe ran again for the new version: `blocked`, PROBE-SITE 1/1 Mbps. The defect is not fixed in this build. |
| Decisions | None. Keep `[topology] source = "shaped_devices"`. |
| Next action | After each LibreQoS update, compare the EAL arguments in `50-ports.conf` and `90-mlx5-compat.conf` ([[libreqos-v3-mlx5-compat]]). Check the Web UI and `/api/v3`. |

Session: [[2026-11-03-libreqos-v3-package-update]].
NIC firmware upgrade `14.25.1020` to `14.32.1912` is prepared, not done: `docs/reports/2026-11-03-nic-firmware-upgrade.md`. Verified images and `mstflint` are in `/home/pi/nic-firmware/`. Update 10:12: the Secure Boot lockdown blocks `mstflint` (`direct PCI access is restricted`). No backup exists. Next: report section 8.4 (signed `mstflint-dkms` module, MOK enrollment at a reboot with a physical console). A bridge stop during the test caused a 2 min 24 s traffic stop (10:12:09 to 10:14:33). The bridge recovered with 26 circuits.
The LibreQoS packages are still not on hold. The earlier hold command names `libreqos-api`, which is now removed. Use `sudo apt-mark hold libreqos libreqos-dataplane`.

## 2026-11-02T18:30:00+08:00 (Asia/Manila) — v3 import request, native probe, bridge monitor

| Item | Value |
|------|-------|
| Goal | Make the live `v3_pair` path fast and safe. Block the native source while LibreQoS has the 1 Mbps defect. Show the bridge load. |
| Last agent | Claude Code |
| Branch and revision | `lqosync3`, `727c068` (pushed) |
| Changes | Reload request and import check (`0978937`), native probe and `v3_network` guard (`68e6005`), defect report and ADR-0011 to 0013 (`ff6756f`), bridge monitor (`727c068`) |
| Runtime | `lqosync3` apply/auto, `v3_pair`. Installed `3.0.128+gff6756f`. Release `727c068` is built and not installed yet. Host time zone Asia/Manila. |
| Checks | 462 tests pass, clippy clean. Live: reload request in the same second, import in about 3 s, `imported by LibreQoS`. Probe `blocked` (1/1 Mbps). |
| Decisions | `subscriber_grace = 0` stays (ADR-0012). LQoSync does not change `/etc/lqos.conf`. |
| Next action | Operator: hold the LibreQoS packages, install `727c068`, post the upstream issue. |

Session: [[2026-11-02-v3-import-probe-bridge-monitor]].
The LibreQoS packages were not on hold on 2026-11-02. Run `sudo apt-mark hold libreqos libreqos-api libreqos-dataplane`.

## 2026-11-02T14:29:00+08:00 (Asia/Shanghai) — native policy fails isolated daemon validation

The user asked whether runtime processing repairs the native compiler's missing node rates.
Two isolated cold starts of the installed daemon compared pair and native sources with the same example config.
The native watcher accepted the exact candidate hash. Operator view and final transaction still show 1 Mbps group limits.
All five group policies differ. All 25 matched runtime leaf plans also differ and have 1 Mbps ceilings.
Both modes emitted 25 of 26 fixture circuits. That shared exclusion remains unexplained.
There was no physical dataplane or throughput test.

Production remains `shaped_devices` / `full`, with active LQoSync and DPDK and a healthy matching seam.
No live file hash changed, no service restarted, and no source switch occurred.
All task containers were removed. Temporary executable copies were removed. Private JSON/log artifacts remain.
Report: `/home/pi/.local/state/libreqos-native-check-s12invb1/runtime-8ac8796e/policy-comparison.json`.
Session: [[2026-11-02-native-runtime-policy-validation]].

Next: correct the full compiler's canonical reconstruction in a separate authorized task, then repeat policy equivalence and recovery checks.
Do not activate native input on this build to test client traffic.
Repository branch/revision: `lqosync3`, `0978937` (existing user commit). This task changed notes only.

## 2026-11-02T13:50:56+08:00 (Asia/Shanghai) — native source validation completed

The user requested validation of `libreqos_network` as the active source.
The live source remains `shaped_devices`, with LQoSync apply/auto and v3_pair.
Both production services remain active. No live input or config hash changed.

The native writer produced five nodes and 26 circuits with zero alerts.
The installed Python binding compiled both formats successfully in isolated directories with socket connections blocked.
Compiled subscriber fields match except comments. Native node policy does not match:
the import retains supplied rates, but the compiled canonical output loses kinds/rates and emits 1 Mbps compatibility node limits.
Do not activate the source until this discrepancy is resolved or a controlled full runtime test proves policy preservation.
The live native watcher was not exercised. No compiler fix or live source change was authorized by this validation request.

Artifacts: `/home/pi/.local/state/libreqos-native-check-s12invb1` (private subscriber data, mode `0700`).
Session: [[2026-11-02-native-topology-source-check]].
Existing uncommitted edits in `src/files.rs` and `src/runner.rs` were present at task start and remain untouched.

## 2026-11-02T13:04:36+08:00 (Asia/Shanghai) — live LibreQoS v3 cutover verified

| Item | Value |
|------|-------|
| Goal | Complete the LibreQoS v3 live cutover |
| Last agent | Codex |
| Branch and revision | `lqosync3`, `55c4852` |
| Worktree | `/home/pi/LQoSync3.0` |
| Repository changes | Home, this handoff, preparation note, and final cutover note. Preserved earlier uncommitted notes. No application code changes. |
| Runtime | All three packages at `3.0.7.202610310038`. Five services active and enabled. LQoSync apply/auto detects v3_pair. Full topology. |
| Checks | Fresh topology ready. Healthy seam with matching applied state. No sync errors or circuit removals. Traffic in both directions. No recovery timers remain. |
| Network | Persistent independent dataplane ports, no `br0`. Management on `enp3s0`. Persistent mlx5 `txq_mem_algn=0` override and hugepages. |
| UI limitation | Network editor expects Linux `br0` and reports Conflict. Do not apply its managed preview or Take Over action to clear the badge. |
| Backup | `/root/libreqos-before-v3-RRb5MEle`; sync-stage rollback files in `sync-apply-FLarX9AD` below it |
| Remaining | Operator package hold command, optional planned reboot test, and upstream netplan-editor compatibility review |
| Next action | Operator: `sudo apt-mark hold libreqos libreqos-api libreqos-dataplane`. The upgrade cleared the previous hold. Do not reboot merely to clear the UI badge. |

Session: [[2026-11-02-libreqos-v3-live-cutover]].
The operational cutover is complete. Boot recovery and actual v2 rollback remain untested.

## 2026-11-02T12:32:10+08:00 (Asia/Shanghai) — v3 installed, dataplane startup failed

| Item | Value |
|------|-------|
| Goal | Complete the user-authorized LibreQoS v3 cutover |
| Last agent | Codex |
| Branch and revision | `lqosync3`, `55c4852` |
| Worktree | `/home/pi/LQoSync3.0` |
| Changed files | This handoff, Home, and the v3 preparation session note |
| Operator changes | Runtime archive, matched v3 package installation, automatic config migration, persistent 3072 hugepages |
| Checks | Resumed DPDK active. LQoSync shadow passed: v3_pair, no circuit changes, two expected node type changes. LQoSync service inactive. |
| Fixed failure | `txq_mem_algn=0` bypassed mlx5 shared Tx umem registration failure. The compatibility override is installed. |
| Network | Resumed persistence succeeded. `br0` absent. Management remains on `enp3s0`. Recovery timer deadline: 2026-11-02T13:04:08+08:00. |
| Next action | After client traffic confirmation, operator runs `/home/pi/.local/state/libreqos-v3-cutover/apply-lqosync-v3.sh` with sudo. It backs up configs/pair, selects full topology, applies once, verifies fresh topology and matching seam, and starts LQoSync. It cancels the network timer after config validation. |
| Access | Agent sudo requires a password. Operator executes root commands in their terminal. |

Session: [[2026-11-02-v3-upgrade-preparation]].
Backup: `/root/libreqos-before-v3-RRb5MEle`.
Rollback artifact: `/home/pi/libreqos-v2-rollback/libreqos_2.2.202609081854_amd64.deb`.
The compatibility override is installed. Successful v3 startup and an applied generation are confirmed.
Root inspection reports managed `br0`, `can_adopt=false`. Its preview retains the kernel bridge.
The operator ran the trial and confirmed client internet. Verify that the trial timer was actually stopped.
The operator ran the persistence script successfully. The persistent recovery timer was still active at inspection.
The reported `BRIDGE_SEAM_REPLACED` warning recovered: healthy seam, matching applied content key, no reject or divergence, and traffic counters in both directions.
Later inspection at 2026-11-02T12:47:19+08:00 confirms timer recovery restored `br0` and stopped DPDK.
The shadow block exited at its first guard. Live LQoSync config remains `apply` / `v2_pair` and the service remains stopped.
The revised persistence and combined resume scripts passed syntax and guard checks but have not run as root.
The operator subsequently ran resume successfully. Current config: shadow/auto, detected v3_pair, expected two-node diff only.
The next activation script is prepared and checked but has not executed as root.

## 2026-11-02T12:15:41+08:00 (Asia/Shanghai) — v3 upgrade preparation resumed

| Item | Value |
|------|-------|
| Goal | Resume the LibreQoS v3 preparation from the inaccessible chat |
| Last agent | Codex |
| Branch and revision | `lqosync3`, `55c4852` |
| Worktree | `/home/pi/LQoSync3.0` |
| Changed files | This handoff, `brain/Home.md`, and the session note below |
| Checks | Read-only package, service, route, bridge, firmware, and memory checks passed. No application code changed or tests ran. |
| Current state | LibreQoS v2.2 remains held. The three existing services remain active. No runtime changes. |
| Unfinished work | Root netplan inspection, verified v2 rollback package, protected backups, firmware compatibility, hugepages, and exact cutover commands |
| Next action | User requested cutover. Recover existing root backup and cutover paths through the operator, then continue the existing procedure. Root authentication remains unavailable to this agent. |

Session: [[2026-11-02-v3-upgrade-preparation]].
The wider search found a v2 archive rollback package and two staged v3 packages.
Their hashes match local APT metadata. APT simulation and rollback dry-run passed.
The v2 artifact is `2.2.202609081854`, not the installed `2.2.202608181454`.
The operator found `/root/libreqos-before-v3-RRb5MEle` and `/root/libreqos-before-root-preflight-0EN8CBr9`.
The file list confirms `configs.tar.gz`, a manifest, package snapshots, holds, and network snapshots.
The operator supplied the archive list and root preflight: `WARN`, no `BLOCK`, valid legacy pair, `br0` adoption required.
Next: stop old services, back up runtime state, and install the matched `3.0.7.202610310038` set through the operator terminal.
Keep LQoSync stopped until the v3 target, topology mode, bridge adoption, and runtime acceptance pass.
Sudo authentication in the operator terminal does not reach the agent session.
Preserved the existing Home edits and three untracked session notes from 2026-11-01.


## 2026-11-01T09:40:00+00:00 — first release v3.0.122

| Item | Value |
|------|-------|
| Goal | Version number, CI, release workflow, installer, release guide |
| Last agent | Claude Code (Opus 5.5), with Sonnet reviewers |
| Version | `3.0.<commits>+g<hash>` from `build.rs`. Refer to `docs/RELEASE.md`. |
| Release | `v3.0.122` on GitHub, published by `.github/workflows/release.yml`. The tag `v3.0.121` has no release (a host-dependent test failed in CI). |
| Shaper | The operator installed `v3.0.122` with `install.sh`. `status.json` shows `3.0.122+gd889101`, `apply`, `v2_pair`, router online. Systemd I/O and IP accounting are on. |
| Checks | CI green (test and msrv jobs). Reviews PASS. |
| Rules | Do not force-push or rebase `lqosync3`. Make a release only from a commit with a green CI. |
| Next action | Optional: router TLS with a CA (`docs/OPERATIONS.md`), delete the tag `v3.0.121`. |

## 2026-11-01T08:26:04+00:00 — btop TUI, review fixes, new LibreQoS installation

| Item | Value |
|------|-------|
| Goal | Make the TUI like btop++, fix the review findings, and support a new LibreQoS installation |
| Last agent | Claude Code (Opus 5.5), with Sonnet reviewers |
| Branch and source revision | `lqosync3`, `fa7ffd9` before the brain notes |
| Worktree | `/home/pi/LQoSync3.0` |
| Changed files | `src/tui/*`, `src/runner.rs`, `src/cli.rs`, `src/config.rs`, `config/defaults.toml`, docs, specs, brain |
| Checks | 437 library tests and all integration tests passed. Clippy and fmt passed. Final reviews: PASS. |
| Deploy | The operator installed the release build and restarted the service. |
| Unfinished work | The approved process chart (CPU, RSS, I/O, network). Real-terminal review of the TUI. |
| Next action | Build the process chart. I/O and network need `IOAccounting=yes` and `IPAccounting=yes` on the service. |

Session: [[2026-11-01-btop-tui-and-fresh-install]].
Decision: [[ADR-0010-first-full-libreqos-run]].

## 2026-11-01T12:45:09+08:00 (Asia/Shanghai) — TUI cosmetics implementation

| Item | Value |
|------|-------|
| Goal | Implement the approved cosmetics plan for all A1–A5 and B6–B10 items |
| Last agent | Codex, with one fresh read-only reviewer |
| Branch and source revision | `lqosync3`, `7ca02c9` |
| Worktree | `/home/pi/LQoSync3.0` |
| Changed files | Theme config/defaults/docs, shared palette, shell, all five view modules, metadata, tests, and these notes |
| Checks | 430 tests passed, 2 intentionally ignored. Clippy, fmt, release timing, debug build, and PTY smoke passed. |
| Review | Three compact-layout findings and one popup canvas finding fixed with regression tests. No deferred code findings. |
| Unfinished work | Human inspection of palette contrast and glyphs on the operator terminal |
| Next action | Review the TUI in the operator terminal. Deploy only when the operator requests it. |

Spec: `docs/superpowers/specs/2026-11-01-lqosync3-tui-cosmetics-design.md`.
Plan: `docs/superpowers/plans/2026-11-01-lqosync3-tui-cosmetics.md`.
Session: [[2026-11-01-tui-cosmetics-implementation]].
No production file, service state, dependency, or status schema changed.
The project graph remains at the earlier baseline.

## 2026-11-01T12:03:25+08:00 (Asia/Shanghai) — TUI cosmetics plan

| Item | Value |
|------|-------|
| Goal | Prepare the implementation plan, review it against the spec, then commit and push the documents |
| Last agent | Codex |
| Branch and baseline | `lqosync3`, `cf2c959` before the documentation commit |
| Worktree | `/home/pi/LQoSync3.0` |
| Changed files | Cosmetics spec and plan, this handoff, `brain/Home.md`, and the cosmetics session note |
| Review | PASS for plan coverage. All A1–A5 and B6–B10 requirements have implementation and test owners. |
| Checks | Fresh TUI baseline: 160 passed, 0 failed, 1 ignored. Documentation structure, links, and diff checks. |
| Unfinished work | Source implementation, full implementation gates, and real-terminal checks |
| Next action | Read the spec and plan, then implement after the operator authorizes source changes |

Plan: `docs/superpowers/plans/2026-11-01-lqosync3-tui-cosmetics.md`.
The operator authorized the documentation commit and push after a successful plan review.
No source changes or deployment form part of this step.
The commit that contains this entry identifies the published documentation revision.

## 2026-11-01T11:54:22+08:00 (Asia/Shanghai) — TUI cosmetics design

| Item | Value |
|------|-------|
| Goal | Complete the cosmetics design spec for all A1–A5 and B6–B10 items |
| Last agent | Codex |
| Branch and baseline | `lqosync3`, `cf2c959` |
| Worktree | `/home/pi/LQoSync3.0` |
| State at takeover | Clean working tree. No cosmetics spec or implementation changes. Claude Code remained open, without a visible build or edit subprocess. |
| Changed files | New cosmetics spec, this handoff, `brain/Home.md`, and the linked session note |
| Checks | TUI baseline: 160 passed, 0 failed, 1 ignored. STE heuristic review and manual requirement review. |
| Unfinished work | Review and implementation of the design. Full build gates and terminal visual checks belong to implementation. |
| Next action | Read the spec and confirm current ownership before implementation edits. Preserve existing keys and the apply flow. |

Spec: `docs/superpowers/specs/2026-11-01-lqosync3-tui-cosmetics-design.md`.
Session: [[2026-11-01-tui-cosmetics-design-handoff]].
No source code or production state changed during this takeover.
The older sections below contain historical context. Verify their claims against current files.

## 2026-10-31 (Asia/Shanghai) — documentation phase

| Item | Value |
|------|-------|
| Goal | Rust replacement for `updatecsv.py` with TOML settings, for LibreQoS 2.2 and 3.x |
| Last agent | Claude Code (Opus 5.5) |
| Directory | `/home/pi/LQoSync3.0` (repository root) |
| Repository | `https://github.com/p33ckab00/LQoSync.git`, branch `lqosync3` (no shared history). The crate, `docs/`, and `brain/` are at the root. |
| Backup | `/home/pi/LQoSync3.0-backup-2026-10-31.tar.gz` (the files before the first move) |
| Phase | Plan 1 (core) and Plan 2 (runtime) done. The binary has `check`, `once`, `run`, `diff`, `defaults`. Plans 2b, 3, 4, 5 are not written. |
| Checks | `cargo test` (88 + 1 + 1), `cargo clippy --all-targets -D warnings`, `cargo fmt --check`. `tools/v3sim/check.sh`: the v3 3.0.7 compiler accepts all nine scenario outputs. |

### Files

- `README.md`
- `docs/ARCHITECTURE.md`, `docs/CONFIGURATION.md`, `docs/INSTALLATION.md`, `docs/OPERATIONS.md`, `docs/LIBREQOS-V3-INGRESS.md`
- `brain/` vault: [[ADR-0001-one-model-many-writers]], [[ADR-0002-v3-target-selection]], [[libreqos-v3-ingress-facts]], [[v3-offline-simulation-method]]
- `graphify-out/`: knowledge graph of the documents and the vault

### Decisions that the operator made

- [[ADR-0003-router-failed-vs-offline]]: a failed router keeps its subscribers. An offline router loses them after `offline_after` (default 300 seconds, confirmed by the operator).
- [[ADR-0004-toml-policy-python-defaults]]: all policy is in TOML. The defaults give the `updatecsv.py` output.
- Name: branch, crate, binary, and service are `lqosync3`. Config: `/etc/lqosync3/lqosync3.toml`. State: `/var/lib/lqosync3`.
- PPPoE speed order: comments first, then `profile_rate_limit`, then `profile_name`, then `default`.
- [[ADR-0005-topology-follows-v3]]: the nodes follow the v3 rules: stable node `id`, `Parent Node ID`, 14/15 CSV columns, max >= 0.2, `flat_promotion` parent fix. Open for a v3 test shaper: `compile_mode` effect and empty Parent Node.
- [[ADR-0006-v3-guards-and-node-types]]: `no_parent` with a v3 target fails `check`. A `compile_mode` other than `full` gives a warning. On v3, group nodes are type `site`.
- [[ADR-0007-fresh-design-not-a-port]]: a new design. `updatecsv.py` is a guide only. No byte parity. Scenario tests in place of golden tests.
- Speed rules: default `rate_limit_order = "routeros_rx_tx"`, `unitless_unit = "bps"` (RouterOS-correct). The operator selected this.
- Removal guard on: hold a cycle that removes more than 20% or more than 50 rows.
- [[ADR-0008-terminal-ui]]: `lqosync3 tui` with monitor, actions, and a config editor (Plan 4).

### Decisions that the operator must make

None at this time.

### Next action

1. Cutover done on 2026-10-31 15:36 UTC: `lqosync3.service` operates in `apply` mode and writes the live LibreQoS files. `lqosync-updatecsv.service` (Python) is stopped and disabled. Backup of the last Python files: `/var/backups/lqosync/20261031T153517Z`. Rollback: OPERATIONS.md, "Rollback to the Python service".
2. Plan 2b (Telegram) is done and operates on the shaper: `[telegram] enabled = true`, chat `810741035`, token in `/etc/lqosync3/secrets/telegram.token`. The test message arrived at 15:24 UTC.
3. Plan 3 (LibreQoS v3) is done in code: `src/detect.rs`, `src/writer/v3network.rs`, `lqosync3 detect`. The production shaper (2.2, fixed `target = "v2_pair"`) does not use it yet. The `v3_network` output passed the offline 3.0.7 parser (spec "Offline check result"). Open: a test on an operating v3 shaper, the `lqosd` accept check (`v3_status_timeout`), `libreqos-v3-preflight`.
4. Plan 4a (TUI, read-only screens and actions) is done in code: `lqosync3 tui`, `src/tui/`. The service writes `status.json` version 3 and `snapshot.json`. The installed binary on the shaper is still the Plan 3 build: deploy the new build to get the new status fields. Plan: `docs/superpowers/plans/2026-10-31-lqosync3-tui-4a.md`.
5. Plan 4b (Settings editor) is done in code: `src/tui/settings/` (meta, draft, validate, secrets, preview, apply, view). The first real apply is on the shaper by the operator. Plan: `docs/superpowers/plans/2026-11-01-lqosync3-tui-4b.md`.
6. Plan 4c (DHCP client overrides) is done in code: `[[routers.dhcp.clients]]` by MAC, `routers.dhcp.shape_unassigned`, the shared lease list `sources::dhcp_leases`, and the DHCP view of the Settings tab. Spec: `docs/superpowers/specs/2026-11-01-lqosync3-dhcp-clients-design.md`.
7. Next: the minors A and B of the owner list, then TUI cosmetics, then Plan 5 (installer).

Router library: `mikrotik-rs =0.9.0` (`mikrotik-tokio` 0.3.0) in this project only. The other project in `/opt/LQoSync3.0/v3` is not changed.

Plan 2 and defaults review minor items: fixed on 2026-10-31 (one test for each). TCP keepalive is not possible: `mikrotik-tokio` 0.3 makes the TCP socket and has no keepalive option. In its place, a kept session that fails gets one reconnect at once, and this reconnect does not use a retry.

Deferred minor items from the Plan 2b review (2026-10-31; the error memory and chunked answer items were fixed on 2026-11-01):
- `include_computation` was removed. An old config with this key does not load (`deny_unknown_fields`). The live config does not have it.

Deferred minor items from the Plan 3 review (2026-10-31):
- Rows of one Circuit ID with different parents or rates: `v3_network` uses the first row and gives no alert.
- A dpkg version with an epoch (`1:3.0.7`) is "not supported" (fails safe).
- A `v3_network` skip alert uses the Telegram title "removal guard held a write".
- `diff` when the detection fails: it compares the pair files.
- `v3_network` with `preserve_network_config = true` reads the old `network.json` each cycle, but does not write it.

Deferred minor items from the Plan 4a review (2026-10-31):
- Just after a service restart, the old `pid` in `status.json` gives a false "the service restarted" message for an action.
- The digest check compares with the `--config` text of the TUI. That file can be a different file from `config_path`, or empty if the TUI cannot read it.
- A `status.json` read or parse error shows no reason (all values show `unknown`).
- IPv4 sorts as text (`10.0.0.10` before `10.0.0.9`).
- The Network detail "circuits" counts device rows.
- The perf test does not time the name sort or the full `App::draw`.
- The Journal selection anchor moves when old lines drop from a full buffer.

Deferred minor items from the Plan 4b review (2026-11-01):
- Each draw of the Settings tab calls `text()` and `diff_lines()`: cache them on a change.
- `add_server` and `remove_server` change the draft before their index checks.
- `Secret` push and pop, and `SecretUndo`, can leave secret bytes in freed memory.

Deferred minor items from the Plan 4c review (2026-11-01):
- The clients group shows "active lease" also for a lease that is skipped (excluded IP, not bound).
- A router that is online with DHCP off, or a draft that does not parse, gives misleading texts in the clients group and the DHCP note.
- A real DHCP server named `unassigned` shares the node of the `unassigned` group.
- The warning "N DHCP leases have no server" is not in OPERATIONS.md.

Deferred minor items from the review of the minor fixes (2026-11-01):
- A chunked Telegram answer whose last chunk has an extension or trailers (`0;x=y`) is read only at the close (as before the fix).
- The Telegram error memory merges errors that differ only in digits or after the first line ("router 1 down" and "router 2 down").
- A TUI that is killed between the temporary secret file and the rename leaves a `.<name>.tmp-<pid>-<n>` file (mode 0600).

Settings: all defaults are in `config/defaults.toml` (embedded). Plan 5 must install a `/etc/lqosync3/lqosync3.toml` with `[[routers]]` and comments that point to the defaults file.

### Production state

- `lqosync-updatecsv.service` (Python) is stopped and disabled since 2026-10-31 15:35 UTC.
- `lqosync3.service` operates in `apply` mode since 2026-10-31 15:36 UTC. It writes `/opt/libreqos/src/ShapedDevices.csv` and `network.json`, and runs `LibreQoS.py --updateonly`. Config: `/etc/lqosync3/lqosync3.toml`. Secret: `/etc/lqosync3/secrets/FSEHub-Distro.password` (`0600` root). Refer to [[2026-10-31-first-live-shadow-run]].
- LibreQoS 2.2 operates. Nobody changed it.
