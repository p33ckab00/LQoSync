---
type: spec
created: 2026-11-19
status: draft
related: ["[[topology-manager-status]]", "[[libreqos-v3-ingress-facts]]", "[[libreqos-v3-mlx5-compat]]"]
---

# Planner admission and live DPDK acceptance design (Phase 3)

## Intent

Before Phase 4 activation, LQoSync must prove that LibreQoS accepts the Topology Manager pair (`ShapedDevices.csv`, `network.json`) and applies it correctly: the shaping planner does not transform it in an unexpected way, and the DPDK dataplane applies the expected circuit and node limits.
The owner has no separate LibreQoS box, so the live proof runs on production, in a short window, with automatic restoration of the legacy pair.

## Owner decisions (2026-11-19)

- The live test runs on the production LibreQoS (v3, DPDK bridge on mlx5).
- Claude stops `lqosync3.service` for the test window and starts it again when the test ends (also after a failure). Claude asks for a "go" right before it stops the service.
- The production enrollment runs in the same window, before the live test.
- LQoSync stays read-only on MikroTik routers. It only orchestrates files that LibreQoS reads.
- Approach A (below).

## Facts that shape the design

- `lqosd` watches the pair with inotify, but the import waits for the scheduler job unless LQoSync writes `state/scheduler/reload_request.json` (`files::request_v3_import`), as the cycle and `publish_compiled` do. The test writes that request after each pair write. Forwarding does not stop; while the manager pair is applied, its policy shapes the subscribers.
- The pair is two files written JSON first, then CSV. The write is not atomic as a pair.
- The running legacy loop writes the legacy pair at each cycle and holds the instance lock. The service must be stopped for the test.
- Manager publication (`Runner::publish_topology`) claims durable writer ownership (`topology-manager/runtime-owner.json`). After that claim, the legacy loop of the new build refuses to write. The test must not claim it.
- Production runs an older build. The test uses the new binary as a one-shot command; this is not a deployment.
- Fresh LibreQoS read-back artifacts (read on 2026-11-19): `state/topology/topology_import.json`, `topology_runtime_status.json` (`ready`, `source_generation`, `shaping_generation`, `effective_generation`), `state/shaping/topology_compiled_shaping.json`, `shaping_inputs.json` (rewritten only when content changes), and `dataplane_transaction.json` (the planned transaction). `planner_state.json`, `queuingStructure.json`, `lastGoodConfig.*`, `ShapedDevices.lastLoaded.csv`, and `bakery_qdisc_handles.json` are stale HTB-path files from 2026-11-02; the test copies them to the backup only.
- The native acknowledgement is the lqosd IPC read that `apply_evidence::read_applied` and `verify` already use for receipts.
- Observed LibreQoS transformations in the live `dataplane_transaction.json` (2026-11-19):
  - A node without a `*BandwidthMbpsMin` key gets commit = floor(sum of its child circuit minimums), for example ceiling 100000 kbps with commit 20000 for 4 circuits at 5 Mbps. It does not get commit = ceiling.
  - A circuit minimum is rounded up to whole Mbps (3.5 Mbps becomes `commit_kbps` 4000).
  - A node ceiling is rounded up to whole Mbps (225.75 becomes 226000 kbps).
  - A circuit ceiling above its parent ceiling is clamped to the parent ceiling (200 Mbps below a 100 Mbps node becomes 100000 kbps).

## Goals

- Offline planner admission: before any live write, the manager pair passes the controlled compiler admission. A model of the observed LibreQoS rules (Facts) gives each node its commitment: the explicit minimum when set, else floor(sum of child commitments). The sum of the sibling commitments must not exceed the parent ceiling. A pair that does not fit is refused with the node names (designer call from the live evidence).
- Parity gate: before the live write, the manager pair must pass the existing legacy shadow parity check (circuit set, parents, rates) against the live legacy pair. A fresh enrollment gives exact parity, so a difference means a defect, and the test stops without a live write (designer call).
- Live acceptance: after LibreQoS imports the manager pair, the planner and dataplane artifacts show every expected circuit and node with the expected limits, and no unexpected circuit or node.
- A baseline read-back of the legacy pair before the test, and a read-back after the restore, so the report shows the differences between the two pairs and proves the restore.
- The production state is the same after the window as before, except the enrollment registry.

## Non-goals

- Phase 4 activation, writer ownership changes, and a deployment of the new build as the service.
- Router writes.
- Traffic or throughput measurement (the proof reads LibreQoS artifacts, not packets).

## Anti-goals

- A test that leaves the manager pair applied, `runtime-owner.json` written, or the service stopped.
- A test that reports success from file writes alone, without the LibreQoS read-back.

