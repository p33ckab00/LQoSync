# Network Layout Drag-and-Drop

LQoSync v2.54.3 wires the Network Layout drag-and-drop behavior. Previous versions showed a topology builder with promote/move controls, but node dragging itself was mostly visual/aesthetic and not fully wired.

## What can be dragged

- Visual Topology node cards
- Topology Tree node items

## Drop targets

- Drop on another node to move the dragged node under that target parent.
- Drop on the root drop zone to move/promote the dragged node back to root level.

## Safety validation

The UI blocks unsafe drag moves before changing the preview:

- cannot move a node under itself
- cannot move a node under its own descendant
- cannot move a node to the same parent as a no-op
- cannot move a node where the target parent already has a child with the same name

## Save behavior

Drag-and-drop changes are preview-only until the operator clicks **Save topology**. Save still uses the existing `/api/network_layout/save` endpoint and backend validation before writing `network.json`.

Recommended workflow:

1. Drag node to desired parent.
2. Review the updated visual topology and JSON preview.
3. Click **Save topology**.
4. Run Dry Run to verify generated parent nodes and affected clients.
5. Allow scheduler/auto-apply only after the topology is validated.

## Mobile behavior

HTML5 drag-and-drop is primarily a desktop browser feature. On mobile or touch devices, use the Node Inspector **Move** dropdown instead.
