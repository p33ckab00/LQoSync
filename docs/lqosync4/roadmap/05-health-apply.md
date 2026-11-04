# Phase 5 — Health, failover and controlled apply

**Entry:** A02 plan/diff, P02 command path, G2 writable-object contract and G3 health/failback policy. **Exit:** A reviewed plan can be applied safely and read back. Production activation is a separate operator action.

| ID / owner | Depends | Implementation instruction | Acceptance evidence |
|---|---|---|---|
| H01 / Core backend | A01, G3 | Implement sampled health states `unknown`, `up`, `down`, `recovering`, with freshness, failure threshold, hold-down and transition history. Replan only when policy permits. Simulate manual pins on outage and the chosen failback rule. | Missing/stale samples, flapping, outage, recovery, restart and no-known-down assignment tests. WAN-02…03. |
| H02 / Core backend + middle layer | A02, H01, G2, G3 | Implement one narrowly scoped WAN-assignment RouterOS writer. Before apply, translate the logical plan into the exact G2 object diff and require authorization plus a current reviewed plan ID. Serialize writes, compare desired/observed state, apply only changed objects, read back each result, retry within bounds, and record partial failures plus compensating/rollback instructions. Keep shadow path available. | Unauthorized and stale-plan rejection, repeat apply makes zero writes, timeout/partial failure/restart/readback tests, redacted audit and exact object diff. WAN-04…06, SAFE-01…02, PERF-04, PERF-06. |
| H03 / Frontend + verifier | H01, H02 | Show health age and transition reason in Dashboard and Load Balancer. Wire preview/approve/apply/retry/rollback status to server jobs. Keep pending, written and verified results separate. Verify the full simulated operator flow before any live use. | Browser and integration tests for outage/recovery, denied write, stale plan and partial result; 320 px actions remain reachable. UI-03, UI-06, UI-08…14. |

MikroTik core owns routing and sessions after assignment changes. Failover can change a public IP and interrupt active sessions; this is accepted. Do not invent a guarantee of seamless session continuity. The health provider is configurable and must be chosen through G3, not hard-coded to one vendor.
