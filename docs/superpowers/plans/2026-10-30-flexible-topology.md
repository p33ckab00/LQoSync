# Flexible Topology Implementation Plan

> **For agentic workers:** Use superpowers:executing-plans with delegated independent modules and a final independent review.

**Goal:** Let operators select and arrange discovered sources, with automatic count assignment on new sessions and reconnects.

**Architecture:** Add independent source selections. Keep committed assignment state separate from candidate session observations.

**Tech Stack:** Rust, Axum, Maud, TOML, current RouterOS read allowlist.

**Spec:** `docs/superpowers/specs/2026-10-30-flexible-topology-design.md`

## Global constraints

Preserve the NOC design, private data, manual core ownership, validation gates, and last valid generation.
Add no dependencies. Do not write RouterOS configuration or restart live services.

## Review focus

- Source identity remains stable after hierarchy edits.
- Only healthy source evidence can establish an absent session.
- Proposed state never masquerades as committed or applied output.
- Repeated scans keep continuous assignments and do not generate needless file pairs.
- Work grows with clients and eligible destinations, without a scan of all selectors for each client.

## Tasks

- [x] Add selector records, indexed matching, group destinations, and backwards-compatible settings defaults.
- [x] Add source controls and automatic branch creation from fresh inventory. Preserve manual node editing and deletion gates.
- [x] Add online-count pool metric and deterministic WAN/destination scoring. Preserve optional planned-load scoring.
- [x] Add session evidence, reconnect planning, durable candidate state, and failed-observation guards.
- [x] Integrate collection, candidate persistence, publication, live validation, and core planning without weakening gates.
- [x] Prove healthy empty scans, failed reads, restarts, pin overrides, and core conflict behavior.
- [x] Run full checks, large-input workloads, browser checks, and independent code/performance review. Fix material defects.
- [x] Record the milestone and commit/push the verified change. Verify the remote head.

## Ownership

Source selection worker: selection, normalizer, source UI, and new source tests.
Session worker: session module and new session tests.
Main agent: shared model, assignment scoring, orchestration, validation, NOC integration, documentation, checks, and Git operations.

## Results

The full suite passed: 177 tests, zero failures. Eleven manual benchmarks remain ignored in the normal suite.
Four relevant 50k workloads passed separately. Format and strict all-target Clippy passed.
Browser checks passed on 12 routes at three widths. Parent edits and source/metric controls also passed.
Independent review findings received regression tests and corrections. Final review found no material issue.
Candidate persistence reuses validated bytes and rejects intervening writes without another TOML parse.
This plan forms part of the verified behavior commit. Verify the branch tip against the remote before takeover.
