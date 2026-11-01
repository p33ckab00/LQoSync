#!/usr/bin/env bash
# lqosync3 installer and upgrader. It installs a release binary from GitHub.
# Usage: sudo bash install.sh [--version v3.0.N] [--mode shadow|apply] [--no-start]
# Refer to docs/INSTALLATION.md and docs/RELEASE.md.
set -euo pipefail

REPO="p33ckab00/LQoSync"
ASSET="lqosync3-x86_64-linux"
BIN="/usr/local/bin/lqosync3"
UNIT="/etc/systemd/system/lqosync3.service"
CONF_DIR="/etc/lqosync3"
CONF="$CONF_DIR/lqosync3.toml"
STATE_DIR="/var/lib/lqosync3"

version=""
mode="shadow"
start=1

die() { echo "install.sh: $*" >&2; exit 1; }

while [ $# -gt 0 ]; do
    case "$1" in
        --version) version="${2:-}"; shift 2 || die "--version needs a tag, for example v3.0.120" ;;
        --mode) mode="${2:-}"; shift 2 || die "--mode needs shadow or apply" ;;
        --no-start) start=0; shift ;;
        -h|--help) sed -n '2,4p' "$0"; exit 0 ;;
        *) die "unknown option $1" ;;
    esac
done
case "$mode" in shadow|apply) ;; *) die "--mode must be shadow or apply" ;; esac
case "$version" in ""|v3.*) ;; *) die "--version must be a v3 tag, for example v3.0.120" ;; esac

# 1. Root, operating system, and CPU.
[ "$(id -u)" -eq 0 ] || die "run as root: sudo bash install.sh"
. /etc/os-release
[ "${ID:-}" = "ubuntu" ] && [ "${VERSION_ID:-}" = "24.04" ] || die "Ubuntu 24.04 is necessary (found ${ID:-?} ${VERSION_ID:-?})"
[ "$(uname -m)" = "x86_64" ] || die "x86_64 is necessary (found $(uname -m))"

# 2. LibreQoS 2.0 to 2.2, or 3.x.
lqos="$(dpkg-query -W -f='${Version}' libreqos 2>/dev/null || true)"
case "$lqos" in
    2.0*|2.1*|2.2*|3.*) echo "LibreQoS $lqos" ;;
    "") die "the package libreqos is not installed" ;;
    *) die "LibreQoS $lqos is not supported (2.0 to 2.2, or 3.x)" ;;
esac

# 3. The release: the newest v3 release, or --version.
if [ -z "$version" ]; then
    version="$(curl -fsSL "https://api.github.com/repos/$REPO/releases" \
        | grep -o '"tag_name": *"v3\.[^"]*"' | head -n 1 | grep -o 'v3\.[^"]*')" \
        || die "cannot find a v3 release of $REPO"
fi
url="https://github.com/$REPO/releases/download/$version"
tmp="$(mktemp -d)"
trap 'rm -rf "$tmp"' EXIT
echo "Download $version"
for f in "$ASSET" "$ASSET.sha256" lqosync3.service; do
    curl -fsSL -o "$tmp/$f" "$url/$f" || die "cannot download $url/$f"
done
(cd "$tmp" && sha256sum -c "$ASSET.sha256") || die "the SHA-256 checksum is not correct"

# 4. The binary and the unit. An installed binary is replaced (upgrade).
install -m 0755 "$tmp/$ASSET" "$BIN"
install -m 0644 "$tmp/lqosync3.service" "$UNIT"
systemctl daemon-reload
echo "Installed $("$BIN" version)"

# 5. The directories, and a config file only if it does not exist.
install -d -m 0755 "$CONF_DIR" "$STATE_DIR"
install -d -m 0700 "$CONF_DIR/secrets"
if [ ! -e "$CONF" ]; then
    printf '# lqosync3 config. The defaults: lqosync3 defaults. Add a router: sudo lqosync3 tui, tab 3, key +.\n[runtime]\nmode = "%s"\n' "$mode" > "$CONF"
    chmod 0644 "$CONF"
    echo "Wrote $CONF (mode $mode, no router yet)"
fi

# 6. The Python service must not write the same files in apply mode.
if systemctl is-active --quiet lqosync-updatecsv.service; then
    echo "WARNING: lqosync-updatecsv.service (the Python service) is active. Stop it before you set mode = \"apply\"."
fi

# 7. Check the config. Start (or restart) the service only if the check has no error.
if ! "$BIN" check --config "$CONF"; then
    echo "The check failed: the service is not started."
    echo "Next: sudo lqosync3 tui (tab 3: add a router with +, then v, A, y), then: sudo systemctl enable --now lqosync3.service"
    exit 0
fi
if [ "$start" -eq 1 ]; then
    systemctl enable lqosync3.service
    systemctl restart lqosync3.service
    echo "lqosync3.service: $(systemctl is-active lqosync3.service)"
else
    echo "--no-start: the service is not started."
fi
