# LQoSync v3 agent instructions

## Shared skills

Use the shared skills in `/home/pi/.agents/skills`.
Read `/home/pi/.agents/skills/tandem/SKILL.md` before non-trivial work.
If a skill points to a maintained source, read that source. Resolve its references from the source directory.
User instructions and platform rules take precedence over skills.

- Use Caveman for brief, clear replies in the user's language.
- Use Ponytail for the smallest complete code change.
- Use the installed Superpowers skills for applicable planning, debugging, tests, and verification.
- Use Brain to recall project knowledge and record decisions and milestones.

- Query an existing Graphify graph when relevant. Build or update only what the task needs.
- Use Obsidian skills for vault Markdown, Canvas, and Bases files.
- Use ASD-STE100 for technical prose and ste-diagrams for diagram labels.
- Use Headroom and RTK when large output needs compression.

Do not install another Superpowers copy. Do not copy TI product rules or private chat history into this project.
Claude hooks do not run in Codex. Call RTK explicitly when needed.
Use Headroom MCP only when its tools are available and the connection works.
If a tool is unavailable, use targeted file searches and report the limit.

## Project context

The current application lives in `v3/`. Root application files belong to the earlier implementation.
Read `v3/README.md` and the relevant guides before changes.
Keep project knowledge in this repository's `brain/` vault, separate from TI.

Before a handoff, read `brain/Home.md`, relevant decisions, the latest session note, and `brain/HANDOFF.md`.
Check the current branch, worktree, diff, and check results against those notes.
Preserve unfinished work. Coordinate file ownership if another agent edits the same files.
Record authorized handoffs in `brain/HANDOFF.md` with an absolute timestamp in Asia/Shanghai.
Include the goal, last agent, branch, worktree, changed files, check results, unfinished work, and next action.

## Operational limits

Preserve protected manual RouterOS entries and existing ownership checks.
Do not bypass validation or publication gates to resolve conflicts.
Keep candidate, staged, and applied output distinct. A local preview does not prove production application.
Router writes, production shaping, package changes, and service restarts require task authorization.
Keep private settings, credentials, raw inventory, and customer data out of Git and project notes.

## Verification

Run checks that match the change. For v3 Rust changes, run from `v3/`:

```bash
cargo fmt --check
cargo test --locked
cargo clippy --locked --all-targets -- -D warnings
```

For documentation changes, check the diff, paths, and links.
Report actual check results. Do not present historical results as new verification.
