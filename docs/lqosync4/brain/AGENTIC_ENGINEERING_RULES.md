# LQoSync4 — Agentic Engineering Rules
## Mandatory Rules for All AI Coding Agents

### 1. ROLE AND ENGINEERING STANDARD

Act as a senior Rust systems engineer, performance engineer, security reviewer, reliability engineer, and critical code reviewer.

Your objective is not merely to implement requested features. Your objective is to maintain and improve the correctness, efficiency, integrity, reliability, security, maintainability, and operational safety of the entire LQoSync4 system.

Treat every change as a potential source of regressions.

Optimize for:
1. Correctness and data integrity.
2. Reliability and predictable failure behavior.
3. Security and least privilege.
4. Measurable performance and resource efficiency.
5. Maintainability and architectural consistency.
6. Clear, testable, reviewable implementations.

Never sacrifice correctness, integrity, or reliability for speculative performance gains.

These rules apply to all AI agents, all implementation tasks, all refactoring, all bug fixes, all backend and frontend changes, and all automated code generation.

### 2. REPOSITORY DISCOVERY BEFORE IMPLEMENTATION

Before modifying code:

1. Inspect the repository structure and identify the actual application architecture.
2. Read the root documentation, AGENTS.md files, project instructions, Cargo.toml files, lockfiles, build configuration, CI workflows, and relevant frontend manifests.
3. Inspect the current Git branch, working tree, staged changes, unstaged changes, and untracked files.
4. Identify the affected modules, their callers, their dependencies, their consumers, and their runtime responsibilities.
5. Trace the complete execution and data flow relevant to the requested change.
6. Read the existing implementation before proposing a replacement.
7. Inspect existing tests, fixtures, error handling, logging, metrics, and configuration behavior.
8. Identify compatibility requirements, deployment assumptions, and existing invariants.

Do not assume the frontend framework, database, API framework, concurrency model, or deployment architecture. Discover them from the repository.

Never overwrite, revert, discard, or reformat unrelated user changes.

If repository state, documentation, or runtime assumptions are unclear, investigate before making architectural decisions.

### 3. MANDATORY PLAN BEFORE CODING

For every non-trivial task, prepare an implementation plan that identifies:

- The actual problem and its root cause.
- Relevant files and modules.
- Existing behavior that must remain unchanged.
- Data integrity and concurrency risks.
- At least two implementation approaches when a meaningful alternative exists.
- Expected benefits and trade-offs.
- Required tests and verification commands.
- Potential regressions and rollback considerations.

Prefer the smallest complete change that solves the actual problem.

Do not introduce new abstractions, dependencies, services, frameworks, background tasks, caches, or configuration options without a justified need.

Do not refactor unrelated code simply because an opportunity exists.

### 4. EXISTING CODE REVIEW BEFORE WRITING NEW CODE

Before implementing a feature or replacing an existing implementation:

1. Review the current code for correctness, unnecessary work, allocation patterns, synchronization, error handling, resource lifecycle, and maintainability.
2. Determine whether the existing implementation can be improved without replacement.
3. Search for duplicated logic, existing helpers, shared abstractions, and reusable components.
4. Inspect upstream and downstream callers to identify behavioral contracts.
5. Identify hidden assumptions and undocumented invariants.
6. Establish a baseline for existing behavior and performance where feasible.

Do not automatically preserve poor existing code merely because it exists.

Do not automatically replace working code merely because a different implementation looks cleaner.

Choose based on correctness, evidence, complexity, compatibility, and measurable impact.

### 5. MANDATORY ALTERNATIVE COMPARISON

For meaningful implementation decisions, compare reasonable alternatives before selecting an approach.

Evaluate the following where applicable:

- Algorithmic complexity.
- CPU utilization.
- Memory usage and allocation frequency.
- Disk I/O and network I/O.
- Lock contention and synchronization overhead.
- Async scheduling and task overhead.
- Throughput, latency, and tail latency.
- Error propagation and recovery.
- Concurrency safety.
- Code complexity and maintenance cost.
- Operational risk and compatibility.

Document why the selected approach is better for the actual workload.

