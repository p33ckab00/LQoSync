---
type: session
created: 2026-11-12
tags: [dpdk, health, diagnosis]
status: investigated
related: ["[[2026-11-12-unified-health-deployment]]", "[[HANDOFF]]"]
---

# Native circuit capacity diagnosis

Observed: 2026-11-12T08:44:12+08:00 (Asia/Shanghai). Agent: Codex `/root`.
The owner approved read-only diagnosis of the native/generated capacity difference.

## Evidence

- Worktree: `/home/pi/LQoSync3.0`, branch `lqosync3`, HEAD `d569bf8`.
- Installed LibreQoS and dataplane packages: `3.0.7.202611102128`.
- The bridge process is PID 630703. It started on November 11 at 08:09:05, before this LQoSync deployment.
- The existing root verification reports 64 applied slots and 1000 desired slots. It verifies the subscriber policy separately.
- Evidence file: `/home/pi/.local/state/lqosync3/health-12e3d57/deployed-policy.json`.
- The current transaction still requests 1000 slots and contains 26 leaf operations.
- Its modification time is 08:39:11 on November 12. Its shaping generation matches the deployment proof.
- The runtime source generation changed. Equal shaping generations do not imply equal source generations.
- The actual units `libreqos-bridge.service` and `libreqos-lqosd.service` are active. Short aliases are inactive.
- The current LQoSync cycle reports no file change or error. Shared health has no tracked update or acceptance.

The capacity result above comes from the existing root verification, not a new native socket sample.
No private subscriber data appears in this note.

## Source findings

Reviewed public LibreQoS source at commit `9ebe29bc9d49013aff9b0544de3c6f6fc6ba2f1d`.
This source revision is not a confirmed build revision for the installed bridge.

1. The emitter derives capacity as the maximum of the circuit count rounded to a power of two, 1000, and carried capacity.
2. The broker compares policy content keys before it emits a change. It has no capacity comparison at that decision.
3. The broker projection sets `circuit_capacity: None`.
4. The incremental delta also sets `circuit_capacity: None`. An empty policy delta produces no transaction.

These paths explain how a desired capacity can differ while the broker considers the policy applied.
They do not prove the origin of the native value 64.
The public repository does not expose the bridge implementation at the referenced path.
A bridge source/build trace is necessary to establish its initial allocation and resize rules.
Journal inspection supplied no explicit circuit-capacity initialization event.

## Assessment

The evidence supports an allocation propagation gap as the leading hypothesis.
It does not establish an LQoSync CSV or rate-generation defect.
The previous native check verified the policy, but it did not verify allocation equality.
The 26 leaf operations alone do not prove allocator headroom or a safe subscriber maximum.
Do not treat 64 as Mbps or as a confirmed maximum subscriber count.

Do not promise that a bridge restart or a no-change sync will correct the allocation.
Do not add the removed legacy `circuit_capacity` setting to `bridge.conf`.
The next technical step is to verify the installed bridge build and its cold-load/resize contract with upstream.
Any repair needs a separate change plan, a maintenance decision, and rollback verification.

## Scope and checks

No product code, configuration, managed output, or service changed.
No restart, deployment, commit, push, or Telegram send occurred.
Checks comprised existing proof inspection, current artifact inspection, package and service metadata, journal inspection, and upstream source review.
No code tests were necessary for this diagnosis. This session note and the handoff are the only new task changes.
The unrelated profile/WAN note and Home link remain intact.

## Sources

