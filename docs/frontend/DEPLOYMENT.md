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

## DPDK Dashboard update

The [DPDK card](DPDK_DASHBOARD.md) updates the web broker, gateway, and browser assets.
It uses the worker and NIC caches already published by the core service.

Build from the repository as user `pi`:

```bash
cd /home/pi/LQoSync3.0
TRUNK=/tmp/lqosync-web-tools/trunk tools/frontend/build.sh
```

Use `tools/frontend/build.sh` without `TRUNK` if Trunk is on your PATH.

Install the prepared web build:

```bash
sudo tools/frontend/install.sh
systemctl status lqosync3-web-broker lqosync3-web
```

The installer restarts the web units.
Open the HTTPS origin and refresh the page.
Compare worker count, bridge health, port state, and source age with the TUI.
The browser chart starts a new session history after a page reload.

## Live traffic update

The [live traffic update](LIVE_TRAFFIC.md) also changes the core telemetry sampler and status summary.
The web installer only installs the web services. It does not install the core binary.
An older core cannot publish the new WAN capacity summary.
The chart shows capacity as unavailable until the updated core publishes current status.

Build as user `pi`, from `/home/pi/LQoSync3.0`:

```bash
cd /home/pi/LQoSync3.0
CARGO_BUILD_JOBS=2 cargo build --release --locked -p lqosync3
TRUNK=/tmp/lqosync-web-tools/trunk tools/frontend/build.sh
```

Use `tools/frontend/build.sh` without `TRUNK` if Trunk is on your PATH.
The temporary Trunk path above applies to this host and can disappear after cleanup.

Install during an approved service maintenance window:

```bash
sudo systemctl stop lqosync3
sudo install -m 0755 target/release/lqosync3 /usr/local/bin/lqosync3
sudo systemctl start lqosync3
sudo tools/frontend/install.sh
systemctl status lqosync3 lqosync3-web-broker lqosync3-web
```

The core restart interrupts collection briefly. It does not request a mode or topology change.
Keep the existing configuration and private databases.
Check the current WAN nodes and their true base rates in the existing topology workflow.
The sum excludes intermediate nodes and subscriber plans.
No configured WAN nodes means no configured capacity claim.

The browser uses `/api/v1/overview/live` for server-sent events.
Keep the response unbuffered and uncached through each proxy.
The gateway sends `text/event-stream`, `Cache-Control: no-store`, and `X-Accel-Buffering: no`.
Traefik forwards this stream through the existing service route. No separate WebSocket route is needed.
The Nginx example disables proxy buffering and caching.
The browser retains a five-second fallback poll if the stream fails.

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


## Aggregate traffic comparison update

The [aggregate comparison](AGGREGATE_TRAFFIC.md) adds Mapped and Unmapped to the existing Actual plot.
It updates the web broker contract and Rust browser assets.
Build from the repository root, then install both together:

```sh
tools/frontend/build.sh
sudo tools/frontend/install.sh
systemctl status lqosync3-web-broker lqosync3-web
```

This update does not require a replacement of the core `lqosync3` binary.
Refresh the browser after installation.
Compare source figures with the TUI at the same published timestamps.


## NIC faceplate update

The [physical NIC faceplate](NIC_FACEPLATE.md) needs the updated core collector for hardware identity and interface roles.
Update the core and the web packages together.
Run these commands from `/home/pi/LQoSync3.0` after the builds pass:

```sh
sudo systemctl stop lqosync3
sudo install -m 0755 target/release/lqosync3 /usr/local/bin/lqosync3
sudo systemctl start lqosync3
sudo tools/frontend/install.sh
systemctl status lqosync3 lqosync3-web-broker lqosync3-web
```

