//! The beta19 analytics protocol: CBOR request, SCM_RIGHTS descriptor, and CBOR snapshot.

use super::Snapshot;
use anyhow::{Context, ensure};
use nix::sys::socket::{ControlMessageOwned, MsgFlags, recvmsg};
use serde::Deserialize;
use std::io::IoSliceMut;
use std::os::fd::{AsRawFd, FromRawFd, OwnedFd};
use std::os::unix::fs::FileExt;
use std::path::Path;
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt, Interest};
use tokio::net::UnixStream;

const MAX_SNAPSHOT: usize = 2 * 1024 * 1024;

fn receive(stream: &UnixStream, header: &mut [u8; 4]) -> std::io::Result<(usize, Vec<OwnedFd>)> {
    let mut io = [IoSliceMut::new(header)];
    // Linux permits at most 253 SCM_RIGHTS descriptors in one message.
    let mut control = nix::cmsg_space!([i32; 253]);
    let message = recvmsg::<()>(
        stream.as_raw_fd(),
        &mut io,
        Some(&mut control),
        MsgFlags::MSG_DONTWAIT | MsgFlags::MSG_CMSG_CLOEXEC,
    )?;
    let mut fds = Vec::new();
    for cmsg in message.cmsgs()? {
        if let ControlMessageOwned::ScmRights(raw) = cmsg {
            for fd in raw {
                // SAFETY: SCM_RIGHTS transfers ownership of each fresh descriptor.
                fds.push(unsafe { OwnedFd::from_raw_fd(fd) });
            }
        }
    }
    Ok((message.bytes, fds))
}

fn decode<T: serde::de::DeserializeOwned>(mut bytes: &[u8]) -> anyhow::Result<T> {
    let value = ciborium::from_reader(&mut bytes).context("invalid analytics CBOR")?;
    ensure!(bytes.is_empty(), "trailing analytics data");
    Ok(value)
}

async fn request<T: serde::de::DeserializeOwned>(
    path: &Path,
    kind: &str,
    limit: usize,
) -> anyhow::Result<T> {
    request_fields(path, kind, serde_json::json!({"kind":kind,"limit":limit})).await
}

async fn request_fields<T: serde::de::DeserializeOwned>(
    path: &Path,
    kind: &str,
    fields: serde_json::Value,
) -> anyhow::Result<T> {
    tokio::time::timeout(Duration::from_secs(2), async {
        let mut stream = UnixStream::connect(path)
            .await
            .context("analytics socket unavailable")?;
        // All supported requests are read-only. The caller bounds snapshot host and flow rows.
        let mut request = Vec::new();
        ciborium::into_writer(&fields, &mut request)?;
        stream
            .write_all(&(request.len() as u32).to_be_bytes())
            .await?;
        stream.write_all(&request).await?;
        let mut header = [0; 4];
        let (received, mut fds) = loop {
            stream.readable().await?;
            match stream.try_io(Interest::READABLE, || receive(&stream, &mut header)) {
                Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => continue,
                result => break result?,
            }
        };
        ensure!(
            received > 0 && fds.len() == 1,
            "analytics reply needs one descriptor"
        );
        let file = std::fs::File::from(fds.pop().unwrap());
        stream.read_exact(&mut header[received..]).await?;
        let length = u32::from_be_bytes(header) as usize;
        ensure!(
            (1..=1024).contains(&length),
            "analytics reply header too large"
        );
        let mut reply = vec![0; length];
        stream.read_exact(&mut reply).await?;
        #[derive(Deserialize)]
        struct Reply {
            kind: Option<String>,
            snapshot_len: usize,
        }
        let reply: Reply = decode(&reply)?;
        ensure!(
            reply.kind.as_deref() == Some(kind) || (kind == "snapshot" && reply.kind.is_none()),
            "unexpected analytics reply kind"
        );
        ensure!(
            (1..=MAX_SNAPSHOT).contains(&reply.snapshot_len),
            "analytics snapshot too large"
        );
        let meta = file.metadata()?;
        ensure!(
            meta.is_file() && meta.len() >= reply.snapshot_len as u64,
            "invalid analytics descriptor length"
        );
        // Read at offset zero without mmap: a truncated producer file returns an error.
        let mut bytes = vec![0; reply.snapshot_len];
        file.read_exact_at(&mut bytes, 0)?;
        decode(&bytes)
    })
    .await
    .context("analytics request timed out")?
}

