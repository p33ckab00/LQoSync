---
type: spec
created: 2026-11-19
status: draft
related: ["[[topology-manager-status]]", "[[2026-11-19-topology-builder-edits]]", "[[2026-11-19-settings-managed-fields]]"]
---

# Fallback group and new subscriber sources design (Phase 2c-2)

## Intent

After enrollment, the operator can add a PPP profile, a DHCP server, or a Hotspot service on a router. The registry does not know that source.
Today a subscriber on an unknown PPP profile fails in the source policy (`MissingProfileMapping`). After Phase 4 activation, Decision 10 (strict publication) holds the whole publication, so new subscribers are not shaped and the previous output stays.
The owner wants shaping to continue: subscribers from a source that the registry does not know go to a fallback group, and the operator then adds the source from the Builder.

## Owner decisions (2026-11-19)

- A fallback group catches subscribers whose source the registry does not know. Shaping of all other subscribers continues.
- The fallback group is an ordinary group node that the operator creates and places, for example one instance below each WAN. The balancer splits it like any group.
- The speed of a fallback subscriber is the router rate-limit when one is readable, else the fallback default plan that the operator sets. The global speed policy (minimum and maximum) applies.
- Without a fallback group, strict publication (Decision 10) stays.
- Not silent: a strip warning ("N subscribers in fallback") and an Assignments list with the reason.
- The Builder Inventory lists each new source with its subscriber count. The operator adds it; the source is made with the enrollment code, with its first instance below a parent that the operator selects, through propose and confirm. The rebalance moves its subscribers from the fallback group to the new group.
- A new protocol on a router (for example Hotspot enabled later) needs an identity authority that the operator selects.
- Approach A (below).

## Non-goals

- A fallback for subscribers with an invalid pin, a restriction conflict, or another placement error. They still hold publication (Decision 10).
- Automatic source creation (Decision 3).
- Router writes.

## Approaches

| Approach | Summary | Trade-off |
|---|---|---|
| A. Explicit fallback policy in the catalog (chosen) | New optional `catalog.fallback` field. The source policy accepts unknown profiles for it, and the resolver uses the fallback nodes only when no source matches | The first registry schema addition. A registry with `fallback` cannot load in an older build (`deny_unknown_fields`); a rollback must remove the field first (runbook item) |
| B. Catch-all source for each scope | A `profile_ref: None` source used only when no specific source matches | `profile_ref: None` already means "every subscriber in the scope"; a new meaning can break current sources |
| C. Automatic source creation into the fallback node | New profiles become sources below the fallback node automatically | Against Decision 3; registry writes without confirm |

## Model

- `TopologyCatalog.fallback: Option<FallbackPolicy>` with `#[serde(default, skip_serializing_if = "Option::is_none")]`, so a registry without it serializes as before.
- `FallbackPolicy { node_ids: Vec<String>, default_plan: RateBps }`.
- Fallback nodes are subscriber group nodes without `source_id`, created by the operator. Validation: each ID is such a node; the list is not empty; `default_plan` is positive; a fallback node cannot also be a source instance.
- `node.create` gets an optional `kind`: `"intermediate"` (default, as today) or `"group"` (a subscriber group without a source, capacity `automatic` or `fixed`, no minimum). This is how the operator creates fallback nodes (designer call).

## Source policy and resolver

