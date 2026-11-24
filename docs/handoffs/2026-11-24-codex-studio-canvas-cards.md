# Codex: Topology Studio canvas cards handoff

Status: ready for Codex. Phase 1 only.
Written by: Claude Code, 2026-11-24 (Asia/Manila). Baseline: `lqosync3` `007a30c`.
Project: `/home/pi/LQoSync3.0`. Owner: the LQoSync operator.

## Read first

1. `docs/superpowers/specs/2026-11-24-topology-studio-canvas-cards-design.md` (the design).
2. `docs/frontend/prototypes/topology-studio-v3/` (the reference images).
3. `docs/handoffs/2026-11-23-codex-topology-frontend.md` (frontend requirements).
4. `docs/superpowers/specs/2026-11-24-topology-sankey-canvas-design.md` (canvas rules).
5. The latest `brain/HANDOFF.md` entry.

## Owner decision

The owner wants one canvas workspace with a router card–driven workflow. Codex builds it.
This change gives Codex the canvas for this work. It supersedes the 2026-11-24 note
"the Codex lane does not change the canvas" for this change only.

## Your scope (phase 1)

Build sections 7.1 to 7.9 of the design. Use only the "Supported" rows of section 8.

Owned files (expected):

- `crates/web-ui/src/topology_canvas.rs` (card hosting, camera moves, event guards)
- `crates/web-ui/src/topology_studio.rs`, `topology_studio_preview.rs`,
  `topology_studio_save.rs`, `topology_source_editor.rs`, `topology_inventory.rs`
- `crates/web-ui/src/router_form.rs` (reuse of the form parts)
- New card components in `crates/web-ui/src/` (names are your call)
- `crates/web-ui/style.css`
- `tools/frontend/*-check.cjs` for the changed screens

Not your scope: `crates/web-contract`, gateway, broker, core service, fixtures. If a
contract field is missing, record the exact need in the return packet. Do not invent it.

## Tasks

Each task ends with its checks green and one commit. Stage explicit paths only.

| ID | Task | Done when |
| --- | --- | --- |
| C0 | Card host in world space: an expanded card in `canvas-layer`, camera move to a card, pan/zoom guards for form targets, 60 % zoom rule, Escape and focus return | Unit test for the event guard. Browser check: typing in a card field does not pan. |
| C1 | Empty canvas and Add router card: inline `NewRouter` fields, draft test stages, "Add router" save, card becomes a router card | Browser check on an empty fixture: add, test, save, router card shows. No dialog opens. |
| C2 | Router card and its settings: Connection, Access, Sources sections; read sources and write grants (WAN address lists only) | Browser check: a read profile and a write profile with different usernames save in the right order. |
| C3 | Discover and tray: collect, table read states, recovery, WAN rows with pin / ignore icons (`wan.candidate_set`) | Browser check: pin two WANs, both show as dashed draft WAN cards. Ignore hides a row. A partial table shows "partial". |
| C4 | WAN card settings inline: name, capacity, factor, latitude, longitude. A draft WAN card saves with `wan.create`. Unpin confirmation with child count. | Browser check: a pinned WAN with capacity becomes a solid WAN card. A capacity edit saves through the current node edit. Unpin of a WAN with children asks first. |
| C5 | Source group cards (profile-first): rate order, plan binding, capacity, parent; "Own rules for this profile" filtered policy | Model unit tests for the policy split and the 32-policy limit. Browser check: preview shows the candidate on the canvas. |
| C6 | Floating action bar and removal of old dialogs: Studio dialog, Add router dialog, Router inventory dialog, Card settings for WAN and group cards | `check-all.sh` green. No `data-panel="studio"`, `studio-add`, or inventory dialog remains. |
| C7 | Evidence and review: screenshots in `docs/frontend/evidence/topology-studio-canvas-cards/`, desktop and narrow width, dark and light | Owner review of the screenshots. |

## Rules

- Keep the current controllers (`preview`, `save`, `StudioMemory`). Move views, not logic.
- Keep secrets in `credentials` only. Clear them when a card closes.
- Show no value that the service did not send. Read section 4 of the design.
- Every icon has a tooltip and an accessible name. Status uses word + icon + color.
- Reserve fixture port `19581` before browser checks. Check with `ss -ltnp`.
- Do not run the full check script at the same time as another lane.
- Production deploy is not part of this work. The owner starts it.

## Return packet (after each task)

Record: task ID, base and head commits, branch, worktree, owned files, check results
(exact command and PASS/FAIL), screenshots, remaining gaps, the next backend need, and an
absolute timestamp with time zone.

## Backend lane (Claude Code) items from this design

Phase 2 waits for an owner go for each item:

- Router information from the test: RouterOS version, board, serial, uptime.
- Write scopes for PPP profile, PPP secret, DHCP lease, and queue.
- The unpin rule when a WAN has topology children (confirm or add).
- WAN ribbon color and description.
- Subscriber data mapping, naming templates, per-subscriber limits, DHCP VLAN association.
