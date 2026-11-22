# WAN allocation modes and failover: implementation plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** The operator selects the allocation mode (count, capacity, or quality). The service monitors each WAN and moves subscribers away from an unavailable WAN, then back after a stable recovery.

**Architecture:** The catalog stores a topology default mode and an automatic-failback switch; each subscriber source can override the mode. A WAN monitor task probes each WAN node through its router every 10 s and publishes a `WanSnapshot`. The resolver reads the snapshot: unavailable WAN branches leave every eligible set, quality mode scales WAN capacity by the quality score, and a failover placement keeps the saved assignment as its home.

**Tech Stack:** Rust 2024, `serde`, `tokio` (`watch`, `interval`), `mikrotik-rs` (existing), Leptos 0.8.23, Playwright checks.

**Spec:** `docs/superpowers/specs/2026-11-22-topology-auto-placement-design.md`, section "Owner decisions, round 2 (2026-11-22)".

## Global Constraints

- Do not deploy. Do not change the live registry, routers, services, or writer ownership.
- No MikroTik configuration write. A probe uses `/ping` and `/interface/print` only. The address-list writer is a separate plan with its own owner approval.
- Do not add a dependency. Keep `leptos = "=0.8.23"`.
- Registry schema becomes `7`. A schema-6 file migrates with no change of behavior.
- Mode names on the wire: `count`, `capacity`, `quality`. The default mode is `count`.
- Monitor values (spec): probe every 10 s, two targets, loss 50% or more, or latency more than 3 times the normal value, for 1 minute; a disabled or not running interface is unavailable at once; subscribers move only after the problem continues for more than 2 minutes; failback after 10 minutes stable.
- Quality mode: WAN share is proportional to capacity × quality score (0 to 1). Stale quality data: use capacity mode.
- Saved assignments stay (S1–S4 = A). A subscriber moves only for a tree change, a manual Rebalance, or a failover. Quality changes affect new subscribers only.
- Write docs, comments, and commit bodies in ASD-STE100. Commit summaries are imperative and 50 characters or less.
- Stage explicit paths only. Other agents edit this worktree.

## Review Focus

1. A WAN that fails for 90 s and then recovers moves nobody. Test: `a_short_failure_moves_nobody` (Task 3).
2. Every eligible WAN is unavailable at the same time. Subscribers stay at their saved assignment; nothing becomes flat or a conflict. Test: `when_every_wan_is_unavailable_nobody_moves` (Task 2).
3. The ping permission is missing or the core router does not answer. The WAN is `unknown` (stale), not unavailable; nobody moves and quality mode uses capacity. Test: `a_probe_error_is_unknown_not_unavailable` (Task 3).
4. Failover with automatic failback off: the WAN recovers and the subscribers stay on the substitute until a Rebalance edit. Test: `manual_failback_waits_for_a_rebalance` (Task 4).
5. A schema-6 registry with sources that never chose a mode keeps capacity mode for those sources; only new sources and automatic sets use the count default. Test: `schema_6_sources_keep_capacity_after_migration` (Task 1).

## File map

| File | Change |
|------|--------|
| `src/topology_manager/catalog.rs` | `AllocationMode::Quality`, `TopologyCatalog.allocation`, `TopologyCatalog.auto_failback`, `GroupDefaults.allocation: Option<_>` |
| `src/topology_manager/model.rs`, `store.rs` | `SCHEMA = 7`, migration arm `6 =>` |
| `src/topology_manager/edits.rs`, `crates/web-contract/src/topology.rs` | Edits `allocation.default`, `failback.auto`; `group.allocation` takes `null` |
| `src/topology_manager/wan_health.rs` (new) | Pure state machine, quality score, `WanSnapshot` |
| `src/topology_manager/observation.rs` | `Reconciliation.wan: WanSnapshot` |
| `src/topology_manager/resolver.rs`, `aggregate.rs` | Effective mode, quality scaling, failover, `Placement.home` |
| `src/wan_monitor.rs` (new), `src/routeros.rs`, `src/mikrotik.rs`, `src/main.rs`, `src/config.rs` | Probe trait, monitor task, `[wan_monitor]` config |
| `src/runner.rs`, `src/topology_manager/coordinator.rs` | Pass the snapshot; Rebalance releases waiting WANs |
| `src/topology_manager/web_view.rs`, `crates/web-ui/src/topology_canvas.rs`, `style.css` | Settings, WAN health chip |
| `tools/frontend/topology-canvas-check.cjs`, `crates/web-gateway/examples/browser_fixture.rs` | Browser cases |

