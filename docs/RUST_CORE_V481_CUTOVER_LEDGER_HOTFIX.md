# Rust Core v4.8.1 Cutover Ledger Hotfix

`rust/lqosync-core = 4.8.1`  
`LQoSync VERSION = 2.118.1-rc1`

## Summary

v4.8.1 fixes the v4.8.0 collector authority promotion cutover ledger unit-test failure.

The failing test expected:

```text
collector_authority_promotion_cutover_ledger_ready
```

but received:

```text
collector_authority_promotion_cutover_ledger_shadow_only
```

## Root cause

The test fixture exercised the cutover stage while rebuilding prerequisite stages through nested calls. Multiple prerequisite gates use distinct manual confirmation tokens, so reusing the root `confirmation` field can cause a nested prerequisite to remain shadow-only.

## Fix

The cutover ledger unit fixture now supplies the prerequisite v4.7 promotion commit plan explicitly, matching the self-test/API handoff pattern.

## Safety behavior

Unchanged:

```text
No live RouterOS reads
No Rust collector promotion
No cleanup authority transfer
No generated file writes
No LibreQoS apply authority
Python collector fallback remains mandatory
```

## Expected validation

```bash
bash scripts/repair-script-permissions.sh
bash scripts/build-rust-core.sh
sudo bash scripts/install-rust-core.sh
sudo bash scripts/install-rust-core-daemon.sh
printf '{"version":"1","op":"self-test","payload":{}}' | lqosync-core
```

Expected operation remains:

```text
build-collector-authority-promotion-cutover-ledger
```
