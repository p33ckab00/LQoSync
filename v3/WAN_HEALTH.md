# WAN health preview

The `wan-health-preview` CLI reads selected Netwatch probes and produces JSON health reports.
It does not change router configuration, save inventory, change assignments, or start a writer.
The [Core control page](CORE_CONTROL.md) can collect and display separate in-memory health and reconciliation previews.
Without WAN health configuration, the CLI reports `configured: false` and makes no router connections.

## Requirements

Use dynamic topology with mapped routing WANs and a core or both-role router.
The operator must configure two distinct external IPv4 ICMP probes per WAN separately.
Each probe needs a dedicated static main-table host route that resolves through the mapped WAN interface.
The route can use a next-hop IP on that interface, or the interface itself for a point-to-point WAN such as PPPoE.
Probe targets must be distinct across all configured WANs.
The preview does not create Netwatch entries, routes, scripts, or VRFs.

This first provider rejects enabled routing rules, probe scripts, explicit probe source bindings, and ambiguous routes.
It rejects output mangle rules, with one exception: reply routing for inbound connections.
An accepted output rule uses `mark-routing` and matches only a `connection-mark`.
Every enabled rule that sets that mark must be in `prerouting` with `connection-state=new`.
Probe connections start on the router, so such a mark cannot apply to a probe.
The provider reads current rules only. Existing connections keep old marks after a rule change.
After you change a mangle rule that sets a connection mark, remove the connections to the probe targets.
See the [reply routing design](../docs/superpowers/specs/2026-10-31-health-reply-routing-design.md).
It requires a uniquely enabled/running mapped interface and active static probe routes.
An absent or inactive probe route produces Unknown. This first provider does not yet infer physical-link failure from that condition.
Unsupported deployments require a separately verified provider before automatic failover can use their evidence.

## Local settings

The following example shows only the optional health section of `settings.toml`.
Keep the existing application settings. Replace the WAN and probe IDs with actual configured identities.
Do not add these sample settings to production without separate configuration authorization.

```toml
[wan_health]
interval_seconds = 10
evidence_seconds = 30
failure_rounds = 3
success_rounds = 3
recovery_seconds = 30

[[wan_health.monitors]]
wan = "wan1"
probes = [
  { id = "*A", target = "1.1.1.1" },
  { id = "*B", target = "8.8.8.8" },
]
```

Health intervals accept 1–300 seconds. Evidence lifetime must cover one interval and cannot exceed 300 seconds.
Failure and success thresholds accept 1–100 rounds. Recovery delay accepts 0–3600 seconds.
At most 64 WAN monitors are supported. Configured WANs must already exist in dynamic topology.
The initial sample establishes a counter baseline. It cannot prove recent completed probes by itself.

## Run the preview

Build from `v3/`:

```bash
cargo build --locked --bin wan-health-preview
```

Run one read-only round:

```bash
/opt/LQoSync3.0/v3/target/debug/wan-health-preview /opt/LQoSync3.0/v3/data
```

Run eight rounds to observe counter progress and recovery thresholds:

```bash
/opt/LQoSync3.0/v3/target/debug/wan-health-preview /opt/LQoSync3.0/v3/data 8
```

The optional round count accepts 1–60. The CLI waits the configured interval between rounds.
Each round prints one JSON object. It includes `router_writes: false`, WAN status, and a reason.
The process keeps health history only in memory. Each new invocation starts with Unknown.
Stop the process with Ctrl+C. The preview does not require a console restart or maintenance pause.

## Status rules

- Healthy: fresh advancing completed samples satisfy the success threshold and recovery delay.
- Unhealthy: both attributed probe targets fail for the configured number of new rounds.
- Unknown: evidence is absent, unsupported, stale, inconsistent, or not yet sufficient.

One successful target makes a round successful. Both probe counters must advance before a round can contribute.
Counter jumps count as one observed round. Repeated reads cannot refresh evidence freshness.
A router/probe reset, policy change, or clock reversal resets the baseline.
Evidence includes its canonical settings revision and WAN identity for downstream validation.
Consumers must check the completed-sample lifetime through `State::status_at`, not only the last read timestamp.
A failed read clears confidence. It does not prove that a WAN failed.
A confirmed state can persist through sub-threshold failures or recovery rounds until evidence expires.

These values are preview evidence. They do not count subscribers, change WAN assignments, or prove the actual traffic path.
The [reconciliation planner](CORE_RECONCILE.md) is a separate pure library. Modal controls and scheduled previews exist. Integrated mutation workers and rollout authorization remain pending.

## Verification limit

Tests use synthetic router inventories and temporary application directories.
They prove state transitions, conservative parsing, read-only CLI behavior, and settings compatibility.
They do not replace isolated verification of actual Netwatch API fields and completed counter behavior on the deployed RouterOS version.
The [official Netwatch manual](https://manual.mikrotik.com/docs/diagnostics-monitoring-and-troubleshooting/netwatch/) describes the counters and probe status.
Live automatic failover remains disabled pending that verification and the remaining implementation phases.
