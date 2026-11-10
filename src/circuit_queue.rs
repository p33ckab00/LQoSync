//! On-demand, read-only diagnostics for one applied shaping circuit.
use anyhow::{Context, ensure};
use serde::{Deserialize, Serialize};
use std::path::Path;

pub const MAX_AGE_MS: u64 = 15000;
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Tin {
    pub sent_bytes: Option<u64>,
    pub backlog_bytes: Option<u64>,
    pub sojourn_p99_ns: Option<u64>,
}
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Direction {
    pub tins: Option<Vec<Tin>>,
    pub memory_used: Option<u64>,
    pub queue_depth_pkts: Option<u64>,
    pub drop_pkts: Option<u64>,
    pub drop_pkts_queue_full: Option<u64>,
    pub aqm_marks: Option<u64>,
}
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Row {
    pub slot_id: u64,
    pub download: Option<Direction>,
    pub upload: Option<Direction>,
    pub rtt_histogram: Option<Vec<u64>>,
}
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Reply {
    pub rows: Vec<Row>,
    pub sampled_at_unix_ms: Option<u64>,
    pub elapsed_ms: Option<u64>,
    pub cadence_ms: Option<u64>,
    pub sample_epoch_unix_ms: Option<u64>,
    pub sample_seq: Option<u64>,
    pub actor_gap: Option<bool>,
    pub counter_reset: Option<bool>,
}
impl Reply {
    pub(crate) fn sanitize(&mut self, slot: u64) -> anyhow::Result<()> {
        ensure!(
            slot > 0 && self.rows.len() == 1 && self.rows[0].slot_id == slot,
            "queue reply must match the single requested slot"
        );
        let row = &mut self.rows[0];
        for direction in [&row.download, &row.upload].into_iter().flatten() {
            ensure!(
                direction.tins.as_ref().is_none_or(|v| v.len() <= 16),
                "too many queue tins"
            );
        }
        if row.rtt_histogram.as_ref().is_some_and(|h| {
            h.len() != 50
                || h.iter()
                    .try_fold(0u64, |sum, n| sum.checked_add(*n))
                    .is_none()
        }) {
            row.rtt_histogram = None;
        }
        Ok(())
    }
}
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Observation {
    pub circuit: String,
    pub slot: u64,
    pub observed_at_unix_ms: u64,
    pub reply: Reply,
}
pub fn now_millis() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
        .try_into()
        .unwrap_or(u64::MAX)
}
pub async fn fetch(path: &Path, circuit: &str) -> anyhow::Result<Observation> {
    ensure!(
        !circuit.is_empty()
            && circuit.len() <= 256
            && !circuit.chars().any(char::is_control)
            && !circuit.starts_with("observed:"),
        "select an assigned shaping circuit"
    );
    let before = crate::worker_util::ipc::read_mapping(path).await?;
    let slot = before
        .names
        .iter()
        .position(|name| name == circuit)
        .context("selected circuit is not in the applied slot order")? as u64
        + 1;
    let reply = crate::worker_util::ipc::read_queue(path, slot).await?;
    let after = crate::worker_util::ipc::read_mapping(path).await?;
    ensure!(before == after, "applied mapping changed during queue read");
    Ok(Observation {
        circuit: circuit.into(),
        slot,
        observed_at_unix_ms: now_millis(),
        reply,
    })
}