pub async fn read_snapshot(path: &Path) -> anyhow::Result<Snapshot> {
    let snapshot: Snapshot = request(path, "snapshot", 1).await?;
    snapshot.validate()?;
    Ok(snapshot)
}

pub async fn read_health(path: &Path) -> anyhow::Result<super::health::Report> {
    let health: super::health::Report = request(path, "health", 1).await?;
    health.validate()?;
    Ok(health)
}

pub(crate) async fn read_hosts(path: &Path) -> anyhow::Result<crate::subscriber_stats::Snapshot> {
    let hosts: crate::subscriber_stats::Snapshot =
        request(path, "snapshot", crate::subscriber_stats::LIMIT).await?;
    hosts.validate()?;
    Ok(hosts)
}

pub(crate) async fn read_mapping(path: &Path) -> anyhow::Result<crate::subscriber_stats::Mapping> {
    let state: crate::subscriber_stats::State = request(path, "state", 1).await?;
    match state {
        crate::subscriber_stats::State::Report(report) => report.mapping(),
        crate::subscriber_stats::State::Unavailable(_) => {
            anyhow::bail!("applied state unavailable")
        }
    }
}

pub(crate) async fn read_queue(
    path: &Path,
    slot: u64,
) -> anyhow::Result<crate::circuit_queue::Reply> {
    ensure!(slot > 0, "best-effort slot has no circuit queue");
    let mut reply: crate::circuit_queue::Reply = request_fields(
        path,
        "queue_stats",
        serde_json::json!({"kind":"queue_stats","limit":1,"queue_slots":[slot]}),
    )
    .await?;
    reply.sanitize(slot)?;
    Ok(reply)
}

#[cfg(test)]
mod tests {
    use super::*;
    use nix::sys::socket::{ControlMessage, sendmsg};
    use serde_json::{Value, json};
    use std::io::{IoSlice, Read, Write};
    use std::os::unix::net::UnixListener;

    fn serve<V: serde::Serialize + Send + 'static>(
        value: V,
        kind: &str,
        length: Option<usize>,
        count: usize,
        expected_request: (&'static str, usize),
    ) -> (tempfile::TempDir, std::thread::JoinHandle<()>) {
        let dir = tempfile::tempdir().unwrap();
        let listener = UnixListener::bind(dir.path().join("analytics.sock")).unwrap();
        let kind = kind.to_owned();
        let file_path = dir.path().join("snapshot.cbor");
        let thread = std::thread::spawn(move || {
            let (mut socket, _) = listener.accept().unwrap();
            socket
                .set_read_timeout(Some(Duration::from_secs(4)))
                .unwrap();
            let mut head = [0; 4];
            socket.read_exact(&mut head).unwrap();
            let mut request = vec![0; u32::from_be_bytes(head) as usize];
            socket.read_exact(&mut request).unwrap();
            let request: Value = decode(&request).unwrap();
            assert_eq!(request["kind"], expected_request.0);
            assert_eq!(request["limit"], expected_request.1);
            if expected_request.0 == "queue_stats" {
                assert_eq!(request["queue_slots"], json!([1]));
            } else {
                assert!(request.get("queue_slots").is_none());
            }
            let mut bytes = Vec::new();
            ciborium::into_writer(&value, &mut bytes).unwrap();
            let mut file = std::fs::File::options()
                .create(true)
                .truncate(true)
                .read(true)
                .write(true)
                .open(file_path)
                .unwrap();
            file.write_all(&bytes).unwrap();
            let mut reply = Vec::new();
            ciborium::into_writer(
                &json!({"kind":kind,"snapshot_len":length.unwrap_or(bytes.len())}),
                &mut reply,
            )
            .unwrap();
            let header = (reply.len() as u32).to_be_bytes();
            let fds = vec![file.as_raw_fd(); count];
            let rights = [ControlMessage::ScmRights(&fds)];
            // A fragmented header catches clients that assume recvmsg always returns four bytes.
            sendmsg::<()>(
                socket.as_raw_fd(),
                &[IoSlice::new(&header[..1])],
                if count == 0 { &[] } else { &rights },
                MsgFlags::empty(),
                None,
            )
            .unwrap();
            let _ = socket.write_all(&header[1..]);
            let _ = socket.write_all(&reply);
        });
        (dir, thread)
    }

