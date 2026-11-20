#!/usr/bin/env bash
# Install only the web services. Preserve existing web environment files.
set -euo pipefail
cd "$(dirname "$0")/../.."
if [[ $EUID != 0 ]]; then echo 'Run this installer with sudo.' >&2; exit 1; fi
for file in target/release/web-gateway target/release/lqosync3-web-broker target/release/lqosync3-web-admin crates/web-ui/dist/bootstrap.js; do
    test -f "$file" || { echo 'Run tools/frontend/build.sh first.' >&2; exit 1; }
done
for unit in lqosync3-web.service lqosync3-web-broker.service; do
    if systemctl is-active --quiet "$unit"; then systemctl stop "$unit"; fi
done
getent passwd lqosync-web >/dev/null || useradd --system --user-group --home-dir /nonexistent --shell /usr/sbin/nologin lqosync-web
install -d -m 0755 /usr/local/libexec /usr/local/share/lqosync3-web /usr/local/bin
install -d -m 0700 /etc/lqosync3-web /var/lib/lqosync3-web-accounts
install -d -m 0700 -o lqosync-web -g lqosync-web /var/lib/lqosync3-web
install -m 0755 target/release/web-gateway /usr/local/libexec/lqosync3-web
install -m 0755 target/release/lqosync3-web-broker /usr/local/libexec/lqosync3-web-broker
install -m 0755 target/release/lqosync3-web-admin /usr/local/bin/lqosync3-web-admin
cp -a crates/web-ui/dist/. /usr/local/share/lqosync3-web/
chown -R root:root /usr/local/share/lqosync3-web
chmod -R a+rX /usr/local/share/lqosync3-web
if [[ ! -f /etc/lqosync3-web/gateway.env ]]; then
    install -m 0600 deploy/frontend/gateway.env.example /etc/lqosync3-web/gateway.env
fi
if [[ ! -f /etc/lqosync3-web/broker.env ]]; then
    printf 'WEB_GATEWAY_UID=%s\n' "$(id -u lqosync-web)" > /etc/lqosync3-web/broker.env
    chmod 0600 /etc/lqosync3-web/broker.env
fi
install -m 0644 deploy/frontend/lqosync3-web*.service /etc/systemd/system/
systemctl daemon-reload
systemctl enable --now lqosync3-web-broker.service lqosync3-web.service
printf '%s\n' 'Web services installed. Create an account with:' 'sudo lqosync3-web-admin user create YOUR_USERNAME' 'Then merge deploy/frontend/traefik.yml into the current Traefik file.'
