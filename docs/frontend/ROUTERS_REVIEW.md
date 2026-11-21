# Routers design and plan review

Date: 2026-11-21. Reviewed HEAD: `45a1be7`. Reviewed documents: [design](ROUTERS_DESIGN.md) and [implementation plan](ROUTERS_IMPLEMENTATION_PLAN.md).
Method: each design claim was compared with the current backend source. Guidance: Vercel frontend-design, the ui-ux rule set, Google modern-web-guidance, and Microsoft Fluent 2 interaction guidance.

## Verdict

The design direction is correct: an inventory table, a detail route with tabs, local registration, a reviewed disable, and no router delete.
The plan has eleven backend gaps (B1 to B11) and seven scope problems (P1 to P7).
The revised slice order at the end of this document replaces the R00 to R07 order. R08 and R09 do not change.

## Backend gaps

| ID | Gap | Evidence | Required change |
| --- | --- | --- | --- |
| B1 | The inventory has no complete source. `collection.json` lists only routers that read subscribers. A WAN-only core router (for example FSE-CORE) is not in it. The Dashboard Routers count has the same error. | `src/collection.rs` `Report::collect`; `src/config.rs:349`; `crates/web-gateway/src/overview.rs:95` | Add a core API operation `routers.list`. It merges the configured routers, the catalog WAN sources, the collection evidence, and `status.json`. Use the same count on the Dashboard. |
| B2 | The web has no path to router configuration. The gateway does not depend on the core crate. The broker reaches the core service only through the root-only `api.sock`. | `crates/web-gateway/Cargo.toml`; `src/api/server.rs` | Add core API operations `router.create` and `router.update`. They reuse the settings draft, secrets, and apply code, then reload in the service. Move that code out of `tui::settings` into a shared module. Do not change the TUI workflow. |
| B3 | The router-wide `enabled` flag is a compatibility risk. `RouterCfg` uses `deny_unknown_fields`. An older binary cannot load a file that has the new field. | `src/config.rs:323` | Default `enabled` to true. Write the field only when it is false. Record the downgrade limit in the release note. Show and keep the flag in the TUI. |
| B4 | The legacy removal guard is not in the disable design. A large removal in one cycle is held until the next confirmation. A disable of a large access router trips the guard. | `src/cycle.rs:335` | The disable preview calculates the guard result. The result shows "Held by the removal guard" as its own stage. The confirmation does not bypass the guard. |
| B5 | Production uses the legacy writer. Topology Manager activation is on standby. R04 describes only the registry and compiler path. | Phase 4 activation plan status | Define disable for both writers. Legacy writer: the router is not read and its subscribers are not carried. Topology Manager: source removal through the compiler. Do the legacy writer first. |
| B6 | The legacy cycle keeps the nodes of a router that is not online. A disabled router must not use this carry rule, or its circuits stay. | `src/topology.rs` `build_nodes` (`carried`) | Add an explicit "disabled, not carried" case and a test for it. |
| B7 | No single-router read exists. `SIGUSR1` (sync now) reads all routers. | `src/tui/dashboard.rs:291` | First version: "Sync now" for all routers, with an honest label. A connection test is a new minimal operation: connect, TLS, login, and one identity read. A per-router source read is later work. |
| B8 | The core API has no actor field and no audit log. The Activity tab has no data source. | `src/api/protocol.rs`; `src/api/dispatch.rs` | The broker sends the signed-in user as `actor`. The core service appends each router action to a bounded JSON-lines audit file in the state directory, with rotation at 1 MiB. |
| B9 | A new TLS router needs a CA file, but `tls_ca_file` is a server path. The browser must not see paths. The broker request limit (4096 bytes) blocks a PEM upload. | `src/config.rs:332`; `crates/web-gateway/src/broker.rs:64` | Add a managed CA store, `<state>/router-ca/<router-id>.pem`, with one dedicated upload operation up to 16 KiB. The owner must approve this (decision D1). |
| B10 | Connection errors have no types. TCP, TLS, and login failures are all `anyhow` text. | `src/routeros.rs` | Add an error class at connect time: unreachable, TLS, certificate, login, timeout, and protocol. |
| B11 | Deep links do not work. The gateway serves `/` only, so `/routers/...` returns 404. The web UI has no client router; the navigation links are static. | `crates/web-gateway/src/http.rs:251`; `crates/web-ui/src/dashboard.rs:78` | Serve the index for `/routers` and its sub-paths. Add one route signal with the History API. Do not add a router crate. |