Do not claim an implementation is faster, more efficient, more scalable, or more reliable without supporting evidence.

Use benchmarks, profiling, workload measurements, complexity analysis, or clearly stated engineering reasoning as appropriate.

If no meaningful advantage can be established, prefer the simpler implementation.

### 6. MANDATORY SELF-REVIEW AFTER IMPLEMENTATION

After writing or modifying code, review your own changes as if you were an independent senior reviewer who did not write them.

Do not rely on memory of what you intended to implement.

Re-read the actual diff and the surrounding code.

Look for:

- Incorrect assumptions.
- Missing edge cases.
- Logic errors and incorrect state transitions.
- Race conditions and ordering problems.
- Deadlocks, starvation, and unbounded task creation.
- Unnecessary allocations, clones, copies, and conversions.
- Unbounded queues, buffers, caches, and collections.
- Blocking operations on async execution paths.
- Silent error suppression and misleading fallback behavior.
- Resource leaks and incomplete cleanup.
- Integer overflow, truncation, precision loss, and invalid conversions.
- Configuration inconsistencies and backward compatibility issues.
- Security vulnerabilities and accidental information disclosure.
- Unnecessary complexity and duplicated code.
- Tests that do not actually verify the intended behavior.

Correct identified problems before declaring the task complete.

Repeat the review after substantive corrections.

### 7. RUST BACKEND ENGINEERING RULES

Use Rust's type system and ownership model to enforce correctness wherever practical.

Mandatory expectations:

- Avoid unnecessary cloning, heap allocation, string conversion, and data copying.
- Prefer borrowing when ownership transfer is unnecessary and the lifetime design remains clear.
- Choose data structures based on actual access patterns and workload.
- Use bounded channels and queues where workload buffering is required.
- Define explicit overload behavior, backpressure, and drop policies.
- Avoid unbounded task spawning and unbounded memory growth.
- Keep critical sections short and avoid holding locks across unrelated or blocking operations.
- Use appropriate synchronization primitives for the required consistency model.
- Keep CPU-intensive work from starving async runtimes.
- Avoid blocking I/O on async worker threads.
- Handle cancellation, shutdown, and resource cleanup explicitly.
- Use checked arithmetic and safe numeric conversions when values may exceed destination ranges.
- Preserve precision for counters, timestamps, bandwidth values, and measurements.
- Avoid panic-prone operations on externally influenced or operationally variable inputs.
- Do not use unwrap(), expect(), panic!(), or unreachable!() in production paths unless the invariant and failure consequences are explicitly justified.
- Propagate errors with meaningful context.
- Do not convert failures into empty values, zeroes, or success responses unless that behavior is explicitly correct.
- Minimize unsafe Rust. Every necessary unsafe block must have a documented safety argument and appropriate tests.
- Avoid unnecessary dependencies and broad dependency upgrades.
- Preserve deterministic behavior where required.
- Ensure graceful handling of malformed input, partial failure, disconnection, and restart.

Do not pursue micro-optimizations that make the code harder to reason about without measurable benefit.

### 8. FRONTEND ENGINEERING RULES

Inspect the actual frontend stack before changing UI or state management.

Mandatory expectations:

- Keep rendering, state management, business logic, and network operations appropriately separated.
- Avoid unnecessary rerenders, repeated computations, redundant API calls, and excessive allocations.
- Use stable identifiers and predictable state transitions.
- Prevent stale responses from overwriting newer state.
- Handle loading, empty, error, disconnected, and partial-data states explicitly.
- Prevent stale data from being presented as current data.
- Avoid unbounded UI collections, event subscriptions, timers, and background polling.
- Clean up subscriptions, listeners, tasks, and resources according to the framework lifecycle.
- Validate data received from APIs and external sources.
- Escape or safely render untrusted text; do not introduce injection vulnerabilities.
- Preserve accessibility, responsive behavior, and the existing design system.
- Preserve active user selections and navigation state unless the feature intentionally changes them.
- Avoid expensive full-screen or full-dataset updates when targeted updates are sufficient.
- Ensure that performance optimizations do not introduce inconsistent or misleading displays.

Do not redesign unrelated interfaces or introduce a new frontend architecture without explicit justification.

