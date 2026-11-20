# Deploy sign-in and Dashboard

The gateway port is `9581`. The public origin is `https://lqosync.fsehub.qzz.io`.
The gateway requires an HTTPS origin for non-loopback deployment.
The proxy terminates TLS. The gateway sets a Secure cookie from the configured origin.
It does not trust a forwarded header to select the cookie policy.

## Build and install


1. Install Rust with the `wasm32-unknown-unknown` target and Trunk 0.22.2.

2. Run `tools/frontend/build.sh` from the repository.

3. Review `deploy/frontend/gateway.env.example`. Check the Traefik bridge address.

4. Run `sudo tools/frontend/install.sh`.

5. Run `sudo lqosync3-web-admin user create YOUR_USERNAME`. Enter the password at the hidden prompt.

6. Add the proxy route described below.

7. Check both web units with `systemctl status lqosync3-web-broker lqosync3-web`.

8. Open the HTTPS origin. Sign in and compare Dashboard evidence with the backend.

The installer adds separate services. It does not restart `lqosync3.service` or change the TUI.
It preserves existing gateway and broker environment files.
It does not create an account or edit the current Traefik file.

For an update, stop the two web services before the installer replaces their binaries.
Start the two web services after installation. Keep the existing private databases.

## Current Traefik layout

The current Traefik container uses the file provider at `/etc/traefik/dynamic.yml`.
Its host file is `/opt/docker/libreqos-mgmt/traefik/dynamic.yml`.
Its Docker network is `proxy`. The bridge gateway is `172.18.0.1`.
The container listens on the `web` entry point. Existing routers must remain in the file.

Back up the host file. Merge the router and service from `deploy/frontend/traefik.yml` into its `http` map.
Do not replace the complete file with the example.
The new service URL is `http://172.18.0.1:9581`.

The gateway example binds only this bridge address.
If the Docker subnet changes, update the gateway bind and the Traefik service URL together.

The browser origin must remain the HTTPS hostname.

The host also has a cloudflared container. Make sure that its ingress accepts the new hostname and forwards it to Traefik.
A Traefik rule alone does not prove DNS, tunnel ingress, or public TLS works.
No tunnel credentials belong in this repository.

## Nginx alternative

Set `WEB_BIND=127.0.0.1:9581` in `/etc/lqosync3-web/gateway.env`.
Use `deploy/frontend/nginx.conf` in the host TLS configuration.
Set the certificate paths. Validate with `nginx -t` before reload.
Keep `WEB_ORIGIN=https://lqosync.fsehub.qzz.io`.

## Rollback

Stop and disable only `lqosync3-web.service` and `lqosync3-web-broker.service`.
Remove the new proxy router and service. Preserve all existing proxy entries.
Keep the account and session databases for a later retry.
The synchronization service and TUI continue independently.

## Session boundary

The current coding session cannot run passwordless sudo.
The code, build output, installer, and proxy examples are prepared for operator review.
Live installation, first-account creation, and public-route validation require the operator's root session.
