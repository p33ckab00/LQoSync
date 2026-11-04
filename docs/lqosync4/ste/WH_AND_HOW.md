# LQoSync4 WHAT, WHY, WHO, WHEN, WHERE, and HOW

This file uses short, controlled English. **Existing** refers to LQoSync3 source on this branch. **Proposed** refers to LQoSync4 design. Each answer identifies the actor and the system boundary. See [function map](../docs/FEATURE_FUNCTION_MAP.md) and [workflows](../docs/ARCHITECTURE_WORKFLOWS.md).

| Feature | WHAT? | WHY? | WHO? | WHEN? | WHERE? | HOW? |
|---|---|---|---|---|---|---|
| Subscriber sync | Read and validate customer connections. | Keep LibreQoS input correct. | Collector and core model. | At a configured scan interval or an approved run. | RouterOS to LQoSync4 to LibreQoS. | Read PPP/DHCP/Hotspot tables; assemble; validate; write changed output. **Existing core.** |
| LibreQoS v3 input | Write the selected source format. | Keep the shaping source in sync with subscribers. | LQoSync writer writes; LibreQoS imports. | After a valid cycle changes the output. | `ShapedDevices.csv` plus `network.json`, or `libreqos_network.json`. | Detect the v3 source; render and back up; write changed files; verify the pair import. Native JSON needs an accepted-generation check. See [v3 data flow](../docs/LIBREQOS_V3_DATA_FLOW.md). **Existing core with proposed verification improvement.** |
| Dashboard | Show current counts, traffic, health, and sync state. | Let an operator detect a problem. | Operator reads; snapshot service provides data. | On page load and on a version change. | Leptos UI and Rust API. | Read a validated, time-stamped snapshot. Do not scan a router in an HTTP request. **Proposed.** |
| WAN assignment | Give each subscriber one effective upstream. | Keep the path stable and distribute load. | Planner decides; MikroTik core routes. | After policy, capacity, or health changes. | LQoSync4 planner and MikroTik core. | Apply percentage, fixed-count, or manual policy; show a diff; write approved objects; read back. **Proposed.** |
| Failover | Move subscribers from a down upstream. | Keep service available. | Health service and planner; MikroTik core routes. | After a failure threshold. | Health service, planner, and core router. | Mark down; exclude path; calculate affected moves; apply per policy; audit. **Proposed.** |
| Recovery | Return a stable upstream to the eligible pool. | Avoid repeated movement during flapping. | Health service and planner. | After a recovery hold-down. | LQoSync4 and core router. | Mark recovering; wait; re-enter; follow the chosen failback rule. **Proposed.** |
| Profiles | Set base and scheduled speed. | Apply subscriber plan rules at the right time. | Operator defines; scheduler resolves. | When a schedule becomes active or ends. | Leptos editor, Rust scheduler, RouterOS/LibreQoS owner. | Select the highest-priority active override; preview effective speed; apply and read back. **Proposed.** |
| Audit | Record who changed policy and what happened. | Make changes traceable. | API and writers. | For each state-changing request and result. | Persistent audit store. | Store actor, reason, before/after, time, and outcome; exclude secrets. **Proposed.** |
| IP intelligence | Identify destination IP, ASN, organization, CDN, and possible app. | Explain traffic by destination and service. | Telemetry and enrichment services. | In a later phase, after source and retention approval. | Separate telemetry store and UI view. | Observe IP/bytes/time; enrich from cached data; show confidence or unknown. **Roadmap.** |

## Questions that still need an owner answer

- **WHAT RouterOS objects may LQoSync4 write?** The core-router contract is not defined.
- **WHO may approve and apply a WAN or profile change?** The exact role matrix is not defined.
- **WHEN does failover apply automatically?** The automatic-apply policy is not defined.
- **WHEN does failback move subscribers?** The recovery policy is not defined.
- **WHERE does upstream health come from?** The monitor source and thresholds are not defined.
- **HOW is a subscriber identified across an IP change?** The stable identity rule needs sample data.
- **HOW long is traffic telemetry retained?** The source, retention, and access policy are not defined.
