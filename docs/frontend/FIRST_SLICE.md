# Sign-in and Dashboard implementation

Date: 2026-11-20 (Asia/Shanghai). Branch: `lqosync3`.

## Delivered scope

The browser application uses Rust, Leptos CSR, and WebAssembly.
The gateway listens on port `9581`. The selected origin is `https://lqosync.fsehub.qzz.io`.
The TUI and the synchronization service keep their current behavior.

The sign-in screen has the accepted split layout, a glass panel, and the LQoSync mark with superscript `v3`.
Light, Dark, and System controls appear on sign-in and in the Dashboard header.
System is the default. A theme cookie stores an explicit preference.
The gateway restores this preference before the browser starts Wasm.
System mode follows changes in the browser environment.

The Dashboard shows subscriber counts, PPPoE counts, router counts, traffic samples, synchronization evidence, and component health.

The browser requests an overview every five seconds when the page is visible.
It keeps one request in progress. It ignores replies from an old session.
Each browser request has a 15-second deadline.

Unknown counts use `—`. Stale data has a visible label.
A publication phase does not prove dataplane acceptance.

The web UI remains the primary configuration and orchestration surface in the full plan.
This first slice supports sign-in and monitoring only.
Feature editors, mutation commands, company branding, and other screens remain in the implementation plan.
There are no inactive navigation links or simulated production values.

## Runtime boundary

| Part | Source | Responsibility |
|---|---|---|
| Browser contracts | `crates/web-contract` | Safe DTOs without core configuration types |
| Browser application | `crates/web-ui` | Forms, themes, Dashboard, and bounded SVG traffic chart |
| HTTP gateway | `crates/web-gateway/src/http.rs` | Session checks, origin checks, CSRF checks, static assets |
| Private broker | `crates/web-gateway/src/broker.rs` | Credential checks, account epoch checks, cached overview |
| Account CLI | `lqosync3-web-admin` | Root-only account changes and audit records |

The gateway runs as `lqosync-web`. It cannot read the root account database.

The root broker accepts a fixed request enum through a Unix socket.
It checks the peer UID. It cannot accept shell commands or arbitrary paths.
The broker has no TCP listener. The socket has mode `0600` and the gateway UID owns it.

The runtime directory is root-owned. The gateway cannot replace the socket.

The broker reads only four fixed cache names in `/var/lib/lqosync3`:
`status.json`, `collection.json`, `system-health.json`, and `worker-utilization.json`.
The existing worker-cache link to `/dev/shm/lqosync3-0` is allowed.
It does not read private snapshots, router credentials, or configuration files.
Requests use a cached projection. They do not trigger router polling.

Each input has a 2 MiB read limit. A larger input produces unknown evidence.
The build produces deterministic gzip variants for browser assets.
The gateway serves these variants when the client accepts gzip.

The traffic output has at most 300 points. The health output has at most 32 components.

The overview rejects stale status, mismatched process IDs, and incomplete collection evidence.
A failed router prevents current subscriber and PPPoE totals.
A missing PPPoE source produces unknown PPPoE counts.

## Accounts and sessions

Use `sudo lqosync3-web-admin user create USERNAME` to create a superadmin.
Use `user list`, `user show USERNAME`, `user passwd USERNAME`, or `user rename OLD NEW` for account maintenance.
Use `user disable USERNAME`, `user enable USERNAME`, or `user delete USERNAME` for access changes.

Passwords use a hidden interactive prompt. They are not CLI arguments.
There is no default account or password.

Each account is a superadmin. The CLI supports multiple accounts.
It blocks removal or disablement of the final active superadmin.
Account changes have an audit record in the private database.

Password, name, or access changes increment the account epoch.
The next authenticated API request checks this epoch and rejects an old session.
There are no signup, account CRUD, or password reset HTTP routes.

Passwords use Argon2id. Login checks have a limit of five attempts per username per minute.
The broker permits at most two password hash checks and sixteen connections at one time.

HTTP bodies have a 4096-byte limit. IPC messages and replies have size and time limits.
Cookies use HttpOnly, SameSite=Strict, and Secure for the HTTPS origin.
The database stores a SHA-256 session token digest, not the browser token.

Sessions expire after eight hours or thirty minutes without API use.
Polling counts as API use. Sign-out requires the configured origin and a CSRF token.
Sign-out revokes the session in the database.
A broker failure rejects authenticated access with HTTP 503.

## Plan status and remaining work

This slice implements parts of F00 through F06. It does not close those complete task cards.
The broker currently supports authentication and overview reads only.
It does not support the planned domain actions, jobs, configuration edits, or service-command recovery.

The overview reads a bounded JSON projection. A compact backend summary cache remains a future optimization for large subscriber sets.
All other screen settings remain on their feature screens in the plan.
Company name and logo remain future global Settings features.

See [deployment](DEPLOYMENT.md), [dependencies](DEPENDENCIES.md), and [verification](evidence/FIRST_SLICE.md).
