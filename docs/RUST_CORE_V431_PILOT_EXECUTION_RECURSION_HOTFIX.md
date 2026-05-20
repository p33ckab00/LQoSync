# Rust Core v4.3.1 — Collector Authority Pilot Execution Recursion Hotfix

## Purpose

This hotfix fixes a compile-time Rust macro recursion error in `collector_authority_pilot_execution.rs`.

The v4.3 implementation built one large nested `serde_json::json!({...})` response object. On the server build, Rust failed with:

```text
recursion limit reached while expanding `$crate::json_internal!`
```

v4.3.1 replaces the large response macro with incremental `serde_json::Map` construction.

## Safety behavior

Runtime behavior is unchanged:

- No live RouterOS reads
- No collector authority switch
- No cleanup authority transfer
- No generated file writes
- No LibreQoS apply authority
- Python collector fallback remains mandatory

## Version

- LQoSync `2.113.1-rc1`
- Rust core `4.3.1`

## Validation

Run:

```bash
bash scripts/repair-script-permissions.sh
bash scripts/build-rust-core.sh
sudo bash scripts/install-rust-core.sh
sudo bash scripts/install-rust-core-daemon.sh
printf '{"version":"1","op":"self-test","payload":{}}' | lqosync-core
```

Expected operation remains:

```text
build-collector-authority-pilot-execution-contract
```
