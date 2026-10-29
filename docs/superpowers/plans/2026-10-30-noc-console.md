# Compact NOC Console Implementation Plan

> **For agentic workers:** Use superpowers:executing-plans. Track each task below.

**Goal:** Apply the user's supplied NOC design to the actual Rust console.

**Architecture:** Reuse the supplied CSS and the shared Maud shell. Preserve current mutations and backend policies.

**Tech Stack:** Rust, Axum, Maud, CSS, and a small same-origin JavaScript asset.

**Spec:** `docs/superpowers/specs/2026-10-30-noc-console-design.md`

## Global Constraints

Preserve private data, current forms, ownership checks, and publication gates. Add no dependencies.
Work in the current feature checkout. Preserve prior documentation changes.
The supplied reference is the user's selected visual design. No more visual choice is necessary.

## Review Focus


- Names containing HTML must remain escaped.

- Missing inventory must show unknown state, not success.

- Snapshot counts must not appear as online session counts.

- Mobile navigation and forms must remain usable.

- Script delivery must retain the same-origin content security policy.

## Tasks


- [x] Add focused HTTP tests for the shared shell, script, and selected-node inspector. Check failures.

- [x] Import reference CSS. Add compatibility rules for current forms and panels.

- [x] Implement the shared shell, grouped navigation, density control, and mobile navigation.

- [x] Adapt Overview and Routers to actual data with the reference panel layout.

- [x] Adapt Topology to inventory, hierarchy, and inspector panes. Keep current save/delete handlers.

- [x] Run the Rust checks. Use isolated browser checks for all pages, mobile navigation, and inspector selection.

- [x] Review the diff and record results in the project vault.

## Execution notes

The user supplied the exact reference and asked to follow it. Execute this visual change in the current session.
Do not restart the console, change production data, commit, or push as part of this change.

## Results

Rust tests: 130 passed, zero failed, nine manual benchmarks ignored.
Format, strict Clippy, and the binary build passed.
Browser checks passed on 12 routes at widths 1440px, 390px, and 320px.
The review found a mobile focus defect. The fix passed a reproduced browser check and follow-up review.
