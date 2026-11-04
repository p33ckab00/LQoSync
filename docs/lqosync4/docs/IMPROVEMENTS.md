# Improvements to resolve before implementation

| Priority | Improvement | Why it matters | Proof |
|---|---|---|---|
| P0 | Define one stable subscriber ID across PPPoE reconnects, DHCP renewal, and IP changes. | An IP is not a stable account identity. Wrong identity can move the wrong subscriber. | Reconnect and address-change fixtures keep one ID. |
| P0 | Define a narrow MikroTik write contract and object ownership. | The LQoSync3 code currently reads routers only. A broad writer can overwrite operator rules. | Readback, idempotency, conflict and rollback tests against a test router. |
| P0 | Separate desired, observed, and applied WAN state. | A successful API request does not prove that the router changed. | UI shows each state and failed readback. |
| P0 | Define health `unknown`, `suspect`, `down`, and `recovering`, plus thresholds. | One failed probe or stale monitor result should not cause mass movement. | Flap and stale-monitor simulations. |
| P1 | Version previews and require a fresh version at apply. | A stale tab could apply an old plan after health changed. | Stale plan is rejected and re-previewed. |
| P1 | Use bounded queues, paged APIs, and time-bucket traffic. | Large tables and fast samples can overload the server or browser. | 1,000 and 10,000 subscriber workload measurements. |
| P1 | Define schedule timezone and overlap priority. | An override can run at the wrong time or win unexpectedly. | Midnight, daylight-saving, overlap, and restart fixtures. |
| P1 | Publish data age and collector error with every operational view. | A green badge on stale data is misleading. | Unreachable-router test shows stale/unknown. |
| P2 | Plan IPv6 and multi-WAN/CGNAT identity separately from IP intelligence. | Destination and subscriber IPs have different meanings. | IPv4/IPv6 sample validation. |
| P2 | Specify telemetry retention and confidence before app labels. | Shared CDN IPs can misidentify an app; detailed traffic data needs access limits. | Accuracy sample and retention test. |

No numerical performance guarantee is set yet. Measure baseline on the target hardware, then set service budgets and a regression threshold.
