# lqosync-core

`lqosync-core` is the optional Rust safety sidecar for LQoSync.

Current scope:

- stable JSON protocol envelope
- bandwidth parser
- ShapedDevices.csv parser/render validator
- network.json parser/tree validator
- config/policy action validator
- collector output trust validator

Python remains the WebUI and orchestrator. If this binary is missing, Python uses
the existing validation path and records a fallback status.

## Build

```bash
scripts/build-rust-core.sh
```

## Install optional binary

```bash
sudo scripts/install-rust-core.sh
```

## Example request

```bash
printf '%s' '{"version":"1","op":"parse-bandwidth","payload":{"parser":"rate_limit","value":"10M/5M"}}' \
  | rust/lqosync-core/target/release/lqosync-core
```
