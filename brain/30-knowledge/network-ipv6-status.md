---
type: knowledge
created: 2026-11-02
tags: [network, ipv6, mikrotik]
status: verified
related: ["[[ADR-0012-no-subscriber-grace]]"]
---

# IPv6 status of the production network

> [!summary]
> On 2026-11-02, the production router `FSE-Distro` has no IPv6 internet and gives no IPv6 address to subscribers.
> LQoSync writes an empty `IPv6` column, and that is correct. Do not add IPv6 discovery until the network gets IPv6.

## Details

The operator ran these read-only commands on the router:

| Check | Result |
|-------|--------|
| `/ipv6/dhcp-server print` | No DHCPv6 server |
| `/ipv6/dhcp-server/binding print count-only` | 0 |
| PPP profiles | No `dhcpv6-pd-pool`, no `remote-ipv6-prefix-pool`. `use-ipv6=yes` gives only link-local addresses. |
| `/ipv6/neighbor print count-only` | 63, all in `fe80::/10` (0 outside it) |
| `/ipv6/address print where !link-local` | Only `::1/128` on `lo` |
| `/ipv6/route print where dst-address=::/0` | No default route |
| `/ipv6/settings` | `ipv6-fast-path-packets: 0` |

LibreQoS has `mikrotikFindIPv6.py`. It maps IPv4 to IPv6 by the MAC address from `/ip/arp`, `/ip/dhcp-server/lease`, `/ipv6/dhcp-server/binding` (DUID) and `/ipv6/neighbor`.
Only the built-in integrations use it (`integrationCommon.py`, `use_mikrotik_ipv6 = true`). The `shaped_devices` source never calls it.
The MAC match does not apply to PPPoE sessions.

## Upstream test on FSE-CORE

The WAN router `FSE-CORE` has two PPPoE client uplinks, `pppoe-out1` (`ether3-WAN`) and `pppoe-out2` (`ether2-WAN`), with profile `default` (`use-ipv6=yes`).
The subscribers connect to `FSE-Distro`, not to the core router.
On 2026-11-02, the operator ran a test DHCPv6 client on both uplinks (`request=prefix`, and `address,prefix` on `pppoe-out1`):

- `disable-ipv6: no` on the core router.
- Neither PPPoE interface has an `fe80::` address. The ISP did not open IPv6CP on the sessions.
- Both DHCPv6 clients stayed `searching...`. No pool and no `::/0` route.
- The core router has no IPv6 firewall rules.

Both uplinks have a private IPv4 address (`172.16.1.246/32`, `172.16.1.247/32`, gateway `172.16.1.1`). The ISP uses NAT.
Thus a 6in4 tunnel (Hurricane Electric) cannot operate: it needs a public IPv4 address that accepts protocol 41.
A WireGuard tunnel to a rented server with an IPv6 prefix is possible, but it adds cost, latency, a wrong geolocation, and a single point of failure. Streaming services often block such prefixes.

Result: the ISP gives no IPv6 on either account. The operator must ask the ISP for IPv6CP and DHCPv6 prefix delegation (/56).
After that: add the IPv6 firewall on the core router, reconnect one PPPoE session, and repeat the DHCPv6 client test.
The core router must then delegate a prefix to `FSE-Distro`, and `FSE-Distro` gives a /64 to each PPPoE subscriber (`dhcpv6-pd-pool`).

## When to examine this again

- The upstream gives an IPv6 prefix, or the router gets an IPv6 default route.
- A DHCPv6 server or a PPP `dhcpv6-pd-pool` is added.

Then read the router tables first. For PPPoE, the delegated prefix of each session is the probable source, not the MAC.

## Sources
- Router output from the operator, 2026-11-02
- `/opt/libreqos/src/mikrotikFindIPv6.py`, `/opt/libreqos/src/integrationCommon.py` (LibreQoS 3.0.7)
