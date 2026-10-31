---
type: community
members: 13
---

# Plan 2 Runtime

**Members:** 13 nodes

## Members
- [[Checks after installation]] - concept - docs/INSTALLATION.md
- [[CycleSummarynext_sleep]] - concept - docs/superpowers/plans/2026-10-31-lqosync3-runtime.md
- [[Installation (docs)]] - document - docs/INSTALLATION.md
- [[Installer options (--mode, --version, --from-source, --import-json, --no-start)]] - concept - docs/INSTALLATION.md
- [[Manual installation]] - concept - docs/INSTALLATION.md
- [[Removal procedure]] - concept - docs/INSTALLATION.md
- [[Requirements (Ubuntu 24.04, LibreQoS 2.0-2.2 or 3.x, Rust 1.89)]] - concept - docs/INSTALLATION.md
- [[Runtime modes (shadow, apply)]] - concept - docs/CONFIGURATION.md
- [[Signals (SIGHUP reload, USR1 cycle now)]] - concept - docs/OPERATIONS.md
- [[What the installer does (10 steps)]] - concept - docs/INSTALLATION.md
- [[install.sh one-line installer (target contract)]] - concept - docs/INSTALLATION.md
- [[lqosync3.service systemd unit]] - concept - docs/INSTALLATION.md
- [[main run loop (tokioselect! on sleep and signals)]] - concept - docs/superpowers/plans/2026-10-31-lqosync3-runtime.md

## Live Query (requires Dataview plugin)

```dataview
TABLE source_file, type FROM #community/Plan_2_Runtime
SORT file.name ASC
```

## Connections to other communities
- 2 edges to [[_COMMUNITY_Session Notes]]
- 2 edges to [[_COMMUNITY_MikroTik Session 4]]
- 1 edge to [[_COMMUNITY_Speed Rules]]
- 1 edge to [[_COMMUNITY_MikroTik Session 5]]
- 1 edge to [[_COMMUNITY_Telegram Notifier 3]]

## Top bridge nodes
- [[Runtime modes (shadow, apply)]] - degree 4, connects to 3 communities
- [[Signals (SIGHUP reload, USR1 cycle now)]] - degree 4, connects to 2 communities
- [[main run loop (tokioselect! on sleep and signals)]] - degree 4, connects to 1 community
- [[CycleSummarynext_sleep]] - degree 2, connects to 1 community