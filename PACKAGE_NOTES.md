# Package Notes

Package: v8.1.0 Rust Scheduler Authority Stable
Version: 2.151.0

This package aligns the codebase and documentation to the current project journey:

```text
Rust authority daemon + Flask WebUI shell
```

It is not Django and not SaaS.

## Key boundary

- Rust owns scheduler authority and production mutation.
- Flask owns the existing WebUI only.
- The old Python scheduler loop is disabled by default when `scheduler.engine=rust`.

## Required verification

```bash
bash scripts/verify-rust-scheduler-authority.sh
python3 scripts/release_check.py
python3 scripts/regression_check.py
python3 scripts/stable_release_check.py
```
