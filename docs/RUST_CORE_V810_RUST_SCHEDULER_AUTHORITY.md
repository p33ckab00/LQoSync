# v8.1.0 Rust Scheduler Authority

This release moves scheduler authority to the Rust daemon while keeping the existing Python Flask WebUI.

## Version

```text
LQoSync VERSION: 2.151.0
Release: v8.1.0 Rust Scheduler Authority
```

## Behavior

`lqosync-core.service` starts with scheduler authority enabled:

```text
/usr/local/bin/lqosync-core --daemon --socket /run/lqosync-core.sock --scheduler --config /opt/libreqos/src/config.json
```

The Flask app still creates a scheduler object for compatibility, but when config says:

```json
{
  "scheduler": {
    "engine": "rust",
    "allow_python_scheduler": false
  }
}
```

Flask does not start the old Python scheduler loop.

## Rust operations added

```text
scheduler-status
scheduler-heartbeat
scheduler-decision
scheduler-run-once
```

## Stable boundary

Python Flask remains the UI shell. Rust owns the scheduler decision, heartbeat, run authorization, and run-once execution command.

The run-cycle command is still invoked through the existing Python entrypoint for compatibility, but production mutation inside that cycle is already Rust-authoritative.

## Verification

```bash
bash scripts/verify-rust-scheduler-authority.sh
printf '{"version":"1","op":"self-test","payload":{}}' | lqosync-core
```

## Operator command

```bash
bash scripts/rust-scheduler-authority-status.sh
bash scripts/rust-scheduler-run-once.sh manual
```
