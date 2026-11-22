# Topology canvas integration: baseline and acceptance manifest

Status: Task 0 baseline. Read-only evidence. No service, router, or shaping file changed.
Recorded: 2026-11-22T02:00+08:00 (Asia/Manila) by Claude Code.
Plan: [canvas integration plan](../../../superpowers/plans/2026-11-22-topology-canvas-integration.md).

## Source and worktree

| Item | Value |
|---|---|
| Branch | `lqosync3` in `/home/pi/LQoSync3.0` |
| Implementation base | `2aa4fb0` (feat(traffic): compact the Traffic card header) |
| Uncommitted work at base | Traffic/overview work of another agent (`web-contract`, `web-gateway`, `web-ui` dashboard and traffic files, `src/status.rs`). It is not part of this plan. |

## Installed state (read-only)

| Item | Value | Source |
|---|---|---|
| Service | `lqosync3.service`, active, `run --config /etc/lqosync3/lqosync3.toml` | `systemctl status` |
| Deployed binary | `/usr/local/bin/lqosync3`, `3.0.388+g70f5594.dirty`, SHA-256 below | `--version`, `sha256sum` |
| Web services | `lqosync3-web.service`, `lqosync3-web-broker.service`, both active | `systemctl list-units` |
| Runtime mode | `apply` | `status.json` |
| Target | `v3_pair` (LibreQoS v3, `/opt/libreqos/src`) | `status.json` |
| Config digest | `c3abec6e67014ccb` | `status.json` |
| Native status | `blocked: node PROBE-SITE compiled to 1/1 Mbps, expected 200/100` | `status.json` |
| Sources | `FSEHub-Distro` (PPPoE per plan, DHCP with 4 servers); `FSE-CORE` (WAN only, PPPoE/DHCP off) | `lqosync3.toml` |
| `network.json` | 1.1 KiB, SHA-256 `12efa9b3…9679c2cd` | `/opt/libreqos/src` |
| `ShapedDevices.csv` | 4.0 KiB, SHA-256 `6ae32b83…eede9848` | `/opt/libreqos/src` |

Full binary SHA-256: `45b2f94876c230ecd41e6f81b821685e8028e119fa75044acf3715a6357e0bfe`.

## Unverified items

The current writer is not verified.
`/var/lib/lqosync3/topology-manager/` (it holds `runtime-owner.json`) and `topology.json` have root-only permissions.
`sudo` needs a password in this session. Do not record the writer as topology or legacy until the owner reads that directory.

## Other writers

| Writer | State |
|---|---|
| `lqosync-updatecsv.service` (old Python sync) | disabled, inactive |
| `libreqos-scheduler.service` | active; UISP, Splynx, Netzur, VISP, Powercode, and Sonar integrations are disabled in `/etc/lqos.conf` |

Before a transfer, check again that no other process writes the shaping pair.

## Protected data

- Static records in the live pair that no router source produces. The Task 6 shadow comparison must list each one by stable identity.
- Router coordinates in `lqosync3.toml` (`12.236199459585354, 122.02419129509887`). These are the legacy fallback for Site locations.
- `/var/lib/lqosync3/applied/` (ten previous pairs) and `/var/lib/lqosync3/config-backups/`.

## Supported configurations

- Target `v3_pair` only. Other targets stay on their current path.
- Sources: MikroTik PPPoE (per plan) and DHCP (named servers, bound leases only).
- One LQoSync service instance with the instance lease.

## Backups before a live window

- Copy of `/usr/local/bin/lqosync3` and the web binaries.
- `/etc/lqosync3/lqosync3.toml`.
- `/var/lib/lqosync3/topology-manager/` (registry and owner marker), by the owner as root.
- The complete matching pair from `/opt/libreqos/src` and its write receipt.

## Semantic comparison fields

Circuit ID, device ID, MAC, IPv4 and IPv6 endpoints, resolved parent node, download/upload minimum and maximum rates, node type, node coordinates, node capacities, circuit count, device count, and protected static records.

## Required live events (Task 7)

1. First topology publication under the exclusive lease, with matching import generation and native ACK.
2. One canvas policy edit that applies without a preview or approval step.
3. One layout-only move. It must not change the pair or request a reload.
4. Three no-op cycles with no file change.
5. One service restart. Recovery must complete before the next publication.
6. One source outage simulation (one router unreachable). Saved intent must stay; observations must show stale.
7. One controlled rollback rehearsal by the documented procedure.

## Observation window

At least 24 hours after the first topology publication, with all events above.
A shorter idle interval does not prove reliability.

## Task 4: canvas user interface (first slice)

Recorded: 2026-11-22 (Asia/Manila). Synthetic fixture only. No production service, router, or shaping file changed.

### What the slice contains

- The Builder tab shows the canvas. The tree Builder and its Details column are removed.
- Cards for each node, with wires from the parent port to the child port. A selected card and its wires use the cyan relationship color.
- Pan by scrolling (arrow keys on the focused canvas region). Zoom from 40% to 160% with buttons.
- Drag a card with a pointer, or press Shift and an arrow key on a focused card. Escape cancels a drag. The layout saves 600 ms after the last move through `PUT /api/v1/topology/layout`. A layout save never changes the policy revision.
- A vertical tool dock with four 44 px icon buttons: Router inventory, Topology settings, Subscriber allocation, and Preview and evidence. Each opens a native modal `<dialog>`.
- The node modal (double-click, or Enter on the selected card) has General, Capacity, Location, and Structure sections. A valid change saves automatically through `POST /api/v1/topology/edits`. There is no Review, Save, or Approve button.
- Location accepts a Google Maps paste (`latitude, longitude`) and keeps all decimal places. An invalid value stays as a draft for that node, with an inline error, and is not sent.
- Router inventory runs the existing explicit reads. Discovery adds nothing. A WAN interface or a source goes on the canvas only after the operator selects it.

