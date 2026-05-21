# Git Install / Update

```bash
sudo git clone --branch lqosync-in-rust https://github.com/p33ckab00/LQoSync.git /opt/LQoSync
cd /opt/LQoSync
sudo bash install-rust-stable-safe.sh
```

Update:

```bash
cd /opt/LQoSync
git fetch origin lqosync-in-rust
git reset --hard origin/lqosync-in-rust
sudo bash install-rust-stable-safe.sh
```
