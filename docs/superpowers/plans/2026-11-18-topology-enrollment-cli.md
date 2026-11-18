# Topology Manager enrollment CLI implementation plan (Phase 1)

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Give the operator a CLI path from the current legacy deployment to an enrolled topology registry, with PPP-name and DHCP-MAC identity rules.

**Architecture:** Add two `ReferenceAuthority` rules, a per-account DHCP MAC conflict, and a dropped-rows manifest category in the backend.
Add a policy proposer that mirrors the legacy configuration, and a mapping proposer that suggests row and node mappings for operator confirmation.
Add `Runner` entry points and `lqosync3 topology ...` commands. Enrollment installs policy only. The legacy loop keeps writing the live files.

**Tech Stack:** Rust 2024, Tokio, serde/serde_json, anyhow. No new dependencies. MSRV 1.89.

**Spec:** `docs/superpowers/specs/2026-11-18-topology-enrollment-cli-design.md`

**Roadmap:** Phases 2–6 in the spec get their own design and plan after this plan is complete.

## Global Constraints

- Do not stop, restart, deploy, or reconfigure `lqosync3.service`. Do not write to routers. Do not write `/opt/libreqos/src` or `/etc/lqosync3`.
- All tests use temporary directories, fixtures, `FakeConnector`, or `tests/mock_router.rs`. Production files may be read only for fixture shape, never copied with subscriber data into the repo.
- No new dependencies. Keep `cargo +1.89 build --locked` working.
- Gate for every task: `cargo fmt --check`, `cargo clippy --all-targets --locked -- -D warnings`, `cargo test --locked`.
- Enrollment installs policy only. No shaping output, no reload request, no writer claim, no service-loop activation.
- Never map identity by IP. Never apply a suggested mapping without `confirmed: true`.
- `check` and `policy propose` take no instance lock and write no file except the requested output file.
- Persisted text (docs, comments, commits) follows ASD-STE100.
- Work on branch `lqosync3` in `/home/pi/LQoSync3.0`. Stage only task files. Unrelated edits in the worktree (HANDOFF, OPERATIONS, Home, 2026-11-16 notes) stay unstaged.

## Review Focus

1. A selected DHCP scope with bound leases that are not in the legacy files (production `ipoevlan10`): `check` must report them as "enters after activation" and name the scope, and `enroll` must stop, not adopt them. Test in Task 6.
2. A PPP secret name with spaces, `@`, or non-ASCII characters: the `account_ref` must encode it without collision. Test in Task 1.
3. Two DHCP leases with the same MAC on one selected server (cloned CPE): a critical `DuplicateReference` for both accounts. Test in Task 1.
4. Data changes between `propose` and `enroll` (reconnect with new IP, new lease): `enroll` stops with a drift report and keeps confirmed decisions on the next `propose`. Test in Task 5.
5. `enroll` while another process holds the instance lock: a clear message that names `lqosync3.service`, exit code non-zero, no file change. Test in Task 7.

---

### Task 1: Name and MAC identity rules

**Files:**
- Modify: `src/topology_manager/identity.rs` (enum `ReferenceAuthority` near line 16, authorization near line 319, `reference()` near line 453)
- Modify: `src/topology_manager/settings.rs` (selection validation)
- Test: `src/topology_manager/identity_tests.rs`, `src/topology_manager/settings_tests.rs`

**Interfaces:**
- Produces: `ReferenceAuthority::PppSecretName` and `ReferenceAuthority::DhcpClientMac` (serde snake_case: `ppp_secret_name`, `dhcp_client_mac`).
- Produces: `account_ref` forms `ppp-name/v1/<encoded name>` and `dhcp-mac/v1/<aa:bb:cc:dd:ee:ff>`.
- Produces: `pub(crate) fn encode_name(name: &str) -> String` in `identity.rs`. Bytes outside `[A-Za-z0-9._-]` become `%XX` (uppercase hex). `%` itself is encoded.

