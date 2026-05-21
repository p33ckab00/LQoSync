# v8.2.1 Operator Error Runbook

This release adds documentation for real errors encountered during live migration.

## Covered incidents

- Existing Python/main install migrated to `lqosync-in-rust`.
- `Rust core requested but cargo is not installed`.
- `Cargo.lock` version 4 with old Ubuntu Cargo.
- `git push` rejected with `fetch first` / `non-fast-forward`.
- Rebase conflict/detached HEAD recovery.
- Production-safe service policy `enable_only`.

## Canonical troubleshooting entry point

Use:

```text
docs/OPERATOR_TROUBLESHOOTING.md
```

## Quick Rust toolchain fix

```bash
apt update
apt install -y curl build-essential pkg-config libssl-dev
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
source /root/.cargo/env
rustup default stable
rustup update stable
export PATH="/root/.cargo/bin:$PATH"
which cargo
cargo --version
```

## Quick Git recovery rule

Never force-push first. Create a rescue branch, fetch, rebase or reset to remote, then recommit cleanly.

```bash
git branch backup/before-git-recovery-$(date +%Y%m%d-%H%M%S) HEAD
git fetch origin lqosync-in-rust
```

See the full troubleshooting guide for the exact path based on your error.
