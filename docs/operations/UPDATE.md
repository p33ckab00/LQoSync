# Update an installed shaper

This guide updates a shaper that already has lqosync3 and the web UI. Use it for a development build from the local clone, or for a build from GitHub.

For a new installation, refer to [INSTALLATION.md](../INSTALLATION.md). For the release procedure, refer to [RELEASE.md](../RELEASE.md).

> **CAUTION:** A new release can change the registry schema when it opens the registry. An older binary cannot open a newer schema. Do the backup in step 2 before the new binary starts. A rollback needs this backup.

> **CAUTION:** The service starts a cycle when it starts. In `apply` mode, the cycle can write the LibreQoS files and reload LibreQoS. Read `runtime.mode` in `/etc/lqosync3/lqosync3.toml` before you start the service.

## Parts of the update

| Part | Files | Service |
|---|---|---|
| Core | `/usr/local/bin/lqosync3`, `/etc/systemd/system/lqosync3.service` | `lqosync3.service` |
| Web UI | `/usr/local/libexec/lqosync3-web*`, `/usr/local/bin/lqosync3-web-admin`, `/usr/local/share/lqosync3-web/` | `lqosync3-web.service`, `lqosync3-web-broker.service` |

The update keeps the config, the secrets, the state, and the web accounts.

## A. Update from the local clone

Use this procedure for a development build. The clone is `/home/pi/LQoSync3.0`.

### Step 1: Build

Build as the normal user. The service continues to operate during the build.

```bash
cd /home/pi/LQoSync3.0
git status --short          # Make sure that the build has only the changes that you want.
cargo build --release --locked -p lqosync3
TRUNK=/home/pi/.cache/lqosync-web-tools/bin/trunk tools/frontend/build.sh
```

Check: both commands end without an error. `target/release/lqosync3` and `crates/web-ui/dist/bootstrap.js` exist.

### Step 2: Backup

First, open a root shell with `sudo -i`, and type the password. Then paste the commands. Do not paste `sudo -i` together with the commands: the password prompt reads the pasted lines, and the commands do not run.

The `systemctl stop` command stops the core service.

```bash
B=/var/backups/lqosync3/update-$(date +%Y%m%d%H%M)
install -d -m 0700 "$B"
systemctl stop lqosync3
cp -a /usr/local/bin/lqosync3 /etc/systemd/system/lqosync3.service "$B"/
cp -a /etc/lqosync3 "$B"/etc-lqosync3
cp -a /var/lib/lqosync3 "$B"/state
cp -a /opt/libreqos/src/network.json /opt/libreqos/src/ShapedDevices.csv "$B"/
echo "$B"
```

Check: the last command shows the folder, and `systemctl is-active lqosync3` shows `inactive`. If the service is still active, stop here: the commands did not run.

Write down the value of `$B`. The rollback needs it.

### Step 3: Install the core

```bash
cd /home/pi/LQoSync3.0
install -m 0755 target/release/lqosync3 /usr/local/bin/lqosync3
install -m 0644 systemd/lqosync3.service /etc/systemd/system/lqosync3.service
systemctl daemon-reload
lqosync3 check
```

Check: `lqosync3 check` shows no `ERROR` line. If it shows an `ERROR` line, do not start the service. Correct the config, or do the rollback.

```bash
systemctl start lqosync3
systemctl show lqosync3 -p MainPID -p ActiveEnterTimestamp
```

Check: the start time is now. `lqosync3 version` shows the version of the file on disk only. It does not show the version of the running process.

### Step 4: Install the web UI

```bash
cd /home/pi/LQoSync3.0
tools/frontend/install.sh
```

The script stops and starts the two web services. It keeps the files `gateway.env` and `broker.env`.

### Step 5: Check

```bash
systemctl status lqosync3 lqosync3-web lqosync3-web-broker --no-pager
journalctl -u lqosync3 -n 50 --no-pager
```

Check:

1. The three services are `active (running)`.
2. The log shows no error after the first cycle.
3. The web UI opens. The TUI Versions card shows the same version for the service and the TUI.

## B. Update from GitHub

### B1. Build the source from GitHub

Get the branch from GitHub, then do procedure A from step 1:

```bash
cd /home/pi/LQoSync3.0
git fetch origin
git status --short           # The worktree must be clean before the pull.
git pull --ff-only origin lqosync3
git log --oneline -1         # Write down the commit.
```

To build one release, use `git checkout v3.0.N` in place of the pull.

### B2. Install a release binary

The installer downloads the release binary from GitHub. It updates the core only. It needs no clone and no Rust.

1. Do step 2 of procedure A (backup).
2. Download the installer, read it, then run it:

   ```bash
   curl -fsSL -o /tmp/install.sh https://raw.githubusercontent.com/p33ckab00/LQoSync/lqosync3/install.sh
   less /tmp/install.sh
   sudo bash /tmp/install.sh --version v3.0.N
   ```

   Without `--version`, the installer installs the newest `v3` release.

3. A release has no web UI files. To update the web UI, run `git checkout v3.0.N` in the clone, then do this:

   ```bash
   cd /home/pi/LQoSync3.0
   TRUNK=/home/pi/.cache/lqosync-web-tools/bin/trunk tools/frontend/build.sh
   sudo tools/frontend/install.sh
   ```

4. Do step 5 of procedure A.

## Rollback

Do these commands as root. `$B` is the backup folder from step 2.

```bash
systemctl stop lqosync3
cp -a "$B"/lqosync3 /usr/local/bin/lqosync3
cp -a "$B"/lqosync3.service /etc/systemd/system/lqosync3.service
rm -rf /var/lib/lqosync3
cp -a "$B"/state /var/lib/lqosync3
systemctl daemon-reload
systemctl start lqosync3
lqosync3 version
```

The state copy puts back the registry with its earlier schema. The earlier binary cannot open a registry with a newer schema.

The state copy also removes the registry changes after the backup, for example canvas edits. Do the rollback soon after the update.

To roll back the web UI, run `git checkout` with the earlier commit in the clone. Then do steps 1 and 4 of procedure A again.

NOTE: `install.sh --version` alone is not a full rollback. It replaces the binary only. It does not put back the registry.
