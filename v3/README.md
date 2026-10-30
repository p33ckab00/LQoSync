# LQoSync 3

This branch starts the new Rust application. The existing application remains in the repository for reference.

The backend uses Axum. The web UI uses Maud templates written in Rust.
No Python backend, JavaScript UI framework, database, or Node build is required.

The compact NOC console follows the supplied dark design with grouped navigation and dense tables.
Use the top bar to change display density. This choice stays in the browser.
Topology shows discovered inventory, the configured hierarchy, and a selected-node inspector.
Use the cog beside an item to edit its configuration in a native modal. **Add node** opens the creation dialog.
Dialogs keep forms out of the main layout and preserve entered values after a failed save.
Router names open the router inspector. Existing save actions and validation rules still apply.
Discovery links open the current source and WAN forms. They do not create nodes or change router records.
Snapshot totals do not indicate online sessions. Count mode uses observed active sessions and bound DHCP leases.
See [flexible topology and reconnect assignment](FLEXIBLE_TOPOLOGY.md) for independent selectors, groups, and session rules.

## Current capabilities

- Register router settings with private TOML credentials or environment references, verified TLS, a custom CA, and operation deadlines.
- Read allowlisted router identity, interfaces, routes, neighbors, and subscriber source tables on demand.
- Save inventory and per-table errors in application-local TOML. Incomplete inventory never replaces the subscriber snapshot.
- Detect the main installed LibreQoS package. Select the legacy adapter for 2.0–2.2, or the v3 scheduler adapter for 3.x.
- Block unknown versions and incomplete package states. Ignore the separate preflight package.
- Preview PPP, DHCP, and Hotspot speeds with rule provenance and unresolved errors.
- Edit minimum-rate policy in application-local TOML through the Rust UI.
- Fit effective subscriber minimums within exported ancestors and shared routing WAN budgets while preserving floors and router maximums.
- Set DHCP subscriber speeds from server comments, server-name suffixes, or per-server manual rules.
- Create, promote, demote, and remove custom exported nodes with explicit replacements for dependencies.
- Use flat roots, arbitrary branches, shared destinations, and independent roots without mandatory service layers.
- Keep routing WAN identity independent of exported ancestry through a reversible settings conversion.
- Map discovered router interfaces to WAN roots and set their directional capacities.
- Select eligible exported node/WAN pairs and explicit subscriber pins. Keep legacy pools readable.
- Select profiles, servers, accounts, and leases independently of the hierarchy. Individual selectors override groups and can exclude records.
- Create named groups and create source branches from fresh inventory. Keep manual hierarchy controls.
- Balance new or reconnected sessions by observed client count. Keep continuous sessions and manual pins stable.
- Optionally balance by planned load and capacity with historical assignment retention.
- Set explicit capacities or compute subscriber aggregates with separate download and upload factors.
- Cap computed nodes at their immediate parent. Keep subscriber rates unchanged.
- Reject cycles, missing parents, duplicate IPv4 addresses, invalid rates, and excessive explicit capacities.
- Bind discovered PPP profiles, DHCP servers, and Hotspot profiles to shaping nodes through Sources.
- Normalize selected inventories with stable IDs, exact names, collision checks, and per-source health.
- Collect selected routers automatically with at most four concurrent reads. Pause stops queued reads.
- Commit normalized snapshots with deterministic CSV/JSON output as one local generation.
- Commit subscriber source ownership and WAN assignments in the same generation.
- Configure per-WAN core lists and custom routing tables through the UI.
- Build owned IPv4 `/32` add/keep/remove plans with protected manual members and conservative RouterOS 7 route checks.
- Review actual core policy with multiple per-WAN findings and separate live, saved, stale, and incomplete inventory labels.
- Trace supported subscriber-forward and router-reply connection-mark paths separately.
- Review subscriber speeds, assignments, CSV/JSON, and the committed core plan on Validation without publication.
- Review explicit static ingress and destination/control-protocol scopes on Core review. See [scoped routing](SCOPED_ROUTING.md).
- Publish valid output automatically into a local generation directory.
- Retain the last valid generation after an invalid snapshot or topology change.

