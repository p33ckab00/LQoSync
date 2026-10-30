# WAN health execution ledger

Scope: implement the authorized health and preview phase only. Later writer tasks remain pending.
Base: `7a9f198`. Branch: `codex/wan-health`. Worktree: `/tmp/lqosync-wan-health`.
Ruling: keep provider support conservative. Unsupported output policy or unverifiable completed probe evidence returns Unknown.
Ruling: do not add Netwatch to automatic inventory reads. The explicit read-only preview requests it only when configured.
Ruling: first sample establishes a counter baseline. Only advancing counters for both probes can contribute a round.
No live reads, writes, settings changes, service changes, or restarts are required for implementation tests.

RED: missing health module/settings field; then missing preview binary; then policy threshold changes retained stale confidence.
GREEN: 9 health/provider tests and 2 CLI tests passed after implementation and context binding.
The reviewer found no material code defect. Isolated RouterOS probe semantics remain an acceptance gate.
Full checks: 243 tests passed, zero failures, 12 ignored. Format, strict Clippy, diff, links, Bash syntax, and TOML example passed.
Provider acceptance remains pending isolated RouterOS verification. No live reads, writes, or restarts occurred.
