---
type: spec
created: 2026-11-19
status: draft
related: ["[[topology-manager-status]]", "[[2026-11-19-topology-builder-edits]]"]
---

# Speed policy and group factor design (Phase 2b-4)

## Intent

The owner wants to edit two speed settings in the Topology TUI:

- The global Subscriber Speed Policy: the minimum and maximum of each subscriber. Today the minimum is 10% of the plan (legacy `min_rate_percentage = 0.1`).
- The factor of each subscriber group. Today it is `0.215` for PPP groups and `0.5` for DHCP groups (legacy `pppoe_aggregate_factor`, `dhcp_download_factor`, `dhcp_upload_factor`).

## Owner decisions (2026-11-19)

- The minimum is global for each subscriber. Plans have no minimum of their own.
- A group node capacity is the sum of its subscriber plan rates times the group factor. When that is above the WAN, the WAN ceiling is the group ceiling. The code does this today (`aggregate.rs`: the local capacity is constrained by the parent ceiling). Subscriber plans are not scaled down to fit a WAN.
- The group factor is editable for each source. All instances of the group use it.
- Backstop, floor, and formula stay read-only and show in Policies.
- Approach A (below).
- No effect on live shaping before Phase 4 activation.

## Non-goals

- Minimum or maximum overrides for each plan or profile.
- Factor overrides for each node instance.
- Backstop, floor, and formula edits. Speed schedules (Phase 5).

## Approaches

| Approach | Summary | Trade-off |
|---|---|---|
| A. New edit kinds in the existing propose and confirm (chosen) | `speed.defaults` and `group.factor`. The proposal reads sources, resolves placements, and runs the aggregate check without a rebalance | A router read for each preview. The commit cannot carry an aggregate conflict. Same flow and popup as the Builder |
| B. Direct commit, like `plan.update` | Operations commit at once. The operator sees the effect in the next preview | Less code. No preview before commit, and a revision with `MinimumExceedsCeiling` can commit and block publication after activation |

## Edit kinds (new)

| `kind` | Fields | Effect |
|---|---|---|
| `speed.defaults` | `down_min`, `up_min`, `down_max`, `up_max`, each `{"percent": p}` or `{"mbps": n}` | Replaces `catalog.defaults` and sets the `speed` of every profile to `SpeedOverrides::default()` (all `Inherit`). `percent` gives `RateRule::BaseFraction`, `mbps` gives `RateRule::FixedBps(n × 1,000,000)` |
| `group.factor` | `source_id`, optional `down`, optional `up` | Replaces the given directions of `GroupDefaults.factors` of the subscriber source. An absent direction keeps its value |

### Where the 10% lives today

Enrollment gives every imported source profile a `speed` override with `RuleValue::Override(LegacyRoundedBase)` on all four rules (`policy_draft.rs`, `legacy_bounds::import`). `catalog.defaults` holds a 1 Mbps minimum and a 100% maximum, and only plan profiles use it.
The owner wants one global minimum for every subscriber and no minimum for each plan. So `speed.defaults` writes the defaults and resets every profile override to `Inherit`, so that the defaults apply to all subscribers (designer call from the owner decision).
This replaces the legacy rounded rules. The proposal warns: "the legacy rounded rules of <n> profiles are replaced; the output no longer matches the legacy loop exactly".

### Rules

- Minimum percent: 0.1 to 100. Maximum percent: 1 to 1,000. Percents take up to one decimal (`Fraction { numerator: p × 10, denominator: 1000 }`). Mbps: whole number 1 to 1,000,000 (designer call on the ranges).
- The existing speed validation must pass: minimum not above maximum for each resolved profile.
- Group factor: a decimal string from `"0.001"` to `"1"` with up to six decimals (`Fraction { numerator, denominator: 1,000,000 }`), so the legacy `0.215` can be entered again. The form shows the stored value as a decimal (designer call).
- A fixed minimum above the maximum of a subscriber (for example 5 Mbps with a 2 Mbps plan) shows as `InvalidRates` in placement. The proposal refuses it with "the minimum is above the maximum for: <subscriber IDs>", not with the placement conflict text (designer call).
- `group.factor` needs a subscriber source ("the source does not exist").
- Node instances with a factor override keep their override.
- Both edits need export ownership, as all edits do.

### Proposal

- Both edits read the sources (`reconcile_for`), resolve placements, and run the aggregate evaluation on the current registry and on the candidate. No rebalance.
- Only a conflict that the current registry does not have refuses the edit, so an edit that lowers a minimum can fix an existing `MinimumExceedsCeiling` (designer call). A new placement conflict refuses ("resolve the placement conflicts first", or the `InvalidRates` text above). A new aggregate conflict refuses with its text.
- `rates`: up to 8 online subscribers whose minimum or maximum changes: `{subscriber_id, label, before: {min_mbps, max_mbps}, after: {min_mbps, max_mbps}}`, and `rates_changed`: the number of online subscribers whose minimum or maximum changes.
- `capacities`: each group node whose exported ceiling changes: `{node_id, before_mbps, after_mbps}`.
- `warnings`: one line for each default that replaces a legacy rounded rule: "<field> no longer matches the legacy loop exactly".
- The other proposal fields stay as in Phase 2b-3b (`moves` is empty).

## Phase 4 note

These edits make the registry differ from the legacy `config.toml` values. `preflight::legacy_issues` reports `GroupFactors` and `DynamicSubscriberBounds` for such differences, and today it runs at legacy adoption. The Phase 4 activation design must treat the registry as the authority and must not re-run that check against `config.toml` after these edits.

## TUI

- Policies shows a "Subscriber Speed Policy" section with the four rules in plain text ("10% of plan", "500 Mbps", "legacy 10% rounded"), and a "Groups" section with each subscriber source: name, factor down and up percent, formula, backstop, and floor.
- A key opens the speed form: four rows, each with a mode (percent or Mbps) and a value. Field errors show next to the field.
- A key on a selected group opens the factor form: down percent and up percent.
- Both forms send `topology.edit.propose`. The proposal popup adds the change line ("speed defaults" or "factor of <group>"), the `rates` samples, the `capacities`, and the warnings.
- Forms fit 80 columns. Keys are the builder's choice and do not collide with the Plans keys.

## Verification

- Edit rule tests: each rule conversion, each range limit, minimum above maximum refused, unknown source refused, instance overrides kept.
- Service tests: a minimum change lists changed subscriber rates; a factor change lists the changed group capacities; a minimum that exceeds a WAN ceiling is refused with the aggregate conflict; a legacy rule replacement gives the warning; no proposal is kept after a refusal.
- TUI tests: section texts, form validation, the propose arguments, the popup lines, 80-column layouts.
- Full suite, format, Clippy with warnings denied, and whitespace checks.

## Left to the builder

- Exact keys and popup line format.
- The order of the `rates` samples.