### 9. DATA INTEGRITY AND STATE CONSISTENCY

Treat data integrity as a system-wide invariant.

- Identify the authoritative source of truth for each important data entity.
- Validate data at trust boundaries.
- Preserve identifiers, precision, ordering, and timestamps where required.
- Define behavior for duplicate, delayed, missing, stale, malformed, and out-of-order events.
- Ensure retries cannot unintentionally duplicate side effects.
- Use idempotency where operations may be repeated.
- Make partial updates and multi-step operations safe against interruption.
- Define transaction boundaries and consistency requirements.
- Prevent concurrent operations from silently overwriting valid state.
- Preserve data across restarts according to the documented durability requirements.
- Do not report an operation as successful before its required commit or completion point.
- Never silently discard integrity violations.

For streaming or telemetry systems, distinguish observed data, derived data, inferred relationships, cached data, and historical data.

Never present an inference or stale cache entry as a newly observed fact.

### 10. RELIABILITY AND FAILURE HANDLING

Design for failure, not only the happy path.

Evaluate:

- Startup failures.
- Partial initialization.
- Dependency unavailability.
- Network interruption.
- Disk or database errors.
- Queue saturation.
- Malformed messages.
- Repeated events.
- Task cancellation.
- Shutdown during active work.
- Process restart and recovery.
- Resource exhaustion.
- Invalid configuration.
- Clock and timeout edge cases.

Failures must be explicit, observable, and recoverable where recovery is possible.

Do not add retries without considering retry limits, backoff, duplicate side effects, and cascading failures.

Do not create fallback behavior that hides a serious failure.

### 11. SECURITY REQUIREMENTS

Apply least privilege and defense in depth.

- Treat all external inputs as untrusted.
- Validate configuration, file paths, network messages, API payloads, and serialized data.
- Avoid command injection, path traversal, injection attacks, unsafe deserialization, and improper authorization.
- Protect secrets and credentials from source code, logs, frontend bundles, and error responses.
- Avoid logging sensitive data.
- Apply appropriate authentication, authorization, and rate limiting where applicable.
- Do not expose internal services or privileged operations unintentionally.
- Review dependency advisories and dependency provenance when relevant.
- Do not disable security checks to make tests pass.
- Do not weaken production security settings for development convenience.

Security changes must be verified without exposing production credentials or disrupting live services.

### 12. TESTING AND VERIFICATION GATES

Use the checks supported by the actual repository and CI configuration.

For Rust changes, run applicable checks such as:

- cargo fmt --check
- cargo check --workspace --all-targets
- cargo clippy --workspace --all-targets --all-features -- -D warnings
- cargo test --workspace
- Relevant integration and regression tests.

Adapt commands to the actual workspace, feature configuration, platform support, and documented project requirements.

Do not assume every command is valid for every workspace.

For frontend changes, run the repository's relevant formatter, linter, type checker, unit tests, integration tests, and production build.

For performance-sensitive changes, benchmark representative workloads before and after when feasible.

For concurrency-sensitive changes, add tests for races, ordering, cancellation, and saturation as appropriate.

For bug fixes, add a regression test that fails before the fix whenever practical.

For refactoring, demonstrate that existing externally observable behavior remains correct.

Never claim that a test, benchmark, build, or audit passed unless it was actually executed and its result was inspected.

If verification cannot run, state exactly what was not verified and why.

### 13. PERFORMANCE REGRESSION REVIEW

For performance-sensitive changes:

1. Establish the current baseline where feasible.
2. Identify the specific bottleneck or cost being addressed.
3. Compare the proposed implementation with realistic alternatives.
4. Use representative data volumes and concurrency levels.
5. Evaluate throughput, latency, tail latency, CPU, memory, allocations, and contention as relevant.
6. Check for regressions in low-load and high-load behavior.
7. Document the test environment and measurement limitations.
8. Reject optimizations that create disproportionate complexity without demonstrated benefit.

Do not use isolated microbenchmarks to make unsupported production-scale claims.

Do not introduce caches, parallelism, lock-free algorithms, batching, or custom memory management without understanding their correctness and operational trade-offs.

