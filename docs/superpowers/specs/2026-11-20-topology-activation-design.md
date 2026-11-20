---
type: spec
created: 2026-11-20
status: draft
related: ["[[topology-manager-status]]", "[[2026-11-19-topology-live-acceptance]]"]
---

# Topology Manager activation design (Phase 4)

## Intent

After activation, the Topology Manager is the only writer of `network.json` and `ShapedDevices.csv`.
The manager registry holds the topology: the node hierarchy (roots, groups, sites, parents) and the subscriber identities. The writer follows the registry, not the legacy `[network] mode`.
The manager gives each subscriber account one stable Circuit ID and Device ID. The legacy writer gives a new random ID when a client comes back, and LibreQoS then loses the history of that subscriber.

Phase 3 passed on 2026-11-20 (report `/var/backups/lqosync3/live-test/report-2026-11-20-5.json`). The production service runs `3.0.320+g1e07c93` with the legacy writer and an enrolled registry (revision 4, a `root_promotion` hierarchy).

## Owner decisions (2026-11-20)

1. One writer only. After activation, the manager is the writer, and its registry decides the topology and the node hierarchy.
2. `[network] mode` is only the start point. The first enrollment uses it to build the first hierarchy. For a new installation, the default is `root_promotion`.
3. When the manager fails in a cycle, the service keeps the last good pair, reports the failure, and tries again in the next cycle. It goes back to normal by itself when the manager works. There is no fallback to the legacy writer. This replaces an earlier decision of the same day.
4. `topology deactivate` stays as a manual emergency rollback to the legacy writer.

## Goals

- A one-time `lqosync3 topology activate` command with gates.
- A service cycle where the manager is the only writer after activation.
- Hold of the last good pair on a manager failure, with a clear status, health, and notification, and an automatic retry.
- The writer status in `status.json`, the TUI, the API (`writer`), and the health report.
- A manual `lqosync3 topology deactivate` that keeps the IDs of known subscribers.
- The default `[network] mode` for a new installation is `root_promotion`.

## Non-goals

- No change of the hierarchy at activation. The registry already reproduces the production `root_promotion` tree.
- No automatic first activation.
- No web frontend work.
- No change to the router read path. Routers stay read-only.

## Anti-goals

- Two writers, or two sources of the topology, after activation.
- A failure that changes the shaped pair (a partial or empty write).
- A silent failure.
- A deactivation that changes the IDs of known subscribers.

## Approaches

| Approach | Summary | Trade-off |
|---|---|---|
| A. Manager first, legacy writer on failure, in each cycle | Considered first on 2026-11-20 | Two writers and two topology sources. Against decision 1. Not used |
| B. One writer, hold of the last good pair on failure (chosen) | After activation, the cycle runs the manager write only. A failed manager cycle writes nothing, so the last good pair stays | One topology source. No ID or rate change on a failure. The cost: while the manager fails, a new subscriber is not shaped until the manager works again |
| C. Automatic `deactivate` on failure, automatic `activate` on success | Write and remove `runtime-owner.json` on each change | Two writers in time, and the activation gate (parity) fails after manager edits. Not used |

## Facts from the code (2026-11-20)

- `runtime-owner.json` (`src/topology_manager/runtime.rs`) holds `{"schema_version": 1, "writer": "topology"}`. `topology_owns` reads it. The legacy cycle refuses to write when it exists (`src/runner.rs` `cycle`, "topology owns the writer; legacy fallback is blocked"). This rule stays.
- `Runner::sync_topology` collects, compiles with `ControlledCompiler`, runs admission, and publishes (`publish_topology`, `publish_compiled`). No service code calls it today.
- `publish_compiled` can claim the runtime owner and move `last_topology_generation` before the write. It can also return `Ok(WriteResult { error: Some(..) })` after the write ("pair written; scheduler reload request failed").
- `Runner::prepare_topology_for(max_age, replace)` and `topology enroll --replace` refresh the adoption.
- `apply_evidence::Monitor` checks each write receipt per identity (take-effect check), for each writer.
- The legacy writer keeps its IDs in `/var/lib/lqosync3/state.json` under `circuits`, keyed by the legacy key (PPP name, or `DHCP-<hostname>` from the lease), with the full row and `last_seen`.
- `config/defaults.toml` sets `[network] mode = "grouped_router_root"` today.

## Design

### 1. `lqosync3 topology activate`

Run it in a window with the service stopped. Without `--yes` it is a dry run (exit status 2). It refuses (exit status 1, nothing written) when one gate fails:

1. The registry is enrolled, and `runtime-owner.json` is absent.
2. The adoption is fresh: the command prepares the pair with `replace = true` and compares it with the live legacy pair. If a legacy row has no alias, the command stops and names it. The operator then runs `propose --replace` and `enroll --replace` first.
3. The manager pair is equal to the live legacy pair, except the expected node rounding (`node_export_adjustments`). `ChangedRow`, `ChangedNetwork`, `AddedRow`, and `MissingRow` must be empty.
4. LibreQoS is `ready`, and the controlled compiler opens and compiles the pair.
5. A live test passed at the current registry revision. Add `registry_revision` to the live-test report. `activate` requires `--report <path>` with `passed: true` and a `registry_revision` equal to the current one. Thus the activation window runs `topology live-test` again just before `activate` (designer call).

