# Service API implementation plan (Phase 2a)

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a root-only JSON-lines Unix socket API to the running service with `status`, `registry.get`, and `policy.commit`, plus `lqosync3 api status`.

**Architecture:** An accept task validates lines and peers, then sends each request through a bounded channel to the main task, which owns the `Runner`. The main loop keeps one sleep deadline for each cycle and handles requests while it waits. A shared client module serves the CLI now and the TUI in 2b.

**Tech Stack:** Rust 2024, Tokio (`net`, `sync`, `time` already enabled), serde/serde_json, anyhow. No new dependencies. MSRV 1.89.

**Spec:** `docs/superpowers/specs/2026-11-18-topology-service-api-design.md`

## Global Constraints

- Do not stop, restart, deploy, or reconfigure `lqosync3.service`. Do not touch `/etc/lqosync3`, `/var/lib/lqosync3`, `/opt/libreqos/src`, or routers.
- Tests use temporary directories and the fake connector only. Socket paths in tests stay under the temporary directory.
- No new dependencies. Keep `cargo +1.89 build --locked` working.
- Gate for every task: `cargo fmt --check`, `cargo clippy --all-targets --locked -- -D warnings`, `cargo test --locked`.
- Run cargo with `nice -n 19` and `CARGO_BUILD_JOBS=2`. The host runs production LibreQoS.
- Work in a separate worktree on branch `topology-service-api` from `lqosync3`. Do not edit `docs/OPERATIONS.md` (owner edits are uncommitted in the main worktree).
- Persisted text follows ASD-STE100.
- Wire format: one JSON object per line, `"v": 1`, request `{v, id, op, args}`, response `{v, id, ok, result}` or `{v, id, ok: false, error: {code, message}}`.
- Error codes, exactly: `bad_request`, `unsupported_version`, `unknown_op`, `unauthorized`, `conflict`, `invalid`, `busy`, `internal`.
- Limits: request line 1 MiB, response line 16 MiB, connection idle timeout 60 s, request channel capacity 16, client default timeout 120 s.
- Socket: `<runtime.state_dir>/api.sock`, mode `0600`.

## Review Focus

1. A client request that arrives during a long cycle: it waits and gets its answer after the cycle, within the 120 s client timeout, and the next cycle still starts at its original deadline. Test in Task 5.
2. A peer that is neither uid 0 nor the service effective uid: `unauthorized` and the connection closes before any request is read. Test in Task 3.
3. Two clients commit with the same `expected_revision`: the first succeeds, the second gets `conflict`, and a third request still works (the store reopens). Test in Task 4.
4. A service crash leaves `api.sock`: the next start replaces it only after `Runner::lock()` succeeds; a regular file at that path stops startup. Test in Task 3.
5. A `registry.get` response above 16 MiB: the server answers `internal` with a size message, not a truncated line. Test in Task 2.

---

### Task 1: Typed store commit errors

**Files:**
- Modify: `src/topology_manager/store.rs` (`commit_inner` near line 236, `preview` near line 293)
- Test: `src/topology_manager/store.rs` tests module

**Interfaces:**
- Produces: `pub struct RevisionConflict;` and `pub struct InvalidPolicy(pub String);` in `store.rs`, both `Debug + Display + std::error::Error`.
- Produces: `commit` returns an `anyhow::Error` whose root cause downcasts to `RevisionConflict` for both revision checks, and to `InvalidPolicy` for candidate validation and preview rule failures. Other failures keep their current form.

- [ ] **Step 1: Write the failing tests:** `commit_with_a_stale_revision_downcasts_to_revision_conflict`, `commit_with_an_invalid_candidate_downcasts_to_invalid_policy`, `the_store_still_commits_after_a_conflict`.
- [ ] **Step 2: Run** `cargo test --locked topology_manager::store` — Expected: FAIL (types missing).
- [ ] **Step 3: Implement.** Replace the two revision `ensure!` calls with `bail!(RevisionConflict)`. Wrap validation and `preview` errors with `InvalidPolicy(format!("{e:#}"))`. Keep every message text.
- [ ] **Step 4: Run** the tests and the full gate — Expected: PASS.
- [ ] **Step 5: Commit** `feat(topology): add typed registry commit errors`.

### Task 2: Protocol and framing

**Files:**
- Create: `src/api/mod.rs`, `src/api/protocol.rs` (register `pub mod api;` in `src/lib.rs`)
- Test: `src/api/protocol.rs` tests module