Router connections use the pinned `mikrotik-rs` 0.7.0 API client.
This release uses MIT/Apache-2.0 licensing and supports async tagged commands and TLS.
Newer 0.8–0.10 releases use AGPL-3.0-only. The pinned version does not include all newer fixes.
A documented local patch to `mikrotik-tokio` prevents dropped rows during response bursts.
See `vendor/mikrotik-tokio/PATCHES.md` for source, license, and removal conditions.
See [the selected API documentation](https://docs.rs/mikrotik-rs/0.7.0/mikrotik_rs/).

Store router passwords in private application-local `credentials.toml`, or set each router's password environment variable before you start the app.
See [core review](CORE_REVIEW.md) for the credential format, verified TLS, and diagnostic commands.
Save its connection settings, then select **Read inventory** to discover source choices.
Create subscriber shaping nodes in Topology, then bind sources in **Sources**.
Selected routers receive automatic read rounds on the configured interval when automatic publication is enabled.
Source bindings and mapped WAN routers resume collection after a restart. Other routers require an on-demand read.
Use a RouterOS account with read/API access. No router configuration commands are exposed.
TLS verifies the certificate name and trust chain; there is no skip-verification option.
Inventory status is from the last read. Concurrent router-setting changes discard the result.
Saved inventory files are diagnostic snapshots; the UI starts with no active inventory after a restart.

Production shaping remains disabled.
Speed rules now resolve inventory into previews. See [speed rules](SPEED_RULES.md) for precedence and compatibility differences.
Source binding, stable IDs, complete snapshot publication, and scheduled read rounds are implemented.
See [subscriber orchestration](SUBSCRIBERS.md) for health gates, overlap ownership, and limits.
See [WAN assignments](WAN_ASSIGNMENTS.md) for interface mapping, pools, pins, and planned-load distribution.
See [core planning](CORE_PLANNER.md) for list ownership, route audit limits, and the automatic local preview.
See [end-to-end validation](VALIDATION.md) for live/saved review, connection-mark support, output integrity, and setup requirements.
Core address-list writes and production apply remain pending.
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
  credentials.toml  # private, ignored by Git
  tls/              # application-local trust files
  state/
    subscribers.toml
    router-inventory/
      ROUTER_ID.toml
    current.toml
  staging/
    GENERATION/
      subscribers.toml
      assignments.toml
      core-plan.json
      ShapedDevices.csv
      network.json
```

The normalized snapshot, assignment state, and both output files exist before the state pointer changes.
With core bindings enabled, the same generation includes `core-plan.json` before that pointer changes.
Older normalized generations without assignment state remain readable.
The committed generation is authoritative. `state/subscribers.toml` is a compatibility mirror for managed generations.
A mirror failure produces a warning; it does not undo or hide the committed snapshot.
Offline examples without source bindings still use `state/subscribers.toml` as their input.
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
The speed resolver converts router-oriented `rx/tx` into subscriber download/upload order.
Use offline examples only in a separate data directory without source bindings.
Managed generations require a fresh, complete, valid selected-source collection.

## Checks

```bash
cargo fmt --check
cargo test --locked
cargo clippy --locked --all-targets -- -D warnings
```

## Production work that remains

1. Add effective RADIUS policy support, IPv6, and multi-device subscriber circuits.
2. Expand the discovered network map. Keep it separate from the shaping tree.
3. Validate deployment on isolated representative router fixtures before production use.
4. Validate actual core paths and implement coordinated execution of the local address-list plan.
5. Implement production execution for the selected adapter. Legacy uses `LibreQoS.py --updateonly`.
   V3 uses an authenticated `ReloadLibreQoS` request followed by applied-generation verification.
   A request acknowledgement is not proof that shaping was applied.
   Do not use the legacy Python apply command on v3.
   The supported detection range is 2.0–2.2 and 3.x; older and future major versions need separate validation.
6. Add authentication, deployment configuration, audit history, and generation retention.
7. Test failure recovery in an isolated environment before production activation.

LibreQoS v2.2 remains unchanged and held during this work.
