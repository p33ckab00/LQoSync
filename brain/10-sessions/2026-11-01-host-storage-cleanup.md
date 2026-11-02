---
type: session
created: 2026-11-01
tags: [storage, maintenance]
status: complete
related: ["[[HANDOFF]]"]
---

# Host storage cleanup

Time: 2026-11-01T16:46:17+08:00 (Asia/Shanghai).

The user requested a storage check and removal of unnecessary build caches.
The cleanup recovered 27,711,209,472 bytes (25.8 GiB).
The root filesystem has 73 GiB available. Use decreased from 58% to 34%.

## Removed files

- `/opt/LQoSync3.0/v3/target`: old build output, 16.1 GiB.
- `/home/pi/LQoSync3.0/target/debug/incremental`: incremental build cache, 4.1 GiB.
- `/home/pi/.cache/pip`: package download cache, 3.2 GiB.
- Three inactive VS Code server versions and the extension download cache: 2.4 GiB.

## Checks and retained files

The process check found no open files or memory maps in the removed directories.
Git status remained identical in both LQoSync repositories and both TI repositories after cache removal.
The current LQoSync release binary and dependency build output remain available.
The active VS Code server remains available.

TI project directories remain at `/home/pi/ti`, `/home/pi/ti-live`, and `/home/pi/ti-legacy`.
The current TI repository has uncommitted work. The legacy repository has an untracked packet capture.
Git lists no other linked worktrees for either TI repository.
The legacy repository also contains three submodules.

The accessible `/tmp` contents use 15 MiB and include active Claude files.
No user-owned regular file in `/tmp` was older than seven days.
The `/var/tmp` contents include TI diagnostic traces. Those files remain available as investigation evidence.
The check could not read the private system service directories in either temporary directory.

The detailed removal log is `/home/pi/.local/state/storage-cleanup-2026-11-01.json`.
Future builds and package downloads can recreate the removed caches.
