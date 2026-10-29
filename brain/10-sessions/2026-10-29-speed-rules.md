---
type: session
created: 2026-10-29
tags: [lqosync-v3, speeds, integrity, performance]
status: complete
---

# Subscriber speed rules

The user authorized speed rules, performance and integrity review, then commit and push.
All work stays in `/opt/LQoSync3.0`. LibreQoS remains unchanged.

The new module resolves speeds from RouterOS inventory. It does not publish subscriber snapshots yet.
PPP precedence: secret comment, active comment, profile comment, profile rate, profile-name suffix.
DHCP uses lease comments and lease rates. Hotspot uses local user comments and mapped profile rules.
No local default maximum is assigned. Unknown RADIUS mappings remain unresolved.

Comments use subscriber download/upload order, as in the reference script.
Router rate fields use receive/transmit order, so the conversion swaps directions.
Bare router rates mean bits per second. Bare profile-name numbers do not supply a speed.
Explicit router minima take precedence. Otherwise the local fraction and floor supply the minimum.
The floor cannot increase the maximum or make the minimum exceed it.
Burst fields receive syntax checks but no burst or priority policy is exported.

`Speed rules` edits application-local TOML. Router pages link to speed previews.
A preview lists source, raw rule, maxima, minima, and unresolved errors.
Every conflicting duplicate remains unresolved. A partial table cannot supply its partial rows.
Concurrent inventory or policy changes discard the calculated preview.
The UI escapes router text and limits displayed entries to 200.
It shares immutable inventories and computes outside the application lock.
Profiles receive one parse per preview; subscriber joins use hash indexes.

Checks: 39 functional tests passed. The manual 50,000-session benchmark passed separately.
The last benchmark result was 220.3 ms in a debug build, with 100 shared profiles.
This is a diagnostic result, not a production latency guarantee.
Strict Clippy, formatting, diff checks, and HTTP smoke passed.
Independent review findings were fixed and rechecked.

Next: stable subscriber identity, source selectors, and complete snapshot publication.
Read [[../HANDOFF]] and `v3/SPEED_RULES.md` before that work.
