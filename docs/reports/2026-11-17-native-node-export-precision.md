# Native node export precision

Timestamp: 2026-11-17T06:28:03+08:00, Asia/Shanghai.
Installed package: `3.0.7.202611162151`.
Binding: `/opt/libreqos/src/liblqos_python.so`.
Binding SHA-256: `b62cf25ed8c25b443c7a7676d46941cd94558a96195e2ae4881de34d96bbaab2`.

## Method

The existing `tools/v3sim/check.sh` compiles synthetic pairs in new scratch directories.
It uses a copy of `lqos.example`, redirected source/state paths, blocked socket connections, and the existing write audit.
The compiler does not apply a native world. No live source, state, configuration, service, or router changes.
The sample addresses are documentation addresses. The sample does not contain customer data.

The checker now compares requested policy with canonical and compiled shaping artifacts.
It checks exact IDs and policy rather than an old `lq3-` prefix.
Its example command cannot authorize fresh subscribers or publication from serialized rows.

## Results

| Requested policy | Result |
|---|---|
| Node 100 Mbps, subscriber 50 Mbps | Compiler and admission check pass |
| Node 100.5 Mbps | Compiler reports success, but intrinsic node rate is 101 Mbps |
| Node 100.5 Mbps | Admission rejects the changed intrinsic rate |
| Node 100 Mbps, subscriber 50.5 Mbps | Compiler and admission check pass with unchanged subscriber maximum |

The compatibility tree alone does not expose the intrinsic-rate increase.
Admission must check the canonical intrinsic rates as well as the tree and compiled devices.
These results apply to the binding above. The runtime owner must bind future results to its compiler/configuration identity.

## Implementation rule

Use the approved ceiling-safe export rule for the current whole-Mbps node contract.
Keep the calculated value in policy. Export the node ceiling downward to whole Mbps.
Expose both values through `node_export_adjustments` for preview.
Reject an exported value below 1 Mbps, subscriber minimum commitments, or a configured automatic group floor.
Keep subscriber min/max speeds unchanged. Reject unsupported subscriber precision instead of changing its value.

This rule does not complete full shaping-planner admission or a live DPDK acceptance test.
The existing post-write verifier and the remaining runtime activation gates still apply.

Evidence logs: `.superpowers/sdd/2026-11-17-topology-migration-admission/`.
Scratch compiler outputs: `target/v3sim/`.
