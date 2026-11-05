# E03 — Synthetic full-cycle performance baseline

**Measured source:** `64f18ebf3563f3197814d6dad5c0a790ba10aed9` on `lqosync4`, before this report and harness. This is a baseline for the existing LQoSync3 core that LQoSync4 will port. It is not a production capacity claim or an LQoSync4 web benchmark.

## What was measured

The repeatable [E03 harness](../../../examples/e03_baseline.rs) calls `Runner::cycle` five times in one process. Its fake RouterOS connector returns a deterministic snapshot from one router: 70% PPPoE and 30% bound DHCP leases, one profile, and one DHCP server. Each cycle makes five fake RouterOS `print` calls (`secret`, `active`, `profile`, `dhcp-server`, `lease`). The harness uses the `v3_root_promotion` configuration with temporary state and LibreQoS directories, forces **shadow** mode, and checks that the first report includes every subscriber. It checks that native rendering has no skipped-circuit alert and that every warm cycle leaves the shaping candidate inode unchanged. The fake connector and temporary paths prevent router and production shaper access.

The outer elapsed time includes the entire `Runner::cycle` call, including status and snapshot writes. `cycle_ms` is the runner's own timer, which ends before status publication. Output bytes are the candidate shaping files only; the harness does not count status, snapshot, or state files. `max_rss_kb` is the process peak from `/usr/bin/time`, including fixture construction and all five cycles, not isolated cycle memory. The first cycle starts with empty candidate files; cycles 1–4 use unchanged input.

**Host:** Intel Core i5-4570T 2.90 GHz, 2 physical cores / 4 logical CPUs, 7.7 GiB RAM, Linux 6.8.0-147-generic x86_64. Rust/Cargo 1.89.0, release profile. This host used a temporary Rust toolchain and Zig 0.13 linker wrappers outside the repository because system `cargo` and a C linker were absent. A regular Rust 1.89 toolchain and C linker can run the commands below.

```bash
cargo build --release --locked --example e03_baseline
for target in v2_pair v3_pair v3_network; do
  for count in 1000 10000; do
    /usr/bin/time -f 'max_rss_kb=%M elapsed_s=%e' \
      target/release/examples/e03_baseline "$count" 5 "$target"
  done
done
```

The harness prints each cycle as `target,subscribers,run,elapsed_us,cycle_ms,api_print_calls,output_bytes,changed_files`. On this host, the binary path was `/tmp/lqosync4-target/release/examples/e03_baseline` because `CARGO_TARGET_DIR` pointed there. One five-cycle process was measured for each target and subscriber count. The table reports the median of warm runs 1–4; the range shows run-to-run spread.

| Target | Subscribers | First outer time | Warm outer median (range) | Warm `cycle_ms` median | Candidate bytes | Process peak RSS | Shaping files changed: first / each warm | Fake calls / cycle |
|---|---:|---:|---:|---:|---:|---:|---:|---:|
| `v2_pair` | 1,000 | 41.4 ms | 28.5 ms (26.5–29.7) | 25.5 ms | 122,922 | 15,488 KiB | 2 / 0 | 5 |
| `v2_pair` | 10,000 | 545.6 ms | 254.8 ms (237.1–295.2) | 245 ms | 1,231,701 | 93,776 KiB | 2 / 0 | 5 |
| `v3_pair` | 1,000 | 40.1 ms | 27.2 ms (26.4–29.0) | 24 ms | 123,937 | 15,252 KiB | 2 / 0 | 5 |
| `v3_pair` | 10,000 | 575.3 ms | 245.0 ms (233.1–269.2) | 237 ms | 1,241,716 | 93,776 KiB | 2 / 0 | 5 |
| `v3_network` | 1,000 | 53.3 ms | 37.8 ms (36.9–38.1) | 34.5 ms | 606,401 | 19,368 KiB | 1 / 0 | 5 |
| `v3_network` | 10,000 | 676.1 ms | 364.6 ms (355.8–381.9) | 358 ms | 6,072,959 | 140,212 KiB | 1 / 0 | 5 |

The 10,000-subscriber native JSON candidate is about 4.9 times the v3 pair bytes in this fixture. The native warm cycle and process peak were higher in this single trial. The candidate shaping files had **zero unchanged writes** in warm cycles; `status.json`, `snapshot.json`, and other state work still occur. These measurements do not include network latency, retries, multiple routers, Hotspot records, live LibreQoS imports, native daemon acceptance, API traffic, or browser rendering. There is no target hardware budget or accepted latency threshold yet.

## Existing write and import risks

| Question | Current evidence | LQoSync4 handoff |
|---|---|---|
| What if the v3 pair update stops between files? | [`files::write_if_changed`](../../../src/files.rs) replaces each file atomically, but `network.json` and `ShapedDevices.csv` form a two-file update, not one transaction. The [`runner` restart test](../../../src/runner.rs) simulates an old CSV after a new network file, restarts the runner, and verifies repair. No live interruption test exists. | Preserve the repair behavior and show source-written versus imported state separately. Keep backup and serialized apply. |
| When does `v3_pair` become imported? | The runner writes a scheduler reload request and compares imported topology. Its pending/delayed-import tests cover the state transition. The pending limit is 40 minutes; a mismatch then reports an error. The reload-request format is a LibreQoS internal interface. See [v3 ingress](../../LIBREQOS-V3-INGRESS.md). | Recheck the integration against the installed LibreQoS version before changing it. Do not present a successful file write as import success. |
| Can native JSON be applied safely? | The installed native compiler probe guards node-rate preservation. The native path has no equivalent accepted-generation readback, and the offline evidence found lost node rates. A skipped circuit blocks native apply. See [v3 data flow](../docs/LIBREQOS_V3_DATA_FLOW.md) and [E01](E01_BASELINE.md). | Keep the native production gate closed until node policy and accepted-generation readback are verified on the intended LibreQoS version. |
| What do five RouterOS calls mean? | Five are logical fake table reads for this one-router PPPoE/DHCP fixture, with no socket traffic. | Measure real router count, request latency, retry cost, and concurrent load separately before setting service budgets. |

## Next use of this baseline

For Phase 1 C01, retain the full-cycle shadow behavior, pair parity, unchanged shaping writes, restart repair, and native apply guard while removing the TUI. For C02, repeat the same measurement on representative hardware and topology, including multiple routers and Hotspot if used; define a numeric cycle and memory budget only after that evidence. Profile native rendering and allocation before optimizing it. Later web measurements must include API polling, paged tables, and browser behavior separately.
