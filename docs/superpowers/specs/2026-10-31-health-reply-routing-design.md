# WAN health: accept reply routing for inbound connection marks

Status: approved by the operator in chat, 2026-10-31.
Result: on `RB5k9Core`, wan1 and wan2 reported Healthy after the recovery delay.

## Intent

The WAN health provider rejects every enabled output mangle rule.
The production core has two such rules, `*C1` and `*C2`. They route router replies for inbound WAN connections.
The rules are correct, so health stays Unknown. This change accepts that rule shape and nothing more.

## Why the shape is safe

Netwatch probes start on the router. The router creates their connections in the output path.
Those connections never pass the prerouting chain as new connections.
A prerouting rule with `connection-state=new` therefore cannot mark a probe connection.
A probe reply arrives as an established packet, so that rule cannot mark it either.
An output rule that matches only such a mark cannot route a probe.

## Accepted output rule

An enabled `output` rule is accepted only when all of these are true:

- `action` is `mark-routing`.
- `connection-mark` is set. It is not empty, not `no-mark`, and not negated with `!`.
- The rule has no other match field. Permitted other keys: `.id`, `chain`, `action`, `comment`,
  `disabled`, `dynamic`, `invalid`, `log`, `log-prefix`, `bytes`, `packets`, `new-routing-mark`, `passthrough`.
- Every enabled mangle rule that sets that mark with `mark-connection` is in `prerouting`,
  with `connection-state` exactly `new`.

Every other enabled output rule still makes health Unknown. The other mangle checks do not change.

## Point-to-point gateway

The provider also accepted only a next-hop gateway in the form `IP%interface`.
A PPPoE WAN route has the interface itself as its immediate gateway, for example `pppoe-out1`.
The provider now also accepts an immediate gateway equal to the mapped WAN interface name.
A list of gateways still makes health Unknown.

## Approaches

- **Selected:** accept the exact rule shape above in the provider. It needs no settings change.
- **Rejected:** a settings flag that tells the provider to trust output rules. It gives no proof.
- **Rejected:** delete `*C1` and `*C2` on the router. Router replies for inbound WAN connections then use the wrong WAN.

## Tests

Provider tests in `v3/tests/wan_health.rs` cover the accepted pair `*A4` and `*C1`, each rejection above, and the interface-only gateway.

## Left to the builder

- Error text. Use the existing message for rejections.