Dependency sequence: Task 1, then Task 2 (it creates the `wan_health.rs` types), then Task 3, Task 4, Task 5, Task 6.

---

### Task 1: Mode and failback settings in the model

**Files:** `catalog.rs`, `model.rs`, `store.rs`, `edits.rs`, `crates/web-contract/src/topology.rs`. Tests: `catalog_tests.rs`, `store_recovery_tests.rs`, `edits_tests.rs`.

**Interfaces:**
- Produces: `AllocationMode { Count, Capacity, Quality }` (serde `snake_case`; `Default` becomes `Count`).
- Produces: `TopologyCatalog.allocation: AllocationMode` (`#[serde(default)]`), `TopologyCatalog.auto_failback: bool` (`#[serde(default = "yes")]`, default `true`).
- Produces: `GroupDefaults.allocation: Option<AllocationMode>` (`None` = topology default).
- Produces: `TopologyCatalog::mode_of(&self, source: Option<&CatalogSource>) -> AllocationMode`.
- Produces: `BuilderEdit::AllocationDefault { mode: String }` (`allocation.default`), `BuilderEdit::AutoFailback { enabled: bool }` (`failback.auto`); `BuilderEdit::GroupAllocation.mode` becomes `Option<String>`.

- [ ] **Step 1: Write the tests that fail**

```rust
#[test] fn mode_of_uses_the_source_override_then_the_topology_default() // None source -> Count; Some(Capacity) override wins
#[test] fn schema_6_sources_keep_capacity_after_migration()            // schema-6 file, group without "allocation" -> Some(Capacity); catalog.allocation == Count; schema 7
#[test] fn allocation_default_and_auto_failback_edits_save()           // {"kind":"allocation.default","mode":"quality"}; {"kind":"failback.auto","enabled":false}
#[test] fn group_allocation_null_returns_to_the_topology_default()    // {"kind":"group.allocation","source_id":"profile-source","mode":null} -> None
#[test] fn an_unknown_mode_is_refused()                                // mode "fastest" -> error contains "allocation mode"
```

- [ ] **Step 2: Run them.** `cargo test --locked -p lqosync3 --lib -- mode_of schema_6 allocation_default group_allocation_null unknown_mode` — Expected: compile errors.
- [ ] **Step 3: Implement.** Migration arm `6 =>` sets `Some(Capacity)` on each group whose value is absent, then `schema_version = 7`. Change the current-arm text to "schema-7". Update the 3 existing count tests for the `Option` field.
- [ ] **Step 4: Run** `cargo test --locked -p lqosync3 --lib topology_manager:: && cargo test --locked -p web-contract` — Expected: 0 failed.
- [ ] **Step 5: Commit** `feat(topology): add allocation modes and failback`.

### Task 2: Resolver modes, quality, and failover

**Files:** `resolver.rs`, `aggregate.rs`, `observation.rs`, new `wan_health.rs` (types only here). Tests: `resolver_tests.rs`, `aggregate_tests.rs`.

**Interfaces:**
- Consumes: `mode_of`, `auto_failback` (Task 1).
- Produces in `wan_health.rs`: `enum WanState { Up, Unknown, Degraded, Unavailable, Recovering, WaitingForRebalance }`; `struct WanStatus { state: WanState, quality: Option<Fraction> }`; `struct WanSnapshot(pub HashMap<String, WanStatus>)` keyed by WAN node ID, `Default`. `WanStatus::excluded(&self) -> bool` is true for `Unavailable`, `Recovering`, `WaitingForRebalance`.
- Produces: `Reconciliation.wan: WanSnapshot` (`reconcile` sets `Default`).
- Produces: `Placement.home: Option<String>` — the saved assignment while a failover keeps the subscriber elsewhere.

