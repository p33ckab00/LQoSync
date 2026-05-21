# Bare Metal Install

Preferred for live LibreQoS.

```bash
sudo apt update
sudo apt install -y git curl build-essential pkg-config libssl-dev python3 python3-venv python3-pip
cd /opt/LQoSync
sudo bash install-rust-stable-safe.sh
```

Start services after review:

```bash
sudo systemctl start lqosync-core
sudo systemctl start lqosync
```

Check:

```bash
systemctl status lqosync-core lqosync --no-pager
bash scripts/rust-scheduler-authority-status.sh
```
