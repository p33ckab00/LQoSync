# One-Line Operations Guide

LQoSync is a local appliance-style app: Rust backend authority daemon plus Flask WebUI shell.

Use `lqosyncctl.sh` for fresh install, update, uninstall, permission adoption, check, verify, start, stop, restart, and repair. The script handles the common live-server problems found during deployment:

- Git `dubious ownership` by adding `/opt/LQoSync` as a safe directory for root.
- Missing or old Cargo by installing/updating Rust stable with rustup.
- Missing ACL tooling by installing `acl` before standalone permission adoption.
- GitHub branch updates from `lqosync-in-rust`.
- Preserve-existing live LibreQoS files.
- Service start policy defaults that avoid surprise WebUI/scheduler starts during install.

## Fresh install from GitHub

```bash
curl -fsSL https://raw.githubusercontent.com/p33ckab00/LQoSync/lqosync-in-rust/lqosyncctl.sh | sudo bash -s -- install
```

## Update from GitHub

```bash
curl -fsSL https://raw.githubusercontent.com/p33ckab00/LQoSync/lqosync-in-rust/lqosyncctl.sh | sudo bash -s -- update
```

## Uninstall LQoSync runtime integration safely

```bash
curl -fsSL https://raw.githubusercontent.com/p33ckab00/LQoSync/lqosync-in-rust/lqosyncctl.sh | sudo bash -s -- uninstall
```

## Adopt the runtime user, ownership, ACLs, and managed-file permissions

```bash
curl -fsSL https://raw.githubusercontent.com/p33ckab00/LQoSync/lqosync-in-rust/lqosyncctl.sh | sudo bash -s -- adopt
```

Standalone `adopt` ensures ACL tooling is present, creates the `lqosync` system user if needed, grants the runtime user access to managed LibreQoS files, and tests temporary-file creation in `/opt/libreqos/src`.

## Check current server status

```bash
curl -fsSL https://raw.githubusercontent.com/p33ckab00/LQoSync/lqosync-in-rust/lqosyncctl.sh | sudo bash -s -- check
```

Or, if already installed:

```bash
sudo /opt/LQoSync/lqosyncctl.sh check
```

## Verify package and Rust authority wiring

```bash
curl -fsSL https://raw.githubusercontent.com/p33ckab00/LQoSync/lqosync-in-rust/lqosyncctl.sh | sudo bash -s -- verify
```

Or locally:

```bash
sudo /opt/LQoSync/lqosyncctl.sh verify
```

## Start services

```bash
sudo /opt/LQoSync/lqosyncctl.sh start
```

## Restart services

```bash
sudo /opt/LQoSync/lqosyncctl.sh restart
```

## Stop services

```bash
sudo /opt/LQoSync/lqosyncctl.sh stop
```

## Repair local install without Git update

```bash
sudo /opt/LQoSync/lqosyncctl.sh repair
```

## Expected WebUI port

```text
http://<server-ip>:9202
```

Rust backend authority daemon uses Unix socket:

```text
/run/lqosync-core.sock
```

## Dry-run Internal Server Error hardening

Starting v8.2.5, `/sync/dry-run` catches dry-run exceptions and renders an operator diagnostic card instead of a raw Flask Internal Server Error page. Check logs with:

```bash
sudo journalctl -u lqosync -n 120 --no-pager -l
sudo /opt/LQoSync/lqosyncctl.sh verify
```
