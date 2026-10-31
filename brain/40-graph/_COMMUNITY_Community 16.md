---
type: community
members: 39
---

# Community 16

**Members:** 39 nodes

## Members
- [[ADR-0003 Router failed vs offline]] - rationale - brain/HANDOFF.md
- [[ADR-0009 DHCP discovery and exclusions]] - rationale - brain/Home.md
- [[Command reference (check, once, diff, run, defaults)]] - concept - docs/OPERATIONS.md
- [[Comment speed format (Name25M5M)]] - concept - docs/CONFIGURATION.md
- [[Configuration reference]] - document - docs/CONFIGURATION.md
- [[DHCP server discovery]] - concept - docs/CONFIGURATION.md
- [[Defaults layering over configdefaults.toml]] - rationale - docs/CONFIGURATION.md
- [[Event memory (no repeat, no retry, in-process)]] - rationale - docs/superpowers/specs/2026-10-31-lqosync3-telegram-design.md
- [[Exclusions (exclude, routers.exclude)]] - concept - docs/CONFIGURATION.md
- [[Global constraints (no token leak, failure never changes cycle)]] - rationale - docs/superpowers/plans/2026-10-31-lqosync3-telegram.md
- [[HTML message format and split]] - concept - docs/superpowers/specs/2026-10-31-lqosync3-telegram-design.md
- [[HTTPS delivery with webpki-roots TLS]] - concept - docs/superpowers/specs/2026-10-31-lqosync3-telegram-design.md
- [[Loader validation (unknown keys, ranges)]] - concept - docs/CONFIGURATION.md
- [[Map from config.json]] - concept - docs/CONFIGURATION.md
- [[Notification events (changes, error, cleared, router state, guard)]] - concept - docs/superpowers/specs/2026-10-31-lqosync3-telegram-design.md
- [[Out of scope (start message, retry, other channels)]] - rationale - docs/superpowers/specs/2026-10-31-lqosync3-telegram-design.md
- [[PPPoE speed sources order]] - concept - docs/CONFIGURATION.md
- [[Plan 2b deferred minor items]] - concept - brain/HANDOFF.md
- [[Plan 2b lqosync3 Telegram implementation]] - document - docs/superpowers/plans/2026-10-31-lqosync3-telegram.md
- [[Policy in TOML, formats in code]] - rationale - docs/CONFIGURATION.md
- [[Removal guard (safety)]] - rationale - docs/CONFIGURATION.md
- [[Review focus (hanging send, changing error text, HTML escape, long line split, empty env)]] - rationale - docs/superpowers/plans/2026-10-31-lqosync3-telegram.md
- [[Router state (onlinefailedunreachableoffline)]] - concept - docs/CONFIGURATION.md
- [[Router state classification (RouterResult, unreachable_since)]] - rationale - docs/superpowers/specs/2026-10-31-lqosync3-runtime-design.md
- [[Secret file rule (0600, root, _file_env)]] - rationale - docs/CONFIGURATION.md
- [[Task 1 Config keys]] - concept - docs/superpowers/plans/2026-10-31-lqosync3-telegram.md
- [[Task 4 Wire into run, once, check]] - concept - docs/superpowers/plans/2026-10-31-lqosync3-telegram.md
- [[Telegram config changes (api_host, api_port, drop include_computation)]] - concept - docs/superpowers/specs/2026-10-31-lqosync3-telegram-design.md
- [[Telegram notifier design (Plan 2b spec)]] - document - docs/superpowers/specs/2026-10-31-lqosync3-telegram-design.md
- [[Telegram settings]] - concept - docs/CONFIGURATION.md
- [[Token and chat ID sources (env first, then file)]] - rationale - docs/superpowers/specs/2026-10-31-lqosync3-telegram-design.md
- [[Turn on Telegram procedure]] - concept - docs/OPERATIONS.md
- [[cliparse  cliCommand]] - concept - docs/superpowers/plans/2026-10-31-lqosync3-runtime.md
- [[configdefaults.toml embedded defaults]] - concept - brain/HANDOFF.md
- [[lqosync3 import-json (planned)]] - concept - docs/CONFIGURATION.md
- [[rate_limit_order and unitless_unit]] - rationale - docs/CONFIGURATION.md
- [[routerstateclassify]] - concept - docs/superpowers/plans/2026-10-31-lqosync3-runtime.md
- [[status.json cycle status]] - concept - docs/OPERATIONS.md
- [[statusStatus  RouterStatus]] - concept - docs/superpowers/plans/2026-10-31-lqosync3-runtime.md

## Live Query (requires Dataview plugin)

```dataview
TABLE source_file, type FROM #community/Community_16
SORT file.name ASC
```

## Connections to other communities
- 8 edges to [[_COMMUNITY_Telegram Notifier]]
- 8 edges to [[_COMMUNITY_Session Notes]]
- 8 edges to [[_COMMUNITY_Community 36]]
- 7 edges to [[_COMMUNITY_Status and Diff 2]]
- 5 edges to [[_COMMUNITY_Community 18]]
- 4 edges to [[_COMMUNITY_Topology 3]]
- 2 edges to [[_COMMUNITY_Speed Rules 3]]
- 2 edges to [[_COMMUNITY_MikroTik Session 5]]
- 1 edge to [[_COMMUNITY_Topology 4]]

## Top bridge nodes
- [[Configuration reference]] - degree 31, connects to 6 communities
- [[Removal guard (safety)]] - degree 7, connects to 3 communities
- [[Telegram notifier design (Plan 2b spec)]] - degree 13, connects to 2 communities
- [[Plan 2b lqosync3 Telegram implementation]] - degree 8, connects to 2 communities
- [[Plan 2b deferred minor items]] - degree 5, connects to 2 communities