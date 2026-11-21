# LibreQoS dataplane update review

Reviewed: 2026-11-16T09:50:25+08:00 (Asia/Shanghai).
Scope: package changes and possible LQoSync3 intelligence features. No production code change.

## Versions and evidence

Installed packages: `libreqos`, `libreqos-dataplane`, and `libreqos-preflight` report `3.0.7.202611141944`.
APT history records installation on 2026-11-15 at 12:31:48+08:00.
The cached predecessor is `3.0.7.202611141145`.
The wider comparison starts at `3.0.7.202611102128`.
Installed bridge revision: `193d067a6ad9`.

Public `main` still points to beta19 projection `9ebe29bc9d49013aff9b0544de3c6f6fc6ba2f1d` from 2026-11-08.
That projection states that the proprietary dataplane source is not published.
Do not equate the current package with that public commit.
Direct remote package-index access returned HTTP 403. The global latest package version remains unverified.

Local evidence: `/home/pi/.local/state/lqosync3/dataplane-update-review/`.
The review compares cached package file hashes, maintainer scripts, and JavaScript source maps.
It also checks static binary field presence and bounded native metadata.
Static strings indicate candidate interfaces. They do not prove behavior or a complete changelog.

## Confirmed latest package changes

1. NUMA-aware hugepage placement now has a helper and an enabled systemd oneshot.
   The bridge unit requests that oneshot through `Wants=`.
   The package installer invokes the helper and reports placement refusal.
   The helper budgets pages from circuit count and RAM, with an explicit operator override.
   The current host reports 3072 two-MiB hugepages. The placement unit reports success.
   LQoSync3 should observe this state. It must not compete with LibreQoS for page allocation.
2. The circuit UI corrected reversed rate assignments.
   `bytes_per_second` is queued demand. `actual_bytes_per_second` is delivered traffic in that API contract.
   The previous assignments used demand as throughput and could show misleading utilization.
   This does not mean that direct analytics snapshot fields share identical names or semantics.
   LQoSync3 currently reads direct aggregate `actual` counters. Do not swap those counters from this UI diff alone.
3. The Workers UI now calls `occupancySegmentsHtml` for its worker meter.
   That function includes busy-phase occupancy and idle space in one bar.
   LQoSync3 can adopt the presentation after a formula and denominator review.
4. Package binaries, Python dependencies, and associated assets changed.
   The latest preflight package has no file-content changes in the package comparison.

## Wider dataplane interface change

The bridge binary contains `slot_numbers` in the November 14 packages.
The November 10 comparison binary lacks that field.
The existing LQoSync3 reader consumes only `generation`, `report_schema`, and encoded `slot_order`.
Subscriber metrics and circuit queue selection currently derive a slot from list position plus one.

The operator ran the read-only native probe after a host reboot.
Three samples report schema 1, transaction schema 6, 27 policy leaf operations, and 27 slot names.
`slot_numbers` is nested CBOR. Its decoded array has 27 unique positive entries and matches the name count.
The current array is dense and one-based. This sample does not prove a current positional mapping error.
It also does not prove behavior for sparse placement or a later topology change.
The metadata decoder passed 15 fixtures across direct arrays, encoded integer arrays, and byte strings.
The first probe counted encoded bytes. Only the corrected probe result applies.

The native policy probe returned exit code 1 after reboot.
Shared apply health reports `import_confirmed_ack_unproven`.
Do not report the current policy as APPLIED or attribute this failure to the update without more evidence.
The capacity report remains 64. The owner previously deferred its separate investigation.

## Adoption order

| Priority | Change | Acceptance condition |
|---|---|---|
| P0 | Decode explicit slot numbers and join them to names | Preserve legacy support, reject malformed arrays and duplicates, test sparse IDs and generation changes |
| P0 | Diagnose current post-reboot policy evidence | Explain the failed proof without a forced write or bridge restart |
| P1 | Compare delivered traffic, queued demand, and plan ceilings | Verify the source contract, preserve missing values, label each direction and denominator |
| P1 | Show busy phases with idle occupancy | Phase segments fit within measured busy occupancy and never imply an OS CPU measurement |
| P2 | Add flow and QoE intelligence | Confirm exposed endpoints, sample bounds, source epoch, freshness, and flow coverage |
| P2 | Add NUMA and hugepage health details | Read-only observation, no automatic page allocation or tuning |

The first intelligence views can show protocol mix, active flow counts, top remote ASNs, and country totals.
They can combine directional RTT, retransmits, delivered rates, and queue pressure for troubleshooting.
Use existing lqosd aggregates where possible. Avoid duplicate collectors and unbounded flow history.
Retain unknown classifications and explicit loss/gap counters. Do not label port guesses as application identification.

The public analytics protocol already contains flow snapshots and completion records.
Its public flow history code already resolves ASN and country dimensions.
These are existing capabilities to integrate, not features proven new in this last package.
No new DPI application-classification interface was confirmed.
Protocol, ASN, geography, and QoE analysis can provide intelligence without a new packet-inspection engine.

## Limits and changes

The review did not install packages, change configuration, restart services, or collect packet payloads.
The host reboot happened outside this review's commands.
Only metadata probes and local research files were written.
Production product-code tests do not apply to this documentation-only review.
Probe syntax, package extraction, hash comparisons, and 15 decoder fixtures passed.
The live policy proof failed and remains an explicit open item.

## Sources

- [Public beta19 projection](https://github.com/LibreQoE/LibreQoS/commit/9ebe29bc9d49013aff9b0544de3c6f6fc6ba2f1d).
- [Public analytics protocol](https://github.com/LibreQoE/LibreQoS/blob/9ebe29bc9d49013aff9b0544de3c6f6fc6ba2f1d/rust/libreqos_dpdk/analytics_ipc/src/ipc.rs).
- [Public analytics records](https://github.com/LibreQoE/LibreQoS/blob/9ebe29bc9d49013aff9b0544de3c6f6fc6ba2f1d/rust/libreqos_dpdk/analytics_ipc/src/lib.rs).
- [Public flow history and ASN/country aggregates](https://github.com/LibreQoE/LibreQoS/blob/9ebe29bc9d49013aff9b0544de3c6f6fc6ba2f1d/src/rust/lqosd/src/bridge_analytics/flow_history.rs).
- Local cached DEBs and extracted source maps. Their source diffs are in the private evidence directory.
