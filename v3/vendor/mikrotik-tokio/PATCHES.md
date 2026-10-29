# Local transport patch

Source: `mikrotik-tokio` 0.1.0 from crates.io.
Upstream: https://github.com/ferrohd/mikrotik-rs
Revision: `c64acc6455e90d491b410bb22523f82fd1f236f1`.
License: MIT OR Apache-2.0. The license texts come from that revision.
Copyright and package author metadata remain unchanged.

LQoSync modification, 2026-10-29:
`src/device.rs` routes events through awaited bounded sends.
The original `try_send` cancels commands when a normal response burst fills its 16-event channel.
Awaiting replies and terminal events preserves every row and completion signal.
The application consumes one command at a time and drops its receiver on timeout.
This lets a blocked send finish with an error when the caller stops reading.
Do not retain an unread receiver while running concurrent commands on this shared actor.

The application regression test sends 200 rows and completion in one network write.
Remove this patch only after the selected upstream dependency passes that test.
No RouterOS protocol parser is reimplemented here.
