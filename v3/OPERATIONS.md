# Run and control the LQoSync v3 console

This guide applies to `/opt/LQoSync3.0/v3` on the current server.
The console uses `127.0.0.1:9303` and the current `v3/data/` directory.
Keep this directory to preserve settings, credentials, and application state.

As of 2026-10-30, the live console uses a manual background process.
The repository includes a systemd template. The template is not installed automatically.

## Quick restart for the current server

Run these commands in the server terminal as `pi`.
Use this section for the current manual process. The systemd commands require service installation first.
A restart does not require a build unless you want to load code changes.

1. Stop the console:

```bash
pkill -TERM -f '^/opt/LQoSync3.0/v3/target/debug/lqosync-v3$'
```

If no console exists, this command returns status 1. Continue with the next check.

2. Check that the console stopped:

```bash
pgrep -af '^/opt/LQoSync3.0/v3/target/debug/lqosync-v3$'
```

Continue only when this command shows no process. If it shows a process, wait and check again.

3. Copy this complete block to start the console in the background:

```bash
cd /opt/LQoSync3.0/v3
LQOSYNC3_DATA=/opt/LQoSync3.0/v3/data LQOSYNC3_BIND=127.0.0.1:9303 nohup /opt/LQoSync3.0/v3/target/debug/lqosync-v3 >> /opt/LQoSync3.0/v3/data/state/web-console.log 2>&1 < /dev/null &
```

Wait two seconds. Then check the console:

```bash
curl --fail --silent --show-error http://127.0.0.1:9303/health
```

Expected result: `ok`. Open `http://127.0.0.1:9303` in the server browser.
If your browser runs on another computer, use the SSH tunnel in **Open the console** below.

If the check fails, read the recent log:

```bash
tail -n 30 /opt/LQoSync3.0/v3/data/state/web-console.log
```

`Address already in use` means another process uses the port. Complete the stop and process checks before another start.
If the log reports migration recovery, do not remove recovery markers.
Use the [read-only recovery viewer](SHADOW_CONTROLS.md) to inspect saved writer records.
Protected-row migrations use the separate [maintenance recovery procedure](CORE_MAINTENANCE.md).

To stop only, complete steps 1 and 2. The current manual setup does not start automatically after a reboot.
Core maintenance commands do not start or restart the console.

## Check the current setup

```bash
systemctl status lqosync-v3 --no-pager
pgrep -af '^/opt/LQoSync3.0/v3/target/debug/lqosync-v3$'
curl --fail --silent --show-error http://127.0.0.1:9303/health
```

Before you install the service, systemctl reports that the unit does not exist.
The process command shows the manual console PID.
A successful health request confirms an HTTP response. It does not validate inventory from routers or applied output.

## Build the executable

Use the current Rust installation. Do not run a second console on the same port.

```bash
cd /opt/LQoSync3.0/v3
CARGO_BUILD_JOBS=2 /home/pi/.cargo/bin/cargo build --locked --bin lqosync-v3
```

If the build fails, keep the current console process active.
A successful build does not update an active process. Restart the console to load the new executable.

## Control the current manual process

### Stop

Send SIGTERM to the exact console command:

```bash
pkill -TERM -f '^/opt/LQoSync3.0/v3/target/debug/lqosync-v3$'
pgrep -af '^/opt/LQoSync3.0/v3/target/debug/lqosync-v3$'
```

The second command must return no process before you start another console.
If the first command finds no process, its exit status is 1.

### Run in the foreground

Use this mode for diagnosis. Keep the terminal open.

```bash
cd /opt/LQoSync3.0/v3
LQOSYNC3_DATA=/opt/LQoSync3.0/v3/data \
LQOSYNC3_BIND=127.0.0.1:9303 \
/opt/LQoSync3.0/v3/target/debug/lqosync-v3
```

Press Ctrl+C to stop this foreground process.

### Run in the background

Start only after the previous console has stopped.

