# Rust Core v4.4.1 — Pilot Result Recursion Hotfix

Version: `2.114.1-rc1`  
Rust core: `lqosync-core 4.4.1`

## Purpose

This hotfix fixes a Rust compile-time recursion-limit failure in the v4.4 collector authority pilot result evaluator tests.

## Fix

The large nested `serde_json::json!({ ... })` object used to build the `rust_core` test gate payload was replaced with incremental `serde_json::Map` construction.

This prevents:

```text
recursion limit reached while expanding `$crate::json_internal!`
```

## Safety status

Runtime behavior remains unchanged:

- no live RouterOS reads
- no collector authority switch
- no cleanup authority transfer
- no generated file writes
- no LibreQoS apply authority
- Python collectors remain authoritative

## Expected validation

Run:

```bash
bash scripts/repair-script-permissions.sh
bash scripts/build-rust-core.sh
sudo bash scripts/install-rust-core.sh
sudo bash scripts/install-rust-core-daemon.sh
printf '{"version":"1","op":"self-test","payload":{}}' | lqosync-core
```

Expected operation:

```text
evaluate-collector-authority-pilot-result
```
