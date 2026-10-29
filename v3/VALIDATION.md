# Subscriber and output validation

Open **Validation** to compare captured router input with the current local output generation.
This GET page does not publish output, change active inventory, change routers, or apply LibreQoS shaping.

## Configure the inputs

1. Configure verified API access on **Routers**.
2. Read the core and subscriber router inventories.
3. Define explicit WAN roots and subscriber branches on **Topology**.
4. Map WAN interfaces and set actual capacities on **WANs**.
5. Bind DHCP servers, PPP profiles, and Hotspot profiles on **Sources**.
6. Select DHCP server speed rules on **Speed rules**, when required.
7. Configure source pools and subscriber pins on **Assignments**, when required.
8. Bind each active WAN to its core address list and custom table on **Core lists**.
9. Open **Validation** after a fresh complete collection.

These settings belong to LQoSync. They do not configure the MikroTik router.
The validator never guesses missing WAN capacities or source bindings.
Missing topology, mappings, or committed output produces `Unavailable`.

## Checks

The review reuses the same components as local publication:

- Normalize selected subscribers with stable IDs, finite speeds, and unique addresses.
- Assign compatible branches and explicit WAN roots.
- Allocate effective minimums proportionally within branch/WAN budgets while preserving per-subscriber floors and maximum speeds.
- Compile the topology, directional capacities, CSV, and JSON.
- Validate subscriber `/32` ownership and supported core routing policy.
- Compare the committed snapshot, assignments, CSV, JSON, and core plan with those candidate results.

The reader selects one immutable generation through one pointer decision.
It checks snapshot and plan integrity before it reads the output artifacts.
Each review artifact must be at most 64 MiB.
CSV and JSON must match the canonical output from the current captured inputs.
Changed speeds, parents, topology, ownership, provenance, or output content produce a conflict.
A generation switch during review invalidates the result.

The page shows WAN coverage and each check result.
Large tables show the first 200 WANs. All assigned subscribers receive validation.
The timestamp identifies the captured review time.
A consistency pass does not prove packet routing, hardware behavior, or production readiness.

## Live and saved inputs

**Live inputs** means inventory in the current LQoSync process.
Required snapshots must be complete, consistent, and no older than five minutes at review time.
Failed latest reads cannot use an older successful snapshot as current evidence.
Settings, read epochs, inventory identity, errors, and generation guards reject superseded work.
Expensive calculations run outside the application lock and async worker.

**Saved inputs** loads only the required local diagnostic snapshots.
This never inserts saved snapshots into active inventory or publishes a generation.
Saved review remains historical, even when timestamps are recent.
Its overall result cannot establish current live validation.

## Connection-mark paths

**Core review** shows connection-mark links separately from overall WAN findings.
It distinguishes two narrow supported forms:

1. **Subscriber forwarding:** a positive source-list classifier marks new unmarked connections.
   A later prerouting rule uses the same source list and connection mark to select the WAN table.
2. **Router replies:** a WAN-interface classifier marks new unmarked ingress connections.
   An output rule uses that mark and the same WAN table for router replies.

A router-reply path does not satisfy a subscriber source-list routing requirement.
Reply classifiers can match local destinations or leave destination type unrestricted.
They cannot exclude local destinations with `!local`.
Forward classifiers must precede their consumers.
Classifiers continue processing. Routing consumers require `passthrough=false`.

Marks require one enabled producer and one routing consumer, known status, and exact table/WAN relationships.
Unknown matchers, duplicate producers, implicit mark clearing, and unmodeled output actions remain blocked.

Mark-link status checks the connection-mark relationship only.
Overall WAN findings still check defaults, interfaces, FIB state, unsupported rules, and inventory health.
The [scoped forwarding checks](SCOPED_ROUTING.md) define supported earlier bypasses and static interface/destination-list conditions.
Other conditional policies, active PCC, and FastTrack remain blockers.
The validator does not simulate NAT, RAW, VRFs, connection state transitions, or all firewall packet behavior.
It does not recommend removal of existing router rules to force a pass.

Connection paths are limited to 512 per core. An explicit global finding marks incomplete review.
The UI shows the first 200 links. Findings remain bounded and cannot silently turn truncation into a pass.
Rule order remains intact. Unrelated route and interface order does not affect the result.

## Command line

Run a fresh read and local end-to-end report:

```bash
cd /opt/LQoSync3.0/v3
cargo run --locked --bin core-validate -- --end-to-end
```

The command reads configured cores and the required selected source/WAN routers through the print allowlist.
It stores raw inventory, `state/core-validation.json`, and `state/end-to-end-validation.json` only in application data.
It does not update the web process, publish a generation, or change router configuration.
Terminal output contains counts and check codes. Detailed local reports can contain private policy identifiers.

With `--end-to-end`, exit code zero requires a complete passing review.
Conflicts, unsupported policy, missing setup, historical input, and unavailable evidence return nonzero after report creation.
Without this flag, the older core-review command retains its input-completion exit behavior.

For historical diagnostics without network access:

```bash
cargo run --locked --bin core-validate -- --saved --end-to-end
```

This command creates a historical report and returns nonzero because saved evidence is not current live validation.
Unknown flags fail before settings or router access.
Use one collector per application data directory.

## Performance check

```bash
cargo test --locked --test connection_paths review_500 -- --ignored --nocapture
```

The workload uses 500 marked WANs and 100,000 routes.
It verifies all links and reverses unrelated input rows while it preserves mangle rule order.
Indexes keep mark, list, table, and interface joins outside repeated route scans.
Measurements exclude collection and disk publication. Debug timings do not guarantee production latency.

RouterOS action and chain behavior follows the official [mangle reference](https://manual.mikrotik.com/docs/cli-reference/ip/firewall/mangle/).
The supported forms above are this validator's conservative subset.
