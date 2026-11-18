---
type: spec
created: 2026-11-18
status: draft
related: ["[[topology-manager-status]]", "[[2026-11-18-topology-tui-readonly]]"]
---

# Subscriber plans design (Phase 2b-2)

## Intent

IPoE clients have no speed control of their own. A DHCP server comment gives one rate to every lease on that server, and dynamic leases make per-lease comments impractical.
The owner wants LQoSync to own speed plans for DHCP/IPoE clients and attach a plan to each client. PPP subscribers keep the router speed (secret comment, profile comment, profile name), and the operator can attach an LQoSync plan as an override for one PPP subscriber.

## Owner decisions

- PPP: the router speed stays the default. An LQoSync plan can override it for one subscriber.
- DHCP/IPoE: the operator creates LQoSync plans and attaches one to each client. The client identity is its MAC (`dhcp-mac/v1/...`), so the plan stays when the lease or IP changes.
- Storage stays the JSON registry. SQLite is a later option for more than about 10,000 subscribers, many concurrent editors, or an edit audit history. The service API hides the storage.
- Pin and scoped rebalance move after Phase 2b-3 (Builder). Before then each source has one node, so no other target exists.
- Full profile editing and speed defaults stay in Phase 2b-4.

## Approaches

| Approach | Summary | Trade-off |
|---|---|---|
| Domain operations in the service (chosen) | `plan.create`, `plan.update`, `plan.delete`, `subscriber.plan` | Thin TUI. The web frontend uses the same operations. Validation stays in one place |
| Client-built documents through `policy.commit` | The TUI edits a copy of the registry | Puts logic in each frontend. Against the owner's thin-TUI decision |

There is one sensible approach.

## Model

- A plan is a `SubscriberProfile` with `base: Local(rate)`, `speed: SpeedOverrides::default()` (the catalog defaults derive min and max), and `eligible_targets: None`.
- A plan ID has the form `plan-<n>`. Plans that enrollment made (`SourceRead`) are not plans. They cannot be changed or deleted through the plan operations.
- Attaching a plan sets `SubscriberPolicy.profile_id` to the plan ID. Detaching sets it to `None`, so the source default applies again (router speed for PPP, server rate for DHCP).
- The resolver already uses a subscriber `profile_id` before the source default (`resolver.rs` placement).

## Operations (new)

| Operation | Args | Result | Rules |
|---|---|---|---|
| `plan.create` | `expected_revision`, `name`, `down_mbps`, `up_mbps` | `{revision, plan_id}` | Name unique among plans, 1 to 64 characters. Rates are positive whole Mbps |
| `plan.update` | `expected_revision`, `plan_id`, optional `name`, `down_mbps`, `up_mbps` | `{revision}` | Only `plan-` profiles |
| `plan.delete` | `expected_revision`, `plan_id` | `{revision}` | Refused with `invalid` while a subscriber uses the plan |
| `subscriber.plan` | `expected_revision`, `identity` (`router_id`, `source_id`, `kind`, `account_ref`), `plan_id` or `null` | `{revision, subscriber_id}` | Finds the subscriber record by identity. Creates the record from the latest preview when the account has none |

Every operation uses the existing revision-checked store commit, so a stale revision gives `conflict` and a rule failure gives `invalid`.

## Rules

- Edits need an enrolled registry. Before enrollment, an edit would change the candidate and break the exact enrollment parity. The operations answer `invalid` with "enroll first".
- `subscriber.plan` for an account without a record: the account must be in the latest preview. The new record gets the next subscriber ID, the preview identity, and the plan. It has no pin and no assignment.
- Edits do not change live shaping before Phase 4 activation. After activation, an edit enters the automatic publication pipeline.

## Builder calls (made by the designer, open to owner correction)

- Identity on the wire: the preview subscriber entry gets an `identities` list with the full scope (`router_id`, `source_id`, `kind`) and `account_ref`. `subscriber.plan` takes one full identity. The TUI sends the first identity of the selected subscriber.
- New records: a record made from the preview gets a new ID from the committed `next_id` counter. It can differ from the tentative preview ID. A preview whose `registry_revision` differs from the current revision is stale: the operation answers `conflict`, and the TUI refreshes the preview.
- Enrolled means that the registry has export ownership. Before enrollment, edits answer `invalid` with "enroll first: the registry has no export ownership".
- DHCP client overrides: a subscriber that a `routers.dhcp.clients` override binds keeps the override. `subscriber.plan` answers `invalid` with "remove the DHCP client override first". Production has no DHCP client overrides on 2026-11-18.
- Plan IDs: `plan-<n>`, where `n` is the highest existing plan number plus one. A deleted number can come back, which is safe because a plan in use cannot be deleted. No registry schema change.
- A plan is a profile with the `plan-` prefix and a `Local` base. Both conditions are necessary. Plan names are unique among plans (the catalog does not check names).

## TUI

- Assignments: a key opens a plan picker for the selected subscriber. The list shows "source default" first, then each plan with its rate. Enter attaches, Esc cancels.
- Policies: a "Plans" section lists the plans. Keys create, edit, and delete a plan through a small form (name, down Mbps, up Mbps) with field validation before send.
- After each edit, the TUI asks for a preview refresh so that the views show the new rates.
- The strip shows "edits apply after activation" before Phase 4.

## Verification

- Service tests for each operation: success, conflict, invalid input, delete of a used plan, edit before enrollment, and record creation from the preview.
- A resolver test: a DHCP client with a plan gets the plan rate; detaching gives the server rate again; a PPP override wins over the router speed.
- TUI tests: plan picker selection and cancel, plan form validation, the refresh after an edit, and 80-column layouts.
- Full suite, format, Clippy with warnings denied, and whitespace checks.

## Left to the builder

- Exact keys and form layout, within the global key rules.
- Plan list order.
