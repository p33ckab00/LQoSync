---
type: spec
created: 2026-11-19
status: draft
related: ["[[topology-manager-status]]", "[[2026-11-19-topology-speed-policy]]"]
---

# Settings fields managed by the Topology Manager (Phase 2c-1)

## Intent

After enrollment, the registry holds node capacity, group factors, and the subscriber speed policy. The legacy Settings page still edits `[nodes]`, `[bandwidth_defaults]`, and `[backstop_caps]` in `config.toml`. The owner wants Settings to show which system uses these fields, so that an edit there does not look like it changes the Topology Manager.

## Owner decisions (2026-11-19)

- Approach A: a warning after enrollment, a lock after activation.
- Before enrollment: no change. The fields are editable, as today.
- Enrolled, not activated: the fields stay editable, because the legacy loop still writes shaping from them. Each field shows: "the legacy loop uses this now; after activation the Topology Manager owns it (Topology > Policies or Builder)".
- Activated (the manager is the writer): the fields are read-only and show: "managed by the Topology Manager: edit it in Topology > Policies or Builder".
- `config.toml` does not change. This is a TUI change only.

## Approaches

| Approach | Summary | Trade-off |
|---|---|---|
| A. Warning after enrollment, lock after activation (chosen) | Notes from enrollment, read-only after activation | The legacy shaping stays adjustable until activation. The lock only acts once Phase 4 can activate |
| B. Lock after enrollment | Read-only as soon as the registry is enrolled | Simpler, but the running legacy shaping cannot be adjusted from the TUI before activation |

Hiding the fields was dropped: the operator could not compare the two systems.

## Design

- Managed fields: every key under `nodes.`, `bandwidth_defaults.`, and `backstop_caps.`.
- State source: the service `status` operation adds `writer`: `"legacy"` or `"topology"`, from `topology-manager/runtime-owner.json` (absent file: `"legacy"`). `enrolled` exists already.
- The TUI asks for `status` when the Settings tab opens and passes `{enrolled, writer}` to the Settings view. Without an answer (no service or no socket), Settings works as today and shows no note (designer call).
- The existing `read_only` field meta carries the lock. The note goes in the field help, like the `runtime.state_dir` note.

## Verification

- Status test: `writer` is `"legacy"` without the owner file and `"topology"` with `{"writer":"topology"}`.
- Settings tests: no note before enrollment; the enrolled note and editable fields; read-only fields with the managed note when the writer is `"topology"`; fields outside the three sections are not affected; no status answer behaves as today.
- Full suite, format, Clippy with warnings denied, and whitespace checks.

## Left to the builder

- How the Settings view receives the state (a setter called by the app).