With `--yes`, the command writes `runtime-owner.json` atomically and prints `activated: the Topology Manager is the writer`.

### 2. The service cycle after activation

When `topology_owns` is `Ok(true)`, the cycle does these steps:

1. It runs the parts of the cycle that do not write the pair, as today (designer call: split only the write step). These parts are the native probe, the bridge statistics, the router status lines, the collection report, the Settings snapshot, and the report summary.
2. It runs the manager write (`sync_topology`). The legacy write step does not run.
3. If the manager write succeeds, `writer = "topology"`, `manager_failed = false`. The per-change Telegram notice continues: the manager write gets the same `Frozen` change notice as a legacy write.
4. If the manager write fails before it writes the pair, the pair on disk stays (the last good pair). `writer = "topology"`, `manager_failed = true`, `manager_error = <{e:#} text>`, `manager_failed_since = <first failed cycle>`.
5. The next cycle tries again. The first good cycle sets `manager_failed = false`. This is the automatic return.

Rules for a manager failure (designer call):

- The manager path must return `Err` without a partial write for a source, identity, or revision refusal, a compile failure, and an admission refusal.
- `publish_compiled` must not claim `runtime-owner.json`. Only `activate` writes it, and only `deactivate` removes it.
- A problem after the write (for example `WriteResult.error`, an import timeout, `VerifyTimeout`) is not a manager failure in this sense. The write receipt and the take-effect health report it.
- An unreachable router: the manager path keeps the last good pair, as the legacy guard does today. Designer call: report it as a collection problem, not as a manager failure.

When `topology_owns` is `Ok(false)`, the cycle runs the legacy path as today (`writer = "legacy"`).
When `topology_owns` returns `Err` (an invalid owner file), the cycle writes nothing and reports the error, as today. Designer call: an invalid owner file never selects the legacy writer.

### 3. `lqosync3 topology deactivate`

Run it in a window with the service stopped. Without `--yes` it is a dry run.
With `--yes`, it does these steps:

1. It reads the last pair on disk (the last good manager pair).
2. It writes each row into the legacy `state.json` `circuits`, under the legacy key that the legacy writer computes from the router data for that account. It updates the node entries the same way. This keeps the manager IDs, also `tm-sub-...` IDs, for the legacy writer (designer call: one-time seed at deactivation, not a copy in each cycle).
3. It removes `runtime-owner.json` atomically.

Known limit (designer call): a manager subscriber with more than one identity or device has no single legacy key. An example is a shared `[lqosync-id:..]` comment reference. The seed skips it, and the legacy writer gives it its own IDs. `deactivate` and `activate` print the count of such subscribers.

### 4. Status, health, and notifications

- `status.json`: new fields `writer` (`"topology"` or `"legacy"`), `manager_failed` (bool), `manager_error` (string or null), `manager_failed_since` (unix seconds or null).
- API `status`: `writer` as today, plus `manager_failed` and `manager_error`.
- TUI: one line on the status screen with the writer and, during a failure, the error and its age.
- Health: a `Domain::Apply` component `writer`. `Healthy` with reason `topology` or `legacy`. `Degraded` with reason `manager_failed` while the manager fails.
- Notifications: one notice when the manager starts to fail, and one when it works again. Use the existing Telegram outbox and its rate rules. No notice on each failed cycle.
- Restart (designer call): at start, the service reads `manager_failed`, `manager_failed_since`, and `writer` from the previous `status.json`. Thus a restart during a failure sends no second notice.

### 5. Default `[network] mode` for a new installation

Change the default in `config/defaults.toml` to `mode = "root_promotion"`. An existing installation with an explicit `mode` does not change. Update the config documentation and the example configuration to show `root_promotion` as the default.

### 6. Rollback to an older build

An older build (`3.0.182+ge8da230`) does not read the registry or `runtime-owner.json`. Before a rollback to such a build, run `topology deactivate`. Then install the old binary as in the deploy record (`/home/pi/.local/state/lqosync3/deploy-1e07c93/README.md`).

## Constraints

- Rust 1.89 compatible. No new dependency.
- Routers stay read-only.
- Each window command needs the instance lock (the service is stopped).
- Persisted text in ASD-STE100.

## Verification

- Unit tests: after activation, a manager success writes once and the legacy write step does not run. A manager failure writes nothing, keeps the pair, sets `manager_failed`, and the next good cycle clears it. One notice per change, none on a restart. The activation gates refuse each failure case. `deactivate` seeds `state.json` and removes the owner file, and the legacy writer then keeps a `tm-sub-...` ID. The default mode is `root_promotion`.
- A production window: run `topology live-test`, then `activate --report <path>`, then observe at least three cycles with `writer = "topology"` and no `manager_failed`. Do not stop LibreQoS services to cause a failure. Run `deactivate` only if the owner asks.

## Left to the builder

- Where the write step splits inside `Runner::cycle`.
- How the service opens and keeps the `ControlledCompiler`.
- The exact legacy key for a row (the same rule as the legacy writer).
