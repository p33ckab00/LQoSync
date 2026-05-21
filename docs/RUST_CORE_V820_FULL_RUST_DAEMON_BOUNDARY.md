# v8.2.0 Full Rust Daemon Boundary Cleanup

This release cleans the remaining scheduler/backend boundary after v8.1.0.

## Canonical project identity

LQoSync is a local appliance-style web application for syncing MikroTik subscriber data into LibreQoS. It is not a SaaS platform and does not use Django.

## Runtime ownership

| Area | Owner |
|---|---|
| Web UI, login, forms, dashboard | Python Flask shell |
| Scheduler authority | Rust daemon |
| Scheduler status / heartbeat / run authorization | Rust daemon |
| Production mutation decision | Rust daemon |
| File write transaction | Rust daemon |
| Transaction journal | Rust daemon |
| LibreQoS apply authority | Rust daemon |
| Rollback / quarantine / readiness gates | Rust daemon |

## Removed production path

The legacy Python scheduler loop is removed from `scheduler/runner.py`. The Flask app keeps the same `LQoSyncScheduler` facade only so existing routes and templates remain stable. All methods delegate to `RustAuthorityScheduler`.

## Remaining Python code

Python modules that still exist are not the production authority. They are retained for one of these reasons:

- Flask WebUI shell;
- configuration forms and dashboard rendering;
- operator diagnostics;
- compatibility transport adapters under Rust gates;
- test/regression helpers.

Deleting those files before a Rust replacement exists would break the appliance UI and/or MikroTik/LibreQoS sync workflows.

## Diagram

The canonical workflow diagram is now stored at:

```text
lqosync_workflow_architecture.svg
docs/lqosync_workflow_architecture.svg
```

It no longer shows a Python legacy scheduler loop. It shows Flask as the UI shell and Rust as the authority daemon.
