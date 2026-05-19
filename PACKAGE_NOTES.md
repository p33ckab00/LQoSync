# LQoSync Runtime Canonical Package

This package canonicalizes LQoSync naming across repository references, operator documentation, runtime service names, Docker container naming, logs, config defaults, and WebUI guidance.

## Canonical names

```text
GitHub repo:      https://github.com/p33ckab00/LQoSync.git
Install path:     /opt/lqosync
Systemd service:  lqosync
Docker container: lqosync
App log:          /opt/lqosync/logs/lqosync.log
System log:       /var/log/lqosync.log
Sudoers file:     /etc/sudoers.d/lqosync
```

## Update safety

The installer/updater keeps production safety behavior:

- backs up `/opt/libreqos/src/config.json`
- backs up `/opt/libreqos/src/ShapedDevices.csv`
- backs up `/opt/libreqos/src/network.json`
- preserves users, `.env`, state, logs, and backups
- creates missing files only by default
- normalizes Git remote to `p33ckab00/LQoSync`
- installs and starts the canonical `lqosync` runtime service

## Migration safety

The only remaining old runtime name references are internal migration variables in the install/update scripts. They are needed to safely stop/disable/remove the previous runtime unit during upgrade so the old and new services do not run at the same time.

After installation/update, operators should use only:

```bash
sudo systemctl status lqosync
sudo journalctl -u lqosync -n 100 --no-pager
sudo systemctl restart lqosync
```


## Rust branch documentation package

This package also includes documentation for the future `lqosync-in-rust` branch. It is documentation-only and does not change runtime behavior.

Included docs:

```text
docs/RUST_CORE_MIGRATION.md
docs/RUST_CORE_PROTOCOL.md
docs/COLLECTOR_OUTPUT_CONTRACT.md
docs/AUTOSAVE_AND_ATOMIC_STATE.md
docs/COMMIT_AND_PUSH_GUIDE.md
docs/assets/lqosync_rust_migration_plan.svg
```

Recommended branch:

```bash
git checkout -b lqosync-in-rust
```

Recommended commit:

```bash
git commit -m "docs(rust): document LQoSync-in-Rust migration plan"
```
