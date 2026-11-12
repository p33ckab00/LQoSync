---
type: session
created: 2026-11-12
tags: [health, telegram, deployment]
status: complete
related: ["[[2026-11-11-unified-health-implementation]]", "[[HANDOFF]]"]
---

# Unified health deployment

Verified: 2026-11-12T08:33:16+08:00 (Asia/Shanghai). Last agent: Codex `/root`.
The owner requested commit, push, and deployment, then ran the prepared root script.

## Release

- Branch: `lqosync3`. Feature commit: `12e3d579b055fd5c42d6757ad4100954d9e7b0b3`.
- Installed version: `3.0.179+g12e3d57`. Running PID: 882084.
- Clean release artifact: `/home/pi/.local/state/lqosync3/health-12e3d57/lqosync3`.
- GitHub CI passed both test and Rust 1.89 jobs: https://github.com/p33ckab00/LQoSync/actions/runs/43127174714.
- Local checks: 690 passed, zero failed, eight manual tests ignored. Clippy, formatting, and release build passed.
- Configuration hash is unchanged. Automatic restarts: 0.
- Backup: `/var/backups/lqosync3/health-12e3d57-146zv46b`. It contains the previous binary, configuration, and state copy.

## Deployment verification

The root script installed the binary atomically and restarted only `lqosync3.service`.
It verified a successful first cycle, RouterOS collection counts, all six telemetry sources, and native policy evidence.
Independent verification matched the installed hash and active PID, then confirmed advancing telemetry and shared-health timestamps.
The installed terminal smoke passed all seven routes, exited normally, and restored the terminal.
Shared health reports `HEALTHY`. The notification worker reports `HEALTHY`.

The observed native allocation remains 64 circuit slots versus 1000 generated slots.
This is independent from the verified subscriber policy. No bridge setting or allocator changed.
The startup cycle had `files_changed = false` and reload result `no change`.
No new managed-file operation was created, so no write-triggered Telegram report was required during this verification.
No synthetic Telegram message or forced shaping-file change was used.
The delivery worker and native policy source were verified; a real two-stage API delivery remains observable on the next actual file update.

## Evidence

Evidence stays in `/home/pi/.local/state/lqosync3/health-12e3d57/`.
Key files: `DEPLOYED.json`, `manifest.json`, `independent-verification.json`, `installed-tui-smoke.json`, and `deployed-policy.json`.
Private native captures remain outside Git.

## Remaining work

No release task remains. Investigate native allocation separately if the owner requests it.
Preserve the unrelated local-profile Knowledge note and its Home link.
