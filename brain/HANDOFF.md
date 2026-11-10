---
type: handoff
created: 2026-10-31
tags: [handoff]
status: active
related: ["[[2026-10-31-project-setup-and-v3-simulation]]"]
---

# Handoff

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
