# Rust Core v0.5 Policy Shadow Engine

This package adds the first Rust-side policy decision engine for the `lqosync-in-rust` branch.

## Purpose

The Rust policy engine is introduced in **shadow mode**. Python remains the authoritative policy engine for cleanup, write, and apply decisions. Rust computes a parallel verdict so operators can compare behavior before any future authority switch.

```text
Python run_cycle.py
  ↓
Python policy_engine.py decides actual behavior
  ↓
Rust evaluate-policy receives the same context
  ↓
Rust returns shadow verdict / risk / parity
  ↓
Dry Run displays comparison
```

## New protocol operation

```text
evaluate-policy
```

Request payload:

```json
{
  "config": {},
  "preflight": {"errors": [], "warnings": []},
  "collector_trust": [],
  "cleanup": {
    "sources": [],
    "candidates": 0,
    "removed": 0,
    "queued": 0,
    "preserved": 0
  },
  "rust_validation": {},
  "python_policy_decision": {},
  "diff_summary": {}
}
```

Response result:

```json
{
  "verdict": "safe_to_apply",
  "risk_score": 0,
  "risk_level": "low",
  "apply_allowed": true,
  "write_allowed": true,
  "cleanup_allowed": true,
  "blocked_reasons": [],
  "warnings": [],
  "recommendations": [],
  "decision_trace": [],
  "parity": {
    "available": true,
    "matches_verdict": true,
    "matches_risk_level": true,
    "matches_write_allowed": true,
    "matches_apply_allowed": true
  },
  "mode": "shadow",
  "authoritative": false
}
```

## Safety model

- Rust policy is **not authoritative** in v0.5.
- Python policy still controls actual cleanup/write/apply behavior.
- Rust policy output is stored in `result.diff.rust_policy_shadow`.
- Dry Run displays Rust verdict, risk, and Python/Rust parity.
- If parity differs, the UI warns but does not alter production behavior.

## What Rust evaluates now

The v0.5 policy shadow engine evaluates:

- preflight errors and warnings
- collector trust failures and cleanup holds
- Rust validation failures
- cleanup candidate/removal/queue/preserve counts
- apply guard settings such as duplicate IP, missing parent, invalid speed, and collector failure blocks
- parity against Python `policy_decision`

## What remains Python-authoritative

These stay Python-owned for now:

- cleanup queue mutation
- pending confirmation creation
- stale lifecycle mutation
- returned-client lifecycle tracking
- exact source-specific cleanup action execution
- final write/apply blocking

## Operator workflow

After installing v0.5:

```bash
scripts/build-rust-core.sh
sudo scripts/install-rust-core.sh
sudo scripts/install-rust-core-daemon.sh
```

Then run Dry Run and inspect:

```text
Dry Run → Rust Policy Shadow
```

A parity mismatch means the Rust shadow logic and Python policy logic do not yet agree. Python remains the source of truth until parity is stable across real production cycles.
