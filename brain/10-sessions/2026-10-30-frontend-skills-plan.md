---
type: session
created: 2026-10-30
tags: [lqosync-v3, skills, tandem, frontend]
status: complete
related: ["[[20-decisions/adr-0003-dioxus-rust-ui]]"]
---

# Frontend skills and migration plan

The operator selected Dioxus and Rust/UI. Scope: skills, Tandem, and a plan only.
The current runtime remains Axum with Maud templates.

Installed the requested skills with the Codex Skill Installer:

- `notmehul/skills`, path `skills/ui-ux`, branch `main`.
- `anthropics/skills`, path `skills/frontend-design`, branch `main`.

The canonical installations are `/home/pi/.agents/skills/ui-ux` and `/home/pi/.agents/skills/frontend-design`.
Claude and Codex skill directories link to these shared installations.
The installer retained the skill resources and the supplied frontend-design license.

Updated `/home/pi/.claude/skills/tandem/SKILL.md`, its session hook, and the shared Codex discovery entry.
Added interface guidance, framework precedence, and rules for unavailable companion tools.
The UI/UX search tool has no Dioxus preset. Use general guidance with official Dioxus sources.
Optional companion skills and the external browser inspector were not installed.

Added the migration design, seven-task plan, stack decision, and current/target distinction in `v3/README.md`.
The plan retains Axum, adds a browser crate, and preserves backend checks during page migration.
Rust/UI source and its installation page differ on framework support. Verify pinned Dioxus source during implementation.

Checks and exact results appear in [[HANDOFF]].
No Rust source, application dependencies, live settings, router state, or services changed.
