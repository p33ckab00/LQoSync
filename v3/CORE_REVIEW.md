# Read-only core review

The **Core review** page reports structural route findings for configured core and both-role routers.
It shares the route analyzer with the core list planner.
Connection-mark links distinguish subscriber forwarding from router replies. See [end-to-end validation](VALIDATION.md).
It does not send router writes, change subscriber assignments, or apply LibreQoS output.

## Credentials and TLS

Store private credentials in `credentials.toml` under the application data directory.
Use the exact stable router ID from `settings.toml`:

```toml
[routers.example-core]
password = "REPLACE_LOCALLY"
```

Set the private file mode to `0600`. Keep this file outside Git.
Git ignores the default `v3/data/` directory.
For another data directory, exclude private files from version control separately.
Do not paste passwords into chat, public fixtures, notes, or issue reports.

The app never renders passwords. Credential parser errors never include source lines.

A local entry for that router takes precedence over the environment reference.
An empty entry or invalid TOML blocks authentication.
If the file or router entry is absent, the app uses the configured password environment reference.
The reference can remain empty when a local entry exists.
Both scheduled reads and on-demand reads use this selection.

TLS verifies the trust chain and certificate name.
For a private CA, store its public PEM file under this application's directory.
Check the CA fingerprint through a trusted source before you use it.
Set the router's `tls_ca_file` to that file path.
If the certificate uses a DNS name, set `tls_server_name` to the name on that certificate.
The app does not disable certificate verification or install a system-wide CA.

## Use the page

1. Open **Routers**.
2. Read the core inventory.
3. Open **Core review**.
4. Review inventory health, observed paths, and all reported findings.
5. Select **Saved inventory** only for historical diagnostics.

Live inventory means inventory in this process. Its age can still be stale.
Saved inventory never becomes fresh live input and never enters the active collector.
Missing, failed, or partial tables produce `Unavailable` findings.
The GET page never publishes a generation.
If settings or live read inputs change during analysis, the page rejects that result.

## Results and limits

- `Pass`: the supported structural checks found no issue for that WAN.
- `Unsupported`: the policy needs checks outside the current analyzer.
- `Conflict`: observed policy conflicts with the selected WAN mapping.
- `Unavailable`: required input is missing, incomplete, or unknown.

Inventory health appears separately from the structural result.
A structural pass does not prove packet routing or authorize a router writer.
Fresh complete inputs and all existing planner checks remain required for local publication.
Subscriber membership and address ownership checks remain on **Core lists**.

With no configured bindings, the review infers candidate paths from active custom-table defaults.
These paths remain **Observed only**. The app does not create WAN roots or list bindings.
A global `mapping-required` finding blocks a pass.
Select the actual WAN interface, source list, and custom table explicitly before you plan members.

The analyzer supports direct RouterOS 7 source-list marks, narrow connection-mark paths, and one resolved WAN default.
Active PCC, unsupported connection-mark conditions, complex prerouting actions, FastTrack, routing rules, and ambiguous defaults require further review.
Disabled rules do not trigger active PCC or FastTrack counts.
The review does not simulate NAT, VRFs, connection state, or all firewall behavior.
See [core planning](CORE_PLANNER.md) for the full supported subset.

The report retains up to 256 findings, then adds a global `finding-limit` result.
Without configured mappings, it analyzes up to 512 observed paths.
It reports omitted paths explicitly with `observed-path-limit` and an omitted count.
Neither limit can turn incomplete review into a pass.
The page shows the first 200 paths. All configured paths receive analysis.

## Diagnostic command

Run from the application directory:

```bash
cd /opt/LQoSync3.0/v3
cargo run --locked --bin core-validate
```

The command reads configured core and both-role routers through verified TLS and the print allowlist.
It saves raw inventory and `state/core-validation.json` only under the application data directory.
Set `LQOSYNC3_DATA` to select another application-owned directory.
Do not run multiple collectors against the same data directory at once.

The command does not update the live web process or publish output.
Its terminal output contains counts and reason codes, without private policy values.

For a historical review without network access:

```bash
cargo run --locked --bin core-validate -- --saved
```

Without `--end-to-end`, exit code zero means the input review completed. It does not mean the policy passed.
With `--end-to-end`, a non-passing review returns nonzero after it saves the local report.
Missing, failed, or partial core input produces a nonzero exit code after it saves the local report.
Saved or stale inventory remains historical evidence regardless of the exit code.

To export a local routing projection from saved inventory:

```bash
cargo run --locked --bin core-validate -- --saved --fixture
```

The export aliases names and replaces private values. It removes comments and unrelated fields.
It checks finding-code counts and connection-path directions/statuses before and after projection.

This projection does not preserve address containment or all packet behavior.
Review exports before you share them. Raw inventory and reports remain private.
See [fixture provenance](tests/fixtures/README.md).

## Performance check

```bash
cargo test --locked --test core_diagnostics review_1000_wans_and_100k_routes -- --ignored --nocapture
```

The workload uses 1,000 mapped WANs, 100,000 routes, and 50,000 address-list rows.
The analyzer indexes rules, tables, interfaces, and defaults once per core.
The test checks every WAN and reversed input order.
Measurements exclude network collection and disk publication. Debug timings do not guarantee production latency.
