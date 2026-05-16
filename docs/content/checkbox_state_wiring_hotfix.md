# Checkbox State Wiring Hotfix

LQoSync v2.70.6 fixes Config Center checkbox visual-state wiring.

## Problem

Some boolean policy values were true in `config.json`, but the corresponding checkbox did not always show a visible checked mark in the Config Center policy hierarchy.

The highest-risk area was the dynamic policy field renderer because it used a one-way checked binding without explicit boolean normalization and visual fallback styling.

## Fix

- Adds `asBool()` in Config Center JavaScript.
- Normalizes true-like values such as `true`, `"true"`, `1`, `"1"`, `yes`, and `on`.
- Updates policy boolean field checkboxes to bind with `asBool(getPath(...))`.
- Adds `x-effect` checked synchronization so the checkbox visual state follows current config state.
- Adds checkbox `accent-color` and checked outline styling for clearer checked-state visibility in light and dark modes.
- Extends UI wiring audit to catch missing checkbox state wiring.

## Scope

This is a UI/UX wiring hotfix. It does not change policy execution, config schema paths, generated files, scheduler behavior, backups, Telegram, or LibreQoS apply mechanics.
