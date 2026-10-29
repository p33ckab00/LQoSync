---
type: handoff
created: 2026-10-29
tags: [lqosync-v3, handoff]
status: active
---

# LQoSync 3 handoff

Updated: 2026-10-29T13:51:23+08:00 (Asia/Shanghai).
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


## Speed rules update

The user authorized speed rules, performance/integrity review, commit, and push.
Base commit for this slice: `56278d6`; branch and checkout remain unchanged.
New modules: `v3/src/speeds.rs`, `speed_preview.rs`, and `speed_web.rs`.
Changed: model settings, read-only RouterOS fields, web routes/inventory sharing,
Rust dependencies, speed and web tests, documentation, and this vault.

Speed resolution and UI previews are complete. Subscriber snapshot publication is not connected to inventory yet.
See [[10-sessions/2026-10-29-speed-rules]] and `v3/SPEED_RULES.md` for exact precedence and semantic corrections.
The old script was read as a reference. It was not run or modified.
No production credentials, router writes, LibreQoS writes, package changes, or service restarts occurred.

Checks: 39 functional tests passed; manual performance test passed separately.
Last benchmark: 50,000 PPP sessions, 100 profiles, 220.3 ms in a debug build.
Strict lint, formatting, diff checks, and HTTP smoke passed.
Independent review found numeric-token and duplicate-selection defects. Both were fixed with regression tests.
A follow-up whitespace regression was also fixed. Final review found no remaining material issue.
Concurrent policy changes invalidate preview work; a deterministic queued-worker test verifies this.
The preview and policy save preserve subscriber snapshots and staged generations.

Next action: implement stable subscriber identity and source selection, then publish only complete valid snapshots.
Keep the discovered map separate from the shaping tree. Do not apply production shaping in that work without scope authorization.


## Subscriber normalization update

Goal complete: subscriber normalization, discovered source selection UI, automatic local publication, review, and verification.
Branch/worktree remain `lqosyncv3` at `/opt/LQoSync3.0`. Base for this slice: `7623e31`.
Last agent: Codex.
Changed modules: `normalizer`, `orchestrator`, `sources_web`, `model`, `store`, `speed_preview`, `speed_web`, `web`, and `main`.
Changed tests, documentation, UUID feature/lockfile, and project notes accompany the code.

Sources bind exact router/profile/server metadata to real subscriber nodes.
Scheduled collection polls only selected routers, at most four at a time. Pausing stops pending launches.
Healthy source changes, reads, and policy saves trigger local normalization automatically.
The snapshot and both generated files share one authoritative immutable generation pointer.
The state snapshot is a compatibility mirror. A failed mirror does not invalidate committed data.
Unknown, partial, stale, empty, or conflicting collections retain the last valid generation.

Verification: 59 functional tests passed. Clippy, formatting, and diff checks passed.
Two manual benchmark tests are ignored by default; the new normalization benchmark passed separately.
Final debug measurement: 50k subscribers /100 selected profiles, 639.2 ms normalization, 921.7 ms including compilation.
Source HTTP tests and a full local mock-router scheduled round prove automatic local publication.
Independent review fixes cover active DHCP fields, unknown nonempty mappings, exact names, queued-read pause, and UI work placement.
Repeat review found no remaining material issue.

No production credentials were used. Mock logins use a non-secret environment value only against local mock sockets.
No LibreQoS files, packages, services, shaping, or router configuration were changed.
Operation and limits: `v3/SUBSCRIBERS.md` and [[10-sessions/2026-10-29-subscriber-normalization]].
Next action: validate representative isolated router behavior before deployment or additional production integrations.
Stable WAN/core-list assignment, effective RADIUS, IPv6/multi-device circuits, authentication, and generation retention remain future work.
Run one app process per data directory. Production export/apply remains disabled.