- [ ] **Step 1: Write the tests that fail**

```rust
#[test] fn count_is_the_default_for_automatic_sets()          // 9 unbound on wan1(310M)/wan2(620M) roots -> (4|5 split, difference <= 1)
#[test] fn capacity_mode_follows_capacity()                   // catalog.allocation = Capacity -> 3 : 6
#[test] fn quality_mode_weights_capacity_by_quality()         // wan1 q=1/1, wan2 q=1/4: 310 : 155 -> 6 : 3 for 9
#[test] fn stale_quality_uses_capacity()                      // Quality mode, no quality values -> same split as capacity
#[test] fn an_unavailable_wan_branch_leaves_every_set()       // wan1 Unavailable -> all on wan2 branch; placed sources too
#[test] fn a_failover_keeps_the_home_assignment()             // saved group1 under wan1, wan1 Unavailable -> target group2, home Some("group1"), candidate assignment "group1"
#[test] fn the_home_returns_when_the_wan_is_up()              // same registry, wan1 Up -> target group1, home None
#[test] fn when_every_wan_is_unavailable_nobody_moves()      // both Unavailable -> targets == saved assignments, no conflict
#[test] fn quality_changes_do_not_move_saved_assignments()    // saved on wan2, quality of wan2 drops -> stays
```

- [ ] **Step 2: Run** `cargo test --locked -p lqosync3 --lib topology_manager::resolver` — Expected: FAIL.
- [ ] **Step 3: Implement.**
  - Effective mode of one subscriber: `mode_of` of its first placed matched source; automatic and fallback sets use the topology default.
  - Count: the existing fewest-subscribers choice, also for automatic sets.
  - Quality: in the score, replace the capacity of each WAN node on the path with `capacity × quality`; a WAN without quality, or `Unknown`, uses capacity.
  - Failover: remove targets below an excluded WAN from `eligible`. If that empties a set that was not empty, keep the set unchanged (Review Focus 2). If the saved assignment is excluded, place by the mode and set `home = Some(saved)`; the candidate keeps `saved`.
  - `aggregate.rs`: accept `assignment == placement.home.or(placement.target_id)`.
- [ ] **Step 4: Run** `cargo test --locked -p lqosync3 --lib topology_manager::` — Expected: 0 failed.
- [ ] **Step 5: Commit** `feat(topology): add mode, quality, and failover rules`.

### Task 3: WAN health state machine and config

**Files:** `wan_health.rs`, `config.rs` (`[wan_monitor]`, defaults in `DEFAULTS_TOML`). Test: `wan_health.rs` tests, `config.rs` tests.

**Interfaces:**
- Produces: `struct MonitorSettings { enabled: bool, interval_secs: u64 = 10, targets: Vec<String> = ["1.1.1.1","8.8.8.8"], loss_percent: u64 = 50, latency_factor: u64 = 3, window_secs: u64 = 60, move_after_secs: u64 = 120, failback_secs: u64 = 600 }` in `Config.wan_monitor` (`enabled` default `true`).
- Produces: `enum Sample { Reply { rtt_us: u64 }, Lost, InterfaceDown, ProbeError }`.
- Produces: `struct WanHealth` with `observe(&mut self, now: u64, sample: Sample)`, `status(&self, now: u64, auto_failback: bool) -> WanStatus`, `release(&mut self)` (WaitingForRebalance → Up).
- Normal latency: the median RTT of `Reply` samples of the last 10 minutes while the WAN was `Up`.
- Quality score: `(1 − loss) × min(1, normal / current median RTT)` over the window. No sample for 3 intervals: `quality = None`, state `Unknown`.

- [ ] **Step 1: Write the tests that fail**

