---
source_file: "docs/superpowers/specs/2026-10-31-lqosync3-runtime-design.md"
type: "document"
community: "MikroTik Session 4"
tags:
  - graphify/document
  - graphify/EXTRACTED
  - community/MikroTik_Session_4
---

# lqosync3 runtime design (Plan 2)

## Connections
- [[Atomic write (temp file, sync_all, rename, fsync folder; only on byte change)]] - `references` [EXTRACTED]
- [[Files lock, state.json, status.json, candidate, applied]] - `references` [EXTRACTED]
- [[First-run seed (seed_state, seed_nodes from live files)]] - `references` [EXTRACTED]
- [[Instance lock statelqosync3.lock (Filetry_lock)]] - `references` [EXTRACTED]
- [[LibreQoS 2.x reload (v2_command v2_args, v2_timeout)]] - `references` [EXTRACTED]
- [[Plan 2 scope (in  not in)]] - `references` [EXTRACTED]
- [[Process run loop, SIGUSR1, SIGHUP, SIGTERMSIGINT, once, check, diff, defaults]] - `references` [EXTRACTED]
- [[Router paths and .proplist fields table]] - `references` [EXTRACTED]
- [[Router reads (print only, timeouts, retries, concurrent sessions)]] - `references` [EXTRACTED]
- [[Router state classification (RouterResult, unreachable_since)]] - `references` [EXTRACTED]
- [[Secrets (password_file read each connect, never logged)]] - `references` [EXTRACTED]
- [[TLS (tls_verify, tls_ca_file, webpki-roots)]] - `references` [EXTRACTED]
- [[Test strategy (fake session, mock RouterOS server, temp folders)]] - `references` [EXTRACTED]
- [[Write order state.json first, then LibreQoS pair]] - `references` [EXTRACTED]
- [[lqosync3 core design spec]] - `references` [EXTRACTED]
- [[mikrotik-rs =0.9.0 (mikrotik-tokio 0.3.0)]] - `references` [EXTRACTED]
- [[rust-version 1.89 (stdfsFiletry_lock)]] - `references` [EXTRACTED]
- [[status.json status file]] - `references` [EXTRACTED]

#graphify/document #graphify/EXTRACTED #community/MikroTik_Session_4