- [ ] **Step 1: Write the failing tests** in `identity_tests.rs`:
  - `ppp_secret_name_authorizes_by_name`: PPP account `LocalAccount { name: "juan dela cruz" }` gets `account_ref == "ppp-name/v1/juan%20dela%20cruz"`.
  - `ppp_names_that_differ_only_by_encoding_do_not_collide`: names `"a b"` and `"a%20b"` give different refs.
  - `duplicate_ppp_name_in_one_scope_is_critical`: two accounts with one name give `IdentityIssue::DuplicateReference` for both.
  - `same_name_on_two_routers_stays_distinct`: no conflict, two identities.
  - `dhcp_client_mac_authorizes_by_lease_mac`: lease MAC `AA-BB-CC-00-11-22` gives `"dhcp-mac/v1/aa:bb:cc:00:11:22"`.
  - `duplicate_mac_on_one_dhcp_server_is_critical`.
  - `lease_without_mac_is_missing_reference`: `Endpoint.mac == None` gives `IdentityIssue::MissingReference` for that account only; other accounts still authorize.
  - `comment_marker_is_ignored_by_name_and_mac_rules`: a `[lqosync-id:x]` comment does not change the ref.
  - In `settings_tests.rs`: `ppp_secret_name_requires_ppp_scope` and `dhcp_client_mac_requires_dhcp_scope` reject the wrong `SourceKind`.
- [ ] **Step 2: Run** `cargo test --locked topology_manager::identity` and `cargo test --locked topology_manager::settings` — Expected: FAIL (unknown variants).
- [ ] **Step 3: Implement.** In the authorization loop, select the reference source by `selection.authority`: comment marker (existing), `LocalAccount.name`, or the MAC of the single endpoint. A DHCP account with more than one distinct MAC gives `InvalidReference`. Keep the existing `claimed` duplicate check for all rules.
- [ ] **Step 4: Run** the same tests — Expected: PASS. Run the full gate.
- [ ] **Step 5: Commit** `feat(topology): add PPP name and DHCP MAC identity rules`.

### Task 2: DHCP lease without a valid MAC fails only that account

**Files:**
- Modify: `src/topology_manager/normalize.rs` (`endpoint()` near line 289, `mac()` near line 278)
- Test: `src/topology_manager/normalize_tests.rs`

**Interfaces:**
- Produces: for `Table::DhcpLeases`, a missing, malformed, zero, or multicast `active-mac-address` gives `Endpoint { mac: None, .. }` instead of a read error. Other tables keep current behavior.

- [ ] **Step 1: Write the failing tests:** `dhcp_lease_without_mac_keeps_the_router_read`, `dhcp_lease_with_multicast_mac_has_no_mac`, `hotspot_host_without_mac_still_fails_the_read` (regression).
- [ ] **Step 2: Run** `cargo test --locked topology_manager::normalize` — Expected: the two DHCP tests FAIL.
- [ ] **Step 3: Implement** in `endpoint()`. Then search every reader of `Endpoint.mac` (`rg "\.mac\b" src/topology_manager`) and confirm that `None` for DHCP gives an empty MAC column in the published CSV, not a panic or an error.
- [ ] **Step 4: Run** tests and the full gate — Expected: PASS.
- [ ] **Step 5: Commit** `fix(topology): keep DHCP reads when one lease has no valid MAC`.

### Task 3: Dropped rows in the migration manifest

**Files:**
- Modify: `src/topology_manager/migration.rs` (`MigrationManifest` line 33, classification near line 197)
- Modify: `src/topology_manager/legacy_static.rs` (`adopt`, `StaticAdoption`)
- Test: `src/topology_manager/migration_tests.rs`, `src/topology_manager/legacy_static_tests.rs`

**Interfaces:**
- Produces: `MigrationManifest.dropped_rows: Vec<RowIdentity>` with `#[serde(default)]`.
- Produces: `StaticAdoption.dropped: Vec<RowIdentity>` (public read access) for the enroll report.
- Produces: `ParityIssue::DroppedRow { circuit_id, device_id }` is not a blocking issue; it is reported.

- [ ] **Step 1: Write the failing tests:** `dropped_row_is_reported_and_does_not_block`, `dropped_row_cannot_also_be_mapped_or_protected` (error), `unknown_dropped_row_is_rejected`, `unclassified_row_still_blocks` (regression), `old_manifest_without_dropped_rows_still_decodes`.
- [ ] **Step 2: Run** `cargo test --locked topology_manager::migration` and `cargo test --locked topology_manager::legacy_static` — Expected: FAIL.
- [ ] **Step 3: Implement.** Exclude dropped rows from the baseline before parity and from the static source. Keep the baseline digest over the full legacy pair.
- [ ] **Step 4: Run** tests and the full gate — Expected: PASS.
- [ ] **Step 5: Commit** `feat(topology): allow explicit dropped legacy rows in enrollment`.

