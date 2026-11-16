---
type: session
created: 2026-11-17
tags: [topology, source, rate, mapping, reliability, performance, ste]
status: source-policy-complete
related: ["[[2026-11-17-topology-identity]]", "[[HANDOFF]]"]
---

# Source policy mapping and exact rates

The owner directs the next source-scope and exact source-rate integration task.
Worktree: `/home/pi/LQoSync3.0`. Branch: `lqosync3`. Base: `489acc7` plus local normalization/identity.

Agent: Codex `/root`. Timestamp: 2026-11-17T03:18:43+08:00, Asia/Shanghai.
The new slices remain local and uncommitted. No push, deployment, live configuration, router write, or service restart occurred.
Unrelated operational work remains intact.

## Changes

Added `source_rate.rs`, `source_rate_tests.rs`, `source_policy.rs`, and `source_policy_tests.rs` under `src/topology_manager/`.
Extended identity source selections with explicit source policy and `service_names` selectors.
Extended current normalization with service inventory, fresh metadata access, and active Hotspot server checks.
Updated reader roles/tests to collect `/ip/hotspot` inventory before/after active tables, with two additional read-only commands for Hotspot.
Updated identity tests, the public catalog integration test, module exports, design, discussion, plan, Home, and handoff.

Current service/profile indexes are built once per router and keyed by protocol/name.
DHCP/Hotspot service names resolve to current server locators. Unknown selected services produce a configuration conflict.
PPP selectors use documented transport names. An empty selection includes the whole router/protocol.
Service/profile names are operator selection rules, not immutable object or subscriber identities.

Profile bindings map current names to explicit stable source profile references.
Recreated locators use fresh current data. A rename needs a revised selector instead of retained locator repair.
Selected catalog profile filters require matching configured bindings.
Active unmapped profiles and invalid enabled rates withhold the collection.

Rate reads are explicit per-profile Disabled/RateLimit modes, with an unprofiled mode for direct lease rates.
Each mapped profile compiles once per selected source. Its active accounts reuse the compiled result.
Disabled mode supplies no numeric evidence. Missing text remains absent.
The resolver requires actual numeric evidence for SourceRead authority and does not invent a fallback.

The parser uses integer arithmetic and explicit direction/unitless settings.
It reads the sustained first token, supports symmetric single values, and preserves k/M/G case compatibility.
It rejects fractional bps, zero, malformed input, and overflow without raw values in errors.
It has no float conversion, rounding, clamp, floor, or guessed speed.
Burst/minimum fields do not replace Topology Manager subscriber policy controls.

## Review corrections

The service regression reproduced empty membership when a DHCP name was compared with a row locator.
The isolated API now uses `service_names`, and the current index resolves those names before endpoint selection.
The Hotspot regression showed that active membership needed current server inventory and disabled checks.
The reader/normalizer now require that inventory. No serialized settings or live callers used the earlier isolated field.

The missing-rate negative assertion first called aggregate evaluation after placement had already blocked the candidate.
That was an assertion error. The corrected check stops at the resolver and confirms its critical conflict and absent candidate.
Production resolver behavior already prevented the fallback.

## Checks

The initial parser/policy red run had 16 failures and 1 pass. All 17 pure checks now pass.
The mapped-authorization, DHCP selector, and Hotspot inventory/member regressions failed before their fixes.
This slice adds 23 checks. All 256 topology checks pass.

The public restart/store test covers both explicit Local and SourceRead authority with a fresh profile locator and stable binding.
Both paths preserve current endpoints, saved pin, and a 50/10 Mbps subscriber maximum.
The missing SourceRead rate case blocks placement. No current endpoints or raw rates enter the durable registry.

Full suite: 973 passed, zero failed, 9 ignored. Format and Clippy with denied warnings passed.
Whitespace/stub scans passed. Dependency, durable schema, legacy parser/reader, runner, and cycle files are unchanged.
A preserved reversed 10,000-account/10,000-service case yields equivalent IDs and current output.
A 2,000-value decimal case matches exact integer bps. These checks do not establish production throughput.
The STE checker is heuristic. Manual review does not establish certified compliance.

Plan: `docs/superpowers/plans/2026-11-17-topology-source-policy.md`.
Logs: `.superpowers/sdd/2026-11-17-topology-source-policy/`.
Design: `docs/design/TOPOLOGY_MANAGER_IMPLEMENTATION.md`.

## Limits and next action

All source/authority modes and selectors remain isolated API inputs until durable settings and UI integration exist.
Legacy comment/name/default speed priorities need explicit migration. There is no implicit import or manager-mode activation.
Read deadlines and generations still apply. Publication must keep/recheck the returned minimum deadline.
Native bounds, export precision, admission, and DPDK application proof remain later gates.
The existing operator-reference contract still requires uniqueness and non-reassignment.

Next: durable source/authority settings and migration/recovery.
Then connect publication/native integration, TUI, and schedules. Do not deploy this isolated path before its gates pass.
