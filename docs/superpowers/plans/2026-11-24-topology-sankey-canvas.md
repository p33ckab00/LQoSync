# Topology Sankey Canvas Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace the Topology canvas with a Sankey canvas. Ribbons show exact live throughput, one colored stream for each WAN, through automatic columns.

**Architecture:** The service adds the routing WAN of each active subscriber to the topology view. The gateway throughput endpoint sums host rates for each (card pair, WAN). A new pure Rust module in `web-ui` calculates columns, card boxes, and ribbon paths. The canvas renders them as SVG. The tree-exclusion rule accepts every intermediate node, so display-only cards (core router, LibreQoS) stay out of `network.json`.

**Tech Stack:** Rust, Leptos (Wasm), axum gateway, Playwright browser checks (`tools/frontend/*.cjs`).

**Spec:** `docs/superpowers/specs/2026-11-24-topology-sankey-canvas-design.md`

## Global Constraints

- Do not show a value without a data source. No CPU, drop rate, queue count, or utilization.
- Add no JavaScript library and no Rust crate.
- Every new contract field has `#[serde(default)]`. Old caches must still parse.
- Status = word + icon + color. Every icon control has an accessible name and a tooltip.
- Modals and the popover trap focus, close on Escape, and return focus to the opener.
- Respect `prefers-reduced-motion`. No horizontal scroll at 390 px width.
- Apply Vercel frontend-design (`~/.agents/skills/frontend-design/SKILL.md`) and Google Modern Web Guidance to Tasks 5–7. Name the guides in the evidence README.
- Work in worktree `/home/pi/LQoSync3.0-worktrees/claude-topology-sankey`, branch `claude/topology-sankey`, from `lqosync3` HEAD. Codex shares the repository: stage explicit paths only.
- Fixture port 19581. Before a browser run, check `ss -ltnp` and stop only the fixture PID.
- Commit messages: imperative summary ≤ 50 characters, STE body, attribution line.

## Review Focus

1. **Tree WAN and routing WAN disagree** for one host. Expected: the stream color is the WAN ancestor in the tree (the shaping path), so every WAN card stays balanced. Test in Task 2.
2. **A host with no subscriber, or a subscriber with no WAN.** Expected: unmapped hosts stay out of ribbons. A host without WAN goes in a `wan: None` stream, drawn grey as "WAN unknown". Tests in Tasks 2 and 4.
3. **The sample is stale or absent.** Expected: ribbons fall back to subscriber counts, cards show subscribers, and the chip says "Subscribers, not live". Tests in Tasks 4 and 6.
4. **A refresh changes the values.** Expected: cards keep their column order and do not jump. Test in Task 4.
5. **More than 512 hosts** (sample row limit). Expected: chip "Partial" with the reason. Test in Task 6.

---

### Task 1: Routing WAN in the view and in the flow contract

**Files:**
- Modify: `crates/web-contract/src/topology.rs` (`Active`, `FlowThroughput`, `Node.tree_excluded` doc comment)
- Modify: `src/topology_manager/web_view.rs:196-210`
- Modify: every other `Active {` and `FlowThroughput {` literal (`rtk grep -rn "Active {\|FlowThroughput {" --include=*.rs src crates`)
- Modify: `crates/web-gateway/examples/browser_fixture.rs` (give active fixture subscribers a `wan`)
- Test: `src/topology_manager/web_view_tests.rs`

**Interfaces:**
- Produces: `Active.wan: Option<String>` (WAN node ID from `RoutingState.assigned`), `FlowThroughput.wan: Option<String>`. Both `#[serde(default)]`.

- [ ] **Step 0: Create the worktree and record the lane change**

```bash
cd /home/pi/LQoSync3.0 && git worktree add ../LQoSync3.0-worktrees/claude-topology-sankey -b claude/topology-sankey lqosync3
```

Add a dated entry at the top of `brain/HANDOFF.md`: the owner moved the Topology canvas UI from Codex to Claude on 2026-11-24, with the branch, the worktree, and the spec path. Commit the spec, this plan, and the HANDOFF entry together.

- [ ] **Step 1: Write the failing test** in `web_view_tests.rs`, next to `view_shows_routing_lists` (use its input builder and the `RoutingState` pattern at line 937):

