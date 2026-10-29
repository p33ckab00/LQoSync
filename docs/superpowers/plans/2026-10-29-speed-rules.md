# Speed Rules Implementation Plan

**Goal:** Resolve subscriber speeds from RouterOS inventory and show the rule source in the Rust UI.
**Architecture:** A pure speed module parses router rates and legacy comment rules. An indexed preview joins inventory tables.
The UI edits application-local minimum rules and shows bounded previews. This slice does not replace subscriber snapshots.
**Tech stack:** Rust, Axum, Maud, TOML. Reuse the existing RouterOS transport and publisher.

## Constraints

- Do not change LibreQoS files, packages, services, or shaping.
- Do not use live router credentials or send router writes.
- Do not add local default maximum speeds. Missing or invalid finite rates remain unresolved.
- Keep subscriber maxima separate from node factors and WAN caps.
- Preserve existing settings through Serde defaults.

## Rule specification

PPP precedence follows the reference script: secret comment, active comment, profile comment, profile rate, profile name.
Comments use subscriber download/upload order. Router rate fields use router receive/transmit order and bare bits per second.
A symmetric router rate is valid. Named profile fallback requires a unit to avoid treating an identifier as a speed.
DHCP uses lease comment, then lease rate. Hotspot uses local user comment, profile comment, then profile rate.
Hotspot has no invented rate when a local user/profile mapping is absent, including unknown RADIUS policies.
Burst fields are validated but not translated into LibreQoS bursts. Expose this limit in the preview.
Use explicit router minima when present. Otherwise use the configured fraction and floor, capped at the router maximum.
Invalid or unlimited explicit rates must not fall through to a guessed profile-name speed.
Malformed or conflicting comment rates remain errors. Ordinary comments without speed continue to the next rule.

## Tasks

- [x] Add failing tests for asymmetric directions, units, source precedence, minima, malformed values and no default maximum.
- [x] Implement pure speed parsing and application-local minimum policy validation.
- [x] Add allowlisted non-secret Hotspot mapping fields and PPP active comments.
- [x] Add indexed inventory preview with duplicate/missing-table checks and per-subscriber errors.
- [x] Add rule forms and read-only preview pages. Protect changes with existing form tokens.
- [x] Test snapshot/generation preservation and old TOML compatibility.
- [x] Measure large synthetic previews. Review performance and integrity independently.
- [x] Run formatting, tests, strict lint, update notes, commit and push `lqosyncv3`.

## Review focus

- Router and subscriber direction must not be confused for asymmetric rates.
- Bare rates, unlimited rates and profile-name numbers must not create huge or invented speeds.
- Partial tables and duplicate identities must not select arbitrary rates.
- Large inventories must not cause repeated profile scans or unbounded HTML pages.
- Speed previews and policy saves must not replace an existing subscriber snapshot or generation.


## Result

Added `speeds.rs` for pure parsing and `speed_preview.rs` for indexed inventory joins.
Added `speed_web.rs` for policy forms and bounded previews.
Existing TOML files remain readable. New settings persist under the application directory.

Verification: 39 functional tests passed. The manual performance test passed separately.
The last measurement was 220.3 ms for 50,000 PPP sessions and 100 shared profiles, using a debug build.
Strict Clippy, formatting, and diff checks passed.
HTTP smoke verified the policy page, protected save, and local TOML persistence.
It created no subscriber snapshot or generation.
Independent review found numeric token and duplicate-row defects. Regression tests reproduced them before correction.
Follow-up review found a whitespace regression. The final parser preserves ordinary free-text comment rules.
All findings are resolved. A queued-worker test proves concurrent policy changes invalidate the preview.
A final syntax test also rejects rate units in burst-time fields.
Production LibreQoS files, packages, services, and routers remain unchanged.
