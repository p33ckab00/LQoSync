# Phase 6 — Traffic history and destination intelligence

**Entry:** W01 snapshot path for traffic history. Destination work also requires G4 source, identity, access and retention decisions. **Exit:** Measured telemetry has clear freshness and classification limits.

| ID / owner | Depends | Implementation instruction | Acceptance evidence |
|---|---|---|---|
| T01 / Core backend + middle layer + frontend | W01, W02 | Collect bounded upstream RX/TX and utilization samples. Store time-bucket aggregates. Expose windowed queries and render Dashboard/Allocation charts only while visible. Match sample windows and capacity denominators; do not sum unrelated WAN utilization percentages. | Empty/stale chart states, mixed-capacity math, background-tab traffic, bytes/refresh, 1k/10k load measurements. UI-02…03, PERF-03…04, PERF-07, PERF-09. |
| T02 / Core backend + middle layer | T01, G4 | Define subscriber-to-destination-IP observation with event time, direction, bytes, IPv4/IPv6 and stable subscriber ID. Apply access control, bounded ingestion and deletion/retention rules. Keep raw observations separate from shaping and assignment state. | Identity across subscriber IP change, CGNAT/IPv6 fixture, bounded storage/retry, authorized query and retention tests. IP-01, SAFE-01. |
| T03 / Core backend + frontend + verifier | T02 | Enrich destinations with versioned, cached ASN and organization data. Label CDN/app only from evidence; show confidence and `unknown` for ambiguous shared IPs. Add `/traffic-intelligence` after its API and privacy gate pass. | Sample accuracy/confusion report, stale enrichment behavior, unknown/shared-CDN cases, browser source/confidence display. IP-02…03. |

The IP intelligence route is future work. A destination IP alone does not identify an app reliably. See [product spec](../docs/PRODUCT_SPEC.md) and [WH/HOW](../ste/WH_AND_HOW.md).
