# Phase 3 — Profiles and subscriber control

**Entry:** W01 contract. G1 ownership decision, including the exact profile-specific RouterOS/LibreQoS write mechanism, is required before P02. **Exit:** An authorized profile or subscriber change reaches its owning target, is read back, and has a visible audited result. WAN assignment writes remain a phase-5 concern.

| ID / owner | Depends | Implementation instruction | Acceptance evidence |
|---|---|---|---|
| P01 / Core backend | W01 | Define IPoE and PPPoE profile identity, baseline down/up rates, day/time/timezone rules, priority and manual override precedence. Implement pure `effective_at` and impact preview before persistence or external write. Specify DST gaps/folds and midnight windows. | Boundary, overlap, timezone and deterministic-preview tests. PROF-01…03. |
| P02 / Middle layer + core backend | P01, W01, G1 | Implement profile CRUD, subscriber authorization/profile assignment and PPPoE profile sync as role-checked commands. Build the profile-specific target adapter defined by G1; validate version/plan ID, enqueue bounded work, write only approved ownership fields, read back target state, and audit actor/result. Return pending, applied, rejected and failed distinctly. | Unauthorized/stale-version rejection; duplicate/retry/restart/partial-failure tests; no secret in response or audit. UI-08, SAFE-01…02. |
| P03 / Frontend | P01 DTO, P02 API | Build `/ipoe/profiles` cards and inline schedule editor, `/pppoe/profiles` router filter/cards, IPoE authorization/profile actions, and PPPoE profile/speed actions. Show effective value and source. Use the documented card/text placement and mobile form stacking. | Browser flows for create/edit/preview/apply/error/retry, keyboard/phone/zoom checks, real API response. UI-07…14, PROF-01…03. |

If the owner has not resolved the writable profile fields, P01 can finish as a pure preview; P02 and its active UI controls remain blocked. See [WH/HOW](../ste/WH_AND_HOW.md) and [route components](../docs/ROUTES_AND_COMPONENTS.md).
