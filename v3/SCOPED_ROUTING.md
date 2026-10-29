# Scoped forwarding checks

Core review can identify a narrow conditional forwarding scope.
This scope applies to packets from a positive subscriber source list on an explicit static ingress interface list.
The routing rule must exclude router-local destinations and a named IPv4 destination list.
It must select the configured WAN table with `passthrough=false`.

## Required evidence

- Fresh complete core inventory and exact WAN/list/table bindings remain required for planning.
- `/interface/list/print` must contain one static non-builtin definition for the ingress list.
- The definition must have no `include` or `exclude` composition.
- `/interface/list/member/print` must contain nonempty, unique, enabled static members.
- Each member must resolve to one enabled running interface. Mapped WAN interfaces cannot be ingress members.
- Destination exclusions must resolve to enabled static IPv4 hosts, CIDRs, or ranges without timeouts.
- Unresolved DNS values, unknown flags, and exclusions that cover all IPv4 destinations block the scope.
- Known active or unknown-status writers in the collected filter/mangle tables cannot target the exclusion list. A missing writer target also blocks review.
- One matching destination-list `accept` rule must precede the forwarding rule.

The [MikroTik interface-list guide](https://help.mikrotik.com/docs/spaces/ROS/pages/47579180/Interface%2BLists) explains list composition.
The member table contains static members. It does not contain effective members from `include` and `exclude` statements.
The validator therefore reads definitions as well as members.

## Bypass forms

The analyzer recognizes these exact earlier `prerouting` accept forms:

1. A positive destination-list match, paired with the routing rules' shared negative destination-list match.
2. IP protocol `ospf` or `89`, with no additional packet matcher.
3. UDP destination ports `3784,3785`, with no additional packet matcher.

All active prerouting routing consumers must have the checked shared destination scope before these bypasses receive support.
Extra source conditions, arbitrary port ranges, broad accepts, wrong order, and unknown matchers remain blockers.
PCC, FastTrack, jumps, routing rules, default-route ambiguity, connection-mark ambiguity, and ownership checks remain unchanged.
See the official [mangle reference](https://manual.mikrotik.com/docs/cli-reference/ip/firewall/mangle/) for RouterOS rule semantics.

## Meaning of the result

Core review displays the source list, ingress interfaces, destination exclusion list, and control-protocol exclusions.
These are explicit limits of the checked forwarding scope.
The result does not prove which interface a subscriber actually uses.
It does not establish the route of excluded traffic or simulate the complete firewall, NAT, VRF, and connection state.
RAW rules, scripts, and external updates are outside this structural review.
Historical evidence remains historical. No review changes router rules or publishes output.

Supported route structure does not establish subscriber membership consistency.
Existing manual entries, broad prefixes, and address ranges remain protected.
They can still conflict with proposed per-subscriber WAN assignments and prevent publication.
The system does not remove those entries or recommend router-rule changes to force a pass.

Definitions, members, interfaces, addresses, and writers use indexed lookups.
The analyzer checks at most 512 conditional candidates and reports an explicit global finding when more exist.
This budget applies before candidate validation, including rejected candidates.
The UI shows at most 200 scopes and 32 ingress names per scope, with explicit omitted counts.
Every member still receives analysis within each admitted candidate.

## Checks

From `v3/`:

```bash
cargo test --locked --test scoped_routing
```

Tests cover exact scopes, missing definitions, composition, dynamic members, duplicate members, WAN ingress, unknown interfaces, and failed tables.
They also cover mutable exclusions, DNS values, full IPv4 coverage, broad accepts, extra matchers, protocol changes, and wrong rule order.
