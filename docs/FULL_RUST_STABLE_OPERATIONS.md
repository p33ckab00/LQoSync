# Full Rust Stable Operations Guide

Canonical install path: `/opt/LQoSync`  
Canonical LibreQoS source path: `/opt/libreqos/src`  
Canonical Rust daemon binary: `/usr/local/bin/lqosync-core`  
Canonical Rust daemon socket: `/run/lqosync-core.sock`

## Fresh bare-metal stable install

```bash
sudo bash install-rust-stable-safe.sh
```

The stable installer preserves live LibreQoS files, builds/installs Rust core when toolchain is present, promotes full Rust authority, creates recovery/readiness evidence, and leaves the Python WebUI/scheduler shell intact.

## Update from GitHub

```bash
cd /opt/LQoSync
git fetch origin lqosync-in-rust
git checkout lqosync-in-rust
git pull --ff-only origin lqosync-in-rust
sudo bash install-rust-stable-safe.sh
```

## Update from ZIP

```bash
mkdir -p /tmp/lqosync-stable
unzip LQoSync_runtime_canonical_FULL_rust_core_v800_stable_cleanup.zip -d /tmp/lqosync-stable
cd /tmp/lqosync-stable
sudo bash install-rust-stable-safe.sh
```

## Docker

Docker remains supported for lab/staging. For the live LibreQoS host, bare-metal/systemd is the recommended stable mode.

```bash
sudo bash install-docker.sh
sudo bash update-docker.sh
sudo bash uninstall-docker.sh
```

## Stable cleanup rule

Do not delete `/opt/LQoSync`, `/opt/libreqos`, `/usr/local/bin/lqosync-core`, `/etc/systemd/system/lqosync-core.service`, or `/run/lqosync-core.sock`.

Legacy duplicate working trees may be archived only through guarded cleanup scripts.
