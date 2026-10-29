# DHCP server speed sources

The user selected all three sources: server comment, server name, and local per-server settings.
Keep MikroTik and LibreQoS configuration unchanged.
Worktree: `/opt/LQoSync3.0`, branch `lqosyncv3`, base `0b365ab`.

## Design

Store rules under `speed_rules.dhcp_servers` in application-local TOML.
Key each rule by exact router ID and DHCP server name.
Offer Auto, Server comment, Server name, and Manual modes on Speed rules.
Auto uses comment, then an explicit unit-marked name suffix, then configured manual rates.
Manual rates are subscriber download/upload Mbps. They are separate from topology capacity.
An invalid selected rule stops resolution. Do not fall through to a weaker source after a parse error.
No lease speed fallback remains. Bound leases inherit the speed of their actual DHCP server.
Old settings without per-server rules use Auto and remain readable.
A server without a finite rule remains unresolved. Keep the last valid generation.
Index and parse each server once per preview, with exact active-server matching.
Failed, duplicate, absent, or disabled server metadata cannot supply a speed.
PPP and Hotspot behavior remains unchanged.

Approaches considered: extend existing lease priority, add server-aware resolution, or replace the speed module.
Server-aware resolution meets the request and reuses existing rate parsing, previews, and publication guards.
Extending lease priority would preserve the wrong source. Replacing the module adds unnecessary risk.
The implementation choices above are agent defaults within the requested three-source behavior.

## Work

- [x] Add failing resolver, inventory, normalization, and UI tests for server-scoped rates and failure behavior.
- [x] Add typed per-server settings, validation, shared parsing, and cached server joins. Collect server comments.
- [x] Add per-server controls, preserve policies on global saves, and trigger existing automatic local refresh.
- [x] Review performance and integrity. Run focused and full checks, plus a DHCP scaling measurement.
- [x] Update guides and project notes. Commit/push the authorized branch and verify its remote head.

## Review focus

- Duplicate router/server keys and partial server reads cannot supply rates.
- Invalid comments or name suffixes cannot silently select manual fallback.
- Active-server names remain exact, including whitespace and shared names across routers.
- Global speed saves preserve per-server settings. Stale forms cannot overwrite newer speed policy.
- Subscriber speeds remain separate from computed branch capacities. No router writes exist.


## Ledger

Resolver and UI regressions failed before implementation, then passed.
The resolver indexes per-server policies and parses each server once. Minimum-only validation avoids repeated full-policy scans per subscriber.
Global speed saves preserve per-server settings. New server forms carry a settings revision.
Removing a rule restores Auto. Old empty settings omit the new field during serialization.
Focused normalization and preview tests pass after fixtures move their speed rules from leases to server metadata.
The DHCP benchmark passed: 50k leases /1,000 server rules, 236.6 ms in debug, excluding collection and disk I/O.
Independent review found one malformed-name suffix defect: `DHCP-100/ 20M` incorrectly became 20/20.
The regression failed before the slash/whitespace guard. Space and tab cases now receive the same rejection as malformed comments.
Final validation: 100 tests passed; six manual benchmarks remain ignored by default.
The final DHCP benchmark passed separately at 245.8 ms debug, excluding collection and disk I/O.
Formatting, strict Clippy, build, diff checks, and four-route isolated HTTP smoke passed.
The latest LQoSync process serves the new controls. Existing app settings remained unchanged during reload and reads.
Fresh distro/core collections completed 14/11 tables with zero errors. Four DHCP servers include comments.
Git commit and remote-head verification are the final authorized publication action.
