---
type: session
created: 2026-11-11
tags: [health, telegram, research, design]
status: review-complete-design-draft
related: ["[[2026-11-11-dpdk-nic-diagnostics]]", "[[HANDOFF]]"]
---

# Unified system health and Telegram research

Updated: 2026-11-11T15:36:15+08:00 (Asia/Shanghai).

The owner requested review, research, and brainstorming for unified system health.
Each managed-file write must include a Telegram health report.
The owner selected an immediate WRITTEN/PENDING report and a later APPLIED or FAILED follow-up.
The selected interaction does not authorize sending test messages during this research.

## Findings

The current notifier already triggers on `files_changed`, but can title a pending import as an applied update.
It has no independent trigger for a later import-confirmed transition without another file change.
Its telemetry section contains cycle/router details and legacy bridge counters, rather than the new sampler health summaries.
The cycle loop awaits Telegram delivery before its next sleep/signal select.
Failed sends are not retried under the original policy. Event memory advances before delivery.
The transport treats HTTP 2xx as success without retaining the Bot API receipt.

Each managed-file rename is atomic. The pair is not one atomic filesystem transaction.
A second-file failure can leave a partial write, while the outer cycle error returns `changed=false`.
The import comparator does not cover all policy-critical fields, including IP addresses and minimum rates.
Its existing timeout is forty minutes. Normal sync cycles currently perform the checks.

The installed runtime status has source/shaping/content generation fields.
Its shaping generation matched the generated transaction in a read-only observation.
The pinned native state contract exposes the last acknowledged generation and applied transaction.
Those generation types have different meanings. Applied proof needs a validated correlation contract before implementation.
The installed scheduler readiness path can participate in world construction. This review did not execute it.

## Proposed design and research

Draft: `docs/superpowers/specs/2026-11-11-unified-system-health-telegram-design.md`.
Use one shared health model with domain status, freshness, applicability, source epoch, and reasons.
Keep update phase separate from health. A previous applied dataplane can remain healthy while a new update waits or fails verification.
Use a persisted write identity and per-file receipts. Handle partial writes, restarts, and superseding updates explicitly.
A bounded private outbox and separate Telegram worker preserve sync cadence during delivery outages.
Respect API result envelopes, rate limits, retry delays, HTML/Unicode limits, and partial delivery receipts.
Do not promise exactly-once delivery across ambiguous network timeouts.

The owner selected the two-stage write-report flow. Independent health incident/recovery alerts remain a recommendation.
Queue limits and detailed aggregation policy are proposals, not accepted production settings.
Keep speed-profile resolution and subscriber collection cadence unchanged.

Reviewed official Telegram API/FAQ, Prometheus Alertmanager concepts, and Google SRE guidance.
Source links and failure checks are in the draft.
The project graph located notifier/runner/status relationships. Current files confirmed the findings.
The 25 focused Telegram tests passed. No production messages or service changes occurred.
Research artifacts: `/home/pi/.local/state/lqosync3/unified-health-research/`.

## Performance follow-up

The owner asked whether the design can reduce performance overhead.
The draft now specifies receipt reuse, event-driven evaluation with freshness deadlines, and pending-only apply checks.
Cache normalized expected outputs per write and parsed import evidence per coherent source generation.
Do not scan full subscriber inventories on each health tick or add RouterOS collection calls.
Use one asynchronous delivery worker and bounded persistent work. Preserve each write event during snapshot coalescing.
Measure cycle overhead, CPU, allocations, cache writes, queue recovery, and TUI render cost before claiming a gain.
No implementation or performance benchmark occurred during this design follow-up.

## Handoff

No product code was changed. No implementation, commit, push, or deployment occurred for this research.
Next: review the draft and validate the native applied-state correlation before writing an implementation plan.
Preserve the unrelated local-profile Knowledge note and its pre-existing Home link.
The Home diff now also contains this research session link. Keep the two changes distinct when staging.
