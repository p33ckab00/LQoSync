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
- run-cycle orchestration;
- dry-run preview generation;
- legacy PPPoE/DHCP/Hotspot row transformation;
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
- config/user/backup/diagnostic support used by the Flask shell.

## Removed legacy loop

`LQoSyncScheduler` no longer contains the old Python scheduler loop. When the web app starts, it only registers/heartbeats with Rust scheduler authority.

## Retirement-ready is stricter than authority-ready

Rust backend authority does not automatically mean the Python backend can be deleted.

Current retirement blockers now include:

- any remaining Flask UI support module that is still imported by `app.py`;
- the remaining safety-gated live RouterOS enablement and diagnostics path, which still needs explicit operator promotion before removing the Flask-adjacent Python support files is safe.

`engine/run_cycle.py` has been retired. The old Python PPPoE/DHCP/Hotspot collector transformation files, Python preflight/duplicate validators, and Python LibreQoS runner have also been removed from the active package. Diff generation, runtime validation, policy shadowing, sync-plan authority preview, and apply-manifest preview are Rust-owned.

Rust now owns read-only dry-run shadow generation for both `ShapedDevices.csv`
and `network.json`, and the WebUI/API dry-run path now calls that Rust-native
preview directly. Scheduled/manual run entry also now enters Rust first through
`run-rust-cycle-authority`. The old `scripts/run_cycle_once.py` bridge has been
removed from that path, so backend deletion is now blocked by Flask UI support
imports and safety promotion evidence, not by an active Python executor in the
Rust authority boundary.

## Compatibility bridge note

Some Python files remain because the Flask UI, operator diagnostics, config/user management, backup views, and read-only RouterOS test helpers still need them. They must not be interpreted as Python backend authority. Stable config disables Python mutation fallback.

`build-python-legacy-retirement-inventory` is the Rust-owned cleanup classifier for this boundary. It preserves Flask WebUI shell files and keeps `delete_allowed=false` until an external rollback-aware cleanup script is explicitly run.
