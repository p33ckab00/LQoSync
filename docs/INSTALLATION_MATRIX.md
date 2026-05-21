# LQoSync Installation / Update / Uninstall Matrix

This is the canonical operator matrix for LQoSync v7.8.1+.

## Canonical paths

| Purpose | Path |
|---|---|
| LQoSync app/runtime | `/opt/LQoSync` |
| LibreQoS app | `/opt/libreqos` |
| LibreQoS source/config path | `/opt/libreqos/src` |
| Config | `/opt/libreqos/src/config.json` |
| ShapedDevices output | `/opt/libreqos/src/ShapedDevices.csv` |
| Network topology output | `/opt/libreqos/src/network.json` |
| Systemd unit | `lqosync.service` |
| Docker container | `lqosync` |

Do not use `/home/pi/lqosync`, `/home/pi/lqosync_docker`, or `/opt/lqosync` as the final production source path. Those paths are legacy/staging working trees only.

## Deployment modes

| Mode | Install command | Update command | Uninstall command | Recommended for live LibreQoS? |
|---|---|---|---|---|
| Bare metal from ZIP | `sudo bash install-from-zip.sh` | `sudo bash update-from-zip.sh` | `sudo bash uninstall.sh` | Yes |
| Bare metal from GitHub | `sudo bash install-from-github.sh` | `sudo bash upgrade.sh` | `sudo bash uninstall.sh` | Yes |
| Production-safe local install | `sudo bash install-production-safe.sh` | apply ZIP/Git update then run same wrapper | `sudo bash uninstall.sh` | Best default |
| Full Rust authority install | `sudo bash install-rust-full-authoritative-safe.sh` | update code then `sudo bash scripts/promote-rust-full-authoritative-safe.sh` | `sudo bash uninstall.sh` | Yes after Rust self-test |
| Docker / Compose | `sudo bash install-docker.sh` | `sudo bash update-docker.sh` | `sudo bash uninstall-docker.sh` | Only if you accept host-integrated Docker |

## Safe defaults

For production, use:

```bash
LQOSYNC_INIT_POLICY=preserve_existing
LQOSYNC_SERVICE_START_POLICY=enable_only
```

This means installers preserve existing LibreQoS files and enable the systemd unit without starting scheduler-capable runtime until the operator reviews config and dry-run output.

## Init policy values

| Value | Meaning |
|---|---|
| `preserve_existing` | Keep existing `config.json`, `ShapedDevices.csv`, and `network.json`; create missing files only. Recommended for live. |
| `create_missing_only` | Create only missing files. Existing files are untouched. |
| `overwrite_with_backup` | Backup existing files, then replace from package templates. Use only for lab/factory reset. |
| `smart_confirm` | Interactive prompt when existing files are detected. Non-interactive falls back to preserve. |

## Service start policy values

| Value | Meaning |
|---|---|
| `restart` | Enable and restart `lqosync`. Historical behavior. |
| `enable_only` | Enable the service but do not start/restart it. Recommended for live updates. |
| `leave_stopped` | Stop and disable the service after install/update. Useful for offline staging. |

## Live promotion sequence

```bash
cd /opt/LQoSync
bash scripts/build-rust-core.sh
sudo bash scripts/install-rust-core.sh
sudo bash scripts/install-rust-core-daemon.sh
printf '{"version":"1","op":"self-test","payload":{}}' | lqosync-core
sudo bash scripts/promote-rust-full-authoritative-safe.sh
bash scripts/verify-rust-set-and-forget-candidate.sh
python3 scripts/release_check.py
python3 scripts/regression_check.py
```

Then start the UI/scheduler shell only after review:

```bash
sudo systemctl start lqosync
```

## Uninstall safety rule

Uninstall scripts remove LQoSync runtime/service integration by default. They do **not** delete LibreQoS working files unless you explicitly remove them yourself.


## Troubleshooting entry point

For real-world install/migration errors, see [Operator Troubleshooting Guide](OPERATOR_TROUBLESHOOTING.md).

Covered errors include missing Cargo, old Cargo/Cargo.lock v4, Git push rejection, non-fast-forward divergence, rebase conflict recovery, and old Python/main migration to `lqosync-in-rust`.
