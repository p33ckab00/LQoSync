# Phase 0 — Evidence and contracts

**Exit:** A coding agent can reproduce the current behavior and identify each open decision before changing a write path. All tasks start as **planned**.

| ID / owner | Depends | Implementation instruction | Acceptance evidence |
|---|---|---|---|
| E01 / Mapper + core backend | None | Pin the base commit. Trace CLI, TUI, configuration, RouterOS reads, `runner::cycle_inputs`, `cycle::assemble`, validation, detection, v2/v3 renderers, shadow/apply, backup, import check and tests. Capture sanitized fixtures for PPPoE, DHCP, Hotspot, missing router, and each LibreQoS target. | Source/function matrix and reproducible baseline commands; fixtures contain no credentials or customer secrets. CORE-01…05. |
| E02 / Coordinator + middle layer | E01 | Use LQoSync4 web workflows and routes as the product contract. Resolve G0 framework, store, auth/session and role matrix for the first web slice. Track G1–G4 questions as open without blocking phase 2. Freeze first-slice DTO fields, version/error semantics, and `/api/v1` endpoint names with backend/frontend owners. | G0 decision evidence, typed contract proposal, negative/unknown states. No proposal is marked accepted without owner evidence. SAFE-01…02. |
| E03 / Verifier + core backend | E01 | Measure current cycle duration, peak memory, output bytes, RouterOS calls, and unchanged-write count on documented 1k and 10k fixtures where available. Record target hardware and command. Inspect two-file v3 pair interruption and native import limits. | Repeatable baseline and risk register; no invented performance threshold. PERF-06…07. |

Read [source mapping](../docs/FEATURE_FUNCTION_MAP.md), [v3 ingress map](../docs/LIBREQOS_V3_DATA_FLOW.md), [backend options](../docs/RUST_BACKEND_OPTIONS.md), and [open decisions](../brain/DECISIONS.md). Work on E01 and the non-decision parts of E03 can proceed before G0 is resolved.