## Approaches

| Approach | Summary | Trade-off |
|---|---|---|
| A. Separate test command on the publication code, without an ownership claim (chosen) | `lqosync3 topology live-test` prepares, compiles, and checks the manager pair with the publication code, backs up the legacy pair, writes the manager pair, waits for import, reads back, restores the legacy pair, and writes a report | One more command, mostly reuse. The legacy loop state stays safe after the test |
| B. Trial flag on the activation path | The real Phase 4 publication with an automatic revert and owner-file removal | Tests the exact activation code, but more state can stay wrong after an interruption (owner file, receipts) |
| C. Manual runbook | Copy files by hand and look at the LibreQoS UI | No code, but no automatic read-back or restore |

## Command

`lqosync3 topology live-test --report <path> [--import-timeout <s>]` (default 120 s), and `lqosync3 topology live-test-restore --backup <dir>` for a manual restore after an interruption.

Steps of `live-test`, each with a time limit; any failure goes to the restore step:

1. Take the instance lock (fails while the service runs). Require an enrolled registry, the apply mode, and the v3 pair target.
2. Prepare the manager pair from a fresh source read (`Coordinator::prepare`), compile it with the controlled compiler, and run the compiler admission and the planner admission check (Goals). Stop with the reasons if one fails; nothing is written.
3. Copy the current legacy pair and the LibreQoS read-back artifacts to a backup directory under the state directory (`topology-manager/live-test/<unix time>/`).
4. Baseline read-back of the legacy pair (step 6 parser).
5. Write the manager pair with the existing pair writer (temporary file, sync, rename, JSON first), without `runtime::claim` and without a registry commit. Wait until `topology_import.json` matches the manager pair and `topology_runtime_status.json` is `ready` with a new `source_generation`, or the import timeout ends.
6. Read back: parse `topology_compiled_shaping.json`, `shaping_inputs.json`, and `dataplane_transaction.json`, and compare each circuit (ceiling, commitment, parent node) and each node (ceiling, commitment) with the values that the LibreQoS rule model (Facts) gives for the compiled pair. Only those modeled transformations are accepted; a clamp of a circuit ceiling to its parent is accepted and listed as a warning. Unexpected and missing entries are differences. Then read the native acknowledgement with `apply_evidence::read_applied` and `verify`; it must accept the manager pair (designer call).
7. Restore: write the backup legacy pair, wait for its import and `ready`, and read back again; the result must match the baseline.
8. Write the report (JSON plus a short text summary): the step results, the differences of the manager read-back, the legacy-to-manager differences, the restore result, and the timings.

Write receipts: the test does not use the write receipt tracker, and it sends no notification (designer call). The production service is an older build whose receipt schema differs; a new receipt could break its restart. The restore writes the backup legacy pair byte for byte, so the files equal the legacy service's own last write and its receipt stays valid. The test checks the byte equality after the restore.
`live-test-restore` writes the backup legacy pair and waits for its import, for the case where the command itself was interrupted.

## Window runbook (Claude runs it after the owner says "go")

1. Read-only pre-checks: `lqosync3.service` active, LibreQoS `ready`, free memory, and a tar backup of `/var/lib/lqosync3/` and of the two source files.
2. Stop `lqosync3.service`.
3. Enrollment with the new binary: policy propose, check, enrollment propose, show the mapping summary to the owner, and enroll after the owner confirms. A failed check ends the window (go to step 6).
4. Run `live-test`.
5. If the command did not complete its restore, run `live-test-restore`.
6. Start `lqosync3.service` and confirm one good legacy cycle (status, pair written, LibreQoS `ready`).
7. Report the result to the owner. The enrollment registry stays for Phase 4; the tar backup allows its removal.

## Verification before the window

- Unit tests: planner admission refuses an implicit commitment above its parent and names the node; the read-back parser handles the current artifact formats (fixtures copied from production, read-only) and finds missing, unexpected, and changed entries; rounding is allowed only within whole-Mbps export.
- Command tests with a fake LibreQoS state directory and a fake native acknowledgement: the full sequence passes; a parity difference writes nothing; an import timeout restores the legacy pair; a read-back difference restores the legacy pair and reports it; `runtime-owner.json` is never written; the registry revision does not change; the write receipt files do not change; the restored pair is byte-identical to the backup.
- Rule-model tests from fixtures copied (read-only) from the live artifacts: implicit node commitment, circuit minimum rounding, node ceiling rounding, and the parent clamp.
- Full suite, format, Clippy with warnings denied, and whitespace checks.

## Left to the builder

- The exact parsers of the LibreQoS artifacts, after a read-only look at the production files.
- The report layout.
- The time limit of each step except the import timeout.
