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
