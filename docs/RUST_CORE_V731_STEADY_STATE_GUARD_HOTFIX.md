# Rust Core v7.3.1 Steady-State Guard Hotfix

`rust/lqosync-core = 7.3.1`  
`LQoSync VERSION = 2.143.1-rc1`

## Summary

v7.3.1 fixes the v7.3.0 `self_test::tests::self_test_passes` failure in the full Rust backend steady-state guard.

The v7.3.0 unit tests passed individually, but the aggregate self-test fixture missed the `webui_static_assets_preserved=true` gate when exercising `build-full-rust-backend-steady-state-guard`. Because the guard requires WebUI/UX/static assets to remain unchanged after Python retirement, the aggregate self-test correctly blocked steady-state verification.

## Fix

The self-test steady-state payload now explicitly includes:

```text
webui_static_assets_preserved = true
```

This aligns the aggregate self-test with the standalone steady-state guard test and with the production requirement that the WebUI/UX/static assets remain as-is.

## Safety behavior

Unchanged:

```text
WebUI/UX/static assets preserved
Python drift must remain absent
Rust backend must remain authoritative
Rollback package must remain available
No blind deletion or service mutation by Rust core self-test
```
