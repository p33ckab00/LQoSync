# Full Rust Backend Boundary
nRust authority daemon = backend authority


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

## Compatibility bridge note

Some Python files remain because the Flask UI, operator diagnostics, and transport compatibility still need them. They must not be interpreted as Python backend authority. Stable config disables Python mutation fallback.