### Task 4: Policy proposer and root_promotion parity gate

This task is a go/no-go gate. If Step 4 cannot pass without a change to the parity rules, stop the plan and report to the owner with the failing difference.

**Files:**
- Create: `src/topology_manager/policy_draft.rs` (register in `src/topology_manager/mod.rs`)
- Test: `src/topology_manager/policy_draft_tests.rs`
- Fixture: `tests/fixtures/topology_enrollment/` (synthetic config and router data shaped like production: one router, PPP with profiles, DHCP servers `LAN`, `Wifi5Soft`, `LibreQoEMgt`, `ipoevlan10`, `network.mode = "root_promotion"`, PPPoE and DHCP backstop caps). Use synthetic names, IPs, and MACs only.

**Interfaces:**
- Produces: `#[derive(Serialize, Deserialize)] #[serde(deny_unknown_fields)] pub struct PolicyDraft { pub catalog: TopologyCatalog, pub source_settings: SourceSettings }`.
- Produces: `pub fn propose(cfg: &Config, legacy_network: &serde_json::Value) -> anyhow::Result<PolicyDraft>`. Selections: PPP uses `PppSecretName`, DHCP uses `DhcpClientMac`. Exclusions copy the configuration. Legacy node IDs are reused when `ids.rs` accepts them.
- Produces: `pub fn document(draft: &PolicyDraft) -> anyhow::Result<RegistryDocument>` (revision 0, validated).

- [ ] **Step 1: Write the failing tests:**
  - `proposed_policy_has_no_legacy_issues`: `preflight::legacy_issues(&cfg, &document(&draft)?)?` is empty for the fixture.
  - `proposed_policy_reproduces_root_promotion_network`: compile a candidate from the fixture router data (follow the `legacy_static_tests::from_pair` pattern) and assert `compare_mapped` against the legacy `topology` output with node mappings from equal IDs gives no `ChangedNetwork`.
  - `proposed_policy_round_trips_through_json`.
- [ ] **Step 2: Run** `cargo test --locked topology_manager::policy_draft` — Expected: FAIL (module missing).
- [ ] **Step 3: Implement** `propose` and `document`. Use `preflight::legacy_issues` kinds as the checklist for each configuration field.
- [ ] **Step 4: Run** tests — Expected: PASS. If `proposed_policy_reproduces_root_promotion_network` fails for a structural reason (for example a synthetic root node), stop and report.
- [ ] **Step 5: Commit** `feat(topology): propose a first policy from the legacy configuration`.

### Task 5: Mapping proposer and manifest builder

**Files:**
- Create: `src/topology_manager/enroll_mapping.rs` (register in `mod.rs`)
- Test: `src/topology_manager/enroll_mapping_tests.rs`

**Interfaces:**
- Consumes: `PreparedPair` (`rows()`, `network()`, `expected_revision()`, `generation()`, `device_identities` — add a `pub(crate)` accessor if needed), `MigrationManifest` with `dropped_rows` (Task 3).
- Produces:
  - `pub enum SuggestRule { CircuitNameEqualsPppName, MacEqualsLeaseMac }`
  - `pub enum RowDecision { Undecided, Map { account_ref: String, rule: Option<SuggestRule>, confirmed: bool }, Protect, Drop }`
  - `pub struct RowEntry { pub legacy: RowIdentity, pub legacy_circuit_name: String, pub legacy_device_name: String, pub decision: RowDecision }`
  - `pub struct NodeEntry { pub legacy_id: String, pub legacy_name: String, pub current_id: Option<String>, pub confirmed: bool }`
  - `pub struct MappingFile { pub rows: Vec<RowEntry>, pub nodes: Vec<NodeEntry>, pub static_roots: Vec<String>, pub attachments: Vec<StaticAttachment>, pub candidate_only: Vec<String> }` (serde, `deny_unknown_fields`; `candidate_only` lists `account_ref` values for information).
  - `pub fn propose(pair: &PreparedPair, legacy_rows: &[CsvRow], legacy_network: &Value, previous: Option<&MappingFile>) -> anyhow::Result<MappingFile>`
  - `pub struct Drift { pub issues: Vec<String> }` (implements `Display`)
  - `pub fn build_manifest(pair: &PreparedPair, legacy_rows: &[CsvRow], legacy_network: &Value, file: &MappingFile) -> Result<(MigrationManifest, Vec<String>, Vec<StaticAttachment>), Drift>`

