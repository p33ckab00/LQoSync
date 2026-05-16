# LibreQoS Apply Failure Visibility

LQoSync v2.70.7-rc1 makes LibreQoS apply failures actionable instead of invisible.

## Problem solved

Previously, Dashboard or Telegram could warn that LibreQoS apply failed, but the warning did not clearly show where to inspect the failure or what page should be opened to resolve it.

## What changed

- Apply health notifications now link to the latest failed apply diagnostic page when a run ID is available.
- Dashboard notification cards are clickable and include an “Open resolve page” hint.
- Operations Center → Apply History now has a **Detail / Resolve** button for each apply run.
- Failed apply runs show a short diagnostic summary and resolution hint inline.
- A new apply diagnostic page shows stderr/stdout tail, exit code, command metadata, working directory, failure classification, suggested next page, and suggested commands.
- A read-only diagnostic API is available for each apply run.

## Routes

```text
/libreqos/apply/<run_id>
/api/libreqos/apply/<run_id>/diagnostic
```

Existing log routes remain:

```text
/api/libreqos/apply/<run_id>/stdout
/api/libreqos/apply/<run_id>/stderr
```

## Typical classifications

The diagnostic helper recognizes common failure patterns:

- invalid `libreqos.working_dir`
- `nsenter` / namespace mode mismatch
- permission denied
- missing file or command
- apply timeout
- LibreQoS Python exception / traceback
- unknown apply failure

## Operator workflow

1. Open Dashboard or Telegram alert.
2. Click the apply failure notification.
3. Open the apply diagnostic page.
4. Review summary, resolution hint, stderr/stdout, and suggested commands.
5. Fix the root cause in Setup / System Validation, Config Center paths, or Operations Center.
6. Retry apply only after the underlying error is fixed.

This is a UI/diagnostics wiring improvement only. It does not change LibreQoS apply mechanics.
