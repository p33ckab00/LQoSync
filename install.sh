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
[ -r /etc/os-release ] || die "cannot read /etc/os-release"
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

# 3. Apply mode must not start while the Python service writes the same files.
cfg_mode="$mode"
if [ -e "$CONF" ]; then
    cfg_mode="$(awk '/^[[:space:]]*\[/ { s = $0 } s ~ /^[[:space:]]*\[runtime\]/ && /^[[:space:]]*mode[[:space:]]*=/ { sub(/^[^"]*"/, ""); sub(/".*/, ""); print; exit }' "$CONF")"
    # A form that this reader does not know counts as apply: the guard below then stops.
    case "$cfg_mode" in shadow|apply) ;; "") cfg_mode=shadow ;; *) cfg_mode="unknown ($cfg_mode)" ;; esac
fi
if [ "$cfg_mode" != "shadow" ] && systemctl is-active --quiet lqosync-updatecsv.service; then
    die "the config has runtime.mode = $cfg_mode and lqosync-updatecsv.service (the Python service) is active. Stop it first: sudo systemctl disable --now lqosync-updatecsv.service"
fi

# 4. The release: the newest v3 release, or --version.
if [ -z "$version" ]; then
    version="$(curl -fsSL "https://api.github.com/repos/$REPO/releases" \
        | grep -o '"tag_name": *"v3\.[^"]*"' | head -n 1 | grep -o 'v3\.[^"]*')" \
        || die "cannot find a v3 release of $REPO (no release, no network, or the GitHub API limit); use --version v3.0.N"
fi
url="https://github.com/$REPO/releases/download/$version"
tmp="$(mktemp -d)"
trap 'rm -rf "$tmp"' EXIT
echo "Download $version"
for f in "$ASSET" "$ASSET.sha256" lqosync3.service; do
    curl -fsSL -o "$tmp/$f" "$url/$f" || die "cannot download $url/$f"
done
(cd "$tmp" && sha256sum -c "$ASSET.sha256") || die "the SHA-256 checksum is not correct"

# 5. The binary and the unit. An installed binary is replaced (upgrade).
# An upgrade keeps the choice of the operator: a stopped or disabled service stays so.
new_install=1
[ -e "$UNIT" ] && new_install=0
was_enabled=0
was_active=0
systemctl is-enabled --quiet lqosync3.service 2>/dev/null && was_enabled=1
systemctl is-active --quiet lqosync3.service && was_active=1
install -m 0755 "$tmp/$ASSET" "$BIN"
install -m 0644 "$tmp/lqosync3.service" "$UNIT"
systemctl daemon-reload
echo "Installed $("$BIN" version)"

# 6. The directories, and a config file only if it does not exist.
install -d -m 0755 "$CONF_DIR" "$STATE_DIR"
install -d -m 0700 "$CONF_DIR/secrets"
if [ ! -e "$CONF" ]; then
    printf '# lqosync3 config. The defaults: lqosync3 defaults. Add a router: sudo lqosync3 tui, tab 3, key +.\n[runtime]\nmode = "%s"\n' "$mode" > "$CONF"
    chmod 0644 "$CONF"
    echo "Wrote $CONF (mode $mode, no router yet)"
else
    echo "Kept $CONF (runtime.mode = $cfg_mode; --mode changes only a new file)"
fi

if systemctl is-active --quiet lqosync-updatecsv.service; then
    echo "WARNING: lqosync-updatecsv.service (the Python service) is active. Stop it before you set runtime.mode = \"apply\"."
fi

# 7. Check the config. Start (or restart) the service only if the check has no error.
if ! "$BIN" check --config "$CONF"; then
    echo "The check failed: the service is not started or restarted."
    echo "Next: sudo lqosync3 tui (tab 3: add a router with +, then v, A, y), then: sudo systemctl enable --now lqosync3.service"
    exit 0
fi
if [ "$start" -eq 0 ]; then
    echo "--no-start: the service is not started."
elif [ "$new_install" -eq 1 ]; then
    systemctl enable --now lqosync3.service
    echo "lqosync3.service: $(systemctl is-active lqosync3.service)"
elif [ "$was_active" -eq 1 ]; then
    systemctl restart lqosync3.service
    echo "lqosync3.service: $(systemctl is-active lqosync3.service)"
else
    echo "lqosync3.service was stopped before the upgrade: it stays stopped (enabled: $was_enabled)."
    echo "To start it: sudo systemctl enable --now lqosync3.service"
fi
