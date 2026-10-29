# Subscriber Normalization and Source Selection

**Goal:** Bind discovered subscriber sources to shaping nodes and publish complete normalized local generations automatically.
**Architecture:** Reuse node router/source fields and indexed speed resolution. Normalize selected sources with stable account/device IDs.
Publish the snapshot and output pair as one immutable generation with one authoritative pointer. Keep a compatibility snapshot mirror.
**Stack:** Rust, Axum, Maud, TOML, existing RouterOS client.

## Constraints and decisions

- Change only LQoSync3.0. No production LibreQoS or router writes.
- Bind one exact PPP profile, DHCP server, or Hotspot profile per real shaping node.
- Reject duplicate source bindings. Unbound nodes remain available for topology design.
- Subscriber IDs use UUID v5 over router/service/account or canonical DHCP MAC. Exclude IP, profile and node.
- Multiple simultaneous IPs for one identity remain blocked until multi-device circuits are supported.
- Reject invalid IPv4/MAC, unknown mappings, unresolved selected rates, duplicate identities and IP conflicts.
- Hotspot supersedes selected DHCP only on the same router, IPv4 and canonical MAC. Do not guess other collisions.
- Require healthy required tables, existing selected metadata and fresh inventories for every selected router.
- Never treat a failed or partial read as subscriber deletion. Guard stale concurrent reads and settings changes.
- Keep empty selected collections blocked to prevent an accidental whole-network deletion.
- Auto-stage pause also pauses collection and normalized publication. On-demand inventory reads remain available.
- Poll only routers with selected sources. Complete read rounds precede automatic normalization.
- Keep legacy offline snapshot operation when no source binding exists. Stop republishing a managed generation after bindings are cleared.

## Work

- [x] Add failing tests for identity stability, selection, overlap, invalid/partial data and unchanged last valid state.
- [x] Implement selected-service speed previews, stable normalization and source validation.
- [x] Add immutable snapshot publication and authoritative generation reads, with a best-effort state snapshot mirror.
- [x] Connect normalization to source edits, inventory reads, policy edits and scheduled selected-router read rounds.
- [x] Add discovered source dropdowns and bounded normalized subscriber UI.
- [x] Optimize topology aggregation for large subscriber sets without changing capacity behavior.
- [x] Test HTTP token checks, concurrency and snapshot/generation integrity. Measure a large synthetic workload.
- [x] Complete independent performance/integrity review, checks, notes, commit and push.

## Review focus

Source selection must not silently omit unmapped selected users or reinterpret a failed scan as an empty scan.
Profile/server changes must preserve IDs. Cross-router identity names must remain separate.
Publication must switch one authoritative pointer only after snapshot and both generated files exist.
A compatibility mirror failure must not corrupt or hide the committed generation.
Read rounds and computation must not publish results after source settings or inventory epochs change.
CPU-heavy normalization and compilation must run outside async workers and the application lock.


## Verification result

59 functional tests passed. Strict Clippy, formatting, and diff checks passed.
Two manual benchmarks are excluded by default; the new normalization/compiler benchmark passed separately.
Final debug measurement: 50,000 subscribers and 100 sources, normalization 639.2 ms, normalization plus compilation 921.7 ms.
Source HTTP and complete scheduled local mock-router tests prove automatic local publication.
Independent review identified active DHCP fields, exact-name matching, unknown-source omission, queued-read pause, and UI lock contention.
Regression tests and implementation fixes resolved all findings. Repeat review found no remaining material issue.
No production LibreQoS or router changes occurred.
