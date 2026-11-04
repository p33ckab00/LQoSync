## Production-Ready Completion Gate — No Half-Baked Code

### 1. Non-Negotiable Standard

Every implementation MUST be fully engineered, functionally complete, performance-conscious, reliable, maintainable, and verified before being declared complete.

The objective is not to merely produce code that compiles, passes a superficial test, or demonstrates a working happy path. The objective is to deliver a complete, integrated, production-quality implementation with no known functional gaps, unfinished logic, missing integration, or avoidable technical debt introduced by the change.

**No half-baked implementations. No fake completeness. No "it compiles, therefore it is done."**

### 2. End-to-End Functional Completeness

For every feature, trace the entire execution path:

- User interface or external input
- Input validation and normalization
- Frontend-to-backend communication
- Backend request handling and business logic
- Database reads, writes, transactions, and consistency
- Background jobs, events, queues, and asynchronous processing where applicable
- Error propagation and user-visible feedback
- State synchronization and refresh behavior
- Logging, diagnostics, and observability
- Output delivery and final user-visible result

Do not implement only one layer while leaving the remaining layers disconnected, mocked, stubbed, or dependent on undocumented manual steps.

A feature is incomplete if its UI exists but its backend behavior is missing, its API exists but is not integrated, its database schema is incomplete, its state becomes inconsistent, or its failure behavior is undefined.

### 3. No Placeholders or Fake Implementations

Unless explicitly requested for a prototype, proof of concept, or documented staged delivery, DO NOT introduce:

- TODO/FIXME markers for required functionality
- Empty function bodies or dummy return values
- Hardcoded responses pretending to be real data
- Mock implementations in production execution paths
- Buttons or UI controls that do nothing
- API endpoints that return success without performing the promised operation
- Error handlers that silently discard failures
- Arbitrary sleeps, retries, or timeouts used to hide race conditions
- Disabled validation or weakened checks to make tests pass
- Unimplemented branches hidden behind default values
- Silent fallbacks that conceal configuration or infrastructure failures
- Temporary compatibility hacks without an explicit removal plan

If a dependency or external system prevents completion, state the exact blocker, its impact, and the remaining work. Never represent blocked or partial functionality as complete.

### 4. Performance Is a Design Requirement

Performance MUST be considered during design, implementation, and review—not postponed until a problem appears.

For every performance-sensitive change:

- Identify the likely bottleneck and expected workload.
- Avoid unnecessary allocations, copies, cloning, serialization, and deserialization.
- Avoid blocking operations inside asynchronous execution paths.
- Avoid holding locks across slow I/O or unrelated operations.
- Review concurrency, backpressure, queue growth, memory usage, and task lifecycle.
- Prevent unnecessary database queries, repeated computation, and inefficient data structures.
- Consider throughput, latency, tail latency, CPU utilization, memory consumption, and contention where relevant.
- Preserve bounded resource usage under sustained load.
- Use profiling, benchmarks, or representative workload tests when the change warrants them.
- Compare the new implementation against an existing baseline whenever measurable.

Do not sacrifice correctness, integrity, or reliability for a speculative performance improvement. Prefer measurable gains and evidence-based engineering decisions over clever but unverified optimizations.

### 5. No Hidden Functional Gaps

Before completion, explicitly inspect:

- Normal operation
- Empty, missing, malformed, and invalid inputs
- Boundary values and maximum supported sizes
- Concurrent requests and duplicate operations
- Partial failures and interrupted operations
- Network timeouts, unavailable services, and connection failures
- Database errors and transaction rollback
- Restart and recovery behavior where applicable
- Configuration errors and missing environment variables
- Resource exhaustion and sustained workload behavior
- Permission boundaries and unauthorized operations
- State transitions, retries, and idempotency where relevant

Not every scenario requires a separate test, but every applicable scenario must be considered and appropriately handled or documented.

Never assume that untested behavior is correct merely because the primary workflow succeeds.

### 6. Rust Backend Quality Gate

For Rust code:

