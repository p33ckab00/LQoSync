---
type: decision
created: 2026-11-02
tags: [adr, pppoe, removal, operator]
status: accepted
supersedes:
related: ["[[ADR-0011-v3-import-request-and-check]]"]
---

# ADR-0012 — No subscriber grace on the production shaper

## Context

On 2026-11-02, one PPP subscriber disconnected and connected again three times in 3 minutes.
With `runtime.subscriber_grace = 0`, each disconnect removed the circuit. Each connection got a new circuit ID.
Each removal gave an R3 cold rebuild in LibreQoS v3. A grace of 300 s would keep the row and its ID through a short disconnect.

## Decision

Keep `subscriber_grace = 0` (the default). The operator decided this.
A disconnect is a device that was unplugged. Its next connection must be a new circuit with a new ID.

## Consequences

- Do not recommend a subscriber grace to stop removals after a short disconnect.
- A disconnect and a connection give two changes, two reload requests, and new IDs. Stats history does not continue across a disconnect.
- A subscriber with an unstable link gives many R3 rebuilds. Find the cause on the router (`/log print where topics~"ppp"`).
