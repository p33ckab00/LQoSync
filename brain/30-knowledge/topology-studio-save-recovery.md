---
type: knowledge
created: 2026-11-24
tags: [topology, frontend, studio, recovery]
related: ["[[2026-11-24-topology-studio-f3]]"]
---

# Topology Studio save recovery

Access configuration and source policy use separate write paths.
Save access configuration first. Wait for the loaded digest before policy save.
An unchanged digest cannot prove that a replacement secret reached the service.
If a secret write has an unknown outcome, block further writes until explicit recovery.
Never store access secrets in draft memory or browser storage.

Policy recovery retains the original actor, request key, and payload in application memory.
Read the operation before any retry. Recheck the live draft and router after that read.
A known saved operation must never trigger another write to get assigned policy IDs.

Adopt IDs only from unique saved values. Preserve local edits and stable local tokens.
Unrelated later entries must not prevent ID adoption.

Rebase uses the saved baseline, current local draft, and fresh cached state.
Preserve entries from other operators. Require explicit choices for changes to the same entries.
Do not combine different revisions from separate cached views.
The API omits explicit-profile usernames. Require confirmation after a configuration change.

A saved receipt proves persistence. Runtime acceptance needs fresh publication and observation evidence for the same revision.
Cached Load saved topology must not write policy, layout, router configuration, or output.

Implementation: `crates/web-ui/src/topology_studio_{api,model,save}.rs`.
Regression checks: `tools/frontend/studio-apply{,-edge,-review}-check.cjs`.
Real service integration remains I0 work.