**Interfaces:**
- Produces: `pub struct Request { pub v: u32, pub id: String, pub op: String, pub args: serde_json::Value }` (serde, `deny_unknown_fields`, `args` defaults to `{}`).
- Produces: `pub enum ErrorCode` (serde `snake_case`, the eight codes) and `pub struct ApiError { pub code: ErrorCode, pub message: String }`.
- Produces: `pub fn response_ok(id: &str, result: Value) -> String` and `pub fn response_err(id: &str, error: &ApiError) -> String`. Both end with `\n`. A line above 16 MiB becomes `response_err(id, internal, "response exceeds 16 MiB")`.
- Produces: `pub fn parse_line(line: &[u8]) -> Result<Request, (String, ApiError)>`. The `String` is the request id when it can be read, else `""`. Version other than 1 gives `unsupported_version`.
- Produces: `pub async fn read_line<R: AsyncBufRead + Unpin>(r: &mut R, max: usize) -> std::io::Result<Option<Vec<u8>>>` that returns an error for a line above `max` without reading it all into memory.

- [ ] **Step 1: Write the failing tests:** `a_valid_request_parses_with_default_args`, `bad_json_is_bad_request`, `version_2_is_unsupported_version`, `unknown_fields_are_bad_request`, `an_oversize_line_is_an_error`, `an_oversize_response_becomes_an_internal_error` (Review Focus 5), `responses_end_with_one_newline`.
- [ ] **Step 2: Run** `cargo test --locked api::protocol` — Expected: FAIL (module missing).
- [ ] **Step 3: Implement** the types and functions.
- [ ] **Step 4: Run** the tests and the full gate — Expected: PASS.
- [ ] **Step 5: Commit** `feat(api): add the JSON-lines protocol`.

### Task 3: Socket server, peer rule, and lifecycle

**Files:**
- Create: `src/api/server.rs`
- Test: `src/api/server.rs` tests module

**Interfaces:**
- Consumes: Task 2 protocol.
- Produces: `pub struct Job { pub request: Request, pub reply: tokio::sync::oneshot::Sender<String> }`.
- Produces: `pub fn peer_allowed(peer_uid: u32, service_euid: u32) -> bool` (true for 0 or equal uids).
- Produces: `pub struct SocketGuard { path: PathBuf }` that removes the socket file on `Drop`.
- Produces: `pub fn bind(state_dir: &Path, _lock_held: &crate::files::InstanceLock) -> anyhow::Result<(tokio::net::UnixListener, SocketGuard)>`. It removes a stale socket file, refuses a non-socket file, binds, and sets mode `0600`. The lock argument proves ownership at the type level.
- Produces: `pub fn spawn(listener: UnixListener, jobs: tokio::sync::mpsc::Sender<Job>) -> tokio::task::JoinHandle<()>`. Each connection: peer check, then a loop of `read_line` (1 MiB) and `parse_line`, then `try_send`. A full channel answers `busy`. Idle timeout 60 s.
- Add `pub(crate) fn directory(&self) -> &Path` access to `InstanceLock` only if `bind` needs it.

- [ ] **Step 1: Write the failing tests:** `peer_rule_accepts_root_and_the_service_uid_only` (Review Focus 2), `bind_replaces_a_stale_socket` (Review Focus 4), `bind_refuses_a_regular_file_at_the_socket_path`, `the_socket_has_mode_0600`, `the_guard_removes_the_socket_on_drop`, `a_request_reaches_the_job_channel_and_the_reply_returns`, `a_full_channel_answers_busy`, `an_idle_connection_closes_after_the_timeout` (paused time).
- [ ] **Step 2: Run** `cargo test --locked api::server` — Expected: FAIL.
- [ ] **Step 3: Implement.** Read the service euid with `libc`-free `std::os::unix::fs::MetadataExt` on `/proc/self` (`metadata("/proc/self")?.uid()`). Use `UnixStream::peer_cred()`.
- [ ] **Step 4: Run** the tests and the full gate — Expected: PASS.
- [ ] **Step 5: Commit** `feat(api): add the root-only socket server`.

### Task 4: Runner operations

**Files:**
- Modify: `src/runner.rs` (near `install_topology_policy`)
- Create: `src/api/dispatch.rs`
- Test: `src/runner/tests/api.rs` (register beside `enrollment.rs`)

**Interfaces:**
- Consumes: Task 1 `RevisionConflict`, `InvalidPolicy`; Task 2 `Request`, `ApiError`, `response_ok`, `response_err`.
- Produces in `runner.rs`:
  - `pub fn api_status(&mut self) -> anyhow::Result<serde_json::Value>`: fields `version` (`crate::VERSION`), `mode`, `registry_revision` (number or `null`), `enrolled` (bool), `last_cycle_started` (seconds or `null`).
  - `pub fn api_registry(&mut self) -> anyhow::Result<Option<RegistryDocument>>`.
  - `pub fn api_policy_commit(&mut self, expected_revision: u64, catalog: TopologyCatalog, source_settings: SourceSettings) -> anyhow::Result<u64>`.
  - Each calls `self.lock()` first. Reads use a presence check of `topology-manager/registry.json` and do not create the store. After any failed commit, set `self.topology_store = None`.
- Produces in `dispatch.rs`: `pub fn handle<C: Connector>(runner: &mut Runner<C>, request: &Request) -> Result<String, LostLock>` that maps errors: `RevisionConflict` to `conflict`, `InvalidPolicy` to `invalid`, bad args to `bad_request`, unknown op to `unknown_op`, a lock failure to `internal` plus `Err(LostLock)`, other errors to `internal`.

