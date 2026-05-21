# v7.8.1 Install / Update / Uninstall Alignment

This maintenance update aligns documentation and operator scripts with the current system design.

## Goals

- Make `/opt/LQoSync` the single documented runtime/source path.
- Make `/opt/libreqos/src` the single documented LibreQoS file path.
- Provide explicit install/update/uninstall wrappers for ZIP, GitHub, bare-metal, and Docker flows.
- Keep production-safe defaults: preserve existing LibreQoS files and avoid automatic service start for live ZIP/local workflows.
- Keep Docker clearly marked as host-integrated and not the preferred live LibreQoS path.

## Added scripts

- `install-from-zip.sh`
- `update-from-zip.sh`
- `install-docker.sh`
- `update-docker.sh`
- `uninstall-docker.sh`
- `scripts/verify-install-update-uninstall-alignment.sh`

## Added docs

- `docs/INSTALLATION_MATRIX.md`
- `docs/ZIP_INSTALL.md`
- `docs/DOCKER_OPERATIONS.md`

## Safety behavior

ZIP/local install and update wrappers default to:

```text
LQOSYNC_INIT_POLICY=preserve_existing
LQOSYNC_SERVICE_START_POLICY=enable_only
```

The historical `install.sh` default remains `restart` for backward compatibility, but production-safe wrappers avoid automatic start/restart.
