# Full Rust Backend Boundary

Rust authority daemon = backend authority


The stable architecture is:

```text
Flask WebUI shell -> Rust authority daemon -> LibreQoS/MikroTik workflows
```

Python Flask remains because LQoSync is a local web app with an existing interface. The WebUI is not being rewritten to Django and the product is not being converted into a SaaS platform.

## What Rust owns

- scheduler authority;
- run authorization;
- collector output validation;
- sync-plan enforcement;
- file write authority;
- transaction journal;
- LibreQoS apply authority;
- rollback/quarantine/watchdog/readiness gates.

## What Python owns

- Flask pages;
- forms and operator UX;
- displaying Rust results;
- calling the Rust daemon over the local protocol.

## Removed legacy loop

`LQoSyncScheduler` no longer contains the old Python scheduler loop. When the web app starts, it only registers/heartbeats with Rust scheduler authority.

## Retirement-ready is stricter than authority-ready

Rust backend authority does not automatically mean the Python backend can be deleted.

Current retirement blockers still include:

- legacy rollback compatibility files such as `engine/run_cycle.py`;
- the remaining safety-gated live RouterOS enablement path, which still needs explicit operator promotion before Python removal is safe.

`engine.run_cycle` now sends its non-mutating sync-engine shadow bundle to Rust through `build-rust-sync-engine-shadow-preview`, so diff generation, runtime validation, policy shadowing, sync-plan authority preview, and apply-manifest preview are already Rust-owned even before live mutation authority is fully cut over.

Rust now owns read-only dry-run shadow generation for both `ShapedDevices.csv`
and `network.json`, and the WebUI/API dry-run path now calls that Rust-native
preview directly. Scheduled/manual run entry also now enters Rust first through
`run-rust-cycle-authority`. The old `scripts/run_cycle_once.py` bridge has been
removed from that path, so backend deletion is now blocked by the remaining
rollback-only Python modules and by safety promotion evidence, not by an active
Python executor in the Rust authority boundary.

## Compatibility bridge note

Some Python files remain because the Flask UI, operator diagnostics, and transport compatibility still need them. They must not be interpreted as Python backend authority. Stable config disables Python mutation fallback.

`build-python-legacy-retirement-inventory` is the Rust-owned cleanup classifier for this boundary. It preserves Flask WebUI shell files, marks backend-only Python paths as guarded archive candidates, and keeps `delete_allowed=false` until an external rollback-aware cleanup script is explicitly run.
