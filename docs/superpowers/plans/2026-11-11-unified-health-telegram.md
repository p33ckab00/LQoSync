# Unified health and Telegram implementation plan

> **For agentic workers:** Use superpowers:executing-plans to implement this plan task by task. Execute inline in the existing `lqosync3` workspace. Preserve unrelated work.

**Goal:** Report each managed-file update as WRITTEN/PENDING, then report its verified outcome without delaying subscriber collection.

**Architecture:** Reuse Rust samplers and runner outputs. Separate health snapshots, durable write receipts, and notification delivery. Verify native acknowledgement before enabling APPLIED.

**Tech Stack:** Rust 1.89, Tokio, serde, existing CBOR analytics transport. No new dependency.

**Spec:** `docs/superpowers/specs/2026-11-11-unified-system-health-telegram-design.md`.

## Global constraints

- Preserve RouterOS collection cadence and speed-profile resolution.
- Keep the existing forty-minute import limit.
- Treat the file pair as one logical operation with per-file outcomes.
- A file rename, import, generated transaction, and native acknowledgement are distinct observations.
- Do not send test Telegram messages or change the production service during implementation.
- Keep write events distinct; coalesce only replaceable health observations.
- Do not commit, push, or deploy without a request for this implementation.

## Review focus

- Address-only and minimum-rate changes must not match an older import.
- Invalid or missing imported fields must fail closed.
- A second-file failure must retain the first file's write outcome.
- Restart and superseding writes must not reuse an older operation's acceptance evidence.
- Stopped producers must expire without another producer event.

## Task 1: Import evidence and native acknowledgement contract

**Files:** `src/import_evidence.rs`, `src/files.rs`, `src/runner.rs`, `src/lib.rs`, `src/worker_util/ipc.rs`, `src/apply_evidence.rs`, `examples/dpdk_apply_probe.rs`.

**Interfaces:** `ExpectedImport::new(network: &str, rows: &[CsvRow]) -> Result<Self>` validates and normalizes expected inputs. `matches(&self, imported: &Value) -> bool` checks one parsed artifact. `read_applied(path: &Path)` reuses the read-only state transport. The probe emits only aggregate contract checks.

- [x] Write regression tests for address/minimum-rate drift, ordering, duplicate identities, and malformed evidence; observe failure.
- [x] Implement normalized import comparison and migrate existing fixtures to the installed producer's full schema.
- [x] Test applied-state CBOR parsing, schema bounds, generation mismatches, unavailable state, and interrupted observations.
- [x] Build a read-only probe that compares stable runtime, generated transaction, and native state samples. Do not claim input-to-ACK proof from generation equality alone.
- [x] Run focused tests and inspect live probe results. Root-only socket access may require operator execution.

## Task 2: Health model and durable write receipts

**Files:** `src/system_health.rs`, `src/write_receipt.rs`, `src/runner.rs`, `src/status.rs`, existing sampler publication points.

**Interfaces:** Seven domain observations carry state, reason, source epoch, observation time, and freshness budget. A persisted receipt identifies a logical write and each file's outcome. Consumers read the same summary.

- [x] Test freshness expiry, disabled inputs, last-good dataplane with pending update, and component identity.
- [x] Test unchanged cycles, network-only and CSV-only changes, partial writes, restart recovery, and superseding writes.
- [x] Persist intent before replacement and outcomes after each replacement. Surface storage errors without fabricating acceptance.
- [x] Use the validated Task 1 contract for pending-only checks. Cache normalized expected data per operation.
- [x] Publish a bounded health summary without subscriber inventories or repeated source reads.

## Task 3: Persistent asynchronous Telegram delivery

**Files:** `src/notify.rs`, `src/notify/outbox.rs`, `src/main.rs`, `src/runner.rs`.

**Interfaces:** The service-owned worker consumes persisted event payloads and records per-part API receipts. Write receipts supply operation identity and phase; health supplies the event snapshot.

- [x] Test a PENDING report followed by a terminal report without another file change.
- [x] Test HTTP/API failures, 429 retry delays, multipart recovery, queue capacity, restart, recipient changes, and Unicode/HTML bounds.
- [x] Move service sends off the cycle loop. Keep one worker, bounded work, explicit delivery faults, and per-write ordering.
- [x] Preserve existing subscriber-change detail options. Label delayed historical reports.
- [x] Prove unchanged collection cadence and responsive shutdown during delivery failure.

## Task 4: Shared TUI projection and release checks

**Files:** existing TUI health views/loaders, health documentation, capture index, Brain session and handoff.

- [x] Read the bounded health summary through existing cache loading. Keep source I/O out of rendering.
- [x] Test stale data and missing/disabled sources. Capture actual terminal views.
- [x] Run formatting, all-target Clippy, full tests, and seven-screen smoke checks.
- [x] Measure changed/unchanged receipt work, comparison cost, health cache writes, queue recovery, and render cost at representative scale. Keep full RouterOS-cycle latency separate.
- [x] Record results, limitations, and deployment prerequisites. Leave production unchanged until deployment is requested.

## Completion evidence

All four tasks are complete locally. No production deployment or live Telegram test send occurred.
Final checks: 690 passed, zero failed, eight manual tests ignored; all-target Clippy, formatting, build, and seven-screen terminal smoke passed.
The operator's read-only native policy proof passed. Native/generated slot capacity differs and remains separately DEGRADED.
See [operations and measured limits](../../operations/UNIFIED_HEALTH_TASK1.md) and the Brain implementation session.
