# LQoSync v3 graph scope

This graph maps the Rust v3 application, tests, current guides, design plans, and project notes.
The interactive view is `graph.html`. The query data is `graph.json`. Read `GRAPH_REPORT.md` for the graph summary.
The HTML viewer loads Vis Network from a public CDN. It needs network access for that library.

## Included sources

- `v3/src/` and `v3/tests/`, except test fixtures.
- The v3 Cargo manifests and Markdown guides.
- The transport vendor README and patch guide.
- `docs/superpowers/` for design intent and plans.
- `brain/` for project knowledge and session history.
- Root `README.md`, `AGENTS.md`, and `CLAUDE.md`.

## Excluded sources

- The earlier root application and its documentation.
- Private runtime data, credentials, and local output.
- Build output and generated graph files.
- Test fixtures, example credentials, and vendor source code.

The saved `.graphify_build.json` defines these exclusions for later updates.
Git ignore rules also apply. The graph does not read the live routers or LibreQoS state.

## Queries

Run these commands from `/opt/LQoSync3.0`:

```bash
graphify query "subscriber normalization and local publication" --budget 1500
graphify query "protected manual membership" --budget 1500
graphify explain "minimums.rs"
```

Check each graph result against the source files before a code change.
Plans describe intent. Session notes describe historical checks. Neither proves current runtime behavior.
The graph records extracted and inferred relationships separately.

## Updates

For code changes, run `graphify update .` from the project root.
For changes to docs or notes, use the Graphify skill with `--update` for semantic extraction.
Use the saved exclusions for both update paths.
Regenerate the HTML after an update if the update command does not regenerate it.

## Limits

The Rust extractor does not resolve every import, type, or function reference.
Unresolved endpoints can appear as external concept nodes. They do not prove an external dependency.

The graph combines repeated relationships between the same endpoints in its default graph representation.
Read the health diagnostic in `GRAPH_REPORT.md` for counts from this build.
This graph supports navigation. It does not prove a complete call graph or production readiness.
Semantic extraction used session agents. The agent tool did not expose token usage, so exact extraction cost is unavailable.
