# LQoSync 3

This branch starts the new Rust application. The existing application remains in the repository for reference.

The backend uses Axum. The web UI uses Maud templates written in Rust.
No Python backend, JavaScript UI framework, database, or Node build is required.

## Current capabilities

- Register and edit router connection settings. Store password environment references, not passwords.
- Create and edit WAN, PPPoE, DHCP, Hotspot, and site nodes.
- Set explicit capacities or compute subscriber aggregates with separate download and upload factors.
- Cap computed nodes at their immediate parent. Keep subscriber rates unchanged.
- Reject cycles, missing parents, duplicate IPv4 addresses, invalid rates, and excessive explicit capacities.
- Read an application-local subscriber snapshot and create deterministic CSV/JSON output.
- Publish valid output automatically into a local generation directory.
- Retain the last valid generation after an invalid snapshot or topology change.

This first delivery does **not** connect to routers or change production shaping.
RouterOS collection, interface discovery, core address-list writes, WAN assignment, and the LibreQoS adapter remain pending.
The UI reports local output as **staged**, never as production **applied**.
Changes to settings require a Save action. Local output publication after that save is automatic.

## Run

Use an existing Rust toolchain. Do not run the old repository installers for this application.

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

1. Implement RouterOS API transport and complete source/health snapshots.
2. Add WAN and neighbor discovery. Keep the network map separate from the shaping tree.
3. Implement source selectors and stable subscriber identity across routers.
4. Implement stable WAN assignments and reviewed core list ownership rules.
5. Validate the exact LibreQoS v3 reload interface and applied-generation acknowledgement.
6. Add authentication, deployment configuration, audit history, and generation retention.
7. Test failure recovery in an isolated environment before production activation.

LibreQoS v2.2 remains unchanged and held during this work.