## Scope problems

| ID | Problem | Change |
| --- | --- | --- |
| P1 | Cursor pagination with snapshot IDs is too much for an ISP router fleet of fewer than 100 routers. | Send one bounded list: up to 200 summaries, at most 60 KiB. Search and filter in the browser. Keep a test for the cap. |
| P2 | R00 freezes every state and command before the first useful screen. | Ship a read-only inventory first. It does no router I/O, so it does not need the gate. |
| P3 | The plan waits for the Dashboard owner. The Dashboard work is complete. | Remove the coordination constraints. |
| P4 | The design does not cover all router-scoped settings: exclusions, location (latitude and longitude), PPPoE per-plan nodes, Hotspot limits and factors, and the DHCP server options. The linked feature screens do not exist. | The Sources tab edits the router-scoped source options. A Settings section holds exclusions and location. |
| P5 | "Disabling" waits for a quiescence acknowledgement that the legacy writer cannot give per router. | The legacy cycle checks the gate at cycle start and before each router read. "Disabling" ends when the current cycle ends. |
| P6 | A full disabled WAN pinning form can look like a working feature. | Keep the owner decision, but show the form as a read-only outline with a prerequisite list: writer, DPI provider, and core WAN target. Show no input fields until the writer exists. |
| P7 | A 5-second inventory poll repeats the same data. The router data changes once per cycle. | Fetch the inventory again when `cycle_seq` changes in the existing overview stream, and when the operator returns to the tab. |

## Interface improvements

These rules come from the ui-ux floor (WCAG 2.2 AA) and the Fluent interaction guidance.

- Show each state as an icon and a text label. Do not use color alone.
- Use one primary action on the inventory: Add router. Put Disable in the detail route and in a row menu, in a separate danger group.
- Show the Add router steps with a step indicator. Back keeps the entered values. Validate a field when it loses focus. Show the error below the field. After a failed save, move focus to the first invalid field.
- Use `autocomplete="new-password"` and a show/hide control for the credential. Never store a credential in browser storage. A blank field keeps the current credential.
- The disable dialog shows the impact numbers first: circuits removed, circuits kept, and the guard result. Confirm with a check box that repeats the circuit count. Return focus to the trigger when the dialog closes.
- Show job stages in place: Saved, Reloaded, Disabled, Shaping removal applied. A toast only links to the result.
- An empty inventory shows one sentence and Add router.
- After a route change, move focus to the page heading. Back restores the filters and the scroll position.
- Use tabular figures for counts and ages. Use 13 px as the smallest readable text.
- Mark a router that has `tls_verify = false` with the text "Certificate not checked".

## Owner decisions (2026-11-21)

- D1, approved: the browser can upload a CA certificate (PEM, up to 16 KiB) to the managed CA store (B9). The browser sees only "CA configured".
- D2, approved: the web asks for the current password again before a disable and before a credential replacement.
- D3, clarified: the no-read and no-write rule applies to a disabled router. An enabled core router can receive the approved R09 address-list writes after lab evidence.

## Change from the review: the inventory is a published cache

The service answers `api.sock` requests only between cycles (`src/main.rs`, `serve_until`). An inventory read through the API waits for the current cycle.
S1 therefore uses the existing cache pattern. At the end of each cycle, the service writes `routers.json` (mode 0600) in the state directory.
The broker reads that file, the same as `collection.json`. Mutations (S3 and later) use `api.sock`.
The file uses the `web-contract` router types, so the core and the gateway share one schema.

## Revised slice order

