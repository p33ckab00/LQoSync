---
type: session
created: 2026-11-19
tags: [topology, planner, live-test, libreqos, runbook]
status: done
related: ["[[topology-manager-status]]", "[[2026-11-19-topology-fallback-and-new-sources]]"]
---

# Topology planner admission and live acceptance tooling (Phase 3)

LQoSync must prove that LibreQoS accepts the manager pair before Phase 4. The owner has no test box, so the proof runs on production in one window, with an automatic restore.

## Results

- Spec and plan: [design](../../docs/superpowers/specs/2026-11-19-topology-live-acceptance-design.md). Commits `ce7b515..HEAD`: `fd9b04a` planner rule model and read-back, `2f8ce17` native acknowledgement check, `556d401` live test, `2fcdee4` content-generation wait, `01f1aa7` CLI commands.
- Commands: `lqosync3 topology live-test --report <file> [--import-timeout <s>]` and `live-test-restore --backup <dir>`.
- Runbook: [TOPOLOGY_LIVE_TEST](../../docs/operations/TOPOLOGY_LIVE_TEST.md). Claude runs it after the owner says "go".
- No live service, router, or LibreQoS file changed in this session. The live window is pending.

## Rulings

- Generations are content hashes. A manager pair equal to the legacy pair gives the same generation, so the test refuses it.
- The test waits for three equal generations (settle) before it reads back.
- No write receipt tracker, runtime claim, registry commit, or notification. The production service is an older build.
- The restore writes the backup bytes and checks them.

## Open minors

- The first production run decides the import timeout default (120 s).
- A registry with `catalog.fallback` does not load in an older build: clear the fallback before a rollback.

Related: [[topology-manager-status]].
