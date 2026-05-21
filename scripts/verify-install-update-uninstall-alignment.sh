#!/usr/bin/env bash
set -euo pipefail
ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"
fail=0

ok() { echo "ok|$1|$2"; }
bad() { echo "FAIL|$1|$2" >&2; fail=1; }

require_file() {
  [ -f "$1" ] && ok file "$1" || bad file "$1 missing"
}

require_exec() {
  [ -x "$1" ] && ok executable "$1" || bad executable "$1 not executable"
}

require_text() {
  local file="$1" pattern="$2" label="$3"
  if grep -q "$pattern" "$file" 2>/dev/null; then
    ok "$label" "$file"
  else
    bad "$label" "$file missing pattern: $pattern"
  fi
}

for f in \
  install.sh install-baremetal.sh install-production-safe.sh install-from-github.sh install-from-zip.sh \
  update-from-zip.sh upgrade.sh uninstall.sh install-docker.sh update-docker.sh uninstall-docker.sh \
  docker-entrypoint.sh compose.yaml compose.preserve-existing.yaml; do
  require_file "$f"
done

for f in install.sh install-baremetal.sh install-production-safe.sh install-from-github.sh install-from-zip.sh update-from-zip.sh upgrade.sh uninstall.sh install-docker.sh update-docker.sh uninstall-docker.sh docker-entrypoint.sh; do
  require_exec "$f"
  bash -n "$f"
done

for f in \
  docs/INSTALLATION_MATRIX.md docs/ZIP_INSTALL.md docs/DOCKER_OPERATIONS.md \
  docs/RUST_CORE_V781_INSTALL_UPDATE_UNINSTALL_ALIGNMENT.md \
  INSTALLATION.md BARE_METAL_INSTALL.md DOCKER_INSTALL.md GIT_INSTALL.md UNINSTALLATION.md docs/UPGRADE_GUIDE.md; do
  require_file "$f"
done

require_text docs/INSTALLATION_MATRIX.md "/opt/LQoSync" canonical-app-path
require_text docs/INSTALLATION_MATRIX.md "/opt/libreqos/src" canonical-libreqos-path
require_text docs/INSTALLATION_MATRIX.md "install-from-zip.sh" zip-install-wrapper
require_text docs/INSTALLATION_MATRIX.md "update-from-zip.sh" zip-update-wrapper
require_text docs/INSTALLATION_MATRIX.md "install-docker.sh" docker-install-wrapper
require_text docs/INSTALLATION_MATRIX.md "uninstall-docker.sh" docker-uninstall-wrapper
require_text docs/ZIP_INSTALL.md "install-from-zip.sh" zip-doc-install
require_text docs/ZIP_INSTALL.md "update-from-zip.sh" zip-doc-update
require_text docs/DOCKER_OPERATIONS.md "host-integrated" docker-host-integrated-warning
require_text install-from-zip.sh "preserve_existing" zip-preserve-default
require_text update-from-zip.sh "enable_only" zip-update-enable-only
require_text compose.yaml "LQOSYNC_INIT_POLICY: \"preserve_existing\"" docker-preserve-default
require_text install.sh "\$LIBREQOS_SRC_DIR/LibreQoS.py --updateonly" sudoers-dynamic-libreqos-path
require_text upgrade.sh "Service active check skipped" upgrade-service-policy-aware
require_text docker-entrypoint.sh "LQOSYNC_DATA_DIR" docker-path-override
require_text RELEASE_NOTES.md "2.148.1-rc1 - v7.8.1 Install / Update / Uninstall Alignment" release-note
require_text README.md "v7.8.1 install/update alignment" readme-note

if grep -RIn "LQoSync_v2_17_opt_lqosync.zip\|lqosync:2.17" INSTALLATION.md BARE_METAL_INSTALL.md DOCKER_INSTALL.md UNINSTALLATION.md docs/COMMANDS.md docs/GITHUB_INSTALL.md docs/UPGRADE_GUIDE.md >/tmp/lqosync_old_install_refs.txt 2>/dev/null; then
  cat /tmp/lqosync_old_install_refs.txt >&2
  bad stale-install-refs "old v2.17/lqos_docker install references remain in active operator docs"
else
  ok stale-install-refs "no old package/image references in active operator docs"
fi

if [ "$fail" -ne 0 ]; then
  echo "FAIL: install/update/uninstall alignment verification failed" >&2
  exit 1
fi

echo "PASS: install/update/uninstall documentation and script alignment verified"