```rust
#[test]
fn view_gives_each_active_subscriber_its_routing_wan() {
    // RoutingState.assigned = {"<subscriber with a WAN>": "<wan node id>"}
    // assert_eq!(active(&view, "<subscriber with a WAN>").wan.as_deref(), Some("<wan node id>"));
    // assert_eq!(active(&view, "<subscriber without a WAN>").wan, None);
    // With routing: None, every Active.wan is None.
}
```

- [ ] **Step 2: Run it.** `cargo test -p lqosync3 view_gives_each_active_subscriber_its_routing_wan`. Expected: FAIL, no field `wan`.
- [ ] **Step 3: Add the two fields.** In `web_view.rs`, set `wan: input.routing.and_then(|r| r.assigned.get(id).cloned())`. Add `wan: None` to every other literal. Change the `tree_excluded` doc comment to "An intermediate node that `network.json` does not contain."
- [ ] **Step 4: Run** `cargo test --workspace` and `cargo clippy --workspace --all-targets -- -D warnings`. Expected: PASS.
- [ ] **Step 5: Commit** `feat(topology): send the routing WAN of each host`

### Task 2: Per-WAN flows in the throughput endpoint

**Files:**
- Modify: `crates/web-gateway/src/throughput.rs` (`compute`, tests)

**Interfaces:**
- Consumes: `Active.wan`, `FlowThroughput.wan` (Task 1).
- Produces: `TopologyThroughput.flows` with one entry for each `(from card, to card, wan)`, sorted by `(from, to, wan)`. `NodeThroughput` does not change.

Rule for one host (spec 7.1, with the Review Focus 1 decision):
1. `path` = the tree ancestors of the host node, from the root down to the node.
2. `wan` = the nearest node of kind `wan` on `path`. If `path` has none, `wan` = `Active.wan`. Else `None`.
3. Add the host rate to each consecutive card pair on `path` (skip a pair with the same card twice), under `wan`.
4. If `path` has no node of kind `wan` and `wan` is `Some(w)`: add the rate to the pair (`card_of(w)`, card of `path[0]`), under `wan`.

- [ ] **Step 1: Write the failing tests** (extend the `view()` helper with a `wan` argument on `sub` and a `node` of kind `intermediate`):

```rust
#[test] fn flows_split_by_wan_along_the_whole_path()
// wan-1 → distro → ppp-1 and wan-2 → distro → ppp-2, ppp cards = "source:ppp".
// assert flows contain ("wan-1","distro",Some("wan-1"),30e6), ("wan-2","distro",Some("wan-2"),30e6),
//        ("distro","source:ppp",Some("wan-1"),30e6), ("distro","source:ppp",Some("wan-2"),30e6)

#[test] fn a_display_parent_before_the_wan_carries_the_wan_stream()
// core (intermediate, tree_excluded) → wan-1 → ppp-1: flow ("core","wan-1",Some("wan-1"),30e6)

#[test] fn the_routing_wan_adds_the_first_edge_when_the_tree_has_no_wan()
// flat tree distro → ppp-1, Active.wan = "wan-1": flows ("wan-1","distro",Some("wan-1")) and ("distro","source:ppp",Some("wan-1"))

#[test] fn the_tree_wan_wins_when_routing_disagrees()
// path has wan-1, Active.wan = "wan-2": every flow of the host has wan Some("wan-1")

#[test] fn a_host_without_wan_has_a_none_stream()
// flat tree, Active.wan None: flows have wan None; unmapped_hosts unchanged

#[test] fn in_equals_out_for_every_card_and_wan()
// for each (card, wan) with both incoming and outgoing flows: sum in == sum out
```

Update `rates_roll_up_to_each_node_and_follow_the_canvas_flows` to expect `wan: Some("wan-1")` and `Some("wan-2")`.

- [ ] **Step 2: Run** `cargo test -p web-gateway throughput`. Expected: the new tests FAIL.
- [ ] **Step 3: Implement the rule** in `compute`. Key `lines` by `(String, String, Option<String>)`.
- [ ] **Step 4: Run** `cargo test -p web-gateway` and Clippy. Expected: PASS.
- [ ] **Step 5: Commit** `feat(web): split live flows by WAN`

### Task 3: Display-only intermediate nodes

