# TUI btop++ refinement

Date: 2026-11-01 (Asia/Shanghai).
Status: Implementation brief for the operator's visual correction.

## Reason

The operator rejected the first cosmetics result as too far from btop++.
The first design specified colors and equal cards, but did not specify a clear visual hierarchy.
Use the [upstream screenshot](https://github.com/aristocratos/btop#screenshots) as the visual reference.

This brief replaces the named-color palette and equal Dashboard card grid in the first cosmetics spec.
Keep its operational, input, data, and apply constraints.
Do not change live configuration, install a binary, or restart a service.

## Direction

Use a large, full-width process graph panel above the operational panels.
Use a narrow left column for Service, Counts, and Versions.
Use a wider right column for Last cycle and Routers.
Keep all six panels, with sizes that follow their content.
At 80×24, reserve eight body rows for the process panel and twelve for the other panels.
With message banners, reduce routine detail and keep errors and action outcomes visible.

Draw CPU and RSS as multi-row braille history traces, with their current values and compact meters above.
Keep the current 120 samples and data sources.
CPU remains percent of one core, with a 100% trace scale and a clamped meter.
RSS remains current / window peak, in MiB.
Show unknown values and retained-history labels explicitly.
Use panel borders for PID, elapsed time, and other process context.
Do not add fake measurements or host-wide CPU/memory values.

Use RGB colors to separate the canvas, text, borders, and selection.
Use muted cyan for the interface accent and cool neutral text.
Use a green RSS trace and a cyan CPU trace.
Use tinted backgrounds for status pills, with readable colored text.
Use subtle alternating rows and a muted selection background.
Keep Mono on terminal defaults, with text and modifiers as the state indicators.

| Role | Dark | Light |
|---|---|---|
| Canvas | `#0b1014` | `#f4f6f8` |
| Text | `#d5dde5` | `#243744` |
| Secondary | `#85939f` | `#607583` |
| Accent | `#59c7d5` | `#006a80` |
| Border | `#365360` | `#71848b` |
| OK | `#8bc88c` | `#2b7453` |
| Warning | `#e0b66d` | `#815700` |
| Error | `#e77676` | `#b2393f` |
| Selection | `#173a45` | `#d7eaf0` |

## Implementation and proof

1. Update shared palette roles and style tests in `src/tui/theme.rs`.
2. Change Dashboard geometry, graph rendering, and compact summaries in `src/tui/dashboard.rs`.
3. Update geometry-dependent tests in `src/tui/app.rs`. Keep behavior assertions.
4. Export actual ratatui frames in Dark and Light at minimum and wide sizes.
5. Inspect the rendered frames. Run tests, clippy, fmt, and both debug and release binary builds.
6. Record the change and manual terminal limits in the project handoff.

No ASCII mockup is required. The previews must come from the actual renderer.