fn value<T: std::fmt::Display>(v: Option<T>) -> String {
    v.map(|n| n.to_string()).unwrap_or_else(|| "unknown".into())
}
impl Observation {
    fn state(&self, now: u64, force_stale: bool) -> &'static str {
        let sampled = self
            .reply
            .sampled_at_unix_ms
            .filter(|n| *n > 0)
            .map(|n| n / 1000);
        let future = sampled.is_some_and(|n| n > now) || self.observed_at_unix_ms / 1000 > now;
        let stale = force_stale
            || now.saturating_sub(self.observed_at_unix_ms / 1000) > MAX_AGE_MS / 1000
            || sampled.is_some_and(|n| now.saturating_sub(n) > MAX_AGE_MS / 1000);
        if stale {
            "STALE (last observed)"
        } else if future {
            "OBSERVED (source clock is ahead)"
        } else if self.reply.actor_gap == Some(true) || self.reply.counter_reset == Some(true) {
            "GAP/RESET observation"
        } else if sampled.is_none() {
            "OBSERVED (source age unknown)"
        } else {
            "FRESH"
        }
    }
    pub fn measurements(&self, now: u64, force_stale: bool) -> Vec<String> {
        let state = self.state(now, force_stale);
        let sampled = self
            .reply
            .sampled_at_unix_ms
            .filter(|n| *n > 0)
            .map(|n| n / 1000);
        let mut lines = vec![
            format!("Queue diagnostics {state}"),
            "Scope: whole circuit, both directions; manual observation".into(),
            format!(
                "Source age: {} seconds; response age: {} seconds",
                value(sampled.map(|n| now.saturating_sub(n))),
                now.saturating_sub(self.observed_at_unix_ms / 1000)
            ),
            format!(
                "Sample sequence: {}; epoch: {} unix ms",
                value(self.reply.sample_seq),
                value(self.reply.sample_epoch_unix_ms)
            ),
            format!(
                "Elapsed: {} ms; cadence: {} ms",
                value(self.reply.elapsed_ms),
                value(self.reply.cadence_ms.filter(|n| *n > 0))
            ),
            format!(
                "Actor gap: {}; counter reset: {}",
                value(self.reply.actor_gap),
                value(self.reply.counter_reset)
            ),
        ];
        let Some(row) = self.reply.rows.first() else {
            lines.push("Queue row unavailable".into());
            return lines;
        };
        for (name, direction) in [("Download", &row.download), ("Upload", &row.upload)] {
            lines.push(name.into());
            lines.push(format!(
                "Queue depth: {} packets",
                value(direction.as_ref().and_then(|d| d.queue_depth_pkts))
            ));
            lines.push(format!(
                "Drops: {}; Queue-full drops: {}; AQM marks: {}",
                value(direction.as_ref().and_then(|d| d.drop_pkts)),
                value(direction.as_ref().and_then(|d| d.drop_pkts_queue_full)),
                value(direction.as_ref().and_then(|d| d.aqm_marks))
            ));
            lines.push(format!(
                "memory_used (reported raw): {}",
                value(direction.as_ref().and_then(|d| d.memory_used))
            ));
            if let Some(tins) = direction.as_ref().and_then(|d| d.tins.as_ref()) {
                for (i, tin) in tins.iter().take(16).enumerate() {
                    lines.push(format!(
                        "Tin {i}: Sent: {} bytes; Backlog: {} bytes; p99 sojourn (reported): {} ms",
                        value(tin.sent_bytes),
                        value(tin.backlog_bytes),
                        tin.sojourn_p99_ns
                            .map(|n| format!("{:.3}", n as f64 / 1_000_000.0))
                            .unwrap_or_else(|| "unknown".into())
                    ));
                }
            } else {
                lines.push("Tins: unknown".into());
            }
        }
        if let Some(hist) = row.rtt_histogram.as_ref().filter(|h| h.len() == 50) {
            if let Some(total) = hist.iter().try_fold(0u64, |sum, n| sum.checked_add(*n)) {
                lines.push(format!("RTT histogram: {total} reported samples (pooled)"));
                for (i, n) in hist.iter().enumerate().filter(|(_, n)| **n > 0) {
                    lines.push(if i == 49 {
                        format!(">=490 ms: {n}")
                    } else {
                        format!("{}-<{} ms: {n}", i * 10, (i + 1) * 10)
                    });
                }
            } else {
                lines.push("RTT histogram: unknown".into());
            }
        } else {
            lines.push("RTT histogram: unknown".into());
        }
        lines
    }
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Request {
    pub id: u64,
    pub circuit: String,
}
#[derive(Debug)]
pub struct Done {
    pub request: Request,
    pub result: Result<Observation, String>,
}
#[derive(Debug, Default)]
pub struct View {
    pending: Option<Request>,
    observed: Option<Observation>,
    error: Option<(String, String)>,
    last_started: Option<std::time::Instant>,
    next_id: u64,
}
impl View {
    pub fn begin(
        &mut self,
        circuit: &str,
        now: std::time::Instant,
    ) -> Result<Request, &'static str> {
        if circuit.is_empty()
            || circuit.len() > 256
            || circuit.chars().any(char::is_control)
            || circuit.starts_with("observed:")
        {
            return Err("Select an assigned shaping circuit");
        }
        if self.pending.is_some() {
            return Err("Queue diagnostics already reading one circuit");
        }
        if self
            .last_started
            .is_some_and(|n| now.saturating_duration_since(n) < std::time::Duration::from_secs(2))
        {
            return Err("Wait two seconds between queue requests");
        }
        self.next_id = self.next_id.wrapping_add(1);
        let request = Request {
            id: self.next_id,
            circuit: circuit.into(),
        };
        self.pending = Some(request.clone());
        self.last_started = Some(now);
        self.error = None;
        Ok(request)
    }
    pub fn complete(&mut self, done: Done) -> bool {
        if self.pending.as_ref() != Some(&done.request) {
            return false;
        }
        self.pending = None;
        let result = done.result.and_then(|mut q| {
            if q.circuit != done.request.circuit {
                return Err("Queue result identity mismatch".into());
            }
            q.reply.sanitize(q.slot).map_err(|e| e.to_string())?;
            Ok(q)
        });
        match result {
            Ok(q) => {
                self.observed = Some(q);
                self.error = None;
            }
            Err(e) => {
                self.error = Some((
                    done.request.circuit,
                    e.chars().filter(|c| !c.is_control()).take(180).collect(),
                ));
            }
        }
        true
    }
    pub fn label(&self, circuit: &str, now: u64) -> Option<&'static str> {
        let pending = self.pending.as_ref().is_some_and(|r| r.circuit == circuit);
        let error = self.error.as_ref().is_some_and(|(c, _)| c == circuit);
        if let Some(q) = self.observed.as_ref().filter(|q| q.circuit == circuit) {
            Some(q.state(now, pending || error))
        } else if pending {
            Some("READING")
        } else if error {
            Some("UNAVAILABLE")
        } else {
            None
        }
    }
    pub fn measurements(&self, circuit: &str, now: u64) -> Vec<String> {
        let pending = self.pending.as_ref().is_some_and(|r| r.circuit == circuit);
        let error = self.error.as_ref().filter(|(c, _)| c == circuit);
        let mut lines = Vec::new();
        if pending {
            lines.push("Queue diagnostics: reading selected circuit".into());
        }
        if let Some((_, e)) = error {
            lines.push(format!("Queue diagnostics unavailable: {e}"));
            if e.to_ascii_lowercase().contains("permission denied") {
                lines.push("Socket access denied; use a root TUI for this root-only source".into());
            }
        }
        if let Some(q) = self.observed.as_ref().filter(|q| q.circuit == circuit) {
            lines.extend(q.measurements(now, pending || error.is_some()));
        } else if !pending && error.is_none() {
            lines.push("Queue diagnostics not requested for selected circuit".into());
        }
        lines
    }
}
pub mod worker;

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    fn observation() -> Observation {
        serde_json::from_value(json!({"circuit":"c1","slot":1,"observed_at_unix_ms":100000,"reply":{
          "rows":[{"slot_id":1,"download":{"tins":[{"sent_bytes":10,"backlog_bytes":20,"sojourn_p99_ns":1500000}],"queue_depth_pkts":2,"drop_pkts":3,"drop_pkts_queue_full":1,"aqm_marks":4,"memory_used":958104},"upload":{},"rtt_histogram":vec![0;50]}],
          "sampled_at_unix_ms":100000,"elapsed_ms":200,"cadence_ms":200,"sample_epoch_unix_ms":90000,"sample_seq":5,"actor_gap":false,"counter_reset":false}})).unwrap()
    }
    #[tokio::test]
    async fn circuit_queue_fetch_rejects_mapping_changes_and_reads_only_one_slot() {
        use nix::sys::socket::{ControlMessage, MsgFlags, sendmsg};
        use std::io::{IoSlice, Read, Write};
        use std::os::{fd::AsRawFd, unix::net::UnixListener};
        for changed in [false, true] {
            let dir = tempfile::tempdir().unwrap();
            let path = dir.path().join("analytics.sock");
            let listener = UnixListener::bind(&path).unwrap();
            let thread = std::thread::spawn(move || {
                for step in 0..3 {
                    let (mut socket, _) = listener.accept().unwrap();
                    socket
                        .set_read_timeout(Some(std::time::Duration::from_secs(2)))
                        .unwrap();
                    let mut head = [0; 4];
                    socket.read_exact(&mut head).unwrap();
                    let mut bytes = vec![0; u32::from_be_bytes(head) as usize];
                    socket.read_exact(&mut bytes).unwrap();
                    let request: serde_json::Value =
                        ciborium::from_reader(bytes.as_slice()).unwrap();
                    let kind = if step == 1 { "queue_stats" } else { "state" };
                    assert_eq!(request["kind"], kind);
                    assert_eq!(request["limit"], 1);
                    let body = if step == 1 {
                        assert_eq!(request["queue_slots"], json!([1]));
                        serde_json::to_value(observation().reply).unwrap()
                    } else {
                        let mut names = Vec::new();
                        ciborium::into_writer(&vec!["c1", "c2"], &mut names).unwrap();
                        json!({"Report":{"generation":if changed && step==2 {"g2"} else {"g1"},"report_schema":1,"slot_order":names}})
                    };
                    bytes.clear();
                    ciborium::into_writer(&body, &mut bytes).unwrap();
                    let mut file = tempfile::tempfile().unwrap();
                    file.write_all(&bytes).unwrap();
                    let mut reply = Vec::new();
                    ciborium::into_writer(
                        &json!({"kind":kind,"snapshot_len":bytes.len()}),
                        &mut reply,
                    )
                    .unwrap();
                    let mut frame = (reply.len() as u32).to_be_bytes().to_vec();
                    frame.extend(reply);
                    let fds = [file.as_raw_fd()];
                    sendmsg::<()>(
                        socket.as_raw_fd(),
                        &[IoSlice::new(&frame[..1])],
                        &[ControlMessage::ScmRights(&fds)],
                        MsgFlags::empty(),
                        None,
                    )
                    .unwrap();
                    socket.write_all(&frame[1..]).unwrap();
                }
            });
            let result = fetch(&path, "c1").await;
            thread.join().unwrap();
            assert_eq!(result.is_err(), changed);
            if let Ok(q) = result {
                assert_eq!(q.slot, 1);
                assert_eq!(q.circuit, "c1");
            }
        }
    }
    #[test]
    fn circuit_queue_maximum_selected_details_stay_bounded() {
        let mut q = observation();
        let mut direction = q.reply.rows[0].download.clone().unwrap();
        let tin = Tin {
            sent_bytes: Some(u64::MAX),
            backlog_bytes: Some(u64::MAX),
            sojourn_p99_ns: Some(u64::MAX),
        };
        direction.tins = Some(vec![tin; 16]);
        q.reply.rows[0].download = Some(direction.clone());
        q.reply.rows[0].upload = Some(direction);
        q.reply.rows[0].rtt_histogram = Some(vec![1; 50]);
        q.reply.sanitize(1).unwrap();
        let start = std::time::Instant::now();
        for _ in 0..1000 {
            let lines = q.measurements(100, false);
            assert!(lines.len() <= 100 && lines.join("\n").len() < 16000);
        }
        eprintln!(
            "maximum circuit detail: {} bytes, 1000 debug formats {:?}",
            q.measurements(100, false).join("\n").len(),
            start.elapsed()
        );
    }
    #[test]
    fn circuit_queue_labels_keep_source_age_unknown_stale_and_gap_states_distinct() {
        let mut q = observation();
        let text = q.measurements(100, false).join("\n");
        for label in [
            "Queue diagnostics FRESH",
            "whole circuit",
            "Download",
            "Queue depth: 2 packets",
            "Drops: 3",
            "Queue-full drops: 1",
            "AQM marks: 4",
            "memory_used (reported raw): 958104",
            "Tin 0",
            "Sent: 10 bytes",
            "Backlog: 20 bytes",
            "p99 sojourn (reported): 1.500 ms",
            "RTT histogram: 0 reported samples",
        ] {
            assert!(text.contains(label), "missing {label}: {text}");
        }
        assert!(
            q.measurements(116, false)
                .join("\n")
                .contains("STALE (last observed)")
        );
        assert!(
            q.measurements(100, true)
                .join("\n")
                .contains("STALE (last observed)")
        );
        q.reply.sampled_at_unix_ms = None;
        assert!(
            q.measurements(100, false)
                .join("\n")
                .contains("OBSERVED (source age unknown)")
        );
        q.reply.sampled_at_unix_ms = Some(200000);
        assert!(
            q.measurements(100, false)
                .join("\n")
                .contains("source clock is ahead")
        );
        q.reply.sampled_at_unix_ms = Some(100000);
        q.reply.actor_gap = Some(true);
        assert!(
            q.measurements(100, false)
                .join("\n")
                .contains("GAP/RESET observation")
        );
    }
    #[test]
    fn circuit_queue_unknown_fields_histogram_bounds_and_requested_identity_are_safe() {
        let mut q = observation();
        q.reply.rows[0].rtt_histogram = Some(vec![1; 49]);
        q.reply.sanitize(1).unwrap();
        assert!(q.reply.rows[0].rtt_histogram.is_none());
        assert!(
            q.measurements(100, false)
                .join("\n")
                .contains("RTT histogram: unknown")
        );
        assert!(q.reply.sanitize(2).is_err());
        q.reply.rows[0].rtt_histogram = Some(vec![u64::MAX; 50]);
        q.reply.sanitize(1).unwrap();
        assert!(q.reply.rows[0].rtt_histogram.is_none());
        q.reply.rows[0].rtt_histogram =
            Some((0..50).map(|i| if i == 49 { 2 } else { 0 }).collect());
        q.reply.sanitize(1).unwrap();
        assert!(
            q.measurements(100, false)
                .join("\n")
                .contains(">=490 ms: 2")
        );
        let upload = q.measurements(100, false).join("\n");
        assert!(upload.contains("Queue depth: unknown packets"));
        q.reply.rows.push(q.reply.rows[0].clone());
        assert!(q.reply.sanitize(1).is_err());
        let mut q = observation();
        let tin = q.reply.rows[0]
            .download
            .as_ref()
            .unwrap()
            .tins
            .as_ref()
            .unwrap()[0]
            .clone();
        q.reply.rows[0].download.as_mut().unwrap().tins = Some(vec![tin; 17]);
        assert!(q.reply.sanitize(1).is_err());
    }
    #[test]
    fn circuit_queue_gate_binds_completions_throttles_requests_and_retains_failed_refreshes() {
        let mut view = View::default();
        let now = std::time::Instant::now();
        let request = view.begin("c1", now).unwrap();
        assert!(view.begin("c2", now).is_err());
        let wrong = Request {
            id: request.id + 1,
            circuit: "c1".into(),
        };
        assert!(!view.complete(Done {
            request: wrong,
            result: Ok(observation())
        }));
        assert!(view.complete(Done {
            request: request.clone(),
            result: Ok(observation())
        }));
        assert!(view.begin("c1", now).is_err());
        assert!(
            view.measurements("c2", 100)
                .join("\n")
                .contains("not requested for selected circuit")
        );
        let next = view
            .begin("c1", now + std::time::Duration::from_secs(2))
            .unwrap();
        assert!(
            view.measurements("c1", 100)
                .join("\n")
                .contains("reading selected circuit")
        );
        view.complete(Done {
            request: next,
            result: Err("Permission denied\n\x1b".into()),
        });
        assert_eq!(view.label("c1", 100), Some("STALE (last observed)"));
        assert_eq!(view.label("c2", 100), None);
        let text = view.measurements("c1", 100).join("\n");
        assert!(
            text.contains("STALE (last observed)")
                && text.contains("root TUI")
                && !text.contains('\x1b')
        );
        assert!(
            view.begin("observed:1", now + std::time::Duration::from_secs(5))
                .is_err()
        );
    }
}
