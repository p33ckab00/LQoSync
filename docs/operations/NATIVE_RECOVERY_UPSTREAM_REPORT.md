# Internal report: native policy acceptance and recovery investigation

Status: project reference. Repository commit and push are authorized.
Do not submit this report as an upstream issue or send it to maintainers.
Owner decision: keep this information as project documentation while LibreQoS v3 beta changes continue.
Report date: 2026-11-16 (Asia/Shanghai, UTC+08:00).
Evidence period: 2026-11-11 through the read-only review on 2026-11-16.

The filename keeps its original name for established links. This document is no longer an outbound report draft.

## 1. Purpose and intended use

This report explains why LQoSync3 investigated native policy acceptance and possible recovery paths.
It records the requirements, work, tests, results, and limits behind the recovery questions.
Use it to compare future LibreQoS beta updates with the observed behavior.
It also preserves the conditions required before future recovery work.

This report does not establish a confirmed private dataplane defect or a supported live recovery procedure.
It records an observed update hold and the evidence that remains insufficient.
All live findings below describe their recorded samples. This documentation update does not refresh the live state.

## 2. Project background

LQoSync3 collects subscriber information from configured MikroTik routers during the normal sync cycle.
It prepares the subscriber and topology inputs that LibreQoS consumes.
For the managed v3 file-pair target, these inputs are `ShapedDevices.csv` and `network.json`.
LibreQoS generates the shaping transaction and applies policy through its control plane and DPDK bridge.
LQoSync3 also presents telemetry and system health through the TUI and Telegram reports.

The owner requested a health report after each changed managed-write operation.
The selected behavior sends WRITTEN/PENDING first, then a terminal outcome after verification.
This requirement created the need to distinguish a completed file write from verified native policy acceptance.
A successful import alone cannot prove which policy the bridge actually applied.

The related telemetry work collects traffic, mappings, worker health, subscriber statistics, circuit queues, and NIC diagnostics.
Correct subscriber and circuit identities matter because a valid traffic value can still belong to the wrong client.
The explicit-slot reader work and the native acceptance investigation address different parts of this requirement.

## 3. Requirements and acceptance criteria

| ID | Requirement | Required evidence or behavior |
| --- | --- | --- |
| R1 | Report each changed managed-write operation accurately | Retain a write ID, input hashes, and each file's outcome. Unchanged cycles create no new write report. |
| R2 | Distinguish write, import, and native acceptance | WRITTEN/PENDING confirms file replacement. Import confirmation cannot produce APPLIED. |
| R3 | Correlate acceptance with the current operation | Compare current inputs, imported data, runtime state, generated policy, and native evidence. Reject evidence from another operation. |
| R4 | Prove the complete subscriber policy | Require matching generations and exact policy operations. Counts alone are insufficient. |
| R5 | Associate telemetry with the correct circuit | Use explicit native slot IDs when present. Reject malformed or changed mappings. |
| R6 | Keep health states independent | Service activity, telemetry freshness, native acceptance, and allocation capacity require separate evidence. |
| R7 | Keep verification bounded | Use the existing deadlines, size limits, cached preparation, and pending-only polling. Do not recollect RouterOS data for Telegram delivery. |
| R8 | Preserve a recoverable live system | Confirm the exact recovery capability, expected interruption, backup, rollback, and acceptance checks before a mutation. |
| R9 | Keep the investigation private | Retain metadata-only summaries. Exclude subscriber identities, credentials, message bodies, and packet payloads. |

The verification deadline remains forty minutes. A timeout means acceptance is unverified, not that traffic has stopped.
APPLIED describes verified subscriber policy. Allocation capacity has a separate health component.
See [Unified health requirements and implementation](UNIFIED_HEALTH_TASK1.md).

## 4. Why the investigation expanded

The first investigation found 64 native allocation slots versus 1000 generated allocation slots.
At that stage, the subscriber policy could pass acceptance despite the separate allocation difference.
This showed why one green status cannot represent every health domain.
The owner deferred capacity repair while the beta implementation remained uncertain.

A later LibreQoS update exposed explicit `slot_numbers` in native applied-state reports.
The earlier reader derived IDs from array positions. That assumption can misidentify sparse native slots.
We implemented and tested explicit-ID support locally before any deployment.

