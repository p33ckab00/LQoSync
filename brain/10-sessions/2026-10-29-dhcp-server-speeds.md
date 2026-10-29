---
type: session
created: 2026-10-29
tags: [lqosync-v3, dhcp, speeds, integrity, performance]
status: complete
---

# DHCP server speed sources

The user corrected the DHCP speed source and selected all three supported inputs.
Use DHCP server comments, unit-marked server names, and per-server manual settings in this application.
Checkout: `/opt/LQoSync3.0`. Branch: `lqosyncv3`. Base: `0b365ab`.
Commit/push authorization continues from the project workflow.
MikroTik and LibreQoS configuration remain unchanged.

The implementation adds `speed_rules.dhcp_servers`, keyed by exact router ID and server name.
The agent default is Auto: comment, name, then configured manual fallback.
Explicit Comment, Name, and Manual modes let the operator choose one source.
Manual rates use subscriber download/upload Mbps and apply to each subscriber.
Branch capacities, computed aggregation factors, and WAN caps remain separate controls.
These comment/name formats are orchestration conventions, not native RouterOS bandwidth enforcement.

Bound leases inherit their actual DHCP server's resolved speed.
Lease comments and lease rates no longer override that server policy.
Wildcard configured servers require an actual server identity.
Missing, failed, duplicate, disabled, or unknown server metadata cannot supply a rate.
Malformed speed rules remain unresolved. Auto does not silently select a later source after a parse error.
Old settings remain readable. Servers without rules use Auto without a manual fallback.
Lease-only deployments need a finite server rule before their next valid local publication.
An invalid new rule retains the last valid generation.

Speed rules now includes server-specific forms, discovered choices, explicit mode, and manual rate fields.
Settings revisions reject stale server forms. Global policy saves preserve all server rules.
Removing a rule restores Auto. Existing automatic local normalization runs after successful saves.
The collector includes DHCP server comments through its existing read-only allowlist.
No dependency was added. PPP and Hotspot resolution remains unchanged.

The preview indexes per-server policy and resolves each shared server once.
Minimum-only validation avoids full server-policy scans inside subscriber rate resolution.
The synthetic workload covers 50k DHCP leases and 1,000 server rules, with distinct server rates and reversed input order.
Initial debug result: 236.6 ms, excluding collection and disk I/O.

Tests cover all sources, mode selection, manual fallback, source isolation, exact active server names, duplicate metadata, and invalid rates.
The UI test checks CSRF, stale forms, persistence, automatic publication, global-save preservation, removal, and generation retention after invalid server text.
Independent review found one malformed-name defect: `DHCP-100/ 20M` incorrectly resolved as 20/20.
A regression failed before the fix. Space and tab cases now pass with the prefix slash guard.
No additional material issue was found in the review.

Final verification: 100 functional tests passed. Six manual benchmarks are ignored by default.
The DHCP benchmark passed separately: 245.8 ms debug preview, excluding collection and disk I/O.
Formatting, strict Clippy, build, diff checks, and four-route isolated HTTP smoke passed.
The isolated smoke published no generation and its temporary process/data were removed.
The LQoSync web process now uses the verified latest binary. Existing settings and CA paths stayed byte-identical across reload and reads.
Fresh verified reads completed 14 distro tables and 11 core tables with zero table errors.
Four DHCP servers were discovered with comments. All four speed-source modes appear in the live UI.
The preceding support steps restored the confirmed core CA and configured the independently confirmed distro CA in app settings only.
Private credentials, certificates, router identifiers, and subscriber data remain outside Git and these notes.
Changed modules: speeds, speed_preview, speed_web, dhcp_speed_web, sources_web, routeros, model, web, and lib.
Fixtures, tests, guides, the plan, and project notes accompany the implementation.
See `v3/SPEED_RULES.md`, [[2026-10-29-speed-rules]], [[2026-10-29-actual-core-validation]], [[../Home]], and [[../HANDOFF]].

Next: select explicit per-server policies in the UI and bind sources to the intended shaping nodes.
Broader core-policy support, router writes, coordinated shaping apply, authentication, and retention remain future work.
