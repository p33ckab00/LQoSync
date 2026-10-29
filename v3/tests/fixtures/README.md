# TLS test fixtures

These certificates and the private key are public test data.
They belong only to the local mock RouterOS server.
Do not use them on any real router or deployed service.

The test CA signs a leaf certificate for `router.test`.
The tests check a trusted name, a wrong name, and an untrusted certificate.
The CA private key was discarded after fixture creation.
The certificates have a ten-year validity period from 2026-10-29.
Replace the fixture pair before expiry.

## Actual core policy projection

`core-policy.actual-sanitized.json` derives from a verified read of a RouterOS 7.25 core on 2026-10-29.
The refreshed source collection completed 12 tables without table errors, including interface-list definitions.
The fixture contains only the fields that the route analyzer needs.
Private identifiers, comments, IP addresses, MAC addresses, and credential data are absent.
Names use consistent aliases. Conditional values use synthetic values.

The fixture preserves two observed custom-table paths, two checked forwarding scopes, and the route finding histogram:

- `mapping-required`: 1

PCC and FastTrack entries exist but are disabled. Their active counts remain zero.
The FIB switch uses the actual presence-only representation.
Known `no-mark` and `connection-state=new` tokens preserve two supported router-reply mark links.
No subscriber-forward connection-mark link is invented.
Known protocol tokens and static interface-list definitions preserve the narrow forwarding scopes and their explicit exclusions.
This fixture tests structural route findings. It does not reproduce address containment, subscriber membership, packet behavior, or WAN assignments.
The export rejects a projection if finding counts, connection-path results, or scope membership counts/exclusions differ from the source review.
This check does not prove full policy equivalence.
