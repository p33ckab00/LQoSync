---
type: handoff
created: 2026-10-29
tags: [lqosync-v3, handoff]
status: active
---

# LQoSync 3 handoff

Updated: 2026-10-29T11:44:12+08:00 (Asia/Shanghai).
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

Normalize the new read-only RouterOS inventory into source/health snapshots before live subscriber ingestion.
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


## RouterOS and version detection update

Added the pinned `mikrotik-rs` 0.7.0 client with verified TLS and operation deadlines.
The Routers UI reads allowlisted inventory tables and saves per-table errors in local TOML.
It does not replace the subscriber snapshot. Missing credentials fail before a connection.
Settings and inventory remain under this application, separate from LibreQoS.
The UI clears active inventory after router settings change. Historical files remain diagnostic only.

The latest client releases changed to AGPL-3.0-only. The selected release retains MIT/Apache licensing.
Independent review found a real burst-response defect in its transport dependency.
The local `mikrotik-tokio` patch awaits bounded sends for reply and terminal events.
The 200-row burst test failed before the patch and passed after it.
Source revision, copyright, licenses, and patch limits are in `v3/vendor/mikrotik-tokio/`.
The client must consume commands sequentially. Do not retain unread concurrent receivers.

The Settings UI detects only the fully installed main `libreqos` package.
It selects legacy for 2.0–2.2 and the v3 scheduler family for 3.x.
Other versions and incomplete package states block selection.
The separate v3 preflight package does not select the v3 adapter.
HTTP smoke on this host selected legacy for `2.2.202608181454`.
Actual production apply execution remains disabled for both families.
V3 needs an authenticated `ReloadLibreQoS` request and applied-generation verification.
A scheduler request acknowledgement is not proof of applied shaping.

Changed files: `v3/src/{routeros,libreqos,model,web,lib}.rs`, Rust dependencies,
transport vendor files, RouterOS/version/UI tests, documentation, and this vault.
The new plan is `docs/superpowers/plans/2026-10-29-routeros-version-adapters.md`.


Update checks: 21 application tests passed; formatting and strict Clippy passed.
The HTTP smoke selected the correct legacy family on this host.
Independent repeat review found no remaining material issue after the burst/TLS fixes.
An optional `cargo test -p mikrotik-tokio` could not run because that dependency is not a workspace member.
Application protocol tests exercise the patched transport directly.
The dependency uses let chains; the declared Rust minimum is now 1.88.
The actual checks ran with Rust 1.99.1.
