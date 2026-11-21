# Topology web views: read-only release (T00–T03)

Date: 2026-11-21, Asia/Manila. Plan: [TOPOLOGY_IMPLEMENTATION_PLAN.md](../../TOPOLOGY_IMPLEMENTATION_PLAN.md).
The screenshots contain synthetic fixture data only.

## Scope

This release adds these read-only views:

- Topology: Builder, Assignments, Policies, and Changes (`/topology?tab=...`).
- PPPoE: Users and Local plans (`/pppoe`, `/pppoe?view=plans`).
- IPoE / DHCP: Clients and Local plans (`/ipoe`, `/ipoe?view=plans`).

No view writes policy. No view reads a router. Write actions show an "arrives with the write release" note.

## Data path

1. At the end of each cycle, the service writes `topology.json` (mode 0600) beside `routers.json`.
   The file holds one snapshot: the registry revision, the cached preview, and the cycle collection.
2. The broker reads the file and answers one bounded reply (64 KiB or less).
3. The gateway serves these authenticated GET routes:
   - `GET /api/v1/topology`: evidence, tree, sources, plans, collection health, and counts.
   - `GET /api/v1/topology/assignments`: saved subscriber records, filtered, then paged.
   - `GET /api/v1/topology/observations`: PPP and DHCP rows, filtered, then paged.
   - `GET /api/v1/topology/subscribers/{id}`: one subscriber and its linked rows.

A cursor holds the snapshot ID, a filter digest, and an offset.
A cursor from another snapshot or filter gives `409`. The page then loads its first page again.

## Identity links

A PPP or DHCP row links to a saved subscriber only through the reference rule of its source:

- `PppSecretName`: `ppp-name/v1/<encoded secret name>`.
- `DhcpClientMac`: `dhcp-mac/v1/<normalized MAC>`.

The link state is `none` when no saved record has that reference.
The link state is `unknown` when the evidence cannot decide:

- The source uses `OperatorManagedComment`. The collection does not hold the comment.
- A PPP source has service-name filters. The collection does not hold the PPP service.
- More than one source selection matches the row.

The view never links a row by IP address, by MAC address alone, or by display name.
A failed router read makes the rows of that router `stale`.

## Checks

| Check | Result |
|---|---|
| `cargo test --locked --lib web_view` | 11 passed, including a 50,000-subscriber projection (145 ms release, 19.5 MB) |
| `cargo test --locked -p web-gateway --test topology` | 7 passed |
| `cargo test --locked -p web-gateway --test http` | Passed: 401, 503, 200, 404, 409, and 405 for POST |
| `cargo test --locked -p web-ui` | Route and model tests passed |
| `node tools/frontend/topology-check.cjs` | Passed in dark and light themes |

The browser check covers these items:

- Sidebar navigation and `aria-current`.
- The evidence strip.
- Tree order and node selection.
- Assignments filters and the subscriber deep link.
- PPPoE and IPoE rows, link filters, and observation-only rows.
- One subscriber with a PPPoE and a DHCP identity, which opens the same record from both views.
- Plan counts: source identities and unique subscribers stay separate.
- Widths of 320 px and 390 px with no page-level horizontal scroll.
- Only GET requests to the topology routes.

## Screenshots

- [Builder](topology-builder-dark.png)
- [PPPoE Users with the selected subscriber](pppoe-dark.png)
- [IPoE Local plans](ipoe-plans-dark.png)

## Limits of this release

- Files/import and native verification show "Not in this release". T10 adds that evidence.
- Speed defaults and source factors do not show. Policies shows plans and fallback only.
- The broker reads the whole cache on each request. Large networks can make reads slower.
- The summary keeps the first nodes that fit one reply and marks the rest as truncated.
- Refresh, discovery, edits, plans, pins, and rebalance are T04–T09.