Status: S1 is implemented. The service writes `routers.json` each cycle. `GET /api/v1/routers` and the `/routers` page read it.
`tools/frontend/routers-check.cjs` holds the browser evidence.
S2 is implemented. `[[routers]] enabled = false` closes four paths: the legacy cycle read, WAN discovery, the Topology Manager batch (which also closes a held session), and the connector itself.
A disabled router keeps no rows and no nodes: its circuits leave after the grace time, through the removal guard. The TUI shows the key per router. `router_defaults.enabled` is not allowed.
S3 is implemented. The broker runs `lqosync3 router-edit` (the TUI Settings draft, check, and apply path) instead of an `api.sock` operation, so a save does not wait for the cycle.
The broker unit can write `/etc/lqosync3` and `/var/lib/lqosync3` for this command. The Activity tab reads `router-audit.jsonl`; it lists web changes only, not TUI changes.
Not in S3: list editors for exclusions, DHCP server settings, and DHCP client overrides. They stay in the TUI.
S4 is implemented for the legacy writer. The inventory carries the circuit count of each router and the removal rules (grace time, `remove_inactive_entries`, guard limits, a held removal).
The disable review shows the impact first, needs an acknowledgement when circuits leave, and needs the operator password (D2). The result has two tracks: router connections and the shaping removal.
Enable removes the `enabled` key and needs no password. The Topology Manager path waits for activation; a batch that selects a disabled router fails before any connection (S2).
S5 is implemented. The service runs the connection test (`router.test` on `api.sock`): TCP, TLS, sign-in, and one identity read, with its own session and the router time limits. The answer is a class only (B10), and the audit log gets one line.
The broker keeps `RestrictAddressFamilies=AF_UNIX`: it never connects to a router. Sync now runs `lqosync3 sync-now` (SIGUSR1) for all enabled routers; a per-router source read stays future work.
S6 is implemented. The WAN pinning tab shows two checks on current data (a catalog WAN link and the enabled state), four future packages as "Not available" with their reasons, and the planned form as text. It has no input and no action.

The first Routers slice (S1 to S6) is complete. Open work:

- R08 address evidence and R09 address-list writer. R09 needs lab evidence and owner approval (D3).
- The Topology Manager disable path, after activation.
- A per-router source read (Sync now reads all routers).

S7a is implemented. The Sources tab edits the three exclusion lists (one entry for each line) and the DHCP client overrides (rows with a MAC address, two rates, and a name).
A list that is sent replaces the whole list. Only the one-router detail (`GET /api/v1/routers/{id}`) carries the lists; the inventory list stays small.
When the detail must cut a long client list to stay under 60 KiB, it flags the list and the page refuses to edit it, so a save cannot remove entries that the page did not get.
S7b is implemented. The Sources tab edits the DHCP server settings (name, mode, speed sources in order, plan defaults, and factors) and the mode of a server without its own settings.
The speed sources are one text field, so the order of an existing list stays. The config rules check a repeated name, an empty source list, and lease_comment in per_site mode. All router-scoped settings are now in the web UI. The drawer from the design is not built: the row link opens the detail route, which has the same content and a stable URL.

Each slice ends with tests, browser evidence, a commit, and a release build.

| Slice | Content | Router I/O |
| --- | --- | --- |
| S1 | Read-only inventory and detail: a `routers.json` cache from each cycle (B1), broker and HTTP GET, deep links (B11), inventory table, detail drawer, Overview, Connection, and Sources tabs as read-only. Correct the Dashboard Routers count. | None |
| S2 | Router-wide `enabled` gate (B3, B6): legacy cycle, WAN discovery, coordinator, TUI display, and compatibility tests. | None (it removes I/O) |
| S3 | Local create and update (B2, B8, B9): core operations, secrets, managed CA store, audit log, Add router form, Connection and Sources editors, Activity tab. New routers are disabled. | None |
| S4 | Disable and enable with a preview (B4, B5): legacy writer first, removal guard stage, two result tracks, disable dialog. | None (it stops I/O) |
| S5 | Connection test and Sync now (B7, B10): typed errors and job results. | Reads only |
| S6 | WAN pinning surface (P6): capability panel and prerequisite list. | None |
| R08, R09 | Unchanged future packages. R09 needs approval D3. | R09 writes |
