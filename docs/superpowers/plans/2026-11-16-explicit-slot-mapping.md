# Explicit Slot Mapping Implementation Plan

**Goal:** Use native slot IDs for subscriber metrics and circuit queue requests.

**Architecture:** Decode the optional nested-CBOR `slot_numbers` field in the shared applied-state reader.
Validate IDs and names together. Store an ordered lookup from slot ID to circuit name.
Use one-based IDs only when an older bridge omits the field.
Preserve generation checks, cache schema, polling, and read-only native requests.

**Tech Stack:** Rust, Serde, ciborium, BTreeMap, existing Unix-socket test fixtures.

- [x] Add regressions for sparse and reordered IDs, malformed fields, legacy mapping, and mapping changes.
- [x] Run the regressions against the old reader and confirm behavior failures.
- [x] Update the shared mapping, subscriber tracker, circuit lookup, and source probe.
- [x] Review bounds, lookup cost, stale-state handling, and all callers.
- [x] Run focused tests, all-target tests, Clippy, formatting, and diff checks.
- [x] Record results and commit only this task locally. Do not push or deploy.

Post-reboot native policy diagnosis and intelligence charts are separate tasks.