    #[tokio::test]
    async fn circuit_queue_request_uses_one_selected_nonzero_slot_and_strict_reply_kind() {
        let value = json!({"rows":[{"slot_id":1,"download":{"tins":[{"sent_bytes":10,"backlog_bytes":20,"sojourn_p99_ns":1500000}],"memory_used":20,"queue_depth_pkts":2,"drop_pkts":3,"drop_pkts_queue_full":1,"aqm_marks":4},"upload":{},"rtt_histogram":vec![0u64;50]}],"sampled_at_unix_ms":100000,"sample_seq":5});
        let (dir, server) = serve(value, "queue_stats", None, 1, ("queue_stats", 1));
        let q = read_queue(&dir.path().join("analytics.sock"), 1)
            .await
            .unwrap();
        server.join().unwrap();
        assert_eq!(q.rows[0].download.as_ref().unwrap().drop_pkts, Some(3));
        assert_eq!(q.sample_seq, Some(5));
        let (dir, server) = serve(sample(), "snapshot", None, 1, ("queue_stats", 1));
        assert!(
            read_queue(&dir.path().join("analytics.sock"), 1)
                .await
                .is_err()
        );
        server.join().unwrap();
        assert!(
            read_queue(&dir.path().join("not-used.sock"), 0)
                .await
                .is_err()
        );
    }

    #[tokio::test]
    async fn subscriber_applied_slot_order_roundtrips_and_requires_current_schema() {
        let mut slots = Vec::new();
        ciborium::into_writer(&vec!["c1", "c2"], &mut slots).unwrap();
        let state = json!({"Report":{"generation":"g1","report_schema":1,"slot_order":slots}});
        let (dir, server) = serve(state.clone(), "state", None, 1, ("state", 1));
        let mapping = read_mapping(&dir.path().join("analytics.sock"))
            .await
            .unwrap();
        server.join().unwrap();
        assert_eq!(mapping.names, vec!["c1", "c2"]);
        for bad in [
            json!({"Unavailable":"no_report_yet"}),
            json!({"Report":{"report_schema":2,"generation":"g1","slot_order":[]}}),
        ] {
            let (dir, server) = serve(bad, "state", None, 1, ("state", 1));
            assert!(
                read_mapping(&dir.path().join("analytics.sock"))
                    .await
                    .is_err()
            );
            server.join().unwrap();
        }
    }

    #[tokio::test]
    async fn subscriber_host_snapshot_decodes_real_ip_and_keeps_aggregate_limit_separate() {
        let snapshot: crate::subscriber_stats::Snapshot = serde_json::from_value(json!({
            "captured_at_millis":1000,"hosts":[{"id":{"circuit_id":1,"subscriber_ip":"192.0.2.1"},
            "actual":{"download_bytes":100,"upload_bytes":200},"rtt_samples_us":[1000,2000],"tcp_retransmits":3}]})).unwrap();
        let (dir, server) = serve(snapshot, "snapshot", None, 1, ("snapshot", 512));
        let decoded = read_hosts(&dir.path().join("analytics.sock"))
            .await
            .unwrap();
        server.join().unwrap();
        assert_eq!(
            decoded.hosts.as_ref().unwrap()[0]
                .id
                .subscriber_ip
                .to_string(),
            "192.0.2.1"
        );
        assert_eq!(decoded.hosts.as_ref().unwrap()[0].tcp_retransmits, Some(3));
    }

    #[tokio::test]
    async fn health_uses_the_bounded_descriptor_transport_and_requires_its_reply_kind() {
        let value = json!({"status":"degraded","version":"v3","uptime_ms":12345,
            "workers":[],"classification_empty":true,"topology_tier":"bootstrap_trunk",
            "degraded_subsystems":["stats_summary"]});
        let (dir, server) = serve(value, "health", None, 1, ("health", 1));
        let health = read_health(&dir.path().join("analytics.sock"))
            .await
            .unwrap();
        server.join().unwrap();
        assert_eq!(health.uptime_ms, 12345);
        assert_eq!(health.status, "degraded");
        assert_eq!(
            health.degraded_subsystems.as_deref().unwrap(),
            &["stats_summary"]
        );
        let (dir, server) = serve(sample(), "snapshot", None, 1, ("health", 1));
        assert!(
            read_health(&dir.path().join("analytics.sock"))
                .await
                .is_err()
        );
        server.join().unwrap();
    }

