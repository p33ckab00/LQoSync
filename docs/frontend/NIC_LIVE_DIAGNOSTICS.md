# Live NIC diagnostics

Date: 2026-11-21 (Asia/Shanghai).

## Shared update cadence

The owner requested RX/TX activity alongside Actual, Mapped, and Unmapped traffic updates.
The core now targets one NIC counter sample each second.
The web broker reads NIC and worker caches in the same one-second refresh.
One overview event supplies both the traffic chart and NIC card.
The browser does not start another telemetry poll.

These sources have independent capture times and directions.
Physical RX/TX does not equal aggregate delivered traffic or mapped/unmapped enqueue traffic.
The UI does not copy aggregate activity into every port.
Each LED pulses once for its own fresh, distinct NIC interval with positive activity.
A duplicate frame, initial baseline, idle interval, reset, down link, or stale source does not create a pulse.
The current 180-ms pulse and Reduced Motion behavior remain.

The one-second cadence is a target, not a timing guarantee.
The collector uses the actual elapsed interval for counters.
It captures the counter timestamp before optional hardware probes. Probe duration does not inflate or shorten the rate interval.

A slow endpoint can delay a sample. Missed ticks do not cause catch-up bursts.
An unsuccessful read has a ten-second retry interval.
The TUI keeps its current presentation and reads the same cache.

## Hardware probe budget

The collector refreshes hardware identity every thirty seconds.
Between refreshes, it skips the optional DPDK module query and kernel queries.
It reuses identity only for the same producer PID, port ID, device name, and link state.
A changed identity schedules a new probe. A settings reload clears the identity cache.
The current socket timeouts, reply limits, and kernel query limits remain.

## Diagnostics panel

The current Vercel frontend-design and Google modern-web-guidance guide the interface.
The panel keeps the approved theme colors, type, and physical-port artwork.
Its translucent surface has a subtle blur, thin border, and solid fallback.
It appears below the ports when space permits, or above them otherwise.
Its content scrolls within the viewport.

- Hover opens the panel after 180 ms. A short exit delay permits pointer entry into the panel.
- Enter, Space, click, or tap opens a persistent panel.
- Escape or Close dismisses the panel. Close returns focus to the physical port.
- Counter details stay open across source updates.

RX/TX sample figures show bytes and packets for the measured interval.
Use these conversions:

```text
Mbps = delta_bytes * 8 / (interval_ms * 1000)
packets_per_second = delta_packets * 1000 / interval_ms
```

Missing measurements stay unknown. A reset or stale source hides current rates.
Cumulative health totals remain visible as last-known evidence.
A lifetime error total does not establish current loss.

Two micro charts show recent RX/TX throughput with independent scales.
Each chart keeps at most sixty distinct valid samples within the last sixty seconds.
A gap longer than three seconds or an invalid interval starts a new trend segment.
The charts do not join valid points across missing evidence.
The RX and TX axes use separate observed maxima. Compare the numeric rates for magnitude.

The composition bar uses only `ipackets`, `rx_broadcast_packets`, and `rx_multicast_packets` when all exist and fit the total.
It labels these shares approximate because driver semantics can differ.
Unknown names or impossible totals produce an unavailable state.
The counter table shows the bounded published details. The TUI can show more driver counters.

## Validation and install

See [screenshots and checks](evidence/nic-live-hud/README.md).
See [the atomic plan](plans/2026-11-21-nic-live-hud.md).
Build and install the core and web artifacts with the [NIC update procedure](DEPLOYMENT.md#nic-faceplate-update).
The implementation checks use synthetic loopback data. Production services do not change during these checks.
