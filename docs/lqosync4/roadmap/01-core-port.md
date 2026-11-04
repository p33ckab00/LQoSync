# Phase 1 — Core port and LibreQoS parity

**Entry:** E01 fixtures and baseline exist. **Exit:** TUI-free Rust core preserves the validated LQoSync3 shaping path. No WAN writer or web app is implied by this exit.

| ID / owner | Depends | Implementation instruction | Acceptance evidence |
|---|---|---|---|
| C01 / Core backend | E01 | Remove TUI command/module/dependencies. Keep sync CLI or service entry needed for scheduled and one-shot cycles. Preserve config interpretation or provide a tested migration with rollback. | Fresh install, config load, one-shot and scheduled cycle tests; no `ratatui`/TUI path. CORE-04…06. |
| C02 / Core backend | C01, E03 | Isolate RouterOS read collection from validation and shaping writes. Use bounded per-router work, cancellation, timeout, and contextual errors. Publish a validated cycle result for snapshots. Preserve shadow behavior, backups, `write_if_changed`, and selected v2/v3 target semantics. | Slow/unreachable-router and unchanged-cycle tests; bounded work and no live write in shadow mode. CORE-01…04, PERF-01, PERF-04, PERF-06. |
| C03 / Core backend + verifier | C02 | Compare v2 pair, v3 pair and v3 native output with golden fixtures. Verify v3 pair reload/import states and recovery after interruption between its two file replacements. Keep native apply gated by the installed probe and accepted-generation evidence. | Byte/semantic parity as appropriate, changed/unchanged tests, delayed/rejected import tests, documented native limitation. CORE-02…04. |

The `v3_pair` files are individually replaced, so the task must test partial update recovery. A native JSON file write does not prove LibreQoS accepted it. See [the exact data flow](../docs/LIBREQOS_V3_DATA_FLOW.md).
