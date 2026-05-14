# Better Fresh Install Experience

LQoSync v2.60 improves the first-run onboarding path so a new installation is guided through setup before the production scheduler is enabled.

## Goals

- Fresh installs should open the First Run Setup Wizard instead of silently landing on the normal Dashboard.
- Existing/upgraded production installs should not be trapped in the wizard if they already have a scheduler or previous run history.
- Scheduler enable should be protected by a production-readiness gate.
- Operators should clearly see what is blocking go-live.

## First-run gate

The wizard checks:

- LibreQoS paths and files
- MikroTik router credentials
- enabled PPPoE/DHCP/Hotspot sources
- selected Network Layout mode
- selected Smart Policy preset
- completed Dry Run
- Setup & Repair failed checks

If the gate is not ready, the scheduler enable button is locked and the Dashboard shows a First Run Setup banner.

## Scheduler protection

Scheduler enable is blocked when any required setup gate is not satisfied. The default requirements are:

```json
{
  "setup_wizard": {
    "scheduler_enable_requires_dry_run": true,
    "scheduler_enable_requires_no_failed_checks": true,
    "scheduler_enable_requires_router_and_source": true
  }
}
```

This prevents a fresh installation from accidentally auto-applying LibreQoS before the operator has tested MikroTik collection, generated files, and policy decisions with Dry Run.

## Existing installs

Existing installs are treated as already acknowledged when they have prior run history or scheduler is already enabled. Operators can still reset the wizard from the Setup Wizard page if they want to re-run onboarding.

## Operator workflow

Recommended fresh install sequence:

1. Confirm LibreQoS paths.
2. Configure MikroTik API access.
3. Choose PPPoE/DHCP/Hotspot sources.
4. Choose Network Layout mode.
5. Choose Smart Policy preset.
6. Run Dry Run.
7. Review Dashboard, Reports, Lifecycle, and Setup & Repair.
8. Enable scheduler deliberately.

## Notes

The First Run Setup Wizard is an onboarding and safety workflow. It does not replace Config Center, Policy Center, Setup & Repair, or the documentation system.
