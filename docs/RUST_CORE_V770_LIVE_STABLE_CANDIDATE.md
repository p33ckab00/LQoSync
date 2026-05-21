# v7.7.0 Rust Live Stable Candidate

v7.7.0 adds the live-stable safety layer that sits above the v7.6 supervisor/watchdog path.

The goal is not to remove the Python WebUI/scheduler shell. The goal is to make promoted full Rust authority safer for unattended operation by adding quarantine, last-good snapshots, and a live soak monitor before trusting scheduler/auto-apply.

## What this adds

New scripts:

- `scripts/rust-authority-quarantine.sh`
- `scripts/rust-authority-last-good-snapshot.sh`
- `scripts/rust-authority-live-soak-monitor.sh`
- `scripts/verify-rust-live-stable-candidate.sh`

New runtime gate:

```text
rust_live_stable_gate_failed
```

New quarantine marker schema:

```text
lqosync.rust_authority_quarantine.v1
```

New last-good snapshot schema:

```text
lqosync.rust_authority_last_good.v1
```

## Live-stable flags

Promotion can enable:

```json
{
  "rust_live_stable_candidate_enabled": true,
  "rust_live_stable_fail_closed": true,
  "rust_live_stable_require_watchdog": true,
  "rust_live_stable_require_recovery_bundle": true,
  "rust_authority_quarantine_enabled": true,
  "rust_authority_auto_quarantine_on_failure": true,
  "rust_authority_quarantine_state": "/opt/LQoSync/state/rust_authority_quarantine.json",
  "rust_authority_last_good_snapshot_dir": "/opt/LQoSync/state/rust_authority_last_good"
}
```

Package defaults keep live-stable candidate disabled until promotion, so upgrades do not unexpectedly block an existing install.

## Quarantine behavior

When enabled, critical Rust authority failures write a quarantine marker and the live-stable gate blocks later production mutation until the operator reviews and clears it.

Critical statuses include:

```text
rust_authority_preflight_required_failed
rust_authority_watchdog_required_failed
rust_authoritative_apply_failed
rust_authoritative_journal_failed
rust_full_authority_file_write_not_executed
rust_full_authority_libreqos_apply_not_executed
libreqos_failed
```

Check status:

```bash
bash scripts/rust-authority-quarantine.sh status
```

Manually enter quarantine:

```bash
bash scripts/rust-authority-quarantine.sh enter operator_hold
```

Clear after review:

```bash
bash scripts/rust-authority-quarantine.sh clear reviewed_and_safe
```

## Last-good snapshot

Create a baseline snapshot before scheduler/auto-apply is trusted:

```bash
bash scripts/rust-authority-last-good-snapshot.sh
```

Snapshots are stored under:

```text
/opt/LQoSync/state/rust_authority_last_good/<timestamp>/MANIFEST.json
```

## Live soak monitor

Run after promotion and during soak testing:

```bash
bash scripts/rust-authority-live-soak-monitor.sh
```

It checks:

- full Rust authority flags;
- Python mutation fallback disabled;
- quarantine is clear;
- recovery bundle exists;
- last-good snapshot exists;
- preflight stamp is fresh;
- transaction journal path is readable;
- runtime state is not in error.

## Recommended live-stable candidate flow

```bash
cd /opt/LQoSync
bash scripts/build-rust-core.sh
sudo bash scripts/install-rust-core.sh
sudo bash scripts/install-rust-core-daemon.sh
printf '{"version":"1","op":"self-test","payload":{}}' | lqosync-core
sudo bash scripts/promote-rust-full-authoritative-safe.sh
bash scripts/verify-rust-live-stable-candidate.sh
bash scripts/rust-authority-live-soak-monitor.sh
```

Then run dry-run/manual apply first. Enable scheduler only after clean cycles.

## No-breakage boundary

- Python still owns WebUI and scheduler shell.
- Python RouterOS transport remains available.
- Rust owns production mutation only after promotion.
- Live-stable candidate gate is disabled by default and enabled by the promotion script.
- Quarantine marker is non-destructive and does not restore/overwrite files by itself.
