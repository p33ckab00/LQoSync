//! The load counters of the LibreQoS v3 DPDK bridge, from its journal. The worker polls the NIC, so
//! its CPU is always 100%. These counters show the real load: `budget_defers` and `iter_gt100ms`
//! go up when the worker cannot keep up. Both count from the start of the bridge.

use serde::{Deserialize, Serialize};
use std::time::Duration;

/// The systemd unit of the bridge.
pub const UNIT: &str = "libreqos-bridge";

/// The last counters of the bridge.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct BridgeStats {
    /// Loops where the worker had more work than its time budget (`FLOW-DIAG`).
    pub budget_defers: u64,
    /// Collector loops longer than 100 ms and 1 s, and the longest loop (`COLLECTOR-ITER-DIAG`).
    pub iter_gt100ms: u64,
    pub iter_gt1s: u64,
    pub iter_max_ms: u64,
}

/// The last value of each counter in the journal text. `None` if a counter is missing.
pub fn parse(text: &str) -> Option<BridgeStats> {
    // The value of `key=` in the last line that has it. A key must start a word:
    // `iter_gap_max_ms=` is not `iter_max_ms=`.
    let last = |key: &str| {
        let pat = format!(" {key}=");
        text.lines().rev().find_map(|l| {
            let rest = &l[l.find(&pat)? + pat.len()..];
            let end = rest
                .find(|c: char| !c.is_ascii_digit())
                .unwrap_or(rest.len());
            rest[..end].parse::<u64>().ok()
        })
    };
    Some(BridgeStats {
        budget_defers: last("budget_defers")?,
        iter_gt100ms: last("iter_gt100ms")?,
        iter_gt1s: last("iter_gt1s")?,
        iter_max_ms: last("iter_max_ms")?,
    })
}

/// Read the last counters from the journal of the bridge. `None` if they cannot be read.
pub async fn read() -> Option<BridgeStats> {
    let out = tokio::time::timeout(
        Duration::from_secs(5),
        tokio::process::Command::new("journalctl")
            .args(["-u", UNIT, "-n", "300", "-o", "cat", "--no-pager"])
            .stdin(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .kill_on_drop(true)
            .output(),
    )
    .await
    .ok()?
    .ok()?;
    parse(&String::from_utf8_lossy(&out.stdout))
}

#[cfg(test)]
mod tests {
    use super::*;

    const FLOW: &str = "2026-11-02T08:35:37.617481Z  INFO bridge_rs::dpdk_core::stats: bridge_rs/src/dpdk_core/stats.rs:7554: FLOW-DIAG workers=1 rx_batch_calls=78143740 rx_pkts=80935723 enq[call=80935723 applied=80271220 no_flowhash=0 best_effort=370800] sent[call=1 applied=1 no_flowhash=0 best_effort=0] flush[push=1 empty=1] budget_defers={D} | merged";
    const ITER: &str = "2026-11-02T08:35:37.617500Z  INFO bridge_rs::dpdk_core::stats: bridge_rs/src/dpdk_core/stats.rs:7564: COLLECTOR-ITER-DIAG iters=7740290 iter_max_ms={M} iter_gap_max_ms=11 iter_gt100ms={G} iter_gt1s=0 iter_gt5s=0 top_max_ms=15";

    fn lines(d: u64, m: u64, g: u64) -> String {
        format!(
            "{}\n{}\n",
            FLOW.replace("{D}", &d.to_string()),
            ITER.replace("{M}", &m.to_string())
                .replace("{G}", &g.to_string())
        )
    }

    #[test]
    fn parse_takes_the_last_value_of_each_counter() {
        let text = lines(1, 94, 0) + "other line\n" + &lines(5, 120, 2);
        assert_eq!(
            parse(&text),
            Some(BridgeStats {
                budget_defers: 5,
                iter_gt100ms: 2,
                iter_gt1s: 0,
                iter_max_ms: 120
            })
        );
    }

    /// Reads the journal of this host: `cargo test -- --ignored read_on_this_host`.
    #[tokio::test]
    #[ignore]
    async fn read_on_this_host() {
        let b = read()
            .await
            .expect("the bridge counters of this LibreQoS v3 host");
        eprintln!("{b:?}");
    }

    #[test]
    fn parse_needs_both_lines() {
        assert_eq!(parse(""), None);
        assert_eq!(parse(&FLOW.replace("{D}", "1")), None);
        assert_eq!(parse("iter_gap_max_ms=3 not a counter line"), None);
    }
}
