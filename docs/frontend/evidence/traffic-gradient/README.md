# Traffic chart gradient

Date: 2026-11-20 (Asia/Shanghai). Branch: `lqosync3`.

## Design

Use the accepted [Dashboard mockup](../../images/01-dashboard.png) as the visual reference.
The download area uses the blue accent. The upload area uses the green status color.
Each area fades to transparency at the zero baseline.
The existing theme tokens supply both colors in Light, Dark, and System modes.

Keep the measured line paths, scale, labels, and stale-data states.
Do not smooth samples or change the reported values.

The implementation uses two closed SVG paths and two native gradients.
Each gradient has a unique ID for its chart instance.
The stops keep one color and change only opacity. This prevents a fade through an unrelated hue.

The gradients follow each area's bounds. A small upload series thus retains a visible green fill.
No chart library, image request, or animation was added.
The existing 300-point limit remains in place.

Vercel frontend-design informed the restrained palette and the match to the approved mockup.

Google [visual-effects guidance](https://github.com/GoogleChrome/modern-web-guidance) informed the fade and the visible text values.
Its rule is: "always pair gradient charts with an accessible textual representation of the data".
The chart retains the Download and Upload labels and their Mbps values.

High-contrast mode hides the fills. It uses solid and dashed foreground lines.
The preference to reduce transparency also hides the fills.

## Rendering constraint

Leptos wraps static SVG fragments in groups. A gradient cannot use a stop inside such a group.
Bind the stop color as a Rust value to keep all stops as direct gradient children.
The browser inspection confirmed six direct stops across the two gradients.

## Checks


- Optimized Wasm build: passed.

- Rust 1.89 Wasm check: passed.

- Formatting and whitespace checks: passed.

- Chromium sign-in, themes, System changes, session persistence, and sign-out: passed.

- Gradient references after the next five-second refresh: valid.

- Light and Dark gradient colors: matched their theme tokens.

- Mobile at 320px: no horizontal overflow.

- High-contrast mode: fills hidden. The upload line uses a dash pattern.

- Browser errors: none.

These pictures show the implemented application with synthetic fixture data.
They do not show the production network.

| Dark | Light | Mobile |
|---|---|---|
| [Picture](dark.png) | [Picture](light.png) | [320px picture](mobile.png) |

Picture hashes are in [SHA256SUMS](SHA256SUMS).

## Apply the update

The browser assets were rebuilt in `crates/web-ui/dist`.
Run `sudo tools/frontend/install.sh` from `/home/pi/LQoSync3.0` to install the update.
The installer restarts the web services. The core synchronization service and TUI keep their current code.
The current local Traefik route returns HTTP 200.
This source change has not yet been installed into the production web service.
