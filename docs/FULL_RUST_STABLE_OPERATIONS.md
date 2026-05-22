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