The host then rebooted outside agent commands at 2026-11-16T09:39:36+08:00.
After that reboot, current import evidence no longer produced a native acceptance proof.
The investigation then moved from telemetry compatibility to native policy comparison, then to possible recovery mechanisms.
The new policy mismatch was separate from the earlier capacity mismatch.

## 5. Environment at diagnosis

| Item | Recorded value |
| --- | --- |
| Project and branch | `/home/pi/LQoSync3.0`, `lqosync3` |
| Installed LQoSync3 release | `3.0.182+ge8da230` |
| LibreQoS control package | `3.0.7.202611141944` |
| LibreQoS dataplane package | `3.0.7.202611141944` |
| LibreQoS preflight package | `3.0.7.202611141944` |
| Bridge build revision | `193d067a6ad9` |
| Public source reference | `9ebe29bc9d49013aff9b0544de3c6f6fc6ba2f1d`, beta19 projection, 2026-11-08 |
| Native socket | `/dev/shm/libreqos-analytics.sock`, root-only |
| Readable broker snapshot | `/run/libreqos/seam_state.json` |
| Local task commits | `700a721` explicit IDs, `d0a6d67` acceptance regression, `60707fd` recovery documentation |

Installed control and bridge hashes match their extracted cached package binaries.
This excludes a local binary difference against those cached packages. It does not prove protocol compatibility.
The public projection excludes the proprietary dataplane. Its source is not an exact description of every installed binary path.
The local explicit-ID change was not deployed or verified through the root-only production socket during these tasks.

## 6. Work performed and tests

### 6.1 Earlier unified-health verification

Earlier read-only native acceptance checks proved that the policy could match while allocation capacity differed.
Two-stage Telegram delivery also passed its earlier receipt verification.
These results establish historical behavior. They do not prove acceptance after the later reboot.

### 6.2 Explicit native slot support

We changed the shared native reader to associate each decoded slot ID with the circuit name at the same wire position.
Subscriber metrics and selected circuit queue requests use that association.
Only an omitted `slot_numbers` field selects the legacy one-based fallback.
A present invalid field cannot silently select that fallback.

Tests covered sparse IDs, reordered equivalent pairs, generation changes, and changed ID-to-name associations.
They also covered null, malformed CBOR, trailing data, duplicate IDs, zero IDs, and count mismatches.
Boundary tests covered 65536 mappings and a large unsigned ID without ID-based array allocation.
A queue regression proved that the old reader requested slot 1 where the fixture required slot 5.

The task added no polling cycle, source request, dependency, configuration key, or rendering step.
Map construction takes O(n log n), storage takes O(n), and lookup takes O(log n).
The on-demand circuit-name search remains O(n).
These are code-path properties, not measured production speed improvements.

### 6.3 Post-reboot native comparison

The operator ran a bounded read-only probe because the native socket requires root access.
The probe took three stable samples and ran the current-policy verifier.
It compared file/import identity, runtime readiness, schemas, generations, and policy operation counts.
It emitted metadata and fixed error categories rather than subscriber inventories.

We reviewed the acceptance code instead of weakening the failed check.
The pending-generation path returns before heavier policy and producer validation.
The monitor retains its two-second interval, skipped missed ticks, and cached input preparation.
No demonstrated LQoSync3 runtime defect justified a production patch for this incident.

### 6.4 Incomplete-policy regression

We added `boot_restored_partial_world_cannot_accept_a_newer_import` with synthetic data.
It models a 28-entry generated world and a 27-entry native boot world.
It rejects the older native generation.
It still rejects incomplete policy after the fixture gives both worlds the same generation.
It accepts only the complete matching policy.

### 6.5 Recovery capability review

We inspected redacted journal findings, three fresh broker snapshots, installed binary markers, and package hashes.
We compared those findings with the public broker and maintenance source.
We also ran the installed bridge's read-only `--check-boot-inputs` check without root.
It exited zero. This proves input readiness only, not a successful restart or the next selected policy.

