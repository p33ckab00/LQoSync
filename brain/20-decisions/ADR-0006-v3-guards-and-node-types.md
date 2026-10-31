---
type: decision
created: 2026-10-31
tags: [adr, topology, libreqos-v3]
status: accepted
supersedes:
related: ["[[ADR-0005-topology-follows-v3]]", "[[ADR-0002-v3-target-selection]]", "[[v3-node-handling]]"]
---

# ADR-0006 — v3 guards for `no_parent` and `compile_mode`, and `site` group nodes on v3

## Context

LibreQoS v3 code says that a producer must not send an empty Parent Node ID, because the bridge boot gate then holds. The mode `no_parent` gives an empty Parent Node.
With `compile_mode = "flat"`, v3 ignores the parents and uses its own buckets (`Parent node ... is invalid for a flat network`).
The v3 example uses `compile_mode = "ap_site"`. This mode can keep only `site` and `ap` nodes. The offline compiler does not show the result. Refer to [[v3-node-handling]].

## Decision

1. If the target is `v3_pair` or `v3_network` and `network.mode = "no_parent"`, `check` fails and the cycle stops. The message tells the operator to use `root_promotion`.
2. On LibreQoS v3, if `[topology] compile_mode` is not `full`, `check` and each cycle give a warning. The tool still writes the files.
3. On a v3 target, the default type of each group node (`ppp`, `ppp_plan`, `hotspot`, `dhcp_site`, `dhcp_plan`) is `site`. On `v2_pair`, the defaults stay as in `updatecsv.py` (`plan` and `site`). A `type` in `[nodes]` overrides both defaults.

## Alternatives

1. Keep `plan` on v3 and wait for a test on a v3 shaper. If `ap_site` drops `plan` nodes, the subscribers lose their group caps.
2. Map `no_parent` to one synthetic root on v3. All circuits then share one HTB root and one CPU.

## Consequences

- The core receives a flag that tells it if the target is v3. Plan 3 sets it from the target detector.
- A fixed `target = "v3_pair"` or `"v3_network"` with `no_parent` fails at config load. With `target = "auto"`, the detector does the check.
- The `client` node type of `flat_promotion` stays `client` on v3.

## Status
accepted on 2026-10-31 (operator decision in chat)
