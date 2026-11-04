# LQoSync4 agent instructions

These instructions apply to the repository. Follow higher-priority instructions and the user's current request when they conflict with local guidance.

## Start here

1. Read `/home/pi/.codex/skills/agent-tandem/SKILL.md` for every task. For coding, review, optimization, or architecture work, read [the engineering rules](docs/lqosync4/brain/AGENTIC_ENGINEERING_RULES.md) and [the completion gate](docs/lqosync4/brain/PRODUCTION_READY_GATE.md).
2. Read [HANDOFF](docs/lqosync4/brain/HANDOFF.md) for current state and [DECISIONS](docs/lqosync4/brain/DECISIONS.md) for accepted versus open decisions. Use [the agent workflow](docs/lqosync4/brain/AGENT_WORKFLOW.md) to coordinate work and handoffs.
3. For architecture or dependency questions, query the existing [Graphify graph](docs/lqosync4/graphify-out/README.md) first, then verify important claims in the current source and specs. The graph is a planning map, not proof that a function exists. Refresh it when an architectural relationship or source document changes materially.
4. Before editing, inspect branch, working tree, relevant code, callers, tests, configuration, and [the LQoSync4 spec](docs/lqosync4/docs/PRODUCT_SPEC.md). Preserve unrelated work.

## Tandem coding

- For nontrivial work that crosses modules or layers, use a coordinator plus distinct mapping/implementation/review responsibilities. Delegate independent work to separate agents when delegation is available and authorized. Assign one file owner per concurrent edit. If only one agent is available, perform the roles in sequence and self-review the final diff.
- The coordinator keeps the task scope, invariants, decisions, and test evidence coherent. A mapper traces source, Graphify, routes, and data flow. A builder makes the smallest complete change. A reviewer checks the actual diff and failure paths. No agent may mark another agent's unverified work complete.
- Use the shared [brain handoff](docs/lqosync4/brain/HANDOFF.md) for durable state and [the decision log](docs/lqosync4/brain/DECISIONS.md) for owner decisions. Do not turn a proposal into an accepted decision without owner evidence. Use the [workflow](docs/lqosync4/brain/AGENT_WORKFLOW.md) for the handoff format.

## Engineering and completion

- Port the LQoSync3 core without its TUI. The planned backend and Leptos browser application use Rust; CSS and generated Wasm glue are allowed. Preserve existing LibreQoS output and shadow/apply behavior until an intentional migration is verified.
- Trace a feature end to end: input/UI, API, validation, state, background work, RouterOS or LibreQoS effects, errors, readback, audit, and user-visible result. Do not present a stub, mock, inactive control, file write, or passing compile as a completed feature.
- For nontrivial changes, plan the smallest complete approach, review the final diff, fix confirmed defects, and rerun task-relevant checks after the final edit. Test meaningful success, failure, boundary, retry, and restart behavior when applicable. Measure performance before claiming improvement; keep queues and memory bounded.
- Never expose secrets, bypass authorization, or make live RouterOS changes during planning. Keep desired, observed, written, and imported states distinct. Follow the explicit owner decisions for any deployment or external mutation.
- Report implemented versus tested versus unverified behavior honestly. Do not commit, push, force-push, reset, or discard user work unless the current request or established repository authorization permits it.
