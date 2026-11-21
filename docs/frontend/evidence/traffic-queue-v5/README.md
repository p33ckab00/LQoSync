# Traffic and queue dynamics v5 evidence

Status: prototype checks passed. Date: 2026-11-21 (Asia/Manila).

The prototype uses synthetic data and local files only.
It does not contact the running Dashboard, LibreQoS, routers, or subscriber services.
Production Rust code and services did not change for this task.

## Reproduce

From `/home/pi/LQoSync3.0`, run:

```bash
PLAYWRIGHT_MODULE=/tmp/lqosync-web-tools/node_modules/playwright \
CHROMIUM_PATH=/home/pi/.cache/ms-playwright/chromium-1248/chrome-linux64/chrome \
node tools/frontend/traffic-queue-mock-check.cjs
```

The check uses the installed Chromium executable explicitly.
The Playwright default points to a different browser revision that is not installed.
No package installation is necessary.

## Results

- Light, Dark, and System follow the selected setting. The explicit theme overrides the operating system.
- The chart measures whole-bridge DPDK activity only. It has no subscriber selector or subscriber RTT metrics.
- Hidden series keep their values in the inspector.
- Pinned timestamps remain stable when new samples arrive.
- An expired timestamp shows an unavailable state.
- Pointer and touch can pin a sample. Keyboard controls can select and clear it.
- Gaps split paths. Unavailable sources draw no rate path.
- Counter resets have no drop or mark delta.
- Unknown capacity removes the capacity gradient and shows an observed scale.
- Stale values keep a visible age label.
- The table contains all 61 sample timestamps.
- The page fits 1440, 768, 390, and 320 px widths.
- SVG labels fit their viewBox at all four widths.
- Text and metric colors meet 4.5:1 contrast against the panel in both themes.
- The browser reported no script errors or HTTP requests.
- XML checks passed for all three SVG exports. They contain no scripts or external images.
- `git diff --check` passed.

Desktop Light, desktop Dark, and narrow renders received visual review.
The optional UI/UX inspector is not installed. Playwright supplied the browser checks.
The STE checker flagged technical names and some prose. Manual review corrected the long paragraphs.
Its heuristic result does not establish certified ASD-STE100 compliance.

## Artifacts

| View | SVG | Screenshot |
| --- | --- | --- |
| DPDK Dark | [SVG](../../images/traffic-queue-dynamics-v5.svg) | [PNG](traffic-queue-dynamics-v5.png) |
| DPDK Light | [SVG](../../images/traffic-queue-dynamics-v5-light.svg) | [PNG](traffic-queue-dynamics-v5-light.png) |
| DPDK narrow | [SVG](../../images/traffic-queue-dynamics-v5-mobile.svg) | [PNG](traffic-queue-dynamics-v5-mobile.png) |

The SVG exports contain paths and text. They are static coding references.
Use [the HTML prototype](../../prototypes/traffic-queue-dynamics-v5.html) to inspect dynamic behavior.
See [the research](../../TRAFFIC_QUEUE_DYNAMICS.md) and [the implementation plan](../../plans/2026-11-21-traffic-queue-dynamics.md).

Production DPDK integration and production Rust checks remain open in the plan.
