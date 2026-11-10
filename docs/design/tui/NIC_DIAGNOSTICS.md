# Native DPDK NIC diagnostics

Implementation date: 2026-11-11. Branch: `lqosync3`.

## Verified source

The operator ran the read-only native telemetry probe against this host.
It found `/run/dpdk/rte/dpdk_telemetry.v2`, owned by `bridge_rs`, with DPDK 25.11.0.
Ports 0 and 1 identify PCI devices `0000:01:00.0` and `0000:01:00.1`.
Both ports reported UP at 10,000 Mbps with full duplex in three samples each.
Each port exposed 78 extended statistics. Basic error, missed-packet, and no-mbuf counters were zero in those samples.
These observations describe the probe interval. They do not establish long-term network health.

Evidence: `/home/pi/.local/state/lqosync3/dpdk-nic-task7/live-source.jsonl`.
The repository fixture retains one port's reply shape and numeric counters. It contains no subscriber identities.
The transport replay uses this fixture on a real local SOCK_SEQPACKET socket.

The [DPDK telemetry guide](https://doc.dpdk.org/guides/howto/telemetry.html) documents endpoint discovery and Ethernet statistics queries.
The implementation reads only `/`, `/ethdev/list`, `/ethdev/info`, `/ethdev/link_status`, `/ethdev/stats`, and `/ethdev/xstats`.

## Collection and ownership

`libreqos.v3_dpdk_telemetry_socket` selects the source. Its default is the verified runtime socket above.
The service queries that configured socket every ten seconds. It does not scan other DPDK instances during collection.
NIC collection has a separate cancellable task and a three-second request deadline.
A slow NIC source cannot block the two-second aggregate analytics sampler.
Configuration reload rejects pending results from a superseded NIC source or state directory.
A NIC-only setting change preserves the analytics histories and subscriber sampler state.
Service shutdown aborts the NIC task.

Each reply has a 64 KiB limit. Each report retains at most eight ports and 256 extended statistics per port.
Capped reports state their limits. The collector checks port IDs and device names again before publication.
The public `nic-telemetry.json` cache has a 1 MiB limit and atomic replacement.
The TUI reuses its parsed cache until the file changes. Frame rendering performs no file or socket reads.
No dependency, socket permission, router setting, or packet-path change is required.

## Measurement boundaries

Port RX/TX describes physical NIC observations. It does not identify subscriber download or upload.
NIC link speed is the reported Ethernet link speed. It does not measure WAN capacity.
Basic counters retain their DPDK names: `ipackets`, `opackets`, `ibytes`, `obytes`, `imissed`, `ierrors`, `oerrors`, and `rx_nombuf`.
Known zero values remain zero. Missing counters remain unknown.

Basic interval increments require consecutive successful reads, the same process and port identity, and a valid local time interval.
A source error, long gap, process change, port replacement, or clock regression clears the baseline.
A decrease in a basic counter marks a possible counter reset and suppresses that port's interval increments.
Intervals use local receive times. The native reply supplies no producer timestamp for these measurements.

Extended statistics retain their raw reported values and names.
Some vendor fields are gauges, including `tx_pp_jitter` and `tx_pp_wander`. The TUI derives no increments for vendor fields.
A decreasing vendor gauge does not reset the basic-counter baseline.
Extended statistics can overlap basic counters. Do not add them to estimate packet loss.
The implementation derives no loss percentage or combined error total.

## TUI contract

The approved panel order and traffic chart remain unchanged.
Dashboard Dataplane health shows per-port link state, speed, duplex, errors, missed packets, no-mbuf counts, and interval increments.
The existing health wheel, keyboard controls, and scrollbar provide access to the additional rows.
Workers details shows basic packet/byte totals and the bounded extended statistics list.
The existing detail scrolling and Copy action include these values.
The NIC source has its own LIVE, STALE, or UNAVAILABLE state and age. Its freshness limit is fifteen seconds.
Stale data does not supply current link states, counters, or increments.
Operational alerts report a fresh DOWN link or positive basic error increments.
An unavailable NIC source remains explicit in Dataplane health without adding duplicate default alerts.

Actual terminal captures use isolated synthetic data:

- [Dashboard NIC summary](captures/dashboard-nic-v1.png)
- [Workers NIC details](captures/workers-nic-v1.png)

## Verification and deployment state

The full suite passed 632 tests, with zero failed and seven manual tests ignored.
Formatting, all-target Clippy with warnings denied, build, whitespace, and terminal capture review passed.
Tests cover the native transport, malformed and oversized replies, missing endpoints, port identity changes, and counter limits.
They also cover resets, raw vendor gauges, freshness, cache bounds, reload ownership, poll cadence, shutdown, cache reuse, and copy behavior.
The existing compact-screen and mouse-scroll tests remain active.
Production now uses verified release `3.0.175+gb4e3846` from source commit `b4e3846`.
CI passed both jobs. The installed binary hash, process, successful cycle, and unchanged configuration passed verification.
Native source/cache checks verified peer and port identities, basic increments, extended field names, and advancing timestamps.
All seven installed TUI routes passed smoke. Deployment evidence is in `/home/pi/.local/state/lqosync3/nic-b4e3846/`.

The maximum cache fixture used 312,567 bytes. One hundred debug detail formats took 90.7 ms.
These synthetic timings do not guarantee a terminal frame rate.
