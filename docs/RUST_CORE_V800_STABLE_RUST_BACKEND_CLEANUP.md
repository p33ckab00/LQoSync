# v8.0.0 Rust Backend Stable Cleanup

This release declares the production backend authority as Rust-owned and treats Python as a WebUI/scheduler compatibility shell only.

## Stable authority boundary

Rust owns the production mutation path:

- validation and sync-plan enforcement;
- `ShapedDevices.csv` write authority;
- `network.json` write authority;
- LibreQoS apply execution authority;
- transaction journal authority;
- recovery/readiness evidence gates;
- quarantine on critical authority failure.

Python remains in the package only for:

- Flask WebUI and HTTP routes;
- scheduler/service shell;
- operator pages, docs, reports, and diagnostics;
- RouterOS transport compatibility where Rust validates collector output before mutation.

Python is **not** allowed to silently take over production mutation when full Rust authority is enabled. `python_mutation_fallback=false` is the stable default.

## Cleanup decision

No in-use Python module is deleted in this package unless it is provably disconnected from the runtime. The previous `applier`, `builders`, `collectors`, `rules`, `parsers`, and `engine` Python modules are still imported by the WebUI/scheduler shell and are therefore retained as compatibility/control-plane code.

The cleanup removes stale authority claims instead of breaking imports. Legacy Python backend authority is retired by configuration, runtime gates, verification scripts, and documentation.

## Stable release checks

Run:

```bash
bash scripts/verify-rust-stable-release-cleanup.sh
python3 scripts/release_check.py
python3 scripts/regression_check.py
python3 scripts/stable_release_check.py
```

Live Rust verification still must be run on the production host:

```bash
bash scripts/build-rust-core.sh
sudo bash scripts/install-rust-core.sh
sudo bash scripts/install-rust-core-daemon.sh
printf '{"version":"1","op":"self-test","payload":{}}' | lqosync-core
sudo bash scripts/promote-rust-full-authoritative-safe.sh
bash scripts/rust-set-and-forget-readiness.sh --write-stamp
bash scripts/verify-rust-stable-release-cleanup.sh
```

## Stable release status

This package is suitable for stable release tagging after the Rust binary self-test and live authority evidence pass on the target server.
