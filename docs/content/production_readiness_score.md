# Production Readiness Score

LQoSync v2.68 adds a read-only Production Readiness score on the Dashboard and through `/api/production/readiness`.

The goal is to give operators one compact go-live confidence indicator without adding another monitoring page.

## What it checks

Production Readiness combines existing system signals:

- config errors and warnings
- Setup Wizard progress and production gate blockers
- first Dry Run status
- MikroTik router/source readiness
- `backup_before_apply` safety when auto-apply is enabled
- LibreQoS generated file paths and working directory
- Policy Conflict Resolver results
- Dashboard source health and performance trends
- LibreQoS apply health
- monitored service health

## Score levels

- `production_ready` — no blockers and score is high enough for unattended operation.
- `ready_with_warnings` — usable, but review warnings first.
- `needs_review` — not recommended for unattended production until warnings are addressed.
- `not_ready` — major readiness problems exist.
- `blocked` — hard blockers are present, such as config errors, missing routers/sources, failed dry run, or invalid paths.

## Safety model

This feature is read-only. It does not enable the scheduler, change policies, contact MikroTik routers, write generated files, or run LibreQoS.

Scheduler, cleanup, and auto-apply behavior continue to be controlled by Config Center, Smart Policies, Setup Wizard, and existing guards.

## Operator workflow

1. Open Dashboard.
2. Review Production Readiness.
3. Fix blockers or warnings using the linked target pages.
4. Run Dry Run.
5. Review Operations Center and Policy Conflict Resolver.
6. Enable scheduler only when readiness is acceptable.

## API

```text
/api/production/readiness
```

The API returns the same score, checks, blockers, warnings, recommendations, and next action used by the Dashboard card.
