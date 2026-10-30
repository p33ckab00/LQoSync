# Core production adapter implementation plan

> **For agentic workers:** Use superpowers:executing-plans inline, TDD, and a final fresh-context review.

**Goal:** Connect the guarded engine to RouterOS, an exclusive maintenance process, and observed-WAN candidate bootstrap. Prove the path with isolated protocol tests. Do not perform live writes.

**Architecture:** A separate maintenance CLI prepares, applies, or rolls back an exact private plan. The console holds a shared OS lease for its lifetime. The CLI requires an exclusive lease and a durable recovery marker before writes. Kernel locks exclude updated cooperating consoles and maintenance processes. An explicit plan-bound operator acknowledgement covers external RouterOS writers, which advisory local locks cannot control.

**Constraints:** Preserve protected manual data, exact row guards, real session continuity, authenticated local operator access, and separate actual validation/publication. Use the existing TLS/credentials client. Never expose generic RouterOS command execution or an HTTP mutation route. A lost write reply remains pending. A recovery marker prevents console restart until verified completion or rollback. Private plans and journals stay outside Git.

## Tasks

- [x] Add failing lease, protocol, and bootstrap tests.
- [x] Implement OS leases and a persistent migration recovery marker. Keep Rust 1.88 support with the already cached libc dependency.
- [x] Add a crate-private address-list-only mutation method to the existing client. Reject traps, incomplete replies, absent returned IDs, and timeout ambiguity.
- [x] Implement fresh normalized evidence, stable revision fingerprints, real-session observed-WAN bootstrap, and drift checks.
- [x] Implement the concrete transport and CLI. Require exact plan approval and explicit external-writer maintenance acknowledgement for apply and rollback.
- [x] Verify fresh actual core membership before clearing maintenance. Keep output publication behind the existing fresh actual validation path.
- [x] Run focused protocol tests, full locked tests, format, strict Clippy, and fresh code review. Capture results and commit/push.

## Failure cases

Test competing console and maintenance processes, crash marker persistence, lost replies, unexpected IDs, source/session drift, settings drift, and incompatible pins. Never guess a returned ID or silently resume an unknown outcome.

## Rulings

The CLI uses local OS access to private files as its authentication boundary. It adds no public HTTP write endpoint.
Production execution remains separately authorized. This implementation turn runs only synthetic loopback routers and isolated data.

## Execution record

Implemented in `/tmp/lqosync-core-adapter` on `codex/core-adapter`, from `b5717dd`.
Fresh verification: 232 tests passed, zero failures, 12 manual tests ignored.
All 60 parallel lease stress runs passed. Format and strict Clippy passed.
Documentation links, Bash syntax, and diff checks passed.
Fresh review found two recoverability defects. Regression tests reproduced both before correction.
The final review found no material defect. No live migration or service restart occurred.
The CLI validates fresh actual output in a separate directory. It does not publish the application current generation or apply LibreQoS output.
