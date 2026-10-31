# RouterOS acceptance, Phase 2: one live subscriber moves between WANs

Status: approved by the operator in chat, 2026-10-31.

## Intent

Phase 1 proved address-list add, same-ID move, and remove with a test host.
Phase 2 moves one live subscriber between WANs with owned rows.
It records which public address the subscriber connections use before, during, and after the move.

The operator selected `10.0.100.220`, PPPoE user `RowenaEstilloso`.
The operator removed the manual WAN rows for this address. The address now uses the `main` default route through `pppoe-out1` (wan1).
The v3 subscriber identity is `6c607ca4-1da1-51e6-a402-802aa35bd43b`.

## Steps

`core-acceptance --address 10.0.100.220 --subscriber <uuid>` runs a dry run. Add `--execute` to send writes.

1. Find exactly one active PPP session with the address on the configured routers. Record its `.id` and name.
2. Add an owned row to `WAN1-CUSTOMERS` with marker `lqosync3:<owner>:<uuid>:wan1`. Traffic does not change, because wan1 is also the `main` default.
3. Move the row to `WAN2-CUSTOMERS`. The subscriber now uses wan2. Open connections can stop, because NAT changes the public address.
4. Hold 60 seconds. Sample the subscriber connections every 15 seconds.
5. Move the row back to `WAN1-CUSTOMERS`.
6. Remove the row. The address-list must equal the baseline.

Before the add and before the move to wan2, the PPP session must have the same `.id` and name.
A reconnect stops the command at those steps. The move back and the remove do not check the session,
because a revert of the owned row is always safe.
After each step, the command records connection counts per reply destination address (the NAT public address).
A failed connection sample is recorded in the report. It does not stop the test.

## Limits

- One address only. No other row, mangle, route, NAT, or filter record changes.
- No change to `ShapedDevices.csv`, `network.json`, settings, or services.
- The final table check ignores dynamic rows. OSPF routes for PPP clients can change during the test.
- The library gets one new read-only path: `/ip/firewall/connection/print`.
- The command stops at the first failed check and does not retry. After a confirmed add, it prints the row `.id` and the removal command.

## Risk

- The subscriber loses open connections two times, about 60 seconds apart. The operator accepted this.
- If the session reconnects with a different address, the command stops. The operator must then remove the row it prints.

## Approaches

- **Selected:** extend the Phase 1 command with an address and subscriber mode.
- **Rejected:** a manual move in Winbox. It does not use the v3 writer or owned markers.
- **Deferred:** the executor journal path (Approach B).
