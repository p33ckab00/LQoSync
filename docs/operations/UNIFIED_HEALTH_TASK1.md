# Unified health and managed-write reports

Status: local implementation on `lqosync3`. Production remains unchanged.
Plan: [Unified health and Telegram](../superpowers/plans/2026-11-11-unified-health-telegram.md).

## Update lifecycle

The service treats `network.json` and `ShapedDevices.csv` as one logical operation.
Each changed operation has a private, durable write ID, SHA256 input hashes, and a result for each file.
An unchanged cycle creates no new write ID or Telegram write report.
A partial write retains the completed file's result. Notification failures do not roll back shaping.

For the LibreQoS v3 pair target in apply mode, the service produces these reports:

1. **WRITTEN / PENDING:** the file replacements completed. Native acceptance is pending.
2. **APPLIED:** the current subscriber policy matches the imported inputs and native acknowledgement.
3. **WRITE FAILED, VERIFY TIMEOUT, or SUPERSEDED:** the operation has the stated outcome.

Import confirmation alone cannot produce APPLIED.
The pending checker runs every two seconds, independently from subscriber collection.
It stops source reads when no operation is pending.
The existing forty-minute verification deadline remains in place.
A timeout means acceptance was not verified. It does not prove that traffic stopped.

The file pair is not one filesystem transaction. Intent and per-file outcomes permit recovery after an interrupted write.
Recovery keeps the original write ID. A newer operation cannot use an older operation's acceptance proof.

Other targets retain their existing evidence rules.
A one-shot `once` command drains notifications for a bounded time. The `run` service resumes remaining delivery and verification work.

## Acceptance evidence

The import comparator checks identities, names, parents, explicit identifiers, addresses, MAC identity, comments, and all four rates.
It follows producer CSV trim behavior and `f32` rate representation.
RADIUS usernames remain case-sensitive. Equivalent address ordering remains valid.
Missing required fields, duplicate identities, malformed addresses, and rate drift cannot produce a match.

The verifier checks the exact managed-input hashes and stable artifact bytes.
It calls the producer's read-only `validated_runtime_shaping_inputs_path()` helper.
It correlates runtime, shaping-input, generated-transaction, and native generations.
Two native samples must agree, including the Unix peer PID.
Unique independent operations may reorder. Policy fields, mappings, and operation counts must still match.
Unknown transaction fields and unsupported schema versions fail closed.
The checker does not mint a world or run the scheduler's readiness repair path.

The native envelope uses schema 1. The installed transaction uses schema 6.
The native content key describes policy operations. It excludes allocation capacity.
APPLIED therefore describes verified subscriber policy, while allocation evidence has its own health component.

The operator's final read-only probe passed on 2026-11-11.
It found **64 native circuit slots and 1000 generated slots**.
The policy matched, but this allocation mismatch remains **DEGRADED** in the health report.
No bridge configuration or allocator was changed.
Missing capacity metadata remains UNKNOWN. Do not treat it as a matching capacity.

Evidence stays outside Git:
`/home/pi/.local/state/lqosync3/unified-health-task1/verified-current.json`.
Private native captures contain subscriber mappings. Their files use mode `0600`.

## Shared health

Seven domains cover service/configuration, collection, validation/write, apply, dataplane, telemetry, and notification delivery.
Each observation has a component identity, source epoch, observation time, freshness budget, and reason code.
The report retains bridge uptime and degraded-subsystem names without copying subscriber inventories.
Fresh NIC data and NIC link health are separate observations.

The service publishes bounded `system-health.json` data and includes a snapshot in cycle status.
The TUI loader reads this cache outside rendering. It parses a changed cache only once.
The view ages observations even when the cache stops changing.
The approved Dashboard layout remains intact; its health panel supports keyboard and mouse scrolling.

Public cache limit: 128 KiB. Component limit: 128, plus bounded synthetic missing-domain/fault entries.
Native state replies retain the existing two-MiB bound and two-second request deadline.
Oversized or unavailable evidence remains unverified. It does not bypass acceptance checks.
Artifact reads have a combined 64-MiB limit.

## Telegram delivery

One worker sends queued events independently from the sync loop.
No notification delivery starts another RouterOS collection.
Reports freeze the original recipient, policy options, subscriber details, write ID, and health snapshot.
A recipient change pauses older events. The worker never forwards them to the new recipient.
Historical reports show their capture time. Multipart progress resumes after a restart.