    fn sample() -> Value {
        json!({"captured_at_millis":100,"workers":[{"queue_id":2,"busy_ppm":850000,"idle_ppm":150000,"backlog_mean":12,"laps_per_s":12345,"busy_laps":42,"cycles_per_busy_lap":250,"phase_cycles":[1,2,3,4,5,6,7,8,9,10]}],"diagnostics":{"rx_nombuf":42,"pool_drop_packets":84,"pool_total_mbufs":1000,"pool_free_mbufs":100},"qdisc_totals":{"drop_pkts":3,"aqm_marks":5,"backlog_bytes":4096},"mapped":{"download_bytes":10,"upload_bytes":20},"unmapped":{"download_bytes":30,"upload_bytes":40},"hosts":[{"private":"ignored"}]})
    }

    #[tokio::test]
    async fn worker_util_ipc_reads_a_fragmented_reply_and_ignores_unrelated_data() {
        let (dir, server) = serve(sample(), "snapshot", None, 1, ("snapshot", 1));
        let sample = read_snapshot(&dir.path().join("analytics.sock"))
            .await
            .unwrap();
        server.join().unwrap();
        assert_eq!(sample.workers[0].busy_ppm, 850000);
        assert_eq!(sample.workers[0].backlog_mean, Some(12));
        assert_eq!(sample.workers[0].idle_ppm, Some(150000));
        assert_eq!(sample.workers[0].laps_per_s, Some(12345));
        assert_eq!(sample.workers[0].busy_laps, Some(42));
        assert_eq!(sample.workers[0].cycles_per_busy_lap, Some(250));
        assert_eq!(
            sample.workers[0].phase_cycles.as_deref(),
            Some(&[1, 2, 3, 4, 5, 6, 7, 8, 9, 10][..])
        );
        let mapped = sample.mapped.unwrap();
        let unmapped = sample.unmapped.unwrap();
        assert_eq!(mapped.download_bytes, 10);
        assert_eq!(mapped.upload_bytes, 20);
        assert_eq!(unmapped.download_bytes, 30);
        assert_eq!(unmapped.upload_bytes, 40);
        let queue = sample.qdisc_totals.unwrap();
        assert_eq!(queue.drop_pkts, Some(3));
        assert_eq!(queue.aqm_marks, Some(5));
        assert_eq!(queue.backlog_bytes, Some(4096));
        let diagnostics = sample.diagnostics.unwrap();
        assert_eq!(diagnostics.rx_nombuf, Some(42));
        assert_eq!(diagnostics.pool_free_mbufs, Some(100));
        assert_eq!(diagnostics.ring_capacity, None);
    }

    #[tokio::test]
    async fn worker_util_ipc_rejects_invalid_sizes_kinds_and_descriptors() {
        for (kind, length, count) in [
            ("health", None, 1),
            ("snapshot", Some(MAX_SNAPSHOT + 1), 1),
            ("snapshot", Some(0), 1),
            ("snapshot", Some(1), 1),
            ("snapshot", None, 0),
            ("snapshot", None, 2),
        ] {
            let (dir, server) = serve(sample(), kind, length, count, ("snapshot", 1));
            assert!(
                read_snapshot(&dir.path().join("analytics.sock"))
                    .await
                    .is_err()
            );
            server.join().unwrap();
            // Both the server and client must release every received descriptor on failure.
            let file = dir.path().join("snapshot.cbor");
            let open = std::fs::read_dir("/proc/self/fd")
                .unwrap()
                .filter_map(Result::ok)
                .filter(|fd| std::fs::read_link(fd.path()).is_ok_and(|path| path == file))
                .count();
            assert_eq!(open, 0, "descriptor leak for {kind}/{length:?}/{count}");
        }
    }

    #[tokio::test]
    async fn worker_util_ipc_old_bridge_is_unavailable_instead_of_zero() {
        let (dir, server) = serve(
            json!({"captured_at_millis":100}),
            "snapshot",
            None,
            1,
            ("snapshot", 1),
        );
        assert!(
            read_snapshot(&dir.path().join("analytics.sock"))
                .await
                .unwrap_err()
                .to_string()
                .contains("no worker")
        );
        server.join().unwrap();
    }

    #[tokio::test]
    async fn worker_util_ipc_times_out_when_the_peer_never_answers() {
        let dir = tempfile::tempdir().unwrap();
        let listener = UnixListener::bind(dir.path().join("socket")).unwrap();
        let server = std::thread::spawn(move || {
            let (_socket, _) = listener.accept().unwrap();
            std::thread::sleep(Duration::from_millis(2300));
        });
        let error = read_snapshot(&dir.path().join("socket")).await.unwrap_err();
        assert!(error.to_string().contains("timed out"), "{error:#}");
        server.join().unwrap();
    }
}
