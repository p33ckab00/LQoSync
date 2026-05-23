# Full Rust Stable Operations

Current stable architecture:

```text
lqosync-core.service = Rust authority daemon
lqosync web service = Flask WebUI shell
```

## Start

```bash
sudo systemctl start lqosync-core
sudo systemctl start lqosync
```

## Check Rust authority

```bash
printf '{"version":"1","op":"self-test","payload":{}}' | lqosync-core
bash scripts/rust-scheduler-authority-status.sh
```

## Run one manual cycle through Rust scheduler authority

```bash
bash scripts/rust-scheduler-run-once.sh manual
```

## Run the native Rust run-cycle authority directly

```bash
bash scripts/rust-run-cycle-authority.sh manual
```

## Rust native dry-run preview

The WebUI/API now uses a Rust-backed dry-run preview path. The legacy
`engine.run_cycle(mode="dry_run")` helper also uses the same Rust-native path
when this flag is enabled:

```json
{
  "rust_core": {
    "native_dry_run_preview_enabled": true
  }
}
```

This still requires the guarded RouterOS live-read adapter gates if you want real
live MikroTik reads. The preview path is intentionally read-only: Flask now sends
one `build-rust-native-dry-run-preview` request into `lqosync-core`, which then
builds the RouterOS plan, optional live reads, shadow collector rows, shadow
`network.json`, current artifact parity, validation, sync-plan preview, and
apply-manifest preview without writing files or running `LibreQoS.py`. When the
flag is enabled, `engine.run_cycle(mode="dry_run")` now short-circuits into the
same Rust-native path instead of running the old Python dry-run collector loop.
Outside that legacy helper path, the regular `engine.run_cycle` shadow phase now also
delegates its diff/validation/policy/sync-plan/apply-manifest preview bundle to
Rust through `build-rust-sync-engine-shadow-preview`.

## Native run-cycle authority

Stable full-Rust run-cycle entry now uses these flags:

```json
{
  "rust_core": {
    "native_run_cycle_authority_enabled": true,
    "native_run_cycle_authority_python_fallback": false
  }
}
```

If RouterOS live-read gates are still disabled, the native authority path fails
closed instead of silently dropping back to the legacy Python backend. The old
`scripts/run_cycle_once.py` bridge has been removed from the scheduled/manual
execution path.

## Production safety chain

Rust authority requires:

- self-test
- preflight stamp
- watchdog
- recovery bundle
- live-stable gate
- set-and-forget readiness
- transaction journal
- rollback drill
- scheduler authority

## Flask role

Flask is an operator UI. It must not silently mutate production files outside Rust authority.

## Python legacy cleanup

Run `build-python-legacy-retirement-inventory` before any final cleanup of Python backend remnants. The Rust inventory preserves Flask WebUI shell paths, marks backend-only Python paths as guarded archive candidates, and keeps deletion disabled by design.
