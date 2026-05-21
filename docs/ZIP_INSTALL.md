# ZIP Install and Update Guide

Use this guide when you downloaded a release ZIP instead of cloning GitHub.

## Fresh or live-safe install from ZIP

```bash
sudo apt update
sudo apt install -y unzip rsync
mkdir -p /tmp/lqosync-package
unzip LQoSync_runtime_canonical_FULL_rust_core_*.zip -d /tmp/lqosync-package
cd /tmp/lqosync-package
sudo bash install-from-zip.sh
```

Default ZIP install behavior is production-safe:

```text
LQOSYNC_INIT_POLICY=preserve_existing
LQOSYNC_SERVICE_START_POLICY=enable_only
```

Start manually after config review and dry-run:

```bash
sudo systemctl start lqosync
```

## Update existing install from ZIP

```bash
sudo apt update
sudo apt install -y unzip rsync
mkdir -p /tmp/lqosync-update
unzip LQoSync_runtime_canonical_FULL_rust_core_*.zip -d /tmp/lqosync-update
cd /tmp/lqosync-update
sudo bash update-from-zip.sh
```

This backs up the current app/runtime state and live LibreQoS files, then refreshes `/opt/LQoSync` from the ZIP while preserving operator files such as `.env`, `users.json`, `state`, `logs`, and backups.

## Start policy override

To restart immediately after ZIP install/update:

```bash
sudo LQOSYNC_SERVICE_START_POLICY=restart bash install-from-zip.sh
sudo LQOSYNC_SERVICE_START_POLICY=restart bash update-from-zip.sh
```

For offline staging:

```bash
sudo LQOSYNC_SERVICE_START_POLICY=leave_stopped bash update-from-zip.sh
```
