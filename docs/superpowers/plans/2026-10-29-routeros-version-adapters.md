# RouterOS client and LibreQoS adapter selection

## Goal

Add a maintained external RouterOS protocol library and automatic local LibreQoS version detection.
Preserve the live package hold. Do not issue production reloads or router configuration changes.

## Dependency decision

Use `mikrotik-rs = 0.7.0` with Tokio and TLS.
This published release has MIT/Apache-2.0 licensing, async tagged commands, and TLS support.
Newer 0.8–0.10 releases changed to AGPL-3.0-only; do not silently adopt that change.
The `mikrotik-client` 0.0.0 published package differs from its current binary-API README.
`mikrotik_api` 0.4.1 uses GPLv3 licensing. Do not introduce a new project-wide license decision here.
Pin the selected version and record the tradeoff. This is not a claim that an older version contains all latest fixes.

## Implementation

- [x] Add tests for version parsing and adapter selection, including the preflight-package false-positive case.
- [x] Detect the main installed `libreqos` package through a bounded read-only package query.
- [x] Select legacy command behavior for older supported families and v3 reload-request behavior for v3.
- [x] Do not assume unknown future releases or unrecognized package states are compatible.
- [x] Add a read-only client with verified TLS, explicit command/proplist allowlists, and operation deadlines.
- [x] Add connection/inventory actions and compatibility status to the Rust UI.
- [x] Test protocol handling with local fake RouterOS servers. Do not use production credentials.
- [x] Run tests, formatting, lint, and independent review. Update project notes and push the branch.

## Limits

Router inventory is separate from a complete subscriber normalization pipeline.
No automatic subscriber deletion follows an incomplete scan.
Production file export, v3 authentication, applied-generation verification, and actual apply execution remain separate work.
The selected adapter describes the correct control family; it does not assert production deployment readiness.


## Verification result

Application tests: 21 passed. Formatting and strict Clippy checks passed.
HTTP smoke selected legacy from the installed main v2.2 package.
The live package hold remains present.
Independent review found a transport burst bug. A 200-row test reproduced the failure.
The vendored bounded-send patch fixed it. Repeat review found no remaining material issue.
Local TLS tests cover trusted matching names, wrong names, and untrusted certificates.
An optional dependency-package test command could not run from the application workspace;
the application tests exercise the patched transport directly.
