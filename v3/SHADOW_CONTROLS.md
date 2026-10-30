# Shadow coordination and recovery inspection

Core control remains a read-only preview. Automatic writes remain disabled.
The offline worker still requires an exclusive lease. It cannot run beside the normal console.
See [Core control](CORE_CONTROL.md) and [the offline worker](CORE_WORKER.md).

## Pause the preview

1. Open **Core control** in the console.
2. Select **Pause shadow preview**.
3. Check that the policy shows **Paused**.

The action checks the form token and current settings revision before it saves the paused policy.
Pause invalidates active shadow tickets in that console. The console discards their results.

A computation that already started can finish its CPU work. It cannot authorize a router mutation.
To resume previews, use the policy cog and select **Preview**. Automatic mode remains blocked.
Ordinary inventory collection uses separate controls. Pause does not disable that collection.

One shadow computation can hold the lock for each canonical data directory.
A process exit releases this lock. It does not remove any recovery marker.
The existing console and maintenance leases remain unchanged.
The worker compares persisted settings before and after computation. An external pause blocks results when those settings differ.
These controls do not provide a distributed mutation grant or recovery approval.

## Inspect saved recovery records

Use **Inspect saved writer journals** from Core control when the normal console can start.
If a recovery marker blocks console startup, use the separate read-only viewer.
Do not remove the marker to start the normal console.

Build the viewer in your source checkout:

```bash
cd /opt/LQoSync3.0/v3
cargo build --locked --bin core-recovery-view
```

Start the viewer in the foreground:

```bash
/opt/LQoSync3.0/v3/target/debug/core-recovery-view /opt/LQoSync3.0/v3/data 127.0.0.1:9304
```

Open `http://127.0.0.1:9304` on the host. Use an SSH tunnel for remote access.

If you set `CARGO_TARGET_DIR`, use the binary in that directory instead.
Press **Ctrl+C** to stop this viewer. The normal console does not need a restart.

The viewer binds only to a loopback address. It has GET routes and rejects mutation requests.
It does not open the normal application, load credentials, contact routers, or get an executor lease.
It reads existing local records. It does not create or change journals, markers, or settings.

## Read the report

- **Pending intent:** A saved request has no known result. Do not retry or infer its outcome.

- **Last known result:** A saved reply exists. This does not prove post-write verification or current router membership.

- **Saved verification:** The journal records completed batch verification. This is historical evidence. Fresh verification remains necessary.

- **Inspection unavailable:** File access, a changed marker, or an inspection limit prevents a complete report.

The viewer reads at most 16 journals, 128 directory entries, and 64 MiB of journal text per inspection.
It rejects symlinks, non-regular files, unsupported marker paths, and invalid journal history.
One asynchronous inspection runs at a time per viewer process. A limited report is incomplete.
Records from the protected-row maintenance engine have a different schema. Use [its recovery guide](CORE_MAINTENANCE.md).

Approval, resume, rollback, and marker removal remain locked. There is no HTTP action to bypass these limits.
Actual online writer coordination and recovery grants remain pending.
Isolated RouterOS acceptance and separate production authorization remain necessary before router writes.
Mock tests do not prove uninterrupted production traffic or RouterOS field behavior.
