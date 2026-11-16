# Post-Reboot Native ACK Investigation Plan

**Goal:** Explain the native policy-proof failure and fix only a demonstrated LQoSync3 defect.

**Architecture:** Compare current inputs, imported topology, generated policy, and stable native reports through the existing read-only probe.
Preserve generation correlation, input hashes, exact policy checks, and capacity evidence.
Do not force shaping writes, restart the bridge, or change its last-known-good state.

**Tech Stack:** Existing Rust evidence verifier, Python metadata wrapper, local regression fixtures.

- [x] Read the verifier, monitor, receipt transitions, and current public runtime metadata.
- [x] Prepare a bounded root-only probe with fixed public error categories and no subscriber identifiers.
- [x] Inspect stable native comparisons and identify the failed verification stage.
- [x] Review the failure. No LQoSync3 runtime defect was demonstrated; add a guard regression for the observed partial world.
- [x] Preserve the correct verifier and document the upstream structural-removal hold.
- [x] Review performance, integrity, and reliability. Checks passed; commit the diagnosis and regression locally.

Root socket access requires an operator terminal because sudo needs a password.
The investigation must not weaken acceptance checks to obtain an APPLIED label.
