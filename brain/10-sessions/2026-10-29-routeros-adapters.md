---
type: session
created: 2026-10-29
tags: [lqosync-v3, routeros, compatibility]
status: active
---

# RouterOS transport and LibreQoS detection

The user requested a better Rust MikroTik API and automatic v2/v3 adapter selection.
Use `mikrotik-rs` 0.7.0 for its async tagged API, verified TLS, and MIT/Apache license.
Later 0.8–0.10 releases use AGPL-3.0-only. The pinned release lacks some newer fixes.
Patch the vendored transport to preserve response bursts through bounded backpressure.
Keep the protocol implementation in the external library.

Router inventory is read-only and on demand. It is not the normalized subscriber source yet.
Partial or failed reads must not replace `state/subscribers.toml`.
The application owns its TOML settings and inventory files.
No production credentials were used during development.

Select the control family from the main installed package, never `libreqos-preflight`.
The supported detection range is 2.0–2.2 and 3.x. Other versions fail closed.
Legacy requests use `LibreQoS.py --updateonly`.
V3 requires authenticated `ReloadLibreQoS`, then applied-generation verification.
The UI shows the selection; production execution remains disabled.
The host still has held LibreQoS 2.2. No package or shaping changes were made.

Tests cover tagged login, property filtering, failed/incomplete replies, timeouts,
200-row bursts, trusted TLS, wrong certificate names, and untrusted certificates.
See [[../HANDOFF]] and `v3/README.md` for the next implementation work.
