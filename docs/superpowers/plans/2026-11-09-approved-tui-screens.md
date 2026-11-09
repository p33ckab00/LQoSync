# Approved TUI screens implementation plan

Goal: Implement all approved screen bodies, with local commits after each screen passes review.
Owner authorization: proceed Dashboard through Journal, review performance/integrity, test, capture, commit locally. Do not push.

## Scope and files

Retain Ratatui and current service workflows. Match `docs/design/tui/README.md` and its canonical PNGs.
Use shared panel, focus, mouse-target, and scrollbar helpers only where screens need them.
Keep drawing free of I/O. Bound row work and history. Preserve missing/stale evidence and all Settings action gates.

## Ordered checkpoints

- [x] Dashboard: add failing rate/reset/gap and layout/input tests; extend `worker_util` actual counters/cache/history; add approved body and controls; capture and review; test/lint; local commit.
- [x] Subscribers: approved cards/filters/table/details/status; source evidence; mouse and keyboard; review/test/capture; local commit.
- [x] Routers: approved cards/filters/table/details/status and navigation; review/test/capture; local commit.
- [x] LibreQoS: approved target/compatibility/source previews with existing confirmation; review/test/capture; local commit.
- [ ] Workers: approved utilization/detail/pressure/status with source semantics; review/test/capture; local commit.
- [ ] Settings: approved router form and typed values/diff with existing draft/preview/apply/secret gates; review/test/capture; local commit.
- [ ] Journal: approved cards/filters/table/details/status, bounded retention, original-text copy and terminal selection; review/test/capture; local commit.

## Verification per checkpoint

Run focused tests after the failing test and implementation. Review callers, input ownership, source freshness, bounds, and failures.
Run the full suite, formatting, and all-target Clippy with warnings denied before commit.
Export actual rendered frames with controlled fixtures, capture as PNG, inspect beside the canonical mockup, and document limits.
Use isolated config for interactive smoke checks. Do not write production configuration or restart production services.
Preserve compact access at 80x24. Visible scrollbars must support wheel, track clicks, and thumb drag.
Record checks and unfinished work in the project brain. Continue to the next screen after the local checkpoint.
