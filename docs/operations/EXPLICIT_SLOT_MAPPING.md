# Explicit native slot mapping

LQoSync3 supports the optional `slot_numbers` field in native applied-state reports.
The field contains a nested-CBOR array of unsigned slot IDs.
Each ID corresponds to the name at the same position in the decoded `slot_order` array.
The shared reader converts these pairs to a BTreeMap from slot ID to circuit name.
Subscriber metrics and selected circuit queue requests use these explicit IDs.

An older bridge can omit `slot_numbers`. Only omission selects the legacy one-based IDs.
A present null, empty field, malformed CBOR, trailing data, duplicate ID, zero ID, or count mismatch causes rejection.
The reader also retains the existing generation, report schema, name uniqueness, and name size checks.
It accepts at most 65536 names and bounds the encoded numeric field before its inner decode.
Sparse IDs and IDs above the array length do not cause array indexing or oversized ID-based allocation.

Mapping checks before and after native reads remain required.
A changed ID-to-name association invalidates the sample even if the generation string stays the same.
The subscriber tracker establishes new rate baselines after a mapping change.
Equivalent ID-to-name pairs in a different wire order remain the same mapping.
Malformed input marks telemetry unavailable or stale. It cannot select the legacy fallback.

## Performance and reliability review

Host lookups take O(log n) for n mapped slots. Hosts remain capped at 512 per sample.
The ordered map needs O(n) storage and O(n log n) construction.
Circuit-name lookup remains a bounded O(n) scan for an on-demand queue request.
No new source request, polling interval, UI render step, dependency, or configuration key was added.
The public telemetry cache schema and durable files remain unchanged.
The maximum-size regression covers 65536 sparse mappings and rejects a larger name list.
No production performance improvement or slowdown measurement applies to this local change.

## Verification

The sparse-host and malformed-ID regressions failed against the previous reader, then passed after the change.
The queue regression also proved that the previous reader requested slot 1 instead of explicit slot 5.
Fixtures cover legacy reports, sparse and reordered IDs, invalid fields, trailing bytes, bounds, and mapping changes.
Socket fixtures verify the requested queue ID and rejection of a changed generation or slot mapping.

Full checks: 704 tests passed, zero failed, nine manual tests ignored, across 19 suites.
Clippy with warnings denied, formatting, diff checks, and both affected example probe builds passed.
Live metadata from the preceding review confirmed the installed field format and a dense 27-entry mapping.
The changed product reader has not been deployed or verified against the root-only production socket.

The subscriber source probe now reports `applied_slot_matches`.
It replaces the positional diagnostics `direct_slot_matches` and `offset_one_matches`.
This changes only the manual example output, not the service cache contract.

The post-reboot native policy-proof failure is a separate open task.
This change does not assert APPLIED, change shaping files, or repair the deferred native capacity mismatch.
