# Sign-in design and application themes

Status: owner direction and picture previews. Application implementation has not started.
The owner accepted the original sign-in composition and requested glass surfaces, a superscript version, and application-wide themes.

## Approved visual direction

Keep the original sign-in layout, layered logo mark, network illustration, text, and field placement.
Add a small superscript `v3` to the LQoSync wordmark. Do not change the logo mark's geometry.
Use the same brand treatment across the application.

Apply restrained frosted glass to the sign-in panel with a translucent surface, subtle blur, and fine edge highlights.
Keep text, input fields, focus rings, and buttons legible in both themes.
Use an opaque surface when blur is unavailable, reduced transparency is preferred, or contrast requires it.
Do not animate blur or apply large blur layers to every operational table.

## Picture references

| Image | Status |
|---|---|
| [Original sign-in](images/12-sign-in-original.png) | Owner-approved composition before glass and version changes |
| [Glass dark](images/13-sign-in-glass-dark.png) | Dark variant for review |
| [Glass light](images/14-sign-in-glass-light.png) | Light variant for review |

The new variants preserve the original composition as closely as image generation allows.
The theme selector in each picture is illustrative. It does not prove working theme behavior.
No signup, password reset, or account-management control belongs on this screen.

## Theme behavior

| Selection | Required behavior |
|---|---|
| `System` | Default. Follow the accessing device/browser color-scheme preference, including changes while the page is open |
| `Light` | Keep the light theme until the user changes the selection |
| `Dark` | Keep the dark theme until the user changes the selection |

Apply the selected theme to sign-in, navigation, forms, tables, charts, dialogs, and native browser controls.
Use the client's environment, not the server's theme or timezone, to resolve System.
Show the Light / Dark / System selector on sign-in and in the shared main application header.
Keep it visible and reachable on Dashboard and every feature screen, including phone layouts.
Do not hide it exclusively inside global Settings.
Theme selection is a local presentation preference, not an account or operational configuration mutation.

Persist only the non-sensitive override for this site. Returning to System removes the override.
Do not clear a manual override merely because the system preference temporarily matches it.
If persistence fails, keep the selector usable and fall back to System on the next visit.
Never store credentials or session tokens with theme preferences.

## Rust and browser implementation boundary

Implement selection and state changes in Rust/Wasm and CSS.
Use `color-scheme` on the root element and declare supported schemes early in the HTML head.
Keep root CSS consistent with the resolved preference. Updating metadata alone must not leave CSS pinned to another theme.

Use CSS system-preference queries for System mode. If Rust reads `matchMedia`, also handle changes and remove listeners on teardown.
Define semantic light/dark color tokens with the browser fallbacks required by the current Google guides.
Preserve focus, entered values, and validation messages when the theme changes.

F05 must choose and document early preference restoration without hand-written JavaScript application logic.
A non-sensitive preference cookie can let the Rust gateway set initial root attributes before the Wasm bundle starts.
If that approach is selected, accept only `light` or `dark` and clear the cookie for System.
Do not add SSR or a new framework solely for this preference.

## Acceptance checks for F05 and F16

- Check all three selections and runtime system changes before and after sign-in.

- Check manual override persistence, System reset, disabled storage, and the initial paint on a cold load.

- Check native inputs, scrollbars, charts, status badges, focus rings, and error messages in both themes.

- Check keyboard access, narrow screens, 200 percent zoom, forced colors, and reduced-transparency fallback.

- Measure blur cost on representative low-end hardware and verify that theme changes do not start backend jobs.

Google guides reviewed: `dark-mode`, `dark-mode-toggle`, and `autofill-sign-in-form`.
Vercel frontend-design informed visual continuity and restrained composition.
These references guide implementation. The pictures do not establish accessibility or performance compliance.
