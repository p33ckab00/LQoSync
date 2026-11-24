# Topology Studio integration evidence (I0)

This file records the I0 integration of the backend lane and the frontend lane.
It is test evidence from a synthetic router and a test runtime. It is not production acceptance.

## Integration commit

| Item | Value |
| --- | --- |
| Branch | `integration/topology-studio`, from `lqosync3` at `a0e6035` |
| Backend lane | `claude/topology-studio-backend` at `86b9aa4`, merged in `77cad12` |
| Frontend lane | `codex/topology-studio-ui` at `8b4c7e3`, merged in `c9ee2f9` |
| Merge conflicts | None. The merged contract is equal to the backend contract at `86b9aa4`. |
| Integration fixes | `d0bc5f1`, `7d10aa7`, `41190c0` (see below) |
| Checked commit | `41190c0` |
| Time | 2026-11-24T13:43:50+08:00 |

## Integration fixes

1. `d0bc5f1`: the service did not refresh `source-studio.json` after a save.
   The source-config GET showed the old policies and the old policy revision until the next cycle.
   The real UI found this fault. The fixture checks did not find it, because their route mocks changed the answer.
   The shared commit helper now writes the cache after each registry change.
2. `7d10aa7`: contract v1 refuses a `read_access_ref` other than `default`.
   A new policy now uses `default`. The editor disables other profiles and shows the reason.
3. `41190c0`: the smoke test now drives the real Wasm UI through the real gateway, broker, and core.
   The check mocks no request. It compares the UI receipt with the service and the registry.

## Checks on `41190c0`

All checks ran in sequence on one host.

| Check | Result |
| --- | --- |
| `cargo fmt --all --check` | Pass |
| `git diff --check lqosync3 HEAD` | Pass |
| `cargo clippy --locked --workspace --all-targets --examples -- -D warnings` | Pass |
| `cargo clippy --locked -p web-ui --target wasm32-unknown-unknown -- -D warnings` | Pass |
| `cargo test --locked --workspace --no-fail-fast` | 2109 passed, 0 failed, 13 ignored |
| `STAGES=browser bash tools/frontend/check-all.sh` | 29 checks and 2 builds pass. `pulse-check` is skipped by default (known failure from before the web work). |
| `bash tools/integration/studio-service-smoke.sh` with Playwright | 22 of 22 pass. See [smoke-evidence.txt](smoke-evidence.txt). |

## Real-service flow

The smoke test starts the fake RouterOS server, the core service, and the gateway with its broker.
All files are under one temporary root. Then the browser does these steps in the real UI:

1. Sign in and open the topology canvas.
2. Open the router source configuration for `Smoke-01`. The editor shows the policy that the service saved (`PPPoE`).
3. Change the policy name. Refresh discovery. The service reads the fake router.
4. Preview the draft. The canvas shows `basic-Smoke-01` at 15 / 15 Mbps from the fake router profile.
5. Save the configuration. The UI shows `Saved policy revision 4` and `Runtime acceptance is not confirmed`.

The check then confirms these facts:

- The UI sent only `collect` and `evaluate` source jobs before the save.
- The UI operation has the state `saved` and the receipt revision 4 (base 3).
- The service source config and the registry hold the name `PPPoE from UI` at revision 4.
- The fake router received no write command. No log contains a password.

| Screenshot | Content |
| --- | --- |
| [studio-real-preview.png](studio-real-preview.png) | Read-only preview from the real service |
| [studio-real-save.png](studio-real-save.png) | Saved receipt from the real service |

[SHA256SUMS](SHA256SUMS) gives the checksums.

## Not done in this evidence

- Publication by the topology writer. Activation needs the live-test gates.
- Production binaries as root, the production config, and a real router.
- A 5,000-subscriber run, restart recovery, and expired-preview tests on the real service.
  The unit tests and the fixture checks cover these cases. The real-service smoke test does not.
- Canvas bounds at 768 and 390 CSS pixels on the real service. The fixture checks measure all three widths.
- A second independent review of the frontend F3 corrections.
- A merge of `integration/topology-studio` into `lqosync3`. The owner decides this step.