- Preserve ownership, borrowing, lifetime, and type-safety guarantees instead of bypassing them unnecessarily.
- Minimize unnecessary `.clone()`, allocations, locking, and blocking work.
- Handle `Result` and `Option` deliberately; do not suppress errors to achieve a clean build.
- Avoid unjustified `.unwrap()`, `.expect()`, and panic-based control flow in production paths.
- Use structured, contextual errors that retain useful diagnostic information.
- Ensure asynchronous tasks have clear cancellation, timeout, shutdown, and resource-lifecycle behavior where applicable.
- Verify channel capacity, backpressure, task supervision, and concurrency assumptions where relevant.
- Ensure database operations and state mutations preserve invariants.
- Keep interfaces, modules, and abstractions coherent with the existing architecture.
- Do not introduce unsafe code without a compelling, documented justification and focused safety review.

### 7. Frontend and Integration Quality Gate

For the frontend:

- Every implemented control must perform its intended action.
- Loading, success, empty, error, and retry states must be handled where applicable.
- Prevent stale data, duplicate submissions, race conditions, and inconsistent UI state.
- Validate user input and display useful error messages.
- Verify API request and response contracts against the actual backend.
- Handle authentication, authorization, expired sessions, and unavailable services where applicable.
- Ensure that navigation, refresh, state persistence, and recovery work as intended.
- Avoid fake data or hardcoded values in production paths unless explicitly designed as configuration or fixtures.
- Verify critical workflows through integration or browser-level tests where available.

A visually complete interface is not functionally complete unless its underlying operations work.

### 8. Mandatory Review → Improve → Verify Loop

Every non-trivial implementation MUST go through this cycle:

1. **Implement:** Build the smallest complete solution consistent with the requirements.
2. **Review:** Re-read the entire changed code and inspect the actual Git diff.
3. **Challenge:** Identify unnecessary complexity, missing behavior, weak error handling, performance risks, security gaps, and architectural inconsistencies.
4. **Compare:** Evaluate the implementation against the previous behavior and at least one reasonable alternative when the trade-off is material.
5. **Improve:** Fix every confirmed defect and worthwhile improvement within scope.
6. **Test:** Run relevant formatting, compilation, linting, unit, integration, and regression checks.
7. **Stress the assumptions:** Test critical boundaries, failure paths, and concurrency behavior where applicable.
8. **Re-review:** Inspect the final diff again after all fixes.
9. **Re-verify:** Re-run the affected checks after the final modification.
10. **Report:** State what was implemented, what was verified, what remains unverified, and any known limitations.

A test run performed before the final code change does not verify the final implementation.

### 9. No Completion Claims Without Evidence

The agent MUST distinguish between:

- Implemented
- Compiled
- Linted
- Unit-tested
- Integration-tested
- End-to-end tested
- Benchmarked
- Validated in a production-like environment

These terms are not interchangeable.

Never claim that a feature is fully working, production-ready, performant, secure, or reliable unless the available evidence supports the claim.

If a test cannot be run, explain why. If a benchmark is unavailable, do not invent performance results. If a behavior depends on the production environment, identify that dependency explicitly.

### 10. Definition of Done — Fully Cooked, Not Merely Compiling

A change may be declared COMPLETE only when all applicable conditions are satisfied:

- [ ] Requirements and acceptance criteria are met.
- [ ] The implementation is complete across all affected layers.
- [ ] No required functionality is left as a placeholder or mock.
- [ ] Existing behavior and compatibility are preserved, or intentional changes are documented.
- [ ] Data integrity, state transitions, and failure handling are addressed.
- [ ] Performance and resource usage have been considered.
- [ ] Security and authorization implications have been reviewed.
- [ ] Relevant formatting, compilation, linting, and tests pass.
- [ ] Regression risks have been assessed.
- [ ] The final Git diff has been independently reviewed by the agent after implementation.
- [ ] All confirmed defects introduced by the change have been fixed.
- [ ] Tests have been rerun after the final modification.
- [ ] Known limitations and unverified assumptions are explicitly disclosed.
- [ ] Documentation and operational instructions are updated where needed.

**Final principle:** Deliver finished engineering, not a code-shaped draft. Optimize for correctness, completeness, measurable performance, operational reliability, and long-term maintainability. If a task cannot be completed within the available scope, deliver a clearly bounded, honest partial result instead of disguising unfinished work as production-ready software.
