# Core maintenance CLI

The maintenance CLI connects the guarded migration engine to RouterOS.
It can change protected address-list rows only through an explicit private plan and operator approval.
The web console and scheduler do not call this writer. No maintenance service starts automatically.

Local OS access to private plan files and router credentials is the authentication boundary.
Run the CLI as the data owner, `pi` on this server. Do not use sudo for these commands.
Do not expose this CLI through an unauthenticated HTTP or shell endpoint.

## Current scope

The CLI supports dynamic topology with online-client policies and complete session evidence.
It preserves each selected client's observed WAN. It does not force equal WAN counts.
The bootstrap uses real handles, uptimes, and source identities. Temporary pins exist only in memory.

It preserves configured manual pins and rejects conflicting ones. It saves candidate evidence through the existing validated CAS API.

The engine preserves the full IPv4 membership of each list. Original protected IDs and comments stay intact.
Unknown write outcomes block automatic retry and rollback. No lookup by comment or address substitutes for a returned row ID.

## Process fence and external writers

Each updated console holds a shared kernel lease for its complete lifetime.
Maintenance requires an exclusive lease on the same data directory.
Stop all consoles for that directory before preparation or execution. This includes other terminals and service instances.
The CLI also checks visible legacy console processes that do not use the lease.

Local leases do not exclude external RouterOS scripts, other applications, or administrator changes.
The operator must establish exclusive RouterOS configuration access for the maintenance window.
The exact-plan acknowledgement confirms that separate operational condition. It does not create a router-side transaction.
Do not acknowledge it while another address-list producer can act.
Do not stop unrelated LibreQoS services as part of this procedure.

## Prepare a private review

Build the maintenance binary from `v3/`:

```bash
cargo build --locked --bin core-migrate
```

Stop the console with the applicable commands in [OPERATIONS.md](OPERATIONS.md).
This procedure does not stop it automatically.

Create an operator-owned private directory and scope file. Use mode `0700` for the directory and `0600` for the file.
List only protected rows that the operator intends to review. Replace these placeholders with exact captured IDs:

```json
[
  {"router":"CORE_ROUTER_ID","id":"PROTECTED_ROW_ID"}
]
```

Prepare a new plan with fresh verified router reads:

Use the application data directory as ROOT. For this server:

```bash
/opt/LQoSync3.0/v3/target/debug/core-migrate prepare \
  /opt/LQoSync3.0/v3/data \
  /PRIVATE_DIRECTORY/scope.json \
  /PRIVATE_DIRECTORY/plan.json
```

Preparation reads routers and creates the private plan. It does not change candidate assignments or router configuration.
The output plan uses mode `0600`. An existing output filename is rejected.

Inspect `operations` for the exact additions and protected-row address changes.
Inspect the captured baseline, selected hosts, observed destinations, and settings.
Keep this customer-level file outside Git. The printed revision identifies the complete review envelope.
The earlier audit `migration-proposal.json` has a different schema and is not executable by this CLI.

## Apply only after authorization

Get explicit authorization for the exact protected-row changes.
Establish the external writer maintenance window first.
Replace REVISION with the exact revision printed by preparation:

```bash
/opt/LQoSync3.0/v3/target/debug/core-migrate apply \
  /opt/LQoSync3.0/v3/data \
  /PRIVATE_DIRECTORY/plan.json \
  REVISION \
  EXCLUSIVE_ROUTER_ACCESS:REVISION
```

The plan expires within 300 seconds of its captured evidence. Regenerate and review it if it expires.
The CLI rejects mismatched settings, eligible destinations, client addresses, session identities, router rows, and approvals.
It records the bootstrap intent, saves observed-WAN candidate evidence, and then executes the guarded migration.
It validates fresh actual router membership and generated output before it clears maintenance.

Validation evidence stays under `data/state/core-migrations/` in a separate actual-validation directory.
These validation inputs come from actual fresh reads. No simulated router rows replace them.

The CLI does not change the application's current generation pointer or LibreQoS files.
After success, restart the console. The normal fresh-collection and publication gates still apply.
Production shaping apply and a continuous address-list writer remain separate tasks.

## Rollback and interruption

Keep the console stopped and keep exclusive external writer access.
Use the same plan and revision:

```bash
/opt/LQoSync3.0/v3/target/debug/core-migrate rollback \
  /opt/LQoSync3.0/v3/data \
  /PRIVATE_DIRECTORY/plan.json \
  REVISION \
  EXCLUSIVE_ROUTER_ACCESS:REVISION
```

Rollback restores original broad rows before it removes exact returned addition IDs.
It does not restore an entire router configuration or edit routing rules.
Candidate evidence stays aligned to the observed WANs. No persistent temporary pin is added.
After router rollback, manual broad entries can cause the original core validation conflict again.

Any previously staged generation remains historical. Fresh validation determines whether publication can proceed.

A durable recovery marker prevents console startup after failed or interrupted maintenance.
Bootstrap recovery accepts only the exact old or new candidate state. It can safely finish an interrupted CAS save.
A known-unsent preflight failure clears only that unsent intent. It does not turn an ambiguous network outcome into a safe retry.
A missing reply or missing returned add ID preserves the pending intent and requires manual reconciliation.

The engine's per-journal `.lock` file can survive a process crash. Before manual removal, prove that its executor no longer exists.
Do not remove recovery markers or edit journals to force console startup.
If a request outcome is unknown, inspect actual rows and the private journal through a separately authorized recovery procedure.
Automatic recovery does not guess which newly observed row belongs to a lost add response.

## Verification boundary

Tests use synthetic loopback routers, private temporary directories, and simulated failures.
They cover real protocol framing, returned IDs, leases across processes, legacy console detection, bootstrap interruption, source drift, and rollback.
They do not prove external writer exclusion or interruption-free behavior on a production router.
No live migration was performed during implementation.
