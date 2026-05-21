# v7.8.0 Rust Set-and-Forget Candidate

This release adds the final stability-hardening layer before declaring the Rust authority path stable for unattended operation.

## Purpose

v7.7.0 introduced quarantine, last-good snapshots, and live soak monitoring. v7.8.0 adds set-and-forget evidence gates:

- transaction journal audit;
- non-destructive rollback drill;
- set-and-forget readiness evidence bundle;
- fail-closed runtime gate before promoted Rust production mutation;
- promotion-script enforcement of the evidence path.

## New scripts

```bash
bash scripts/rust-authority-journal-audit.sh
bash scripts/rust-authority-rollback-drill.sh
bash scripts/rust-set-and-forget-readiness.sh --write-stamp
bash scripts/verify-rust-set-and-forget-candidate.sh
```

## New runtime status

```text
rust_set_and_forget_gate_failed
```

## Readiness evidence

The readiness stamp is written to:

```text
/opt/LQoSync/state/rust_set_and_forget_readiness.json
```

The runtime gate requires this stamp to be fresh and passing when `rust_set_and_forget_candidate_enabled=true`.

## No-breakage boundary

Python still hosts the WebUI and scheduler shell. The promoted mutation path remains Rust-owned and fail-closed. The rollback drill is non-destructive and validates restore evidence without writing to live LibreQoS files.
