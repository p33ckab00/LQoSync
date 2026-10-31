---
type: decision
created: 2026-10-31
tags: [adr, architecture, performance, integrity]
status: accepted
supersedes: "[[ADR-0004-toml-policy-python-defaults]] (items 3 and 4, and the parity goal)"
related: ["[[ADR-0004-toml-policy-python-defaults]]", "[[ADR-0005-topology-follows-v3]]", "[[updatecsv-py-dissection]]"]
---

# ADR-0007 — lqosync3 is a new design, not a port of `updatecsv.py`

## Context

The operator does not want a copy of `updatecsv.py`. The operator wants new code with high performance and high data integrity.
`updatecsv.py` stays a guide for the domain rules: the sources, the speed sources, the network modes, and the LibreQoS file requirements.

## Decision

1. Do not copy the output format or the defects of `updatecsv.py`. Byte parity with `updatecsv.py` is not a goal.
2. Remove the golden parity tests that run `updatecsv.py`. Use scenario tests with expected output that we write from the spec.
3. Keep ADR-0004 items 1, 2, and 5: all policy in TOML, formats in code, and the speed source lists.
4. Before the cutover, the tool compares its output with the live files at the semantic level: circuits, IPs, parents, and rates. It does not compare bytes.
5. Performance and integrity rules have priority over compatibility with `updatecsv.py` behavior.

## Alternatives

1. Byte parity with `updatecsv.py` (the old plan). It keeps the Python quirks, for example Python float text and the comment parse with no word boundary.

## Consequences

- The Python float format functions (`py_round3`, `py_float_str`) are not necessary.
- A default can differ from `updatecsv.py` if it is more correct. The semantic compare shows each difference before the cutover.
- The `updatecsv.py` dissection stays the reference for the domain rules.

## Status
accepted on 2026-10-31 (operator direction in chat)