| Check | Recorded result | Limit |
| --- | --- | --- |
| Explicit-ID full suite | 704 passed, 0 failed, 9 ignored across 19 suites | Local test result, not deployed behavior |
| ACK regression full suite | 705 passed, 0 failed, 9 ignored across 19 suites | Synthetic policy regression plus existing suites |
| Static checks | Final Clippy, formatting, and diff checks passed | No production load benchmark |
| Native comparison | Three stable samples, acceptance unproven | Read-only observation, no recovery attempted |
| Recovery audit | Snapshot freshness, sanitization, package hashes, boot readiness passed | No live recovery or rollback proof |

The 704-test and 705-test results are successive suite states. Do not add them as independent test counts.
No product tests were rerun for this documentation expansion.

## 7. Recorded findings

### 7.1 Current inputs and native policy differed

| Evidence | Generated/runtime side | Native side |
| --- | --- | --- |
| Input/import comparison | Current file pair matched the import | Not a native acceptance proof |
| Runtime readiness | Ready | Separate from policy acceptance |
| Transaction schema | 6 | 6 |
| Inner operations | 4 | 4 |
| Leaf operations | 28 | 27 |
| Mapping operations | 28 | 27 |
| Generation comparison | Runtime matched generated transaction | Native differed from runtime |
| Allocation capacity | 1000 | 64 |

The current-policy verifier returned `native generation still pending`.
The shared apply observation remained `IMPORT_CONFIRMED` with `import_confirmed_ack_unproven`.
The native report included a content key but no desired-world content key.
The absent desired key alone was not the failed acceptance condition.

The count difference does not prove that exactly one named subscriber lacked shaping.
Operation identities and policy contents can differ beyond their net count.
Neither this result nor the capacity difference proves a throughput limit or a traffic outage.
The value 64 denotes allocation slots, not WAN Mbps or a confirmed subscriber maximum.

### 7.2 Structural and maintenance holds appeared together

The bridge boot journal recorded a structural rebuild and a persisted boot world with 27 slots.
The broker then repeatedly logged `NON_SUFFIX_STRUCTURAL_REMOVE`.
The warning describes a removal set that is not a contiguous suffix of the dense slot table.
The broker permits the removable tail and fences the remaining structural change.

The later broker snapshots reported:

| Field or observation | Recorded value |
| --- | --- |
| Execution state | `healthy` |
| Snapshot age | About one second |
| In-flight transaction | None |
| Current reject | None |
| Non-suffix removals | 1 |
| Native slot-table length | 27 |
| Deferred maintenance class | `full-world bulk/reload` |
| Deferred operation count | 48 |
| Forced maintenance | `false` |
| Public recovery gate field | `pending_non_suffix` absent |

The journal also contained `MAINTENANCE_WINDOW_HOLD`.
A healthy execution label and no current reject do not prove convergence to the desired policy.
The structural fence and maintenance deferral require separate consideration.
Their full internal interaction on this installed build remains unproven.

### 7.3 Public and installed recovery capabilities were not correlated

The public beta19 broker contains a non-suffix R3/re-adopt path.
That path uses a separate gate and waits for a full-world report before admitting further incremental work.
The installed control binary contains the legacy warning and maintenance apply-now markers.
It lacks the sampled public R3 pending/complete strings and `non_suffix_rebuild_trigger` marker.
The sampled journal contained no R3 recovery event.

Absent strings and fields support a possible capability difference. They do not prove every private compiled branch is absent.
A later package timestamp does not establish correspondence with all public source paths.

### 7.4 Other boot observations remain separate

The journal recorded a Node Manager address-bind failure and a temporary DNS failure.
Their current state and causal relationship to the acceptance hold were not established.
Do not present them as confirmed causes of the native policy difference.

## 8. Why the recovery questions matter

These are internal research questions. They are not requests to send a report.

| Question | Why it arose | Requirement affected | Evidence needed before action |
| --- | --- | --- | --- |
| Which matched build supports non-suffix R3 recovery? | Public source exposes a path that installed observations do not establish. | R4, R8 | Build-specific capability evidence and matching control/dataplane versions |
| Can apply-now resolve this combined hold? | A full-world reload is maintenance-deferred while the structural fence also appears. | R4, R8 | Proof that the selected mechanism handles both conditions |
| What interruption can a full-world install cause? | Recovery can replace policy while the bridge carries live traffic. | R8 | Supported interruption expectations and a maintenance plan |
| How can an applied change be reversed? | The broker consumes a one-shot control request. Deleting it cannot undo completed work. | R8 | Verified restore steps for the exact build and state |
| What does broker `healthy` guarantee? | That label coexists with an unproven desired policy. | R2, R6 | Separate execution, convergence, and freshness contracts |