- [ ] **Step 1: Write the failing tests:**
  - `ppp_row_is_suggested_by_circuit_name` and `dhcp_row_is_suggested_by_mac`, both `confirmed: false`.
  - `ip_match_alone_is_never_suggested`.
  - `equal_node_ids_are_suggested`.
  - `previous_confirmed_decision_carries_over_for_the_same_legacy_row`.
  - `unconfirmed_or_undecided_entry_is_drift`.
  - `candidate_row_without_legacy_row_is_drift_and_listed_in_candidate_only`.
  - `reconnect_with_new_ip_is_drift_and_names_the_row` (Review Focus 4).
  - `confirmed_file_builds_a_manifest_that_compare_mapped_accepts` (uses the Task 4 fixture).
- [ ] **Step 2: Run** `cargo test --locked topology_manager::enroll_mapping` — Expected: FAIL.
- [ ] **Step 3: Implement.** `build_manifest` fills `expected_revision`, `generation`, and both digests from the fresh pair and `migration::fingerprint`. It resolves each `account_ref` to the current circuit/device IDs from the fresh pair. It runs `migration::compare_mapped` and turns blocking issues into `Drift`.
- [ ] **Step 4: Run** tests and the full gate — Expected: PASS.
- [ ] **Step 5: Commit** `feat(topology): propose enrollment mappings for operator confirmation`.

### Task 6: Runner entry points

**Files:**
- Modify: `src/runner.rs` (near `enroll_topology` line 452 and `topology_preflight` line 484)
- Test: `src/runner/tests/enrollment.rs`

**Interfaces:**
- Consumes: `PolicyDraft`, `policy_draft::document` (Task 4), `enroll_mapping::{propose, build_manifest, MappingFile}` (Task 5).
- Produces:
  - `pub struct TopologyCheck { pub policy_issues: Vec<PolicyIssue>, pub prepare_error: Option<String>, pub candidate_only_by_scope: BTreeMap<String, usize>, pub suggested: usize, pub unmatched_legacy: usize, pub random_macs: Vec<String> }`
  - `pub async fn check_topology(&mut self, draft: &PolicyDraft, max_age: Duration) -> anyhow::Result<TopologyCheck>` — no lock, no store, no file write. Checks mode/target like `topology_preflight` without the lock.
  - `pub fn install_topology_policy(&mut self, draft: &PolicyDraft) -> anyhow::Result<u64>` — takes the lock. Allowed only while the registry has no `export_ownership`. Commits through `RegistryStore::commit`.
  - `pub async fn propose_enrollment(&mut self, previous: Option<&MappingFile>, max_age: Duration) -> anyhow::Result<MappingFile>` — lock, `prepare_topology`, bounded legacy read from `enrollment.rs`.
  - `pub async fn enroll_with_mapping(&mut self, file: &MappingFile, max_age: Duration) -> anyhow::Result<EnrollOutcome>` with `pub struct EnrollOutcome { pub revision: u64, pub dropped: Vec<RowIdentity> }` — fresh prepare, `build_manifest`, `enroll_topology`.

- [ ] **Step 1: Write the failing tests:**
  - `check_takes_no_lock_and_writes_nothing`: another `InstanceLock` is held; the state directory file list is equal before and after.
  - `check_reports_candidate_only_scope` (Review Focus 1): fixture lease on `ipoevlan10` not in legacy gives `candidate_only_by_scope["…ipoevlan10…"] == 1`.
  - `check_warns_for_locally_administered_mac`: a lease MAC `02:…` appears in `random_macs` and is not a policy issue.
  - `install_policy_then_propose_then_enroll_installs_one_revision`.
  - `install_policy_is_refused_after_enrollment`.
  - `enroll_with_drift_writes_nothing`.
  - `enroll_does_not_write_shaping_files_or_claim_a_writer` (assert live pair bytes unchanged and no topology runtime owner file).
