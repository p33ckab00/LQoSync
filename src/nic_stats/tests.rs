use super::*;
fn source(pid: u32, count: u64) -> Snapshot {
    Snapshot {
        pid,
        version: "DPDK fixture".into(),
        ports_capped: false,
        ports: vec![Port {
            id: 0,
            name: Some("0000:01:00.0".into()),
            status: Some("UP".into()),
            speed_mbps: Some(10000),
            duplex: Some("full-duplex".into()),
            stats: BTreeMap::from([("ierrors".into(), count), ("opackets".into(), count * 2)]),
            xstats: BTreeMap::from([("rx_phy_crc_errors".into(), count)]),
            ..Default::default()
        }],
    }
}
#[test]
fn counters_rebase_on_failures_gaps_resets_restarts_and_port_replacement() {
    let mut t = Tracker::default();
    t.record(Ok(source(10, 5)), 100_000);
    assert!(t.report.live(100));
    assert!(t.report.ports[0].deltas.is_empty());
    t.record(Ok(source(10, 7)), 110_000);
    assert_eq!(t.report.interval_ms, Some(10000));
    assert_eq!(t.report.ports[0].deltas["ierrors"], 2);
    assert_eq!(t.report.ports[0].xstats["rx_phy_crc_errors"], 7);
    t.record(Ok(source(10, 1)), 120_000);
    assert!(t.report.ports[0].reset && t.report.ports[0].deltas.is_empty());
    t.record(Ok(source(10, 2)), 130_000);
    assert_eq!(t.report.ports[0].deltas["ierrors"], 1);
    t.record(Err(anyhow::anyhow!("socket unavailable")), 140_000);
    assert!(!t.report.live(140));
    assert!(
        t.report
            .measurements(140, true)
            .join("\n")
            .contains("RX errors unknown")
    );
    t.record(Ok(source(10, 3)), 150_000);
    assert!(t.report.ports[0].deltas.is_empty());
    t.record(Ok(source(11, 4)), 160_000);
    assert!(t.report.ports[0].deltas.is_empty());
    let mut changed = source(11, 5);
    changed.ports[0].name = Some("replacement".into());
    t.record(Ok(changed), 170_000);
    assert!(t.report.ports[0].deltas.is_empty());
    t.record(Ok(source(11, 9)), 200_000);
    assert!(t.report.interval_ms.is_none());
    t.record(Ok(source(11, 10)), 199_000);
    assert!(t.report.interval_ms.is_none());
    assert!(!t.report.live(198) && !t.report.live(215));
}
#[test]
fn absent_counters_stay_unknown_and_new_counters_need_a_baseline() {
    let mut t = Tracker::default();
    t.record(Ok(source(1, 0)), 100_000);
    let mut s = source(1, 0);
    s.ports[0].stats.insert("imissed".into(), 5);
    t.record(Ok(s), 110_000);
    assert!(!t.report.ports[0].deltas.contains_key("imissed"));
    let text = t.report.measurements(110, true).join("\n");
    assert!(text.contains("RX errors 0 | TX errors unknown | missed 5"));
    assert!(text.contains("may overlap basic counters; do not sum"));
    assert!(text.contains("10") && !text.contains("WAN"));
}
#[test]
fn bounded_cache_rejects_corruption_and_retains_zero_unknown_and_stale() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join(CACHE);
    assert!(read_cache(&path).stale);
    let mut t = Tracker::default();
    t.record(Ok(source(1, 0)), 100_000);
    publish(dir.path(), &t.report).unwrap();
    assert!(read_cache(&path).live(100));
    assert_eq!(read_cache(&path).ports[0].stats["ierrors"], 0);
    std::fs::write(&path, b"invalid").unwrap();
    assert!(read_cache(&path).stale);
    let mut invalid = t.report.clone();
    invalid.ports.push(invalid.ports[0].clone());
    assert!(publish(dir.path(), &invalid).is_err());
    invalid = t.report.clone();
    invalid.ports[0].xstats.insert("bad\nname".into(), 0);
    assert!(publish(dir.path(), &invalid).is_err());
    invalid = t.report.clone();
    invalid.version = 42;
    assert!(publish(dir.path(), &invalid).is_err());
    std::fs::write(&path, vec![b' '; MAX_BYTES as usize + 1]).unwrap();
    assert!(read_cache(&path).stale);
}
fn server<F>(reply: F) -> (tempfile::TempDir, std::thread::JoinHandle<Vec<String>>)
where
    F: Fn(&str) -> serde_json::Value + Send + 'static,
{
    use nix::sys::socket::{self, AddressFamily, Backlog, SockFlag, SockType, UnixAddr};
    use std::{
        io::{Read, Write},
        os::fd::{AsRawFd, FromRawFd},
    };
    let dir = tempfile::tempdir().unwrap();
    let fd = socket::socket(
        AddressFamily::Unix,
        SockType::SeqPacket,
        SockFlag::SOCK_CLOEXEC,
        None,
    )
    .unwrap();
    socket::bind(
        fd.as_raw_fd(),
        &UnixAddr::new(&dir.path().join("telemetry")).unwrap(),
    )
    .unwrap();
    socket::listen(&fd, Backlog::new(8).unwrap()).unwrap();
    let thread = std::thread::spawn(move || {
        let accepted = socket::accept(fd.as_raw_fd()).unwrap();
        // SAFETY: accept transfers one new socket descriptor to this stream.
        let mut stream = unsafe { std::os::unix::net::UnixStream::from_raw_fd(accepted) };
        stream
            .set_read_timeout(Some(std::time::Duration::from_secs(4)))
            .unwrap();
        stream.write_all(br#"{"version":"DPDK fixture"}"#).unwrap();
        let mut requests = Vec::new();
        let mut bytes = [0; 1024];
        loop {
            let n = stream.read(&mut bytes).unwrap();
            if n == 0 {
                break;
            }
            let cmd = std::str::from_utf8(&bytes[..n]).unwrap();
            requests.push(cmd.to_string());
            let key = cmd.split(',').next().unwrap();
            let value = serde_json::json!({key:reply(cmd)});
            if stream
                .write_all(&serde_json::to_vec(&value).unwrap())
                .is_err()
            {
                break;
            }
        }
        requests
    });
    (dir, thread)
}
fn response(command: &str) -> serde_json::Value {
    use serde_json::json;
    let fixture: serde_json::Value =
        serde_json::from_str(include_str!("../../tests/fixtures/dpdk-nic-source.json")).unwrap();
    match command {
        "/" => json!([
            "/ethdev/list",
            "/ethdev/info",
            "/ethdev/link_status",
            "/ethdev/stats",
            "/ethdev/xstats"
        ]),
        "/ethdev/list" => json!([0]),
        _ => fixture[command
            .split(',')
            .next()
            .unwrap()
            .trim_start_matches("/ethdev/")]
        .clone(),
    }
}
#[tokio::test]
async fn native_seqpacket_transport_matches_verified_live_shape_and_only_reads() {
    let (dir, thread) = server(response);
    let s = sample(&dir.path().join("telemetry")).await.unwrap();
    assert_eq!(s.ports.len(), 1);
    assert_eq!(s.ports[0].status.as_deref(), Some("UP"));
    assert_eq!(s.ports[0].speed_mbps, Some(10000));
    assert_eq!(s.ports[0].xstats.len(), 78);
    let requests = thread.join().unwrap();
    assert_eq!(requests.len(), 8);
    assert!(
        requests
            .iter()
            .all(|v| v == "/" || v.starts_with("/ethdev/"))
    );
}
#[tokio::test]
async fn unsupported_optional_endpoints_are_unknown_and_errors_are_not_zero() {
    let (dir, thread) = server(|command| match command {
        "/" => serde_json::json!(["/ethdev/list"]),
        _ => serde_json::json!([0]),
    });
    let s = sample(&dir.path().join("telemetry")).await.unwrap();
    assert!(s.ports[0].stats.is_empty() && s.ports[0].speed_mbps.is_none());
    let mut tracker = Tracker::default();
    tracker.record(Ok(s), 100_000);
    assert!(
        tracker
            .report
            .measurements(100, true)
            .join("\n")
            .contains("Extended stats unavailable or empty")
    );
    thread.join().unwrap();
    let (dir, thread) = server(|command| {
        if command.starts_with("/ethdev/stats,") {
            serde_json::json!({"ierrors":-1})
        } else {
            response(command)
        }
    });
    assert!(sample(&dir.path().join("telemetry")).await.is_err());
    thread.join().unwrap();
    let (dir, thread) = server(|command| {
        if command == "/ethdev/list" {
            serde_json::json!([0, 0])
        } else {
            response(command)
        }
    });
    assert!(sample(&dir.path().join("telemetry")).await.is_err());
    thread.join().unwrap();
}
#[tokio::test]
async fn transport_rejects_oversized_replies_and_changed_port_identity_and_caps_xstats() {
    let (dir, thread) = server(|cmd| {
        if cmd.starts_with("/ethdev/xstats,") {
            serde_json::json!({"oversized":"x".repeat(70000)})
        } else {
            response(cmd)
        }
    });
    assert!(sample(&dir.path().join("telemetry")).await.is_err());
    thread.join().unwrap();
    let seen = std::sync::atomic::AtomicUsize::new(0);
    let (dir, thread) = server(move |cmd| {
        if cmd.starts_with("/ethdev/info,")
            && seen.fetch_add(1, std::sync::atomic::Ordering::SeqCst) > 0
        {
            serde_json::json!({"name":"replacement"})
        } else {
            response(cmd)
        }
    });
    assert!(sample(&dir.path().join("telemetry")).await.is_err());
    thread.join().unwrap();
    let (dir, thread) = server(|cmd| {
        if cmd.starts_with("/ethdev/xstats,") {
            serde_json::Value::Object(
                (0..300)
                    .map(|i| (format!("counter_{i:03}"), serde_json::json!(i)))
                    .collect(),
            )
        } else {
            response(cmd)
        }
    });
    let s = sample(&dir.path().join("telemetry")).await.unwrap();
    assert_eq!(s.ports[0].xstats.len(), 256);
    assert!(s.ports[0].xstats_capped);
    thread.join().unwrap();
}
#[test]
fn alerts_require_fresh_source_and_new_error_counters_never_infer_packet_loss() {
    let mut t = Tracker::default();
    t.record(Ok(source(1, 10)), 100_000);
    assert!(t.report.alerts(100).is_empty());
    let mut s = source(1, 11);
    s.ports[0].status = Some("DOWN".into());
    t.record(Ok(s), 110_000);
    let alerts = t.report.alerts(110).join("\n");
    assert!(alerts.contains("link DOWN") && alerts.contains("ierrors: +1"));
    assert!(!alerts.contains("loss") && !alerts.contains("%"));
    assert!(t.report.alerts(126).is_empty());
}
#[tokio::test(start_paused = true)]
async fn failure_polling_is_bounded_to_ten_seconds_and_abort_stops_publication() {
    use std::os::unix::fs::MetadataExt;
    let dir = tempfile::tempdir().unwrap();
    let settings = crate::worker_util::Settings {
        v3: true,
        socket: "unused".into(),
        nic_socket: dir.path().join("missing").display().to_string(),
        state: dir.path().display().to_string(),
    };
    let (_tx, rx) = tokio::sync::watch::channel(settings);
    let task = start(rx, crate::system_health::Hub::default());
    for _ in 0..100 {
        tokio::task::yield_now().await;
    }
    let path = dir.path().join(CACHE);
    let initial = std::fs::metadata(&path).unwrap().ino();
    for n in 1..=5 {
        tokio::time::advance(std::time::Duration::from_secs(2)).await;
        for _ in 0..100 {
            tokio::task::yield_now().await;
        }
        let same = std::fs::metadata(&path).unwrap().ino() == initial;
        assert_eq!(same, n < 5);
    }
    task.abort();
    let _ = task.await;
    let before = std::fs::metadata(&path).unwrap().ino();
    tokio::time::advance(std::time::Duration::from_secs(60)).await;
    for _ in 0..100 {
        tokio::task::yield_now().await;
    }
    assert_eq!(std::fs::metadata(&path).unwrap().ino(), before);
}
#[tokio::test]
async fn superseded_nic_read_cannot_publish_after_settings_reload() {
    let first = tempfile::tempdir().unwrap();
    let second = tempfile::tempdir().unwrap();
    let gate = std::sync::Arc::new(std::sync::Barrier::new(2));
    let worker_gate = gate.clone();
    let (arrived, mut waiting) = tokio::sync::mpsc::unbounded_channel();
    let (socket, thread) = server(move |cmd| {
        if cmd.starts_with("/ethdev/stats,") {
            arrived.send(()).unwrap();
            worker_gate.wait();
        }
        response(cmd)
    });
    let mut settings = crate::worker_util::Settings {
        v3: true,
        socket: "unused".into(),
        nic_socket: socket.path().join("telemetry").display().to_string(),
        state: first.path().display().to_string(),
    };
    let (tx, rx) = tokio::sync::watch::channel(settings.clone());
    let task = start(rx, crate::system_health::Hub::default());
    tokio::time::timeout(std::time::Duration::from_secs(2), waiting.recv())
        .await
        .unwrap()
        .unwrap();
    settings.state = second.path().display().to_string();
    settings.nic_socket = second.path().join("missing").display().to_string();
    tx.send(settings).unwrap();
    gate.wait();
    tokio::time::timeout(std::time::Duration::from_secs(2), async {
        while !second.path().join(CACHE).exists() {
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        }
    })
    .await
    .unwrap();
    assert!(!first.path().join(CACHE).exists());
    assert!(read_cache(&second.path().join(CACHE)).stale);
    task.abort();
    let _ = task.await;
    thread.join().unwrap();
}
#[test]
fn vendor_gauges_can_decrease_without_resetting_basic_counter_intervals() {
    let mut t = Tracker::default();
    let mut a = source(1, 1);
    a.ports[0].xstats.insert("tx_pp_jitter".into(), 100);
    t.record(Ok(a), 100_000);
    let mut b = source(1, 2);
    b.ports[0].xstats.insert("tx_pp_jitter".into(), 10);
    t.record(Ok(b), 110_000);
    assert!(!t.report.ports[0].reset);
    assert_eq!(t.report.ports[0].deltas["ierrors"], 1);
    assert!(
        t.report
            .measurements(110, true)
            .join("\n")
            .contains("tx_pp_jitter: 10")
    );
}
#[test]
fn maximum_nic_report_fits_cache_budget_and_bounded_detail_formatting() {
    let mut t = Tracker::default();
    let mut s = source(1, 0);
    s.ports.clear();
    for id in 0..8 {
        s.ports.push(Port {
            id,
            name: Some(format!("port-{id}")),
            xstats: (0..256)
                .map(|n| (format!("{n:04}_{}", "x".repeat(123)), u64::MAX))
                .collect(),
            ..Default::default()
        });
    }
    t.record(Ok(s), 100_000);
    let dir = tempfile::tempdir().unwrap();
    publish(dir.path(), &t.report).unwrap();
    let bytes = std::fs::metadata(dir.path().join(CACHE)).unwrap().len();
    assert!(bytes <= MAX_BYTES);
    assert_eq!(read_cache(&dir.path().join(CACHE)).ports.len(), 8);
    let start = std::time::Instant::now();
    for _ in 0..100 {
        let lines = t.report.measurements(100, true);
        assert!(lines.len() < 2200);
    }
    eprintln!(
        "maximum NIC cache {bytes} bytes; 100 debug detail formats {:?}",
        start.elapsed()
    );
}

#[test]
fn unified_health_separates_fresh_nic_data_from_link_failure() {
    let hub = crate::system_health::Hub::default();
    let mut tracker = Tracker::default();
    tracker.record(Ok(source(10, 0)), 100_000);
    tracker.report.ports[0].status = Some("DOWN".into());
    record_system_health(&hub, &tracker.report, 100, "nic.sock");
    let r = hub.snapshot(100, None);
    assert_eq!(
        r.components
            .iter()
            .find(|c| c.component == "nic")
            .unwrap()
            .state,
        crate::system_health::State::Healthy
    );
    assert_eq!(
        r.components
            .iter()
            .find(|c| c.component == "nic_links")
            .unwrap()
            .state,
        crate::system_health::State::Degraded
    );
    tracker.report.ports[0].id = 1;
    tracker.report.ports[0].status = Some("UP".into());
    record_system_health(&hub, &tracker.report, 100, "nic.sock");
    let r = hub.snapshot(100, None);
    assert!(!r.components.iter().any(|c| c.component == "nic_port/0"));
    assert_eq!(
        r.components
            .iter()
            .find(|c| c.component == "nic_links")
            .unwrap()
            .state,
        crate::system_health::State::Healthy
    );
}
