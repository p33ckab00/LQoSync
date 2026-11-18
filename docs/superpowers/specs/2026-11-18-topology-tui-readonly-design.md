---
type: spec
created: 2026-11-18
status: draft
related: ["[[topology-manager-status]]", "[[2026-11-18-topology-service-api]]"]
---

# Topology TUI read-only views design (Phase 2b-1)

## Intent

The owner wants to see and later edit the Topology Manager in `sudo lqosync3 tui`, through the service API, without a service stop.
Phase 2b-1 adds the Topology route and its read-only views. Later slices add edits:

1. 2b-1: Topology route, read-only views, and an on-demand preview (this document).
2. 2b-2: Assignments edits: per-subscriber plan, pin and unpin, and scoped rebalance.
3. 2b-3: Builder structural edits: create, move, promote, demote, and remove nodes.
4. 2b-4: Policies edits: profiles and speed defaults.

Owner reason for the order: IPoE clients on one DHCP server with dynamic leases need per-subscriber plans in LQoSync.

## Baseline

The approved layout in `docs/design/tui/TOPOLOGY_MANAGER_LAYOUT.md` applies. In short:

- One Topology entry after Routers and before LibreQoS. Workspace title: Topology Manager.
- Views inside Topology: Builder, Assignments, Policies, Changes.
- Builder at wide sizes: Discovery Inventory, Topology Tree, and Details panels, left to right. One focused panel with selectors when space is insufficient (80 columns).
- A compact publication strip stays visible across all Topology views.
- Keyboard reach for every action. No drag. Background refresh keeps focus, selection, scroll, and field input.
- Build view indexes when the source revision changes, not on each draw. Render only the visible viewport.

## Owner decisions

- Before Phase 4 activation, the preview is on demand. The TUI asks for a fresh preview when the operator opens the Topology route or presses refresh. The service does not read routers for the preview at other times.
- After activation, the views use the data of the latest manager cycle. There is no separate preview read.
- Live shaping stays automatic and unchanged: the legacy loop writes `ShapedDevices.csv` and `network.json` each cycle until activation.

## Approaches

| Approach | Summary | Trade-off |
|---|---|---|
| Service view models (chosen) | The service builds the tree, assignment rows, and counts, and returns them through the API. The TUI only draws | Thin TUI. The web frontend reuses the same operations |
| Raw data | The service returns the registry and the prepared pair. The TUI computes views | Duplicates logic in each frontend. Against the owner decision for a thin TUI |

There is one sensible approach. The raw-data approach conflicts with the owner's thin-TUI decision.

## Goals

1. A Topology route that shows a "not enrolled" state, a "policy installed, not enrolled" state, or the four views.
2. New read-only API operations that return view models.
3. An on-demand preview that runs in the service between cycles and is cached with its collection time.
4. Read-only Builder, Assignments, Policies, and Changes views that follow the approved layout.

## Non-goals

- No edit operation (2b-2 to 2b-4).
- No speed schedules (Phase 5).
- No change to live shaping or to the legacy loop.
- No automatic preview refresh.

## API operations (new)

| Operation | Args | Result |
|---|---|---|
| `topology.preview.refresh` | none | Runs a fresh preview (router reads) and caches it. Returns the same result as `topology.preview.get` |
| `topology.preview.get` | none | The cached preview, or `null` when none exists. No router read |
| `topology.policies` | none | Defaults and profiles with resolved min/max Mbps, from the registry. No router read |

A preview result contains:

- `collected_at` (Unix seconds), `registry_revision`, `generation`, and `state`: `ok` or `conflicts`.
- `nodes`: id, label, parent, kind, source label, calculated and exported ceiling (Mbps), subscriber count.
- `subscribers`: subscriber ID, identity (`account_ref` and scope), current IPs and MACs, target node, pin, profile, resolved min/max Mbps.
- `inventory`: catalog sources with their scope, kind, current member count, and the node instances that use them.
- `conflicts`: the identity and source conflicts, when `state` is `conflicts`. A conflict preview has no nodes or subscribers.
- `rounded`: the node export adjustments.

The preview needs the held instance lock and an installed policy (catalog and source settings). Enrollment is not necessary.
A missing policy gives `invalid` with a clear message, and the TUI shows the "not enrolled" state.
A preview is valid for display only. It is never published.

## Builder calls (made by the designer, open to owner correction)

- Preview gates: a new preview path skips the writer-only checks (apply mode, write tracker, topology runtime owner). The preview then also works in shadow mode and before enrollment. Publication keeps all existing gates.
- View data: the publication preparation keeps a view struct with the per-subscriber placement, endpoints, profile, pin, resolved rates, and calculated and exported node ceilings. The preview does not derive these values from CSV text.
- Conflicts as data: preparation returns a typed error that carries the identity, placement, and aggregate conflicts. The publish path still fails on it as before. The preview turns it into the `conflicts` list.
- Cycle timing: the preview runs on the service task between cycles. A refresh can start the next cycle late by up to the preview read time. The preview uses a 30 s source age limit. The refresh is on demand only, so this delay is rare.
- Generation: a preview advances the source generation. A later publication prepares again and needs a newer generation, so the preview does not block it.
- API handlers become async so that the preview can read routers. Other operations keep their current behavior.
- Size: one subscriber entry is about 300 bytes, so the 16 MiB response limit allows about 50,000 subscribers. Production has 28 rows on 2026-11-18.

## TUI behavior

- The Topology route reads `status` first. `registry_revision: null` gives the "not enrolled" state with the runbook commands. A registry without export ownership gives "policy installed, not enrolled".
- Opening the route asks for `topology.preview.get`. When the cached preview is older than 5 minutes or absent, the route asks once for `topology.preview.refresh`. A refresh key repeats the refresh.
- The strip shows: preview age, registry revision, and `legacy loop writes shaping` before activation.
- While a request waits for a running cycle, the strip shows "waiting for the service".
- `lqosync3.service` not answering shows a clear message and keeps the rest of the TUI usable.
- The TUI runs API requests in its loader thread. It never blocks the draw loop.

## Verification

- Service tests: preview refresh and get with the fake connector and the v3_root_promotion scenario; cached preview reuse without router reads; policies output; conflict preview content.
- TUI render tests with fixed preview data: each view at 80 and 160 columns, the "not enrolled" state, the strip, and selection retention across a refresh.
- Full suite, format, Clippy with warnings denied, and whitespace checks.

## Left to the builder

- Exact key bindings, within the existing global shortcut rules.
- Exact field names in the view models.
- How the loader thread runs the async client.
