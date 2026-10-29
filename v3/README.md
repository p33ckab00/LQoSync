# LQoSync 3

This branch starts the new Rust application. The existing application remains in the repository for reference.

The backend uses Axum. The web UI uses Maud templates written in Rust.
No Python backend, JavaScript UI framework, database, or Node build is required.

## Current capabilities

- Register router settings with password environment references, verified TLS, optional custom CA, and operation deadlines.
- Read allowlisted router identity, interfaces, routes, neighbors, and subscriber source tables on demand.
- Save inventory and per-table errors in application-local TOML. Incomplete inventory never replaces the subscriber snapshot.
- Detect the main installed LibreQoS package. Select the legacy adapter for 2.0–2.2, or the v3 scheduler adapter for 3.x.
- Block unknown versions and incomplete package states. Ignore the separate preflight package.
- Create and edit WAN, PPPoE, DHCP, Hotspot, and site nodes.
- Set explicit capacities or compute subscriber aggregates with separate download and upload factors.
- Cap computed nodes at their immediate parent. Keep subscriber rates unchanged.
- Reject cycles, missing parents, duplicate IPv4 addresses, invalid rates, and excessive explicit capacities.
- Read an application-local subscriber snapshot and create deterministic CSV/JSON output.
- Publish valid output automatically into a local generation directory.
- Retain the last valid generation after an invalid snapshot or topology change.

Router connections use the pinned `mikrotik-rs` 0.7.0 API client.
This release uses MIT/Apache-2.0 licensing and supports async tagged commands and TLS.
Newer 0.8–0.10 releases use AGPL-3.0-only. The pinned version does not include all newer fixes.
A documented local patch to `mikrotik-tokio` prevents dropped rows during response bursts.
See `vendor/mikrotik-tokio/PATCHES.md` for source, license, and removal conditions.
See [the selected API documentation](https://docs.rs/mikrotik-rs/0.7.0/mikrotik_rs/).

Set each router's password environment variable before starting the app.
Save its connection settings, then select **Read inventory**. Saving alone does not connect.
Use a RouterOS account with read/API access. No router configuration commands are exposed.
TLS verifies the certificate name and trust chain; there is no skip-verification option.
Inventory status is from the last read. Concurrent router-setting changes discard the result.
Saved inventory files are diagnostic snapshots; the UI starts with no active inventory after a restart.

Production shaping remains disabled.
Source normalization, WAN assignment, core address-list writes, and production apply execution remain pending.
The UI reports local output as **staged**, never as production **applied**.
Changes to settings require a Save action. Local output publication after that save is automatic.

## Run

Use an existing Rust toolchain (Rust 1.88 or newer). Do not run the old repository installers for this application.

```bash
cd /opt/LQoSync3.0/v3
cargo run --locked
```

Open `http://127.0.0.1:9303` on the server.
For remote access, use an SSH tunnel:

```bash
ssh -L 9303:127.0.0.1:9303 pi@SERVER
```

Then open `http://127.0.0.1:9303` on your computer.
The development console accepts loopback connections only. It has no user login yet.
Form tokens protect settings edits. Do not expose it through a public proxy.

## Application data

By default, the app uses `data/` under the current directory.
Set `LQOSYNC3_DATA` to use another application-owned directory.
Set `LQOSYNC3_BIND` to change the loopback address or port.

```text
data/
  settings.toml
  state/
    subscribers.toml
    router-inventory/
      ROUTER_ID.toml
    current.toml
  staging/
    GENERATION/
      ShapedDevices.csv
      network.json
```

Both output files exist before the state pointer changes.
Identical output reuses the current generation.
There is no export to `/opt/libreqos`, router write, package update, or service restart.
Old local generations remain available. Retention limits are pending before long-term use.

## Offline example

Stop the app before you replace settings files.
Use the examples only in a separate application data directory:

```bash
mkdir -p data-example/state
cp examples/settings.toml data-example/settings.toml
cp examples/subscribers.toml data-example/state/subscribers.toml
LQOSYNC3_DATA="$PWD/data-example" cargo run --locked
```

The snapshot rates are already in subscriber download/upload order.
A future RouterOS adapter must convert router-oriented `rx/tx` correctly.
The current snapshot format represents a complete successful collection.
Per-router collection health and partial-scan preservation must exist before live collection is enabled.

## Checks

```bash
cargo fmt --check
cargo test --locked
cargo clippy --locked --all-targets -- -D warnings
```

## Production work that remains

1. Normalize RouterOS inventory into complete subscriber snapshots with per-source health.
2. Add WAN and neighbor discovery. Keep the network map separate from the shaping tree.
3. Implement source selectors and stable subscriber identity across routers.
4. Implement stable WAN assignments and reviewed core list ownership rules.
5. Implement production execution for the selected adapter. Legacy uses `LibreQoS.py --updateonly`.
   V3 uses an authenticated `ReloadLibreQoS` request followed by applied-generation verification.
   A request acknowledgement is not proof that shaping was applied.
   Do not use the legacy Python apply command on v3.
   The supported detection range is 2.0–2.2 and 3.x; older and future major versions need separate validation.
6. Add authentication, deployment configuration, audit history, and generation retention.
7. Test failure recovery in an isolated environment before production activation.

LibreQoS v2.2 remains unchanged and held during this work.