**Files:**
- Modify: `src/topology_manager/edits.rs:1946-1956` and the `node.router` check that refuses a router-less excluded node
- Test: `src/topology_manager/edits_tests.rs:1917` (rename the test), the publication test that covers `tree_excluded`

**Interfaces:**
- Produces: `node.tree_exclude` accepts every node of kind `intermediate`. It refuses `wan` and `group` with the message "only an intermediate node can be excluded from the network tree".

- [ ] **Step 1: Write the failing tests**

```rust
#[test] fn tree_exclusion_accepts_intermediate_nodes_only()
// tower (intermediate, no router_id): exclude → Ok, tree_excluded == true
// a wan node and a group node: refused, error contains "intermediate node"
// node.router null on an excluded intermediate node: Ok (it stays excluded)

#[test] fn a_wan_node_can_have_a_display_only_parent()
// add intermediate "core", set it as parent of a WAN node, exclude "core":
// publication output: the WAN has no parent, "core" is not in network.json
```

- [ ] **Step 2: Run** `cargo test -p lqosync3 tree_exclusion` and `cargo test -p lqosync3 wan_node_can_have`. Expected: FAIL.
- [ ] **Step 3: Change the rule** from `router_id.is_none()` to `kind != "intermediate"`. Remove the "leaves its router" refusal for excluded nodes.
  If `a_wan_node_can_have_a_display_only_parent` fails because a tree rule refuses a parent for a WAN node, STOP. Report the rule and its file to the owner (spec 7.2a). Do not change that rule without the owner.
- [ ] **Step 4: Run** `cargo test --workspace` and Clippy. Expected: PASS.
- [ ] **Step 5: Commit** `feat(topology): exclude any intermediate node`

### Task 4: Sankey layout model

**Files:**
- Create: `crates/web-ui/src/sankey.rs` (pure Rust, no Leptos imports, unit tests in the same file)
- Modify: `crates/web-ui/src/main.rs` (`mod sankey;`)

**Interfaces:**
- Consumes: `TopologySummary`, `TopologyThroughput` (contract).
- Produces:

```rust
pub enum Metric { LiveMbps, Subscribers }
pub struct Segment { pub from: String, pub to: String, pub wan: Option<String>, pub value: f64 }
pub struct Input { pub cards: Vec<String>, pub segments: Vec<Segment>, pub wans: Vec<String>, pub metric: Metric }
pub struct CardBox { pub id: String, pub column: usize, pub x: f64, pub y: f64, pub w: f64, pub h: f64, pub value: f64 }
pub struct Ribbon { pub from: String, pub to: String, pub wan: Option<String>, pub value: f64, pub d: String }
pub struct Sankey { pub cards: Vec<CardBox>, pub ribbons: Vec<Ribbon>, pub columns: usize, pub cycle: bool, pub bounds: [f64; 4] }

/// Live flows when `t` is fresh, else subscriber flows with `wan: None`. Adds the service tier
/// cards `tier:ppp`, `tier:dhcp`, `tier:hotspot` (from `Source.access` of each group node).
pub fn input(summary: &TopologySummary, t: Option<&TopologyThroughput>) -> Input
/// `order`: the card order of the last layout, for stable columns.
pub fn layout(input: &Input, order: &[String]) -> Sankey
```

Values (builder may tune after the visual check): `CARD_W = 232.0`, `COL_GAP = 168.0`, `GAP = 14.0`, `MIN_H = 72.0`, tallest column band total = `720.0` px.
Card value = `max(sum in, sum out)`. Card height = `max(MIN_H, value × scale)`. Streams stack on each card edge in `wans` order, `None` last, centered in the card. Column = longest path, with a visit guard. Order in a column: index in `order` if present, else barycenter of parent centers, then ID. Ribbon `d` = a closed path: top cubic, line down, bottom cubic back, `Z`.

- [ ] **Step 1: Write the failing tests** in `sankey.rs`:

