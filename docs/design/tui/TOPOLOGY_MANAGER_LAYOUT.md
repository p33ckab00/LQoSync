# Topology Manager initial TUI layout

Status: initial layout approved by the owner. Further interface improvements are expected in later reviews.
Recorded: 2026-11-16, Asia/Shanghai.
Requirements: [Topology Manager discussion](../TOPOLOGY_MANAGER_DISCUSSION.md).
Implementation boundaries: [product design](../TOPOLOGY_MANAGER_IMPLEMENTATION.md).

The approval establishes the navigation, views, and panel arrangement as an initial baseline.
It does not freeze the interface or claim usability verification.
Refine presentation and interaction through subsequent wireframe and implementation reviews.
Preserve the approved behavior requirements during refinement.

## Current navigation

`src/tui/app.rs` lists Dashboard, Subscribers, Routers, LibreQoS, Workers, Settings, and Journal.
`src/tui/shell.rs` renders these routes in a left sidebar.
The legacy Network route exists in the enum but is absent from the primary route list.
Decide route reuse during implementation rather than adding competing Network and Topology destinations.

## Primary entry

Add one Topology entry after Routers and before LibreQoS.
Use Topology Manager as the workspace title.
Retain the current shell, theme, status conventions, and terminal-size behavior.
Do not create separate primary routes for inventory, capacities, factors, or balancing.

## Views inside Topology

| View | Purpose |
|---|---|
| Builder | Discovery inventory, selected hierarchy, and selected-item properties |
| Assignments | Subscriber pool, current placement, eligibility, pins, and scoped rebalance |
| Policies | Global and shared-source subscriber min/max controls |
| Changes | Pending/completed changes, revision evidence, warnings, and critical conflicts |

These are the initial approved view labels. Later reviews can refine them without adding manual publication gates.

## Builder at wide terminal sizes

Keep the main route sidebar on the far left.
Arrange three workspace panels from left to right:

1. Discovery Inventory: search, router/source filters, available sources, usage, freshness, and missing state.
2. Topology Tree: synthetic root label, actual instances, hierarchy, capacities, and subscriber counts.
3. Details: selected-item identity, reference, parent, capacity mode, factors, effective ceilings, and scoped actions.

The source inventory must remain distinct from topology instances.
Selection of a source must expose target-parent selection rather than silently attach it everywhere.
Show shared-target placement and explicit single-parent placement as different operations.
One source can have several tree instances, each with its own stable instance ID.

Put global actions such as Create Node and search in the workspace toolbar.
Put Move, Promote, Demote, placement changes, and removal in the selected-item action area.
Keep mode-specific capacity fields in Details, not in a global settings form.
WAN, subscriber-container, and intermediate nodes expose their approved distinct capacity policies.
Removal or movement must show affected members and stop unresolved critical conflicts.

Avoid a terminal graph canvas that relies on dragging.
Use a tree and parent picker so every structural operation works with a keyboard.

## Assignments and policies

Assignments shows current fresh subscriber observations and resolved topology destinations.
Filters include source router, source type, profile/service, WAN, instance, pin, and conflict state.
Selected-subscriber details show source identity separately from current IP/MAC.
Pin/unpin and scoped rebalance follow the approved rules.

Policies contains global Subscriber Speed Policy defaults and shared profile/service overrides.
Each download/upload min/max control shows inheritance, unit mode, value, and resolved Mbps.
Do not copy these subscriber speed controls into every WAN instance's capacity form.
General Settings retains connection, collection, paths, and notification configuration.

## Changes and automatic application

Keep a compact publication strip visible across all Topology views.
Show collection freshness, desired revision, publication phase, and native verification separately.
Do not label an input write as DPDK Applied.
Changes is an optional diff/history view, not a required approval screen.
There is no mandatory Validate or Apply button in this workflow.
Completed valid edits and fresh changed collections enter the automatic pipeline.
Critical conflicts provide a direct path to the affected node, policy, or subscriber.
Background refresh must preserve focus, selection, scroll, and active field input.
Build view indexes when their source revision changes, not on each draw.
Render the current viewport rather than every subscriber or tree row.
Keep source freshness separate from export state and native acceptance.
Expose a publication hold even when the previous native world remains active.

## Keyboard and compact layouts

Use visible focus and a contextual shortcut footer.
Provide keyboard navigation between panels, tree expansion, search, editing, and parent selection.
Keep existing global shortcut and text-entry safeguards. Choose exact new shortcuts during the input-map review.
When space is insufficient, show one focused panel with Inventory, Tree, and Details selectors.
Do not squeeze three unusable panels into an 80-column terminal.
Keep publication/conflict status accessible in compact mode.
Mouse support can mirror keyboard actions, without making drag-and-drop necessary.

## Verification boundary

The owner approves recurring speed schedules with weekday selection, time ranges, and Mbps/Percentage settings.
Location: Policies > Speed Schedules. Allow multiple named schedule profiles.
Each profile has a configurable timezone dropdown, default `Asia/Manila`.
Distinguish schedule profiles from subscriber speed/service profiles in labels and selectors.
Targets include subscriber policies and WAN/group capacities.
Show the selected target and its capacity ceiling in the schedule editor.
Overlap and time-boundary behavior remain under discussion.
Show active schedule and next transition without changing the approved automatic publication workflow.

This is an approved initial layout. Phase 2b-1 (2026-11-18) implements the Topology route, the read-only views, and the strip. Edits follow in Phase 2b-2 to 2b-4.
Review wide and compact wireframes against the required operator tasks during the next interface refinement.
Implementation checks must cover focus retention, edit completion, invalid fields, conflicts, keyboard reach, and terminal resize.
