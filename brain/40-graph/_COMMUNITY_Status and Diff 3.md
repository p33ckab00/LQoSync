---
type: community
members: 13
---

# Status and Diff 3

**Members:** 13 nodes

## Members
- [[Backups that the tool keeps (last 10 in varliblqosync3applied)]] - concept - docs/OPERATIONS.md
- [[Command reference (check, once, once --force, diff, run, defaults)]] - concept - docs/OPERATIONS.md
- [[Daily tasks (systemctl status, journalctl)]] - concept - docs/OPERATIONS.md
- [[Operational rules (one apply tool per shaper, start in shadow)]] - rationale - docs/OPERATIONS.md
- [[Operations]] - document - docs/OPERATIONS.md
- [[Plan 3 LibreQoS v3 detection and writers]] - concept - docs/superpowers/plans/2026-10-31-lqosync3-runtime.md
- [[Process run loop, SIGUSR1, SIGHUP, SIGTERMSIGINT, once, check, diff, defaults]] - concept - docs/superpowers/specs/2026-10-31-lqosync3-runtime-design.md
- [[Signals SIGHUP reload, SIGUSR1 cycle now]] - concept - docs/OPERATIONS.md
- [[cliCommand (check, once --force, run, diff, defaults)]] - concept - docs/superpowers/plans/2026-10-31-lqosync3-runtime.md
- [[diffdiff_pair (semantic pair diff)]] - code - docs/superpowers/plans/2026-10-31-lqosync3-runtime.md
- [[filesbackup with runtime.keep_backups = 10]] - code - docs/superpowers/plans/2026-10-31-lqosync3-runtime.md
- [[lqosync3 detect (Plan 3)]] - concept - docs/OPERATIONS.md
- [[lqosync3 once --dry-run]] - concept - docs/OPERATIONS.md

## Live Query (requires Dataview plugin)

```dataview
TABLE source_file, type FROM #community/Status_and_Diff_3
SORT file.name ASC
```

## Connections to other communities
- 6 edges to [[_COMMUNITY_Router Reads 2]]
- 3 edges to [[_COMMUNITY_Config 2]]
- 3 edges to [[_COMMUNITY_CLI and Checks 2]]
- 2 edges to [[_COMMUNITY_MikroTik Session 4]]
- 1 edge to [[_COMMUNITY_Topology 3]]
- 1 edge to [[_COMMUNITY_Topology 4]]
- 1 edge to [[_COMMUNITY_MikroTik Session 3]]
- 1 edge to [[_COMMUNITY_MikroTik Session 5]]

## Top bridge nodes
- [[Operations]] - degree 16, connects to 6 communities
- [[Process run loop, SIGUSR1, SIGHUP, SIGTERMSIGINT, once, check, diff, defaults]] - degree 4, connects to 2 communities
- [[Plan 3 LibreQoS v3 detection and writers]] - degree 3, connects to 2 communities
- [[cliCommand (check, once --force, run, diff, defaults)]] - degree 5, connects to 1 community
- [[filesbackup with runtime.keep_backups = 10]] - degree 3, connects to 1 community