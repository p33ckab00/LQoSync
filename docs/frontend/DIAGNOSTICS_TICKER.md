# Service diagnostics banner

Status: implemented. Runtime installation is pending.

## Design

Show the ticker above the Dashboard heading, below the shared application header.
Use the existing Plex font, tabular numbers, theme colors, and panel borders.
Keep Pause/Resume, hover/focus pause, and the static reduced-motion view.

The following five fields always remain in the ticker:

- Service health.
- Runtime mode.
- Output target.
- Publication phase.
- Last cycle.

Show the source publication value, such as `IMPORT_CONFIRMED`, without changing its meaning.
Do not replace a confirmed import with an applied-shaping claim.
Show the current cycle duration in milliseconds.

Component states do not form a permanent ticker list.
Show an activity entry only when the browser observes a new status or component change.
The initial snapshot establishes a baseline. It does not replay existing component states as activity.
Identical snapshots and traffic-only updates do not add entries.

A later status timestamp produces a status-snapshot update entry.
This entry does not prove that a job completed.

Keep at most eight recent entries. Each entry expires after 120 seconds, even without another source update.
Expiry removes activity entries only. The five status fields remain.

Pause stops movement. Live values and activity expiry continue.
Full static diagnostics remain available under View diagnostics below the main panels.

## Atomic plan and result

1. Move the banner above the Dashboard heading. Keep static diagnostics independently available.

2. Keep the five permanent fields and their source values.

3. Compare only status metadata and component states. Do not copy traffic history into the activity tracker.

4. Add a bounded activity buffer with duplicate suppression and timed expiry.

5. Keep the animation track stable. Hide the duplicate group from assistive technology.

6. Test baseline behavior, state changes, repeated snapshots, expiry, and the eight-entry bound.

7. Check placement, live updates, pause controls, themes, reduced motion, and mobile layout in a browser.

8. Build, save evidence, document, commit, and push.

A small local timer expires old entries. The Dashboard disposes the timer on exit.
The change adds no network request, backend operation, or JavaScript animation loop.
Only `prefers-reduced-motion: no-preference` enables movement.
The static view wraps all current entries and hides the duplicate group.

## Backend scope and gap

Activity entries are observations from this browser session, derived from the current overview.
They are not a durable service event history or a complete job-progress feed.
The current browser contract does not provide detailed orchestration activity events.
That feature needs an authenticated backend event projection before the UI can show those events.
Do not infer them from traffic, cycle duration, or an unchanged publication phase.

## Checks and deployment

Five native chart/activity tests, Wasm release build, Clippy, and Rust 1.89 checks passed.
Browser checks cover top placement, five permanent fields, conditional activity, duplicates, expiry, pause, and stale evidence.
They also cover Light/Dark themes, reduced motion, and 768/320px widths.
The existing sign-in and Dashboard smoke check passed.
See [current browser evidence](evidence/diagnostics-ticker-top/README.md).

Install the prepared web build:

```bash
cd /home/pi/LQoSync3.0
sudo tools/frontend/install.sh
```

## Skills

Vercel frontend-design keeps the existing product design and the requested ticker behavior.
Google modern-web-guidance `motion` says: "Provide a pause mechanism".
The implementation keeps the visible keyboard control, transform animation, and static reduced-motion view.
