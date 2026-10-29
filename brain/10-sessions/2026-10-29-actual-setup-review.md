---
type: session
created: 2026-10-29
tags: [lqosync-v3, topology, sources, configuration, validation]
status: complete
---

# Actual local configuration and candidate review

The user authorized local LQoSync configuration after validator completion.
The user confirmed two WAN roots, each with 320 Mbps download and upload.
Each root has DHCP, PPPoE, and Hotspot branches. All computed factors are 1.0.
All services use automatic client balancing. A PPP profile identifies its source/default WAN only.
No subscriber has a fixed WAN pin. Router configuration and LibreQoS changes remain outside the scope.
Branch: `lqosyncv3`. Checkout: `/opt/LQoSync3.0`. Base: `6bafc91`.

## Configuration

The agent used the current UI forms. No application code changed.
Local publication paused while the complete configuration was assembled, then resumed.
A private settings backup exists under application state.
The two WANs map to their respective PPP client interfaces and existing custom tables/address lists.

Six service branches and ten source branches produce 18 configured nodes.
Ten source bindings comprise three DHCP servers, six PPP profiles, and one Hotspot profile.
Each source has a balancing pool with an eligible destination on the other WAN of the same service type.
DHCP Auto uses server-owned speed evidence. Management DHCP remains outside subscriber selection.

The default Hotspot profile is selected, but it has no active clients or finite speed evidence.
Future unresolved Hotspot rates must block publication. No speed was invented.
Router connection settings, credentials references, and confirmed CA paths remained unchanged.

## Verification

Fresh verified UI reads completed 14 distro tables and 11 core tables without errors.
The speed preview resolved 26 entries without source errors. Management exclusion leaves 25 selected subscribers.
Live Validation passed inventory health, subscriber speeds, subscriber assignment, and shaping capacity.
The candidate split is 12 subscribers on WAN1 and 13 on WAN2.
WAN1 has three DHCP and nine PPPoE clients. WAN2 has one DHCP and twelve PPPoE clients.

Each WAN has 650 Mbps of total subscriber plan rates. Each root caps shaping capacity at 320 Mbps.
The current 50 percent minimum policy produces 325 Mbps of total subscriber minimum rates per WAN.
This exceeds each root by 5 Mbps. The minimum policy remains unchanged and requires separate capacity review.

The agent reused the compiled application library for a private candidate, without publication.
The preview has 25 unique CSV identities and IPv4 addresses. Every parent reference exists.
Directional capacities remain positive and within their ancestors. Empty computed branches are omitted.
The preview contains nine exported nodes. Nine empty configured nodes remain outside this output.

The existing v3 preflight accepted the pair: topology validated, at least 25 circuits, pre-install exit 0.
Landlock denied filesystem mutations in the diagnostic child. Only its previously opened private output log remained writable.
This file-pair check does not prove packet routing or runtime shaping behavior.
Eight UI routes passed. Scheduled collection remains active and fresh. All pools preserve the service type.
No code changed, so the agent did not repeat the application test suite.

## Remaining gates and artifacts

The exact core mappings removed the earlier mapping-required finding.
Seven core-policy findings remain: three complex-prerouting and four conditional-matcher findings.
Core membership remains Unsupported. The complete end-to-end result remains Unavailable without a committed generation.
The application did not publish a generation or bypass its core-policy gate.
The candidate split is a preview, not a committed sticky assignment or router route.

No router configuration, LibreQoS files/packages/services, or production shaping changed.

Private artifacts remain under ignored `v3/data/state/` with mode 0600.
The `setup-preview/` directory has mode 0700 and contains the CSV/JSON pair, candidate records, summary, and preflight report.
`topology-setup-result.json` records the configuration result. `topology-proposal.json` now records the configured policy.
Credentials, subscriber names, IP addresses, tokens, and private policy identifiers remain outside Git and this vault.

Next: review support for the remaining conditional/bypass policies and review aggregate minimum rates.
Do not change router rules or relax validator gates to force success.
Router writes, coordinated shaping apply, full packet-path proof, authentication, and retention remain future work.
See [[2026-10-29-connection-mark-validation]], [[2026-10-29-dhcp-server-speeds]], [[../Home]], and [[../HANDOFF]].
