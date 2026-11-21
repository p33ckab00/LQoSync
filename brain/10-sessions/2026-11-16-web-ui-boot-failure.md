---
type: session
created: 2026-11-16
tags: [libreqos, web-ui, boot, investigation]
status: diagnosis-complete
related: ["[[HANDOFF]]", "[[2026-11-16-native-recovery-path-review]]"]
---

# Web UI boot failure

Reviewed: 2026-11-16T11:11:22+08:00 (Asia/Shanghai). Agent: Codex `/root`.
The owner requested diagnosis before changes.
Branch: `lqosync3`. Worktree: `/home/pi/LQoSync3.0`. HEAD: `0ad8b8d`.
Existing unrelated changes remain intact.

## Evidence

- Public HTTPS and local Traefik with the LibreQoS Host header return HTTP 502.
- `/etc/libreqos/lqos.conf:8` sets `webserver_listen = "172.18.0.1:9123"`.
- Docker network `proxy` supplies gateway `172.18.0.1`. Traefik uses this address for LibreQoS.
- The November 16 boot started lqosd at 09:40:04, before Docker restored its bridge at 09:40:05–06.
- Node Manager failed at 09:40:04 with `Cannot assign requested address (os error 99)`.
- The November 15 boot shows the same error before Docker completed initialization at 09:59:07.
- The lqosd unit waits for `network-online.target`. It has no Docker dependency or address-readiness check.
- The lqosd process remains active. Its `Restart=on-failure` cannot recover a failed web component inside an active process.
- Current socket checks show no listener on port 9123. Direct requests to `172.18.0.1:9123` fail immediately.

## Conclusion and limits

The confirmed reboot cause is a startup race with the Docker bridge address.
The missing listener causes the current Traefik 502 response.
Initial tunnel DNS errors also occurred. They do not explain the current local backend failure.
The reviewed upgrade restarts do not establish the same bind error.
Do not claim that all upgrade failures share this cause.
This web failure has separate evidence from the native policy acceptance hold.

## Next action

Prepare a systemd drop-in that orders lqosd after Docker and checks the configured address before startup.
Preserve the restricted listen address and the required lqosd-before-bridge order.
Check the dependency graph before application. Verify the listener and both HTTP paths after any authorized restart.
No configuration change, service restart, package action, or production write occurred during this review.
Only project notes changed. No recovery or reboot validation occurred.

## Authorized workaround preparation

The owner authorized the host workaround. Host sudo requires a password, so installation remains pending.
Artifacts: `/home/pi/.local/state/lqosync3/web-ui-startup-fix/`.
The drop-in adds `Wants=docker.service`, `After=docker.service`, and an address check before startup.
The helper reads `LQOS_CONFIG` and waits at most 60 seconds for the configured web address.
It skips the check if the web server is disabled. It fails visibly if the configuration contract changes.
The installer refuses to overwrite either target file. It reloads systemd and verifies the unit before restart.
It restarts only lqosd, then checks the local Traefik route and public HTTPS URL.

Checks passed: current address probe, retry and success, timeout failure, disabled web server, and shell syntax.
The combined proposed unit passed `systemd-analyze verify` with no dependency cycle.
That offline check used the staged helper path because the production helper does not exist yet.
No production file or service changed. No reboot, upgrade, or successful HTTP recovery is claimed.

Install from the operator terminal:

```bash
sudo bash /home/pi/.local/state/lqosync3/web-ui-startup-fix/install.sh
```

To remove the workaround, remove only these files and run `sudo systemctl daemon-reload`:

- `/etc/systemd/system/libreqos-lqosd.service.d/20-web-address.conf`
- `/usr/local/libexec/libreqos-wait-web-address`

Removal does not restart lqosd or reverse any control-plane activity after a restart.
The native policy acceptance investigation remains separate. Reboot and upgrade checks remain pending.

## Installation and HTTP verification

The operator ran the installer on 2026-11-16 at 11:19:55+08:00 (Asia/Shanghai).
The installed drop-in matches the prepared file. The address check exited zero.
The new lqosd process is PID 41738. Port `172.18.0.1:9123` now accepts connections.
The startup journal shows no new Node Manager bind failure in the reviewed interval.

The installer first saw a local 502 during startup, then received the normal redirect response.
Independent checks at 11:20:15+08:00 confirmed the local route returns 308 to `/index.html`.
Public HTTPS follows 308 to `/index.html`, then 307 to `/login.html`, then HTTP 200.
This proves public access to the login page. Authenticated UI behavior was not checked.
Updated the staged installer to follow public redirects during verification.
No additional restart occurred. Reboot and upgrade validation remain pending.
