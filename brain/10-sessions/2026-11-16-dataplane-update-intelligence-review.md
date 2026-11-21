---
type: session
created: 2026-11-16
tags: [libreqos-v3, dataplane, intelligence, research]
status: reviewed
related: ["[[2026-11-16-ram-cache-deployment]]", "[[HANDOFF]]"]
---

# Dataplane update and intelligence review

Reviewed: 2026-11-16T09:50:25+08:00 (Asia/Shanghai). Agent: Codex `/root`.
The owner requested a review of new LibreQoS changes and possible intelligence integration.

Installed package: `3.0.7.202611141944`. Bridge revision: `193d067a6ad9`.
The public source remains at beta19. Cached packages supply the latest package comparison.
Confirmed additions include NUMA-aware hugepage placement and bridge-unit dependencies.
Source-map diffs show corrected circuit delivered/demand assignments and a worker occupancy-meter fix.
The wider package comparison exposes a new `slot_numbers` field.

Three corrected native samples show 27 names and 27 nested-CBOR slot numbers.
Those numbers are unique, positive, and currently dense one-based.
LQoSync3 currently derives slots by position. Explicit slot-number support is the recommended first compatibility change.
The decoder passed 15 fixture checks. The first probe counted encoded bytes and is superseded.

The host rebooted outside the review commands at 2026-11-16T09:39:36+08:00.
Native policy verification returned exit code 1 after reboot. Shared apply health is `import_confirmed_ack_unproven`.
No forced write, product-code change, restart, or package installation occurred in this review.
Do not attribute the policy-proof failure to a specific upstream defect without more evidence.

After compatibility checks, prioritize delivered/demand/plan comparisons, worker occupancy, and bounded flow/QoE views.
Public source already supports flow completion records and ASN/country aggregates.
Those are existing integration options. No new DPI interface was confirmed.

Full report: `docs/operations/DATAPLANE_UPDATE_REVIEW_2026_11_16.md`.
Evidence: `/home/pi/.local/state/lqosync3/dataplane-update-review/`.
The direct remote package-index request returned HTTP 403. Do not claim global latest-version verification.
Preserve the unrelated profile/WAN notes and existing deployment documentation.