```bash
cd /opt/LQoSync3.0/v3
mkdir -p data/state
LQOSYNC3_DATA=/opt/LQoSync3.0/v3/data \
LQOSYNC3_BIND=127.0.0.1:9303 \
nohup /opt/LQoSync3.0/v3/target/debug/lqosync-v3 \
  >> data/state/web-console.log 2>&1 < /dev/null &
```

Check the result:

```bash
curl --fail --silent --show-error http://127.0.0.1:9303/health
tail -n 50 /opt/LQoSync3.0/v3/data/state/web-console.log
```

### Restart or disable

To restart, complete **Stop**, then **Run in the background**.
For code changes, build the executable first.

The manual process has no systemd boot configuration to disable.
Stop the process and do not start it again.
Use the service procedure below if you need automatic start after a server reboot.

## Install the systemd service once

These commands change the host and start the console.
The unit uses account `pi` and the current data directory.
It restarts the console after a process failure.

1. Build the executable with the command above.
2. Stop the manual console with the exact process command above.
3. Make sure that the process command returns no console PID.
4. Install the unit:

```bash
sudo install -m 0644 /opt/LQoSync3.0/v3/systemd/lqosync-v3.service /etc/systemd/system/lqosync-v3.service
sudo systemctl daemon-reload
sudo systemctl enable --now lqosync-v3
```

5. Check the service:

```bash
systemctl status lqosync-v3 --no-pager
curl --fail --silent --show-error http://127.0.0.1:9303/health
```

Do not use the manual start command after service installation.
Use systemctl. Only one console process can use port 9303.

## Control the installed service

| Task | Command | Result |
| --- | --- | --- |
| Run now | `sudo systemctl start lqosync-v3` | Start the console now. |
| Restart | `sudo systemctl restart lqosync-v3` | Stop and start the console with the current executable. |
| Stop now | `sudo systemctl stop lqosync-v3` | Stop now. Keep the boot configuration. |
| Enable at boot | `sudo systemctl enable lqosync-v3` | Start after the next boot. Do not start now. |
| Enable and run | `sudo systemctl enable --now lqosync-v3` | Start now and after each boot. |
| Disable at boot | `sudo systemctl disable lqosync-v3` | Remove automatic boot start. Keep the current process active. |
| Stop and disable | `sudo systemctl disable --now lqosync-v3` | Stop now and remove automatic boot start. |
| Check status | `systemctl status lqosync-v3 --no-pager` | Show the service state. |
| Check boot configuration | `systemctl is-enabled lqosync-v3` | Show enabled or disabled. |
| Show recent logs | `journalctl -u lqosync-v3 -n 100 --no-pager` | Show service logs. |
| Follow logs | `journalctl -u lqosync-v3 -f` | Show new log messages. Press Ctrl+C to exit. |

If you cannot read the journal, repeat the journal command with sudo.
Service logs use the journal. Earlier manual logs remain in `data/state/web-console.log`.
An explicit systemctl stop does not trigger the unit's failure restart.

## Open the console

On the server, open `http://127.0.0.1:9303`.
For another computer, use an SSH tunnel:

```bash
ssh -L 9303:127.0.0.1:9303 pi@SERVER
```

Replace `SERVER` with the server address. Open the same loopback URL on your computer.
The console has no user login. Keep the bind address on loopback.

## Check after a restart

1. Check `/health` and the process or service status.
2. Open the console. Refresh the browser to load the current CSS and JavaScript.
3. Check router collection and validation before you trust new candidate output.

The console starts without active inventory. Selected sources and mapped WAN routers resume collection.
Candidate, staged, and applied output remain separate.
A healthy console does not prove that production shaping changed.

The console commands do not control `lqosync-updatecsv.service`, `lqosd.service`, or other LibreQoS services.
These separate services stay active when you stop this console.

## Core maintenance

See [the maintenance CLI guide](CORE_MAINTENANCE.md) for explicit protected-row migration and recovery.
The maintenance CLI requires every console for the same data directory to stop first.
Do not run maintenance commands to restart the console.
