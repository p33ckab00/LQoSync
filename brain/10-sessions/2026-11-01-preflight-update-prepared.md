---
type: session
created: 2026-11-01
tags: [libreqos, preflight]
status: complete
related: ["[[libreqos-v3-ingress-facts]]"]
---

# Preflight update prepared

Date: 2026-11-01 (Asia/Shanghai).

The user requested an update of the standalone LibreQoS preflight checker.
The installed package is `3.0.7.202610271607`.
The signed beta archive provides `3.0.7.202610310038`.
GitHub main points to `2d922cb6d6ce`, dated 2026-10-31 UTC.

The package is in `/home/pi/.local/state/libreqos-preflight-update/`.
Its filename is `libreqos-preflight_3.0.7.202610310038_amd64.deb`.
The LibreQoS archive signature and the SHA-256 hashes passed verification.
The installation simulation selected only `libreqos-preflight` for upgrade.
The package contains no maintainer scripts.

The extracted binary ran against `/opt/libreqos/src` without root privileges.
It validated the legacy topology pair and reported at least 26 circuits.
It returned `WARN` with exit code 1.
It reported no reserved hugepages and an unsupported installation tree without package ownership.
The complete output is in `check-before-install.txt` beside the package.

The user installed the package with `sudo dpkg -i`.
The installed package version is `3.0.7.202610310038`.
The installed binary hash matches the verified package binary.
The `dpkg --verify libreqos-preflight` check reported no differences.
The main LibreQoS package remains `2.2.202608181454`.
The checker reports binary version `0.1.0`. Use the package version to confirm the update.
