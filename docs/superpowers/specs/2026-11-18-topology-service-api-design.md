---
type: spec
created: 2026-11-18
status: draft
related: ["[[topology-manager-status]]", "[[2026-11-18-topology-enrollment-cli]]"]
---

# Service API design (Phase 2a)

## Intent

The running `lqosync3.service` holds the instance lock, the registry, and the router sessions.
Another process cannot change the registry while the service runs.
The owner wants the TUI now, and a web frontend later, to edit Topology Manager policy without a service stop.

Phase 2a adds a local API in the service. Phase 2b (Topology TUI) and 2c (Settings migration) use it.
A later web frontend is a separate process that uses the same API.

## Roadmap context

- 2a: service API (this document).
- 2b: Topology TUI: Builder, Assignments with per-subscriber plan and pin, Policies, and a Changes strip.
- 2c: Settings migration of node capacity, factors, and subscriber policy controls.
- Phases 3 to 6 stay as approved: planner/DPDK proof, activation, scheduler, other migration paths.

## Owner decisions

- Transport: JSON lines over a Unix domain socket (Approach A). No new dependency.
- Access: root only. The owner runs `sudo lqosync3 tui`. An operator group can come later without a protocol change.
- Both plan models stay: per-plan VLAN/DHCP server, and per-subscriber plan in LQoSync. 2b adds the per-subscriber operations.

## Approaches

| Approach | Summary | Trade-off |
|---|---|---|
| A. JSON lines over a Unix socket | One JSON request and one JSON response on each line | No dependency, immediate answers. No `curl`. A web backend needs a small translator |
| B. HTTP/1.1 over a Unix socket (`hyper`) | Standard HTTP framing | `curl` works. Three new crates and more code in the sync service |
| C. File inbox | Request and result files, processed each cycle | No socket. An answer can take one full cycle interval |

The owner chose **A**.

## Goals

1. The service listens on a root-only Unix socket while it holds the instance lock.
2. Each request runs on the service task that owns the `Runner`, between cycles. The cycle cadence does not change.
3. A shared client module serves the CLI, the TUI, and later a web backend.
4. First operations: `status`, `registry.get`, and a revision-checked `policy.commit`.

## Non-goals

- No HTTP, TLS, network listener, or user login in the service.
- No domain edit operations beyond `policy.commit` (2b adds them).
- No enrollment through the API in this phase.
- No TUI change in this phase.
- No change to live shaping before Phase 4 activation. After activation, a committed edit enters the approved automatic publication pipeline (see the Topology TUI layout). There is no separate Apply step.

## Anti-goals

- An API request that delays or skips a sync cycle by more than the time to process that request.
- A process that is not the service owner or root that reads or changes the registry.
- A socket that stays usable after the instance lock is lost.

## Transport and framing

- Socket path: `<runtime.state_dir>/api.sock`. Production: `/var/lib/lqosync3/api.sock`. The systemd unit already allows writes there, so no unit change is necessary.
- Socket file mode `0600`.
- Each request and each response is one UTF-8 JSON object on one line, ended by `\n`.
- Maximum request line: 1 MiB. Maximum response line: 16 MiB. A longer line closes the connection with an error.
- One connection can send several requests. The server answers them in order. Each connection has an idle timeout of 60 s.

Request:

```json
{"v": 1, "id": "client-chosen-string", "op": "status", "args": {}}
```

Response:

```json
{"v": 1, "id": "client-chosen-string", "ok": true, "result": {}}
{"v": 1, "id": "client-chosen-string", "ok": false, "error": {"code": "conflict", "message": "..."}}
```

Error codes: `bad_request`, `unsupported_version`, `unknown_op`, `unauthorized`, `conflict`, `invalid`, `busy`, `internal`.
An unknown `v` gives `unsupported_version`. The server does not guess a version.

## Access control

