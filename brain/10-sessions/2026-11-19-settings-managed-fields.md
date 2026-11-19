---
type: session
created: 2026-11-19
tags: [settings, tui, api, topology]
status: done
related: ["[[topology-manager-status]]", "[[2026-11-19-topology-speed-policy]]"]
---

# Settings managed fields (Phase 2c-1)

After enrollment the registry owns node capacity, group factors, and the speed policy. Settings now shows which system uses the matching `config.toml` fields.

## Results

- Plan: [2026-11-19-settings-managed-fields](../../docs/superpowers/plans/2026-11-19-settings-managed-fields.md). Three tasks on branch `settings-managed-fields`.
- `status` reports `writer`. Settings notes the managed fields when enrolled and locks them when the Topology Manager is the writer.
- No change to `config.toml`, the legacy loop, or the registry.

Related: [[topology-manager-status]].
