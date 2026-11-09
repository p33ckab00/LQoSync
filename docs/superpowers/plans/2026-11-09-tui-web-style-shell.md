# TUI web-style shell implementation plan

Goal: Start the approved redesign with the shared shell and real navigation.
Reference: `docs/design/tui/README.md` and its approved PNGs.
The owner authorized starting the evolution. Execute this first slice inline.

## First-slice acceptance

- Wide terminals use the approved sidebar groups, top bar, status area, and navigation order.
- Dashboard is the initial screen. Keys 1 through 7 and sidebar clicks select the same routes.
- Routers and LibreQoS navigation opens cached operational status. Missing evidence remains unknown.
- Existing screen bodies and Settings apply controls remain operational during migration.
- Parent-node navigation retains the Network detail view.
- Dialogs and active inputs prevent background mouse navigation.
- The compact layout retains the existing header/body/footer structure and minimum 80x24 support.
- No fixture records, network queries during draw, new dependency, configuration write, or service restart.

## Work

- [x] Read the approved contract and current app, loader, settings, and mouse paths.
- [x] Run baseline tests: 511 pass, zero fail, six manual tests ignored.
- [x] Add failing tests for approved navigation, wide shell, mouse guards, and resize targets.
- [x] Implement shell layout/styles and cached Routers/LibreQoS status bodies.
- [x] Wire navigation and preserve Network detail, Settings, keyboard, and modal behavior.
- [x] Run focused tests, full tests, formatting, Clippy, and review.
- [x] Record the actual first-slice result and remaining work.

## Later slices and open considerations

This first slice is not full screenshot parity. Each screen body still needs its approved layout and interaction implementation.
DPDK traffic needs directional counter provenance, valid deltas, bounded history, and gap/restart handling.
Subscriber online state needs collection evidence, not presence in shaping output.
Settings must retain draft, validation, preview, backup, conflict, reload, and verification gates.
Mouse support requires per-control targets, independent wheel routing, and scrollbar click/drag checks across every screen.
The Journal has no separately approved mockup. Preserve the current Journal behavior until its design is resolved.
The compact layout is transitional. A new compact composition requires its own visual review.
Real-terminal captures and functional acceptance remain required before claiming full mockup parity.
Commit, deployment, and production restart are outside this first-slice task.

## First-slice result

Implemented the shared shell, seven primary routes, guarded mouse navigation, local theme switching,
and temporary cached Routers and LibreQoS bodies. Existing Settings and parent-node workflows remain.
The full suite passes: 516 tests, zero failures, six ignored manual tests.
Clippy with warnings denied and the debug build pass. Formatting and final whitespace checks also pass.
The frame exporter passes and produces 63 synthetic-data frames. An isolated pseudo-terminal smoke check passes.
No production configuration or service changed. Full screen-body parity and complete mouse support remain pending.
