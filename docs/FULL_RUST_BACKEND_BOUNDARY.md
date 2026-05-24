# Full Rust Backend Boundary

Rust backend service = backend authority and web runtime


The stable architecture is:

```text
Svelte operator console -> Rust HTTP/API server -> LibreQoS/MikroTik workflows
```

The supported runtime is Svelte + Rust. The WebUI is not being rewritten to Django and the product is not being converted into a SaaS platform.

## What Rust owns

- scheduler authority;
- run authorization;
- run-cycle orchestration;
- dry-run preview generation;
- legacy PPPoE/DHCP/Hotspot row transformation;
- collector output validation;
- sync-plan enforcement;
- file write authority;
- transaction journal;
- LibreQoS apply authority;
- rollback/quarantine/watchdog/readiness gates.
- HTTP/API endpoints;
- embedded Svelte static asset serving;
- login/session handling for the Rust web console.

## What Python owns

Python no longer owns a supported backend runtime role. Remaining Python files are legacy/tooling assets, migration helpers, historical diagnostics, or rollback reference material unless a script explicitly documents otherwise.

## Removed legacy loop

`LQoSyncScheduler` no longer contains the old Python scheduler loop. The supported web app is served by `lqosync-core`.

## Retirement-ready is stricter than authority-ready

Rust backend authority does not automatically mean every Python file in the repository can be deleted.

Current retirement blockers now include:

- migration scripts that still normalize older installs;
- operator diagnostics that have not yet been ported or archived;
- historical docs and rollback references.

`engine/run_cycle.py` has been retired. The old Python PPPoE/DHCP/Hotspot collector transformation files, Python preflight/duplicate validators, and Python LibreQoS runner have also been removed from the active package. Diff generation, runtime validation, policy shadowing, sync-plan authority preview, and apply-manifest preview are Rust-owned.

Rust now owns read-only dry-run shadow generation for both `ShapedDevices.csv`
and `network.json`, and the WebUI/API dry-run path now calls that Rust-native
preview directly. Scheduled/manual run entry also now enters Rust first through
`run-rust-cycle-authority`. The old `scripts/run_cycle_once.py` bridge has been
removed from that path. Backend runtime deletion is complete; repository cleanup is now a separate archive/removal task for legacy Python files and old Flask documentation.

## Compatibility bridge note

Some Python files remain because operator diagnostics, migration checks, and historical support scripts still exist. They must not be interpreted as Python backend authority. Stable config disables Python mutation fallback, and the install/startup paths do not start Flask/Gunicorn.

`build-python-legacy-retirement-inventory` is the Rust-owned cleanup classifier for this boundary. It should be used before deleting or archiving remaining Python files.