The objective is to restore verifiable convergence while it rejects stale or incomplete policy.
It is not to make a warning disappear or force an APPLIED label.

## 9. Public research and unresolved limits

The public main branch remained at the beta19 projection during the research.
The queried GitHub release list supplied v2 releases, not a v3 recovery-build mapping.
Issue searches for the non-suffix warning and maintenance with 3.0 returned no matching report.
Those searches do not cover private support channels or prove that development has stopped.

The official v3 installation guide documents generic beta package backup and rollback.
It requires the configuration, LKG, and all three old package artifacts before an upgrade.
That procedure does not establish recovery from this exact combined hold.
The v2 migration rollback tool addresses a different transition.

Direct requests for beta `InRelease` and uncompressed `Packages` returned HTTP 403.
The local APT list dated from 2026-11-15. It could not establish the current repository candidate.
No current stable-v3 availability claim follows from these limited checks.

## 10. Decisions, exclusions, and future acceptance

The owner chose to keep this report as project documentation while beta changes continue.
The owner subsequently authorized its commit and push to the LQoSync repository.
The current instruction prohibits upstream issue submission and messages to maintainers.
Treat the recovery mechanism as unresolved. Do not treat the suspected capability difference as a confirmed private implementation bug.

The investigation did not install packages, restart services, delete LKG, rewrite shaping inputs, or publish maintenance controls.
It did not send Telegram messages or capture packets.
The earlier RAM-cache deployment is historical context, not recovery work in this investigation.
No recovery backup, automatic rollback, or successful recovery is claimed.

Future review can resume when a relevant update or supported procedure becomes available.
Before live execution, require a private checkpoint, exact package artifacts, an interruption plan, and a verified rollback procedure.
After execution, require stable current-input, import, generation, exact-policy, and producer validation evidence.
Also verify traffic, telemetry identity, queue/NIC health, and service stability.
Keep the capacity difference separate if policy acceptance succeeds.

Independent LQoSync3 work can continue with these acceptance conditions intact.

## 11. Evidence and related documents

Local evidence directories remain outside Git:

- `/home/pi/.local/state/lqosync3/unified-health-task1/`: earlier native acceptance evidence.
- `/home/pi/.local/state/lqosync3/capacity-trace/`: the separate allocation investigation.
- `/home/pi/.local/state/lqosync3/explicit-slots-task1/`: explicit-ID test and review evidence.
- `/home/pi/.local/state/lqosync3/post-reboot-ack-task2/`: native metadata, redacted journal findings, and test summary.
- `/home/pi/.local/state/lqosync3/recovery-path-review/`: read-only audit and metadata.

Private captures can contain native subscriber mappings. They are not part of this report.
Use counts, fixed reason codes, and redacted observations for later comparisons.

Project references:

- [Unified health and managed-write reports](UNIFIED_HEALTH_TASK1.md).
- [Explicit native slot mapping](EXPLICIT_SLOT_MAPPING.md).
- [Post-reboot native ACK diagnosis](POST_REBOOT_NATIVE_ACK_DIAGNOSIS.md).
- [Conditional recovery review and rollback boundary](NATIVE_RECOVERY_PATH_REVIEW.md).

Public references, pinned to the reviewed source:

- [Beta19 source projection](https://github.com/LibreQoE/LibreQoS/commit/9ebe29bc9d49013aff9b0544de3c6f6fc6ba2f1d).
- [Broker structural guard and recovery path](https://github.com/LibreQoE/LibreQoS/blob/9ebe29bc9d49013aff9b0544de3c6f6fc6ba2f1d/src/rust/lqosd/src/bridge_broker/mod.rs).
- [Maintenance classification](https://github.com/LibreQoE/LibreQoS/blob/9ebe29bc9d49013aff9b0544de3c6f6fc6ba2f1d/src/rust/lqosd/src/bridge_broker/maintenance.rs).
- [V3 install, beta upgrade, and rollback guide](https://github.com/LibreQoE/LibreQoS/blob/9ebe29bc9d49013aff9b0544de3c6f6fc6ba2f1d/docs/v3/install.md).