- On each accepted connection, the server reads `SO_PEERCRED` (`UnixStream::peer_cred`).
- The server accepts a peer when its uid is 0 or equals the service process effective uid. Otherwise it answers `unauthorized` and closes.
- Production runs the service as root, so only root can connect. Tests run as a normal user, and the same rule then accepts that user.

## Lifecycle

- The service binds the socket after `Runner::lock()` succeeds. The instance lock proves ownership.
- A stale `api.sock` file is removed only while the service holds the lock. A non-socket file at that path stops startup.
- On every exit path of `run`, a drop guard removes the socket file.
- If lock ownership is lost (`Runner::lock()` fails for a request), the server answers `internal` and the service exits with an error.

## Concurrency with the cycle loop

- An accept task reads and validates lines and sends each request through a bounded channel (capacity 16) to the main task. The reply comes back through a one-shot channel.
- The main loop keeps one sleep deadline for each cycle. While it waits, it handles API requests, then waits again for the same deadline.
- A request that arrives during a cycle waits until the cycle ends. The client sees `busy` only when the channel is full.
- The client default timeout is 120 s, which covers a long cycle.
- `SIGHUP`, `SIGUSR1`, `SIGTERM`, and `SIGINT` keep their current behavior.

## Operations

| Operation | Args | Result | Notes |
|---|---|---|---|
| `status` | none | service version, runtime mode, registry revision (`null` when no registry exists), enrolled (true when export ownership exists), last cycle start time | Read-only. Does not create the store |
| `registry.get` | none | the current `RegistryDocument`, or `null` | Read-only. Root-only data. Does not create the store |
| `policy.commit` | `expected_revision`, `catalog`, `source_settings` | new registry revision | Validates the full document and commits through the existing store. A different revision gives `conflict`. A validation error gives `invalid` |

`policy.commit` replaces only the `catalog` and `source_settings` sections. Subscribers and export ownership stay as they are.
The existing validation also checks installed subscribers and export ownership against the new catalog.

## Builder calls (made by the designer, open to owner correction)

- Preconditions: `status` and `registry.get` need only the held instance lock. `policy.commit` needs the lock and the existing store commit checks. None of the three needs the v3 pair apply target, so they also work in shadow mode.
- Lock check: each request runs `Runner::lock()` first. A lost lock answers `internal`, and `run` then exits with an error.
- No store creation on read: when `topology-manager/registry.json` is absent, `status` reports `"registry_revision": null` and `registry.get` returns `null`. Reads do not create the store.
- Commit errors: the store gets a small typed error so the handler maps a revision mismatch to `conflict` and a validation failure to `invalid`. No string matching. After a failed commit, the handler drops the cached store and reopens it on the next request, the same rule as recovery.
- Commit checks: reuse `RegistryDocument::validate` and the store `preview` rules (ownership preservation). No extra check.
- Cycle wait: a request waits for a running cycle. The client default timeout is 120 s. The TUI shows a waiting state. `busy` means only a full channel.
- Socket removal: a drop guard removes the socket file on every exit path of `run`, not only on signals.

## Client and CLI

- Add `lqosync3::api::client` with `request(state_dir, op, args, timeout) -> Result<Value, ApiError>`.
- Add `lqosync3 api status` for operators and for the runbook. It prints the status result. Exit status 1 when the service does not answer.

## Verification

- Unit tests: framing (oversize line, bad JSON, unknown op, unknown version), error codes, and the peer rule.
- Service tests with a temporary state directory and the fake connector: `status`, `registry.get`, `policy.commit` success, revision `conflict`, and `invalid`.
- A test that requests during the wait do not start a cycle early and do not move the next cycle.
- A test that a stale socket file is replaced only under the lock, and a non-socket file stops startup.
- A test that the socket file is removed at shutdown.
- Full suite, format, Clippy with warnings denied, and whitespace checks.

## Left to the builder

- Module layout under `src/api/`.
- Exact result field names for `status`.
- How the main loop structure keeps one sleep deadline.
