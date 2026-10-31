---
type: decision
created: 2026-10-31
tags: [adr, configuration, dhcp]
status: accepted
supersedes:
related: ["[[ADR-0004-toml-policy-python-defaults]]", "[[ADR-0007-fresh-design-not-a-port]]"]
---

# ADR-0009 — DHCP servers are discovered like PPP, with exclusions in TOML

## Context

`updatecsv.py` reads only the DHCP servers that `config.json` lists. PPP is dynamic: the tool reads all secrets and profiles.
The operator wants DHCP to be dynamic too, and wants TOML settings to exclude DHCP servers, PPP profiles, and IP addresses.
The live router has a management DHCP server that must not be shaped.

## Decision

1. The tool reads all DHCP servers from `/ip/dhcp-server`. A disabled server is skipped. `routers.dhcp.discover = false` uses only the listed servers.
2. `[[routers.dhcp.servers]]` is optional. It changes the settings of one server. Other servers use the defaults.
3. `[exclude]` (all routers) and `[routers.exclude]` (one router) have `dhcp_servers`, `ppp_profiles`, and `ips`. The tool uses both lists.
4. Names accept `*` wildcards and ignore case. `ips` accepts IPv4 addresses and CIDR blocks. The loader rejects a bad address and an empty name.

## Alternatives

1. A fixed server list, as in `updatecsv.py`. A new server on the router then needs a config change.
2. Regular expressions for names. More power, but easy to get wrong. Wildcards cover the operator cases.

## Consequences

- A new DHCP server on a router is shaped at once with the default rate. Exclude it if that is not wanted.
- An excluded subscriber leaves the files through the normal removal, so the removal guard protects a large change.

## Status
accepted on 2026-10-31 (operator request in chat)