- A subscriber whose PPP profile has no binding in its selected scope: when `fallback` is set, the source policy gives `ObservedPolicy { profile: Known(Some(<router profile name>)), plan }` instead of `MissingProfileMapping`. `plan` is only the `rate-limit` of that router profile, parsed with the `RateFormat` of the scope's legacy priority (or the RouterOS default format when the scope has none), or `None` when it is absent or cannot be read. The comment chain and the settings default of the legacy priority do not apply (designer call from the owner decision "router rate-limit, else default plan").
- A record with an attached LQoSync plan (`profile_id` with a `Local` base) keeps that plan in the fallback, as the resolver already prefers the record profile (designer call).
- In placement, an account that matches no source gets the fallback nodes as its eligible set. Its base rate is `plan`, else `default_plan`. Rates use the global speed policy. A subscriber that matches a source but has another placement error does not use the fallback.
- A DHCP server outside the scope's service list stays outside the read, as today (not in fallback, no hold). The Builder lists it as a new source so the operator can add it.
- Candidates come from the routers and protocols that `config.toml` enables, through the enrollment draft. A new protocol on a router needs a `config.toml` change first (Settings), as today (designer call).
- Hotspot is out of this slice: the enrollment draft does not support Hotspot sources yet, and production has Hotspot disabled. Hotspot candidates and their authority picker move to a later slice (designer call).
- When the draft refuses the configuration (for example DHCP client overrides), `topology.sources.candidates` answers `invalid` with the draft refusal text.
- A PPP scope enrolled without `per_plan_node` has one scope-wide source, so a new profile there is placed in that group without the fallback. Production uses `per_plan_node = true` (read on 2026-11-19).
- Owner decision (2026-11-19): a saved assignment that is no longer eligible, on a subscriber without a pin, is dropped in placement instead of giving `InvalidAssignment`. The subscriber is placed again: in its real group when its new profile is known, else in the fallback. This covers a customer moved to a new router plan, and the reverse (a fallback subscriber whose source is added). A pin stays strict (Decision 6): an ineligible pin still gives `InvalidPin` and holds publication.
- Valid saved assignments stay, so fallback subscribers do not move on each read. Adding their real source and its rebalance moves them.

## Operations

| Operation or edit | Effect |
|---|---|
| `fallback.set` edit (`node_ids`, `default_down_mbps`, `default_up_mbps`) | Sets the fallback policy. Affected: the subscribers in fallback (rebalance of the fallback "pool") |
| `fallback.clear` edit | Removes the policy. Refused while online subscribers are in a fallback node ("move the fallback subscribers first"). Offline records assigned to a fallback node lose that assignment, as `node.remove` does (designer call) |
| `topology.sources.candidates` (new read operation) | Reads the routers (as for an enrollment proposal), runs the enrollment draft, and lists draft sources whose origin the registry catalog does not have: `{key, kind, router, name, subscribers}` |
| `source.add` edit (`key`, `parent`, optional `authority` for a new scope) | Adds the draft source, its profile, its binding or selection, and its first instance below `parent`, as enrollment builds them. Affected: the new source and the fallback pool |

All edits use `topology.edit.propose` and `topology.edit.confirm`, with the Phase 2b-3b refusal rules. `source.add` reads the routers in `propose` and refuses when the candidate is gone ("the source is no longer on the router").

## TUI

- Builder: a key marks a selected group node without a source as a fallback node, or opens the fallback form (nodes, default plan).
- Builder Inventory: a "New sources" section from `topology.sources.candidates` (key to refresh), with the subscriber count. Enter opens the parent picker for `source.add`; a new scope also asks for the authority.
- Strip: "N subscribers in fallback" when the preview has fallback placements.
- Assignments: fallback rows show "fallback: <profile or server name>".

## Rollback note

A registry with `fallback` does not load in a build without this field. Before a rollback to an older build, clear the fallback (`fallback.clear`). The Phase 4 runbook gets this step.

## Verification

- Source policy tests: an unknown PPP profile with a fallback gives the rate-limit plan, without a fallback gives `MissingProfileMapping`; an unreadable rate-limit gives `None`.
- Resolver tests: an unmatched subscriber goes to a fallback node with the router plan or `default_plan`; a subscriber with an invalid pin does not; the fallback is balanced across its instances.
- Resolver tests: a subscriber whose saved assignment is no longer eligible and who has no pin is placed again (its real group, or the fallback); the same case with a pin still gives `InvalidPin`.
- Edit and service tests: `fallback.set` validation; `fallback.clear` refusal; `topology.sources.candidates` lists a new PPP profile and a new DHCP server and leaves out known sources; `source.add` makes the same source as enrollment and moves the fallback subscribers; a vanished candidate is refused.
- A schema test: a registry without `fallback` round-trips byte for byte.
- TUI tests: the fallback form, the New sources section, the strip warning, the Assignments reason, 80-column layouts.
- Full suite, format, Clippy with warnings denied, and whitespace checks.

## Left to the builder

- Exact keys and line formats.
- The `key` format of a candidate (stable for one read).
