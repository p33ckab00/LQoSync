# Claude Code: Topology Studio backend handoff

Status: ready for backend implementation after the shared plan commit.
Last agent: Codex. Baseline: `d38113e` on `lqosync3`.
Project: `/home/pi/LQoSync3.0`. Frontend owner: Codex. Backend owner: Claude Code.

## Read first

1. Read the project `AGENTS.md` instructions supplied by the owner.
2. Read `brain/Home.md` and the latest entry in `brain/HANDOFF.md`.
3. Read [the product design](../frontend/ROUTER_SOURCE_CONFIGURATION.md).

4. Read [the API contract](../frontend/TOPOLOGY_STUDIO_API.md).
5. Read [the tandem plan](../superpowers/plans/2026-11-23-topology-studio-tandem.md).

Use the current files as evidence. Older handoff entries contain stale branch and deployment facts.
The owner requested this division of work. Do not replace it with an independent frontend rewrite.

## Your scope

Implement B0, B1, B2, B3, B4, and B5 from the tandem plan.
Own shared Rust types, gateway HTTP, broker IPC, service commands, discovery, preview, policy storage, and publisher integration.
Own the browser fixture and its synthetic source-policy cases.
Provide `tools/integration/studio-service-smoke.sh` with real gateway/broker/core binaries and isolated fake RouterOS/runtime endpoints.
Keep every smoke-test config and output under its temporary root. Mark its runtime responses as test evidence.

Keep application UI edits with Codex after B0 mechanical compatibility changes finish.

Start with B0. Publish a compiling contract commit before the UI consumes new APIs.
Keep unavailable production capabilities false until the matching backend task passes.
Send the contract commit, fixture cases, and checks through this handoff file.

Do not infer that a file update automatically starts the frontend agent.

## Backend requirements

- Keep all RouterOS I/O in the core service.

- Preserve the disabled registered-router rule. Add a separate explicit draft test.

- Keep draft secrets out of logs, receipts, snapshots, and reports.

- Distinguish complete-empty tables from failed or partial tables.

- Evaluate the draft against frozen evidence and the complete current topology.

- Keep preview free of durable ID, grace, routing, policy, and output mutations.

- Save policy through the current edit receipt and publisher path.

- Check the config digest and registry revision separately.

- Preserve the distinction between legacy policy `None` and explicit policy `Some([])`.

- Preserve stable identities, ancestor limits, shared budgets, and one writer.

- Do not enable RouterOS write scopes or change live ownership as a side effect.

## Worktree and shared-file rules

Before parallel edits, commit the plan package and complete G0.
Use branch `claude/topology-studio-backend` in a separate worktree from the agreed G0 commit.
Record the exact branch and absolute worktree path here when work starts.

Do not edit the frontend lane's files after B0 without an explicit ownership handoff.
Only one worker uses browser fixture port `19581` at a time.
Do not force-push, deploy, restart production services, or run live collection as a test of this plan.

## Required return packet

Record this packet after each backend milestone:

| Item | Required value |
| --- | --- |
| Task | B0, B1, B2, B3, B4, or B5 |
| Branch/worktree | Exact branch and absolute directory |
| Base/head | Full commit IDs |
| Contract version | `1` and the contract commit |
| Files | Owned changed paths |
| API state | Implemented routes and advertised features |
| Fixture state | Supported scenario names |
| Checks | Exact commands, result, log location |
| Migration | Schema, backup, compatibility, and rollback method |
| Open work | Remaining task IDs and known limits |
| Next frontend step | Matching F task or required contract correction |
| Time | Absolute timestamp with timezone |

Use STE prose and exact technical names.
Do not report a task complete from a mock response or an unexecuted test command.
