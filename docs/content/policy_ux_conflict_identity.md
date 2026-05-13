# v2.56 Policy UX + Conflict Intelligence

LQoSync v2.56 improves the Smart Policy Center with read-only conflict detection, stronger preset comparison, and source-aware client identity guidance.

## Policy Conflict Resolver

The Policy Conflict Resolver reviews the current `config.json -> policies` block and explains risky policy combinations before they affect cleanup, Dry Run, or LibreQoS apply behavior.

Examples detected:

- `cleanup_immediate` normal cleanup combined with permissive zero-result behavior
- collector-failed action that can delete rows
- source-disabled cleanup set to immediate
- high/critical risk auto-apply enabled
- apply guards disabled
- grace enabled for mixed/unstable identity sources

Each conflict includes:

- severity
- what is configured
- why it matters
- recommended fix
- affected config paths

The resolver is read-only. It does not write config or change policy values.

## Better Policy Preset Comparison

Policy Center now shows a clearer current-vs-preset table. The table includes:

- setting label
- section/category
- current value
- selected preset value
- risk level
- setup guidance

This helps operators understand exactly how their custom policy differs from Conservative, Balanced, or Aggressive presets.

## Client Identity Handling

Lifecycle and cleanup decisions depend on how stable a client identity is.

Recommended identity model:

- PPPoE: username, usually stable
- DHCP: DHCP server + MAC, mixed stability because private/random MAC can create new clients
- Hotspot: username/voucher or MAC, stable only when username/voucher-based
- Static/manual: manual identity, stable

Grace/stale lifecycle behavior should remain optional and source-aware. It is safest on stable identities such as PPPoE usernames, and should normally stay disabled for DHCP environments where devices may use randomized MAC addresses.