```rust
#[test] fn columns_follow_the_longest_path()
// core→wan→lqos→distro→grp: columns 0..=4. A second grp directly under wan with an edge from distro: column 4.

#[test] fn bands_fill_card_edges_without_overlap()
// for each card edge: stream intervals do not overlap, and the sum of band heights == value × scale

#[test] fn streams_stack_in_wan_order_with_none_last()

#[test] fn a_cycle_stops_and_sets_the_flag()
// a→b→a: layout returns, cycle == true, both cards placed

#[test] fn order_is_stable_between_refreshes()
// layout twice with changed values and the first order: same order in every column

#[test] fn a_zero_value_card_keeps_the_minimum_height()

#[test] fn a_stale_sample_gives_subscriber_segments()
// input(summary, Some(stale)) → metric Subscribers, every wan None, values = Flow.subscribers

#[test] fn groups_feed_their_service_tier_card()
// group with Source.access "ppp" → segment (group card, "tier:ppp")
```

- [ ] **Step 2: Run** `cargo test -p web-ui sankey`. Expected: FAIL (module empty).
- [ ] **Step 3: Implement** `input` and `layout`.
- [ ] **Step 4: Run** `cargo test -p web-ui` and Clippy for native and `--target wasm32-unknown-unknown`. Expected: PASS.
- [ ] **Step 5: Commit** `feat(web): add the Sankey layout model`

### Task 5: Sankey canvas render

**Files:**
- Modify: `crates/web-ui/src/topology_canvas.rs` (`Canvas`), `crates/web-ui/src/topology_canvas_model.rs`, `crates/web-ui/style.css`
- Modify: `crates/web-ui/src/canvas_layout.rs` (delete `layout`, `layout_in`, `edge_path` and their tests when nothing uses them; keep `fit`, `clamp_zoom`)
- Modify: `tools/frontend/topology-canvas-check.cjs`

**Interfaces:**
- Consumes: `sankey::input`, `sankey::layout` (Task 4); `GET /api/v1/topology/throughput` → `TopologyThroughput`.
- Produces: `Canvas` keeps its props. It exposes `data-metric="live|subscribers"` on the canvas root for Task 6 and the browser check.

- [ ] **Step 1: Change the browser check first.** In `topology-canvas-check.cjs`, remove the sections for card drag, Shift+Arrow move, connector drag, layout direction, and "Remove link". Add:
  - one `path.sankey-ribbon` for each fixture flow, each with `data-wan`, and a non-empty `d` that ends with `Z`;
  - cards in `.sankey-card` with `data-column`; the WAN cards have the lowest column of the fixture chain;
  - click a card → `[role=dialog].sankey-popover` with the card name, a line for each WAN, and a button "Open full settings". The button opens the current card modal. Escape closes the popover and focus returns to the card;
  - click a ribbon → popover text "WAN North → … → …" with Mbps down and up;
  - Tab reaches each card; Enter opens the popover;
  - "Fit view" shows all cards inside the viewport (bounding boxes);
  - no `.sankey-card` box overlaps another one.
- [ ] **Step 2: Run** `STAGES=browser CHECKS=topology-canvas-check tools/frontend/check-all.sh`. Expected: FAIL.
- [ ] **Step 3: Render the Sankey.**
  - Fetch the throughput with `get::<TopologyThroughput>(session, "/api/v1/topology/throughput")` every 5 s while the tab is open. Keep the last good value.
  - Keep the order of the last layout in a `StoredValue<Vec<String>>` and pass it to `layout`.
  - SVG: one `<linearGradient>` for each WAN; ribbons below cards; a highlight class on hover and focus; other streams dim.
  - Remove card drag, connector drag, Shift+Arrow move, the direction setting, and "Remove link". Do not read `Node.position`.
  - Keep pan, zoom, "Fit view", "100%", the routing warning, and the current card modal.
  - Add a minimap at the bottom right (the canvas has none today): the card boxes at small scale and a rectangle for the viewport. A click on the minimap pans to that point. It has `aria-hidden="true"`; "Fit view" is the keyboard path.
  - Card content: role eyebrow, name, rate (`412 Mbps`) or `21 subs` for `Metric::Subscribers`, one meta line (hosts, median RTT, capacity when the node has one). A tree-excluded card shows "Display only · not shaped". A card without data shows "No live data".
  - WAN colors: WAN 1 `#4fd1e8`, WAN 2 `#2bd391`, then `#80aaff`, `#c792ea`, `#ff9e64`. A backup WAN (name or role marks it) and the "WAN unknown" stream: amber `#f2c041` and grey `#8d909d`.