```rust
#[test] fn an_interface_down_is_degraded_at_once_and_unavailable_after_two_minutes()
#[test] fn half_lost_probes_for_one_minute_degrade_the_wan()
#[test] fn latency_above_three_times_normal_degrades_the_wan()
#[test] fn a_short_failure_moves_nobody()                         // 90 s bad then good -> never Unavailable
#[test] fn recovery_needs_ten_stable_minutes()                    // Unavailable -> Recovering -> Up at +600 s (auto on)
#[test] fn manual_failback_waits_in_waiting_for_rebalance()       // auto off -> WaitingForRebalance until release()
#[test] fn a_probe_error_is_unknown_not_unavailable()             // ProbeError for 5 min -> Unknown, quality None
#[test] fn quality_score_follows_loss_and_latency()               // 10% loss, rtt 2x normal -> 0.9 x 0.5 = 0.45
#[test] fn wan_monitor_config_defaults_match_the_spec()
```

- [ ] **Step 2: Run** `cargo test --locked -p lqosync3 --lib -- wan_health wan_monitor_config` — Expected: FAIL.
- [ ] **Step 3: Implement** the state machine with plain `u64` seconds (no clock reads) and a bounded sample ring (`window_secs / interval_secs` and 10 minutes of normal RTT).
- [ ] **Step 4: Run** the same command — Expected: PASS.
- [ ] **Step 5: Commit** `feat(wan): add the WAN health state machine`.

### Task 4: Probe, monitor task, and service wiring

**Files:** `src/routeros.rs` (trait), `src/mikrotik.rs`, new `src/wan_monitor.rs`, `src/main.rs`, `src/runner.rs`, `coordinator.rs`. Tests: `tests/mock_router.rs`, `src/wan_monitor.rs` tests, runner tests.

**Interfaces:**
- Produces in `routeros.rs`: `trait RouterProbe: Send { fn ping(&mut self, address: &str, interface: &str) -> impl Future<Output = Result<Option<u64>, ReadError>> + Send; }` — `Some(rtt_us)` or `None` for a lost reply. One `/ping` with `count=1`.
- Produces: `wan_monitor::Monitor::start<C: Connector>(cfg: &Config, connector: Arc<C>, hub: Hub) -> Monitor where C::Session: RouterProbe`; `Monitor::reconfigure(&self, cfg)`; `Monitor::set_wans(&self, wans: Vec<WanProbe>)`; `Monitor::snapshot(&self, auto_failback: bool) -> WanSnapshot`; `Monitor::release_waiting(&self)`.
- Produces: `struct WanProbe { node_id: String, router: String, interface: String }` from each WAN node (`SourceOrigin::Wan { router_id, endpoint_ref }`, router by catalog name).
- Produces: `Coordinator::set_wan(&mut self, wan: WanSnapshot)`; `prepare` copies it into `Reconciliation.wan`.
- The monitor writes `state_dir/wan-health.json` (state, loss, median RTT, quality per WAN node) for the web view.

- [ ] **Step 1: Write the tests that fail**

```rust
// tests/mock_router.rs
#[tokio::test] async fn ping_reads_the_round_trip_time_and_a_lost_reply()  // "time=12ms345us" -> Some(12345); no reply -> None
#[tokio::test] async fn ping_permission_error_is_a_read_error()            // trap "not enough permissions" -> Err
// src/wan_monitor.rs
#[tokio::test(start_paused = true)] async fn each_wan_is_probed_through_its_own_interface() // fake probe records (router, interface, target)
#[tokio::test(start_paused = true)] async fn a_router_that_does_not_answer_gives_unknown()
// runner tests
#[tokio::test] async fn manual_failback_waits_for_a_rebalance()           // auto_failback false; WaitingForRebalance; Rebalance edit -> release_waiting -> home again
#[tokio::test] async fn the_cycle_passes_the_wan_snapshot_to_placement()  // Unavailable wan1 -> pair rows under wan2
```

