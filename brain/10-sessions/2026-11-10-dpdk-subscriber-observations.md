---
type: session
created: 2026-11-10
tags: [dpdk, telemetry, subscribers]
status: implemented-not-deployed
related: ["[[2026-11-10-dpdk-bridge-health]]", "[[HANDOFF]]"]
---

# DPDK statistics: task 5

Updated: 2026-11-10T20:49:33+08:00 (Asia/Shanghai).

The owner approved the next checkpoint and ran the root-only subscriber probe.
Three samples contained 26 shaped hosts. All 26 matched applied circuit names and exact CSV IPs.
Slot 1 maps to the first circuit name. Direct zero-based lookup matched none.
Producer timestamps advanced. Delivered-byte and pooled/directional TCP fields were present.
The live CSV had no IPv6 or routed-prefix fields.

Evidence: `/home/pi/.local/state/lqosync3/dpdk-subscriber-task5/live-source.jsonl`.
Probe SHA256: `e0c9756dea403895a6461f599efbde4a0e59f5ae56e11c367c57f6e53da0e9c2`.
The probe prints counts only, without circuit names or subscriber IPs.
It does not claim measured subscriber rates or long-term RTT quality.

## Implementation

`src/subscriber_stats.rs` owns bounded host decoding, applied mapping, rate baselines, cache, ownership index, and labels.
`worker_util::ipc` reuses the descriptor transport with explicit request limits and reply kinds.
A separate sampler task shares the monitor's reload and shutdown lifecycle. It reads state/snapshot/state every ten seconds.
State generation and authoritative slot order must agree before and after the host read.
Two stable samples and producer progress establish live attribution. Each request keeps its two-second deadline and 2 MiB reply bound.

The source sample has at most 512 shaped hosts. Best-effort hosts and flow records are ignored.
The separate public cache stores metric summaries by circuit ID and host IP, within 1 MiB.
It stores no subscriber-name fields, credentials, remote flow identities, RTT rings, or packets.
Aggregate polling remains at limit 1 every two seconds. Its history/cache remain independent.

The join requires canonical exact host addresses and unique ownership. Host masks `/32` and `/128` are supported.
Routed prefixes remain unsupported. At most sixteen exact host IPs are supported per row.
Ownership includes overlapping prefixes and projected collection-only records. A missing shaping circuit remains unavailable.
Ownership indexes are cached across telemetry-only updates. Frames do not scan all subscriber rows.

The selected pane shows delivered rates/bytes, RTT sample medians, observed TCP retransmit counters, and packet age at read.
RTT uses retained source samples. Retransmits do not represent packet-loss percentages.
Missing hosts remain unknown. Stale values retain last-known labels and cannot show current rates.
Mapping changes, failures, resets, duplicates, missing counters, and gaps cannot invent rates.
The RouterOS collection cycle, online/offline state, shaping rules, and topology remain unchanged.

## Verification and review

Initial regression tests failed before implementation. Final suite: 596 passed, zero failed, seven manual tests ignored.
Formatting, all-target Clippy with warnings denied, build, whitespace, source probe, and terminal captures passed.
Tests cover source decoding, schema/slot order, counter resets, omissions, optional fields, conflicts, cache limits, and sampler lifecycle.
The final review found and fixed ownership projection for collection-only rows. It also checked mouse focus and wrapped page scrolling.
The pane title keeps the selected subscriber visible. Copy uses the same scope and stale labels.

The maximum 512-host cache fixture measured 351,809 bytes and 35.1 ms for one debug publish/read run.
A 75,000-row ownership index took 634.9 ms in one debug run. This work occurs on row changes, not on frames.
These synthetic results are not production performance guarantees.
Captures: `docs/design/tui/captures/subscribers-dpdk-v1.png` and `subscribers-dpdk-default-v1.png`.
They use isolated synthetic records. Logs: `/home/pi/.local/state/lqosync3/dpdk-subscriber-*.log`.

No dependency, runtime key, router read/write, push, deployment, restart, or production configuration change occurred.
Next: Task 6 on-demand circuit queue diagnostics.
Preserve the unrelated local-profile Knowledge link and note.
Source: https://github.com/LibreQoE/LibreQoS/blob/9ebe29bc/rust/libreqos_dpdk/analytics_ipc/src/lib.rs
