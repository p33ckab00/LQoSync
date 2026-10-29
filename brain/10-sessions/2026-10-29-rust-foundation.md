---
type: session
created: 2026-10-29
tags: [lqosync-v3, rust]
status: active
---

# Rust foundation

The user authorized a new checkout at `/opt/LQoSync3.0` and a push to branch `lqosyncv3`.
The user selected Rust for the backend and web UI.
The application uses local TOML settings and state. It does not use a database.

The first slice contains router forms, a shaping tree editor, capacity rules, snapshot validation, and local publication.
All production adapters remain absent. No router or LibreQoS changes are sent.
The discovered network map and exported shaping hierarchy must remain separate.
Every exported shaping node is real. V2 virtual nodes do not belong in the v3 design.

The Rust UI uses Maud with Axum. This is a server-rendered interface.
The existing repository remains intact; the new crate is under `v3/`.

Current checks and remaining work are in [[../HANDOFF]].