- [ ] **Step 2: Run** `cargo test --locked -p lqosync3 --test mock_router && cargo test --locked -p lqosync3 --lib -- wan_monitor manual_failback the_cycle_passes` — Expected: FAIL.
- [ ] **Step 3: Implement.** The monitor task uses `tokio::time::interval` with `MissedTickBehavior::Skip`, one session for each router, and reads `/interface/print` (`name,disabled,running`) once for each interval. `main.rs` starts it next to `worker_monitor` and calls `reconfigure` on SIGHUP. Each cycle, the runner calls `set_wans` from the registry and `set_wan(snapshot)` before `prepare_topology_for`. A saved Rebalance edit calls `release_waiting`. `lqosync3 check` warns when the ping of a WAN router is refused for permission.
- [ ] **Step 4: Run** the same commands — Expected: PASS.
- [ ] **Step 5: Commit** `feat(wan): monitor each WAN and feed placement`.

### Task 5: Web view and canvas controls

**Files:** `web_view.rs`, `crates/web-contract/src/topology.rs`, `topology_canvas.rs`, `style.css`, `browser_fixture.rs`, `topology-canvas-check.cjs`.

**Interfaces:**
- Produces: `Node.wan_health: Option<WanHealthView { state: String, loss_percent: Option<u64>, rtt_ms: Option<f64>, quality_percent: Option<u64> }>` (`#[serde(default)]`), from `wan-health.json`.
- Produces: `TopologySummary.allocation: String` and `TopologySummary.auto_failback: bool` (`#[serde(default)]`).
- Consumes: `allocation.default`, `failback.auto`, `group.allocation` with `null` (Task 1).

- [ ] **Step 1: Add the browser cases that fail**

- `allocation modal saves the topology default` (one `allocation.default` edit with `quality`).
- `a group can follow the topology default` (one `group.allocation` edit with `mode: null`).
- `automatic failback switch saves one edit` (`failback.auto`, `enabled:false`).
- `WAN card shows its health as text` (`Unavailable` on WAN South in the fixture; `Quality 87%` on WAN North).
- Light and dark themes, 375 px and 768 px, no sideways scroll, focus returns after Escape.

- [ ] **Step 2: Run** `STAGES=browser CHECKS=topology-canvas-check tools/frontend/check-all.sh` — Expected: FAIL.
- [ ] **Step 3: Implement.** Allocation modal: a "Topology default" radio group (Equal count, Capacity, Quality) and, for a selected group, a "This group" select with "Use topology default". Remove the "Quality-aware (not available)" row. Settings modal: an "Automatic failback" checkbox with the help text "Off: subscribers stay on the other WAN until you run Rebalance." WAN card: a text chip with the state and the quality. Reserve the chip space.
- [ ] **Step 4: Run** `tools/frontend/check-all.sh` — Expected: all PASS (`pulse-check` can SKIP).
- [ ] **Step 5: Commit** `feat(canvas): add mode settings and WAN health`.

### Task 6: Acceptance and docs

**Files:** `docs/frontend/TOPOLOGY_CANVAS_DESIGN.md` ("Subscriber allocation" state), `docs/frontend/DEPLOYMENT.md` (ping permission of the read-only API user, `[wan_monitor]`), `brain/HANDOFF.md`, `src/runner/tests/canvas_shadow.rs`.

- [ ] **Step 1: Add a shadow case** `a_wan_failover_moves_only_the_affected_subscribers` (enrolled scenario plus two WAN nodes; one Unavailable; rows under the other WAN keep their identities; no added or missing row).
- [ ] **Step 2: Run** `cargo fmt --all -- --check && cargo clippy --locked --workspace --all-targets -- -D warnings && cargo test --locked --workspace` — Expected: 0 failed.
- [ ] **Step 3: Run** `tools/frontend/check-all.sh` — Expected: all PASS.
- [ ] **Step 4: Update the docs.** STE check: `python3 ~/.claude/skills/asd-ste100/scripts/ste_check.py <changed docs>` — no new findings except list-length false positives.
- [ ] **Step 5: Commit** `docs(wan): record allocation and failover state`.

## Not in this plan

- Reading the manual address-lists on the core router, and the address-list writer (spec "WAN choice and routing"). Separate plans; the writer needs owner approval first.
- MikroTik route changes. A failover changes the LibreQoS placement only, until the writer exists.
