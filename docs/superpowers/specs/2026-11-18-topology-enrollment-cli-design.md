---
type: spec
created: 2026-11-18
status: draft
related: ["[[topology-manager-status]]", "[[2026-11-17-topology-enrollment]]"]
---

# Topology Manager enrollment CLI design (Phase 1)

## Intent

The owner wants a complete Topology Manager. The running legacy sync does not shape all subscribers that the owner needs, for example the IPoE clients on DHCP server `ipoevlan10`.
The owner will patch or update production after the Topology Manager is complete.

Phase 1 gives the operator a command-line path from the current legacy deployment to an enrolled topology registry.
The backend already has `Runner::prepare_topology` and `Runner::enroll_topology`. No operator interface uses them.

## Roadmap context

The owner approved this order. Each phase gets its own design and plan.

1. Enrollment CLI and identity rules (this document).
2. Topology Manager TUI tab, including the Settings migration for node capacity, factors, and subscriber policy.
3. Native planner admission proof and live DPDK acceptance proof.
4. Manager service activation code and an activation runbook.
5. Speed scheduler and schedule profiles (timezone default `Asia/Manila`).
6. Other migration paths (existing-source replacement, active-owner migration, schema-2 circuit/device-ID migration), only if production needs them.

## Constraints

- Do not touch the running `lqosync3.service`. Do not stop, restart, or deploy it. Do not write to routers. Do not write live shaping files.
- Do all work in code, tests, and scratch environments. The owner runs production steps in a maintenance window.
- The owner accepts a service stop in a maintenance window for the one-time enrollment.
- `enroll_topology` needs the instance lock. The running service holds that lock. The enrollment command must fail with a clear message when another process holds the lock.
- Do not add dependencies.
- Keep the existing gates: v3 pair apply target, receipt recovery, critical conflicts block work, unknown files stay.
- Enrollment installs policy only. The legacy service loop keeps its behavior until Phase 4 activation.

## Goals

1. Add identity rules that fit the owner's network without router changes.
2. Give the operator a read-only readiness report before enrollment.
3. Let the operator create and install the first topology policy (catalog and source settings).
4. Propose a reviewable mapping from the current legacy files to the fresh candidate.
5. Run enrollment with the reviewed mapping, and stop safely on any change.
6. Give the owner a maintenance-window runbook.

## Non-goals

- No TUI work in this phase.
- No service activation, no manager writer, no live shaping output.
- No `[lqosync-id:REF]` markers and no router writes. The owner rejected the marker workflow for this deployment.
- No speed scheduler.

## Anti-goals

These results are failures even if the code works:

- An enrollment that silently drops, merges, or re-parents a live subscriber.
- A command that changes the running service or the live LibreQoS files.
- An identity rule that gives one subscriber the pin or placement of another subscriber without a visible conflict.

## Identity rules (owner decision)

The owner chose name and MAC identity rules only. The owner asserts these rules for the deployment.
Each rule is opt-in for each `SourceSelection`. Add them as new `ReferenceAuthority` variants beside `Unconfigured` and `OperatorManagedComment`.

| Rule | Source | Evidence | Identity |
|---|---|---|---|
| PPP secret name | PPP | `AuthorityEvidence::LocalAccount.name` | The secret name, scoped to the router, protocol, and service |
| DHCP client MAC | DHCP, for each selected server such as `ipoevlan10` | `Endpoint.mac` of the bound lease | The normalized MAC, scoped to the router, protocol, and server |

Owner assertions recorded in this design:

- The operator does not reuse a PPP secret name for a different subscriber.
- Each IPoE client on a selected DHCP server is a fixed CPE or router with a stable MAC.

Required behavior:

- Two current accounts with the same name or MAC in one scope produce a critical `DuplicateReference` conflict.
- A DHCP lease without a MAC produces a critical conflict for that account.
- Each rule uses its own `account_ref` namespace, for example `ppp-name/v1/...` and `dhcp-mac/v1/...`. A value from one rule never matches a value from another rule.
- A renamed PPP secret becomes a new identity. The readiness report must show this case clearly when the registry already has records.
- Equal values on different routers or protocols stay different identities.

## Enrollment scope (owner decisions after review)

The backend adoption is a strict snapshot bridge. The adopted legacy rows and the fresh candidate must match exactly (`ownership.rs` parity check, `parity.rs` `AddedRow`).

1. **Exact current snapshot.** Enrollment adopts the current legacy rows only. Production has 28 rows on 2026-11-18. It has no `ipoevlan10` row.
   Enrollment does not add new subscribers. The `ipoevlan10` clients enter through normal manager collection after Phase 4 activation.
2. **Suggested mappings that the operator confirms.** Legacy IDs are random and candidate IDs use the `tm-sub-` and `tm-device-` forms, so exact ID matches do not occur.
   The proposer suggests a row mapping when the legacy Circuit Name equals the PPP secret name, or the legacy MAC equals the lease MAC.
   Each suggestion is marked unconfirmed in the mapping file. Enrollment uses only confirmed mappings.
   Node mappings use the same suggest-and-confirm rule. `policy propose` can copy legacy node IDs into the catalog when `ids.rs` allows them.
3. **Rows without a current session.**
   - Configured static rows become protected static rows.
   - Offline and grace rows (from `subscriber_grace`) get an explicit drop decision. The manifest needs a new dropped-rows category, because today an unclassified row blocks adoption (`migration.rs` `UnclassifiedRow`).
   - The enroll report lists each dropped row. A drop does not change live shaping, because the legacy loop keeps writing the live files until Phase 4.

## Builder calls (made by the designer, open to owner correction)

