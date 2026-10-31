---
source_file: "docs/superpowers/plans/2026-10-31-lqosync3-runtime.md"
type: "document"
community: "Router Reads 2"
tags:
  - graphify/document
  - graphify/EXTRACTED
  - community/Router_Reads_2
---

# lqosync3 Runtime Implementation Plan (Plan 2)

## Connections
- [[ADR-0003 Router failed vs offline]] - `cites` [EXTRACTED]
- [[ADR-0008 Terminal UI]] - `cites` [EXTRACTED]
- [[ADR-0009 DHCP discovery and exclusions]] - `cites` [EXTRACTED]
- [[Configuration]] - `references` [EXTRACTED]
- [[Connector trait (connect - Session)]] - `references` [EXTRACTED]
- [[CycleSummarynext_sleep (error_retry  active  idle)]] - `references` [EXTRACTED]
- [[Global constraints (print-only, no secrets in logs, defaults in TOML)]] - `references` [EXTRACTED]
- [[MikrotikConnector  MikrotikSession  tls_client_config]] - `references` [EXTRACTED]
- [[Mock RouterOS server tests (testsmock_router.rs)]] - `references` [EXTRACTED]
- [[Plan 2b Telegram]] - `references` [EXTRACTED]
- [[Plan 3 LibreQoS v3 detection and writers]] - `references` [EXTRACTED]
- [[Plan 4 TUI]] - `references` [EXTRACTED]
- [[Plan 5 installer]] - `references` [EXTRACTED]
- [[Project phase Plan 1 core and Plan 2 runtime done]] - `references` [INFERRED]
- [[Review Focus (silent router, two processes, crash between writes, bad network.json, SIGHUP bad config)]] - `references` [EXTRACTED]
- [[RouterCfg tls_verify  tls_ca_file]] - `references` [EXTRACTED]
- [[RouterSession trait (print path proplist)]] - `references` [EXTRACTED]
- [[Runnercycle (lock, read, classify, assemble, write, reload, status)]] - `references` [EXTRACTED]
- [[TABLES const (6 paths with .proplist)]] - `references` [EXTRACTED]
- [[Task 9 Step 5 manual shadow read (operator approval)]] - `references` [EXTRACTED]
- [[cliCommand (check, once --force, run, diff, defaults)]] - `references` [EXTRACTED]
- [[diffdiff_pair (semantic pair diff)]] - `references` [EXTRACTED]
- [[filesbackup with runtime.keep_backups = 10]] - `references` [EXTRACTED]
- [[filesload_state (state.json or seed)]] - `references` [EXTRACTED]
- [[fileslock (try_lock, 'another lqosync3 is running')]] - `references` [EXTRACTED]
- [[filesread_live  Live (static rows, preserve network)]] - `references` [EXTRACTED]
- [[filesrun_reload (kill on timeout, last 20 stderr lines; pending retry)]] - `references` [EXTRACTED]
- [[fileswrite_if_changed (atomic write)]] - `references` [EXTRACTED]
- [[lqosync3 Core Implementation Plan (Plan 1)]] - `conceptually_related_to` [INFERRED]
- [[lqosync3 core design spec]] - `references` [EXTRACTED]
- [[lqosync3 runtime design (Plan 2)]] - `implements` [EXTRACTED]
- [[read_snapshot  read_router (timeouts, retries, session reuse)]] - `references` [EXTRACTED]
- [[routerosReadError (Unreachable, Login, Tls, Timeout, Trap, Protocol)]] - `references` [EXTRACTED]
- [[routerstateclassify]] - `references` [EXTRACTED]
- [[run loop with tokioselect! on SIGUSR1, SIGHUP, SIGTERMSIGINT]] - `references` [EXTRACTED]
- [[secretsread_secret (mode & 0o077 == 0, owner uid)]] - `references` [EXTRACTED]
- [[statusStatus  RouterStatus]] - `references` [EXTRACTED]
- [[systemdlqosync3.service]] - `references` [EXTRACTED]

#graphify/document #graphify/EXTRACTED #community/Router_Reads_2