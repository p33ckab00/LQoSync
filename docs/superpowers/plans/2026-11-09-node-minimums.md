# Optional node minimums implementation plan

Goal: Add optional node minimums without changes to default output or live policy.

Use the shared config, node state, and pair writer. Add no dependencies.
Execute this plan inline under the user's authorization to implement, review, commit, push, and restart.

- [x] Add regression tests for root and group minimums, default output, old state, invalid rates, and minimum-only diffs.
- [x] Add optional whole-Mbps keys to `Network` and `NodeTemplate`, with positive-value validation.
- [x] Carry the values through `StoredNode`, seed import, and JSON output. Omit absent values.
- [x] Reject minimums above generated ceilings before any live write.
- [x] Include minimums in diffs, native format translation, Settings metadata, and documentation.
- [x] Compile a synthetic pair with the installed LibreQoS binding in isolation. Confirm retained minimums. Runtime parent clamps remain untested.
- [x] Review performance and integrity. Run formatting, clippy, and the full test suite.
- [ ] Commit and push to `lqosync3`. Build the committed revision and prepare a binary backup before deployment.
- [ ] Deploy and restart `lqosync3` if host permissions permit. Verify the process revision and a successful cycle.

Review focus: no defaults added to live config, no lost minimums across state reloads, no hidden diff changes,
no invalid generated minimums, and no switch to the blocked native source.
