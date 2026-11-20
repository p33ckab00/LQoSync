# Service diagnostics banner

Status: implemented. Runtime installation is pending.

## Design

Show the ticker above the Dashboard heading, below the shared application header.
Use a thin strip with the existing Plex font, tabular numbers, and theme colors.
Do not add a card, title, subtitle, or pause control.
Roll continuously from right to left. Hover does not stop movement.
Use a static view when the device requests reduced motion.

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

Live values and activity expiry continue in both motion modes.
Full static diagnostics remain available under View diagnostics below the main panels.

## Atomic plan and result

1. Move the banner above the Dashboard heading. Keep static diagnostics independently available.

2. Keep the five permanent fields and their source values.

3. Compare only status metadata and component states. Do not copy traffic history into the activity tracker.

4. Add a bounded activity buffer with duplicate suppression and timed expiry.

5. Move one measured item at a time. Append activity at the tail. Hide the moving copy from assistive technology.

6. Test baseline behavior, state changes, repeated snapshots, expiry, and the eight-entry bound.

7. Check placement, compact height, absent controls, continuous motion, themes, reduced motion, and mobile layout in a browser.

8. Build, save evidence, document, commit, and push.

A small local timer expires old entries. The Dashboard disposes the timer on exit.
The change adds no network request or backend operation.
CSS moves each item at 32 pixels per second.
Rust measures the next item once at each handoff with one animation-frame callback.
There is no per-frame application loop.
Only `prefers-reduced-motion: no-preference` enables movement.
The static view wraps all current entries and hides the moving copy.

## Continuous movement

The previous track used a percentage of its total width.
Activity changed both that width and the animation duration.
A browser regression test measured an 83-pixel jump during insertion.

The new track uses a fixed pixel distance for the current item.
New activity goes to the tail. It does not change the active distance or duration.
Two sets of permanent fields keep the viewport covered.
The moving queue holds at most 18 items: ten status items and eight activity items.

After an item leaves the left edge, the track moves it to the tail.
A status item receives the latest source value at this point.
Visible text keeps its width until it leaves the viewport.
The accessible status list and static diagnostics receive source changes immediately.

The activity feed still expires entries after 120 seconds.
Expired waiting entries leave the queue beyond the right edge.
An expired visible entry finishes its pass before removal.
The strip does not remove text in the middle of a pass.

## Backend scope and gap

Activity entries are observations from this browser session, derived from the current overview.
They are not a durable service event history or a complete job-progress feed.
The current browser contract does not provide detailed orchestration activity events.
That feature needs an authenticated backend event projection before the UI can show those events.
Do not infer them from traffic, cycle duration, or an unchanged publication phase.

## Checks and deployment

Five native chart/activity tests, Wasm release build, Clippy, and Rust 1.89 checks passed.
Browser checks cover top placement, five permanent fields, conditional activity, duplicates, expiry, continuous motion, absent controls, and stale evidence.
They also cover Light/Dark themes, reduced motion, and 768/320px widths.
The existing sign-in and Dashboard smoke check passed.
See [current browser evidence](evidence/diagnostics-continuous/README.md).

Install the prepared web build:

```bash
cd /home/pi/LQoSync3.0
sudo tools/frontend/install.sh
```

## Skills

Vercel frontend-design keeps the existing product design and the requested ticker behavior.

Google modern-web-guidance `motion` says: "Provide a pause mechanism".
The owner explicitly requested no pause control. This requirement overrides that guidance for this strip.
The strip retains transform animation and the static reduced-motion view.
Reduced motion does not replace a pause control for WCAG 2.2.2. This strip does not meet that criterion.
