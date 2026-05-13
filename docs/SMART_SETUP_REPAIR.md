# Smart Setup / Repair Center

LQoSync v2.48 adds a guided Setup / Repair Center for operators who want a safer way to verify, repair, and prepare an installation without guessing which command to run.

The center is read-only by default. It does not automatically repair the server when opened. Instead, it checks the current config, paths, services, Git status, LibreQoS runner settings, router configuration completeness, and backup readiness, then explains what is healthy, what needs attention, and what command should be run from SSH.

## What it checks

- `config.json` validation errors and warnings
- LibreQoS managed files: `ShapedDevices.csv` and `network.json`
- Runtime files such as `runtime_state.json`, `policy_state.json`, `audit.jsonl`, and backups
- LibreQoS command and working directory
- Bare-metal runner mode safety, especially `run_mode=direct`
- Router configuration completeness
- Required service status for `lqosd`, `lqos_scheduler`, and `lqos_shaped_sync`
- Git/update state when available
- Backup-before-apply readiness

## Readiness levels

- `ready` means no failed checks or warnings were found.
- `needs_attention` means no hard failure was found, but warnings should be reviewed before enabling scheduler or auto-apply.
- `repair_required` means one or more failed checks should be fixed before production use.

## Guided setup checklist

The page shows a first-install checklist:

1. Confirm LibreQoS base path.
2. Create restricted MikroTik API user.
3. Add or verify routers.
4. Discover DHCP servers.
5. Choose network layout mode.
6. Select Smart Policy preset.
7. Run Dry Run.
8. Enable scheduler only after the dry run is clean.

## Guided repair commands

The page provides copy-ready repair commands for common scenarios:

- Safe bare-metal repair/reinstall with `LQOSYNC_INIT_POLICY=preserve_existing`
- Restore LibreQoS permissions after uninstall or stale ACLs
- Run the environment doctor
- Safe GitHub update with `UPDATE_POLICY=preserve_and_migrate`
- Adopt ZIP/manual install into GitHub-managed install
- Check LibreQoS core services

## Policy preset setup

The page can apply one of the built-in Smart Policy presets:

- Conservative: safest live production behavior with more confirmations.
- Balanced: recommended default.
- Aggressive: fast cleanup for lab or highly dynamic environments.

After changing presets, always run Dry Run before enabling scheduler or auto-apply.

## MikroTik connection testing

The page lists configured routers and links operators to Config Center for live API tests. The Setup / Repair Center itself avoids contacting routers during page load so it remains safe and fast.

## Safety principle

The Setup / Repair Center explains actions and gives commands. It does not blindly modify LibreQoS files, run Git updates, restart services, or contact routers just because the page is opened.
