---
type: decision
created: 2026-10-31
tags: [adr, collector, router-state]
status: accepted
supersedes:
related: ["[[ADR-0001-one-model-many-writers]]", "[[updatecsv-py-dissection]]", "[[2026-10-31-updatecsv-dissection]]"]
---

# ADR-0003 — A failed router keeps its subscribers, an offline router loses them

## Context

`updatecsv.py` removes all subscribers that it did not see in the cycle, if one router succeeded.
Thus a router with a read error loses all its subscribers. A read error also returns an empty list, so it looks like zero users.
The operator wants two different results: a failed router keeps its subscribers, and an offline router loses them.

## Decision

The collector sets one state for each router in each cycle:

1. `online`: all reads passed. The tool uses the new data.
2. `failed`: the connection passed, but a read failed after `read_retries` more attempts. A login error is also `failed`. The tool keeps the last good rows.
3. `unreachable`: the connection failed, for less than `offline_after` seconds. The tool keeps the last good rows.
4. `offline`: the connection failed for `offline_after` seconds or more. The tool applies `offline_action` (default `remove`).

An API timeout is a read failure. The tool opens a new session and tries again.
The tool removes inactive subscribers only from routers in the `online` state.

## Alternatives

1. Remove the subscribers at the first failed connection. This is `offline_after = 0`. A short network problem then removes all shaping for the router.
2. Never remove the subscribers of an offline router. This is `offline_action = "keep"`. Old rows then stay in LibreQoS for an unlimited time.

## Consequences

- The tool keeps the last good rows of each router in `state_dir`.
- "Offline" means that the shaper cannot connect to the API. If only the management path is down, the subscribers lose shaping after `offline_after`.
- The settings are in `[router_access]`. Refer to `docs/CONFIGURATION.md`.

## Status
accepted on 2026-10-31 (operator decision in chat)