- [ ] **Step 4: Run** `cargo test -p web-ui`, both Clippy runs, and the browser check of Step 2. Expected: PASS.
- [ ] **Step 5: Commit** `feat(web): draw the topology as a Sankey`

### Task 6: Toolbar, legend, and data states

**Files:**
- Modify: `crates/web-ui/src/topology.rs` (`TopologyPage` tabs and toolbar), `crates/web-ui/style.css`
- Modify: `tools/frontend/topology-canvas-check.cjs`
- Modify: `crates/web-gateway/examples/browser_fixture.rs` (a fixture control to make the sample stale, and one to mark it at the row limit, if the fixture has none)

**Interfaces:**
- Consumes: `data-metric` and the throughput signal (Task 5).

- [ ] **Step 1: Write the failing checks:**
  - the tabs show text "Sankey", "Assignments", "Policies", "Changes" (not icon-only);
  - chips "Saved rev N" and "Applied rev N" from the summary;
  - live sample → chip "Live · Ns ago"; `complete: false` → chip "Partial" and the reason in its tooltip; stale sample → chip "Subscribers, not live" and `data-metric="subscribers"`;
  - the legend has one swatch and one name for each WAN, and "WAN unknown" only when a `None` stream exists;
  - the legend does not overlap any `.sankey-card` (bounding boxes);
  - at 390 px width: no horizontal scroll on the page.
- [ ] **Step 2: Run** the check. Expected: FAIL.
- [ ] **Step 3: Implement** the toolbar in one row (wraps on narrow screens). Rename the visible label of the `builder` tab to "Sankey". Keep the route key `builder`. Move the current preview controls into the row. Add no "Save" button (spec 7.5).
- [ ] **Step 4: Run** the check and `cargo test -p web-ui`. Expected: PASS.
- [ ] **Step 5: Commit** `feat(web): label the topology toolbar and states`

### Task 7: App navigation rail

**Files:**
- Modify: `crates/web-ui/src/app.rs:164-173` (`aside.sidebar`, `Navigation`), `crates/web-ui/style.css`
- Create: `tools/frontend/shell-rail-check.cjs`

**Interfaces:**
- Produces: `aside.sidebar[data-collapsed="true|false"]`, a toggle button `aria-label="Expand navigation"` / `"Collapse navigation"` with `aria-expanded`, saved in `localStorage["lqosync.nav.collapsed"]`. Default: collapsed on `/topology`, expanded on other pages, until the operator chooses.

- [ ] **Step 1: Write** `shell-rail-check.cjs`: the toggle changes `data-collapsed`; in rail mode each nav link keeps its accessible name and shows a tooltip on hover and focus; the choice survives a reload; keyboard Tab order is unchanged; at 390 px the page has no horizontal scroll.
- [ ] **Step 2: Run** `STAGES=browser CHECKS=shell-rail-check tools/frontend/check-all.sh`. Expected: FAIL.
- [ ] **Step 3: Implement** the rail. Icons only in rail mode, labels with `.sr-only`.
- [ ] **Step 4: Run** the full gate `tools/frontend/check-all.sh`. Expected: every gate PASS except the known `pulse-check` exclusion.
- [ ] **Step 5: Commit** `feat(web): collapse the navigation to a rail`

### Task 8: Evidence and handoff

**Files:**
- Create: `docs/frontend/evidence/topology-sankey/README.md`, screenshots at 1440, 768, and 390 px, dark and light, `SHA256SUMS`
- Modify: `brain/HANDOFF.md`

- [ ] **Step 1: Take the screenshots** from the fixture with the live sample, the stale sample, and an open popover.
- [ ] **Step 2: Compare** the 1440 px dark screenshot with `docs/frontend/prototypes/topology-studio-v2/04-owner-sankey-reference.png`. List each visible difference in the README (columns, ribbon fill, card accent bar, colors).
- [ ] **Step 3: Write** the README: commands, gate results, guides used (frontend-design, Modern Web Guidance), known limits (512-host sample, routing writer off).
- [ ] **Step 4: Add** the HANDOFF entry: branch, commits, gate results, not merged, not deployed.
- [ ] **Step 5: Commit** `docs(topology): record the Sankey canvas evidence`
