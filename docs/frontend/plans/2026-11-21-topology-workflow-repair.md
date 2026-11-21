# Topology workflow repair plan

**Goal:** Make the Builder the clear policy editor for automatic topology compilation and subscriber hierarchy.

**Design:** Keep IBM Plex Sans and the current Light, Dark, and System tokens.
Use a compact Inventory / Hierarchy / Details workspace from the documented Builder reference.
Use nested branches, disclosure controls, a synthetic Network root, and readable parent/source names.
Keep discovery results separate from subscriber collection so a WAN-only router remains understandable.
Use a source table with a selected detail column for PPPoE and IPoE/DHCP.

## Acceptance

- [x] The service cycle uses the topology compiler when durable writer ownership selects topology.
- [x] Topology failures cannot fall back to a legacy writer or overwrite shaping files.
- [x] A cycle updates source collection evidence without a second router collection.
- [x] WAN discovery reports each enabled router, including empty and failed results.
- [x] The hierarchy supports nested disclosure, search with ancestors, selection, and contextual child/instance actions.
- [x] Cached UI refresh does not issue router reads or erase drafts, selection, or disclosure state.
- [x] PPPoE and IPoE show endpoint/MAC fields, reliable filters, selected details, and explicit shared policy actions.
- [x] Source filters reject late replies and search the complete server-side result set.
- [x] Themes, narrow widths, keyboard controls, proposal review, and publication evidence remain functional.

## Boundaries

The existing Runner owns collection, registry policy, compilation, and publication.
The browser reads caches and sends explicit reviewed changes. It does not publish files or run another compiler.
Durable activation, mode, enrollment, lease, revision, and publication checks remain mandatory.
The explicit `topology activate` CLI supplies the activation path, with preview and `--yes` confirmation.
Do not infer WAN routing from shaping placement.
Do not deploy, change production configuration, or write router settings as part of this source repair.

## Work ownership

- Core: `src/runner.rs`, its topology cycle helper/tests, and source coordinator reuse.
- Source UI: `subscriber_views.rs`, gateway observation filters/tests, and a focused browser check.
- Builder: `topology.rs`, `topology_model.rs`, `topology_forms.rs`, shared CSS additions, and browser evidence.
- Preserve all pre-existing Routers changes in shared CSS and notes.

## Validation

Run focused failing regression tests before each behavior fix.
Run core, contract, gateway, and UI tests, Wasm/native Clippy, formatting, and the release browser build.
Exercise the existing topology suite and the new workflow checks with synthetic fixtures.
Inspect desktop/narrow screenshots in both themes. Record deployment limits separately from source checks.

## Guidance

[Vercel frontend-design](https://github.com/vercel-labs/open-agents/blob/main/.agents/skills/frontend-design/SKILL.md) guides hierarchy and controlled density.
[Vercel interface rules](https://github.com/vercel-labs/web-interface-guidelines/blob/main/command.md) guide accessible controls and stable state.
[Google Modern Web Guidance](https://github.com/GoogleChrome/modern-web-guidance) supplies `css-layout`, `responsive-table`, and `accordion` guidance.
Use named grid regions and `min-inline-size: 0`. Keep semantic tables and keyboard-scrollable regions.
Use ordinary nested lists and disclosure buttons instead of an incomplete ARIA tree.
