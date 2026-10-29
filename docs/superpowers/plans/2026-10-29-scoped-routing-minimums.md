# Scoped routing and minimum allocation

Goal: review the actual conditional routing structure and fit subscriber minimums within every shaping ancestor.
Scope: `/opt/LQoSync3.0`, branch `lqosyncv3`, base `1a51b9e`. No router or LibreQoS writes.

The user authorized this work and selected automatic proportional minimum adjustment.
Keep router-owned maximum rates. Preserve the configured floor, capped by the subscriber maximum.
Block when floors cannot fit. Do not change the configured base fraction to a guessed fixed value.

## Design

Recognize a narrow forwarding scope: positive static ingress interface list, positive subscriber list, and a negated destination list.
Read interface-list definitions as well as members. Reject include/exclude composition, unknown flags, missing evidence, and ambiguous members.
Require a matching earlier destination-list bypass. Recognize only exact OSPF and UDP 3784/3785 bypass forms as explicit scope exclusions.
Report the scope in Core review. This does not prove a subscriber's ingress interface or complete packet path.
All other matcher, order, PCC, FastTrack, default-route, ownership, and freshness gates remain active.
Existing manual broad/range address lists may still conflict with proposed assignments. Do not remove or override them.

Allocate effective minimums after subscriber assignment and before compilation/publication.
For each direction, process ancestors before descendants. Reserve floors, then scale remaining minimum headroom proportionally to capacity.
Use deterministic subscriber order and contiguous subtree ranges. Commit input changes only after both directions pass.
The snapshot, CSV, and validation candidate must contain the same effective minimums.
The compiler also rejects aggregate minimums above any ancestor, so callers cannot bypass the allocator.

## Tasks

- [x] Reproduce supported scoped routing and unsafe variants with failing tests.
- [x] Add bounded scope analysis, interface-list collection, diagnostic UI, and faithful fixture projection.
- [x] Reproduce overloaded minimums, nested caps, floor exhaustion, asymmetric rates, and deterministic allocation with failing tests.
- [x] Integrate allocation into normalization/publication and validation. Add compiler admission checks.
- [x] Run focused and full checks, performance workloads, and a fresh independent review. Fix proven defects.
- [x] Refresh the local console and read-only candidate. Run existing v3 preflight with filesystem-write confinement.
- [x] Capture exact remaining blockers and prepare verified changes for authorized commit/push and remote-head verification.

## Review focus

- A list member table alone cannot prove effective interface-list membership.
- Extra matchers, mutable lists, unsupported bypasses, and reordered rules cannot produce a false pass.
- Scoped success cannot be presented as proof of all subscriber packet paths.
- Minimum allocation preserves maxima, floors, identity, and ownership. A failed allocation leaves its input unchanged.
- Nested and asymmetric caps hold without a fixed client count or percentage workaround.

## Ledger

The worktree was clean at `1a51b9e`. Brain recall confirmed local-only scope and active publication gates.
The actual candidate has 25 clients, split 12/13, with 325 Mbps minimum totals under each 320 Mbps WAN.
The user selected automatic proportional minimum allocation with a floor.
Official MikroTik documentation states that composed list membership is absent from the member table.
Read definitions before accepting a static ingress scope. Do not infer completeness from member rows.


Final checks passed: 127 functional tests, formatting, strict Clippy, binary build, live UI, and read-only preflight.
Independent review defects received failing regressions and verified fixes. No material review finding remains unresolved.
The current roster changed from 25 to 26 during live checks. Final verification compares current counts rather than a fixed roster size.
Candidate minima fit both 320 Mbps WANs. The seven route findings are resolved for the explicit checked scope.
Protected manual address-list overlap remains a real membership blocker. No output generation or router change was made.
The verified latest console is active. Git publication is the final authorized action.
