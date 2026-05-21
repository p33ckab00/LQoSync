# Flask UI Shell

The WebUI stays Flask.

## Non-goals

- Do not migrate this appliance to Django.
- Do not convert this to SaaS.
- Do not introduce multi-tenant product behavior.
- Do not redesign the WebUI unless needed for Rust wiring.

## Flask responsibilities

Flask owns:

- login/session UI
- dashboard
- Config Center
- Dry Run page
- Operations Center
- docs/search pages
- manual buttons and forms

Flask does not own production mutation authority.

## Rust call path

```text
Flask route/button
  → engine.rust_core / engine.rust_scheduler wrapper
  → /run/lqosync-core.sock
  → Rust authority daemon
  → JSON response
  → Flask displays result
```

## Scheduler behavior

When `scheduler.engine=rust`, Flask does not start the old Python scheduler loop. It uses `RustAuthorityScheduler` as a compatibility facade.
