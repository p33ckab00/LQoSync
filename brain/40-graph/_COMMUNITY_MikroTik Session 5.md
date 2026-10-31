---
type: community
members: 18
---

# MikroTik Session 5

**Members:** 18 nodes

## Members
- [[Atomic write (temp file, sync_all, rename, fsync folder; only on byte change)]] - concept - docs/superpowers/specs/2026-10-31-lqosync3-runtime-design.md
- [[Files lock, state.json, status.json, candidate, applied]] - concept - docs/superpowers/specs/2026-10-31-lqosync3-runtime-design.md
- [[First-run seed (seed_state, seed_nodes from live files)]] - concept - docs/superpowers/specs/2026-10-31-lqosync3-runtime-design.md
- [[Instance lock statelqosync3.lock (Filetry_lock)]] - concept - docs/superpowers/specs/2026-10-31-lqosync3-runtime-design.md
- [[LibreQoS 2.x reload (v2_command v2_args, v2_timeout)]] - concept - docs/superpowers/specs/2026-10-31-lqosync3-runtime-design.md
- [[Plan 2 scope (in  not in)]] - concept - docs/superpowers/specs/2026-10-31-lqosync3-runtime-design.md
- [[Process run loop, SIGUSR1, SIGHUP, SIGTERMSIGINT, once, check, diff, defaults]] - concept - docs/superpowers/specs/2026-10-31-lqosync3-runtime-design.md
- [[Router paths and .proplist fields table]] - concept - docs/superpowers/specs/2026-10-31-lqosync3-runtime-design.md
- [[Router reads (print only, timeouts, retries, concurrent sessions)]] - rationale - docs/superpowers/specs/2026-10-31-lqosync3-runtime-design.md
- [[Secrets (password_file read each connect, never logged)]] - concept - docs/superpowers/specs/2026-10-31-lqosync3-runtime-design.md
- [[TLS (tls_verify, tls_ca_file, webpki-roots)]] - concept - docs/superpowers/specs/2026-10-31-lqosync3-runtime-design.md
- [[Test strategy (fake session, mock RouterOS server, temp folders)]] - concept - docs/superpowers/specs/2026-10-31-lqosync3-runtime-design.md
- [[Write order state.json first, then LibreQoS pair]] - rationale - docs/superpowers/specs/2026-10-31-lqosync3-runtime-design.md
- [[lqosync3 runtime design (Plan 2)]] - document - docs/superpowers/specs/2026-10-31-lqosync3-runtime-design.md
- [[mikrotik-rs =0.9.0 (mikrotik-tokio 0.3.0)]] - rationale - docs/superpowers/specs/2026-10-31-lqosync3-runtime-design.md
- [[mikrotik-tokio 0.1.0 16-reply burst cancel bug]] - concept - docs/superpowers/specs/2026-10-31-lqosync3-runtime-design.md
- [[rust-version 1.89 (stdfsFiletry_lock)]] - rationale - docs/superpowers/specs/2026-10-31-lqosync3-runtime-design.md
- [[status.json status file]] - concept - docs/superpowers/specs/2026-10-31-lqosync3-runtime-design.md

## Live Query (requires Dataview plugin)

```dataview
TABLE source_file, type FROM #community/MikroTik_Session_5
SORT file.name ASC
```

## Connections to other communities
- 1 edge to [[_COMMUNITY_Topology 3]]
- 1 edge to [[_COMMUNITY_Plan 2 Runtime]]
- 1 edge to [[_COMMUNITY_Router State 3]]
- 1 edge to [[_COMMUNITY_Speed Rules 3]]

## Top bridge nodes
- [[lqosync3 runtime design (Plan 2)]] - degree 18, connects to 2 communities
- [[Files lock, state.json, status.json, candidate, applied]] - degree 7, connects to 1 community
- [[Router reads (print only, timeouts, retries, concurrent sessions)]] - degree 3, connects to 1 community