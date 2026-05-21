# Operator Troubleshooting Guide

This guide captures real operator errors observed during migration from the old Python/main LQoSync install to the current `lqosync-in-rust` stable branch.

Canonical project boundary:

```text
LQoSync is a local appliance-style web app.
Rust authority daemon = backend authority.
Python Flask = WebUI shell only.
No Django. No SaaS. No Python scheduler authority.
```

## Start with the error message

| Error / symptom | Meaning | Start here |
|---|---|---|
| `Rust core requested but cargo is not installed` | Rust toolchain is missing. Flask install may have succeeded, but Rust daemon build did not run. | [Rust/Cargo errors](#rustcargo-errors) |
| `lock file version 4 requires -Znext-lockfile-bump` | The system Cargo is too old for the packaged `Cargo.lock`. | [Cargo.lock version 4](#cargolock-version-4-requires--znext-lockfile-bump) |
| `git push rejected (fetch first)` | Remote branch has newer commits than the local branch. | [Git push rejected](#git-push-rejected-fetch-first) |
| `non-fast-forward` | Local branch is behind or diverged from GitHub. | [Git branch diverged](#git-branch-diverged--non-fast-forward) |
| `interactive rebase in progress` | A failed/conflicted rebase is still active. | [Rebase conflict recovery](#rebase-conflict-recovery) |
| `Service policy: enabled but not started/restarted` | This is intentional for production-safe installs. | [Service not started](#service-enabled-but-not-started) |
| WebUI opens but scheduler does not run | Rust scheduler authority may not be started/promoted. | [Rust scheduler service checks](#rust-scheduler-service-checks) |

## Golden recovery rule

Do not force-push and do not delete live LibreQoS files when troubleshooting.

Always preserve:

```text
/opt/libreqos/src/config.json
/opt/libreqos/src/ShapedDevices.csv
/opt/libreqos/src/network.json
/opt/LQoSync/state
/opt/LQoSync/.env
```

Production-safe install/update should use:

```bash
cd /opt/LQoSync
LQOSYNC_INIT_POLICY=preserve_existing \
LQOSYNC_SERVICE_START_POLICY=enable_only \
bash install-rust-stable-safe.sh
```

## Rust/Cargo errors

### `Rust core requested but cargo is not installed`

Meaning: the Python/Flask shell and installer completed enough to preserve files, install dependencies, and write systemd units, but Rust cannot build because Cargo is missing.

Fix:

```bash
cd /opt/LQoSync
apt update
apt install -y curl build-essential pkg-config libssl-dev
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
source /root/.cargo/env
rustup default stable
rustup update stable
export PATH="/root/.cargo/bin:$PATH"
which cargo
cargo --version
rustc --version
```

Expected:

```text
/root/.cargo/bin/cargo
```

Then rerun:

```bash
cd /opt/LQoSync
export PATH="/root/.cargo/bin:$PATH"
LQOSYNC_INIT_POLICY=preserve_existing \
LQOSYNC_SERVICE_START_POLICY=enable_only \
bash install-rust-stable-safe.sh
```

### `Cargo.lock version 4 requires -Znext-lockfile-bump`

Meaning: Ubuntu's packaged `cargo` is older than the lockfile format used by this Rust branch.

Fix by using rustup Cargo instead of `/usr/bin/cargo`:

```bash
source /root/.cargo/env 2>/dev/null || true
export PATH="/root/.cargo/bin:/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin"
hash -r
which cargo
cargo --version
```

If `which cargo` still shows `/usr/bin/cargo`, install rustup:

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
source /root/.cargo/env
rustup default stable
rustup update stable
```

Then build:

```bash
cd /opt/LQoSync
bash scripts/build-rust-core.sh
bash scripts/install-rust-core.sh
bash scripts/install-rust-core-daemon.sh
printf '{"version":"1","op":"self-test","payload":{}}' | lqosync-core
```

## Git errors

### `git push rejected (fetch first)`

Meaning: GitHub has newer commits than your local branch.

Safe fix:

```bash
cd /opt/LQoSync
git branch backup/before-fetch-$(date +%Y%m%d-%H%M%S) HEAD
git fetch origin lqosync-in-rust
git rebase origin/lqosync-in-rust
git push -u origin lqosync-in-rust
```

### Git branch diverged / `non-fast-forward`

If local and remote diverged and you want to apply the current package cleanly on top of GitHub:

```bash
cd /opt/LQoSync
git branch backup/before-clean-apply-$(date +%Y%m%d-%H%M%S) HEAD
git fetch origin lqosync-in-rust
git switch lqosync-in-rust
git reset --hard origin/lqosync-in-rust

# apply the extracted release package
rsync -av --delete --exclude='.git' /tmp/lqosync-release/ /opt/LQoSync/

git add -A
git commit -m "chore(docs): update operator troubleshooting guide"
git push -u origin lqosync-in-rust
```

Do not use `git push --force` unless you explicitly intend to rewrite public branch history.

### Rebase conflict recovery

If you see:

```text
interactive rebase in progress
CONFLICT (content)
```

First rescue your commits:

```bash
cd /opt/LQoSync
git branch rescue/rebase-state-$(date +%Y%m%d-%H%M%S) HEAD
```

Abort the broken rebase:

```bash
git rebase --abort || true
git switch lqosync-in-rust
```

Then either rebase again carefully, or reset to remote and apply the release package cleanly.

## Service enabled but not started

This is expected for safe live installs:

```text
Service policy: enabled but not started/restarted
```

Reason: a preserved config could have scheduler enabled. The installer avoids starting runtime automatically until the operator reviews config and runs a dry run.

Start only after Rust build/self-test and promotion pass:

```bash
systemctl start lqosync-core
systemctl start lqosync
systemctl status lqosync-core --no-pager
systemctl status lqosync --no-pager
```

## Rust scheduler service checks

```bash
cd /opt/LQoSync
bash scripts/verify-full-rust-daemon-boundary.sh
bash scripts/verify-rust-scheduler-authority.sh
bash scripts/rust-scheduler-authority-status.sh
bash scripts/rust-authority-watchdog.sh
bash scripts/rust-set-and-forget-readiness.sh
```

Expected architecture:

```text
Rust = scheduler/backend authority
Flask = WebUI shell
Python scheduler loop = removed
Python mutation fallback = disabled
```

## Migration from old Python/main install

Use a clean migration when coming from old Python/main:

```bash
systemctl stop lqosync 2>/dev/null || true
systemctl stop lqos_shaped_sync 2>/dev/null || true

mkdir -p /root/lqosync-migration-backup/$(date +%Y%m%d-%H%M%S)
BACKUP_DIR="/root/lqosync-migration-backup/$(date +%Y%m%d-%H%M%S)"
mkdir -p "$BACKUP_DIR"
cp -a /opt/libreqos/src/config.json "$BACKUP_DIR/config.json" 2>/dev/null || true
cp -a /opt/libreqos/src/ShapedDevices.csv "$BACKUP_DIR/ShapedDevices.csv" 2>/dev/null || true
cp -a /opt/libreqos/src/network.json "$BACKUP_DIR/network.json" 2>/dev/null || true

mv /opt/LQoSync /opt/LQoSync.legacy-python-main.$(date +%Y%m%d-%H%M%S) 2>/dev/null || true
git clone https://github.com/p33ckab00/LQoSync.git /opt/LQoSync
cd /opt/LQoSync
git branch --show-current
```

Expected branch:

```text
lqosync-in-rust
```

Then:

```bash
LQOSYNC_INIT_POLICY=preserve_existing \
LQOSYNC_SERVICE_START_POLICY=enable_only \
bash install-rust-stable-safe.sh
```

## Final live cutover checklist

```bash
cd /opt/LQoSync
bash scripts/build-rust-core.sh
bash scripts/install-rust-core.sh
bash scripts/install-rust-core-daemon.sh
printf '{"version":"1","op":"self-test","payload":{}}' | lqosync-core

bash scripts/promote-rust-full-authoritative-safe.sh
bash scripts/verify-full-rust-daemon-boundary.sh
bash scripts/verify-rust-scheduler-authority.sh
python3 scripts/release_check.py
python3 scripts/regression_check.py
python3 scripts/stable_release_check.py
```

Then:

```bash
systemctl start lqosync-core
systemctl start lqosync
```

First production action should be:

```text
WebUI → Dry Run → review diff → Manual Apply → check Operations Center → then scheduler
```
