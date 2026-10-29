# LQoSync v3 foundation implementation plan

**Goal:** Build the first Rust web console and local publication workflow.

**Architecture:** Axum serves Maud pages. A typed domain model validates topology and creates local generations.

**Spec:** `docs/superpowers/specs/2026-10-29-lqosync-v3-design.md`.

## Constraints

- Change only the new checkout. Preserve LibreQoS v2.2 and its package hold.
- Make no router writes or production apply requests.
- Keep runtime files inside the selected application data directory.

## Tasks

- [x] Test and implement hierarchy validation, directional caps, and subscriber CSV/JSON generation.
- [x] Test and implement atomic configuration and local generation publication.
- [x] Build Rust pages and forms for routers and topology. Reject invalid edits without changing stored state.
- [x] Add automatic local publication and truthful status reporting.
- [x] Run tests, formatting, lint, and HTTP smoke checks.
- [x] Record the handoff, commit, and push branch `lqosyncv3`.