- The mapping file keys each decision on the identity `account_ref` and the legacy circuit/device IDs, not on candidate `tm-` IDs. `enroll` resolves candidate IDs from the fresh pair.
- When fresh data changes between `propose` and `enroll` (reconnect, new IP, new row), `enroll` stops with a report. The operator runs `propose` again. Confirmed decisions for unchanged identities carry over.
- `policy propose` runs before `check`. `check` evaluates the draft policy file in memory and needs no lock and no installed registry.
- A missing or invalid MAC on a selected DHCP scope gives a conflict for that account only. Change the shared normalizer so that unselected DHCP servers do not fail the router read (today `normalize.rs` fails the whole read).
- Production uses `runtime.mode = "apply"`, `libreqos.target = "auto"`, and `network.mode = "root_promotion"`. The builder must prove that a catalog can reproduce the `root_promotion` network exactly, before the CLI work. If it cannot, stop and report.
- The confirmation step must finish before `pair.deadline()`. If the deadline passes, prepare again.

## Speed sources (already supported)

The owner's speed chain already exists in `LegacyRateSource`:

- PPP: `AccountComment` (secret comment), then `ProfileComment`, then `ProfileName`.
- IPoE: `ServerComment` (DHCP server comment).

Phase 1 uses these sources unchanged.

## Approaches

| Approach | Summary | Trade-off |
|---|---|---|
| A. CLI with an auto-proposed mapping | New `lqosync3 topology ...` commands. Proposal matches exact IDs and lists all unmatched items | Smallest. Easy to test in scratch. Phase 2 TUI reuses the proposer. Needs a service stop for enrollment. JSON review is less friendly |
| B. TUI enrollment wizard | Preview, mapping, edit, and confirm in one TUI screen | Better review. Larger. Still needs a service stop. UI work comes before proof of the flow |
| C. Command channel to the running service | Unix socket. No service stop | Largest. Adds a new control surface that needs access control. Changes the running service |

The owner chose **A**.

## Commands

All commands use the existing `--config <path>` option. Exact flag names are left to the builder.

### `lqosync3 topology check`

Read-only readiness report.

- Read the configuration, the registry (if present), the live legacy pair, and current router data.
- Run source policy preflight, legacy preflight, and identity authorization with the configured rules.
- Show counts and each issue: missing MAC, duplicate name or MAC, unmatched legacy rows, policy conflicts, and source filter conflicts.
- Write nothing. Do not take the instance lock. The command can run while the service runs.
- Exit with a non-zero status when a critical issue exists.

### `lqosync3 topology policy propose` and `lqosync3 topology policy install`

- `propose` reads the configuration, the live legacy pair, and current router data. It takes no lock and writes only the draft file.
- `propose` writes a draft policy file. It contains the `catalog` and `source_settings` for the first registry revision.
- The draft mirrors the current legacy topology and configuration: routers, WAN and group nodes, rates, and the selected PPP and DHCP sources with their identity rules.
- The operator reviews and edits the file.
- `install` validates the file and commits it to the registry through the existing store. It takes the instance lock.
- An installed policy does not shape traffic. Shaping stays with the legacy loop.

### `lqosync3 topology propose`

- Prepare a fresh candidate with `prepare_topology`.
- Read the live legacy pair with the bounded reader in `enrollment.rs`.
- Write a mapping file. It contains row mappings, node mappings, protected rows, static roots, and static attachments.
- Suggest mappings with the rules in "Enrollment scope". Never map by IP. Never apply a suggestion without operator confirmation.
- List every unmatched legacy row and unmatched node. The operator decides each one: map, protect, or drop (drop is for legacy rows only).
- A candidate row with no legacy row blocks enrollment. Report it as a subscriber that enters after activation.
- Takes the instance lock.

### `lqosync3 topology enroll`

- Take the instance lock. Fail with a clear message if the service holds it.
- Prepare a fresh candidate again. Build the manifest from the fresh pair and the reviewed mapping file.
- Stop with a report if the fresh data differs from the reviewed mapping, for example a new or removed row. Do not guess a mapping.
- Show a summary of what enrollment installs, and require explicit operator confirmation.
- Call `enroll_topology`. Print the installed registry revision.

## Runbook (owner runs it in production)

Write `docs/operations/TOPOLOGY_ENROLLMENT.md`. It contains the steps in this order:

1. Back up the state directory and the live LibreQoS pair.
2. Run `topology policy propose` while the service runs. Review the draft policy file.
3. Run `topology check` with the draft policy while the service runs. Fix all critical issues.
4. Stop `lqosync3.service`.
5. Run `topology policy install`, then `topology propose`. Review the mapping file.
6. Run `topology enroll`.
7. Start `lqosync3.service`. Confirm that the legacy loop runs as before.
8. Rollback: the steps to restore the backed-up state directory.

## Verification

- Unit tests for each identity rule: unique, duplicate, missing MAC, rename, different routers, and rule namespaces.
- CLI tests for each command with fake connectors and temporary state directories.
- A parity test that a proposed catalog reproduces a `root_promotion` legacy network exactly.
- An end-to-end scratch test: policy propose, check, policy install, propose, and enroll against fixture router data that includes PPP rows, DHCP rows, an offline grace row, and a dynamic `ipoevlan10` lease that is not in the legacy files.
- Tests for the dropped-rows manifest category: a dropped row is reported, a dropped row cannot also be mapped or protected, and an unclassified row still blocks.
- A test that `enroll` fails cleanly while another process holds the instance lock.
- A test that `check` writes no file and takes no lock.
- Full suite, format, Clippy with warnings denied, and whitespace checks.

## Left to the builder

- Exact command and flag names, file formats (JSON or TOML), and report layout.
- Exact `account_ref` encoding for names with characters outside the reference character set.
- How `policy propose` maps legacy configuration fields to catalog fields.
- Whether a locally administered (random) MAC gives a warning in `check`. My call: a warning, not a conflict.
