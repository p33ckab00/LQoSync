# Controlled compiler and runtime writer owner

## Goal

Bind manager compilation to exact current library/configuration bytes.
Reuse the runner instance lock and receipt tracker. Prevent the legacy writer from resuming after manager ownership.

## Acceptance

- [x] Share a typed instance-lock guard with the registry. Reject a different state directory.
- [x] Compile the exact fresh pair in a private workspace with bounded time, input, and artifact sizes.
- [x] Deny compiler socket operations and writes outside its private workspace. Fail if the required kernel controls are unavailable.
- [x] Check exact compiled policy and recheck source/configuration identity before publication.
- [x] Bind compiler identity to the receipt and its current post-write proof.
- [x] Persist writer ownership before the first manager write. Reuse one tracker and reject legacy fallback after restart.
- [x] Prove failure, restart, deadline, drift, lock, and write-boundary behavior. Run the full suite and static checks.

## Boundaries

Keep the service loop on its current path until activation gates pass.
Add an explicit runner API for manager publication. Do not deploy, activate the live manager, or change routing.
Reuse current authorization, registry commit, pair admission, atomic writes, and native acknowledgement checks.
Do not add a dependency or a second process writer.

## Recovery

Compiler failure leaves the pair and registry unchanged.
A durable manager ownership marker prevents implicit legacy fallback, also after a partial publication.
Corrupt ownership or registry state blocks the writer. Recovery remains necessary for pending receipts.

## Results

- The full suite passes: 1,069 tests passed, zero failed, and 11 ignored.
- The suite includes 337 topology tests and 19 new active tests.
- Both optional installed-library tests pass when explicitly selected.
- The scratch compiler test preserves the exact pair within the kernel controls.
- The public runner API test publishes active and disconnected batches into temporary directories.
- Format, Clippy with warnings denied, and whitespace checks pass.
- Native config migration requires writes within the private workspace. The receipt records the normalized effective config digest.
- The compiler preserves original config and library bytes. It rejects current-source drift before publication and acknowledgement acceptance.
- Regression tests cover replaced lock paths, unbound publication, ownership restart, socket denial, binary copies, and receipt identity.
- Author review only. No independent reviewer, production benchmark, live apply, or deployment occurred.

## Remaining activation gates

Connect configured source reads and identity authorization to the service loop.
Complete legacy policy parity and native planner bounds before activation.
Require independent identity enrollment and live DPDK proof.
The current API does not select manager mode automatically.
TUI, schedules, and full corrupt-store reconstruction remain separate tasks.