A successful part requires HTTP success, Bot API `ok=true`, and a positive `message_id`.
The worker persists each API receipt before advancing to the next part.
It sends at most one request per three-second worker tick.
Transient errors use bounded exponential delays. A server `retry_after` delays the whole recipient queue.
Permanent API errors pause that event. The shared health report exposes the failure.
HTML splitting preserves supported tags, entities, and Unicode boundaries.

Delivery is at least once. A timeout can occur after Telegram accepts a part but before its receipt reaches the client.
The event/write ID helps identify such duplicates. Exactly-once chat delivery is not guaranteed.

The write-event queue and general-notice queue each reserve 256 events and 8 MiB.
Delivery records have a separate bounded budget. Each event or delivery record is limited to 256 KiB.
Queues and records use private directories and mode `0600` files.
Completed write receipts remain until a newer operation makes their recovery markers unnecessary.
Capacity or storage failures keep shaping active and expose a notification fault.
`write-event-gap.json` retains a delivery-gap warning across restarts. Do not clear it before the operator reviews the gap.

To recover a permanent delivery error, first fix its cause.
Stop the service before editing private delivery records.
Back up the state directory. Preserve `parts` and accepted `receipts`.
For the intended event only, set `paused` to `false` and `next_attempt` to `0`, then restart the service.
Keep its frozen route unchanged. A changed recipient must not receive an older private report.

A live reload cannot move `runtime.state_dir`. Restart the service to change that directory and its lock/receipt ownership.

## Development checks

Final local checks: 690 tests passed, zero failed, eight manual tests ignored.
All-target Clippy, formatting, the debug build, and seven-screen terminal smoke passed.

```bash
cargo test --locked --all-targets
cargo clippy --locked --all-targets -- -D warnings
cargo fmt --check
cargo run --example import_evidence_bench
cargo run --example import_evidence_bench -- --live
cargo run --example health_write_bench
python3 scripts/dev/capture_tui.py --screen 1 --system-health --output /tmp/dashboard-health.png
```

Benchmarks identify their build profile. Debug results are not production speedup claims.
Synthetic terminal captures are separate from live probe evidence.
This implementation has sent no live Telegram test message and has not changed the production service.

## Measured development results

The following measurements use debug builds on this host. They exclude import JSON parse time.
They are not production throughput or end-to-end cycle claims.

| Devices | Reused expected-input model | Rebuilt expected-input model |
| --- | ---: | ---: |
| 1,000 | 6.384 ms | 12.579 ms |
| 10,000 | 65.686 ms | 128.278 ms |
| 50,000 | 349.993 ms | 699.330 ms |

A test produced 1,000 health snapshots for 87 components in 1.023 seconds, with one unchanged-cache write.
A 108,032-byte HTML report produced 24 bounded parts in 32.43 ms.
The chart test reused its curves across 100 redraws with three 1,801-point histories.
Those redraws took 1.507 seconds in debug mode. The gapped-history case took 0.820 seconds.

The isolated receipt path took 63.366 ms for a changed 1k-row pair and 66.328 ms for a changed 10k-row pair.
Unchanged medians were 20 and 88 microseconds respectively. These measurements exclude CSV rendering and the full sync cycle.

All seven TUI routes passed actual terminal capture checks with isolated synthetic data.
The capture helper now waits for its terminal window instead of relying only on a fixed startup delay.
The Dashboard screenshot is linked in [the capture index](../design/tui/captures/README.md).

## Producer references

- [Input parser](https://github.com/LibreQoE/LibreQoS/blob/9ebe29bc/src/rust/lqos_config/src/shaped_devices/shaped_device.rs).
- [Source-generation validation](https://github.com/LibreQoE/LibreQoS/blob/9ebe29bc/src/rust/lqos_config/src/topology_runtime_state.rs).
- [Transaction policy identity and allocation metadata](https://github.com/LibreQoE/LibreQoS/blob/9ebe29bc/src/rust/lqos_dataplane_types/src/lib.rs).
- [Native state report](https://github.com/LibreQoE/LibreQoS/blob/9ebe29bc/rust/libreqos_dpdk/analytics_ipc/src/ipc.rs).
