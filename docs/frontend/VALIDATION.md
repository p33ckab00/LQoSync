# Documentation validation

Date: 2026-11-20T16:11:44+08:00 (Asia/Shanghai).
Code baseline: `9a465e7`. Branch: `lqosync3`.

## Results

| Check | Result |
|---|---|
| Image inventory | 11 original PNG files copied |
| Image SHA-256 manifest | All 11 hashes match |
| Screen catalog | 11 embedded images with feature/task links and corrections |
| Relative Markdown links | All referenced repository files exist at the documentation checkpoint |
| Atomic task IDs | 19 unique cards, F00–F18, with dependencies and checks |
| Task order | Listed execution order satisfies each declared dependency |
| Source review | Service transport, dispatch, Runner, collection, plans, topology, settings, and take-effect evidence inspected |
| STE heuristic | No sentence-length, paragraph-length, listed unapproved-word, or semicolon flags in the frontend documents before this report |
| STE manual review | Reviewed technical names, source identifiers, ownership statements, failure states, and instruction clarity |
| Whitespace | `git diff --check` passes |
| Branch comparison before commit | Local HEAD and `origin/lqosync3` both at `9a465e7` |

The heuristic still flags technical noun groups, words with `-ing`, and some descriptive constructions.
These flags do not establish a language defect by themselves.
The exact image prompts remain unchanged inside fenced blocks for provenance.
No certified ASD-STE100 compliance is claimed.

## Limits

Follow-up owner direction replaces web account administration with CLI-only superadmin CRUD.
The account policy, feature design, contracts, screen corrections, and F04/F14 tasks now reflect this direction.
The original Users image remains unchanged as a withdrawn concept.

This change contains documents and images only. It does not implement or benchmark the frontend.
Rust application tests, Wasm builds, browser tests, and production deployment checks were not run.
The implementation plan lists those checks for future execution.
No live router, LibreQoS input, service, credential, or runtime configuration changed.
The existing graph was used for research leads. It was not rebuilt for this documentation package.
Existing unrelated operational notes remain outside the commit.

## Recheck images

From `docs/frontend/images`, run:

```sh
sha256sum --check SHA256SUMS
```

Run the bundled STE checker on each Markdown file separately.
Review its findings manually because the checker is heuristic.


## Sign-in theme follow-up

Checkpoint: 2026-11-20T16:47:12+08:00 (Asia/Shanghai).
The image set now contains 14 PNG files. All hashes in SHA256SUMS match.
Relative document links pass. The original sign-in and two glass variants remain picture previews.
System/Light/Dark behavior, blur performance, accessibility, and browser rendering still require implementation and tests.


## Screen-owned settings follow-up

Checkpoint: 2026-11-20T16:53:01+08:00 (Asia/Shanghai).
Updated shared header theme placement and feature-local configuration ownership.
Document links, 19-task dependency order, 14 image hashes, STE review, and whitespace checks pass.
Scoped endpoint rejection, cross-screen conflicts, and theme behavior remain future implementation tests.


## Company branding follow-up

Checkpoint: 2026-11-20T16:58:02+08:00 (Asia/Shanghai).
Added global company name/logo requirements and F19. The plan now has 20 task cards.
Local links, task dependencies, 14 existing image hashes, STE review, and whitespace checks pass.
Upload security, public DTO redaction, theme rendering, and asset recovery require future implementation tests.