- [Capacity derivation](https://github.com/LibreQoE/LibreQoS/blob/9ebe29bc9d49013aff9b0544de3c6f6fc6ba2f1d/src/rust/lqos_topology/src/circuit_capacity.rs#L43).
- [Broker policy comparison](https://github.com/LibreQoE/LibreQoS/blob/9ebe29bc9d49013aff9b0544de3c6f6fc6ba2f1d/src/rust/lqosd/src/bridge_broker/mod.rs#L2382).
- [Projection capacity](https://github.com/LibreQoE/LibreQoS/blob/9ebe29bc9d49013aff9b0544de3c6f6fc6ba2f1d/src/rust/lqosd/src/bridge_broker/translate.rs#L844).
- [Incremental capacity](https://github.com/LibreQoE/LibreQoS/blob/9ebe29bc9d49013aff9b0544de3c6f6fc6ba2f1d/src/rust/lqosd/src/bridge_broker/translate.rs#L1441).

## Installed bridge trace

Trace time: 2026-11-12T14:28:03+08:00 (Asia/Shanghai).
The bridge reports build revision `f1cc998a013e` through its documented `--print-build-rev` command.
The ELF build ID is `7d2a946d53a6712aa9fc2315b2cb31b7dbbd4e98`.
The installed binary SHA256 is `90af73922af1599af586f87e55b9756a72042d1dbf2cea659955471d01816578`.
The package describes the bridge as proprietary and does not supply its source.
The public repository does not resolve this build revision.

The startup journal supplies stronger evidence than the previous generic source review:

- At November 11, 08:09:06, the bridge selected autonomous LKG boot.
- It restored a world with 26 leaves and four inner nodes.
- The placement section required reconstruction because its revision differed.
- The bridge then saved the boot world for the next restart.

Static inspection of the installed executable identifies the allocation selection in `bridge_launch::run_bridge`.
One branch computes a power-of-two bound from a boot object and a carried bound.
A separate branch has the 1000-slot fallback.
The selected value initializes `carried::CARRIED_CAPACITY` at address `0x3b0748`.
The direct symbol references show one initialization store and later reads.
These observations support a startup-derived bound. They do not prove all resize behavior through indirect references.
A doubled count of 26 rounds to 64, but the exact boot-object field still needs confirmation.

Evidence and the read-only root probe are in `/home/pi/.local/state/lqosync3/capacity-trace/`.
The probe reports only allocation metadata, counts, and binary hashes.
It reads the root-only LKG and uses the established native policy verification command.
The operator terminal step remains pending because host sudo requires a password.
Do not conclude that the LKG contains 64 until its metadata result arrives.
No service or product file changed.

## Live metadata verification and result

Verified at 2026-11-12T14:31:45+08:00 (Asia/Shanghai). The owner ran the read-only root probe.
Its result is `capacity-trace/live-metadata.json` under the private evidence directory.

- Native policy verification passed. The probe wrote no managed file.
- Native capacity remains 64. Desired capacity remains 1000.
- Running and installed executable hashes match the SHA256 above.
- The LKG has 26 leaf operations and schema 1.
- The recursive probe found no integer field named `capacity`, `circuit_capacity`, `applied_circuit_capacity`, or `desired_circuit_capacity` in that LKG.

This corrects the earlier suggestion that the LKG explicitly stores 64.
The probe found no such field. The evidence instead supports capacity derivation from restored placement.

The installed code has a placement-bundle branch before its transaction/fallback branch.
At `0x3b0219`, it reads the count at offset `0x20` from the bundle's Arc allocation.
The bundle clone copies a vector of 16-bit entries. Its length occupies this offset after the Arc header.
The startup branch takes the maximum of twice that count, count plus one, and an optional carried bound.
It rounds that result to a power of two, then initializes the shared capacity.
For a 26-entry placement, this calculation gives 64.
The 1000-slot floor appears in the separate fallback branch, after the placement branch.
Thus the installed binary can select a smaller placement-derived bound despite the producer's 1000-slot target.

The journal confirms placement reconstruction at startup and a 26-slot boot world.
The formula, that boot evidence, and the live value support this as the cause of the observed mismatch.
The private bridge source and a controlled restart test remain unavailable as independent confirmation of the exact branch.
No controlled restart test was attempted on the live node.

The state-report function reads the initialized shared capacity at `0x21a8ab`.
The R3 rebuild functions also read it at `0x228439` and `0x266595`.
The static reference inventory shows one direct store, in the startup function.
It supplies no evidence that an ordinary incremental policy update replaces this shared bound.
The public broker source also omits capacity from its incremental transaction.

## Recommended repair boundary

Treat this as a LibreQoS bootstrap/allocation contract issue for upstream review.
Keep the emitter's desired capacity visible beside the native capacity.
A candidate upstream correction must reconcile the placement-derived bound with desired capacity before it initializes the world.
It must also define capacity-only changes after boot, including effects on worker arrays and state reports.
Tests must cover restored placement, explicit desired capacity, capacity-only updates, and preserved policy.

An ordinary restart can repeat the LKG boot path. It is not a verified repair.
Deleting or modifying the LKG is not part of this diagnosis.
The trace is complete to the supported mechanism and upstream repair boundary.
No production change is necessary to preserve the policy verified by this probe.

Static excerpts and the complete reference inventory are `capacity-path.disassembly` and `capacity-references.json` in the private evidence directory.