- [ ] **Step 1: Write the failing tests:** `status_without_a_registry_reports_null_and_creates_nothing`, `registry_get_without_a_registry_is_null`, `status_after_install_reports_the_revision`, `policy_commit_succeeds_and_returns_the_new_revision`, `two_commits_with_one_revision_give_conflict_then_the_store_works` (Review Focus 3), `an_invalid_catalog_gives_invalid`, `unknown_op_and_bad_args_are_reported`, `a_lost_lock_answers_internal_and_returns_lost_lock`.
- [ ] **Step 2: Run** `cargo test --locked runner::tests::api` — Expected: FAIL.
- [ ] **Step 3: Implement.** Reuse `open_topology_store` for commits. Record `last_cycle_started` at the start of `Runner::cycle`.
- [ ] **Step 4: Run** the tests and the full gate — Expected: PASS.
- [ ] **Step 5: Commit** `feat(api): add status, registry, and policy commit operations`.

### Task 5: Main loop integration

**Files:**
- Create: `src/api/wait.rs`
- Modify: `src/main.rs` (`run`, near lines 165-235)
- Test: `src/api/wait.rs` tests module

**Interfaces:**
- Consumes: Task 3 `bind`, `spawn`, `Job`; Task 4 `handle`.
- Produces: `pub async fn serve_until<F>(deadline: tokio::time::Instant, jobs: &mut tokio::sync::mpsc::Receiver<Job>, handle: F) -> Result<(), LostLock>` where `F: FnMut(&Request) -> Result<String, LostLock>`. It returns at `deadline` and answers each job in between.
- `run` binds the socket after `runner.lock()`, keeps the `SocketGuard` until return, computes the deadline once after each cycle, and replaces `_ = sleep => {}` with `r = serve_until(deadline, &mut jobs, |req| dispatch::handle(&mut runner, req)) => r?`. Signal arms stay unchanged.

- [ ] **Step 1: Write the failing tests:** `jobs_are_answered_and_the_deadline_does_not_move` (paused time: deadline 20 s, a job at 5 s, returns at 20 s, Review Focus 1), `a_job_queued_during_a_cycle_is_answered_after_it`, `a_lost_lock_stops_serving`.
- [ ] **Step 2: Run** `cargo test --locked api::wait` — Expected: FAIL.
- [ ] **Step 3: Implement** `serve_until` and the `run` changes. Build the binary with `cargo build --locked`.
- [ ] **Step 4: Run** the tests and the full gate — Expected: PASS.
- [ ] **Step 5: Commit** `feat(api): serve API requests between service cycles`.

### Task 6: Client and `lqosync3 api status`

**Files:**
- Create: `src/api/client.rs`
- Modify: `src/cli.rs` (`Command::Api(ApiCommand)`, `ApiCommand::Status`, parse, `USAGE`), `src/main.rs` (dispatch)
- Test: `src/api/client.rs` tests module, `src/cli.rs` tests

**Interfaces:**
- Produces: `pub async fn request(state_dir: &Path, op: &str, args: Value, timeout: Duration) -> Result<Value, ClientError>` with `pub enum ClientError { NotRunning, Timeout, Api(ApiError), Protocol(String) }`. `NotRunning` covers a missing socket and a refused connection.
- `lqosync3 api status` prints the result as pretty JSON. Exit 1 on any `ClientError`, with the message `lqosync3.service does not answer on <path>` for `NotRunning`.

- [ ] **Step 1: Write the failing tests:** `api_status_parses`, `client_against_a_served_runner_gets_status` (spawn the Task 3 server and a Task 4 handler in the test), `a_missing_socket_is_not_running`, `an_api_error_keeps_its_code`.
- [ ] **Step 2: Run** `cargo test --locked api::client` and `cargo test --locked cli::` — Expected: FAIL.
- [ ] **Step 3: Implement.** Default timeout 120 s.
- [ ] **Step 4: Run** the tests and the full gate — Expected: PASS.
- [ ] **Step 5: Commit** `feat(cli): add lqosync3 api status`.

### Task 7: Records and final verification

**Files:**
- Modify: `docs/design/TOPOLOGY_MANAGER_IMPLEMENTATION.md` (new section "Service API"), `brain/30-knowledge/topology-manager-status.md`, `docs/operations/TOPOLOGY_ENROLLMENT.md` (add `lqosync3 api status` as a service check in step 8)
- Create: `brain/10-sessions/2026-11-18-topology-service-api.md`

- [ ] **Step 1: Run** `cargo fmt --check && cargo clippy --all-targets --locked -- -D warnings && cargo test --locked && git diff --check` — Expected: all pass.
- [ ] **Step 2: Update** the records with results and the 2b next step.
- [ ] **Step 3: Commit** `docs(api): record the service API`.
