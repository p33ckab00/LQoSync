# LQoSync3 Rust web frontend

Status: sign-in and Dashboard implementation. Live deployment is pending.
See [first slice](FIRST_SLICE.md) and [deployment](DEPLOYMENT.md).
Date: 2026-11-20 (Asia/Shanghai). Backend baseline: `9a465e7`, branch `lqosync3`.

The web frontend will use the existing LQoSync3 service.
The browser application will use Rust, Leptos, and WebAssembly, as selected in the LQoSync4 documents.
CSS, HTML assets, and generated Wasm bindings remain necessary.

Do not add hand-written JavaScript or TypeScript application logic.
The web UI is the primary frontend and operator workflow surface.
The current TUI stays as is and remains independently available.
The Rust service remains the execution authority and continues without a browser.
[Account administration](CLI_ACCOUNT_POLICY.md) is CLI-only. The web supports sign-in/sign-out and has no signup or user-management screen.

## Read order

1. [Feature design](DESIGN.md): scope, ownership, layouts, and behavior.
2. [Screen catalog](SCREENS.md): 11 initial screen concepts plus three sign-in references, intended features, and corrections.
3. [Backend map and gaps](BACKEND_MAP.md): current code, missing functions, and task owners.

4. [API contracts](CONTRACTS.md): proposed reads, actions, identity, and failure rules.
5. [Quality requirements](QUALITY.md): performance, reliability, integrity, and browser checks.
6. [Atomic implementation plan](IMPLEMENTATION_PLAN.md): dependencies, files, interfaces, and checks.

7. [Sources](SOURCES.md): evidence, skill references, and image provenance.

8. [CLI account policy](CLI_ACCOUNT_POLICY.md): username/password CRUD and multiple superadmins, with web sign-in only.

9. [Screen-owned settings](SCREEN_SETTINGS.md): feature configuration placement and shared header theme control.

10. [Company branding](COMPANY_BRANDING.md): company name/logo beside LQoSync, configured in global Settings.

## Scope and evidence

The [sign-in and theme direction](THEMING.md) preserves the accepted logo/layout, adds superscript `v3`, and requires Light, Dark, and System modes.

The pictures illustrate the LQoSync4 blueprint. They contain sample data and proposed controls.
They are not screenshots of an implemented application.
The written design takes precedence over generated text, counts, dates, and graphs.

The backend map separates reusable code from missing web integration and deferred domain features.
No plan task is complete merely because its picture exists.

The first release supplies authenticated monitoring and supported topology edits.
RouterOS authorization, subscriber disable, router profile writes, and automatic routing changes remain separate domain projects.
Recurring schedules need their own backend rules before the web editor can save them.

The roadmap keeps these gaps visible without presenting nonfunctional controls.

## Delivery rules

Implement one task at a time in dependency order.
Connect every released screen to a tested backend read or action.
Never ship fixture records through production endpoints.

Keep source ownership, saved policy, published input, imported state, and dataplane evidence distinct.
Read the relevant source again before implementation because this baseline can change.

The documents use short technical English under the ASD-STE100 skill.
The STE checker is heuristic. Its output does not establish certified compliance.
