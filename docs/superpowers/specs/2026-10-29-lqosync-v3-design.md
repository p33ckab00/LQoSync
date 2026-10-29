# LQoSync v3 design

The user authorized a new application in `/opt/LQoSync3.0` on branch `lqosyncv3`.
Rust owns the backend and web UI. The existing LibreQoS v2.2 installation must remain unchanged.

## Agreed behavior

- Keep application settings, state, logs, and backups inside the application directory.
- Use TOML settings. Do not use a database or the existing custom `config.json`.
- Read subscriber identities and speed sources from MikroTik routers.
- Separate the discovered network map from the exported shaping tree.
- Export real shaping nodes. Do not use v2 virtual nodes.
- Support explicit node capacities and computed subscriber aggregates with separate direction factors.
- Cap computed nodes at their immediate parent capacity. Reject explicit values above that capacity.
- Keep subscriber rates separate from branch capacities. Count each subscriber once per subtree.
- Support router/profile/server selectors and stable subscriber-to-WAN assignments.
- Eventually update system-owned core address lists with individual IPv4 `/32` entries.
- Apply valid changes automatically. Verification is automatic, not an operator confirmation step.
- Preserve the last valid state after incomplete collection or partial application failure.

## Implementation choice

Use Axum with Maud templates. Maud renders HTML from Rust; no Python or JavaScript UI framework is required.
The alternative was a Rust WebAssembly UI. Server rendering needs fewer build tools for this first slice.
Keep the existing repository intact and add the application under `v3/`.

## First delivery

Build a runnable web console for routers, topology, subscribers, and settings.
Persist validated configuration through atomic TOML replacement.
Build deterministic CSV/JSON previews from an application-local subscriber snapshot.
Publish both artifacts into an immutable local generation, then switch one state pointer.
Automatically stage valid changes after configuration edits or snapshot changes.
Show staging separately from production application. Never label a local preview as applied shaping.
Bind to loopback. Require a session-specific form token for mutations.
Resolve credentials by environment-variable reference; never display or copy production credentials.

## Remaining adapters

RouterOS transport, WAN discovery, core writes, WAN redistribution, and LibreQoS reload/acknowledgement remain separate work.
Do not enable production writes from this first delivery.
The existing file-pair preflight validates one current snapshot, not all future topology modes.
The production adapter must confirm exact v3 authentication and applied-generation semantics before activation.

## Known rule differences

The current script uses custom defaults for DHCP per-site and Hotspot.
It also computes group caps and subscriber minimums outside the router.
Do not describe those behaviors as router-native policy.
RouterOS PPP `rx/tx` means subscriber upload/download; preserve that direction when converting asymmetric rates.
