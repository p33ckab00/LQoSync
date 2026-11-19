---
type: spec
created: 2026-11-20
status: draft
related: ["[[topology-manager-status]]", "[[2026-11-19-topology-live-acceptance]]"]
---

# Take-effect check design (Phase 3)

## Intent

LQoSync writes `network.json` and `ShapedDevices.csv` from the MikroTik data. LibreQoS then shapes from these files.
The owner wants proof that each change took effect in the LibreQoS dataplane: a speed change, a name change, a new subscriber, or a removed subscriber.
The proof must use the LQoSync identities (Circuit ID, Device ID, IP, node ID), not the generation identities that LibreQoS makes for its own tracking.
A change that LibreQoS has not applied yet is "waiting to take effect". It is not an error until it waits too long.

The current check (`apply_evidence`) cannot give this proof:

- It first requires that the bridge generation is equal to the LibreQoS `shaping_generation`.
- Then it requires that the whole applied world is equal to the whole generated world (`transaction::matches`).
- The result is all or nothing. It does not tell which circuits wait.

The 2026-11-20 live test showed the problem. See the [dataplane hold report](../../operations/LIBREQOS_V3_DATAPLANE_HOLD_REPORT.md).

## Goals

- Make the check per identity. For each Circuit ID, Device ID with IP, and node ID of the LQoSync pair, tell if the bridge applied it.
- Show the waiting identities and the different fields.
- Use the check in the live test (`acknowledgement` step) and in the service health (`apply_evidence::Monitor`).
- Keep all existing integrity checks that do not depend on the bridge generation.

## Non-goals

- No change to LibreQoS files, settings, or services. The check is read-only.
- No new notification. The existing `VerifyTimeout` notification (after `IMPORT_LIMIT_SECS`, 40 min) stays.
- No calculation of the LibreQoS rate translation in LQoSync.
- No web frontend work.

## Anti-goals

- A check that reports "applied" while the bridge shapes an old rate for a circuit.
- A check that depends on the format of LibreQoS generation IDs.

## Facts from production (2026-11-20)

- LibreQoS keeps the LQoSync identities without change from the CSV to the bridge. A leaf op has `circuit_id` (for example `1EVR260Q`) and `parent_node_id` (the `id` of the `network.json` node, for example `lq3-ad596f3fef4a18ec`). A mapping op has `circuit_id`, `device_id`, `ip`, and `prefix_len`. An inner op has `node_id` and `parent_node_id` (`root` at the top).
- Names are not in the dataplane. A name change is visible only in `topology_import.json`.
- LibreQoS translates the rates. A leaf is direct (50 Mbps gives `ceil_kbps` 50000, minimum 5 Mbps gives `commit_kbps` 5000). A node is not direct: 225.75 Mbps gives `ceil_kbps` 226000, and `commit_kbps` comes from the LibreQoS planner.
- The bridge `state` report (`/dev/shm/libreqos-analytics.sock`) gives the applied world in the same op format as `dataplane_transaction.json`.

## Approaches

| Approach | Summary | Trade-off |
|---|---|---|
| A. Compare the bridge with values that LQoSync calculates | LQoSync makes the expected kbps from its own pair | Pure LQoSync values, but LQoSync must copy the LibreQoS planner rules for node rates and rounding. A beta change of these rules gives a false "waiting". |
| B. Compare the bridge with the LibreQoS translation, keyed by LQoSync identities (chosen) | Identities and links come from the LQoSync pair. Rates come from `dataplane_transaction.json`, compared op by op with the bridge | No copy of LibreQoS rules. The identity check makes sure that the translation is of the current LQoSync pair. |
| C. Keep the current check | Generation gate and whole-world equality | All or nothing. No per-circuit result. Depends on the bridge generation identity. |

Designer call: approach B. The owner gave the choice to the designer on 2026-11-20.

## Design

### The check in four levels

1. **Import (no change).** The LibreQoS import is equal to the LQoSync pair, also names, IPs, rates, and parents (`ExpectedImport::matches`).
2. **Published files (no change).** The LibreQoS files on disk agree with each other: `topology_runtime_status.json` is `ready`, and its `shaping_generation` is equal to the generation in `shaping_inputs.json` and in `dataplane_transaction.json`. This check reads LibreQoS files only. It does not compare LibreQoS identities with the bridge.
3. **Identity coverage (new).** `dataplane_transaction.json` has the identities of the LQoSync pair, and no other identities:
   - the set of leaf `circuit_id` values is the set of Circuit IDs in the CSV,
   - each leaf `parent_node_id` is the `id` of the CSV parent node in `network.json`,
   - the set of mapping ops (`circuit_id`, `device_id`, `ip`, `prefix_len`) is the set of device addresses in the CSV,
   - the set of inner `node_id` values with their `parent_node_id` is the node tree of `network.json`.
   A failure of level 3 is an error, not a wait: LibreQoS published a world that is not the LQoSync pair.
4. **Take effect (new, replaces the generation gate and `transaction::matches`).** For each identity, compare the bridge op with the op of `dataplane_transaction.json`:
   - leaf by `circuit_id`: all fields,
   - mapping by (`ip`, `prefix_len`): all fields,
   - inner by `node_id`: all fields,
   - a bridge op with an identity that is not in the generated world is a waiting removal.

### The result

```text
TakeEffect {
  applied: count of identities that are equal,
  waiting: list of (kind, identity, fields that differ or "missing"),
  waiting_removal: list of (kind, identity),
}
```

The pair took effect when `waiting` and `waiting_removal` are empty.
The two bridge reads must stay equal (existing `before == after` check). The input files must not change during the check (existing check).

### Where the check runs

- **Live test, `acknowledgement` step:** the step passes when the pair took effect. It tries again until the import timeout. On a timeout, the step error and the report give the waiting identities (at most 20 in the error text, all in the report).
- **Service health, `apply_evidence::Monitor`:** after `ImportConfirmed`, the receipt goes to `Applied` when the pair took effect. While identities wait, the health reason shows the count of waiting identities and the time since the write. After `IMPORT_LIMIT_SECS`, the receipt goes to `VerifyTimeout` as today.
- **Late apply (designer call):** LibreQoS can apply a held change hours later, for example in its maintenance window. The monitor continues to check a `VerifyTimeout` receipt of the current write. When the pair took effect, the receipt goes to `Applied` and the health becomes healthy. No new notification.

### Kept checks

The compiler binding check, the receipt and file hash checks, the import check, the stable bridge reads, and the producer path check stay. The `circuit_capacity` evidence stays in `Acceptance`.

## Constraints

- Read-only access to the LibreQoS files and the analytics socket.
- The bridge report format is a LibreQoS beta interface. Unknown fields in an op make the compare fail closed (the op is "waiting" with the field name), not pass.
- Rust 1.89 compatible code. No new dependency.

## Left to the builder

- The module layout (a new `apply_evidence/take_effect.rs` is a natural place).
- The exact health reason text.
- How `VerifyTimeout` to `Applied` fits the `write_receipt` transition rules. If the rules forbid it, add the transition with a test.
- Tests: a fixture world where one leaf rate differs, one mapping is missing, one extra leaf stays (removal), one node rate differs, and an identity coverage failure.