The implementation session prepares the binaries and assets. It does not execute these production changes.
Allow one NIC baseline and the next counter sample after the restart.
NIC counters and web traffic updates now target one second.
The full service summary still refreshes every five seconds.
Hardware identity refreshes every thirty seconds. Failed NIC reads retry after ten seconds.
The [live NIC diagnostics panel](NIC_LIVE_DIAGNOSTICS.md) needs this core update for the faster counter cadence.
Unknown hardware remains generic when the driver cannot supply connector evidence.
Roles remain unknown when no explicit LibreQoS interface mapping matches.
The core can use `/usr/sbin/ethtool` for optional kernel module/media evidence.
Missing utility access does not block NIC counters or link telemetry.

## Topology Manager update

The Topology, PPPoE, and IPoE / DHCP screens need the new core service and the new web packages together.
The core writes `topology.json` and keeps web change receipts in `topology-manager/web-receipts/`.
The core creates both at the first cycle. No configuration change is necessary.

Before the update, record the running versions:

```sh
sha256sum /usr/local/bin/lqosync3
systemctl status lqosync3 lqosync3-web-broker lqosync3-web
```

Run these commands from `/home/pi/LQoSync3.0` after the release gates pass:

```sh
bash tools/frontend/build.sh
cargo build --locked --release -p lqosync3
sudo install -m 0755 /usr/local/bin/lqosync3 /usr/local/bin/lqosync3.before-topology
sudo systemctl stop lqosync3
sudo install -m 0755 target/release/lqosync3 /usr/local/bin/lqosync3
sudo systemctl start lqosync3
sudo tools/frontend/install.sh
systemctl status lqosync3 lqosync3-web-broker lqosync3-web
```

The implementation session prepares the binaries and assets. It does not execute these production changes.

After the next cycle, do these checks:

1. Open **Topology**. The evidence strip shows **View** with an age of seconds.
2. Open **PPPoE** and **IPoE / DHCP**. Each source shows its router state.
3. Do not save a change until the strip shows a known writer.

Opening a page reads no router. Only **Refresh observations**, **Discover WAN interfaces**, and **Discover sources** read routers.
Every change goes through **Review change**. Nothing changes until **Save change**.

### Rollback

1. Stop the service: `sudo systemctl stop lqosync3`.
2. Put back the previous binary: `sudo install -m 0755 /usr/local/bin/lqosync3.before-topology /usr/local/bin/lqosync3`.
3. Start the service: `sudo systemctl start lqosync3`.
4. Install the previous web build with the normal frontend workflow.

The previous binary ignores `topology.json` and the receipt directory. They can stay.
Policy revisions that the web saved stay in the registry. The previous binary reads them as before.

### WAN monitor

The service probes each WAN through its router with `/interface/print` and `/ping`.
The router user of LQoSync needs the `test` policy for `/ping`. Without it, each WAN shows "No probe data" and no failover occurs.
The `[wan_monitor]` section in `lqosync3.toml` changes the probe values. `enabled = false` stops the probes. Then placement treats every WAN as available.
A change of `interval_secs` applies after a service restart. The other values apply after a config reload.

Rollback: registry schema 7 needs the schema-6 backup for a schema-6 binary.

### WAN routing lists

The routing list writer is off by default. To set it up, follow [WAN routing lists](../operations/WAN_ROUTING_LISTS.md): router users, writer credentials, adoption, writer on, mangle order, and rollback.
Registry schema 8 adds the list name of a WAN node. A rollback to a schema-7 binary needs the schema-7 backup.

### Source and tree exclusion

Registry schema 9 adds two flags: "Do not read" on a source, and "Exclude from network tree" on a router node.
A rollback to a schema-8 binary needs the schema-8 backup. A schema-8 binary cannot open a schema-9 registry.

### Limits of this release

- The broker reads the whole `topology.json` for each request. At 50,000 subscribers and 5,000 nodes, the file is 31 MB.
  Each read takes about 260 ms on an Intel i5-9500. The answer itself takes less than 10 ms.
- The browser checks ran in Chromium only. Firefox and WebKit are not installed on this host.
- The web change history lists only web changes. TUI and CLI changes do not show.
