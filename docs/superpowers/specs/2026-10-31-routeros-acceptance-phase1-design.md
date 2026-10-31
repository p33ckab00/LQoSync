# RouterOS acceptance, Phase 1: one test host on the production core

Status: proposed, 2026-10-31.

## Intent

Mock tests cannot prove RouterOS address-list behavior.
This phase proves that behavior on the production core router with one test host.
The test host receives no traffic. Subscribers see no change.

The operator has no lab and no VM. The operator approved Approach A.
The operator requires no destructive change on the router.

## Questions this phase answers

1. Does `add` return the actual new row `.id`?
2. Does the scoped move keep the same `.id`, address, flags, and timeout? Do only `list` and `comment` change?
3. Does `remove` by `.id` delete only that row?
4. After the test, are all other address-list rows the same as before?
5. Are mangle, route, and routing table records the same as before?
6. Does RouterOS return the same row fields on two reads with no change between them?

## Scope

- Router: `RB5k9Core` only.
- Lists: `WAN1-CUSTOMERS` and `WAN2-CUSTOMERS`, from the existing `core_lists.bindings`.
- Test host: `192.0.2.254/32`. RFC 5737 reserves this address for documentation. No subscriber uses it.
- Comment marker: `lqosync3:<owner>:acce5500-0000-4000-8000-000000000001:wan1`, then `...:wan2`. `core_plan::marker` makes the marker.
  Owned markers require a canonical UUID subscriber identity. This fixed UUID identifies the test row.

## Non-goals

- No change to `ShapedDevices.csv` or `network.json`. `lqosync-updatecsv` continues to run.
- No change to subscriber rows, protected ranges, mangle, routes, tables, NAT, or filter rules.
- No Netwatch, WAN health, session, or executor journal checks. Phase 2 and Approach B cover these.
- No console stop, no service restart, and no settings change.
- No automatic mode, scheduler caller, or HTTP action.

## Approaches

- **A (selected):** a manual command uses the existing scoped `RouterTransport`.
  It sends three operations for one test host. It compares full reads before and after each step.
  It needs no WAN health and no exclusive lease. It does not prove the executor journal path.
- **B (deferred):** a run through plan, batch, and executor with a synthetic subscriber.
  It needs Netwatch configuration, settings changes, and a stopped console. Do it after A.

## Design

Add one binary: `v3/src/bin/core-acceptance.rs`.

- Without `--execute`, the command reads the router and prints the plan. It sends no write.
- With `--execute`, the command does these steps in this sequence:
  1. Read the address-list two times. Stop if the reads are different (question 6).
  2. Stop if the test host overlaps any existing row in the two lists, or if any row has the test marker.
  3. Read mangle, routes, and routing tables as a baseline.
  4. Add the test host to `WAN1-CUSTOMERS`. Record the returned `.id`.
     Read again. Exactly one new row must exist, with the expected fields.
  5. Move the row to `WAN2-CUSTOMERS`. Read again.
     The `.id`, address, flags, and timeout must not change. Only `list` and `comment` can change.
  6. Remove the row by `.id`. Read again. The rows must equal the step 1 baseline.
  7. Read mangle, routes, and routing tables again. They must equal the step 3 baseline.
- The verifier for the transport is a fresh address-list read of `RB5k9Core`.
  The transport preflight then applies its existing scope, owner, flag, overlap, and freshness checks.
  A change by another writer between reads stops the command before the next send.
- The command stops at the first failed check. It does not retry a request.
- If the command stops after the add, it prints the test row `.id` and a removal command for the operator.
  Do not remove any other row.
- The command writes a private JSON report to `v3/data/state/acceptance/`. Keep the report outside Git.
- The comparison checks are pure functions. Unit tests in the binary cover them.

## Risk

- A failure can leave one test row with the address `192.0.2.254`. That row affects no traffic.
- The running v3 console preview can show the test row during the test. The preview cannot write.
- External administrators and scripts can change the router during the test. The command then stops.

## Left to the builder

- Report field names and the format of the printed output.
- The exact read paths for the baseline tables. Use the existing `routeros::Client::read`.
