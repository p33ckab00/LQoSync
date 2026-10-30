# Core control preview

Open **Core control** under **Routing & verification**.
The console shows WAN health, observed online counts, desired counts, and reconciliation decisions.
These values describe a local preview. They do not prove router changes or production shaping.

Automatic writes remain blocked in this build. Both the form and settings validation reject automatic mode.
The guarded writer library has no console caller.

## Configure the preview

1. Configure the dynamic topology, selected subscriber sources, WAN interfaces, and core list/table pairs.
2. Open the cog on each required WAN card.
3. Select two existing ICMP Netwatch entry IDs and their external IPv4 targets.
4. Save the local monitor selection.
5. Open the policy cog.
6. Select **Preview** and save the policy.

The console does not create Netwatch entries, routes, or scripts.
The provider requires supported probe attribution through dedicated active main-table host routes.
Missing fields, unsupported policy, failed reads, and stale samples produce **Unknown** health.
See [WAN health](WAN_HEALTH.md) for exact provider requirements.

A monitor selection does not start collection until the operator saves a Preview policy.
Legacy settings keep this preview disabled. Monitor selection alone cannot enable router writes.

## Read the results

- **Observed online** counts selected online clients with exactly one actual WAN list match, with protected ranges.

- **Desired** counts clients in the current reconciliation plan.

- The separate ambiguity count covers absent membership and multiple WAN matches.

- Health requires new completed probe rounds. Repeated counters cannot prove recovery or refresh completed-sample freshness.

- Decisions distinguish Keep, Add, Move, Remove, and Blocked. The page displays at most 100 decisions.

- Incomplete subscriber or core evidence blocks the plan and clears offline cleanup receipts. Available health diagnostics remain visible.

- Expired or superseded plans display unknown counts. Settings changes invalidate prior health and cleanup context.

The policy controls failure behavior, pin override, offline confirmation, health thresholds, recovery delay, and evidence lifetime.
The batch limit reserves the bounded writer policy. It does not start a writer.
See [reconciliation](CORE_RECONCILE.md) for assignment and cleanup rules.

## Collection and pause

Preview collects selected source and WAN routers at the shorter configured health or ordinary collection interval.
Collection uses at most four concurrent router reads. Network I/O and preview computation run outside the App lock.

Only configured health cores receive the extra Netwatch read.
Normal local publication retains its own interval. Intermediate health ticks do not publish normal generations.
Existing successful configuration actions can still request normal local publication.

Actual membership validation continues to guard publication.

Select **Paused** in the policy modal to stop health preview collection and discard its runtime evidence.
Ordinary collection remains controlled by **Automatic collection** in Settings.
To stop both paths, pause Core control and disable ordinary automatic collection.
An in-flight router read can complete, but superseded results cannot authorize a preview.

Health baselines and preview cleanup receipts exist only in memory. A restart requires new complete rounds.
The preview does not save candidate assignments or change session baselines.
Ordinary local staging retains its existing persistence behavior.

## Remaining live integration

The console has no mutation worker, production verifier, pending-journal controls, or last-operation report.
The existing recovery marker and maintenance lease still prevent unsafe console startup.
Do not remove a marker to start the console. Use [writer recovery](CORE_WRITER.md) or [maintenance recovery](CORE_MAINTENANCE.md).

Live integration must coordinate the console lease with the exclusive writer lease.
It must verify fresh sources, sessions, settings, health, and complete actual core rows before each mutation.
Isolated RouterOS acceptance must prove probe fields, WAN attribution, same-ID transitions, and connection-mark limits.
Production activation requires a separately reviewed scope and authorization.
No HTTP option can assert or bypass these prerequisites.
