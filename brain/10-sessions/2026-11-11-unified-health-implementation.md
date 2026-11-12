---
type: session
created: 2026-11-11
tags: [health, telegram, implementation]
status: complete-local
related: ["[[2026-11-11-unified-health-telegram-research]]", "[[HANDOFF]]"]
---

# Unified health implementation

Updated: 2026-11-12T08:18:12+08:00 (Asia/Shanghai).
Branch/worktree: `lqosync3`, `/home/pi/LQoSync3.0`. Last agent: Codex `/root`, inline.
The owner approved implementation and continuation. All four plan tasks are complete locally.
Plan: `docs/superpowers/plans/2026-11-11-unified-health-telegram.md`.
No commit, push, deployment, production restart, or live Telegram test send occurred.

## Result

- Strict import comparison covers addresses, all rates, identity fields, explicit parents/anchors, comments, producer `f32` rates, and CSV trim behavior.
- Numeric representation equivalence cannot hide large-integer precision loss. Usernames remain case-sensitive.
- The shared health hub covers seven domains, freshness, source epochs, bridge uptime, degraded subsystems, and allocation evidence.
- Durable receipts track one write ID, input hashes, per-file outcomes, interrupted writes, superseding operations, and event recovery.
- A pending-only checker uses cached expected inputs and stable producer/native evidence. It does not start RouterOS collection or mint a world.
- One durable Telegram worker sends WRITTEN/PENDING and terminal reports. It preserves the original route, detail options, and event health snapshot.
- API receipts, HTML/Unicode bounds, transient retries, recipient pauses, restart recovery, and explicit queue gaps are implemented.
- Notification storage failure no longer stops service startup. The worker exposes the fault and retries independently.
- The TUI loads the bounded shared cache outside rendering. Expired observations cannot remain healthy when the producer stops.
- The approved Dashboard panel order and mirror chart remain intact. Mouse and keyboard scrolling remain available.
- Live reload cannot change `runtime.state_dir`; restart must transfer lock/receipt ownership.

## Native findings

The envelope schema is 1; the transaction schema is 6. The first probe incorrectly treated them as one schema.
Native operations can reorder. The private capture confirmed equivalent unique policy operations after normalization.
The final operator-run read-only policy check passed, including producer source validation and native generation correlation.
It observed peer PID 630703 and 26 imported devices on 2026-11-11.
The source/native generations matched. No managed file was written by the probe.

The native bridge reported 64 circuit slots; the generated artifact requested 1000.
The producer's policy content key excludes allocation capacity.
APPLIED therefore describes verified subscriber policy. Allocation mismatch remains a separate DEGRADED health component.
Missing capacity metadata remains UNKNOWN. No allocator or bridge setting was changed.
Do not describe the observed slot mismatch as an established traffic outage.

Private captures contain subscriber mappings and stay outside Git with mode `0600`.
Evidence directory: `/home/pi/.local/state/lqosync3/unified-health-task1/`.
Key files: `verified-current.json`, `final-tests.log`, `final-clippy.log`, and `smoke/results.json`.

## Final checks

- `cargo test --locked --all-targets`: 690 passed, zero failed, eight manual tests ignored, across 19 suites.
- All-target Clippy with `-D warnings`, formatting, debug binary build, and `git diff --check` passed.
- All seven routes passed actual terminal capture checks with isolated synthetic data.
- Dashboard capture: `docs/design/tui/captures/dashboard-unified-health-v1.png`.
- The live read-only import check still matched all 26 rows after the final numeric fix.
- The STE heuristic ran; technical terms and manual language review remain necessary. No certification claim is made.

## Performance evidence

Debug measurements exclude full RouterOS-cycle latency and import JSON parse time.
Reused import comparisons took 6.384/65.686/349.993 ms for 1k/10k/50k devices.
Repeated normalization took 12.579/128.278/699.330 ms for the same cases.
One thousand health snapshots for 87 components took 1.023 seconds and produced one cache write.
A 108,032-byte HTML report produced 24 bounded parts in 32.43 ms.
One hundred cached chart redraws took 1.507 seconds; the gapped-history case took 0.820 seconds.
Receipt-path measurements are recorded in `receipt-performance.jsonl`; they exclude CSV rendering and the full cycle.
These are development measurements, not production speedup claims.

## Handoff and limits

The worktree is ready for a separate commit/deployment request. Production cutover remains a later action.
The native capacity mismatch needs a separate producer/allocator investigation if the owner requests it.
Native state replies retain a two-MiB bound. Oversized or unsupported evidence remains unverified.
Permanent Telegram errors pause the event; operations documentation explains controlled recovery.
Exactly-once Telegram delivery is not guaranteed after an ambiguous API timeout.
Queue gaps remain visible through `write-event-gap.json` and require operator review.

Preserve unrelated `brain/30-knowledge/lqosync-owned-profiles-and-wan-policy.md` and its pre-existing Home link.
Do not stage the entire Home file without separating that unrelated link.