- [ ] **Step 2: Run** `cargo test --locked runner::tests::enrollment` — Expected: FAIL (check the real module path with `cargo test --locked -- --list | grep enrollment`).
- [ ] **Step 3: Implement** the methods. Reuse `topology_preflight`, `open_topology_store`, and `enrollment::prepare` logic. Do not duplicate the bounded reader; extract the CSV/network read from `enrollment::prepare` into `pub(crate) fn read_live_pair(cfg) -> anyhow::Result<(Vec<CsvRow>, Value)>`.
- [ ] **Step 4: Run** tests and the full gate — Expected: PASS.
- [ ] **Step 5: Commit** `feat(topology): add runner entry points for check, policy, propose, and enroll`.

### Task 7: `lqosync3 topology` commands

**Files:**
- Modify: `src/cli.rs` (`Command`, `parse`, `USAGE`), `src/main.rs` (dispatch)
- Test: `src/cli.rs` tests module; `tests/topology_enrollment.rs` (create)

**Interfaces:**
- Produces: `Command::Topology(TopologyCommand)` with
  `pub enum TopologyCommand { Check { policy: PathBuf }, PolicyPropose { out: PathBuf }, PolicyInstall { policy: PathBuf }, Propose { out: PathBuf, previous: Option<PathBuf> }, Enroll { mapping: PathBuf, yes: bool } }`.
- Command lines:
  - `lqosync3 topology policy propose --out <file>`
  - `lqosync3 topology check --policy <file>`
  - `lqosync3 topology policy install --policy <file>`
  - `lqosync3 topology propose --out <file> [--previous <file>]`
  - `lqosync3 topology enroll --mapping <file> [--yes]`
- `enroll` without `--yes` builds the manifest, prints the summary (mapped, protected, dropped counts and each dropped row), and exits with status 2. With `--yes` it enrolls and prints `installed registry revision N`.
- Lock failure message: `another lqosync3 process holds the instance lock; stop lqosync3.service before this command`.
- Output files are written with mode `0600`, refuse to overwrite an existing file, and never contain secrets.
- `Command` loses `Copy`; update callers.

- [ ] **Step 1: Write the failing tests:** parse tests for each command line and each missing-argument error; `tests/topology_enrollment.rs` end-to-end with `tests/mock_router.rs` and a temporary config/state/LibreQoS directory: policy propose, check, policy install, propose, edit the file to confirm all, enroll without `--yes` (status 2, no change), enroll with `--yes` (revision printed); a run with a held lock gives the lock message (Review Focus 5); a fixture with an `ipoevlan10` lease not in legacy makes `check` exit non-zero and name the scope.
- [ ] **Step 2: Run** `cargo test --locked --test topology_enrollment` and `cargo test --locked cli::` — Expected: FAIL.
- [ ] **Step 3: Implement** parse and dispatch. Use `MikrotikConnector { owner_uid: SECRET_OWNER_UID }` like `Command::Once`. Use `max_age = Duration::from_secs(120)` for all prepare calls.
- [ ] **Step 4: Run** tests and the full gate — Expected: PASS.
- [ ] **Step 5: Commit** `feat(cli): add topology enrollment commands`.

### Task 8: Runbook, records, and final verification

**Files:**
- Create: `docs/operations/TOPOLOGY_ENROLLMENT.md`
- Modify: `docs/design/TOPOLOGY_MANAGER_IMPLEMENTATION.md` (new section "Enrollment CLI and identity rules"), `brain/30-knowledge/topology-manager-status.md`, `brain/HANDOFF.md` (new top entry only), `README.md` usage line if it lists commands
- Create: `brain/10-sessions/2026-11-18-topology-enrollment-cli.md`

- [ ] **Step 1: Write the runbook** with the spec's eight steps. Add the `ipoevlan10` rule: when `check` reports candidate-only rows for a scope, add that DHCP server to `[routers.exclude] dhcp_servers` for enrollment (the legacy files have no rows for it, so live shaping does not change), and remove it after Phase 4 activation. Add rollback: restore the backed-up state directory while the service is stopped.
- [ ] **Step 2: Run** `cargo fmt --check && cargo clippy --all-targets --locked -- -D warnings && cargo test --locked && cargo +1.89 build --locked && git diff --check` — Expected: all pass. Save logs in `.superpowers/sdd/2026-11-18-topology-enrollment-cli/`.
- [ ] **Step 3: Update** the design section, status note (Phase 1 done, Phase 2 next), session note, and a HANDOFF top entry with results and remaining gates.
- [ ] **Step 4: Commit** `docs(topology): record enrollment CLI, identity rules, and runbook`, then push `lqosync3`.
