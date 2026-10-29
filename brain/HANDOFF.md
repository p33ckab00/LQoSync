---
type: handoff
created: 2026-10-29
tags: [lqosync-v3, handoff]
status: active
---

# LQoSync 3 handoff

Updated: 2026-10-29T11:11:24+08:00 (Asia/Shanghai).
Last agent: Codex.
Branch: `lqosyncv3`.
Checkout: `/opt/LQoSync3.0`.
Base: `f22cfbf8d974c10300a7a704cd7a712509d9827d` from `lqosync-in-rust`.

## Goal

Build a Rust backend and Rust web UI for MikroTik subscriber and topology orchestration.
Use local TOML settings and state. Preserve the live LibreQoS v2.2 installation and its package hold.
The user authorized commit and push to `p33ckab00/LQoSync`, branch `lqosyncv3`.

## First slice

New application: `v3/`.
Axum serves Maud pages written in Rust.
Router forms store password environment references.
The topology editor supports explicit WAN roots and manual/computed branches.
The compiler validates parents, cycles, directional caps, duplicate IPv4 addresses, and subscriber rates.
The local publisher completes both files before it changes a durable generation pointer.
No production adapters exist in this first slice.
Existing root-level application files remain available for reference.

Changed files: `v3/`, root `README.md`, `docs/superpowers/`, and this project-specific `brain/` vault.

## Checks

- `cargo test --locked`: 11 tests passed.
- `cargo clippy --locked --all-targets -- -D warnings`: passed.
- `cargo fmt --check`: passed.
- HTTP smoke: six routes responded; automatic local publication worked.
- Invalid snapshot smoke: the previous generation pointer remained unchanged.
- Existing v3 preflight against generated offline example: topology validated; two circuits detected.
- Overall preflight exit: 1 (existing host warnings). This is not deployment clearance.
- Independent review: fixed computed-parent validation and generation directory durability. Follow-up review found no remaining material issue in those fixes.
- Live package remains `2.2.202608181454`, held. LibreQoS and sync process IDs remained unchanged.

## Next work

Implement RouterOS read transport and per-source collection health before live ingestion.
Separate the discovered network graph from real shaping nodes.
Read source-owned subscriber speeds; correct router `rx/tx` direction for asymmetric PPP rates.
Current script defaults and aggregation rules must not be misrepresented as router-native policy.
Add stable WAN assignments and system-owned individual IPv4 `/32` core lists.
Review existing routing rules before any router write.
Validate the exact v3 reload authentication and applied-generation acknowledgement.
Add user authentication, controlled deployment, audit history, and generation retention before network exposure.
The current console binds only to loopback and publishes only local output.

Start the console from `v3/` with `cargo run --locked`.
See `v3/README.md` for the offline example and SSH access.