### Not in this slice

- Plans, speed rules, group factors, and fallback still use the reviewed editors on the Policies tab. The settings modal links to it.
- Drag from the inventory onto the canvas. "Add to canvas" places items under the selected card.
- Equal-count and quality-aware allocation (Task 3). The allocation modal shows them as not available.
- Undo and redo.

### Guidance used

- Vercel frontend-design: one memorable element (wires and ports), existing tokens and IBM Plex, sentence-case labels.
- Google Modern Web Guidance: `ui-components/dialog.md` (native `showModal`, browser focus return and Escape), `ui-behaviors/2d-panning.md` (focusable `role="region"` scroll container), `ui-behaviors/drag-and-drop.md` (pointer capture, Escape cancels, keyboard alternative, `touch-action: none` on the card only).

### Checks

| Check | Result |
|---|---|
| `tools/frontend/topology-canvas-check.cjs` | Pass: 44 px tools, selection, Shift+Arrow layout save with no policy edit, full-decimal paste edit, focus return after Escape and Close, invalid draft kept and not sent, WAN placement after discovery, read-only preview, dark theme, 375 px width with no sideways scroll |
| `tools/frontend/topology-check.cjs` | Pass after the port to canvas |
| `tools/frontend/topology-initialization-check.cjs` | Pass: one-click empty start with no review, second parent for a shared source |
| `tools/frontend/topology-workflow-check.cjs` | Removed: it tested the tree disclosure and search of the removed tree Builder |

Captures: `canvas-light.png`, `canvas-dark.png`, `node-modal.png`, `node-modal-375.png`.

## Task 5: coordinated pair publication

Recorded: 2026-11-22 (Asia/Manila). Source inspection of the installed LibreQoS is read-only.

### Importer finding

The installed LibreQoS does not exclude a mixed pair.
`/opt/libreqos/src/scheduler.py` reads `network.json` and `ShapedDevices.csv` on a reload request and on each scheduled partial reload (`importAndShapePartialReload`, `calculate_shaping_runtime_hash`). It has no pair generation gate.
`lqosd` also loads `ShapedDevices.csv` when the file changes. The `topology_effective_publish.lock.guard` file belongs to the `lqosd` output, not to the input pair.

### Publisher protocol

1. Save the write intent with the SHA-256 of each file (unchanged).
2. Stage every changed file beside its target as `.<name>.stage-<write id>`. Sync each stage and its folder.
3. Replace the targets only after all stages are durable. Record each outcome.
4. On restart, `Tracker::load` completes an interrupted write before any cycle: it installs the stages only when every unfinished file has a stage with the recorded hash. Otherwise it installs nothing and removes the stages.
5. After a completed recovery, the service requests a LibreQoS import, as after a normal write.

Results: a crash before or during staging keeps the previous pair. A crash between the two replacements is completed on restart. A corrupt stage is never installed, and a mixed pair is never reconciled or confirmed.

### Residual risk

During a normal write, the two renames are microseconds apart. A scheduled LibreQoS read in that interval can still read a mixed pair.
Only a reader-side gate in LibreQoS can close this interval. Record this risk in the Task 7 live window decision.

### Checks

| Check | Result |
|---|---|
| `tests/write_receipt.rs` (21 tests) | Pass: staging failure replaces no file, no stage remains, restart completes after the first replacement and before any replacement, interrupted staging keeps the old pair, corrupt stage is never installed |
| `runner::tests::a_restart_completes_an_interrupted_pair_and_asks_for_its_import` | Pass |
| Existing: `superseded_operation_cannot_consume_an_older_proof`, `only_correlated_evidence_can_accept_the_same_operation`, `input_changes_restarts_and_wrong_runtime_evidence_cannot_accept`, `compiler_drift_invalidates_prepared_and_already_constructed_proofs` | Pass: stale ACK, late receipts, digest drift, and compiler drift cannot mark a new revision Applied |
| Existing: `a_replaced_instance_lock_prevents_staging_and_recovery`, `unknown_source_and_owner_changes_block_recovery_and_keep_the_journal` | Pass: lock loss and unknown ownership block writes |

## Task 3 (remainder): discovery, allocation, and shared limits

Recorded: 2026-11-22 (Asia/Manila). Synthetic tests and fixture only.

| Item | State | Proof |
|---|---|---|
| Selective WAN discovery | Done. `ReadRequest.routers` selects router IDs; the service reads and lists only those. Empty means every enabled router. The Inventory modal shows a checkbox per enabled router; WAN routers start selected. | `runner::tests::wan_discovery_reads_only_the_selected_routers`, `a_read_with_too_many_or_empty_router_ids_is_refused`, canvas check |
| Selective source discovery | Not done. Source discovery still reads all enabled subscriber routers, because the collector reads them together. | — |
| Equal subscriber count | Done per source (`group.allocation`). New subscribers go to the placement with the fewest subscribers; a source-pool rebalance equalizes counts; pins stay and count. | `count_mode_*` tests (3), `group_allocation_*`, canvas check |
| Shared bottleneck | A new fixed capacity on one placement of a shared source is refused with an explanation. An older saved policy with this state still opens. | `a_fixed_capacity_on_a_shared_source_placement_is_refused` |
| Quality mode, strict pin failure | Proposal only, in the canvas design. The control stays unavailable. | — |
| Overlapping PPP selectors, incomplete evidence, outages | Existing coverage (identity, source policy, coordinator tests). No change. | Workspace suite |