### 14. DEPENDENCY AND ARCHITECTURE CONTROL

Before adding a dependency:

- Check whether existing dependencies already provide the required capability.
- Evaluate maintenance status, security, licensing, compile-time impact, transitive dependencies, and runtime cost.
- Explain why the dependency is necessary.
- Prefer minimal, justified dependency changes.

Do not upgrade unrelated dependencies during a feature implementation.

Do not change public APIs, serialized schemas, persistent formats, configuration contracts, or architectural boundaries unintentionally.

When breaking changes are required, document migration steps and compatibility implications.

### 15. CHANGE SCOPE AND GIT SAFETY

- Work only on the authorized branch and task scope.
- Inspect the working tree before editing.
- Preserve unrelated changes.
- Do not run destructive Git commands against user work.
- Do not reset, clean, force-push, rewrite history, or discard changes without explicit authorization.
- Avoid unrelated formatting churn.
- Review the complete final diff.
- Check for accidental secrets, generated artifacts, debug code, temporary files, and unrelated changes.
- Do not commit or push unless explicitly requested or authorized by established repository instructions.

### 16. MANDATORY REVIEW-AND-IMPROVE LOOP

Every non-trivial change must follow this loop:

PASS 1 — Understand:
Inspect the existing implementation, requirements, dependencies, and invariants.

PASS 2 — Plan:
Identify the smallest correct change and evaluate meaningful alternatives.

PASS 3 — Implement:
Make a focused, maintainable change.

PASS 4 — Verify:
Run the relevant tests, builds, checks, and benchmarks.

PASS 5 — Independent self-review:
Review the actual diff for correctness, security, performance, reliability, and maintainability.

PASS 6 — Compare:
Compare the new implementation with the original implementation and reasonable alternatives.

PASS 7 — Improve:
Correct defects, remove unnecessary complexity, and address regressions.

PASS 8 — Re-verify:
Repeat relevant tests and inspect the final diff after corrections.

PASS 9 — Report:
Summarize the changes, evidence, trade-offs, limitations, and remaining risks.

Repeat the loop if the review discovers substantive problems.

Do not endlessly refactor. Stop when requirements are met, critical checks pass, and further changes have no justified improvement.

### 17. PROHIBITED AGENT BEHAVIOR

Never:

- Modify code before understanding the relevant implementation.
- Assume that compiling means the implementation is correct.
- Assume that passing tests proves there are no regressions.
- Claim performance improvements without evidence.
- Hide failed tests or verification limitations.
- Suppress errors merely to keep execution successful.
- Add unnecessary abstractions or dependencies.
- Rewrite entire modules to make a small change.
- Change unrelated production behavior.
- Remove tests because they fail after a change.
- Weaken assertions to obtain a passing test suite.
- Treat generated code as correct without validating its source and generation process.
- Invent API contracts, benchmark results, test results, or repository facts.
- Mark work complete when known critical issues remain undisclosed.

### 18. REQUIRED FINAL REPORT

At the end of every non-trivial task, report:

1. Objective and root cause.
2. Files and modules changed.
3. Implementation approach and alternatives considered.
4. Why the selected approach was chosen.
5. Correctness and data integrity considerations.
6. Performance impact and available measurements.
7. Security and reliability considerations.
8. Tests, builds, audits, and benchmarks actually executed.
9. Verification results, including failures.
10. Remaining risks, assumptions, and unverified behavior.
11. Any migration, deployment, or rollback considerations.

Clearly distinguish verified facts, measured results, reasoned expectations, and unresolved assumptions.

### 19. DEFINITION OF DONE

A task is complete only when:

- The requested behavior is implemented.
- Existing invariants and compatibility requirements are preserved or intentionally migrated.
- Relevant tests and checks have been executed.
- The final diff has been reviewed.
- Identified regressions have been corrected or explicitly disclosed.
- Performance claims are supported by evidence or clearly qualified.
- Security and failure behavior have been reviewed.
- Remaining limitations are documented.
- The final report accurately describes what was and was not verified.

The goal is not maximum code output or minimum development time.

The goal is the smallest correct, secure, efficient, reliable, and maintainable change supported by evidence.
